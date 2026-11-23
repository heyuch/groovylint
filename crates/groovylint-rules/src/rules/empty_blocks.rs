//! `CodeNarc` basic rules that detect empty blocks.

use groovylint_syntax::ast::Stmt;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};

/// Detects an `if` whose then branch is empty.
pub struct EmptyIfStatementRule;

impl Rule for EmptyIfStatementRule {}

impl AstRule for EmptyIfStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::If(if_stmt)) = node
            && if_stmt.then_branch.is_empty_block()
        {
            sink.report_with_help(
                if_stmt.span,
                "The if statement is empty",
                "Add a statement or remove the if statement.",
            );
        }
    }
}

/// Detects an `else` block with no statements.
pub struct EmptyElseBlockRule;

impl Rule for EmptyElseBlockRule {}

impl AstRule for EmptyElseBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::If(if_stmt)) = node
            && if_stmt
                .else_branch
                .as_deref()
                .is_some_and(Stmt::is_empty_block)
        {
            sink.report_with_help(
                if_stmt.else_span.unwrap_or(if_stmt.span),
                "The else block is empty",
                "Add a statement or remove the else block.",
            );
        }
    }
}

/// Detects a `while` loop with an empty body.
pub struct EmptyWhileStatementRule;

impl Rule for EmptyWhileStatementRule {}

impl AstRule for EmptyWhileStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::While)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::While(while_stmt)) = node
            && while_stmt.body.is_empty_block()
        {
            sink.report_with_help(
                while_stmt.span,
                "The while statement is empty",
                "Add a statement or remove the while loop.",
            );
        }
    }
}

/// Detects a classic or enhanced `for` loop with an empty body.
pub struct EmptyForStatementRule;

impl Rule for EmptyForStatementRule {}

impl AstRule for EmptyForStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::For).union(NodeMask::of(NodeKind::ForEach))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(stmt) = node {
            let (body, span) = match stmt {
                Stmt::For(for_stmt) => (&for_stmt.body, for_stmt.span),
                Stmt::ForEach(for_each) => (&for_each.body, for_each.span),
                _ => return,
            };
            if body.is_empty_block() {
                sink.report_with_help(
                    span,
                    "The for statement is empty",
                    "Add a statement or remove the for loop.",
                );
            }
        }
    }
}

/// Detects a `try` block with no statements.
pub struct EmptyTryBlockRule;

impl Rule for EmptyTryBlockRule {}

impl AstRule for EmptyTryBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::Try(try_stmt)) = node
            && try_stmt.body.statements.is_empty()
        {
            sink.report_with_help(
                try_stmt.span,
                "The try block is empty",
                "Add a statement or remove the try block.",
            );
        }
    }
}

/// Detects a `catch` block with no statements.
pub struct EmptyCatchBlockRule;

impl Rule for EmptyCatchBlockRule {}

impl AstRule for EmptyCatchBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::Try(try_stmt)) = node {
            for catch in &try_stmt.catches {
                if catch
                    .param_name
                    .as_deref()
                    .is_some_and(|name| matches!(name, "ignore" | "ignored"))
                {
                    continue;
                }
                if catch.body.statements.is_empty() {
                    sink.report_with_help(
                        catch.span,
                        "The catch block is empty",
                        "Handle or log the exception instead of ignoring it.",
                    );
                }
            }
        }
    }
}

/// Detects a `finally` block with no statements.
pub struct EmptyFinallyBlockRule;

impl Rule for EmptyFinallyBlockRule {}

impl AstRule for EmptyFinallyBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::Try(try_stmt)) = node
            && try_stmt
                .finally
                .as_ref()
                .is_some_and(|block| block.statements.is_empty())
        {
            sink.report_with_help(
                try_stmt.finally_span.unwrap_or(try_stmt.span),
                "The finally block is empty",
                "Add cleanup code or remove the finally block.",
            );
        }
    }
}

/// Descriptors for empty-block rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "EmptyIfStatement",
                "org.codenarc.rule.basic.EmptyIfStatementRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty if statements.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyIfStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyElseBlock",
                "org.codenarc.rule.basic.EmptyElseBlockRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty else blocks.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyElseBlockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyWhileStatement",
                "org.codenarc.rule.basic.EmptyWhileStatementRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty while statements.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyWhileStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyForStatement",
                "org.codenarc.rule.basic.EmptyForStatementRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty for statements.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyForStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyTryBlock",
                "org.codenarc.rule.basic.EmptyTryBlockRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty try blocks.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyTryBlockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyCatchBlock",
                "org.codenarc.rule.basic.EmptyCatchBlockRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty catch blocks.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyCatchBlockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyFinallyBlock",
                "org.codenarc.rule.basic.EmptyFinallyBlockRule",
                Category::Basic,
                groovylint_core::Severity::Warning,
                "Checks for empty finally blocks.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyFinallyBlockRule)),
        ),
    ]
}
