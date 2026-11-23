//! Rules about conditional expressions.

use groovylint_syntax::ast::{BinaryOp, Expr, ExprKind, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{AstRule, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule};
use crate::rules::util::{
    ClassStack, expr_text, is_comparison, is_constant, unparen, variable_name,
};

fn condition_expressions(node: NodeRef<'_>) -> Vec<&Expr> {
    match node {
        NodeRef::Stmt(Stmt::If(if_stmt)) => vec![&if_stmt.condition],
        NodeRef::Stmt(Stmt::While(while_stmt)) => vec![&while_stmt.condition],
        NodeRef::Expr(expr) if matches!(expr.kind, ExprKind::Ternary { .. }) => match &expr.kind {
            ExprKind::Ternary { condition, .. } => vec![condition],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn find_assignments<'a>(expr: &'a Expr, found: &mut Vec<&'a Expr>) {
    match &unparen(expr).kind {
        ExprKind::Assign { .. } => found.push(expr),
        ExprKind::Binary {
            op: BinaryOp::And | BinaryOp::Or,
            left,
            right,
        } => {
            find_assignments(left, found);
            find_assignments(right, found);
        }
        _ => {}
    }
}

fn find_bitwise<'a>(expr: &'a Expr, found: &mut Vec<(&'a Expr, &'static str)>) {
    if let ExprKind::Binary { op, left, right } = &unparen(expr).kind {
        match op {
            BinaryOp::BitOr => found.push((expr, "or (|)")),
            BinaryOp::BitAnd => found.push((expr, "and (&)")),
            _ => {
                find_bitwise(left, found);
                find_bitwise(right, found);
            }
        }
    }
}

/// Checks for assignments used as conditional values.
pub struct AssignmentInConditionalRule;

impl Rule for AssignmentInConditionalRule {}

impl AstRule for AssignmentInConditionalRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
            .union(NodeMask::of(NodeKind::While))
            .union(NodeMask::of(NodeKind::Expr))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let mut assignments = Vec::new();
        for condition in condition_expressions(node) {
            find_assignments(condition, &mut assignments);
        }
        for assignment in assignments {
            sink.report(
                assignment.span,
                "Assignment used as conditional value, which always results in true. Use the == operator instead",
            );
        }
    }
}

/// Checks for bitwise operators in conditional expressions.
pub struct BitwiseOperatorInConditionalRule;

impl Rule for BitwiseOperatorInConditionalRule {}

impl AstRule for BitwiseOperatorInConditionalRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
            .union(NodeMask::of(NodeKind::While))
            .union(NodeMask::of(NodeKind::Expr))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let mut bitwise = Vec::new();
        for condition in condition_expressions(node) {
            find_bitwise(condition, &mut bitwise);
        }
        for (expression, operator) in bitwise {
            sink.report(
                expression.span,
                format!("Use of a bitwise {operator} operator in a conditional"),
            );
        }
    }
}

/// Checks for constant `if` conditions.
pub struct ConstantIfExpressionRule;

impl Rule for ConstantIfExpressionRule {}

impl AstRule for ConstantIfExpressionRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::If(if_stmt)) = node
            && is_constant(&if_stmt.condition)
        {
            sink.report(
                if_stmt.span,
                format!(
                    "The if statement condition ({}) contains a constant",
                    expr_text(&if_stmt.condition, context.source.text())
                ),
            );
        }
    }
}

/// Checks for constant ternary conditions, including Elvis expressions.
pub struct ConstantTernaryExpressionRule;

impl Rule for ConstantTernaryExpressionRule {}

impl AstRule for ConstantTernaryExpressionRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Expr(expr) = node
            && let ExprKind::Ternary { condition, .. } = &expr.kind
            && is_constant(condition)
        {
            sink.report(
                expr.span,
                format!(
                    "The ternary expression contains the constant {}",
                    expr_text(condition, context.source.text())
                ),
            );
        }
    }
}

/// Checks for constant assert conditions.
#[derive(Default)]
pub struct ConstantAssertExpressionRule {
    classes: ClassStack,
}

impl Rule for ConstantAssertExpressionRule {}

impl AstRule for ConstantAssertExpressionRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Assert))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.classes.enter(node);
        if let NodeRef::Stmt(Stmt::Assert(assert_stmt)) = node
            && is_constant(&assert_stmt.condition)
        {
            sink.report(
                assert_stmt.span,
                format!(
                    "The assert statement within class {} has a constant boolean expression [{}]",
                    self.classes.current(),
                    expr_text(&assert_stmt.condition, context.source.text())
                ),
            );
        }
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

