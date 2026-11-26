//! `CodeNarc` rules that report redundant code.

use groovylint_core::Severity;
use groovylint_syntax::ast::{
    AssignOp, BinaryOp, ClassDecl, Expr, ExprKind, Item, Literal, MethodDecl, Stmt,
};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{ClassStack, expr_text, is_constant, unparen, variable_name};

fn is_null(expr: &Expr) -> bool {
    matches!(&unparen(expr).kind, ExprKind::Literal(Literal::Null))
}

fn bool_literal(expr: &Expr) -> Option<bool> {
    match &unparen(expr).kind {
        ExprKind::Literal(Literal::Boolean(value)) => Some(*value),
        _ => None,
    }
}

fn number_literal(expr: &Expr) -> Option<&str> {
    match &unparen(expr).kind {
        ExprKind::Literal(Literal::Number(text)) => Some(text),
        _ => None,
    }
}

fn string_literal(expr: &Expr) -> Option<&str> {
    match &unparen(expr).kind {
        ExprKind::Literal(Literal::String(text)) => Some(text),
        _ => None,
    }
}

/// Returns the property name of `receiver.name` expressions.
fn property_parts(expr: &Expr) -> Option<(&Expr, &str)> {
    let ExprKind::Property {
        receiver,
        name,
        safe: false,
    } = &unparen(expr).kind
    else {
        return None;
    };
    Some((receiver, name))
}

fn call_parts(expr: &Expr) -> Option<(&Expr, &str, &[Expr])> {
    let ExprKind::Call {
        receiver: Some(receiver),
        name,
        args,
        ..
    } = &unparen(expr).kind
    else {
        return None;
    };
    Some((receiver, name, args))
}

/// Flags `X.class` where `X` is a class name.
pub struct UnnecessaryDotClassRule;

impl Rule for UnnecessaryDotClassRule {}

impl AstRule for UnnecessaryDotClassRule {
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
        let Some((receiver, name)) = property_parts(expr) else {
            return;
        };
        if name != "class" {
            return;
        }
        let text = context.source.text();
        let receiver_text = expr_text(receiver, text);
        let is_class_name = match &unparen(receiver).kind {
            ExprKind::Variable(name) | ExprKind::Property { name, .. } => {
                name.chars().next().is_some_and(char::is_uppercase)
            }
            _ => false,
        };
        if !is_class_name {
            return;
        }
        sink.report(
            expr.span,
            format!("{receiver_text}.class can be rewritten as {receiver_text}"),
        );
    }
}

/// Flags runs of statements that repeat the same receiver.
#[derive(Default)]
pub struct UnnecessaryObjectReferencesRule {
    max_references_allowed: Option<usize>,
}

impl Rule for UnnecessaryObjectReferencesRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        self.max_references_allowed = match properties.get("maxReferencesAllowed") {
            None => None,
            Some(value) => Some(
                value
                    .parse()
                    .map_err(|_| "maxReferencesAllowed must be a number".to_string())?,
            ),
        };
        Ok(())
    }
}

impl AstRule for UnnecessaryObjectReferencesRule {
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
        let limit = self.max_references_allowed.unwrap_or(5);
        let mut current: Option<String> = None;
        let mut count = 0;

        for statement in &block.statements {
            let Stmt::Expr(expr) = statement else {
                current = None;
                count = 0;
                continue;
            };
            let Some(receiver) = statement_receiver(expr) else {
                current = None;
                count = 0;
                continue;
            };

            if current.as_deref() == Some(receiver.as_str()) {
                if count == limit {
                    sink.report(
                        expr.span,
                        "The code could be more concise by using a with() or identity() block",
                    );
                } else {
                    count += 1;
                }
            } else if receiver != "this" {
                current = Some(receiver);
                count = 1;
            } else {
                current = None;
                count = 0;
            }
        }
    }
}

/// Returns the receiver variable of a statement that starts with one.
fn statement_receiver(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Call {
            receiver: Some(receiver),
            ..
        } => variable_name(receiver).map(ToString::to_string),
        ExprKind::Assign {
            target,
            op: AssignOp::Assign,
            ..
        } => {
            let (receiver, _) = property_parts(target)?;
            variable_name(receiver).map(ToString::to_string)
        }
        _ => None,
    }
}

/// Flags `new BigDecimal(...)` when a literal would do.
pub struct UnnecessaryBigDecimalInstantiationRule;

/// Flags `new BigInteger(...)` when a literal would do.
pub struct UnnecessaryBigIntegerInstantiationRule;

/// Flags `new Boolean(...)`.
pub struct UnnecessaryBooleanInstantiationRule;

/// Flags `new Double(...)`.
pub struct UnnecessaryDoubleInstantiationRule;

/// Flags `new Float(...)`.
pub struct UnnecessaryFloatInstantiationRule;

/// Flags `new Integer(...)`.
pub struct UnnecessaryIntegerInstantiationRule;

/// Flags `new Long(...)`.
pub struct UnnecessaryLongInstantiationRule;

/// Flags `new String(...)`.
pub struct UnnecessaryStringInstantiationRule;

impl Rule for UnnecessaryBigDecimalInstantiationRule {}
impl Rule for UnnecessaryBigIntegerInstantiationRule {}
impl Rule for UnnecessaryBooleanInstantiationRule {}
impl Rule for UnnecessaryDoubleInstantiationRule {}
impl Rule for UnnecessaryFloatInstantiationRule {}
impl Rule for UnnecessaryIntegerInstantiationRule {}
impl Rule for UnnecessaryLongInstantiationRule {}
impl Rule for UnnecessaryStringInstantiationRule {}

/// Reports `new Type(literal)` for the boxed types.
fn check_instantiation(
    node: NodeRef<'_>,
    sink: &mut DiagnosticSink<'_>,
    type_name: &str,
    message: &str,
) {
    let NodeRef::Expr(expr) = node else {
        return;
    };
    let ExprKind::New {
        type_name: created,
        args,
        ..
    } = &expr.kind
    else {
        return;
    };
    let simple = created.rsplit('.').next().unwrap_or(created);
    if simple != type_name {
        return;
    }
    let Some(argument) = args.first() else {
        return;
    };
    if !is_constant(argument) {
        return;
    }
    sink.report(expr.span, message.to_string());
}

