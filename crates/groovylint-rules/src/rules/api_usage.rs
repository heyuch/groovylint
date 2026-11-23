//! Rules about API usage patterns.

use groovylint_core::Severity;
use groovylint_syntax::ast::{Expr, ExprKind, Literal, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{call_with_receiver, expr_text, string_literal, unparen, variable_name};

fn call_expression(node: NodeRef<'_>) -> Option<&Expr> {
    let NodeRef::Expr(expr) = node else {
        return None;
    };
    Some(expr)
}

/// Detects `Boolean.getBoolean(String)`.
pub struct BooleanGetBooleanRule;

impl Rule for BooleanGetBooleanRule {}

impl AstRule for BooleanGetBooleanRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let Some(expr) = call_expression(node)
            && call_with_receiver(expr, "Boolean", "getBoolean", 1).is_some()
        {
            sink.report(
                expr.span,
                "Boolean.getBoolean(String) is a confusing API for reading System properties. Prefer the System.getProperty(String) API.",
            );
        }
    }
}

/// Detects `Class.forName(...)`.
pub struct ClassForNameRule;

impl Rule for ClassForNameRule {}

impl AstRule for ClassForNameRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let Some(expr) = call_expression(node)
            && (call_with_receiver(expr, "Class", "forName", 1).is_some()
                || call_with_receiver(expr, "Class", "forName", 3).is_some())
        {
            sink.report(
                expr.span,
                "Methods calls to Class.forName(...) can create resource leaks and should almost always be replaced with calls to ClassLoader.loadClass(...)",
            );
        }
    }
}

/// Detects `Integer.getInteger(...)`.
pub struct IntegerGetIntegerRule;

impl Rule for IntegerGetIntegerRule {}

impl AstRule for IntegerGetIntegerRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let Some(expr) = call_expression(node) else {
            return;
        };
        if call_with_receiver(expr, "Integer", "getInteger", 1).is_some() {
            sink.report(
                expr.span,
                "Integer.getInteger(String) is a confusing API for reading System properties. Prefer the System.getProperty(String) API.",
            );
        } else if call_with_receiver(expr, "Integer", "getInteger", 2).is_some() {
            sink.report(
                expr.span,
                "Integer.getInteger(String, Integer) is a confusing API for reading System properties. Prefer the System.getProperty(String) API.",
            );
        }
    }
}

/// Detects explicit garbage-collection calls.
pub struct ExplicitGarbageCollectionRule;

impl Rule for ExplicitGarbageCollectionRule {}

impl AstRule for ExplicitGarbageCollectionRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let Some(expr) = call_expression(node) else {
            return;
        };
        let explicit = call_with_receiver(expr, "System", "gc", 0).is_some()
            || call_with_receiver(expr, "System", "runFinalization", 0).is_some()
            || runtime_gc(expr);
        if explicit {
            sink.report(
                expr.span,
                "Garbage collection should not be explicitly forced",
            );
        }
    }
}

fn runtime_gc(expr: &Expr) -> bool {
    let ExprKind::Call {
        receiver: Some(receiver),
        name,
        args,
        ..
    } = &expr.kind
    else {
        return false;
    };
    if name != "gc" || !args.is_empty() {
        return false;
    }
    match &unparen(receiver).kind {
        ExprKind::Call {
            receiver: None,
            name,
            args,
            ..
        } => name == "getRuntime" && args.is_empty(),
        ExprKind::Property {
            receiver,
            name,
            safe: false,
        } => name == "runtime" && variable_name(receiver) == Some("Runtime"),
        _ => false,
    }
}

/// Detects `x.removeAll(x)`.
pub struct RemoveAllOnSelfRule;

impl Rule for RemoveAllOnSelfRule {}

impl AstRule for RemoveAllOnSelfRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let Some(expr) = call_expression(node) else {
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
        if name != "removeAll" || args.len() != 1 {
            return;
        }
        if expr_text(receiver, context.source.text()) == expr_text(&args[0], context.source.text())
        {
            sink.report(
                expr.span,
                format!(
                    "A call to {} can be replaced with {}.clear()",
                    expr_text(expr, context.source.text()),
                    expr_text(receiver, context.source.text())
                ),
            );
        }
    }
}

