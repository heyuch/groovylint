//! Additional `CodeNarc` convention rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{BinaryOp, ClassKind, Expr, ExprKind, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{ClassStack, expr_text, is_constant, unparen};
use crate::traverse::{AstVisitor, walk_block};

/// Requires every class to be annotated with one of the static compilation
/// annotations.
#[derive(Default)]
pub struct CompileStaticRule {
    classes: ClassStack,
}

impl Rule for CompileStaticRule {}

impl AstRule for CompileStaticRule {
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

        let is_inner = self.classes.depth() > 0;
        self.classes.push(&class.name);
        if is_inner {
            return;
        }
        if matches!(class.kind, ClassKind::Interface | ClassKind::Annotation) {
            return;
        }

        let is_marked = class.annotations.iter().any(|annotation| {
            matches!(
                annotation.as_str(),
                "CompileStatic" | "GrailsCompileStatic" | "CompileDynamic"
            )
        });
        if is_marked {
            return;
        }

        sink.report(
            class.span,
            "Class should be marked with one of @GrailsCompileStatic, @CompileStatic or @CompileDynamic",
        );
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Class(_) = node {
            self.classes.pop();
        }
    }
}

/// Requires closure parameters to be declared explicitly instead of using the
/// implicit `it`.
pub struct ImplicitClosureParameterRule;

impl Rule for ImplicitClosureParameterRule {}

impl AstRule for ImplicitClosureParameterRule {
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
        let ExprKind::Closure {
            params,
            has_arrow,
            body,
        } = &expr.kind
        else {
            return;
        };
        let _ = has_arrow;
        let declares_it = params.iter().any(|param| param.name == "it");
        if declares_it {
            sink.report(
                expr.span,
                "By convention \"it\" should not be used as a closure parameter name.",
            );
            return;
        }
        if !params.is_empty() {
            return;
        }

        let mut collector = ItCollector {
            spans: Vec::new(),
            depth: 0,
        };
        walk_block(body, &mut collector);
        for span in collector.spans {
            sink.report(
                span,
                "By convention closure parameters should be specified explicitly.",
            );
        }
    }
}

struct ItCollector {
    spans: Vec<groovylint_core::Span>,
    depth: usize,
}

impl AstVisitor for ItCollector {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        match &expr.kind {
            ExprKind::Closure { .. } => self.depth += 1,
            ExprKind::Variable(name) if name == "it" && self.depth == 0 => {
                self.spans.push(expr.span);
            }
            _ => {}
        }
    }

    fn exit(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        if matches!(expr.kind, ExprKind::Closure { .. }) {
            self.depth = self.depth.saturating_sub(1);
        }
    }
}

/// Flags `if (!x) { x = y }` that could be written as an elvis assignment.
pub struct CouldBeElvisRule;

impl Rule for CouldBeElvisRule {}

impl AstRule for CouldBeElvisRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::If(if_stmt)) = node else {
            return;
        };
        let ExprKind::Unary {
            op: groovylint_syntax::ast::UnaryOp::Not,
            operand,
        } = &unparen(&if_stmt.condition).kind
        else {
            return;
        };
        let Some(assignment) = only_assignment(&if_stmt.then_branch) else {
            return;
        };
        let ExprKind::Assign {
            target,
            op: groovylint_syntax::ast::AssignOp::Assign,
            value,
        } = &assignment.kind
        else {
            return;
        };

        let source = context.source.text();
        let condition_text = expr_text(operand, source);
        let target_text = expr_text(target, source);
        if condition_text != target_text || condition_text.contains(' ') {
            return;
        }
        let value_text = expr_text(value, source);
        sink.report(
            if_stmt.span,
            format!(
                "Code could use elvis operator: {target_text} = {target_text} ?: {value_text} "
            ),
        );
    }
}

fn only_assignment(statement: &Stmt) -> Option<&Expr> {
    match statement {
        Stmt::Expr(expr) => Some(expr),
        Stmt::Block(block) if block.statements.len() == 1 => match &block.statements[0] {
            Stmt::Expr(expr) => Some(expr),
            _ => None,
        },
        _ => None,
    }
}

/// Flags ternaries whose condition and true branch are the same expression.
pub struct TernaryCouldBeElvisRule;

impl Rule for TernaryCouldBeElvisRule {}

impl AstRule for TernaryCouldBeElvisRule {
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
            is_elvis,
            condition,
            then_expr,
            else_expr,
        } = &expr.kind
        else {
            return;
        };
        if *is_elvis {
            return;
        }
        let source = context.source.text();
        if !same_expression(condition, then_expr, source) {
            return;
        }
        let whole = expr_text(expr, source);
        let then_text = expr_text(then_expr, source);
        let else_text = expr_text(else_expr, source);
        sink.report(
            expr.span,
            format!("{whole} can be simplified to {then_text} ?: {else_text}"),
        );
    }
}

fn same_expression(left: &Expr, right: &Expr, source: &str) -> bool {
    match (&unparen(left).kind, &unparen(right).kind) {
        (ExprKind::Variable(a), ExprKind::Variable(b)) => a == b,
        (ExprKind::Call { .. }, ExprKind::Call { .. }) => {
            expr_text(left, source) == expr_text(right, source)
        }
        _ => false,
    }
}

/// Flags comparisons that put a constant on the left-hand side.
pub struct InvertedConditionRule;

