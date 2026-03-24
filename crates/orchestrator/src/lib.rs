//! Orchestrator for managing and coordinating Mago's analysis tools.
//!
//! The orchestrator crate provides a high-level interface for running various static analysis
//! tasks on PHP codebases. It coordinates between the database, parser, analyzer, linter,
//! formatter, and architectural guard to provide a unified workflow.
//!
//! # Architecture
//!
//! The orchestrator follows a service-oriented architecture where each tool (linter, analyzer,
//! formatter, guard) is encapsulated in its own service. The [`Orchestrator`] struct acts as
//! a factory and coordinator, managing:
//!
//! - **Database**: File system scanning and caching via [`mago_database::Database`]
//! - **Codebase**: Metadata and symbol references via [`mago_codex`]
//! - **Services**: Tool-specific services that operate on the database and codebase
//!
//! # Services
//!
//! The orchestrator provides four main services:
//!
//! - [`LintService`]: Runs linting rules on PHP code
//! - [`AnalysisService`]: Performs static analysis
//! - [`GuardService`]: Enforces architectural rules
//! - [`FormatService`]: Formats PHP code
//!
//! # Workflow
//!
//! A typical workflow involves:
//!
//! 1. Create an [`Orchestrator`] with an [`OrchestratorConfiguration`]
//! 2. Load the database using [`Orchestrator::load_database`]
//! 3. Obtain the desired service (e.g., [`Orchestrator::get_lint_service`])
//! 4. Run the service to get results

use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

use bumpalo::Bump;
use foldhash::HashSet;
use glob::glob;
use glob::Pattern;
use mago_analyzer::plugin::PluginRegistry;
use mago_analyzer::plugin::create_registry_with_plugins;
use mago_codex::metadata::CodebaseMetadata;
use mago_codex::reference::SymbolReferences;
use mago_database::Database;
use mago_database::DatabaseConfiguration;
use mago_database::DatabaseReader;
use mago_database::ReadDatabase;
use mago_database::exclusion::Exclusion;
use mago_database::file::File;
use mago_database::file::FileId;
use mago_database::loader::DatabaseLoader;
use walkdir::WalkDir;

use crate::service::analysis::AnalysisService;
use crate::service::format::FileFormatStatus;
use crate::service::format::FormatService;
use crate::service::guard::GuardService;
use crate::service::incremental_analysis::IncrementalAnalysisService;
use crate::service::lint::LintService;

pub use config::OrchestratorConfiguration;
pub use error::OrchestratorError;

pub mod config;
pub mod error;
pub mod progress;
pub mod service;

fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[') || pattern.contains('{')
}

fn normalize_match_path(path: &str) -> String {
    path.trim_start_matches("./").trim_start_matches(".\\").replace('\\', "/").trim_end_matches('/').to_string()
}

fn file_name_matches_stub_extension(path: &Path, stub_file_extensions: &[&str]) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    stub_file_extensions.iter().any(|extension| {
        if extension.starts_with('.') {
            file_name.ends_with(extension)
        } else {
            file_name.ends_with(&format!(".{extension}"))
        }
    })
}

#[derive(Debug, Default)]
struct StubFileMatcher {
    exact_names: HashSet<String>,
    exact_paths: Vec<PathBuf>,
    directory_names: Vec<String>,
    directory_paths: Vec<PathBuf>,
    glob_patterns: Vec<Pattern>,
}

