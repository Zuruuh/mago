use mago_span::HasSpan;
use mago_span::Span;
use mago_syntax::ast::*;

use crate::document::Document;
use crate::document::Group;
use crate::document::IndentIfBreak;
use crate::document::Line;
use crate::document::Separator;
use crate::internal::FormatterState;
use crate::internal::comment::CommentFlags;
use crate::internal::format::Format;
use crate::internal::format::member_access::collect_member_access_chain;
use crate::internal::format::statement::print_statement_sequence;
use crate::settings::BraceStyle;

use super::block::block_is_empty;

pub(super) fn has_new_line_in_range(text: &str, start: usize, end: usize) -> bool {
    text[start..end].contains('\n')
}

/// Determines whether an expression can be "hugged" within brackets without line breaks.
///
/// # Overview
///
/// A "huggable" expression can be formatted compactly within parentheses `()` or square brackets `[]`
/// without requiring additional line breaks or indentation. This means the expression can be
/// rendered on the same line as the opening and closing brackets.
///
/// # Hugging Rules
///
/// 1. Nested expressions are recursively checked
/// 2. Expressions with leading or trailing comments cannot be hugged
/// 3. Specific expression types are considered huggable
///
/// # Supported Huggable Expressions
///
/// - Arrays
/// - Legacy Arrays
/// - Lists
/// - Closures
/// - Closure Creations
/// - Function Calls
/// - Anonymous Classes
/// - Match Expressions
///
/// # Parameters
///
/// - `f`: The formatter context
/// - `expression`: The expression to check for hugging potential
///
/// # Returns
///
/// `true` if the expression can be formatted compactly, `false` otherwise
///
/// # Performance
///
/// O(1) for most checks, with potential O(n) recursion for nested expressions
pub(super) fn should_hug_expression<'a>(
    f: &FormatterState<'a>,
    expression: &'a Expression,
    arrow_function_recursion: bool,
) -> bool {
    if let Expression::Parenthesized(inner) = expression {
        return should_hug_expression(f, &inner.expression, arrow_function_recursion);
    }

    if let Expression::UnaryPrefix(operation) = expression {
        return should_hug_expression(f, &operation.operand, arrow_function_recursion);
    }

    // if the expression has leading or trailing comments, we can't hug it
    if f.has_comment(expression.span(), CommentFlags::Leading | CommentFlags::Trailing) {
        return false;
    }

    if let Expression::Call(_) | Expression::Access(_) = expression {
        // Don't hug calls/accesses if they are part of a member access chain
        return collect_member_access_chain(expression).is_none_or(|chain| !chain.is_eligible_for_chaining(f));
    }

    if let Expression::ArrowFunction(arrow_function) = expression {
        return !arrow_function_recursion && should_hug_expression(f, &arrow_function.expression, true);
    }

    if let Expression::Binary(binary) = expression {
        let is_left_hand_side_simple = is_simple_expression(&binary.lhs);
        let is_right_hand_side_simple = is_simple_expression(&binary.rhs);

        // Hug binary expressions if they are simple and not too complex
        if is_left_hand_side_simple && is_right_hand_side_simple {
            return true;
        }

        if binary.operator.is_concatenation() {
            return (is_left_hand_side_simple && should_hug_expression(f, &binary.rhs, arrow_function_recursion))
                || (is_right_hand_side_simple && should_hug_expression(f, &binary.lhs, arrow_function_recursion));
        }

        return false;
    }

    let Expression::Instantiation(instantiation) = expression else {
        return matches!(
            expression,
            Expression::Array(_)
                | Expression::LegacyArray(_)
                | Expression::List(_)
                | Expression::Closure(_)
                | Expression::ClosureCreation(_)
                | Expression::AnonymousClass(_)
                | Expression::Match(_)
        );
    };

    // Hug instantiations if it is a simple class instantiation
    let Expression::Identifier(_) = instantiation.class.as_ref() else {
        return false;
    };

    // And either:
    match &instantiation.argument_list {
        // a. The instantiation is a simple class instantiation without arguments
        None => true,
        Some(argument_list) => {
            let arguments_len = argument_list.arguments.len();
            if 0 == arguments_len {
                false
            } else if arguments_len == 1 {
                // b. The instantiation has a single non-named argument that is huggable or an instantiation
                //   (e.g. `new Foo(new Bar())`)
                match &argument_list.arguments.as_slice()[0] {
                    Argument::Named(_) => false,
                    Argument::Positional(positional) => {
                        matches!(positional.value, Expression::Instantiation(_))
                            || should_hug_expression(f, &positional.value, arrow_function_recursion)
                    }
                }
            } else {
                // c. The instantiation has multiple arguments and all are named.
                argument_list.arguments.iter().all(|arg| matches!(arg, Argument::Named(_))) ||
                // d. The instantiation has less than 4 non-named arguments,
                // all of which are simple expressions
                (arguments_len < 4 && argument_list.arguments.iter().all(|arg| {
                    matches!(arg, Argument::Positional(positional) if is_simple_expression(&positional.value))
                }))
            }
        }
    }
}