macro_rules! instantiation_rule {
    ($struct_name:ident, $type_name:literal, $message:literal) => {
        impl AstRule for $struct_name {
            fn interests(&self) -> NodeMask {
                NodeMask::of(NodeKind::Expr)
            }

            fn enter(
                &mut self,
                node: NodeRef<'_>,
                _context: &FileContext<'_>,
                sink: &mut DiagnosticSink<'_>,
            ) {
                check_instantiation(node, sink, $type_name, $message);
            }
        }
    };
}

instantiation_rule!(
    UnnecessaryBigDecimalInstantiationRule,
    "BigDecimal",
    "It is unnecessary to instantiate BigDecimal objects. Use a decimal literal instead"
);
instantiation_rule!(
    UnnecessaryBigIntegerInstantiationRule,
    "BigInteger",
    "It is unnecessary to instantiate BigInteger objects. Use a literal with the G suffix instead"
);
instantiation_rule!(
    UnnecessaryBooleanInstantiationRule,
    "Boolean",
    "Use Boolean.valueOf() or Boolean.TRUE/Boolean.FALSE instead of the constructor"
);
instantiation_rule!(
    UnnecessaryDoubleInstantiationRule,
    "Double",
    "It is unnecessary to instantiate Double objects. Use a double literal instead"
);
instantiation_rule!(
    UnnecessaryFloatInstantiationRule,
    "Float",
    "It is unnecessary to instantiate Float objects. Use a float literal instead"
);
instantiation_rule!(
    UnnecessaryIntegerInstantiationRule,
    "Integer",
    "It is unnecessary to instantiate Integer objects. Use a literal instead"
);
instantiation_rule!(
    UnnecessaryLongInstantiationRule,
    "Long",
    "It is unnecessary to instantiate Long objects. Use a literal instead"
);
instantiation_rule!(
    UnnecessaryStringInstantiationRule,
    "String",
    "Use a String literal instead of calling the String constructor"
);

/// Flags an `else` branch after an `if` chain where every branch returns.
pub struct UnnecessaryElseStatementRule;

impl Rule for UnnecessaryElseStatementRule {}

impl AstRule for UnnecessaryElseStatementRule {
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
        let Some(else_span) = if_stmt.else_span else {
            return;
        };

        let mut current = if_stmt;
        loop {
            if !block_ends_with_return(&current.then_branch) {
                return;
            }
            match current.else_branch.as_deref() {
                Some(Stmt::If(next)) => current = next,
                Some(_) => break,
                None => return,
            }
        }

        sink.report(
            else_span,
            "When an if statement block ends with a return statement the else is unnecessary",
        );
    }
}

fn block_ends_with_return(statement: &Stmt) -> bool {
    match statement {
        Stmt::Return(_) => true,
        Stmt::Block(block) => block
            .statements
            .last()
            .is_some_and(|last| matches!(last, Stmt::Return(_))),
        _ => false,
    }
}

/// Flags `if` statements that only return `true`/`false`.
pub struct UnnecessaryIfStatementRule;

impl Rule for UnnecessaryIfStatementRule {}

impl AstRule for UnnecessaryIfStatementRule {
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
        let Some(else_branch) = if_stmt.else_branch.as_deref() else {
            return;
        };
        if returns_constant(&if_stmt.then_branch, true) && returns_constant(else_branch, false) {
            sink.report(
                if_stmt.span,
                "The if and else blocks merely return true/false",
            );
        }
    }
}

fn returns_constant(statement: &Stmt, value: bool) -> bool {
    let returned = match statement {
        Stmt::Return(return_stmt) => return_stmt.value.as_ref(),
        Stmt::Block(block) => match block.statements.as_slice() {
            [Stmt::Return(return_stmt)] => return_stmt.value.as_ref(),
            _ => None,
        },
        _ => None,
    };
    returned.and_then(bool_literal) == Some(value)
}

/// Flags fully-qualified references to classes Groovy imports by default.
pub struct UnnecessaryPackageReferenceRule;

impl Rule for UnnecessaryPackageReferenceRule {}

impl AstRule for UnnecessaryPackageReferenceRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr).union(NodeMask::of(NodeKind::Field))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let text = context.source.text();
        let (span, type_name) = match node {
            NodeRef::Expr(expr) => {
                let ExprKind::New { type_name, .. } = &expr.kind else {
                    return;
                };
                (expr.span, type_name)
            }
            NodeRef::Field(field) => {
                let Some(type_name) = &field.type_name else {
                    return;
                };
                (field.span, type_name)
            }
            _ => return,
        };
        if !is_auto_imported(type_name) {
            return;
        }
        sink.report(
            span,
            format!("{type_name} is imported by default and can be referenced by its simple name"),
        );
        let _ = text;
    }
}

/// Returns whether a fully qualified name is imported by default in Groovy.
fn is_auto_imported(name: &str) -> bool {
    const PACKAGES: &[&str] = &[
        "java.lang.",
        "java.util.",
        "java.io.",
        "java.net.",
        "groovy.lang.",
        "groovy.util.",
    ];
    PACKAGES.iter().any(|package| name.starts_with(package))
}

/// Flags the `public` modifier on classes and methods.
pub struct UnnecessaryPublicModifierRule;

impl Rule for UnnecessaryPublicModifierRule {}

impl AstRule for UnnecessaryPublicModifierRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Method))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let (has_public, span) = match node {
            NodeRef::Class(class) => (class.is_public, class.span),
            NodeRef::Method(method) => (method.is_public, method.span),
            _ => return,
        };
        if !has_public {
            return;
        }
        // The declaration span starts at the keyword, so the modifier is only
        // visible on the declaration line.
        let line = context.source.line_col(span.start).line;
        let Some(text) = context.source.line_text(line) else {
            return;
        };
        if !text.trim_start().starts_with("public") {
            return;
        }
        sink.report(span, "The 'public' modifier is not required here");
    }
}