impl StubFileMatcher {
    fn from_config(workspace: &Path, stub_files: &[String]) -> Self {
        let mut matcher = Self::default();

        for stub_file in stub_files {
            if is_glob_pattern(stub_file) {
                let pattern_source =
                    if Path::new(stub_file).is_absolute() { stub_file.clone() } else { normalize_match_path(stub_file) };
                if let Ok(pattern) = Pattern::new(&pattern_source) {
                    matcher.glob_patterns.push(pattern);
                }

                continue;
            }

            let absolute_path = if Path::new(stub_file).is_absolute() {
                PathBuf::from(stub_file)
            } else {
                workspace.join(stub_file)
            };
            let normalized_name = normalize_match_path(stub_file);

            if absolute_path.is_dir() {
                matcher.directory_names.push(normalized_name);
                matcher.directory_paths.push(absolute_path.canonicalize().unwrap_or(absolute_path));
                continue;
            }

            matcher.exact_names.insert(normalized_name);
            matcher.exact_paths.push(absolute_path.canonicalize().unwrap_or(absolute_path));
        }

        matcher
    }

    fn matches_file(&self, file: &File) -> bool {
        let normalized_name = normalize_match_path(file.name.as_ref());

        if self.exact_names.contains(&normalized_name) {
            return true;
        }

        if self
            .directory_names
            .iter()
            .any(|directory| normalized_name == *directory || normalized_name.starts_with(&format!("{directory}/")))
        {
            return true;
        }

        if self.glob_patterns.iter().any(|pattern| pattern.matches(&normalized_name)) {
            return true;
        }

        let Some(path) = file.path.as_deref() else {
            return false;
        };

        if self.exact_paths.iter().any(|exact_path| exact_path == path) {
            return true;
        }

        if self.directory_paths.iter().any(|directory| path.starts_with(directory)) {
            return true;
        }

        let Some(path_str) = path.to_str() else {
            return false;
        };

        self.glob_patterns.iter().any(|pattern| pattern.matches(path_str))
    }
}

pub(crate) fn resolve_stub_file_ids(database: &ReadDatabase, workspace: &Path, stub_files: &[String]) -> HashSet<FileId> {
    if stub_files.is_empty() {
        return HashSet::default();
    }

    let matcher = StubFileMatcher::from_config(workspace, stub_files);

    database
        .files()
        .filter(|file| matcher.matches_file(file))
        .map(|file| file.id)
        .collect()
}

/// The main orchestrator for running operations on PHP code.
///
/// The [`Orchestrator`] is the central coordinator that provides factory methods for creating
/// various services (linting, analysis, formatting, guarding) and manages the shared configuration
/// and database loading.
///
/// # Responsibilities
///
/// - **Configuration Management**: Stores and provides access to the configuration for all services
/// - **Database Loading**: Handles file system scanning and database initialization
/// - **Service Creation**: Acts as a factory for creating tool-specific services
/// - **Path Management**: Manages source paths and exclusion patterns
#[derive(Debug)]
pub struct Orchestrator<'a> {
    /// Configuration for all operations.
    pub config: OrchestratorConfiguration<'a>,
    /// Plugin registry for the analyzer, lazily initialized.
    plugin_registry: OnceLock<Arc<PluginRegistry>>,
}

impl<'a> Orchestrator<'a> {
    /// Creates a new orchestrator with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration specifying PHP version, paths, tool settings, etc.
    #[must_use]
    pub fn new(config: OrchestratorConfiguration<'a>) -> Self {
        Self { config, plugin_registry: OnceLock::new() }
    }

    /// Gets the analyzer plugin registry, initializing it if necessary.
    ///
    /// This method returns a shared reference to the plugin registry used by the analysis service.
    /// If the registry has not been initialized yet, it creates a new one based on the configuration.
    ///
    /// # Returns
    ///
    /// An `Arc` pointing to the `PluginRegistry`.
    pub fn get_analyzer_plugin_registry(&self) -> Arc<PluginRegistry> {
        Arc::clone(self.plugin_registry.get_or_init(|| {
            Arc::new(create_registry_with_plugins(
                &self.config.analyzer_plugins,
                self.config.disable_default_analyzer_plugins,
            ))
        }))
    }

