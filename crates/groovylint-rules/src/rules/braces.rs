//! `CodeNarc` brace rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::Stmt;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};

fn is_block(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Block(_))
}

/// Flags `if`, `for` and `while` statements whose body is not a block.
pub struct BodyBracesRule {
    kinds: NodeMask,
    message: &'static str,
}

impl Rule for BodyBracesRule {}

impl AstRule for BodyBracesRule {
    fn interests(&self) -> NodeMask {
        self.kinds
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(stmt) = node else {
            return;
        };
        let (body, span) = match stmt {
            Stmt::If(if_stmt) => (&if_stmt.then_branch, if_stmt.span),
            Stmt::While(while_stmt) => (&while_stmt.body, while_stmt.span),
            Stmt::For(for_stmt) => (&for_stmt.body, for_stmt.span),
            Stmt::ForEach(for_each) => (&for_each.body, for_each.span),
            _ => return,
        };
        if is_block(body) {
            return;
        }
        sink.report(span, self.message);
    }
}

/// Flags `else` branches whose body is not a block.
#[derive(Default)]
pub struct ElseBlockBracesRule {
    /// Whether `else if` also needs braces.
    braces_required_for_else_if: bool,
}

impl Rule for ElseBlockBracesRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        self.braces_required_for_else_if = match properties.get("bracesRequiredForElseIf") {
            None => false,
            Some(value) if value.eq_ignore_ascii_case("true") => true,
            Some(value) if value.eq_ignore_ascii_case("false") => false,
            Some(value) => {
                return Err(format!(
                    "property `bracesRequiredForElseIf` must be true or false, got `{value}`"
                ));
            }
        };
        Ok(())
    }
}

impl AstRule for ElseBlockBracesRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::If(if_stmt)) = node else {
            return;
        };
        let Some(else_branch) = &if_stmt.else_branch else {
            return;
        };
        if is_block(else_branch) {
            return;
        }
        if !self.braces_required_for_else_if && matches!(**else_branch, Stmt::If(_)) {
            return;
        }
        sink.report(if_stmt.span, "The else block lacks braces");
    }
}

/// Descriptors for brace rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "IfStatementBraces",
                "org.codenarc.rule.braces.IfStatementBracesRule",
                Category::Braces,
                Severity::Warning,
                "Checks for if statements without braces.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(BodyBracesRule {
                    kinds: NodeMask::of(NodeKind::If),
                    message: "The if statement lacks braces",
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "ElseBlockBraces",
                "org.codenarc.rule.braces.ElseBlockBracesRule",
                Category::Braces,
                Severity::Warning,
                "Checks for else blocks without braces.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ElseBlockBracesRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ForStatementBraces",
                "org.codenarc.rule.braces.ForStatementBracesRule",
                Category::Braces,
                Severity::Warning,
                "Checks for for loops without braces.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(BodyBracesRule {
                    kinds: NodeMask::of(NodeKind::For).union(NodeMask::of(NodeKind::ForEach)),
                    message: "The for statement lacks braces",
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "WhileStatementBraces",
                "org.codenarc.rule.braces.WhileStatementBracesRule",
                Category::Braces,
                Severity::Warning,
                "Checks for while loops without braces.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(BodyBracesRule {
                    kinds: NodeMask::of(NodeKind::While),
                    message: "The while statement lacks braces",
                }))
            },
        ),
    ]
}