/// Flags `final` on private methods.
pub struct UnnecessaryFinalOnPrivateMethodRule;

impl Rule for UnnecessaryFinalOnPrivateMethodRule {}

impl AstRule for UnnecessaryFinalOnPrivateMethodRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Method(method) = node else {
            return;
        };
        if !method.is_private || !method.is_final {
            return;
        }
        let Some(text) = context.source.text().get(method.span.range()) else {
            return;
        };
        if !text.contains("final") {
            return;
        }
        sink.report(
            method.span,
            "A private method is marked final. Private methods cannot be overridden",
        );
    }
}

/// Flags self assignments.
pub struct UnnecessarySelfAssignmentRule;

impl Rule for UnnecessarySelfAssignmentRule {}

impl AstRule for UnnecessarySelfAssignmentRule {
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
        let ExprKind::Assign {
            target,
            op: AssignOp::Assign,
            value,
        } = &expr.kind
        else {
            return;
        };
        let text = context.source.text();
        if expr_text(target, text) != expr_text(value, text) {
            return;
        }
        sink.report(expr.span, "Self-assignment is pointless");
    }
}

/// Flags `!x instanceof Type` style checks.
pub struct UnnecessaryInstanceOfCheckRule;

impl Rule for UnnecessaryInstanceOfCheckRule {}

impl AstRule for UnnecessaryInstanceOfCheckRule {
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
        let ExprKind::Binary {
            op: BinaryOp::InstanceOf,
            left,
            ..
        } = &expr.kind
        else {
            return;
        };
        if !matches!(
            unparen(left).kind,
            ExprKind::Unary { .. } | ExprKind::Binary { .. }
        ) {
            return;
        }
        sink.report(
            expr.span,
            "The result of the expression on the left of instanceof is always a boolean",
        );
    }
}

/// Flags `% 1`.
pub struct UnnecessaryModOneRule;

impl Rule for UnnecessaryModOneRule {}

impl AstRule for UnnecessaryModOneRule {
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
        let ExprKind::Binary {
            op: BinaryOp::Mod,
            right,
            ..
        } = &expr.kind
        else {
            return;
        };
        if number_literal(right) != Some("1") {
            return;
        }
        sink.report(
            expr.span,
            "Any expression mod 1 is always zero, which is probably an error",
        );
    }
}

/// Flags `substring(0)`.
pub struct UnnecessaryCallToSubstringRule;

impl Rule for UnnecessaryCallToSubstringRule {}

impl AstRule for UnnecessaryCallToSubstringRule {
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
        let ExprKind::Call { name, args, .. } = &expr.kind else {
            return;
        };
        if name != "substring" || args.len() != 1 {
            return;
        }
        if number_literal(&args[0]) != Some("0") {
            return;
        }
        sink.report(
            expr.span,
            "Calling String.substring(0) always returns the original string",
        );
    }
}

/// Flags `c.containsAll(c)` and `c.retainAll(c)`.
pub struct UnnecessaryCollectionCallRule;

impl Rule for UnnecessaryCollectionCallRule {}

impl AstRule for UnnecessaryCollectionCallRule {
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
        let ExprKind::Call {
            receiver: Some(receiver),
            name,
            args,
            ..
        } = &expr.kind
        else {
            return;
        };
        if !matches!(name.as_str(), "containsAll" | "retainAll") || args.len() != 1 {
            return;
        }
        let text = context.source.text();
        if expr_text(receiver, text) != expr_text(&args[0], text) {
            return;
        }
        sink.report(
            expr.span,
            format!("The call to {name} on the collection itself is useless"),
        );
    }
}

/// Flags safe navigation on constants.
pub struct UnnecessarySafeNavigationOperatorRule;

impl Rule for UnnecessarySafeNavigationOperatorRule {}

impl AstRule for UnnecessarySafeNavigationOperatorRule {
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
        let (ExprKind::Property {
            receiver,
            safe: true,
            ..
        }
        | ExprKind::Call {
            receiver: Some(receiver),
            safe: true,
            ..
        }) = &expr.kind
        else {
            return;
        };
        if !is_constant(receiver) || is_null(receiver) {
            return;
        }
        sink.report(
            expr.span,
            "The safe navigation operator is applied to a constant, which can never be null",
        );
    }
}

/// Flags `x && true`, `x || false` and `!true`.
pub struct UnnecessaryBooleanExpressionRule;

impl Rule for UnnecessaryBooleanExpressionRule {}

impl AstRule for UnnecessaryBooleanExpressionRule {
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
        let redundant = match &expr.kind {
            ExprKind::Binary {
                op: BinaryOp::And,
                left,
                right,
            } => bool_literal(left) == Some(true) || bool_literal(right) == Some(true),
            ExprKind::Binary {
                op: BinaryOp::Or,
                left,
                right,
            } => bool_literal(left) == Some(false) || bool_literal(right) == Some(false),
            ExprKind::Unary {
                op: groovylint_syntax::ast::UnaryOp::Not,
                operand,
            } => is_constant(operand),
            _ => false,
        };
        if !redundant {
            return;
        }
        sink.report(expr.span, "The boolean expression is unnecessary");
    }
}

/// Flags `'' + x`.
pub struct AddEmptyStringRule;

impl Rule for AddEmptyStringRule {}

impl AstRule for AddEmptyStringRule {
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
        let ExprKind::Binary {
            op: BinaryOp::Add,
            left,
            ..
        } = &expr.kind
        else {
            return;
        };
        if string_literal(left).is_none_or(|text| !text.is_empty()) {
            return;
        }
        sink.report(
            expr.span,
            "Adding an empty string is not necessary to convert a value to a String",
        );
    }
}

