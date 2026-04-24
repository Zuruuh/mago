use std::cmp::Ordering;

use indoc::indoc;
use mago_php_version::PHPVersion;
use mago_php_version::PHPVersionRange;
use mago_reporting::Annotation;
use mago_reporting::Issue;
use mago_reporting::Level;
use mago_span::HasSpan;
use mago_span::Span;
use mago_syntax::ast::AttributeList;
use mago_syntax::ast::Node;
use mago_syntax::ast::NodeKind;
use mago_syntax::ast::Sequence;
use mago_text_edit::TextEdit;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::MapAccess;
use serde::de::Visitor;
use serde::de::{self};

use crate::category::Category;
use crate::context::LintContext;
use crate::requirements::RuleRequirements;
use crate::rule::Config;
use crate::rule::LintRule;
use crate::rule_meta::RuleMeta;
use crate::settings::RuleSettings;

#[derive(Debug, Clone)]
pub struct OrderedAttributesRule {
    meta: &'static RuleMeta,
    cfg: OrderedAttributesConfig,
}

#[inline]
const fn default_level() -> Level {
    Level::Note
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OrderedAttributesConfig {
    Alpha {
        #[serde(default = "default_level")]
        level: Level,
    },
    Custom {
        #[serde(default = "default_level")]
        level: Level,
        order: Vec<OrderedAttributesOrder>,
        #[serde(default = "default_fallback")]
        fallback: OrderedAttributesFallback,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum OrderedAttributesSortAlgorithm {
    #[default]
    Alpha,
    Custom,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum OrderedAttributesFallback {
    Alpha,
    Preserve,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum OrderedAttributesOrder {
    Class(String),
    Namespace { namespace: String, order: OrderedAttributesFallback },
}

impl From<String> for OrderedAttributesOrder {
    fn from(value: String) -> Self {
        Self::Class(value)
    }
}

impl From<&str> for OrderedAttributesOrder {
    fn from(value: &str) -> Self {
        Self::Class(value.to_string())
    }
}

impl OrderedAttributesOrder {
    fn from_toml_string(value: String) -> Self {
        if value.ends_with('\\') {
            Self::Namespace { namespace: value, order: OrderedAttributesFallback::Alpha }
        } else {
            Self::Class(value)
        }
    }
}

impl<'de> Deserialize<'de> for OrderedAttributesOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderedAttributesOrderVisitor;

        impl<'de> Visitor<'de> for OrderedAttributesOrderVisitor {
            type Value = OrderedAttributesOrder;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a class string, a namespace string ending in \\, or a namespace table")
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(OrderedAttributesOrder::from_toml_string(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(OrderedAttributesOrder::from_toml_string(value.to_string()))
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(OrderedAttributesOrder::from_toml_string(value.to_string()))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut namespace = None;
                let mut order = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "namespace" => namespace = Some(map.next_value::<String>()?),
                        "order" => order = Some(map.next_value::<OrderedAttributesFallback>()?),
                        other => return Err(de::Error::unknown_field(other, &["namespace", "order"])),
                    }
                }

                let namespace = namespace.ok_or_else(|| de::Error::missing_field("namespace"))?;
                let order = order.unwrap_or(OrderedAttributesFallback::Alpha);

                Ok(OrderedAttributesOrder::Namespace { namespace, order })
            }
        }

        deserializer.deserialize_any(OrderedAttributesOrderVisitor)
    }
}

impl Default for OrderedAttributesConfig {
    fn default() -> Self {
        Self::Alpha { level: default_level() }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OrderedAttributesConfigInput {
    #[serde(default)]
    sort_algorithm: OrderedAttributesSortAlgorithm,
    #[serde(default = "default_level")]
    level: Level,
    order: Option<Vec<OrderedAttributesOrder>>,
    fallback: Option<OrderedAttributesFallback>,
}

impl<'de> Deserialize<'de> for OrderedAttributesConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = OrderedAttributesConfigInput::deserialize(deserializer)?;

        match input.sort_algorithm {
            OrderedAttributesSortAlgorithm::Alpha => Ok(Self::Alpha { level: input.level }),
            OrderedAttributesSortAlgorithm::Custom => {
                let order = input.order.ok_or_else(|| {
                    de::Error::custom(
                        "the custom order strategy requires providing `order` option with a list of attributes's FQNs",
                    )
                })?;

                if order.is_empty() {
                    return Err(de::Error::custom(
                        "the custom order strategy requires providing `order` option with a list of attributes's FQNs",
                    ));
                }

                Ok(Self::Custom {
                    level: input.level,
                    order,
                    fallback: input.fallback.unwrap_or_else(default_fallback),
                })
            }
        }
    }
}

#[inline]
const fn default_fallback() -> OrderedAttributesFallback {
    OrderedAttributesFallback::Preserve
}

impl Config for OrderedAttributesConfig {
    fn level(&self) -> Level {
        *match self {
            OrderedAttributesConfig::Alpha { level } => level,
            OrderedAttributesConfig::Custom { level, .. } => level,
        }
    }
}

impl LintRule for OrderedAttributesRule {
    type Config = OrderedAttributesConfig;

    fn meta() -> &'static RuleMeta {
        const META: RuleMeta = RuleMeta {
            name: "Ordered Attributes",
            code: "ordered-attributes",
            description: indoc! {r"
                Sorts attributes using the configured sort algorithm.
            "},
            good_example: indoc! {r"
                <?php

                #[Hello]
                #[World]
                class A {}
            "},
            bad_example: indoc! {r"
                <?php

                #[World]
                #[Hello]
                class A {}
            "},
            category: Category::Consistency,
            requirements: RuleRequirements::PHPVersion(PHPVersionRange::from(PHPVersion::PHP80)),
        };

        &META
    }

    fn targets() -> &'static [NodeKind] {
        const TARGETS: &[NodeKind] = &[
            NodeKind::Class,
            NodeKind::Function,
            NodeKind::AnonymousClass,
            NodeKind::Method,
            NodeKind::PlainProperty,
            NodeKind::FunctionLikeParameter,
        ];

        TARGETS
    }

    fn build(settings: &RuleSettings<Self::Config>) -> Self {
        Self { meta: Self::meta(), cfg: settings.config.clone() }
    }

    fn check<'arena>(&self, ctx: &mut LintContext<'_, 'arena>, node: Node<'_, 'arena>) {
        let attribute_lists = match node {
            Node::Class(class) => &class.attribute_lists,
            Node::Function(function) => &function.attribute_lists,
            Node::AnonymousClass(class) => &class.attribute_lists,
            Node::Method(method) => &method.attribute_lists,
            Node::PlainProperty(property) => &property.attribute_lists,
            Node::FunctionLikeParameter(parameter) => &parameter.attribute_lists,
            _ => return,
        };

        if let Some((span, fixed)) = self.reorder_attribute_lists(ctx, attribute_lists) {
            ctx.collector.report(
                Issue::new(self.cfg.level(), "Attributes are not ordered correctly.")
                    .with_code(self.meta.code)
                    .with_annotation(Annotation::primary(span).with_message("Attribute list is not ordered correctly"))
                    .with_help("Reorder the attributes to match the configured order.")
                    .with_edit(span.file_id, TextEdit::replace(span, fixed)),
            );
        }
    }
}

#[derive(Debug, Clone)]
struct OrderedAttributeGroup<'a> {
    span: Span,
    group_items: Vec<OrderedAttributesSegment<'a>>,
}

#[derive(Debug, Clone)]
struct OrderedAttributesSegment<'a> {
    original_index: usize,
    written_name: &'a str,
    text: &'a str,
    sort_key: OrderedAttributesSegmentKey,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum OrderedAttributesSegmentKey {
    Exact { order: usize },
    Namespace { order: usize, sort: OrderedAttributesFallback },
    Fallback { sort: OrderedAttributesFallback },
}

impl OrderedAttributesRule {
    fn reorder_attribute_lists<'arena>(
        &self,
        ctx: &LintContext<'_, 'arena>,
        attribute_lists: &Sequence<'arena, AttributeList<'arena>>,
    ) -> Option<(Span, String)> {
        if attribute_lists.is_empty() {
            return None;
        }

        let source = ctx.source_file.contents.as_ref();
        let mut groups = Vec::with_capacity(attribute_lists.len());
        let mut items = Vec::new();
        let mut next_index = 0usize;

        for attribute_list in attribute_lists {
            if attribute_list.attributes.is_empty() {
                continue;
            }

            let middle_start = attribute_list.hash_left_bracket.end.offset as usize;
            let middle_end = attribute_list.right_bracket.start.offset as usize;
            let mut previous_end = middle_start;
            let mut group_items = Vec::with_capacity(attribute_list.attributes.len());

            for (_index, attribute, separator) in attribute_list.attributes.iter_with_tokens() {
                let separator_start = separator.map_or(middle_end, |token| token.start.offset as usize);
                let separator_end = separator.map_or(middle_end, |token| token.start.offset as usize + token.value.len());

                let written_name = attribute.name.value();
                let resolved_name = Self::normalize_fqn(ctx.lookup_name(&attribute.name));

                let item = OrderedAttributesSegment {
                    original_index: next_index,
                    written_name,
                    text: &source[previous_end..separator_start],
                    sort_key: self.segment_sort_key(resolved_name),
                };
                next_index += 1;
                previous_end = separator_end;

                items.push(item.clone());
                group_items.push(item);
            }

            groups.push(OrderedAttributeGroup { span: attribute_list.span(), group_items });
        }

        if items.len() < 2 {
            return None;
        }

        let mut sorted_items = items.clone();
        sorted_items.sort_by(|a, b| self.compare_segments(a, b));

        if items.iter().map(|item| item.original_index).eq(sorted_items.iter().map(|item| item.original_index)) {
            return None;
        }

        let first_group = groups.first()?;
        let last_group = groups.last()?;
        let combined_span = Span::new(first_group.span.file_id, first_group.span.start, last_group.span.end);

        let mut fixed = String::new();
        let mut cursor = combined_span.start.offset as usize;
        let mut sorted_offset = 0usize;

        for group in &groups {
            fixed.push_str(&source[cursor..group.span.start.offset as usize]);

            let group_len = group.group_items.len();
            let replacement_items = &sorted_items[sorted_offset..sorted_offset + group_len];
            sorted_offset += group_len;

            fixed.push_str(&source[group.span.start.offset as usize..group.span.start.offset as usize + 2]);
            for (i, item) in replacement_items.iter().enumerate() {
                fixed.push_str(Self::normalize_segment_text(item.text));
                if i + 1 != replacement_items.len() {
                    fixed.push_str(", ");
                }
            }
            fixed.push_str(&source[group.span.end.offset as usize - 1..group.span.end.offset as usize]);

            cursor = group.span.end.offset as usize;
        }

        fixed.push_str(&source[cursor..combined_span.end.offset as usize]);

        Some((combined_span, fixed))
    }

    fn compare_segments(&self, a: &OrderedAttributesSegment<'_>, b: &OrderedAttributesSegment<'_>) -> Ordering {
        match (&a.sort_key, &b.sort_key) {
            (
                OrderedAttributesSegmentKey::Exact { order: a_order },
                OrderedAttributesSegmentKey::Exact { order: b_order },
            ) => a_order.cmp(b_order).then_with(|| a.original_index.cmp(&b.original_index)),
            (OrderedAttributesSegmentKey::Namespace { order: a_order, sort: a_sort }, OrderedAttributesSegmentKey::Namespace { order: b_order, sort: b_sort }) => {
                a_order
                    .cmp(b_order)
                    .then_with(|| {
                        self.compare_inner_sort(
                            *a_sort,
                            *b_sort,
                            a.written_name,
                            b.written_name,
                            a.original_index,
                            b.original_index,
                        )
                    })
                    .then_with(|| a.original_index.cmp(&b.original_index))
            }
            (
                OrderedAttributesSegmentKey::Fallback { sort: a_sort },
                OrderedAttributesSegmentKey::Fallback { sort: b_sort },
            ) => self.compare_inner_sort(
                *a_sort,
                *b_sort,
                a.written_name,
                b.written_name,
                a.original_index,
                b.original_index,
            ),
            (OrderedAttributesSegmentKey::Exact { .. }, _) => Ordering::Less,
            (_, OrderedAttributesSegmentKey::Exact { .. }) => Ordering::Greater,
            (OrderedAttributesSegmentKey::Namespace { .. }, OrderedAttributesSegmentKey::Fallback { .. }) => {
                Ordering::Less
            }
            (OrderedAttributesSegmentKey::Fallback { .. }, OrderedAttributesSegmentKey::Namespace { .. }) => {
                Ordering::Greater
            }
        }
    }

    fn compare_inner_sort(
        &self,
        a: OrderedAttributesFallback,
        b: OrderedAttributesFallback,
        a_name: &str,
        b_name: &str,
        a_index: usize,
        b_index: usize,
    ) -> Ordering {
        match (a, b) {
            (OrderedAttributesFallback::Alpha, OrderedAttributesFallback::Alpha) => {
                a_name.cmp(b_name).then_with(|| a_index.cmp(&b_index))
            }
            (OrderedAttributesFallback::Preserve, OrderedAttributesFallback::Preserve) => a_index.cmp(&b_index),
            (OrderedAttributesFallback::Alpha, OrderedAttributesFallback::Preserve) => Ordering::Less,
            (OrderedAttributesFallback::Preserve, OrderedAttributesFallback::Alpha) => Ordering::Greater,
        }
    }

    fn segment_sort_key(&self, resolved_name: &str) -> OrderedAttributesSegmentKey {
        match &self.cfg {
            OrderedAttributesConfig::Alpha { .. } => {
                OrderedAttributesSegmentKey::Fallback { sort: OrderedAttributesFallback::Alpha }
            }
            OrderedAttributesConfig::Custom { order, fallback, .. } => {
                let normalized_resolved = Self::normalize_fqn(resolved_name);

                let mut best_namespace: Option<(usize, usize, OrderedAttributesFallback)> = None;

                for (order_index, entry) in order.iter().enumerate() {
                    match entry {
                        OrderedAttributesOrder::Class(class_name) => {
                            if Self::normalize_fqn(class_name) == normalized_resolved {
                                return OrderedAttributesSegmentKey::Exact { order: order_index };
                            }
                        }
                        OrderedAttributesOrder::Namespace { namespace, order } => {
                            let namespace = Self::normalize_fqn(namespace);
                            if normalized_resolved.starts_with(namespace) {
                                let candidate = (namespace.len(), order_index, *order);
                                if best_namespace.as_ref().is_none_or(|best| {
                                    candidate.0 > best.0 || (candidate.0 == best.0 && candidate.1 < best.1)
                                }) {
                                    best_namespace = Some(candidate);
                                }
                            }
                        }
                    }
                }

                if let Some((_, order_index, sort)) = best_namespace {
                    OrderedAttributesSegmentKey::Namespace { order: order_index, sort }
                } else {
                    OrderedAttributesSegmentKey::Fallback { sort: *fallback }
                }
            }
        }
    }

    fn normalize_fqn(value: &str) -> &str {
        value.trim_start_matches('\\')
    }

    fn normalize_segment_text(value: &str) -> &str {
        value.trim_matches(|c| c == ' ' || c == '\t')
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use mago_reporting::Level;

    use crate::rule::OrderedAttributesConfig;
    use crate::rule::OrderedAttributesFallback;
    use crate::rule::OrderedAttributesOrder;
    use crate::rule::consistency::OrderedAttributesRule;
    use crate::settings::Settings;
    use crate::test_lint_fix;
    use crate::test_lint_success;

    #[test]
    fn parses_toml_custom_order_with_namespace_entry() {
        let config = toml::from_str::<OrderedAttributesConfig>(
            r#"
            sort_algorithm = "custom"
            order = [
              "OpenApi\\Attributes\\Response",
              { namespace = "OpenApi\\Attributes\\", order = "alpha" },
            ]
            fallback = "preserve"
            "#,
        )
        .expect("config should parse");

        let OrderedAttributesConfig::Custom { order, fallback, .. } = config else {
            panic!("expected custom config");
        };

        assert_eq!(fallback, OrderedAttributesFallback::Preserve);
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn parses_toml_namespace_shorthand_string() {
        let config = toml::from_str::<OrderedAttributesConfig>(
            r#"
            sort_algorithm = "custom"
            order = ["OpenApi\\Attributes\\Response", "OpenApi\\Attributes\\"]
            "#,
        )
        .expect("config should parse");

        let OrderedAttributesConfig::Custom { order, .. } = config else {
            panic!("expected custom config");
        };

        assert!(matches!(order[1], OrderedAttributesOrder::Namespace { .. }));
    }

    fn custom_namespace_sort_settings(
        order: Vec<OrderedAttributesOrder>,
        fallback: OrderedAttributesFallback,
    ) -> impl FnOnce(&mut Settings) {
        move |settings| {
            settings.rules.ordered_attributes.config =
                OrderedAttributesConfig::Custom { level: Level::Note, order, fallback };
        }
    }

    test_lint_success! {
        name = alpha_already_sorted_is_ignored,
        rule = OrderedAttributesRule,
        code = indoc! {r#"
            <?php

            #[Bar, Foo, Qux]
            class Demo {}
        "#}
    }

    test_lint_fix! {
        name = alpha_sorts_across_attribute_lists,
        rule = OrderedAttributesRule,
        code = indoc! {r#"
            <?php

            #[Qux]
            #[Foo]
            #[Bar]
            class Demo {}
        "#},
        fixed = indoc! {r#"
            <?php

            #[Bar]
            #[Foo]
            #[Qux]
            class Demo {}
        "#}
    }

    test_lint_fix! {
        name = alpha_sorts_attributes_by_written_name,
        rule = OrderedAttributesRule,
        code = indoc! {r#"
            <?php

            #[Qux, Foo, Bar]
            class Demo {}
        "#},
        fixed = indoc! {r#"
            <?php

            #[Bar, Foo, Qux]
            class Demo {}
        "#}
    }

    test_lint_fix! {
        name = exact_class_before_namespace_with_alpha_fallback,
        rule = OrderedAttributesRule,
        settings = custom_namespace_sort_settings(
            vec![
                "Doctrine\\ORM\\Mapping\\Id".into(),
                OrderedAttributesOrder::Namespace {
                    namespace: "Doctrine\\ORM\\Mapping\\".to_string(),
                    order: OrderedAttributesFallback::Alpha,
                },
            ],
            OrderedAttributesFallback::Alpha,
        ),
        code = indoc! {r#"
            <?php

            use Doctrine\ORM\Mapping as ORM;

            #[ORM\Column, ORM\Id, ORM\JoinColumn, App\IgnoreMe]
            function f() {}
        "#},
        fixed = indoc! {r#"
            <?php

            use Doctrine\ORM\Mapping as ORM;

            #[ORM\Id, ORM\Column, ORM\JoinColumn, App\IgnoreMe]
            function f() {}
        "#}
    }

    test_lint_fix! {
        name = namespace_bucket_preserve_order,
        rule = OrderedAttributesRule,
        settings = custom_namespace_sort_settings(
            vec![
                "Doctrine\\ORM\\Mapping\\Id".into(),
                OrderedAttributesOrder::Namespace {
                    namespace: "Doctrine\\ORM\\Mapping\\".to_string(),
                    order: OrderedAttributesFallback::Preserve,
                },
            ],
            OrderedAttributesFallback::Preserve,
        ),
        code = indoc! {r#"
            <?php

            use Doctrine\ORM\Mapping as ORM;

            #[ORM\JoinColumn, ORM\Id, ORM\GeneratedValue, ORM\Column]
            function f() {}
        "#},
        fixed = indoc! {r#"
            <?php

            use Doctrine\ORM\Mapping as ORM;

            #[ORM\Id, ORM\JoinColumn, ORM\GeneratedValue, ORM\Column]
            function f() {}
        "#}
    }

    test_lint_fix! {
        name = unmatched_fallback_alpha_orders_leftovers,
        rule = OrderedAttributesRule,
        settings = custom_namespace_sort_settings(
            vec![
                "Doctrine\\ORM\\Mapping\\Id".into(),
                OrderedAttributesOrder::Namespace {
                    namespace: "Doctrine\\ORM\\Mapping\\".to_string(),
                    order: OrderedAttributesFallback::Alpha,
                },
            ],
            OrderedAttributesFallback::Alpha,
        ),
        code = indoc! {r#"
            <?php

            use Doctrine\ORM\Mapping as ORM;

            #[App\Zulu, ORM\Column, ORM\Id, App\Alpha, ORM\GeneratedValue]
            function f() {}
        "#},
        fixed = indoc! {r#"
            <?php

            use Doctrine\ORM\Mapping as ORM;

            #[ORM\Id, ORM\Column, ORM\GeneratedValue, App\Alpha, App\Zulu]
            function f() {}
        "#}
    }

    test_lint_fix! {
        name = comments_and_docblocks_move_with_attributes,
        rule = OrderedAttributesRule,
        settings = custom_namespace_sort_settings(
            vec![
                "App\\Important".into(),
                OrderedAttributesOrder::Namespace {
                    namespace: "App\\".to_string(),
                    order: OrderedAttributesFallback::Alpha,
                },
            ],
            OrderedAttributesFallback::Preserve,
        ),
        code = indoc! {r#"
            <?php

            #[/* first */\App\Gamma, /** important */\App\Important, /* second */\App\Beta]
            function f() {}
        "#},
        fixed = indoc! {r#"
            <?php

            #[/** important */\App\Important, /* second */\App\Beta, /* first */\App\Gamma]
            function f() {}
        "#}
    }
}