    /// Adds additional exclusion patterns to the orchestrator's configuration.
    ///
    /// These patterns will be used when loading the database to exclude files and directories
    /// from scanning. Patterns can be glob patterns (e.g., `"*.tmp"`, `"vendor/*"`) or
    /// direct paths.
    ///
    /// # Arguments
    ///
    /// * `patterns` - A vector of string patterns to exclude from file scanning
    pub fn add_exclude_patterns<T>(&mut self, patterns: impl Iterator<Item = &'a T>)
    where
        T: AsRef<str> + 'a,
    {
        self.config.excludes.extend(patterns.map(std::convert::AsRef::as_ref));
    }

    /// Sets new source paths and moves the old paths to the includes list.
    ///
    /// This method replaces the current source paths with the provided paths and moves
    /// the old source paths to the includes list. This is useful when you want to change
    /// the primary analysis targets while keeping the old paths as context providers.
    ///
    /// # Arguments
    ///
    /// * `paths` - The new source paths or glob patterns to analyze
    pub fn set_source_paths(&mut self, paths: impl IntoIterator<Item = impl AsRef<str>>) {
        let old_paths = std::mem::take(&mut self.config.paths);

        self.config.paths = paths.into_iter().map(|p| p.as_ref().to_string()).collect();
        self.config.includes.extend(old_paths);
    }

    /// Loads the database by scanning the file system according to the configuration.
    ///
    /// This method scans the workspace directory and builds a database of all PHP files
    /// according to the configured paths, includes, excludes, and extensions. The database
    /// provides fast access to file contents and metadata for all tools.
    ///
    /// # Arguments
    ///
    /// * `workspace` - The root directory of the project to analyze
    /// * `include_externals` - Whether to include files from the `includes` list in the database.
    ///   External files (e.g., vendor dependencies) provide context for analysis but are not
    ///   directly analyzed, linted, or formatted.
    /// * `prelude_database` - An optional pre-existing database to merge with. This is useful
    ///   for including standard library or framework stubs.
    ///
    /// # Returns
    ///
    /// Returns a [`Database`] containing all discovered PHP files, or an [`OrchestratorError`]
    /// if the database could not be loaded.
    pub fn load_database<'b>(
        &'b self,
        workspace: &'a Path,
        include_externals: bool,
        prelude_database: Option<Database<'static>>,
    ) -> Result<Database<'a>, OrchestratorError>
    where
        'b: 'a,
    {
        /// Converts string patterns from the configuration into `Exclusion` types.
        fn create_excludes_from_patterns<'a>(patterns: &[&'a str], root: &Path) -> Vec<Exclusion<'a>> {
            patterns
                .iter()
                .map(|pattern| {
                    if pattern.contains('*') {
                        if let Some(stripped) = pattern.strip_prefix("./") {
                            let rooted_pattern = root.join(stripped).to_string_lossy().into_owned();

                            Exclusion::Pattern(Cow::Owned(rooted_pattern))
                        } else {
                            Exclusion::Pattern(Cow::Borrowed(pattern))
                        }
                    } else {
                        let path = PathBuf::from(pattern);
                        let path_buf = if path.is_absolute() { path } else { root.join(path) };

                        Exclusion::Path(Cow::Owned(path_buf.canonicalize().unwrap_or(path_buf)))
                    }
                })
                .collect()
        }

        fn load_stub_files_into_database(
            database: &mut Database<'_>,
            workspace: &Path,
            stub_files: &[String],
            stub_file_extensions: &[&str],
        ) -> Result<(), OrchestratorError> {
            for stub_file in stub_files {
                if is_glob_pattern(stub_file) {
                    let pattern = if Path::new(stub_file).is_absolute() {
                        stub_file.clone()
                    } else {
                        workspace.join(stub_file).to_string_lossy().into_owned()
                    };

                    for entry in glob(&pattern).map_err(|error| OrchestratorError::General(error.to_string()))? {
                        let path = entry.map_err(|error| OrchestratorError::General(error.to_string()))?;
                        if !path.is_file() {
                            continue;
                        }

                        database.add(File::read(workspace, &path, mago_database::file::FileType::Vendored)?);
                    }

                    continue;
                }

                let path = if Path::new(stub_file).is_absolute() {
                    PathBuf::from(stub_file)
                } else {
                    workspace.join(stub_file)
                };

                if path.is_dir() {
                    for entry in WalkDir::new(&path).into_iter().filter_map(Result::ok) {
                        let stub_path = entry.path();
                        if !stub_path.is_file() || !file_name_matches_stub_extension(stub_path, stub_file_extensions) {
                            continue;
                        }

                        database.add(File::read(workspace, stub_path, mago_database::file::FileType::Vendored)?);
                    }

                    continue;
                }

                if path.is_file() {
                    database.add(File::read(workspace, &path, mago_database::file::FileType::Vendored)?);
                }
            }

            Ok(())
        }

        let includes = if include_externals {
            self.config.includes.iter().map(|s| Cow::Borrowed(s.as_ref())).collect::<Vec<Cow<'a, str>>>()
        } else {
            Vec::new()
        };

        let configuration: DatabaseConfiguration<'a> = DatabaseConfiguration {
            workspace: Cow::Borrowed(workspace),
            paths: self.config.paths.iter().map(|s| Cow::Borrowed(s.as_ref())).collect(),
            includes,
            excludes: create_excludes_from_patterns(&self.config.excludes, workspace),
            extensions: self.config.extensions.iter().map(|s| Cow::Borrowed(*s)).collect(),
        };

        let mut loader = DatabaseLoader::new(configuration);

        if let Some(prelude_db) = prelude_database {
            loader = loader.with_database(prelude_db);
        }

        let mut result = loader.load().map_err(OrchestratorError::Database)?;
        if include_externals && !self.config.stub_files.is_empty() {
            load_stub_files_into_database(
                &mut result,
                workspace,
                &self.config.stub_files,
                &self.config.stub_file_extensions,
            )?;
        }

        Ok(result)
    }

    /// Creates a linting service with the current configuration.
    ///
    /// The linting service checks PHP code against a set of rules to identify potential
    /// issues, style violations, and code smells.
    ///
    /// # Arguments
    ///
    /// * `database` - A read-only database handle containing the PHP files to lint
    ///
    /// # Returns
    ///
    /// A [`LintService`] configured with the orchestrator's linter settings and progress bar preferences.
    pub fn get_lint_service(&self, database: ReadDatabase) -> LintService {
        LintService::new(
            database,
            self.config.linter_settings.clone(),
            self.config.parser_settings,
            self.config.use_progress_bars,
        )
    }

    /// Creates an architectural guard service with the current configuration.
    ///
    /// The guard service enforces architectural constraints and layer dependencies in your
    /// codebase, ensuring that code follows the defined architectural rules.
    ///
    /// # Arguments
    ///
    /// * `database` - A read-only database handle containing the PHP files to check
    /// * `codebase` - Metadata about the codebase structure and symbols
    ///
    /// # Returns
    ///
    /// A [`GuardService`] configured with the orchestrator's guard settings and progress bar preferences.
    pub fn get_guard_service(&self, database: ReadDatabase, codebase: CodebaseMetadata) -> GuardService {
        GuardService::new(
            database,
            codebase,
            self.config.guard_settings.clone(),
            self.config.parser_settings,
            self.config.use_progress_bars,
        )
    }

    /// Creates a static analysis service with the current configuration.
    ///
    /// The analysis service performs deep static analysis on PHP code, including type checking,
    /// control flow analysis, and detection of logical errors and type mismatches.
    ///
    /// For incremental/watch mode analysis, use [`get_incremental_analysis_service`](Self::get_incremental_analysis_service) instead.
    ///
    /// # Arguments
    ///
    /// * `database` - A read-only database handle containing the PHP files to analyze
    /// * `codebase` - Metadata about the codebase structure and symbols
    /// * `symbol_references` - Information about symbol usage and references across the codebase
    ///
    /// # Returns
    ///
    /// An [`AnalysisService`] configured with the orchestrator's analyzer settings and progress bar preferences.
    pub fn get_analysis_service(
        &self,
        database: ReadDatabase,
        codebase: CodebaseMetadata,
        symbol_references: SymbolReferences,
    ) -> AnalysisService {
        AnalysisService::new(
            database,
            codebase,
            symbol_references,
            self.config.analyzer_settings.clone(),
            self.config.parser_settings,
            self.config.use_progress_bars,
            self.get_analyzer_plugin_registry(),
            self.config.workspace.to_path_buf(),
            self.config.stub_files.clone(),
            self.config.stub_file_extensions.iter().map(|extension| (*extension).to_string()).collect(),
        )
    }

    /// Creates an incremental analysis service for watch mode or LSP integration.
    ///
    /// The service manages its own incremental state internally and provides a clean API
    /// for running full and incremental analysis without being coupled to CLI output
    /// or file watchers.
    ///
    /// # Arguments
    ///
    /// * `database` - A read-only database handle containing the PHP files to analyze
    /// * `codebase` - Base codebase metadata (prelude only, no user symbols)
    /// * `symbol_references` - Base symbol references (prelude only)
    ///
    /// # Returns
    ///
    /// An [`IncrementalAnalysisService`] ready for analysis.
    pub fn get_incremental_analysis_service(
        &self,
        database: ReadDatabase,
        codebase: CodebaseMetadata,
        symbol_references: SymbolReferences,
    ) -> IncrementalAnalysisService {
        IncrementalAnalysisService::new(
            database,
            codebase,
            symbol_references,
            self.config.analyzer_settings.clone(),
            self.config.parser_settings,
            self.get_analyzer_plugin_registry(),
            self.config.workspace.to_path_buf(),
            self.config.stub_files.clone(),
            self.config.stub_file_extensions.iter().map(|extension| (*extension).to_string()).collect(),
        )
    }

    /// Creates a code formatting service with the current configuration.
    ///
    /// The formatting service formats PHP code according to the configured style settings,
    /// ensuring consistent code style across the codebase.
    ///
    /// # Arguments
    ///
    /// * `database` - A read-only database handle containing the PHP files to format
    ///
    /// # Returns
    ///
    /// A [`FormatService`] configured with the orchestrator's formatter settings, PHP version,
    /// and progress bar preferences.
    pub fn get_format_service(&self, database: ReadDatabase) -> FormatService {
        FormatService::new(
            database,
            self.config.php_version,
            self.config.formatter_settings,
            self.config.parser_settings,
            self.config.use_progress_bars,
        )
    }

    /// Formats a single file according to the configured style settings.
    ///
    /// This is a convenience method for formatting an individual file without requiring
    /// a full database. It creates a temporary format service with an empty database and
    /// uses it to format the provided file.
    ///
    /// # Arguments
    ///
    /// * `file` - The file to format
    ///
    /// # Returns
    ///
    /// - `Ok(FileFormatStatus::Unchanged)` if the file is already properly formatted
    /// - `Ok(FileFormatStatus::Changed(String))` if the file was formatted, containing the new content
    /// - `Ok(FileFormatStatus::FailedToParse(ParseError))` if the file couldn't be parsed
    /// - `Err(OrchestratorError)` if formatting failed for other reasons
    ///
    /// # Performance
    ///
    /// This method allocates a new bump arena for each call. For formatting multiple files,
    /// consider using [`get_format_service`](Self::get_format_service) and calling the
    /// service's methods with a reused arena.
    pub fn format_file(&self, file: &File) -> Result<FileFormatStatus, OrchestratorError> {
        let service = self.get_format_service(ReadDatabase::empty());

        service.format_file(file)
    }

    /// Formats a single file using a provided bump arena for allocations.
    ///
    /// This method is similar to [`format_file`](Self::format_file) but allows you to
    /// provide your own bump arena for memory allocations. This is more efficient when
    /// formatting multiple files sequentially, as you can reuse and reset the same arena.
    ///
    /// # Arguments
    ///
    /// * `file` - The file to format
    /// * `arena` - A bump allocator for temporary allocations during formatting
    ///
    /// # Returns
    ///
    /// - `Ok(FileFormatStatus::Unchanged)` if the file is already properly formatted
    /// - `Ok(FileFormatStatus::Changed(String))` if the file was formatted, containing the new content
    /// - `Ok(FileFormatStatus::FailedToParse(ParseError))` if the file couldn't be parsed
    /// - `Err(OrchestratorError)` if formatting failed for other reasons
    ///
    /// # Performance
    ///
    /// Using this method with a reused arena (resetting it between calls) is significantly
    /// more efficient than calling [`format_file`](Self::format_file) repeatedly, as it
    /// avoids repeated allocator initialization.
