//! `CodeNarc` JDBC rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ExprKind, Item};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};

fn references_connection(type_name: &str) -> bool {
    matches!(
        type_name.rsplit('.').next().unwrap_or(type_name),
        "Connection"
    )
}

/// Flags direct references to `java.sql.Connection`.
pub struct JdbcConnectionReferenceRule;

impl Rule for JdbcConnectionReferenceRule {}

impl FileRule for JdbcConnectionReferenceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        const MESSAGE: &str =
            "Direct use of java.sql.Connection is discouraged; use a connection pool instead";
        for item in &context.script.items {
            check_item(item, sink, MESSAGE);
        }
    }
}

fn check_item(item: &Item, sink: &mut DiagnosticSink<'_>, message: &str) {
    match item {
        Item::Class(class) => {
            for member in &class.members {
                check_item(member, sink, message);
            }
        }
        Item::Field(field) => {
            if let Some(type_name) = &field.type_name
                && references_connection(type_name)
            {
                sink.report(field.span, message);
            }
        }
        Item::Method(method) => {
            if let Some(return_type) = &method.return_type
                && references_connection(return_type)
            {
                sink.report(method.span, message);
            }
            for param in &method.params {
                if let Some(type_name) = &param.type_name
                    && references_connection(type_name)
                {
                    sink.report(param.span, message);
                }
            }
        }
        _ => {}
    }
}

/// Flags `DriverManager.getConnection(...)` calls.
pub struct DirectConnectionManagementRule;

impl Rule for DirectConnectionManagementRule {}

impl AstRule for DirectConnectionManagementRule {
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
        let ExprKind::Call {
            receiver: Some(receiver),
            name,
            ..
        } = &expr.kind
        else {
            return;
        };
        if name != "getConnection" {
            return;
        }
        let ExprKind::Variable(owner) = &receiver.kind else {
            return;
        };
        if owner != "DriverManager" {
            return;
        }
        sink.report(
            expr.span,
            "The application should let the container manage database connections",
        );
    }
}

/// Descriptors for JDBC rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "JdbcConnectionReference",
                "org.codenarc.rule.jdbc.JdbcConnectionReferenceRule",
                Category::Jdbc,
                Severity::Warning,
                "Checks for direct references to java.sql.Connection.",
            ),
            || RuleInstanceImpl::File(Box::new(JdbcConnectionReferenceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "DirectConnectionManagement",
                "org.codenarc.rule.jdbc.DirectConnectionManagementRule",
                Category::Jdbc,
                Severity::Warning,
                "Checks for DriverManager.getConnection() calls.",
            ),
            || RuleInstanceImpl::Ast(Box::new(DirectConnectionManagementRule)),
        ),
    ]
}
