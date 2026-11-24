//! `CodeNarc` DRY rules for duplicated literals.

use std::collections::BTreeSet;

use groovylint_core::Severity;
use groovylint_syntax::ast::{Expr, ExprKind};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};

/// Flags collection literals that appear more than once in a file.
#[derive(Default)]
pub struct DuplicateLiteralRule {
    is_map: bool,
    seen: BTreeSet<String>,
}

impl Rule for DuplicateLiteralRule {}

impl AstRule for DuplicateLiteralRule {
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
        let matches = match (&expr.kind, self.is_map) {
            (ExprKind::List(items), false) => items.len() > 1 && items.iter().all(is_constant),
            (ExprKind::Map(entries), true) => {
                !entries.is_empty()
                    && entries
                        .iter()
                        .all(|(key, value)| is_constant(key) && is_constant(value))
            }
            _ => false,
        };
        if !matches {
            return;
        }
        let Some(text) = literal_text(expr, context.source.text()) else {
            return;
        };
        if !self.seen.insert(text.clone()) {
            let kind = if self.is_map { "Map" } else { "List" };
            sink.report(expr.span, format!("Duplicate {kind} literal: {text}"));
        }
    }
}

/// Returns whether an expression is a constant, so the literal could be a field.
fn is_constant(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Literal(_) => true,
        ExprKind::List(items) => items.iter().all(is_constant),
        ExprKind::Map(entries) => entries
            .iter()
            .all(|(key, value)| is_constant(key) && is_constant(value)),
        _ => false,
    }
}

fn literal_text(expr: &Expr, source: &str) -> Option<String> {
    let text = source.get(expr.span.range())?;
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return None;
    }
    Some(trimmed.to_string())
}

/// Descriptors for DRY rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "DuplicateListLiteral",
                "org.codenarc.rule.dry.DuplicateListLiteralRule",
                Category::Dry,
                Severity::Warning,
                "Checks for duplicated list literals.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(DuplicateLiteralRule {
                    is_map: false,
                    seen: BTreeSet::new(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "DuplicateMapLiteral",
                "org.codenarc.rule.dry.DuplicateMapLiteralRule",
                Category::Dry,
                Severity::Warning,
                "Checks for duplicated map literals.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(DuplicateLiteralRule {
                    is_map: true,
                    seen: BTreeSet::new(),
                }))
            },
        ),
    ]
}
