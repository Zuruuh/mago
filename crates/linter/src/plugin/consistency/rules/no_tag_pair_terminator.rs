use indoc::indoc;

use mago_fixer::SafetyClassification;
use mago_reporting::*;
use mago_span::HasSpan;
use mago_syntax::ast::*;

use crate::context::LintContext;
use crate::definition::RuleDefinition;
use crate::definition::RuleUsageExample;
use crate::directive::LintDirective;
use crate::rule::Rule;

#[derive(Clone, Debug)]
pub struct NoTagPairTerminatorRule;

impl Rule for NoTagPairTerminatorRule {
    fn get_definition(&self) -> RuleDefinition {
        RuleDefinition::enabled("No Tag Pair Terminator", Level::Note)
            .with_description(indoc! {"
                   Discourages the use of `?><?php` as a statement terminator. Recommends using a semicolon
                   (`;`) instead for clarity and consistency.
               "})
            .with_example(RuleUsageExample::valid(
                "Using semicolon to terminate statements",
                indoc! {r#"
                    <?php

                    echo "Hello World";
                "#},
            ))
            .with_example(RuleUsageExample::invalid(
                "Using `?><?php` to terminate statements",
                indoc! {r#"
                    <?php

                    echo "Hello World" ?><?php
                "#},
            ))
    }

    fn lint_node(&self, node: Node<'_>, context: &mut LintContext<'_>) -> LintDirective {
        let Node::Terminator(terminator) = node else { return LintDirective::default() };
        let Terminator::TagPair(close, open) = terminator else {
            return LintDirective::default();
        };

        let issue = Issue::new(context.level(), "Semicolon terminator is preferred over tag-pair terminator")
            .with_annotation(
                Annotation::primary(close.span().join(open.span()))
                    .with_message("This tag-pair terminator `?><?php` is not recommended."),
            )
            .with_help("Replace `?><?php` with a `;`");

        context.propose(issue, |plan| {
            plan.replace(close.span().join(open.span()).to_range(), ";", SafetyClassification::Safe)
        });

        LintDirective::default()
    }
}