/// Flags concatenation of two literals.
pub struct ConsecutiveStringConcatenationRule;

impl Rule for ConsecutiveStringConcatenationRule {}

impl AstRule for ConsecutiveStringConcatenationRule {
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
        let ExprKind::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = &expr.kind
        else {
            return;
        };
        if string_literal(left).is_none() || string_literal(right).is_none() {
            return;
        }
        sink.report(
            expr.span,
            "The two literals can be concatenated into one constant",
        );
    }
}

/// Flags chained `append` calls with literal arguments.
pub struct ConsecutiveLiteralAppendsRule;

impl Rule for ConsecutiveLiteralAppendsRule {}

impl AstRule for ConsecutiveLiteralAppendsRule {
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
        if name != "append" {
            return;
        }
        let ExprKind::Call {
            name: inner_name,
            args,
            ..
        } = &unparen(receiver).kind
        else {
            return;
        };
        if inner_name != "append" || args.is_empty() || !args.iter().all(is_constant) {
            return;
        }
        sink.report(
            expr.span,
            "Consecutive literal append calls can be combined into one call",
        );
    }
}

/// Flags `new X().getClass()`.
pub struct UnnecessaryInstantiationToGetClassRule;

impl Rule for UnnecessaryInstantiationToGetClassRule {}

impl AstRule for UnnecessaryInstantiationToGetClassRule {
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
        if name != "getClass" || !matches!(unparen(receiver).kind, ExprKind::New { .. }) {
            return;
        }
        sink.report(
            expr.span,
            "Avoid instantiating an object just to call getClass() on it; use the .class literal",
        );
    }
}

/// Flags `x != null && x.foo` that could use the safe navigation operator.
pub struct UnnecessaryNullCheckRule;

impl Rule for UnnecessaryNullCheckRule {}

impl AstRule for UnnecessaryNullCheckRule {
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
            op: BinaryOp::And,
            left,
            right,
        } = &expr.kind
        else {
            return;
        };
        let source = context.source.text();
        let redundant = match null_check_target(left) {
            Some(target) => dereferences(right, target),
            None => match null_check_target(right) {
                Some(target) => dereferences(left, target),
                None => false,
            },
        };
        if !redundant {
            return;
        }
        sink.report(
            expr.span,
            "The null check can be simplified using the safe navigation operator",
        );
        let _ = source;
    }
}

/// Returns the variable name of a `x != null` check.
fn null_check_target(expr: &Expr) -> Option<&str> {
    let ExprKind::Binary {
        op: BinaryOp::NotEq,
        left,
        right,
    } = &unparen(expr).kind
    else {
        return None;
    };
    if is_null(right) {
        return variable_name(left);
    }
    if is_null(left) {
        return variable_name(right);
    }
    None
}

/// Returns whether an expression dereferences a variable.
fn dereferences(expr: &Expr, name: &str) -> bool {
    match &unparen(expr).kind {
        ExprKind::Property { receiver, .. }
        | ExprKind::Call {
            receiver: Some(receiver),
            ..
        } => variable_name(receiver) == Some(name),
        _ => false,
    }
}

/// Flags `x != null && x instanceof Type`.
pub struct UnnecessaryNullCheckBeforeInstanceOfRule;

impl Rule for UnnecessaryNullCheckBeforeInstanceOfRule {}

impl AstRule for UnnecessaryNullCheckBeforeInstanceOfRule {
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
        let ExprKind::Binary {
            op: BinaryOp::And,
            left,
            right,
        } = &expr.kind
        else {
            return;
        };
        let ExprKind::Binary {
            op: BinaryOp::NotEq,
            left: checked,
            right: null,
        } = &unparen(left).kind
        else {
            return;
        };
        if !is_null(null) {
            return;
        }
        let ExprKind::Binary {
            op: BinaryOp::InstanceOf,
            left: instance,
            ..
        } = &unparen(right).kind
        else {
            return;
        };
        if variable_name(checked) != variable_name(instance) {
            return;
        }
        sink.report(
            expr.span,
            "There is no need to check for null before an instanceof",
        );
    }
}

/// Flags `foo({ ... })` where the parentheses are unnecessary.
pub struct UnnecessaryParenthesesForMethodCallWithClosureRule;

impl Rule for UnnecessaryParenthesesForMethodCallWithClosureRule {}

impl AstRule for UnnecessaryParenthesesForMethodCallWithClosureRule {
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
        let ExprKind::Call { name, args, .. } = &expr.kind else {
            return;
        };
        if args.len() != 1 || !matches!(args[0].kind, ExprKind::Closure { .. }) {
            return;
        }
        let Some(text) = context.source.text().get(expr.span.range()) else {
            return;
        };
        let trimmed = text.trim_end();
        if !trimmed.starts_with(&format!("{name}(")) || !trimmed.ends_with(')') {
            return;
        }
        sink.report(
            expr.span,
            format!("The parentheses in the call to {name} can be omitted"),
        );
    }
}

/// Flags ternaries that return booleans or repeat a branch.
pub struct UnnecessaryTernaryExpressionRule;

impl Rule for UnnecessaryTernaryExpressionRule {}

impl AstRule for UnnecessaryTernaryExpressionRule {
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
        let ExprKind::Ternary {
            then_expr,
            else_expr,
            ..
        } = &expr.kind
        else {
            return;
        };
        let both_boolean = bool_literal(then_expr).is_some() && bool_literal(else_expr).is_some();
        let same = expr_text(then_expr, context.source.text())
            == expr_text(else_expr, context.source.text());
        if !both_boolean && !same {
            return;
        }
        sink.report(expr.span, "The ternary expression is unnecessary");
    }
}

/// Flags `transient` on fields of classes that are not serializable.
#[derive(Default)]
pub struct UnnecessaryTransientModifierRule {
    classes: ClassStack,
    serializable: bool,
}

impl Rule for UnnecessaryTransientModifierRule {}

impl AstRule for UnnecessaryTransientModifierRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Field))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(class) => {
                self.classes.push(&class.name);
                self.serializable = is_serializable(class);
            }
            NodeRef::Field(field) => {
                if !field.is_transient || self.serializable {
                    return;
                }
                sink.report(
                    field.span,
                    "The field is marked transient but the class is not Serializable",
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
        if matches!(node, NodeRef::Class(_)) {
            self.classes.pop();
        }
    }
}

fn is_serializable(class: &ClassDecl) -> bool {
    class
        .implements
        .iter()
        .chain(class.extends.iter())
        .any(|name| name.rsplit('.').next() == Some("Serializable"))
}

/// The modifiers that make `def` redundant, in the order `CodeNarc` reports them.
const REDUNDANT_WITH_DEF: &[&str] = &[
    "private",
    "protected",
    "public",
    "static",
    "final",
    "synchronized",
    "abstract",
    "transient",
    "volatile",
    "strictfp",
    "Object",
];

/// The declaration text up to its value or comment, which is what `CodeNarc` scans.
fn declaration_head<'a>(
    context: &'a FileContext<'_>,
    span: groovylint_core::Span,
) -> Option<&'a str> {
    let text = context.source.text().get(span.range())?;
    let mut head = text;
    if let Some(index) = head.find('=') {
        head = &head[..index];
    }
    if let Some(index) = head.find("//") {
        head = &head[..index];
    }
    Some(head)
}

/// The modifier that already says what `def` says, if the declaration has one.
fn redundant_with_def(head: &str) -> Option<&'static str> {
    if !head.split_whitespace().any(|word| word == "def") {
        return None;
    }
    REDUNDANT_WITH_DEF
        .iter()
        .copied()
        .find(|keyword| head.split_whitespace().any(|word| word == *keyword))
}

/// Flags `def` on field declarations that already carry a modifier.
pub struct UnnecessaryDefInFieldDeclarationRule;

impl Rule for UnnecessaryDefInFieldDeclarationRule {}

impl AstRule for UnnecessaryDefInFieldDeclarationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Field(field) = node else {
            return;
        };
        let Some(head) = declaration_head(context, field.span) else {
            return;
        };
        let Some(keyword) = redundant_with_def(head) else {
            return;
        };
        sink.report(
            field.span,
            format!("The def keyword is unneeded when a field is marked {keyword}"),
        );
    }
}

/// Flags `def` on method declarations that already carry a modifier.
pub struct UnnecessaryDefInMethodDeclarationRule;

impl Rule for UnnecessaryDefInMethodDeclarationRule {}

impl AstRule for UnnecessaryDefInMethodDeclarationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Method(method) = node else {
            return;
        };
        let Some(head) = declaration_head(context, method.span) else {
            return;
        };
        // A quoted method name or a parameter list is not part of the declaration
        // proper; CodeNarc removes both before looking for keywords.
        let stripped = remove_distracting_parts(head);
        if stripped.contains('<') {
            return;
        }
        let Some(keyword) = redundant_with_def(&stripped) else {
            return;
        };
        sink.report(
            method.span,
            format!("The def keyword is unneeded when a method is marked {keyword}"),
        );
    }
}

/// Drops quoted names, parameter lists and generics from a declaration head.
fn remove_distracting_parts(head: &str) -> String {
    let mut out = String::with_capacity(head.len());
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for character in head.chars() {
        if let Some(active) = quote {
            if character == active {
                quote = None;
                out.push(' ');
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(character),
            _ => {}
        }
    }
    out
}

/// Flags `def` on variable declarations that already carry a modifier.
pub struct UnnecessaryDefInVariableDeclarationRule;

impl Rule for UnnecessaryDefInVariableDeclarationRule {}

impl AstRule for UnnecessaryDefInVariableDeclarationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::VarDecl)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::VarDecl(decl)) = node else {
            return;
        };
        let Some(head) = declaration_head(context, decl.span) else {
            return;
        };
        let Some(keyword) = redundant_with_def(head) else {
            return;
        };
        sink.report(
            decl.span,
            format!("The def keyword is unneeded when a variable is marked {keyword}"),
        );
    }
}

/// Flags constructors that only call `super()` with the same arguments.
pub struct UnnecessaryOverridingMethodRule;

impl Rule for UnnecessaryOverridingMethodRule {}

impl AstRule for UnnecessaryOverridingMethodRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Method(method) = node else {
            return;
        };
        let Some(body) = &method.body else {
            return;
        };
        if body.statements.len() != 1 {
            return;
        }
        let Stmt::Return(return_stmt) = &body.statements[0] else {
            return;
        };
        let Some(value) = &return_stmt.value else {
            return;
        };
        let ExprKind::Call {
            receiver: Some(receiver),
            name,
            args,
            ..
        } = &value.kind
        else {
            return;
        };
        if !matches!(receiver.kind, ExprKind::Super) || *name != method.name {
            return;
        }
        let forwards_arguments = args.iter().enumerate().all(|(index, arg)| {
            variable_name(arg).is_some_and(|name| {
                name == method.params[index.min(method.params.len().saturating_sub(1))].name
            })
        });
        if !forwards_arguments || args.len() != method.params.len() {
            return;
        }
        sink.report(
            method.span,
            "The overriding method merely calls the same method defined in a superclass",
        );
    }
}

/// Flags `catch` blocks that only rethrow the caught exception.
pub struct UnnecessaryCatchBlockRule;

impl Rule for UnnecessaryCatchBlockRule {}

impl AstRule for UnnecessaryCatchBlockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Try(try_stmt)) = node else {
            return;
        };
        for catch in &try_stmt.catches {
            let statements = &catch.body.statements;
            if statements.len() != 1 {
                continue;
            }
            let Stmt::Throw(throw_stmt) = &statements[0] else {
                continue;
            };
            let Some(name) = &catch.param_name else {
                continue;
            };
            if variable_name(&throw_stmt.value) != Some(name.as_str()) {
                continue;
            }
            sink.report(
                catch.span,
                "The catch block only rethrows the exception and can be removed",
            );
        }
    }
}

