use mago_php_version::feature::Feature;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use mago_syntax::token::GetPrecedence;
use mago_syntax::token::Precedence;

use crate::document::Document;
use crate::document::Group;
use crate::document::Line;
use crate::internal::FormatterState;
use crate::internal::binaryish::should_flatten;

impl<'a> FormatterState<'a> {
    pub(crate) fn wrap_parens(&mut self, document: Document<'a>, node: Node<'a>) -> Document<'a> {
        if self.need_parens(node) {
            if self.should_indent(node) {
                Document::Group(Group::new(vec![
                    Document::String("("),
                    Document::Indent(vec![
                        if self.settings.space_within_grouping_parenthesis {
                            Document::Line(Line::default())
                        } else {
                            Document::Line(Line::soft())
                        },
                        document,
                    ]),
                    if self.settings.space_within_grouping_parenthesis {
                        Document::Line(Line::default())
                    } else {
                        Document::Line(Line::soft())
                    },
                    Document::String(")"),
                ]))
            } else {
                Document::Group(Group::new(vec![
                    Document::String("("),
                    if self.settings.space_within_grouping_parenthesis { Document::space() } else { Document::empty() },
                    document,
                    if self.settings.space_within_grouping_parenthesis { Document::space() } else { Document::empty() },
                    Document::String(")"),
                ]))
            }
        } else {
            document
        }
    }

    fn should_indent(&self, node: Node<'a>) -> bool {
        if matches!(node, Node::Program(_)) || node.is_statement() {
            return false;
        }

        self.is_unary_or_binary_or_ternary(node)
    }

    fn need_parens(&mut self, node: Node<'a>) -> bool {
        if matches!(node, Node::Program(_)) || node.is_statement() {
            return false;
        }

        self.called_or_accessed_node_needs_parenthesis(node)
            || self.binary_node_needs_parens(node)
            || self.unary_prefix_node_needs_parens(node)
            || self.conditional_or_assignment_needs_parenthesis(node)
            || self.literal_needs_parens(node)
            || self.pipe_node_needs_parens(node)
    }

    fn literal_needs_parens(&self, node: Node<'a>) -> bool {
        let Node::Literal(Literal::Integer(_) | Literal::Float(_)) = node else {
            return false;
        };

        if let Some(Node::Binary(binary)) = self.nth_parent_kind(2)
            && let BinaryOperator::StringConcat(_) = binary.operator
        {
            return true;
        }

        false
    }

    fn conditional_or_assignment_needs_parenthesis(&self, node: Node<'a>) -> bool {
        if !matches!(node, Node::Assignment(_) | Node::Conditional(_)) {
            return false;
        }

        let Some(parent_node) = self.nth_parent_kind(2) else {
            return false;
        };

        if let Node::ArrowFunction(_) = parent_node {
            return matches!(self.nth_parent_kind(3), Some(Node::Pipe(_)));
        }

        self.is_unary_or_binary_or_ternary(parent_node) || matches!(parent_node, Node::VariadicArrayElement(_))
    }

    fn pipe_node_needs_parens(&self, node: Node<'a>) -> bool {
        let Node::Pipe(_) = node else {
            return false;
        };

        match self.nth_parent_kind(2) {
            Some(Node::Binary(e)) => {
                let precedence = e.operator.precedence();

                if precedence >= Precedence::Pipe {
                    return true;
                }

                false
            }
            Some(Node::Assignment(_)) => false,
            Some(Node::UnaryPrefix(_) | Node::UnaryPostfix(_)) => true,
            Some(Node::VariadicArrayElement(_)) => true,
            Some(Node::ArrayAppend(_)) => true,
            Some(Node::Conditional(_)) => true,
            _ => false,
        }
    }

    fn binary_node_needs_parens(&self, node: Node<'a>) -> bool {
        let operator = match node {
            Node::Binary(e) => &e.operator,
            _ => return false,
        };

        let parent_operator = match self.nth_parent_kind(2) {
            Some(Node::VariadicArrayElement(_)) => {
                return true;
            }
            Some(Node::Binary(e)) => {
                if operator.is_low_precedence() {
                    return true;
                }

                if let BinaryOperator::NullCoalesce(_) = e.operator {
                    // Add parentheses if parent is a coalesce operator,
                    //  unless the child is a coalesce operator as well.
                    return !matches!(operator, BinaryOperator::NullCoalesce(_));
                }

                if let BinaryOperator::Instanceof(_) = e.operator {
                    // Add parentheses if parent is an instanceof operator.
                    return true;
                }

                if let BinaryOperator::Elvis(_) = e.operator {
                    // Add parentheses if parent is an elvis operator.
                    return true;
                }

                if let BinaryOperator::StringConcat(_) = e.operator {
                    // Add parentheses if parent is a string concat operator,
                    //  unless the child is a string concat operator as well.
                    return !matches!(operator, BinaryOperator::StringConcat(_));
                }

                &e.operator
            }
            Some(Node::Pipe(_)) => {
                let precedence = operator.precedence();
                return precedence <= Precedence::Pipe;
            }
            Some(Node::ArrowFunction(_)) => {
                let grand_parent_node = self.nth_parent_kind(3);
                if let Some(Node::Pipe(_)) = grand_parent_node {
                    return true;
                }

                return false;
            }
            Some(Node::UnaryPrefix(_) | Node::UnaryPostfix(_)) => {
                // Add parentheses if parent is an unary operator.
                return true;
            }
            Some(Node::Conditional(_)) => {
                if (operator.is_logical() && !operator.is_low_precedence()) || operator.is_comparison() {
                    return false;
                }

                return true;
            }
            Some(Node::ArrayAppend(_)) => {
                return true;
            }
            Some(Node::ArrayAccess(access)) => {
                // we add parentheses if the parent is an array access and the child is a binaryish node
                //
                // Example:
                //
                // ```php
                // ($foo ?? $bar)[$baz];
                // ```
                //
                // requires parentheses, if we remove them, the code will be interpreted as:
                //
                // ```php
                // $foo ?? ($bar[$baz]);
                // ```
                return access.left_bracket.start.offset > node.span().start.offset;
            }
            parent => {
                if let Some(Node::Assignment(_)) = parent
                    && operator.is_low_precedence()
                {
                    return true;
                }

                let grand_parent_node = self.nth_parent_kind(3);

                if let Some(Node::Access(_)) = grand_parent_node {
                    return true;
                } else {
                    return false;
                }
            }
        };

        if operator.is_bit_shift() || operator.is_low_precedence() {
            return true;
        }

        if parent_operator.is_comparison() {
            return true;
        }

        if parent_operator.is_bitwise() {
            return !operator.is_same_as(parent_operator);
        }

        if operator.is_comparison() {
            return !parent_operator.is_logical();
        }

        // Add parentheses if operators have different precedence
        let precedence = operator.precedence();
        let parent_precedence = parent_operator.precedence();
        if parent_precedence > precedence {
            return true;
        }

        if operator.is_arithmetic() && parent_operator.is_arithmetic() && !operator.is_same_as(parent_operator) {
            return true;
        }

        if parent_precedence < precedence {
            return false;
        }

        if !should_flatten(operator, parent_operator) {
            return true;
        }

        false
    }

    fn unary_prefix_node_needs_parens(&self, node: Node<'a>) -> bool {
        let operator = match node {
            Node::UnaryPrefix(e) => &e.operator,
            _ => return false,
        };

        if operator.is_error_control() {
            let Some(parent_node) = self.nth_parent_kind(2) else {
                return false;
            };

            let Node::Binary(binary) = parent_node else {
                return false;
            };

            return node.span().end.offset < binary.operator.span().start.offset;
        }

        if operator.is_cast() {
            let Some(parent_node) = self.nth_parent_kind(2) else {
                return false;
            };

            return self.is_unary_or_binary_or_ternary(parent_node);
        }

        false
    }

    fn called_or_accessed_node_needs_parenthesis(&self, node: Node<'a>) -> bool {
        let Node::Expression(expression) = node else {
            return false;
        };

        if let Some(Node::ClosureCreation(closure)) = self.grandparent_node() {
            if let ClosureCreation::Function(_) = closure {
                return self.function_callee_expression_need_parenthesis(expression);
            }

            return self.callee_expression_need_parenthesis(expression, false);
        }

        if let Node::Call(call) = self.parent_node() {
            if let Call::Function(_) = call {
                return self.function_callee_expression_need_parenthesis(expression);
            }

            if let Expression::Instantiation(instantiation) = expression {
                return instantiation_needs_parens(self, instantiation);
            } else {
                return self.callee_expression_need_parenthesis(expression, false);
            }
        }

        if let Node::Instantiation(_) = self.parent_node() {
            return self.callee_expression_need_parenthesis(expression, true);
        }

        if let Node::ArrayAccess(access) = self.parent_node() {
            return if expression.span().end.offset < access.left_bracket.start.offset {
                self.callee_expression_need_parenthesis(expression, false)
            } else {
                false
            };
        }

        if let Some(Node::Access(access)) = self.grandparent_node() {
            let offset = match access {
                Access::Property(property_access) => property_access.arrow.start.offset,
                Access::NullSafeProperty(null_safe_property_access) => {
                    null_safe_property_access.question_mark_arrow.start.offset
                }
                Access::StaticProperty(static_property_access) => static_property_access.double_colon.start.offset,
                Access::ClassConstant(class_constant_access) => class_constant_access.double_colon.start.offset,
            };

            return if expression.span().end.offset < offset {
                self.callee_expression_need_parenthesis(expression, false)
            } else {
                false
            };
        }

        false
    }

    const fn callee_expression_need_parenthesis(&self, expression: &'a Expression, instantiation: bool) -> bool {
        if instantiation && matches!(expression, Expression::Call(_)) {
            return true;
        }

        if let Expression::Construct(construct) = expression {
            return !construct.has_bounds();
        }

        !matches!(
            expression,
            Expression::Literal(_)
                | Expression::Array(_)
                | Expression::LegacyArray(_)
                | Expression::ArrayAccess(_)
                | Expression::Variable(_)
                | Expression::Identifier(_)
                | Expression::ConstantAccess(_)
                | Expression::Call(_)
                | Expression::Access(_)
                | Expression::ClosureCreation(_)
                | Expression::Static(_)
                | Expression::Self_(_)
                | Expression::Parent(_)
        )
    }

    const fn function_callee_expression_need_parenthesis(&self, expression: &'a Expression) -> bool {
        !matches!(
            expression,
            Expression::Literal(_)
                | Expression::Array(_)
                | Expression::LegacyArray(_)
                | Expression::ArrayAccess(_)
                | Expression::Variable(_)
                | Expression::Identifier(_)
                | Expression::Construct(_)
                | Expression::Call(_)
                | Expression::ClosureCreation(_)
                | Expression::Static(_)
                | Expression::Self_(_)
                | Expression::Parent(_)
        )
    }

    const fn is_unary_or_binary_or_ternary(&self, node: Node<'a>) -> bool {
        self.is_unary(node) || self.is_binaryish(node) || self.is_conditional(node)
    }

    const fn is_binaryish(&self, node: Node<'a>) -> bool {
        match node {
            Node::Binary(_) => true,
            Node::Conditional(conditional) => conditional.then.is_none(),
            Node::Pipe(_) => true,
            _ => false,
        }
    }

    const fn is_unary(&self, node: Node<'a>) -> bool {
        matches!(node, Node::UnaryPrefix(_) | Node::UnaryPostfix(_))
    }

    const fn is_conditional(&self, node: Node<'a>) -> bool {
        if let Node::Conditional(op) = node { op.then.is_some() } else { false }
    }
}

pub(crate) fn instantiation_needs_parens(f: &FormatterState<'_>, i: &Instantiation) -> bool {
    if f.php_version.is_supported(Feature::NewWithoutParentheses) {
        if i.argument_list.as_ref().is_none_or(|list| list.arguments.is_empty()) {
            if f.settings.parentheses_in_new_expression {
                f.settings.parentheses_around_new_in_member_access
            } else {
                true
            }
        } else {
            f.settings.parentheses_around_new_in_member_access
        }
    } else {
        true
    }
}
