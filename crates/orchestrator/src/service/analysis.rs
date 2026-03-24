use std::path::PathBuf;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use bumpalo::Bump;
use foldhash::HashSet;

use mago_analyzer::Analyzer;
use mago_analyzer::analysis_result::AnalysisResult;
use mago_analyzer::plugin::PluginRegistry;
use mago_analyzer::settings::Settings;
use mago_atom::AtomSet;
use mago_codex::metadata::CodebaseMetadata;
use mago_codex::populator::populate_codebase;
use mago_codex::reference::SymbolReferences;
use mago_codex::scanner::scan_program;
use mago_database::DatabaseReader;
use mago_database::ReadDatabase;
use mago_database::file::FileId;
use mago_names::resolver::NameResolver;
use mago_reporting::Issue;
use mago_reporting::IssueCollection;
use mago_semantics::SemanticsChecker;
use mago_syntax::parser::parse_file_with_settings;
use mago_syntax::settings::ParserSettings;

use crate::error::OrchestratorError;
use crate::resolve_stub_file_ids;
use crate::service::pipeline::ParallelPipeline;
use crate::service::pipeline::Reducer;

pub struct AnalysisService {
    database: ReadDatabase,
    codebase: CodebaseMetadata,
    symbol_references: SymbolReferences,
    settings: Settings,
    parser_settings: ParserSettings,
    use_progress_bars: bool,
    plugin_registry: Arc<PluginRegistry>,
    stub_files: Vec<String>,
    workspace: PathBuf,
}

impl std::fmt::Debug for AnalysisService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisService")
            .field("database", &self.database)
            .field("codebase", &self.codebase)
            .field("symbol_references", &self.symbol_references)
            .field("settings", &self.settings)
            .field("parser_settings", &self.parser_settings)
            .field("use_progress_bars", &self.use_progress_bars)
            .field("plugin_registry", &self.plugin_registry)
            .field("stub_files", &self.stub_files)
            .field("workspace", &self.workspace)
            .finish()
    }
}

impl AnalysisService {
    #[must_use]
    pub fn new(
        database: ReadDatabase,
        codebase: CodebaseMetadata,
        symbol_references: SymbolReferences,
        settings: Settings,
        parser_settings: ParserSettings,
        use_progress_bars: bool,
        plugin_registry: Arc<PluginRegistry>,
        workspace: PathBuf,
        stub_files: Vec<String>,
        _stub_file_extensions: Vec<String>,
    ) -> Self {
        Self {
            database,
            codebase,
            symbol_references,
            settings,
            parser_settings,
            use_progress_bars,
            plugin_registry,
            stub_files,
            workspace,
        }
    }

    /// Analyzes a single file synchronously without using parallel processing.
    ///
    /// This method is designed for environments where threading is not available,
    /// such as WebAssembly. It performs static analysis on a single file by:
    /// 1. Parsing the file
    /// 2. Resolving names
    /// 3. Scanning symbols and extending the provided codebase
    /// 4. Populating the codebase (resolving inheritance, traits, etc.)
    /// 5. Running the analyzer
    ///
    /// # Arguments
    ///
    /// * `file_id` - The ID of the file to analyze.
    ///
    /// # Returns
    ///
    /// An `IssueCollection` containing all issues found in the file.
    pub fn oneshot(mut self, file_id: FileId) -> IssueCollection {
        let Ok(file) = self.database.get_ref(&file_id) else {
            tracing::error!("File with ID {:?} not found in database", file_id);

            return IssueCollection::default();
        };

        let arena = Bump::new();

        let program = parse_file_with_settings(&arena, file, self.parser_settings);
        let resolved_names = NameResolver::new(&arena).resolve(program);

        let mut issues = IssueCollection::new();
        if program.has_errors() {
            for error in program.errors.iter() {
                issues.push(Issue::from(error));
            }
        }

        let semantics_checker = SemanticsChecker::new(self.settings.version);
        issues.extend(semantics_checker.check(file, program, &resolved_names));

        let stub_file_ids = resolve_stub_file_ids(&self.database, &self.workspace, &self.stub_files);
        self.codebase = build_codebase_with_stubs(
            &self.database,
            std::mem::take(&mut self.codebase),
            stub_file_ids,
            self.parser_settings,
            Some(file_id),
        );

        let user_codebase = scan_program(&arena, file, program, &resolved_names);
        self.codebase.extend(user_codebase);

        populate_codebase(&mut self.codebase, &mut self.symbol_references, AtomSet::default(), HashSet::default());

        // Run the analyzer
        let mut analysis_result = AnalysisResult::new(self.symbol_references);
        let analyzer =
            Analyzer::new(&arena, file, &resolved_names, &self.codebase, &self.plugin_registry, self.settings);

        if let Err(err) = analyzer.analyze(program, &mut analysis_result) {
            issues.push(Issue::error(format!("Analysis error: {err}")));
        }

        issues.extend(analysis_result.issues);
        issues.extend(self.codebase.take_issues(true));
        issues
    }