/// Flags a single public no-argument constructor with an empty body.
pub struct UnnecessaryConstructorRule;

impl Rule for UnnecessaryConstructorRule {}

impl AstRule for UnnecessaryConstructorRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Class(class) = node else {
            return;
        };
        let constructors: Vec<&MethodDecl> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Method(method) if method.name == class.name => Some(method),
                _ => None,
            })
            .collect();
        if constructors.len() != 1 {
            return;
        }
        let constructor = constructors[0];
        if !constructor.params.is_empty() || constructor.is_private || constructor.is_protected {
            return;
        }
        let Some(body) = &constructor.body else {
            return;
        };
        if !body.statements.is_empty() {
            return;
        }
        sink.report(
            constructor.span,
            "The constructor is not necessary and can be removed",
        );
    }
}

/// Flags `list[list.size() - 1]`.
pub struct UnnecessaryCallForLastElementRule;

impl Rule for UnnecessaryCallForLastElementRule {}

impl AstRule for UnnecessaryCallForLastElementRule {
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
        let ExprKind::Index { target, index, .. } = &expr.kind else {
            return;
        };
        let ExprKind::Binary {
            op: BinaryOp::Sub,
            left,
            right,
        } = &unparen(index).kind
        else {
            return;
        };
        if number_literal(right) != Some("1") {
            return;
        }
        let ExprKind::Call {
            receiver: Some(receiver),
            name,
            args,
            ..
        } = &unparen(left).kind
        else {
            return;
        };
        if name != "size" || !args.is_empty() {
            return;
        }
        let text = context.source.text();
        if expr_text(receiver, text) != expr_text(target, text) {
            return;
        }
        sink.report(
            expr.span,
            "The last element can be accessed with last() or a negative index",
        );
    }
}

/// Flags casts that cannot change the value.
pub struct UnnecessaryCastRule;

impl Rule for UnnecessaryCastRule {}

impl AstRule for UnnecessaryCastRule {
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
        let ExprKind::Cast {
            expr: inner,
            type_name,
        } = &expr.kind
        else {
            return;
        };
        let target = type_name.rsplit('.').next().unwrap_or(type_name);
        let matches_literal = match &unparen(inner).kind {
            ExprKind::Literal(Literal::String(_)) => target == "String",
            ExprKind::Literal(Literal::Number(_)) => matches!(
                target,
                "int"
                    | "Integer"
                    | "long"
                    | "Long"
                    | "double"
                    | "Double"
                    | "float"
                    | "Float"
                    | "short"
                    | "Short"
                    | "byte"
                    | "Byte"
                    | "BigDecimal"
                    | "BigInteger"
            ),
            ExprKind::List(_) => matches!(target, "List" | "ArrayList" | "Collection" | "Iterable"),
            ExprKind::Map(_) => matches!(target, "Map" | "HashMap" | "LinkedHashMap"),
            _ => false,
        };
        if matches_literal {
            sink.report(expr.span, format!("The cast to {target} is unnecessary"));
        }
    }
}

/// Flags a single-statement collect closure that could use the spread operator.
pub struct UnnecessaryCollectCallRule;

impl Rule for UnnecessaryCollectCallRule {}

impl AstRule for UnnecessaryCollectCallRule {
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
            receiver: Some(_),
            name,
            args,
            ..
        } = &expr.kind
        else {
            return;
        };
        if name != "collect" || args.len() != 1 {
            return;
        }
        let ExprKind::Closure { params, body, .. } = &unparen(&args[0]).kind else {
            return;
        };
        let param = params.first().map_or("it", |param| param.name.as_str());
        let [Stmt::Expr(inner)] = body.statements.as_slice() else {
            return;
        };
        let Some((receiver, _, _)) = call_parts(inner) else {
            // A property access on the closure parameter is also spreadable.
            if let ExprKind::Property { receiver, .. } = &unparen(inner).kind
                && variable_name(receiver) == Some(param)
            {
                sink.report(
                    expr.span,
                    "The call to collect could probably be rewritten as a spread expression",
                );
            }
            return;
        };
        if variable_name(receiver) == Some(param) {
            sink.report(
                expr.span,
                "The call to collect could probably be rewritten as a spread expression",
            );
        }
    }
}

/// Flags double-quoted strings without interpolation.
pub struct UnnecessaryGStringRule;

impl Rule for UnnecessaryGStringRule {}

impl AstRule for UnnecessaryGStringRule {
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
        let ExprKind::Literal(Literal::String(raw)) = &expr.kind else {
            return;
        };
        if raw.starts_with('"') && !raw.starts_with("\"\"\"") && !raw.contains('$') {
            sink.report(
                expr.span,
                "The string can be wrapped in single quotes instead of double quotes",
            );
        }
    }
}

/// Flags getter calls that could use property access.
pub struct UnnecessaryGetterRule;

impl Rule for UnnecessaryGetterRule {}

impl AstRule for UnnecessaryGetterRule {
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
            receiver: Some(_),
            name,
            args,
            ..
        } = &expr.kind
        else {
            return;
        };
        if !args.is_empty() || name == "getClass" {
            return;
        }
        let is_getter =
            name.starts_with("get") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase();
        let is_is_getter =
            name.starts_with("is") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase();
        if is_getter || is_is_getter {
            let property = if is_getter {
                format!(
                    "{}{}",
                    name.as_bytes()[3].to_ascii_lowercase() as char,
                    &name[4..]
                )
            } else {
                format!(
                    "{}{}",
                    name.as_bytes()[2].to_ascii_lowercase() as char,
                    &name[3..]
                )
            };
            sink.report(
                expr.span,
                format!("{name}() can probably be rewritten as {property}"),
            );
        }
    }
}

/// Flags an explicit return as the last statement of a method or closure.
pub struct UnnecessaryReturnKeywordRule;

impl Rule for UnnecessaryReturnKeywordRule {}

impl AstRule for UnnecessaryReturnKeywordRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method).union(NodeMask::of(NodeKind::Expr))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let block = match node {
            NodeRef::Method(method) => method.body.as_ref(),
            NodeRef::Expr(expr) => match &expr.kind {
                ExprKind::Closure { body, .. } => Some(body),
                _ => None,
            },
            _ => None,
        };
        let Some(block) = block else {
            return;
        };
        let Some(Stmt::Return(ret)) = block.statements.last() else {
            return;
        };
        if ret.value.is_some() {
            sink.report(
                ret.span,
                "The return keyword is not needed and can be removed",
            );
        }
    }
}

/// Flags explicit setter calls that could use property assignment.
pub struct UnnecessarySetterRule;

impl Rule for UnnecessarySetterRule {}

impl AstRule for UnnecessarySetterRule {
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
            args,
            ..
        } = &expr.kind
        else {
            return;
        };
        if args.len() != 1
            || !name.starts_with("set")
            || name.len() <= 3
            || !name.as_bytes()[3].is_ascii_uppercase()
            || variable_name(receiver) == Some("super")
        {
            return;
        }
        let property = format!(
            "{}{}",
            name.as_bytes()[3].to_ascii_lowercase() as char,
            &name[4..]
        );
        sink.report(
            expr.span,
            format!("{name}(...) can probably be rewritten as {property} = ..."),
        );
    }
}

/// Flags calls to `toString()` on values already known to be strings.
pub struct UnnecessaryToStringRule;

impl Rule for UnnecessaryToStringRule {}

impl AstRule for UnnecessaryToStringRule {
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
            args,
            ..
        } = &expr.kind
        else {
            return;
        };
        if name != "toString" || !args.is_empty() {
            return;
        }
        let receiver = unparen(receiver);
        let is_string = matches!(&receiver.kind, ExprKind::Literal(Literal::String(_)));
        if is_string {
            sink.report(
                expr.span,
                "Calling toString() on a String expression is unnecessary",
            );
        }
    }
}