pub fn format_file_in(&self, file: &File, arena: &Bump) -> Result<FileFormatStatus, OrchestratorError> {
        let service = self.get_format_service(ReadDatabase::empty());

        service.format_file_in(file, arena)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use mago_analyzer::settings::Settings as AnalyzerSettings;
    use mago_formatter::settings::FormatSettings;
    use mago_guard::settings::Settings as GuardSettings;
    use mago_linter::settings::Settings as LinterSettings;
    use mago_php_version::PHPVersion;
    use mago_syntax::settings::ParserSettings;
    use tempfile::TempDir;

    use super::*;

    fn test_config<'a>() -> OrchestratorConfiguration<'a> {
        OrchestratorConfiguration {
            php_version: PHPVersion::PHP84,
            workspace: Path::new("/test"),
            paths: vec!["src".to_string()],
            includes: vec![],
            stub_files: vec!["stubs".to_string()],
            stub_file_extensions: vec![".phpstub"],
            excludes: vec![],
            extensions: vec!["php"],
            parser_settings: ParserSettings::default(),
            analyzer_settings: AnalyzerSettings::default(),
            linter_settings: LinterSettings::default(),
            guard_settings: GuardSettings::default(),
            formatter_settings: FormatSettings::default(),
            disable_default_analyzer_plugins: false,
            analyzer_plugins: vec![],
            use_progress_bars: false,
            use_colors: false,
        }
    }

    #[test]
    fn directory_stub_files_load_using_stub_file_extensions() {
        let workspace = TempDir::new().expect("temp dir should be created");
        fs::create_dir_all(workspace.path().join("src")).expect("src dir should be created");
        fs::create_dir_all(workspace.path().join("stubs/nested")).expect("stubs dir should be created");
        fs::write(workspace.path().join("src/main.php"), "<?php\nclass Foo {}\n").expect("main file should be written");
        fs::write(workspace.path().join("stubs/Foo.phpstub"), "<?php\nclass Foo {}\n")
            .expect("stub file should be written");
        fs::write(workspace.path().join("stubs/nested/ignored.php"), "<?php\nclass Ignored {}\n")
            .expect("non-stub file should be written");

        let orchestrator = Orchestrator::new(test_config());
        let database = orchestrator
            .load_database(workspace.path(), true, None)
            .expect("database should load with directory stubs");

        assert!(database.files().any(|file| file.name.as_ref() == "stubs/Foo.phpstub"));
        assert!(!database.files().any(|file| file.name.as_ref() == "stubs/nested/ignored.php"));

        let stub_file_ids = resolve_stub_file_ids(&database.read_only(), workspace.path(), &["stubs".to_string()]);
        let foo_stub_id = database
            .files()
            .find(|file| file.name.as_ref() == "stubs/Foo.phpstub")
            .map(|file| file.id)
            .expect("stub file should be present");

        assert!(stub_file_ids.contains(&foo_stub_id));
    }
}
