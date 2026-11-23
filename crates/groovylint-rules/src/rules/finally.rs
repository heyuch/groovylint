//! Rules that inspect code inside `finally` blocks.

use groovylint_core::Severity;
use groovylint_syntax::ast::Stmt;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};

/// Detects `assert` inside a `finally` block.
#[derive(Default)]
pub struct AssertWithinFinallyBlockRule {
    finally_depth: usize,
    classes: Vec<String>,
}

impl Rule for AssertWithinFinallyBlockRule {}

impl AstRule for AssertWithinFinallyBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Finally)
            .union(NodeMask::of(NodeKind::Assert))
            .union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(class) => self.classes.push(class.name.clone()),
            NodeRef::Finally(_) => self.finally_depth += 1,
            NodeRef::Stmt(Stmt::Assert(assert_stmt)) if self.finally_depth > 0 => {
                let class_name = self.classes.last().map_or("null", String::as_str);
                sink.report(
                    assert_stmt.span,
                    format!(
                        "A finally block within class {class_name} contains an assert statement, potentially hiding the original exception, if there is one"
                    ),
                );
            }
            _ => {}
        }
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(_) => {
                self.classes.pop();
            }
            NodeRef::Finally(_) => self.finally_depth = self.finally_depth.saturating_sub(1),
            _ => {}
        }
    }
}

/// Detects `return` inside a `finally` block.
#[derive(Default)]
pub struct ReturnFromFinallyBlockRule {
    finally_depth: usize,
}

impl Rule for ReturnFromFinallyBlockRule {}

impl AstRule for ReturnFromFinallyBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Finally).union(NodeMask::of(NodeKind::Return))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Finally(_) => self.finally_depth += 1,
            NodeRef::Stmt(Stmt::Return(return_stmt)) if self.finally_depth > 0 => {
                sink.report(return_stmt.span, "finally statements cannot return a value");
            }
            _ => {}
        }
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Finally(_)) {
            self.finally_depth = self.finally_depth.saturating_sub(1);
        }
    }
}

/// Detects `throw` inside a `finally` block.
#[derive(Default)]
pub struct ThrowExceptionFromFinallyBlockRule {
    finally_depth: usize,
}

impl Rule for ThrowExceptionFromFinallyBlockRule {}

impl AstRule for ThrowExceptionFromFinallyBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Finally).union(NodeMask::of(NodeKind::Throw))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Finally(_) => self.finally_depth += 1,
            NodeRef::Stmt(Stmt::Throw(throw_stmt)) if self.finally_depth > 0 => {
                sink.report(
                    throw_stmt.span,
                    "Throwing an exception from a finally block can hide an underlying error",
                );
            }
            _ => {}
        }
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Finally(_)) {
            self.finally_depth = self.finally_depth.saturating_sub(1);
        }
    }
}

const ASSERT_META: RuleMeta = RuleMeta::new(
    "AssertWithinFinallyBlock",
    "org.codenarc.rule.basic.AssertWithinFinallyBlockRule",
    Category::Basic,
    Severity::Warning,
    "Checks for assert statements inside finally blocks.",
);
const RETURN_META: RuleMeta = RuleMeta::new(
    "ReturnFromFinallyBlock",
    "org.codenarc.rule.basic.ReturnFromFinallyBlockRule",
    Category::Basic,
    Severity::Warning,
    "Checks for return statements inside finally blocks.",
);
const THROW_META: RuleMeta = RuleMeta::new(
    "ThrowExceptionFromFinallyBlock",
    "org.codenarc.rule.basic.ThrowExceptionFromFinallyBlockRule",
    Category::Basic,
    Severity::Warning,
    "Checks for throw statements inside finally blocks.",
);

/// Descriptors for finally rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(ASSERT_META, || {
            RuleInstanceImpl::Ast(Box::new(AssertWithinFinallyBlockRule::default()))
        }),
        descriptor(RETURN_META, || {
            RuleInstanceImpl::Ast(Box::new(ReturnFromFinallyBlockRule::default()))
        }),
        descriptor(THROW_META, || {
            RuleInstanceImpl::Ast(Box::new(ThrowExceptionFromFinallyBlockRule::default()))
        }),
    ]
}