/// Descriptors for the rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "AddEmptyString",
                "org.codenarc.rule.unnecessary.AddEmptyStringRule",
                Category::Unnecessary,
                Severity::Info,
                "Adding an empty string to convert a value is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(AddEmptyStringRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ConsecutiveLiteralAppends",
                "org.codenarc.rule.unnecessary.ConsecutiveLiteralAppendsRule",
                Category::Unnecessary,
                Severity::Info,
                "Consecutive literal append calls should be combined.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ConsecutiveLiteralAppendsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ConsecutiveStringConcatenation",
                "org.codenarc.rule.unnecessary.ConsecutiveStringConcatenationRule",
                Category::Unnecessary,
                Severity::Info,
                "Adjacent string literals should be combined.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ConsecutiveStringConcatenationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryBigDecimalInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryBigDecimalInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "BigDecimal objects should be written as literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryBigDecimalInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryBigIntegerInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryBigIntegerInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "BigInteger objects should be written as literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryBigIntegerInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryBooleanExpression",
                "org.codenarc.rule.unnecessary.UnnecessaryBooleanExpressionRule",
                Category::Unnecessary,
                Severity::Info,
                "Boolean expressions with constants are unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryBooleanExpressionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryBooleanInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryBooleanInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "Boolean objects should not be constructed directly.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryBooleanInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryCallForLastElement",
                "org.codenarc.rule.unnecessary.UnnecessaryCallForLastElementRule",
                Category::Unnecessary,
                Severity::Info,
                "The last element has a simpler accessor.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryCallForLastElementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryCallToSubstring",
                "org.codenarc.rule.unnecessary.UnnecessaryCallToSubstringRule",
                Category::Unnecessary,
                Severity::Info,
                "substring(0) returns the original string.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryCallToSubstringRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryCatchBlock",
                "org.codenarc.rule.unnecessary.UnnecessaryCatchBlockRule",
                Category::Unnecessary,
                Severity::Info,
                "Catch blocks that rethrow the exception are unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryCatchBlockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryCollectionCall",
                "org.codenarc.rule.unnecessary.UnnecessaryCollectionCallRule",
                Category::Unnecessary,
                Severity::Info,
                "Calls such as containsAll on the collection itself are useless.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryCollectionCallRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryConstructor",
                "org.codenarc.rule.unnecessary.UnnecessaryConstructorRule",
                Category::Unnecessary,
                Severity::Info,
                "An empty public no-argument constructor is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryConstructorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryDefInFieldDeclaration",
                "org.codenarc.rule.unnecessary.UnnecessaryDefInFieldDeclarationRule",
                Category::Unnecessary,
                Severity::Info,
                "`def` is unneeded when a field has a type or modifier.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryDefInFieldDeclarationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryDefInMethodDeclaration",
                "org.codenarc.rule.unnecessary.UnnecessaryDefInMethodDeclarationRule",
                Category::Unnecessary,
                Severity::Info,
                "`def` is unneeded when a method has a modifier.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryDefInMethodDeclarationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryDefInVariableDeclaration",
                "org.codenarc.rule.unnecessary.UnnecessaryDefInVariableDeclarationRule",
                Category::Unnecessary,
                Severity::Info,
                "`def` is unneeded when a variable has a type or modifier.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryDefInVariableDeclarationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryDotClass",
                "org.codenarc.rule.unnecessary.UnnecessaryDotClassRule",
                Category::Unnecessary,
                Severity::Info,
                "`X.class` can be shortened to `X`.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryDotClassRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryDoubleInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryDoubleInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "Double objects should be written as literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryDoubleInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryElseStatement",
                "org.codenarc.rule.unnecessary.UnnecessaryElseStatementRule",
                Category::Unnecessary,
                Severity::Info,
                "The else branch is unnecessary after returning branches.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryElseStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryFinalOnPrivateMethod",
                "org.codenarc.rule.unnecessary.UnnecessaryFinalOnPrivateMethodRule",
                Category::Unnecessary,
                Severity::Info,
                "Private methods cannot be overridden, so `final` is unneeded.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryFinalOnPrivateMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryFloatInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryFloatInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "Float objects should be written as literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryFloatInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryIfStatement",
                "org.codenarc.rule.unnecessary.UnnecessaryIfStatementRule",
                Category::Unnecessary,
                Severity::Info,
                "If statements that return true/false are unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryIfStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryInstanceOfCheck",
                "org.codenarc.rule.unnecessary.UnnecessaryInstanceOfCheckRule",
                Category::Unnecessary,
                Severity::Info,
                "An instanceof check on a boolean expression can never be true.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryInstanceOfCheckRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryInstantiationToGetClass",
                "org.codenarc.rule.unnecessary.UnnecessaryInstantiationToGetClassRule",
                Category::Unnecessary,
                Severity::Info,
                "Use `X.class` instead of instantiating an object to call getClass().",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryInstantiationToGetClassRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryIntegerInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryIntegerInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "Integer objects should be written as literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryIntegerInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryLongInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryLongInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "Long objects should be written as literals.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryLongInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryModOne",
                "org.codenarc.rule.unnecessary.UnnecessaryModOneRule",
                Category::Unnecessary,
                Severity::Info,
                "Any expression mod 1 is zero.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryModOneRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryNullCheck",
                "org.codenarc.rule.unnecessary.UnnecessaryNullCheckRule",
                Category::Unnecessary,
                Severity::Info,
                "The safe navigation operator can replace an explicit null check.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryNullCheckRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryNullCheckBeforeInstanceOf",
                "org.codenarc.rule.unnecessary.UnnecessaryNullCheckBeforeInstanceOfRule",
                Category::Unnecessary,
                Severity::Info,
                "instanceof already returns false for null.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryNullCheckBeforeInstanceOfRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryObjectReferences",
                "org.codenarc.rule.unnecessary.UnnecessaryObjectReferencesRule",
                Category::Unnecessary,
                Severity::Info,
                "Repeated object references can use a with() block.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryObjectReferencesRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryOverridingMethod",
                "org.codenarc.rule.unnecessary.UnnecessaryOverridingMethodRule",
                Category::Unnecessary,
                Severity::Info,
                "The overriding method only calls the superclass method.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryOverridingMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryPackageReference",
                "org.codenarc.rule.unnecessary.UnnecessaryPackageReferenceRule",
                Category::Unnecessary,
                Severity::Info,
                "Default imports do not need their package prefix.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryPackageReferenceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryParenthesesForMethodCallWithClosure",
                "org.codenarc.rule.unnecessary.UnnecessaryParenthesesForMethodCallWithClosureRule",
                Category::Unnecessary,
                Severity::Info,
                "Parentheses around a trailing closure are unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryParenthesesForMethodCallWithClosureRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryPublicModifier",
                "org.codenarc.rule.unnecessary.UnnecessaryPublicModifierRule",
                Category::Unnecessary,
                Severity::Info,
                "The `public` modifier is not needed.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryPublicModifierRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessarySafeNavigationOperator",
                "org.codenarc.rule.unnecessary.UnnecessarySafeNavigationOperatorRule",
                Category::Unnecessary,
                Severity::Info,
                "Safe navigation on a constant is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessarySafeNavigationOperatorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessarySelfAssignment",
                "org.codenarc.rule.unnecessary.UnnecessarySelfAssignmentRule",
                Category::Unnecessary,
                Severity::Info,
                "Self-assignment is pointless.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessarySelfAssignmentRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryStringInstantiation",
                "org.codenarc.rule.unnecessary.UnnecessaryStringInstantiationRule",
                Category::Unnecessary,
                Severity::Info,
                "Use a string literal instead of the String constructor.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryStringInstantiationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryTernaryExpression",
                "org.codenarc.rule.unnecessary.UnnecessaryTernaryExpressionRule",
                Category::Unnecessary,
                Severity::Info,
                "The ternary expression is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryTernaryExpressionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryTransientModifier",
                "org.codenarc.rule.unnecessary.UnnecessaryTransientModifierRule",
                Category::Unnecessary,
                Severity::Info,
                "`transient` has no effect outside serializable classes.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryTransientModifierRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryCast",
                "org.codenarc.rule.unnecessary.UnnecessaryCastRule",
                Category::Unnecessary,
                Severity::Warning,
                "The cast is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryCastRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryCollectCall",
                "org.codenarc.rule.unnecessary.UnnecessaryCollectCallRule",
                Category::Unnecessary,
                Severity::Info,
                "collect() can use a spread expression.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryCollectCallRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryGString",
                "org.codenarc.rule.unnecessary.UnnecessaryGStringRule",
                Category::Unnecessary,
                Severity::Info,
                "Double-quoted strings without interpolation can use single quotes.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryGStringRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryGetter",
                "org.codenarc.rule.unnecessary.UnnecessaryGetterRule",
                Category::Unnecessary,
                Severity::Info,
                "Getter calls can use property access.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryGetterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryReturnKeyword",
                "org.codenarc.rule.unnecessary.UnnecessaryReturnKeywordRule",
                Category::Unnecessary,
                Severity::Info,
                "The return keyword is optional at the end of a method or closure.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryReturnKeywordRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessarySetter",
                "org.codenarc.rule.unnecessary.UnnecessarySetterRule",
                Category::Unnecessary,
                Severity::Info,
                "Setter calls can use property assignment.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessarySetterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryToString",
                "org.codenarc.rule.unnecessary.UnnecessaryToStringRule",
                Category::Unnecessary,
                Severity::Warning,
                "toString() on a String is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryToStringRule)),
        ),
    ]
}
