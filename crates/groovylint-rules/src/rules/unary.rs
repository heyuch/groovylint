//! Rules about unary operators.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ExprKind, Stmt, UnaryOp};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{ClassStack, expr_text, unparen};

fn is_unary(op: UnaryOp) -> bool {
    matches!(
        op,
        UnaryOp::Not | UnaryOp::BitNot | UnaryOp::Neg | UnaryOp::Pos
    )
}

fn op_text(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
        UnaryOp::Neg => "-",
        UnaryOp::Pos => "+",
        UnaryOp::PreInc
        | UnaryOp::PreDec
        | UnaryOp::PostInc
        | UnaryOp::PostDec
        | UnaryOp::Spread => "",
    }
}

/// Detects `!!x` and `!(!x)`.
pub struct DoubleNegativeRule;

impl Rule for DoubleNegativeRule {}

impl AstRule for DoubleNegativeRule {
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
        if let ExprKind::Unary {
            op: UnaryOp::Not,
            operand,
        } = &expr.kind
            && matches!(
                &unparen(operand).kind,
                ExprKind::Unary {
                    op: UnaryOp::Not,
                    ..
                }
            )
        {
            sink.report(
                expr.span,
                format!(
                    "The expression ({}) is a confusing double negative",
                    expr_text(expr, context.source.text())
                ),
            );
        }
    }
}

/// Detects consecutive unary operators such as `!!`, `~~`, `-+`.
#[derive(Default)]
pub struct MultipleUnaryOperatorsRule {
    classes: ClassStack,
}

impl Rule for MultipleUnaryOperatorsRule {}

impl AstRule for MultipleUnaryOperatorsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Expr))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.classes.enter(node);
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Unary { op, operand } = &expr.kind else {
            return;
        };
        if !is_unary(*op) {
            return;
        }
        let ExprKind::Unary { op: second_op, .. } = &unparen(operand).kind else {
            return;
        };
        if !is_unary(*second_op) {
            return;
        }
        let operators = format!("{}{}", op_text(*op), op_text(*second_op));
        sink.report(
            expr.span,
            format!(
                "The expression ({}) in class {} contains confusing multiple consecutive unary operators ({operators})",
                expr_text(expr, context.source.text()),
                self.classes.current()
            ),
        );
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        self.classes.exit(node);
    }
}

/// Detects code after a statement that always transfers control.
pub struct DeadCodeRule;

impl Rule for DeadCodeRule {}

impl AstRule for DeadCodeRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Block(block) = node else {
            return;
        };
        for pair in block.statements.windows(2) {
            if forces_return(&pair[0]) {
                sink.report(pair[1].span(), "This code cannot be reached");
            }
        }
    }
}

fn forces_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) | Stmt::Throw(_) => true,
        Stmt::If(if_stmt) => if_stmt.else_branch.as_deref().is_some_and(|else_branch| {
            stmt_block_forces_return(&if_stmt.then_branch) && stmt_block_forces_return(else_branch)
        }),
        _ => false,
    }
}

fn stmt_block_forces_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block(block) => block.statements.last().is_some_and(forces_return),
        other => forces_return(other),
    }
}

/// Metadata for unary/dead-code rules.
const META: [RuleMeta; 3] = [
    RuleMeta::new(
        "DoubleNegative",
        "org.codenarc.rule.basic.DoubleNegativeRule",
        Category::Basic,
        Severity::Warning,
        "Checks for double negatives.",
    ),
    RuleMeta::new(
        "MultipleUnaryOperators",
        "org.codenarc.rule.basic.MultipleUnaryOperatorsRule",
        Category::Basic,
        Severity::Warning,
        "Checks for multiple consecutive unary operators.",
    ),
    RuleMeta::new(
        "DeadCode",
        "org.codenarc.rule.basic.DeadCodeRule",
        Category::Basic,
        Severity::Warning,
        "Checks for unreachable code.",
    ),
];

/// Descriptors for unary rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(META[0], || {
            RuleInstanceImpl::Ast(Box::new(DoubleNegativeRule))
        }),
        descriptor(META[1], || {
            RuleInstanceImpl::Ast(Box::new(MultipleUnaryOperatorsRule::default()))
        }),
        descriptor(META[2], || RuleInstanceImpl::Ast(Box::new(DeadCodeRule))),
    ]
}