    /// Runs the full analysis pipeline.
    ///
    /// This method scans all source files, builds the codebase, and runs the analyzer.
    pub fn run(self) -> Result<AnalysisResult, OrchestratorError> {
        #[cfg(not(target_arch = "wasm32"))]
        const ANALYSIS_DURATION_THRESHOLD: Duration = Duration::from_millis(5000);
        const ANALYSIS_PROGRESS_PREFIX: &str = "🔬 Analyzing";
        let stub_file_ids = resolve_stub_file_ids(&self.database, &self.workspace, &self.stub_files);

        let pipeline = ParallelPipeline::new(
            ANALYSIS_PROGRESS_PREFIX,
            self.database,
            self.codebase,
            self.symbol_references,
            (self.settings.clone(), self.parser_settings),
            self.parser_settings,
            Box::new(AnalysisResultReducer),
            self.use_progress_bars,
            stub_file_ids,
        );

        let plugin_registry = Arc::clone(&self.plugin_registry);

        pipeline.run(move |(settings, parser_settings), arena, source_file, codebase| {
            let mut analysis_result = AnalysisResult::new(SymbolReferences::new());

            let program = parse_file_with_settings(arena, &source_file, parser_settings);
            let resolved_names = NameResolver::new(arena).resolve(program);

            if program.has_errors() {
                analysis_result.issues.extend(program.errors.iter().map(Issue::from));
            }

            let semantics_checker = SemanticsChecker::new(settings.version);
            let analyzer = Analyzer::new(arena, &source_file, &resolved_names, &codebase, &plugin_registry, settings);

            analysis_result.issues.extend(semantics_checker.check(&source_file, program, &resolved_names));
            analyzer.analyze(program, &mut analysis_result)?;

            #[cfg(not(target_arch = "wasm32"))]
            if analysis_result.time_in_analysis > ANALYSIS_DURATION_THRESHOLD {
                tracing::warn!(
                    "Analysis of source file '{}' took longer than {}s: {}s",
                    source_file.name,
                    ANALYSIS_DURATION_THRESHOLD.as_secs_f32(),
                    analysis_result.time_in_analysis.as_secs_f32()
                );
            }

            Ok(analysis_result)
        })
    }
}

fn build_codebase_with_stubs(
    database: &ReadDatabase,
    mut codebase: CodebaseMetadata,
    stub_file_ids: HashSet<FileId>,
    parser_settings: ParserSettings,
    skip_file_id: Option<FileId>,
) -> CodebaseMetadata {
    for file in database.files() {
        if file.file_type.is_builtin() || stub_file_ids.contains(&file.id) || skip_file_id == Some(file.id) {
            continue;
        }

        let arena = Bump::new();
        let program = parse_file_with_settings(&arena, &file, parser_settings);
        let resolved_names = NameResolver::new(&arena).resolve(program);
        let metadata = scan_program(&arena, &file, program, &resolved_names);
        codebase.extend(metadata);
    }

    for file in database.files() {
        if !stub_file_ids.contains(&file.id) || skip_file_id == Some(file.id) {
            continue;
        }

        let arena = Bump::new();
        let program = parse_file_with_settings(&arena, &file, parser_settings);
        let resolved_names = NameResolver::new(&arena).resolve(program);
        let mut metadata = scan_program(&arena, &file, program, &resolved_names);
        metadata.mark_as_stub();
        codebase.apply_stub_metadata(metadata);
    }

    codebase
}

/// The "reduce" step for the analysis pipeline.
///
/// This struct aggregates the `AnalysisResult` from each parallel task into a single,
/// final `AnalysisResult` for the entire project.
#[derive(Debug, Clone)]
struct AnalysisResultReducer;