fn numeric_literal_is_floating(expr: &Expr) -> bool {
    let ExprKind::Literal(Literal::Number(raw)) = &unparen(expr).kind else {
        return false;
    };
    raw.contains('.')
        || raw.contains('e')
        || raw.contains('E')
        || raw.ends_with('d')
        || raw.ends_with('D')
        || raw.ends_with('f')
        || raw.ends_with('F')
}

/// Detects `new BigDecimal(doubleLiteral)`.
pub struct BigDecimalInstantiationRule;

impl Rule for BigDecimalInstantiationRule {}

impl AstRule for BigDecimalInstantiationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let Some(expr) = call_expression(node) else {
            return;
        };
        let ExprKind::New {
            type_name, args, ..
        } = &expr.kind
        else {
            return;
        };
        if type_name.rsplit('.').next() != Some("BigDecimal") {
            return;
        }
        let Some(first) = args.first() else {
            return;
        };
        if !numeric_literal_is_floating(first) {
            return;
        }
        sink.report(
            expr.span,
            format!(
                "Call to {} uses the double constructor and should probably be replaced with new BigDecimal(\"{}\")",
                expr_text(expr, context.source.text()),
                expr_text(first, context.source.text())
            ),
        );
    }
}

/// Detects Windows separators passed to `new File(...)`.
pub struct HardCodedWindowsFileSeparatorRule;

impl Rule for HardCodedWindowsFileSeparatorRule {}

impl AstRule for HardCodedWindowsFileSeparatorRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        for (argument, value) in file_constructor_arguments(node) {
            if value.contains('\\') {
                sink.report(argument.span, "The windows file separator is not portable");
            }
        }
    }
}

/// Detects Windows root paths passed to `new File(...)`.
pub struct HardCodedWindowsRootDirectoryRule;

impl Rule for HardCodedWindowsRootDirectoryRule {}

impl AstRule for HardCodedWindowsRootDirectoryRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        for (argument, value) in file_constructor_arguments(node) {
            let mut chars = value.chars();
            if chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
                && chars.next() == Some(':')
                && chars.next() == Some('\\')
            {
                sink.report(
                    argument.span,
                    format!(
                        "The file location {} is not portable",
                        value.chars().take(3).collect::<String>()
                    ),
                );
            }
        }
    }
}

fn file_constructor_arguments(node: NodeRef<'_>) -> Vec<(&Expr, String)> {
    let NodeRef::Expr(expr) = node else {
        return Vec::new();
    };
    let ExprKind::New {
        type_name, args, ..
    } = &expr.kind
    else {
        return Vec::new();
    };
    if type_name.rsplit('.').next() != Some("File") {
        return Vec::new();
    }
    args.iter()
        .filter_map(|argument| string_literal(argument).map(|value| (argument, value)))
        .collect()
}

/// Suggests `while` for a `for` loop with only a condition.
pub struct ForLoopShouldBeWhileLoopRule;

impl Rule for ForLoopShouldBeWhileLoopRule {}

impl AstRule for ForLoopShouldBeWhileLoopRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::For)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::For(for_stmt)) = node
            && for_stmt.init.is_none()
            && for_stmt.update.is_none()
            && for_stmt.condition.is_some()
        {
            sink.report(
                for_stmt.span,
                "The for loop can be simplified to a while loop",
            );
        }
    }
}

/// Detects duplicate constant keys in map literals.
pub struct DuplicateMapKeyRule;

impl Rule for DuplicateMapKeyRule {}

impl AstRule for DuplicateMapKeyRule {
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
        let ExprKind::Map(entries) = &expr.kind else {
            return;
        };
        let mut seen = Vec::new();
        for (key, _) in entries {
            let text = if crate::rules::util::is_constant(key) {
                expr_text(key, context.source.text())
            } else if let Some(name) = variable_name(key) {
                name.to_string()
            } else {
                continue;
            };
            if seen.contains(&text) {
                sink.report(key.span, format!("Key '{text}' is duplicated."));
            } else {
                seen.push(text);
            }
        }
    }
}

