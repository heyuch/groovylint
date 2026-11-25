//! `CodeNarc` JDBC rules.
//!
//! The three reference rules share one walker: `CodeNarc` drives them through
//! `ClassReferenceAstVisitor`, which reports imports, field types, method
//! signatures, local variables, casts, constructor calls, class literals and
//! supertypes.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ExprKind, ImportDecl, Item, Script, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};

/// Which JDBC types a reference rule tracks, and the message it reports.
#[derive(Clone, Copy)]
pub struct ReferenceSpec {
    types: &'static [&'static str],
    message: &'static str,
}

impl ReferenceSpec {
    const CONNECTION: Self = Self {
        types: &["Connection"],
        message: "Direct use of java.sql.Connection is discouraged",
    };
    const RESULT_SET: Self = Self {
        types: &["ResultSet"],
        message: "Direct use of java.sql.ResultSet is discouraged",
    };
    const STATEMENT: Self = Self {
        types: &["Statement", "PreparedStatement", "CallableStatement"],
        message: "Direct use of java.sql.Statement is discouraged",
    };

    fn matches(&self, type_name: &str) -> bool {
        let simple = type_name.rsplit('.').next().unwrap_or(type_name);
        self.types.contains(&simple)
    }

    fn message(&self, type_name: &str) -> String {
        let simple = type_name.rsplit('.').next().unwrap_or(type_name);
        format!("{} ({simple})", self.message)
    }
}

/// Checks one file for references to a spec's types.
fn check_script(script: &Script, spec: ReferenceSpec, sink: &mut DiagnosticSink<'_>) {
    // `CodeNarc` reports the import itself, not only its usages.
    for import in &script.imports {
        if let Some(name) = matched_import(import, spec) {
            sink.report(import.span, spec.message(name));
        }
    }

    let mut collector = Collector {
        spec,
        spans: Vec::new(),
    };
    crate::traverse::walk_script(script, &mut collector);
    for (span, type_name) in collector.spans {
        sink.report(span, spec.message(&type_name));
    }
}

fn matched_import(import: &ImportDecl, spec: ReferenceSpec) -> Option<&str> {
    let path = import.path.as_str();
    if spec.matches(path) {
        return Some(path);
    }
    None
}

/// Collects declarations and expressions that name a tracked type.
struct Collector {
    spec: ReferenceSpec,
    spans: Vec<(groovylint_core::Span, String)>,
}

impl crate::traverse::AstVisitor for Collector {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Class(class) => {
                let mut names = Vec::new();
                if let Some(parent) = &class.extends {
                    names.push(parent.clone());
                }
                names.extend(class.implements.iter().cloned());
                for name in names {
                    if self.spec.matches(&name) {
                        self.spans.push((class.span, name));
                    }
                }
            }
            NodeRef::Field(field) => {
                if let Some(type_name) = &field.type_name
                    && self.spec.matches(type_name)
                {
                    self.spans.push((field.span, type_name.clone()));
                }
            }
            NodeRef::Method(method) => {
                if let Some(return_type) = &method.return_type
                    && self.spec.matches(return_type)
                {
                    self.spans.push((method.span, return_type.clone()));
                }
                for param in &method.params {
                    if let Some(type_name) = &param.type_name
                        && self.spec.matches(type_name)
                    {
                        self.spans.push((param.span, type_name.clone()));
                    }
                }
            }
            NodeRef::Stmt(Stmt::VarDecl(decl)) => {
                if let Some(type_name) = &decl.type_name
                    && self.spec.matches(type_name)
                {
                    self.spans.push((decl.span, type_name.clone()));
                }
            }
            NodeRef::Stmt(Stmt::Try(try_stmt)) => {
                for catch in &try_stmt.catches {
                    for type_name in &catch.type_names {
                        if self.spec.matches(type_name) {
                            self.spans.push((catch.span, type_name.clone()));
                        }
                    }
                }
            }
            NodeRef::Expr(expr) => match &expr.kind {
                ExprKind::New { type_name, .. } | ExprKind::Cast { type_name, .. } => {
                    if self.spec.matches(type_name) {
                        self.spans.push((expr.span, type_name.clone()));
                    }
                }
                ExprKind::Property {
                    receiver,
                    name,
                    safe: false,
                } if name == "class" => {
                    if let ExprKind::Variable(base) = &receiver.kind
                        && self.spec.matches(base)
                    {
                        self.spans.push((expr.span, base.clone()));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags direct references to `java.sql.Connection`.
pub struct JdbcConnectionReferenceRule;

impl Rule for JdbcConnectionReferenceRule {}

impl FileRule for JdbcConnectionReferenceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_script(context.script, ReferenceSpec::CONNECTION, sink);
    }
}

/// Flags direct references to `java.sql.ResultSet`.
pub struct JdbcResultSetReferenceRule;

impl Rule for JdbcResultSetReferenceRule {}

impl FileRule for JdbcResultSetReferenceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_script(context.script, ReferenceSpec::RESULT_SET, sink);
    }
}

/// Flags direct references to `java.sql.Statement` and its subtypes.
pub struct JdbcStatementReferenceRule;

impl Rule for JdbcStatementReferenceRule {}

impl FileRule for JdbcStatementReferenceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_script(context.script, ReferenceSpec::STATEMENT, sink);
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
                "JdbcResultSetReference",
                "org.codenarc.rule.jdbc.JdbcResultSetReferenceRule",
                Category::Jdbc,
                Severity::Warning,
                "Checks for direct references to java.sql.ResultSet.",
            ),
            || RuleInstanceImpl::File(Box::new(JdbcResultSetReferenceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JdbcStatementReference",
                "org.codenarc.rule.jdbc.JdbcStatementReferenceRule",
                Category::Jdbc,
                Severity::Warning,
                "Checks for direct references to java.sql.Statement.",
            ),
            || RuleInstanceImpl::File(Box::new(JdbcStatementReferenceRule)),
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

#[allow(dead_code)]
fn unused(_: &Item) {}