fn is_null_literal(expr: &Expr) -> bool {
    matches!(
        &unparen(expr).kind,
        ExprKind::Literal(groovylint_syntax::ast::Literal::Null)
    )
}

fn is_comparison_with_null(expr: &Expr, op: BinaryOp) -> Option<String> {
    let ExprKind::Binary {
        op: actual,
        left,
        right,
    } = &unparen(expr).kind
    else {
        return None;
    };
    if *actual != op || !is_null_literal(right) {
        return None;
    }
    variable_name(left).map(ToString::to_string)
}

fn accesses_variable(expr: &Expr, name: &str) -> bool {
    match &unparen(expr).kind {
        ExprKind::Property { receiver, .. }
        | ExprKind::Call {
            receiver: Some(receiver),
            ..
        } => variable_name(receiver) == Some(name),
        ExprKind::Binary { left, .. } => accesses_variable(left, name),
        ExprKind::Unary { operand, .. } => accesses_variable(operand, name),
        _ => false,
    }
}

/// Detects null checks combined with member access using the wrong boolean
/// operator.
#[derive(Default)]
pub struct BrokenNullCheckRule {
    classes: ClassStack,
}

impl Rule for BrokenNullCheckRule {}

impl AstRule for BrokenNullCheckRule {
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
        let ExprKind::Binary { op, left, right } = &expr.kind else {
            return;
        };
        let variable = match op {
            BinaryOp::Or => is_comparison_with_null(left, BinaryOp::NotEq),
            BinaryOp::And => is_comparison_with_null(left, BinaryOp::Eq),
            _ => None,
        };
        if let Some(variable) = variable
            && accesses_variable(right, &variable)
        {
            sink.report(
                expr.span,
                format!(
                    "The expression {} within class {} incorrectly checks for null, and can throw a NullPointerException.",
                    expr_text(expr, context.source.text()),
                    self.classes.current()
                ),
            );
        }
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

fn is_constant_int(expr: &Expr, expected: &str) -> bool {
    matches!(
        &unparen(expr).kind,
        ExprKind::Literal(groovylint_syntax::ast::Literal::Number(value)) if value == expected
    )
}

fn modulus_by_two(expr: &Expr) -> Option<&Expr> {
    let ExprKind::Binary {
        op: BinaryOp::Mod,
        left,
        right,
    } = &unparen(expr).kind
    else {
        return None;
    };
    is_constant_int(right, "2").then_some(left)
}

/// Detects `x % 2 == 1` oddness checks, which fail for negative numbers.
pub struct BrokenOddnessCheckRule;

impl Rule for BrokenOddnessCheckRule {}

impl AstRule for BrokenOddnessCheckRule {
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
        let ExprKind::Binary {
            op: BinaryOp::Eq,
            left,
            right,
        } = &unparen(expr).kind
        else {
            return;
        };

        let (variable, prefix) = if let Some(modulus) = modulus_by_two(left)
            && is_constant_int(right, "1")
        {
            (modulus, "")
        } else if let Some(modulus) = modulus_by_two(right)
            && is_constant_int(left, "1")
        {
            (modulus, "1 == ")
        } else {
            return;
        };

        let variable_text = expr_text(variable, context.source.text());
        sink.report(
            expr.span,
            format!(
                "The code uses '({prefix}{variable_text} % 2 == 1)' to check for oddness, which does not work for negative numbers. Use ({variable_text} & 1 == 1) or ({variable_text} % 2 != 0) instead"
            ),
        );
    }
}

/// Checks for comparisons against the same variable.
pub struct ComparisonWithSelfRule;

impl Rule for ComparisonWithSelfRule {}

impl AstRule for ComparisonWithSelfRule {
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
        let same = match &expr.kind {
            ExprKind::Binary { op, left, right } if is_comparison(*op) => {
                variable_name(left).is_some_and(|name| variable_name(right) == Some(name))
            }
            ExprKind::Call {
                receiver: Some(receiver),
                name,
                args,
                ..
            } if matches!(name.as_str(), "equals" | "compareTo") && args.len() == 1 => {
                variable_name(receiver).is_some_and(|name| variable_name(&args[0]) == Some(name))
            }
            _ => false,
        };
        if same {
            sink.report(
                expr.span,
                format!(
                    "Comparing an object to itself is useless and may indicate a bug: {}",
                    expr_text(expr, context.source.text())
                ),
            );
        }
    }
}