const META: [RuleMeta; 9] = [
    RuleMeta::new(
        "BooleanGetBoolean",
        "org.codenarc.rule.basic.BooleanGetBooleanRule",
        Category::Basic,
        Severity::Warning,
        "Checks for Boolean.getBoolean(String).",
    ),
    RuleMeta::new(
        "ClassForName",
        "org.codenarc.rule.basic.ClassForNameRule",
        Category::Basic,
        Severity::Warning,
        "Checks for Class.forName calls.",
    ),
    RuleMeta::new(
        "IntegerGetInteger",
        "org.codenarc.rule.basic.IntegerGetIntegerRule",
        Category::Basic,
        Severity::Warning,
        "Checks for Integer.getInteger calls.",
    ),
    RuleMeta::new(
        "ExplicitGarbageCollection",
        "org.codenarc.rule.basic.ExplicitGarbageCollectionRule",
        Category::Basic,
        Severity::Warning,
        "Checks for explicit garbage collection.",
    ),
    RuleMeta::new(
        "RemoveAllOnSelf",
        "org.codenarc.rule.basic.RemoveAllOnSelfRule",
        Category::Basic,
        Severity::Warning,
        "Checks for removeAll called with the receiver.",
    ),
    RuleMeta::new(
        "BigDecimalInstantiation",
        "org.codenarc.rule.basic.BigDecimalInstantiationRule",
        Category::Basic,
        Severity::Warning,
        "Checks for BigDecimal construction from floating literals.",
    ),
    RuleMeta::new(
        "HardCodedWindowsFileSeparator",
        "org.codenarc.rule.basic.HardCodedWindowsFileSeparatorRule",
        Category::Basic,
        Severity::Warning,
        "Checks for Windows file separators in File constructors.",
    ),
    RuleMeta::new(
        "HardCodedWindowsRootDirectory",
        "org.codenarc.rule.basic.HardCodedWindowsRootDirectoryRule",
        Category::Basic,
        Severity::Warning,
        "Checks for Windows root paths in File constructors.",
    ),
    RuleMeta::new(
        "ForLoopShouldBeWhileLoop",
        "org.codenarc.rule.basic.ForLoopShouldBeWhileLoopRule",
        Category::Basic,
        Severity::Info,
        "Checks for for loops that can be while loops.",
    ),
];

/// Descriptors for API-usage rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(META[0], || {
            RuleInstanceImpl::Ast(Box::new(BooleanGetBooleanRule))
        }),
        descriptor(META[1], || {
            RuleInstanceImpl::Ast(Box::new(ClassForNameRule))
        }),
        descriptor(META[2], || {
            RuleInstanceImpl::Ast(Box::new(IntegerGetIntegerRule))
        }),
        descriptor(META[3], || {
            RuleInstanceImpl::Ast(Box::new(ExplicitGarbageCollectionRule))
        }),
        descriptor(META[4], || {
            RuleInstanceImpl::Ast(Box::new(RemoveAllOnSelfRule))
        }),
        descriptor(META[5], || {
            RuleInstanceImpl::Ast(Box::new(BigDecimalInstantiationRule))
        }),
        descriptor(META[6], || {
            RuleInstanceImpl::Ast(Box::new(HardCodedWindowsFileSeparatorRule))
        }),
        descriptor(META[7], || {
            RuleInstanceImpl::Ast(Box::new(HardCodedWindowsRootDirectoryRule))
        }),
        descriptor(META[8], || {
            RuleInstanceImpl::Ast(Box::new(ForLoopShouldBeWhileLoopRule))
        }),
        descriptor(
            RuleMeta::new(
                "DuplicateMapKey",
                "org.codenarc.rule.basic.DuplicateMapKeyRule",
                Category::Basic,
                Severity::Warning,
                "Checks for duplicate constant map keys.",
            ),
            || RuleInstanceImpl::Ast(Box::new(DuplicateMapKeyRule)),
        ),
    ]
}