impl Reducer<AnalysisResult, AnalysisResult> for AnalysisResultReducer {
    fn reduce(
        &self,
        mut codebase: CodebaseMetadata,
        symbol_references: SymbolReferences,
        results: Vec<AnalysisResult>,
    ) -> Result<AnalysisResult, OrchestratorError> {
        let mut aggregated_result = AnalysisResult::new(symbol_references);
        for result in results {
            aggregated_result.extend(result);
        }

        aggregated_result.issues.extend(codebase.take_issues(true));

        Ok(aggregated_result)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::path::Path;
    use std::sync::LazyLock;

    use mago_analyzer::plugin::PluginRegistry;
    use mago_database::Database;
    use mago_database::DatabaseConfiguration;
    use mago_database::file::File;
    use mago_database::file::FileType;

    use super::*;

    static PLUGIN_REGISTRY: LazyLock<Arc<PluginRegistry>> =
        LazyLock::new(|| Arc::new(PluginRegistry::with_library_providers()));

    #[test]
    fn stub_files_patch_existing_definitions_during_analysis() {
        let config = DatabaseConfiguration {
            workspace: Cow::Owned(Path::new("/test").to_path_buf()),
            paths: vec![Cow::Borrowed("src")],
            includes: vec![Cow::Borrowed("stubs")],
            excludes: vec![],
            extensions: vec![Cow::Borrowed("php")],
        };

        let mut database = Database::new(config);
        database.add(File::new(
            Cow::Borrowed("src/datetime.php"),
            FileType::Host,
            None,
            Cow::Borrowed("<?php\ninterface DateTimeInterface {}\n"),
        ));
        database.add(File::new(
            Cow::Borrowed("src/main.php"),
            FileType::Host,
            None,
            Cow::Borrowed(
                "<?php\nclass Foo {\n    public function __call(string $name, array $arguments): mixed { return 1; }\n}\nfunction demo(Foo $foo): int {\n    return $foo->magic();\n}\n",
            ),
        ));
        database.add(File::new(
            Cow::Borrowed("stubs/foo.stub.php"),
            FileType::Vendored,
            None,
            Cow::Borrowed("<?php\n/** @method int magic() */\nclass Foo {}\n"),
        ));

        let service = AnalysisService::new(
            database.read_only(),
            CodebaseMetadata::new(),
            SymbolReferences::new(),
            Settings::default(),
            ParserSettings::default(),
            false,
            Arc::clone(&PLUGIN_REGISTRY),
            Path::new("/test").to_path_buf(),
            vec!["stubs/foo.stub.php".to_string()],
            vec![".stub".to_string()],
        );

        let result = service.run().expect("analysis should succeed");
        assert!(
            result.issues.is_empty(),
            "stub file should provide the missing magic method type: {:?}",
            result.issues
        );
    }

    #[test]
    fn stub_files_patch_declared_methods_on_existing_interfaces() {
        let config = DatabaseConfiguration {
            workspace: Cow::Owned(Path::new("/test").to_path_buf()),
            paths: vec![Cow::Borrowed("src")],
            includes: vec![Cow::Borrowed("stubs")],
            excludes: vec![],
            extensions: vec![Cow::Borrowed("php")],
        };

        let mut database = Database::new(config);
        database.add(File::new(
            Cow::Borrowed("src/datetime.php"),
            FileType::Host,
            None,
            Cow::Borrowed("<?php\ninterface DateTimeInterface {}\n"),
        ));
        database.add(File::new(
            Cow::Borrowed("src/main.php"),
            FileType::Host,
            None,
            Cow::Borrowed(
                "<?php\nfunction update(DateTimeInterface $date): DateTimeInterface {\n    return $date->modify('+1 day');\n}\n",
            ),
        ));
        database.add(File::new(
            Cow::Borrowed("stubs/DateTimeInterface.phpstub"),
            FileType::Vendored,
            None,
            Cow::Borrowed(
                "<?php\ninterface DateTimeInterface {\n    /** @return $this */\n    public function modify(string $modifier): static;\n}\n",
            ),
        ));

        let service = AnalysisService::new(
            database.read_only(),
            CodebaseMetadata::new(),
            SymbolReferences::new(),
            Settings::default(),
            ParserSettings::default(),
            false,
            Arc::clone(&PLUGIN_REGISTRY),
            Path::new("/test").to_path_buf(),
            vec!["stubs/DateTimeInterface.phpstub".to_string()],
            vec![".stub".to_string()],
        );

        let result = service.run().expect("analysis should succeed");
        assert!(
            result.issues.is_empty(),
            "stub file should patch the declared interface method: {:?}",
            result.issues
        );
    }
}
