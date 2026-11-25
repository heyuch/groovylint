//! `CodeNarc` DRY rules for duplicated literals.

use std::collections::BTreeSet;

use groovylint_core::Severity;
use groovylint_syntax::ast::{Expr, ExprKind};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};

/// Flags constant collection literals that appear more than once in a file.
///
/// `CodeNarc` only considers literals made up entirely of constants and skips
/// test sources, so the same restrictions apply here.
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
        // `CodeNarc` does not apply the DRY rules to test sources.
        if crate::rules::util::is_test_file(context.source.path().to_string_lossy().as_ref()) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let matches = match (&expr.kind, self.is_map) {
            (ExprKind::List(items), false) => !items.is_empty(),
            (ExprKind::Map(entries), true) => !entries.is_empty(),
            _ => false,
        };
        // Named arguments of a call are a map literal in Groovy's AST.
        let named_span = if self.is_map {
            named_arguments(expr)
        } else {
            None
        };
        if !matches && named_span.is_none() {
            return;
        }
        let Some(text) = literal_text(expr) else {
            return;
        };
        let span = named_span.unwrap_or(expr.span);
        if !self.seen.insert(text.clone()) {
            let kind = if self.is_map { "Map" } else { "List" };
            sink.report(span, format!("Duplicate {kind} literal: {text}"));
        }
    }
}

/// Returns the span covered by the named arguments of a call, if any.
fn named_arguments(expr: &Expr) -> Option<groovylint_core::Span> {
    let (ExprKind::Call { args, .. } | ExprKind::New { args, .. }) = &expr.kind else {
        return None;
    };
    let mut first: Option<groovylint_core::Span> = None;
    let mut last = None;
    for arg in args {
        if !matches!(arg.kind, ExprKind::NamedArg { .. }) {
            continue;
        }
        first.get_or_insert(arg.span);
        last = Some(arg.span);
    }
    Some(first?.cover(last?))
}

/// Renders a collection literal of constants as a canonical key.
///
/// Only literals (including nested lists and maps of literals) qualify, which is
/// what `CodeNarc`'s `isListLiteralWithOnlyConstantValues` checks. Values are
/// compared rather than source text, so `['a']` and `["a"]` are the same literal.
fn literal_text(expr: &Expr) -> Option<String> {
    let mut out = String::new();
    if render_constant(expr, &mut out) {
        Some(out)
    } else {
        None
    }
}

fn render_constant(expr: &Expr, out: &mut String) -> bool {
    use groovylint_syntax::ast::Literal;
    match &expr.kind {
        ExprKind::Literal(Literal::String(text)) => {
            out.push('"');
            out.push_str(text);
            out.push('"');
            true
        }
        ExprKind::Literal(Literal::Regex(text)) => {
            out.push('/');
            out.push_str(text);
            out.push('/');
            true
        }
        ExprKind::Literal(Literal::Number(text)) => {
            out.push_str(text);
            true
        }
        ExprKind::Literal(Literal::Boolean(value)) => {
            out.push_str(if *value { "true" } else { "false" });
            true
        }
        ExprKind::Literal(Literal::Null) => {
            out.push_str("null");
            true
        }
        ExprKind::List(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if !render_constant(item, out) {
                    return false;
                }
            }
            out.push(']');
            true
        }
        ExprKind::Map(entries) => {
            out.push('[');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if !render_constant(key, out) || !render_constant(value, out) {
                    return false;
                }
            }
            out.push(']');
            true
        }
        ExprKind::Paren(inner) => render_constant(inner, out),
        // Named arguments of a call form a map literal.
        ExprKind::Call { args, .. } | ExprKind::New { args, .. } => {
            let named: Vec<&Expr> = args
                .iter()
                .filter(|arg| matches!(arg.kind, ExprKind::NamedArg { .. }))
                .collect();
            if named.is_empty() {
                return false;
            }
            out.push('[');
            for (index, arg) in named.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if !render_constant(arg, out) {
                    return false;
                }
            }
            out.push(']');
            true
        }
        ExprKind::NamedArg { name, value } => {
            out.push_str(name);
            out.push('=');
            render_constant(value, out)
        }
        _ => false,
    }
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
