//! Rules that inspect type-coercion expressions.

use groovylint_core::Severity;
use groovylint_syntax::ast::ExprKind;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{call_with_receiver, expr_text, is_constant, string_literal, unparen};

/// Detects duplicate constant values in `[...] as Set` literals.
pub struct DuplicateSetValueRule;

impl Rule for DuplicateSetValueRule {}

impl AstRule for DuplicateSetValueRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Cast {
            expr: inner,
            type_name,
        } = &expr.kind
        else {
            return;
        };
        if !type_name
            .rsplit('.')
            .next()
            .is_some_and(|name| name.ends_with("Set"))
        {
            return;
        }
        let ExprKind::List(items) = &unparen(inner).kind else {
            return;
        };

        let mut seen = Vec::new();
        for item in items {
            if !is_constant(item) {
                continue;
            }
            let text = expr_text(item, context.source.text());
            if seen.contains(&text) {
                let value = string_literal(item).unwrap_or(text);
                let message = if matches!(
                    &unparen(item).kind,
                    ExprKind::Literal(groovylint_syntax::ast::Literal::Null)
                ) {
                    "The constant value null is duplicated in the Set literal".to_string()
                } else if string_literal(item).is_some() {
                    format!("The constant value '{value}' is duplicated in the Set literal")
                } else {
                    format!("The constant value {value} is duplicated in the Set literal")
                };
                sink.report(item.span, message);
            } else {
                seen.push(text);
            }
        }
    }
}

/// Detects `Math.random() as int` / `as long`, which always yields zero.
pub struct RandomDoubleCoercedToZeroRule;

impl Rule for RandomDoubleCoercedToZeroRule {}

impl AstRule for RandomDoubleCoercedToZeroRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Cast {
            expr: inner,
            type_name,
        } = &expr.kind
        else {
            return;
        };
        let type_name = type_name.rsplit('.').next().unwrap_or(type_name);
        if !matches!(type_name, "int" | "Integer" | "long" | "Long") {
            return;
        }
        if call_with_receiver(inner, "Math", "random", 0).is_some() {
            let article = if type_name.starts_with('i') {
                "an"
            } else {
                "a"
            };
            sink.report(
                expr.span,
                format!(
                    "Casting the result of Math.random() to {article} {type_name} always results in 0"
                ),
            );
        }
    }
}

/// Descriptors for cast rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "DuplicateSetValue",
                "org.codenarc.rule.basic.DuplicateSetValueRule",
                Category::Basic,
                Severity::Warning,
                "Checks for duplicate constant values in Set literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(DuplicateSetValueRule)),
        ),
        descriptor(
            RuleMeta::new(
                "RandomDoubleCoercedToZero",
                "org.codenarc.rule.basic.RandomDoubleCoercedToZeroRule",
                Category::Basic,
                Severity::Warning,
                "Checks for coercing Math.random() to an integer.",
            ),
            || RuleInstanceImpl::Ast(Box::new(RandomDoubleCoercedToZeroRule)),
        ),
    ]
}