/// Checks for comparisons between two constants.
pub struct ComparisonOfTwoConstantsRule;

impl Rule for ComparisonOfTwoConstantsRule {}

impl AstRule for ComparisonOfTwoConstantsRule {
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
        let both_constant = match &expr.kind {
            ExprKind::Binary { op, left, right } if is_comparison(*op) => {
                is_constant(left) && is_constant(right)
            }
            ExprKind::Call {
                receiver: Some(receiver),
                name,
                args,
                ..
            } if matches!(name.as_str(), "equals" | "compareTo") && args.len() == 1 => {
                is_constant(receiver) && is_constant(&args[0])
            }
            _ => false,
        };
        if both_constant {
            sink.report(
                expr.span,
                format!(
                    "Comparing two constants or constant literals is useless and may indicate a bug: {}",
                    expr_text(expr, context.source.text())
                ),
            );
        }
    }
}

/// Returns metadata for a conditional rule.
#[must_use]
pub const fn metadata() -> [crate::rule::RuleMeta; 9] {
    [
        crate::rule::RuleMeta::new(
            "AssignmentInConditional",
            "org.codenarc.rule.basic.AssignmentInConditionalRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for assignment in conditional expressions.",
        ),
        crate::rule::RuleMeta::new(
            "BitwiseOperatorInConditional",
            "org.codenarc.rule.basic.BitwiseOperatorInConditionalRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for bitwise operators in conditional expressions.",
        ),
        crate::rule::RuleMeta::new(
            "ConstantIfExpression",
            "org.codenarc.rule.basic.ConstantIfExpressionRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for constant if conditions.",
        ),
        crate::rule::RuleMeta::new(
            "ConstantTernaryExpression",
            "org.codenarc.rule.basic.ConstantTernaryExpressionRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for constant ternary conditions.",
        ),
        crate::rule::RuleMeta::new(
            "ConstantAssertExpression",
            "org.codenarc.rule.basic.ConstantAssertExpressionRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Info,
            "Checks for constant assert conditions.",
        ),
        crate::rule::RuleMeta::new(
            "BrokenNullCheck",
            "org.codenarc.rule.basic.BrokenNullCheckRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for incorrect null-check boolean logic.",
        ),
        crate::rule::RuleMeta::new(
            "BrokenOddnessCheck",
            "org.codenarc.rule.basic.BrokenOddnessCheckRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for modulo-based oddness checks.",
        ),
        crate::rule::RuleMeta::new(
            "ComparisonWithSelf",
            "org.codenarc.rule.basic.ComparisonWithSelfRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for comparisons of an object with itself.",
        ),
        crate::rule::RuleMeta::new(
            "ComparisonOfTwoConstants",
            "org.codenarc.rule.basic.ComparisonOfTwoConstantsRule",
            crate::rule::Category::Basic,
            groovylint_core::Severity::Warning,
            "Checks for comparisons between constants.",
        ),
    ]
}

/// Descriptors for the conditional rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    let meta = metadata();
    vec![
        descriptor(meta[0], || {
            RuleInstanceImpl::Ast(Box::new(AssignmentInConditionalRule))
        }),
        descriptor(meta[1], || {
            RuleInstanceImpl::Ast(Box::new(BitwiseOperatorInConditionalRule))
        }),
        descriptor(meta[2], || {
            RuleInstanceImpl::Ast(Box::new(ConstantIfExpressionRule))
        }),
        descriptor(meta[3], || {
            RuleInstanceImpl::Ast(Box::new(ConstantTernaryExpressionRule))
        }),
        descriptor(meta[4], || {
            RuleInstanceImpl::Ast(Box::new(ConstantAssertExpressionRule::default()))
        }),
        descriptor(meta[5], || {
            RuleInstanceImpl::Ast(Box::new(BrokenNullCheckRule::default()))
        }),
        descriptor(meta[6], || {
            RuleInstanceImpl::Ast(Box::new(BrokenOddnessCheckRule))
        }),
        descriptor(meta[7], || {
            RuleInstanceImpl::Ast(Box::new(ComparisonWithSelfRule))
        }),
        descriptor(meta[8], || {
            RuleInstanceImpl::Ast(Box::new(ComparisonOfTwoConstantsRule))
        }),
    ]
}
