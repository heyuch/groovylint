//! Empty declaration and control-flow rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassKind, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{expr_text, has_annotation, is_constant};

/// Detects classes with no methods, fields, or properties.
pub struct EmptyClassRule;

impl Rule for EmptyClassRule {}

impl AstRule for EmptyClassRule {
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
        let is_subclass = class
            .extends
            .as_deref()
            .is_some_and(|name| !matches!(name, "Object" | "java.lang.Object"));
        if class.kind != ClassKind::Class
            || class.is_abstract
            || is_subclass
            || !class.annotations.is_empty()
        {
            return;
        }
        let has_body = class.members.iter().any(|member| {
            matches!(
                member,
                groovylint_syntax::ast::Item::Method(_) | groovylint_syntax::ast::Item::Field(_)
            )
        });
        if !has_body {
            sink.report(
                class.span,
                format!(
                    "Class '{}' is empty (has no methods, fields or properties). Why would you need a class like this?",
                    class.name
                ),
            );
        }
    }
}

/// Detects empty methods that are not marked with `@Override`.
#[derive(Default)]
pub struct EmptyMethodRule {
    classes: Vec<ClassContext>,
}

struct ClassContext {
    name: String,
    kind: ClassKind,
    is_abstract: bool,
}

impl Rule for EmptyMethodRule {}

impl AstRule for EmptyMethodRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Method))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(class) => self.classes.push(ClassContext {
                name: class.name.clone(),
                kind: class.kind,
                is_abstract: class.is_abstract,
            }),
            NodeRef::Method(method) => {
                let Some(class) = self.classes.last() else {
                    return;
                };
                if class.kind != ClassKind::Class
                    || class.is_abstract
                    || method.name == class.name
                    || has_annotation(&method.annotations, "Override")
                    || has_annotation(&method.annotations, "Pointcut")
                {
                    return;
                }
                if method
                    .body
                    .as_ref()
                    .is_some_and(|body| body.statements.is_empty())
                {
                    sink.report(
                        method.span,
                        format!(
                            "The method {} is both empty and not marked with @Override",
                            method.name
                        ),
                    );
                }
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

/// Detects an empty `switch` statement.
pub struct EmptySwitchStatementRule;

impl Rule for EmptySwitchStatementRule {}

impl AstRule for EmptySwitchStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Switch)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::Switch(switch_stmt)) = node
            && switch_stmt.cases.is_empty()
        {
            sink.report(switch_stmt.span, "The switch statement is empty");
        }
    }
}

/// Detects an empty `synchronized` statement.
pub struct EmptySynchronizedStatementRule;

impl Rule for EmptySynchronizedStatementRule {}

impl AstRule for EmptySynchronizedStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Synchronized)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Stmt(Stmt::Synchronized(synchronized)) = node
            && synchronized.body.is_empty_block()
        {
            sink.report(synchronized.span, "The synchronized statement is empty");
        }
    }
}

/// Detects an empty static initializer.
#[derive(Default)]
pub struct EmptyStaticInitializerRule {
    classes: Vec<String>,
}

impl Rule for EmptyStaticInitializerRule {}

impl AstRule for EmptyStaticInitializerRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Initializer))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(class) => self.classes.push(class.name.clone()),
            NodeRef::Initializer(initializer)
                if initializer.is_static && initializer.body.statements.is_empty() =>
            {
                let class_name = self.classes.last().map_or("null", String::as_str);
                sink.report(
                    initializer.span,
                    format!(
                        "The class {class_name} has an empty static initializer. It is safe to delete it"
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
        if matches!(node, NodeRef::Class(_)) {
            self.classes.pop();
        }
    }
}

/// Detects an empty instance initializer.
#[derive(Default)]
pub struct EmptyInstanceInitializerRule {
    classes: Vec<String>,
}

impl Rule for EmptyInstanceInitializerRule {}

impl AstRule for EmptyInstanceInitializerRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Initializer))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(class) => self.classes.push(class.name.clone()),
            NodeRef::Initializer(initializer)
                if !initializer.is_static && initializer.body.statements.is_empty() =>
            {
                let class_name = self.classes.last().map_or("null", String::as_str);
                sink.report(
                    initializer.span,
                    format!(
                        "The class {class_name} defines an empty instance initializer. It is safe to delete it"
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
        if matches!(node, NodeRef::Class(_)) {
            self.classes.pop();
        }
    }
}

/// Detects duplicate constant `case` labels in a switch.
pub struct DuplicateCaseStatementRule;

impl Rule for DuplicateCaseStatementRule {}

impl AstRule for DuplicateCaseStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Switch)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Switch(switch_stmt)) = node else {
            return;
        };
        let mut seen = Vec::new();
        for case in &switch_stmt.cases {
            let Some(label) = &case.label else {
                continue;
            };
            if !is_constant(label) {
                continue;
            }
            let text = expr_text(label, context.source.text());
            if seen.contains(&text) {
                sink.report(
                    case.span,
                    format!("Duplicate case statements found in switch. {text} appears twice"),
                );
            } else {
                seen.push(text);
            }
        }
    }
}

const META: [RuleMeta; 7] = [
    RuleMeta::new(
        "EmptyClass",
        "org.codenarc.rule.basic.EmptyClassRule",
        Category::Basic,
        Severity::Warning,
        "Checks for empty classes.",
    ),
    RuleMeta::new(
        "EmptyMethod",
        "org.codenarc.rule.basic.EmptyMethodRule",
        Category::Basic,
        Severity::Warning,
        "Checks for empty methods.",
    ),
    RuleMeta::new(
        "EmptySwitchStatement",
        "org.codenarc.rule.basic.EmptySwitchStatementRule",
        Category::Basic,
        Severity::Warning,
        "Checks for empty switch statements.",
    ),
    RuleMeta::new(
        "EmptySynchronizedStatement",
        "org.codenarc.rule.basic.EmptySynchronizedStatementRule",
        Category::Basic,
        Severity::Warning,
        "Checks for empty synchronized statements.",
    ),
    RuleMeta::new(
        "EmptyStaticInitializer",
        "org.codenarc.rule.basic.EmptyStaticInitializerRule",
        Category::Basic,
        Severity::Warning,
        "Checks for empty static initializers.",
    ),
    RuleMeta::new(
        "EmptyInstanceInitializer",
        "org.codenarc.rule.basic.EmptyInstanceInitializerRule",
        Category::Basic,
        Severity::Warning,
        "Checks for empty instance initializers.",
    ),
    RuleMeta::new(
        "DuplicateCaseStatement",
        "org.codenarc.rule.basic.DuplicateCaseStatementRule",
        Category::Basic,
        Severity::Warning,
        "Checks for duplicate switch case labels.",
    ),
];

/// Descriptors for empty declaration rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(META[0], || RuleInstanceImpl::Ast(Box::new(EmptyClassRule))),
        descriptor(META[1], || {
            RuleInstanceImpl::Ast(Box::new(EmptyMethodRule::default()))
        }),
        descriptor(META[2], || {
            RuleInstanceImpl::Ast(Box::new(EmptySwitchStatementRule))
        }),
        descriptor(META[3], || {
            RuleInstanceImpl::Ast(Box::new(EmptySynchronizedStatementRule))
        }),
        descriptor(META[4], || {
            RuleInstanceImpl::Ast(Box::new(EmptyStaticInitializerRule::default()))
        }),
        descriptor(META[5], || {
            RuleInstanceImpl::Ast(Box::new(EmptyInstanceInitializerRule::default()))
        }),
        descriptor(META[6], || {
            RuleInstanceImpl::Ast(Box::new(DuplicateCaseStatementRule))
        }),
    ]
}