pub fn is_simple_expression(node: &Expression) -> bool {
    if let Expression::Parenthesized(inner) = node {
        return is_simple_expression(&inner.expression);
    }

    if let Expression::UnaryPrefix(operation) = node {
        return is_simple_expression(&operation.operand);
    }

    if let Expression::Binary(operation) = node {
        return is_simple_expression(&operation.lhs) && is_simple_expression(&operation.rhs);
    }

    matches!(
        node,
        Expression::Static(_)
            | Expression::Parent(_)
            | Expression::Self_(_)
            | Expression::MagicConstant(_)
            | Expression::Literal(_)
            | Expression::Identifier(_)
            | Expression::ConstantAccess(_)
            | Expression::Variable(_)
            | Expression::Access(Access::ClassConstant(_))
    )
}

#[inline]
pub(super) const fn is_string_word_type(node: &Expression) -> bool {
    matches!(
        node,
        Expression::Static(_)
            | Expression::Parent(_)
            | Expression::Self_(_)
            | Expression::MagicConstant(_)
            | Expression::Identifier(Identifier::Local(_))
            | Expression::ConstantAccess(ConstantAccess { name: Identifier::Local(_) })
            | Expression::Variable(Variable::Direct(_))
    )
}

pub(super) fn print_colon_delimited_body<'a>(
    f: &mut FormatterState<'a>,
    colon: &'a Span,
    statements: &'a Sequence<Statement>,
    end_keyword: &'a Keyword,
    terminator: &'a Terminator,
) -> Document<'a> {
    let mut parts = vec![Document::String(":")];

    let mut printed_statements = print_statement_sequence(f, statements);
    if !printed_statements.is_empty() {
        if let Some(Statement::ClosingTag(_)) = statements.first() {
            printed_statements.insert(0, Document::String(" "));
            parts.push(Document::Array(printed_statements));
        } else {
            printed_statements.insert(0, Document::Line(Line::hard()));
            parts.push(Document::Indent(printed_statements));
        }
    }

    if let Some(comments) = f.print_dangling_comments(colon.join(terminator.span()), true) {
        parts.push(comments);
    } else if !matches!(statements.last(), Some(Statement::OpeningTag(_))) {
        parts.push(Document::Line(Line::hard()));
    } else {
        parts.push(Document::String(" "));
    }

    parts.push(end_keyword.format(f));
    parts.push(terminator.format(f));

    Document::Group(Group::new(parts).with_break(true))
}

pub(super) fn print_modifiers<'a>(f: &mut FormatterState<'a>, modifiers: &'a Sequence<Modifier>) -> Vec<Document<'a>> {
    let mut printed_modifiers = vec![];

    if let Some(modifier) = modifiers.get_final() {
        printed_modifiers.push(modifier.format(f));
    }

    if let Some(modifier) = modifiers.get_abstract() {
        printed_modifiers.push(modifier.format(f));
    }

    if f.settings.static_before_visibility {
        if let Some(modifier) = modifiers.get_static() {
            printed_modifiers.push(modifier.format(f));
        }

        if let Some(modifier) = modifiers.get_readonly() {
            printed_modifiers.push(modifier.format(f));
        }

        if let Some(modifier) = modifiers.get_first_read_visibility() {
            printed_modifiers.push(modifier.format(f));
        }

        if let Some(modifier) = modifiers.get_first_write_visibility() {
            printed_modifiers.push(modifier.format(f));
        }
    } else {
        if let Some(modifier) = modifiers.get_first_read_visibility() {
            printed_modifiers.push(modifier.format(f));
        }

        if let Some(modifier) = modifiers.get_first_write_visibility() {
            printed_modifiers.push(modifier.format(f));
        }

        if let Some(modifier) = modifiers.get_static() {
            printed_modifiers.push(modifier.format(f));
        }

        if let Some(modifier) = modifiers.get_readonly() {
            printed_modifiers.push(modifier.format(f));
        }
    }

    Document::join(printed_modifiers, Separator::Space)
}