impl Rule for InvertedConditionRule {}

impl AstRule for InvertedConditionRule {
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
            ..
        } = &expr.kind
        else {
            return;
        };
        if !is_constant(left) {
            return;
        }
        let text = expr_text(left, context.source.text());
        sink.report(
            expr.span,
            format!(
                "{text} is a constant expression on the left side of a compare equals operation"
            ),
        );
    }
}

/// Requires a trailing comma in multi-line list and map literals.
#[derive(Default)]
pub struct TrailingCommaRule {
    check_list: Option<bool>,
    check_map: Option<bool>,
    ignore_single_element_list: Option<bool>,
    ignore_single_element_map: Option<bool>,
}

impl Rule for TrailingCommaRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        self.check_list = parse_bool(properties, "checkList")?;
        self.check_map = parse_bool(properties, "checkMap")?;
        self.ignore_single_element_list = parse_bool(properties, "ignoreSingleElementList")?;
        self.ignore_single_element_map = parse_bool(properties, "ignoreSingleElementMap")?;
        Ok(())
    }
}

fn parse_bool(
    properties: &crate::rule::RuleProperties,
    name: &str,
) -> Result<Option<bool>, String> {
    match properties.get(name) {
        None => Ok(None),
        Some(value) => match value.as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => Err(format!("property {name} must be true or false")),
        },
    }
}

impl AstRule for TrailingCommaRule {
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
        let text = context.source.text();
        let span_text = || text.get(expr.span.range()).unwrap_or_default();

        match &expr.kind {
            ExprKind::List(items) => {
                if self.check_list == Some(false) || items.is_empty() {
                    return;
                }
                if self.ignore_single_element_list != Some(false) && items.len() == 1 {
                    return;
                }
                let source = span_text();
                if !source.contains('\n') {
                    return;
                }
                let Some(last) = items.last() else {
                    return;
                };
                if line_of(context, last.span.start) == line_of(context, expr.span.end) {
                    return;
                }
                if ends_with_comma(text, last.span.end, expr.span.end) {
                    return;
                }
                sink.report(expr.span, "List should contain trailing comma.");
            }
            ExprKind::Map(entries) => {
                if self.check_map == Some(false) || entries.is_empty() {
                    return;
                }
                if self.ignore_single_element_map != Some(false) && entries.len() == 1 {
                    return;
                }
                let source = span_text();
                if !source.contains('\n') {
                    return;
                }
                let Some(last) = entries.last() else {
                    return;
                };
                if closes_right_after(context, last.1.span.end) {
                    return;
                }
                if ends_with_comma(text, last.1.span.end, expr.span.end) {
                    return;
                }
                sink.report(expr.span, "Map should contain trailing comma.");
            }
            _ => {}
        }
    }
}

/// Whether only a closing bracket follows the entry on its own line, which is how
/// `CodeNarc` treats a map that ends flush with its container.
fn line_of(context: &FileContext<'_>, offset: usize) -> usize {
    context.source.line_col(offset).line
}

fn closes_right_after(context: &FileContext<'_>, from: usize) -> bool {
    let line = line_of(context, from);
    let Some(text) = context.source.line_text(line) else {
        return false;
    };
    let column = context.source.line_col(from).column - 1;
    let rest = text.get(column..).unwrap_or_default();
    rest.trim_start().starts_with(']')
}

fn ends_with_comma(text: &str, from: usize, to: usize) -> bool {
    let Some(tail) = text.get(from..to) else {
        return false;
    };
    let chars = tail.chars().rev();
    for ch in chars {
        match ch {
            ',' => return true,
            ']' | '}' => return false,
            ch if ch.is_whitespace() => {}
            _ => return false,
        }
    }
    false
}

/// Descriptors for the additional convention rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "CompileStatic",
                "org.codenarc.rule.convention.CompileStaticRule",
                Category::Convention,
                Severity::Warning,
                "Classes should be annotated with a static-compilation annotation.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CompileStaticRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ImplicitClosureParameter",
                "org.codenarc.rule.convention.ImplicitClosureParameterRule",
                Category::Convention,
                Severity::Info,
                "Closure parameters should be declared explicitly.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ImplicitClosureParameterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CouldBeElvis",
                "org.codenarc.rule.convention.CouldBeElvisRule",
                Category::Convention,
                Severity::Info,
                "An if block that could be written as an elvis expression.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CouldBeElvisRule)),
        ),
        descriptor(
            RuleMeta::new(
                "TernaryCouldBeElvis",
                "org.codenarc.rule.convention.TernaryCouldBeElvisRule",
                Category::Convention,
                Severity::Info,
                "A ternary whose condition repeats the true branch.",
            ),
            || RuleInstanceImpl::Ast(Box::new(TernaryCouldBeElvisRule)),
        ),
        descriptor(
            RuleMeta::new(
                "InvertedCondition",
                "org.codenarc.rule.convention.InvertedConditionRule",
                Category::Convention,
                Severity::Info,
                "A constant on the left side of an equals comparison.",
            ),
            || RuleInstanceImpl::Ast(Box::new(InvertedConditionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "TrailingComma",
                "org.codenarc.rule.convention.TrailingCommaRule",
                Category::Convention,
                Severity::Info,
                "Multi-line list and map literals should end with a comma.",
            ),
            || RuleInstanceImpl::Ast(Box::new(TrailingCommaRule::default())),
        ),
    ]
}
