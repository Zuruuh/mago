use indoc::indoc;
use toml::Value;

use mago_reporting::*;
use mago_span::*;
use mago_syntax::ast::*;

use crate::context::LintContext;
use crate::definition::RuleDefinition;
use crate::definition::RuleOptionDefinition;
use crate::definition::RuleUsageExample;
use crate::directive::LintDirective;
use crate::rule::Rule;

const CAMEL: &str = "camel";
const CAMEL_DEFAULT: bool = false;
const EITHER: &str = "either";
const EITHER_DEFAULT: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct FunctionRule;

impl Rule for FunctionRule {
    fn get_definition(&self) -> RuleDefinition {
        RuleDefinition::enabled("Function", Level::Help)
            .with_description(indoc! {"
                Detects function declarations that do not follow camel or snake naming convention.
                Function names should be in camel case or snake case, depending on the configuration.
            "})
            .with_option(RuleOptionDefinition {
                name: CAMEL,
                r#type: "boolean",
                description: "Whether function names should be in camel case.",
                default: Value::Boolean(CAMEL_DEFAULT),
            })
            .with_option(RuleOptionDefinition {
                name: EITHER,
                r#type: "boolean",
                description: "Whether function names should be in either camel case or snake case.",
                default: Value::Boolean(EITHER_DEFAULT),
            })
            .with_example(RuleUsageExample::valid(
                "A function name in snake case",
                indoc! {r#"
                    <?php

                    function my_function() {}
                "#},
            ))
            .with_example(
                RuleUsageExample::valid(
                    "A function name in camel case, with camel case enabled",
                    indoc! {r#"
                    <?php

                    function myFunction() {}
                "#},
                )
                .with_option(CAMEL, Value::Boolean(true)),
            )
            .with_example(
                RuleUsageExample::valid(
                    "Function names in either camel or snake case, with either case enabled",
                    indoc! {r#"
                    <?php

                    function myFunction() {}
                    function my_function() {}
                "#},
                )
                .with_option(EITHER, Value::Boolean(true)),
            )
            .with_example(RuleUsageExample::invalid(
                "A function name not in snake case",
                indoc! {r#"
                    <?php

                    function MyFunction() {}
                    function My_Function() {}
                "#},
            ))
            .with_example(
                RuleUsageExample::invalid(
                    "A function name not in camel case, with camel case enabled",
                    indoc! {r#"
                    <?php

                    function my_function() {}
                "#},
                )
                .with_option(CAMEL, Value::Boolean(true)),
            )
    }

    fn lint_node(&self, node: Node<'_>, context: &mut LintContext<'_>) -> LintDirective {
        let Node::Function(function) = node else { return LintDirective::default() };

        let name = context.lookup(&function.name.value);
        let fqfn = context.lookup_name(&function.name);
        let camel_case = context.option(CAMEL).and_then(|v| v.as_bool()).unwrap_or(CAMEL_DEFAULT);
        let either_case = context.option(EITHER).and_then(|v| v.as_bool()).unwrap_or(EITHER_DEFAULT);

        if either_case {
            if !mago_casing::is_camel_case(name) && !mago_casing::is_snake_case(name) {
                context.report(
                    Issue::new(
                        context.level(),
                        format!("Function name `{name}` should be in either camel case or snake case."),
                    )
                    .with_annotation(
                        Annotation::primary(function.name.span())
                            .with_message(format!("Function `{name}` is declared here.`")),
                    )
                    .with_annotation(
                        Annotation::secondary(function.span())
                            .with_message(format!("Function `{fqfn}` is defined here.")),
                    )
                    .with_note(format!(
                        "The function name `{name}` does not follow either camel case or snake naming convention."
                    ))
                    .with_help(format!(
                        "Consider renaming it to `{}` or `{}` to adhere to the naming convention.",
                        mago_casing::to_camel_case(name),
                        mago_casing::to_snake_case(name)
                    )),
                );
            }
        } else if camel_case {
            if !mago_casing::is_camel_case(name) {
                context.report(
                    Issue::new(context.level(), format!("Function name `{name}` should be in camel case."))
                        .with_annotation(
                            Annotation::primary(function.name.span())
                                .with_message(format!("Function `{name}` is declared here.`")),
                        )
                        .with_annotation(
                            Annotation::secondary(function.span())
                                .with_message(format!("Function `{fqfn}` is defined here.")),
                        )
                        .with_note(format!("The function name `{name}` does not follow camel naming convention."))
                        .with_help(format!(
                            "Consider renaming it to `{}` to adhere to the naming convention.",
                            mago_casing::to_camel_case(name)
                        )),
                );
            }
        } else if !mago_casing::is_snake_case(name) {
            context.report(
                Issue::new(context.level(), format!("Function name `{name}` should be in snake case."))
                    .with_annotation(
                        Annotation::primary(function.name.span())
                            .with_message(format!("Function `{name}` is declared here.`")),
                    )
                    .with_annotation(
                        Annotation::secondary(function.span())
                            .with_message(format!("Function `{fqfn}` is defined here.")),
                    )
                    .with_note(format!("The function name `{name}` does not follow snake naming convention."))
                    .with_help(format!(
                        "Consider renaming it to `{}` to adhere to the naming convention.",
                        mago_casing::to_snake_case(name)
                    )),
            );
        }

        LintDirective::default()
    }
}
