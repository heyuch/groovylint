//! Shared helpers for rule implementations.

use groovylint_syntax::ast::{BinaryOp, Expr, ExprKind};

use crate::rule::NodeRef;

/// Returns the source text for an expression, trimmed.
pub(crate) fn expr_text(expr: &Expr, source: &str) -> String {
    source
        .get(expr.span.range())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Removes parentheses around an expression.
pub(crate) fn unparen(mut expr: &Expr) -> &Expr {
    while let ExprKind::Paren(inner) = &expr.kind {
        expr = inner;
    }
    expr
}

/// Returns the variable name for a simple variable expression.
pub(crate) fn variable_name(expr: &Expr) -> Option<&str> {
    match &unparen(expr).kind {
        ExprKind::Variable(name) => Some(name),
        _ => None,
    }
}

/// Returns the receiver name of a call or property access, if it is a variable.
pub(crate) fn receiver_name(expr: &Expr) -> Option<&str> {
    variable_name(expr)
}

/// Matches `receiver.method(args...)` with a variable receiver and exact arity.
pub(crate) fn call_with_receiver<'a>(
    expr: &'a Expr,
    receiver: &str,
    method: &str,
    arity: usize,
) -> Option<&'a [Expr]> {
    let ExprKind::Call {
        receiver: Some(recv),
        name,
        args,
        safe: false,
    } = &expr.kind
    else {
        return None;
    };
    if name == method && args.len() == arity && receiver_name(recv) == Some(receiver) {
        Some(args)
    } else {
        None
    }
}

/// Returns whether an expression is a literal, a collection of literals, or a
/// recognized constant such as `Boolean.TRUE`.
pub(crate) fn is_constant(expr: &Expr) -> bool {
    match &unparen(expr).kind {
        ExprKind::Literal(groovylint_syntax::ast::Literal::Regex(_)) => false,
        ExprKind::Literal(_) => true,
        ExprKind::List(items) => items.iter().all(is_constant),
        ExprKind::Map(entries) => entries
            .iter()
            .all(|(key, value)| is_constant(key) && is_constant(value)),
        ExprKind::Property {
            receiver,
            name,
            safe: false,
        } => {
            matches!(variable_name(receiver), Some("Boolean"))
                && matches!(name.as_str(), "TRUE" | "FALSE")
        }
        _ => false,
    }
}

/// Returns the decoded value of a string literal, if this is one.
pub(crate) fn string_literal(expr: &Expr) -> Option<String> {
    let ExprKind::Literal(groovylint_syntax::ast::Literal::String(raw)) = &unparen(expr).kind
    else {
        return None;
    };
    decode_string(raw)
}

fn decode_string(raw: &str) -> Option<String> {
    let body = if let Some(body) = raw.strip_prefix("'''") {
        body.strip_suffix("'''")?
    } else if let Some(body) = raw.strip_prefix("\"\"\"") {
        body.strip_suffix("\"\"\"")?
    } else if let Some(body) = raw.strip_prefix('\'') {
        body.strip_suffix('\'')?
    } else {
        raw.strip_prefix('"')?.strip_suffix('"')?
    };

    let mut decoded = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }
        match chars.next()? {
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            '\\' => decoded.push('\\'),
            '\'' => decoded.push('\''),
            '"' => decoded.push('"'),
            other => {
                decoded.push('\\');
                decoded.push(other);
            }
        }
    }
    Some(decoded)
}

/// Returns whether a binary expression uses a comparison operator.
pub(crate) fn is_comparison(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
    )
}

/// Returns whether an annotation list contains a simple or qualified name.
pub(crate) fn has_annotation(annotations: &[String], name: &str) -> bool {
    annotations
        .iter()
        .any(|annotation| annotation == name || annotation.ends_with(&format!(".{name}")))
}

/// Tracks the current class name during a single AST traversal.
#[derive(Default)]
pub(crate) struct ClassStack {
    names: Vec<String>,
}

impl ClassStack {
    pub(crate) fn enter(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Class(class) = node {
            self.names.push(class.name.clone());
        }
    }

    pub(crate) fn exit(&mut self, node: NodeRef<'_>) {
        if matches!(node, NodeRef::Class(_)) {
            self.names.pop();
        }
    }

    pub(crate) fn current(&self) -> &str {
        self.names.last().map_or("null", String::as_str)
    }
}

/// Tracks whether traversal is inside a `finally` block.
#[derive(Default)]
pub(crate) struct FinallyDepth {
    depth: usize,
}

impl FinallyDepth {
    pub(crate) fn enter(&mut self, node: NodeRef<'_>) {
        if matches!(node, NodeRef::Finally(_)) {
            self.depth += 1;
        }
    }

    pub(crate) fn exit(&mut self, node: NodeRef<'_>) {
        if matches!(node, NodeRef::Finally(_)) {
            self.depth = self.depth.saturating_sub(1);
        }
    }

    pub(crate) fn is_inside(&self) -> bool {
        self.depth > 0
    }
}