pub(super) fn print_attribute_list_sequence<'a>(
    f: &mut FormatterState<'a>,
    attribute_lists: &'a Sequence<AttributeList>,
) -> Option<Document<'a>> {
    if attribute_lists.is_empty() {
        return None;
    }

    let mut lists = vec![];
    let mut has_new_line = false;
    let mut has_potentially_long_attribute = false;
    for attribute_list in attribute_lists.iter() {
        if !has_potentially_long_attribute {
            for attribute in attribute_list.attributes.iter() {
                has_potentially_long_attribute =
                    !attribute.argument_list.as_ref().is_none_or(|args| args.arguments.is_empty());

                if has_potentially_long_attribute {
                    break;
                }
            }
        }

        lists.push(attribute_list.format(f));

        has_new_line = has_new_line || f.is_next_line_empty(attribute_list.span());
    }

    let mut contents = vec![];
    let len = lists.len();
    for (i, attribute_list) in lists.into_iter().enumerate() {
        contents.push(attribute_list);

        if i != len - 1 {
            contents.push(Document::Line(Line::hard()));
        }
    }

    Some(Document::Group(Group::new(contents)))
}

pub(super) fn print_clause<'a>(f: &mut FormatterState<'a>, node: &'a Statement, force_space: bool) -> Document<'a> {
    let clause = node.format(f);

    adjust_clause(f, node, clause, force_space)
}

pub(super) fn adjust_clause<'a>(
    f: &mut FormatterState<'a>,
    node: &'a Statement,
    clause: Document<'a>,
    mut force_space: bool,
) -> Document<'a> {
    let mut is_block = false;

    let has_trailing_segment = match f.current_node() {
        Node::IfStatementBody(b) => b.else_clause.is_some() || !b.else_if_clauses.is_empty(),
        Node::IfStatementBodyElseClause(_) => {
            if let Statement::If(_) = node {
                force_space = true;
            }

            false
        }
        Node::IfStatementBodyElseIfClause(c) => {
            if let Node::IfStatementBody(b) = f.parent_node() {
                b.else_clause.is_some()
                    || b.else_if_clauses.iter().any(|clause| clause.span().start.offset >= c.span().end.offset)
            } else {
                false
            }
        }
        Node::DoWhile(_) => true,
        _ => false,
    };

    let clause = match node {
        Statement::Noop(_) => clause,
        Statement::Block(block) => {
            is_block = true;

            let is_block_empty = block_is_empty(f, &block.left_brace, &block.right_brace);
            match f.settings.control_brace_style {
                BraceStyle::SameLine => Document::Array(vec![Document::space(), clause]),
                BraceStyle::NextLine => {
                    if f.settings.inline_empty_control_braces && is_block_empty {
                        Document::Array(vec![Document::space(), clause])
                    } else {
                        Document::Array(vec![Document::Line(Line::default()), clause])
                    }
                }
            }
        }
        _ => {
            if force_space {
                Document::Array(vec![Document::space(), clause])
            } else {
                Document::Indent(vec![Document::BreakParent, Document::Line(Line::hard()), clause])
            }
        }
    };

    if has_trailing_segment {
        if is_block {
            Document::Array(vec![clause, Document::space()])
        } else {
            Document::Indent(vec![Document::BreakParent, clause, Document::Line(Line::hard())])
        }
    } else {
        clause
    }
}

pub(super) fn print_condition<'a>(
    f: &mut FormatterState<'a>,
    condition: &'a Expression,
    space_before: bool,
    space_within: bool,
) -> Document<'a> {
    let was_in_condition = f.in_condition;
    f.in_condition = true;

    let condition = Document::Group(Group::new(vec![
        if space_before { Document::space() } else { Document::empty() },
        Document::String("("),
        Document::IndentIfBreak(IndentIfBreak::new(vec![
            Document::Line(if space_within { Line::default() } else { Line::soft() }),
            condition.format(f),
        ])),
        Document::Line(if space_within { Line::default() } else { Line::soft() }),
        Document::String(")"),
    ]));

    f.in_condition = was_in_condition;

    condition
}
