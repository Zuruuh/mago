use indoc::indoc;

use mago_ast::*;
use mago_fixer::SafetyClassification;
use mago_reporting::*;
use mago_span::HasSpan;
use mago_walker::Walker;

use crate::context::LintContext;
use crate::definition::RuleDefinition;
use crate::definition::RuleUsageExample;
use crate::rule::Rule;

#[derive(Clone, Debug)]
pub struct InterfaceShouldBeUsed;

impl Rule for InterfaceShouldBeUsed {
    fn get_definition(&self) -> RuleDefinition {
        RuleDefinition::enabled("Interface Should Be Used", Level::Note)
            .with_description(indoc! {"
                Detects when an implementation class is used instead of the interface.
            "})
            .with_example(RuleUsageExample::valid(
                "A controller that uses the interface instead of the implementation",
                indoc! {r#"
                    <?php

                    namespace App\Controller;

                    use Symfony\Component\HttpFoundation\Request;
                    use Symfony\Component\HttpFoundation\Response;
                    use Symfony\Component\Routing\Annotation\Route;
                    use Symfony\Component\Serializer\SerializerInterface;

                    class UserController
                    {
                        public function __construct(SerializerInterface $serializer)
                        {
                            $this->serializer = $serializer;
                        }

                        // ...
                    }
                "#},
            ))
            .with_example(RuleUsageExample::invalid(
                "A controller that uses the implementation instead of the interface",
                indoc! {r#"
                    <?php

                    namespace App\Controller;

                    use Symfony\Component\HttpFoundation\Request;
                    use Symfony\Component\HttpFoundation\Response;
                    use Symfony\Component\Routing\Annotation\Route;
                    use Symfony\Component\Serializer\Serializer;

                    class UserController
                    {
                        public function __construct(Serializer $serializer)
                        {
                            $this->serializer = $serializer;
                        }

                        // ...
                    }
                "#},
            ))
    }
}

impl<'a> Walker<LintContext<'a>> for InterfaceShouldBeUsed {
    fn walk_in_hint<'ast>(&self, hint: &'ast Hint, context: &mut LintContext<'a>) {
        let Hint::Identifier(identifier) = hint else {
            return;
        };

        let fqcn = context.lookup_name(identifier);
        for (implementation, interface) in IMPLEMENTATION_TO_INTERFACE.iter() {
            if fqcn == *implementation {
                let issue = Issue::new(
                    context.level(),
                    format!("Use the interface `{}` instead of the implementation `{}`", interface, implementation,),
                )
                .with_annotation(
                    Annotation::primary(identifier.span())
                        .with_message("This uses the implementation instead of the interface."),
                );

                context.report_with_fix(issue, |plan| {
                    // the change is potentially unsafe because we don't
                    // know if the user is using implementation-specific methods/properties
                    // that are not part of the interface
                    plan.replace(
                        identifier.span().to_range(),
                        format!("\\{}", interface),
                        SafetyClassification::PotentiallyUnsafe,
                    )
                });

                return;
            }
        }
    }
}

const IMPLEMENTATION_TO_INTERFACE: [(&str, &str); 3] = [
    ("Symfony\\Component\\Serializer\\Serializer", "Symfony\\Component\\Serializer\\SerializerInterface"),
    (
        "Symfony\\'Component\\Serializer\\Encoder\\JsonEncode",
        "Symfony\\Component\\Serializer\\Encoder\\EncoderInterface",
    ),
    (
        "Symfony\\'Component\\Serializer\\Encoder\\JsonDecode",
        "Symfony\\Component\\Serializer\\Encoder\\DecoderInterface",
    ),
];
