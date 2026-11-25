//! Remaining `CodeNarc` rules: concurrency, JDBC, serialization and a few
//! naming and unused-code rules.

use std::collections::{BTreeMap, BTreeSet};

use groovylint_core::Severity;
use groovylint_syntax::ast::{Expr, ExprKind, Item, MethodDecl, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};
use crate::rules::util::{has_annotation, is_serializable, simple_name, unparen, variable_name};

fn statement_is(statement: &Stmt) -> Option<&Expr> {
    match statement {
        Stmt::Expr(expr) => Some(expr),
        _ => None,
    }
}

fn call_on<'a>(expr: &'a Expr, receiver: &str, method: &str) -> Option<&'a [Expr]> {
    let ExprKind::Call {
        receiver: Some(actual),
        name,
        args,
        ..
    } = &expr.kind
    else {
        return None;
    };
    if name != method || variable_name(actual) != Some(receiver) {
        return None;
    }
    Some(args)
}

/// Flags `Thread.yield()`.
pub struct ThreadYieldRule;

impl Rule for ThreadYieldRule {}

impl AstRule for ThreadYieldRule {
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
        if call_on(expr, "Thread", "yield").is_some() {
            sink.report(
                expr.span,
                "Calling Thread.yield() has no useful guaranteed semantics",
            );
        }
    }
}

/// Flags `System.runFinalizersOnExit()`.
pub struct SystemRunFinalizersOnExitRule;

impl Rule for SystemRunFinalizersOnExitRule {}

impl AstRule for SystemRunFinalizersOnExitRule {
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
        if call_on(expr, "System", "runFinalizersOnExit").is_some() {
            sink.report(
                expr.span,
                "System.runFinalizersOnExit() is deprecated and unsafe",
            );
        }
    }
}

/// Flags `notify()` instead of `notifyAll()`.
pub struct UseOfNotifyMethodRule;

impl Rule for UseOfNotifyMethodRule {}

impl AstRule for UseOfNotifyMethodRule {
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
        if name != "notify" || !args.is_empty() {
            return;
        }
        sink.report(expr.span, "Call notifyAll() instead of notify()");
    }
}

/// Flags `wait()` outside a while loop.
pub struct WaitOutsideOfWhileLoopRule;

impl Rule for WaitOutsideOfWhileLoopRule {}

impl AstRule for WaitOutsideOfWhileLoopRule {
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
        if name != "wait" || !args.is_empty() {
            return;
        }
        if inside_while_loop(context, expr) {
            return;
        }
        sink.report(expr.span, "Calls to wait() must be within a while loop");
    }
}

fn inside_while_loop(context: &FileContext<'_>, expr: &Expr) -> bool {
    let mut finder = WhileFinder {
        target: expr.span.start,
        found: false,
    };
    crate::traverse::walk_script(context.script, &mut finder);
    finder.found
}

struct WhileFinder {
    target: usize,
    found: bool,
}

impl crate::traverse::AstVisitor for WhileFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Stmt(Stmt::While(while_stmt)) = node {
            let range = while_stmt.span.range();
            if range.contains(&self.target) {
                self.found = true;
            }
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags synchronized statements nested inside other synchronized statements.
pub struct NestedSynchronizationRule;

impl Rule for NestedSynchronizationRule {}

impl AstRule for NestedSynchronizationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Synchronized)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Synchronized(inner)) = node else {
            return;
        };
        let mut finder = SyncFinder {
            target: inner.span.start,
            outer: None,
        };
        crate::traverse::walk_script(context.script, &mut finder);
        if finder.outer.is_some() {
            sink.report(
                inner.span,
                "Nested synchronized statements should be avoided",
            );
        }
    }
}

struct SyncFinder {
    target: usize,
    outer: Option<groovylint_core::Span>,
}

impl crate::traverse::AstVisitor for SyncFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Stmt(Stmt::Synchronized(sync)) = node {
            let range = sync.span.range();
            if range.contains(&self.target) && sync.span.start != self.target {
                self.outer = Some(sync.span);
            }
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags `synchronized` methods.
pub struct SynchronizedMethodRule;

impl Rule for SynchronizedMethodRule {}

impl AstRule for SynchronizedMethodRule {
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
        if !method.is_synchronized {
            return;
        }
        sink.report(
            method.span,
            "Synchronized methods make the synchronization policy public",
        );
    }
}

/// Flags `synchronized (this.getClass())`.
pub struct SynchronizedOnGetClassRule;

impl Rule for SynchronizedOnGetClassRule {}

impl AstRule for SynchronizedOnGetClassRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Synchronized)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Synchronized(sync)) = node else {
            return;
        };
        let ExprKind::Call { name, .. } = &unparen(&sync.expression).kind else {
            return;
        };
        if name != "getClass" {
            return;
        }
        sink.report(
            sync.span,
            "Synchronization on getClass() rather than a class literal is not reliable",
        );
    }
}

/// Flags `synchronized` on strings.
pub struct SynchronizedOnStringRule;

impl Rule for SynchronizedOnStringRule {}

impl AstRule for SynchronizedOnStringRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Synchronized)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Synchronized(sync)) = node else {
            return;
        };
        if !matches!(
            &unparen(&sync.expression).kind,
            ExprKind::Literal(groovylint_syntax::ast::Literal::String(_))
        ) {
            return;
        }
        sink.report(
            sync.span,
            "Synchronizing on a String literal can lead to deadlock",
        );
    }
}

/// Flags `synchronized` on boxed primitives.
pub struct SynchronizedOnBoxedPrimitiveRule;

impl Rule for SynchronizedOnBoxedPrimitiveRule {}

impl AstRule for SynchronizedOnBoxedPrimitiveRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Synchronized)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Synchronized(sync)) = node else {
            return;
        };
        if !matches!(
            &unparen(&sync.expression).kind,
            ExprKind::Literal(groovylint_syntax::ast::Literal::Number(_))
        ) {
            return;
        }
        sink.report(
            sync.span,
            "Synchronizing on a boxed primitive can share a lock with unrelated code",
        );
    }
}

/// Flags `synchronized` on a `ReentrantLock` field.
pub struct SynchronizedOnReentrantLockRule;

impl Rule for SynchronizedOnReentrantLockRule {}

impl AstRule for SynchronizedOnReentrantLockRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Class(class) = node else {
            return;
        };
        let lock_fields: Vec<&str> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Field(field)
                    if field
                        .type_name
                        .as_deref()
                        .is_some_and(|name| simple_name(name) == "ReentrantLock") =>
                {
                    Some(field.name.as_str())
                }
                _ => None,
            })
            .collect();
        if lock_fields.is_empty() {
            return;
        }
        let mut finder = SyncTargetFinder {
            targets: lock_fields.iter().map(ToString::to_string).collect(),
            spans: Vec::new(),
        };
        crate::traverse::walk_script(context.script, &mut finder);
        for span in finder.spans {
            sink.report(
                span,
                "Synchronizing on a ReentrantLock field is almost never intended",
            );
        }
    }
}

struct SyncTargetFinder {
    targets: BTreeSet<String>,
    spans: Vec<groovylint_core::Span>,
}

impl crate::traverse::AstVisitor for SyncTargetFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Stmt(Stmt::Synchronized(sync)) = node else {
            return;
        };
        if let Some(name) = variable_name(&sync.expression)
            && self.targets.contains(name)
        {
            self.spans.push(sync.span);
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags `readObject` methods that are synchronized in serializable classes.
pub struct SynchronizedReadObjectMethodRule;

impl Rule for SynchronizedReadObjectMethodRule {}

impl AstRule for SynchronizedReadObjectMethodRule {
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
        if !is_serializable(class) {
            return;
        }
        for member in &class.members {
            let Item::Method(method) = member else {
                continue;
            };
            if method.name != "readObject" || !method.is_synchronized {
                continue;
            }
            sink.report(
                method.span,
                "A deserialized object is only reachable by one thread, so readObject() does not need to be synchronized",
            );
        }
    }
}

macro_rules! static_field_type_rule {
    ($struct_name:ident, $rule:literal, $types:expr, $message:literal, $description:literal) => {
        #[doc = $description]
        pub struct $struct_name;

        impl Rule for $struct_name {}

        impl AstRule for $struct_name {
            fn interests(&self) -> NodeMask {
                NodeMask::of(NodeKind::Field)
            }

            fn enter(
                &mut self,
                node: NodeRef<'_>,
                _context: &FileContext<'_>,
                sink: &mut DiagnosticSink<'_>,
            ) {
                let NodeRef::Field(field) = node else {
                    return;
                };
                if !field.is_static {
                    return;
                }
                let Some(type_name) = &field.type_name else {
                    return;
                };
                let types: &[&str] = &$types;
                if !types.contains(&simple_name(type_name)) {
                    return;
                }
                sink.report(field.span, $message.to_string());
            }
        }
    };
}

static_field_type_rule!(
    StaticCalendarFieldRule,
    "StaticCalendarField",
    ["Calendar", "GregorianCalendar"],
    "Calendar objects should not be used as static fields",
    "Flags static `Calendar` fields."
);
static_field_type_rule!(
    StaticDateFormatFieldRule,
    "StaticDateFormatField",
    ["DateFormat"],
    "DateFormat objects should not be used as static fields",
    "Flags static `DateFormat` fields."
);
static_field_type_rule!(
    StaticSimpleDateFormatFieldRule,
    "StaticSimpleDateFormatField",
    ["SimpleDateFormat"],
    "SimpleDateFormat objects should not be used as static fields",
    "Flags static `SimpleDateFormat` fields."
);
static_field_type_rule!(
    StaticMatcherFieldRule,
    "StaticMatcherField",
    ["Matcher"],
    "Matcher objects should not be used as static fields",
    "Flags static `Matcher` fields."
);
static_field_type_rule!(
    StaticConnectionRule,
    "StaticConnection",
    ["Connection"],
    "Database connections should not be used as static fields",
    "Flags static `Connection` fields."
);
static_field_type_rule!(
    ThreadGroupRule,
    "ThreadGroup",
    ["ThreadGroup"],
    "Avoid using ThreadGroup",
    "Flags `ThreadGroup` fields."
);

/// Flags non-final `ThreadLocal` fields.
pub struct ThreadLocalNotStaticFinalRule;

impl Rule for ThreadLocalNotStaticFinalRule {}

impl AstRule for ThreadLocalNotStaticFinalRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Field(field) = node else {
            return;
        };
        let is_thread_local = field
            .type_name
            .as_deref()
            .is_some_and(|name| simple_name(name) == "ThreadLocal");
        if !is_thread_local {
            return;
        }
        if field.is_static && field.is_final {
            return;
        }
        sink.report(
            field.span,
            "ThreadLocal fields should be declared static and final",
        );
    }
}

/// Flags volatile array fields.
pub struct VolatileArrayFieldRule;

impl Rule for VolatileArrayFieldRule {}

impl AstRule for VolatileArrayFieldRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Field(field) = node else {
            return;
        };
        if !field.is_volatile {
            return;
        }
        let is_array = field
            .type_name
            .as_deref()
            .is_some_and(|name| name.ends_with("[]"));
        if is_array {
            sink.report(
                field.span,
                "Volatile array fields are unsafe because array elements are not volatile",
            );
        }
    }
}

/// Flags volatile `long` and `double` fields.
pub struct VolatileLongOrDoubleFieldRule;

impl Rule for VolatileLongOrDoubleFieldRule {}

impl AstRule for VolatileLongOrDoubleFieldRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Field(field) = node else {
            return;
        };
        if !field.is_volatile {
            return;
        }
        let is_long_or_double = field
            .type_name
            .as_deref()
            .is_some_and(|name| matches!(simple_name(name), "long" | "double" | "Long" | "Double"));
        if is_long_or_double {
            sink.report(
                field.span,
                "Volatile long and double fields are not reliably atomic",
            );
        }
    }
}

/// Flags busy waiting on a loop that only calls `sleep`.
pub struct BusyWaitRule;

impl Rule for BusyWaitRule {}

impl AstRule for BusyWaitRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::While)
            .union(NodeMask::of(NodeKind::For))
            .union(NodeMask::of(NodeKind::ForEach))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let body = match node {
            NodeRef::Stmt(Stmt::While(while_stmt)) => &while_stmt.body,
            NodeRef::Stmt(Stmt::For(for_stmt)) => &for_stmt.body,
            NodeRef::Stmt(Stmt::ForEach(for_each)) => &for_each.body,
            _ => return,
        };
        let Stmt::Block(block) = body.as_ref() else {
            return;
        };
        let [Stmt::Expr(expr)] = block.statements.as_slice() else {
            return;
        };
        let ExprKind::Call { name, args, .. } = &expr.kind else {
            return;
        };
        if name == "sleep" && (args.len() == 1 || args.len() == 2) {
            sink.report(
                expr.span,
                "Busy wait detected; use a lock or gate from java.util.concurrent",
            );
        }
    }
}

/// Flags `this` escaping from a constructor.
pub struct ThisReferenceEscapesConstructorRule;

impl Rule for ThisReferenceEscapesConstructorRule {}

impl AstRule for ThisReferenceEscapesConstructorRule {
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
        for member in &class.members {
            let Item::Method(method) = member else {
                continue;
            };
            if method.name != class.name {
                continue;
            }
            let Some(body) = &method.body else {
                continue;
            };
            for statement in &body.statements {
                let Some(expr) = statement_is(statement) else {
                    continue;
                };
                if passes_this(expr) {
                    sink.report(
                        expr.span,
                        "Passing 'this' from a constructor exposes a partially constructed object",
                    );
                }
            }
        }
    }
}

fn passes_this(expr: &Expr) -> bool {
    let ExprKind::Call { args, .. } = &expr.kind else {
        return false;
    };
    args.iter()
        .any(|arg| matches!(unparen(arg).kind, ExprKind::This))
}

/// Flags fields of serializable classes whose types are not serializable.
pub struct NonSerializableFieldInSerializableClassRule;

impl Rule for NonSerializableFieldInSerializableClassRule {}

impl AstRule for NonSerializableFieldInSerializableClassRule {
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
        if !is_serializable(class) {
            return;
        }
        for member in &class.members {
            let Item::Field(field) = member else {
                continue;
            };
            if field.is_static || field.is_transient {
                continue;
            }
            let Some(type_name) = &field.type_name else {
                continue;
            };
            let bare = simple_name(type_name).replace("[]", "");
            let simple = simple_name(&bare);
            if NON_SERIALIZABLE.contains(&simple) {
                sink.report(
                    field.span,
                    format!(
                        "The field {} of serializable class {} has a non-serializable type {simple}",
                        field.name, class.name
                    ),
                );
            }
        }
    }
}

const NON_SERIALIZABLE: &[&str] = &[
    "Object",
    "Thread",
    "ThreadLocal",
    "InputStream",
    "OutputStream",
    "Reader",
    "Writer",
    "Connection",
    "Socket",
];

/// Requires `serialVersionUID` in serializable classes.
pub struct SerializableClassMustDefineSerialVersionUIDRule;

impl Rule for SerializableClassMustDefineSerialVersionUIDRule {}

impl AstRule for SerializableClassMustDefineSerialVersionUIDRule {
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
        if !is_serializable(class) {
            return;
        }
        let declared = class
            .members
            .iter()
            .any(|member| matches!(member, Item::Field(field) if field.name == "serialVersionUID"));
        if declared {
            return;
        }
        sink.report(
            class.span,
            format!(
                "The serializable class {} does not define a serialVersionUID",
                class.name
            ),
        );
    }
}

/// Flags unused parameters of non-private methods.
pub struct UnusedMethodParameterRule;

impl Rule for UnusedMethodParameterRule {}

impl AstRule for UnusedMethodParameterRule {
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
        if method.is_private || method.is_abstract {
            return;
        }
        if has_annotation(&method.annotations, "Override")
            || has_annotation(&method.annotations, "Pointcut")
        {
            return;
        }
        if method.name == "main" {
            return;
        }
        let Some(body) = &method.body else {
            return;
        };
        let Some(text) = context.source.text().get(body.span.range()) else {
            return;
        };
        for param in &method.params {
            if param.name.starts_with("ignore") || param.name == "_" {
                continue;
            }
            if text_contains_identifier(text, &param.name) {
                continue;
            }
            sink.report(
                param.span,
                format!(
                    "The parameter {} of method {} is not used",
                    param.name, method.name
                ),
            );
        }
    }
}

fn text_contains_identifier(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let target = name.as_bytes();
    let mut index = 0;
    while index + target.len() <= bytes.len() {
        if &bytes[index..index + target.len()] == target {
            let before_ok = index == 0
                || !(bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_');
            let after = bytes.get(index + target.len()).copied();
            let after_ok = after.is_none_or(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'));
            if before_ok && after_ok {
                return true;
            }
        }
        index += 1;
    }
    false
}

/// Flags methods with a high cyclomatic complexity.
#[derive(Default)]
pub struct CrapMetricRule;

impl Rule for CrapMetricRule {}

impl AstRule for CrapMetricRule {
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
        let Some(body) = &method.body else {
            return;
        };
        let complexity = cyclomatic_complexity(body);
        if complexity < 30 {
            return;
        }
        let _ = context;
        sink.report(
            method.span,
            format!(
                "The method {} has a cyclomatic complexity of {complexity}",
                method.name
            ),
        );
    }
}

fn cyclomatic_complexity(block: &groovylint_syntax::ast::Block) -> usize {
    struct Counter {
        count: usize,
    }

    impl crate::traverse::AstVisitor for Counter {
        fn enter(&mut self, node: NodeRef<'_>) {
            match node {
                NodeRef::Stmt(
                    Stmt::If(_)
                    | Stmt::For(_)
                    | Stmt::ForEach(_)
                    | Stmt::While(_)
                    | Stmt::DoWhile(_)
                    | Stmt::Try(_),
                ) => self.count += 1,
                NodeRef::Expr(expr) => {
                    if matches!(
                        expr.kind,
                        ExprKind::Binary {
                            op: groovylint_syntax::ast::BinaryOp::And
                                | groovylint_syntax::ast::BinaryOp::Or,
                            ..
                        }
                    ) {
                        self.count += 1;
                    }
                }
                _ => {}
            }
        }

        fn exit(&mut self, _node: NodeRef<'_>) {}
    }

    let mut counter = Counter { count: 1 };
    crate::traverse::walk_block(block, &mut counter);
    counter.count
}

/// Flags script assignments that create implicit bindings.
pub struct NoScriptBindingsRule;

impl Rule for NoScriptBindingsRule {}

impl FileRule for NoScriptBindingsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut declared = BTreeSet::new();
        collect_top_level_declarations(&context.script.items, &mut declared);
        for item in &context.script.items {
            if let Item::Statement(statement) = item {
                let mut finder = ScriptBindingFinder {
                    declared: &declared,
                    violations: Vec::new(),
                };
                crate::traverse::walk_stmt(statement, &mut finder);
                for span in finder.violations {
                    sink.report(
                        span,
                        "The script variable has no type declaration and becomes a script binding",
                    );
                }
            }
        }
    }
}

fn collect_top_level_declarations(items: &[Item], declared: &mut BTreeSet<String>) {
    for item in items {
        if let Item::Statement(statement) = item {
            let mut collector = DeclarationCollector { names: declared };
            crate::traverse::walk_stmt(statement, &mut collector);
        }
    }
}

struct DeclarationCollector<'a> {
    names: &'a mut BTreeSet<String>,
}

impl crate::traverse::AstVisitor for DeclarationCollector<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Stmt(Stmt::VarDecl(decl)) = node {
            self.names.insert(decl.name.clone());
        }
    }
    fn exit(&mut self, _node: NodeRef<'_>) {}
}

struct ScriptBindingFinder<'a> {
    declared: &'a BTreeSet<String>,
    violations: Vec<groovylint_core::Span>,
}

impl crate::traverse::AstVisitor for ScriptBindingFinder<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Assign {
            target,
            op: groovylint_syntax::ast::AssignOp::Assign,
            ..
        } = &expr.kind
        else {
            return;
        };
        let Some(name) = variable_name(target) else {
            return;
        };
        if !self.declared.contains(name) {
            self.violations.push(expr.span);
        }
    }
    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags getter/setter pairs with inconsistent lock annotations.
pub struct InconsistentPropertyLockingRule;

impl Rule for InconsistentPropertyLockingRule {}

impl FileRule for InconsistentPropertyLockingRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_property_guards(context.script, sink, GuardKind::Lock);
    }
}

/// Flags getter/setter pairs with inconsistent synchronization.
pub struct InconsistentPropertySynchronizationRule;

impl Rule for InconsistentPropertySynchronizationRule {}

impl FileRule for InconsistentPropertySynchronizationRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_property_guards(context.script, sink, GuardKind::Synchronized);
    }
}

#[derive(Clone, Copy)]
enum GuardKind {
    Lock,
    Synchronized,
}

fn check_property_guards(
    script: &groovylint_syntax::ast::Script,
    sink: &mut DiagnosticSink<'_>,
    kind: GuardKind,
) {
    for item in &script.items {
        let Item::Class(class) = item else {
            continue;
        };
        let mut getters = BTreeMap::<String, (&MethodDecl, bool)>::new();
        let mut setters = BTreeMap::<String, (&MethodDecl, bool)>::new();
        for member in &class.members {
            let Item::Method(method) = member else {
                continue;
            };
            let name = method.name.as_str();
            let is_getter = method.params.is_empty()
                && ((name.starts_with("get") && name.len() > 3)
                    || (name.starts_with("is") && name.len() > 2));
            let is_setter = method.params.len() == 1 && name.starts_with("set") && name.len() > 3;
            if !is_getter && !is_setter {
                continue;
            }
            let property = property_name(name);
            let guarded = match kind {
                GuardKind::Lock => {
                    has_annotation(&method.annotations, "WithReadLock")
                        || has_annotation(&method.annotations, "WithWriteLock")
                }
                GuardKind::Synchronized => {
                    method.is_synchronized || has_annotation(&method.annotations, "Synchronized")
                }
            };
            let target = if method.params.is_empty() {
                &mut getters
            } else {
                &mut setters
            };
            target.insert(property, (method, guarded));
        }
        for (property, (getter, getter_guarded)) in &getters {
            let Some((setter, setter_guarded)) = setters.get(property) else {
                continue;
            };
            if getter_guarded == setter_guarded {
                continue;
            }
            let unguarded = if *getter_guarded { setter } else { getter };
            sink.report(
                unguarded.span,
                format!(
                    "The getter and setter for property {property} do not use the same synchronization or lock"
                ),
            );
        }
    }
}

fn property_name(method: &str) -> String {
    let name = method
        .strip_prefix("get")
        .or_else(|| method.strip_prefix("set"))
        .or_else(|| method.strip_prefix("is"))
        .unwrap_or(method);
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_ascii_lowercase(), chars.as_str())
    })
}

/// Flags double-checked locking.
pub struct DoubleCheckedLockingRule;

impl Rule for DoubleCheckedLockingRule {}

impl AstRule for DoubleCheckedLockingRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::If(outer)) = node else {
            return;
        };
        let Some(variable) = null_check_variable(&outer.condition) else {
            return;
        };
        let Some(synchronized) = single_synchronized(&outer.then_branch) else {
            return;
        };
        let Some(inner) = single_if(&synchronized.body) else {
            return;
        };
        if null_check_variable(&inner.condition) != Some(variable) {
            return;
        }
        let Some(assignment) = single_statement(&inner.then_branch) else {
            return;
        };
        let Stmt::Expr(expr) = assignment else {
            return;
        };
        let ExprKind::Assign { target, .. } = &expr.kind else {
            return;
        };
        if variable_name(target) != Some(variable) {
            return;
        }
        sink.report(
            expr.span,
            format!(
                "Double checked locking detected for variable {variable}; use robust lazy initialization"
            ),
        );
    }
}

fn single_statement(statement: &Stmt) -> Option<&Stmt> {
    match statement {
        Stmt::Block(block) if block.statements.len() == 1 => block.statements.first(),
        _ => Some(statement),
    }
}

fn single_synchronized(statement: &Stmt) -> Option<&groovylint_syntax::ast::SynchronizedStmt> {
    match single_statement(statement)? {
        Stmt::Synchronized(synchronized) => Some(synchronized),
        _ => None,
    }
}

fn single_if(statement: &Stmt) -> Option<&groovylint_syntax::ast::IfStmt> {
    match single_statement(statement)? {
        Stmt::If(if_stmt) => Some(if_stmt),
        _ => None,
    }
}

fn null_check_variable(expr: &Expr) -> Option<&str> {
    let ExprKind::Binary {
        op: groovylint_syntax::ast::BinaryOp::Eq,
        left,
        right,
    } = &expr.kind
    else {
        return None;
    };
    if matches!(
        unparen(left).kind,
        ExprKind::Literal(groovylint_syntax::ast::Literal::Null)
    ) {
        return variable_name(right);
    }
    if matches!(
        unparen(right).kind,
        ExprKind::Literal(groovylint_syntax::ast::Literal::Null)
    ) {
        return variable_name(left);
    }
    None
}

/// Flags synchronization on `this`.
pub struct SynchronizedOnThisRule;

impl Rule for SynchronizedOnThisRule {}

impl AstRule for SynchronizedOnThisRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Synchronized)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Synchronized(synchronized)) = node else {
            return;
        };
        if matches!(unparen(&synchronized.expression).kind, ExprKind::This) {
            sink.report(
                synchronized.span,
                "Synchronizing on this is unsafe; use a private lock object",
            );
        }
    }
}

/// Descriptors for the concurrency rules.
#[must_use]
pub fn concurrency_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "SynchronizedOnThis",
                "org.codenarc.rule.concurrency.SynchronizedOnThisRule",
                Category::Concurrency,
                Severity::Warning,
                "Avoid synchronizing on this.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedOnThisRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BusyWait",
                "org.codenarc.rule.concurrency.BusyWaitRule",
                Category::Concurrency,
                Severity::Info,
                "BusyWait concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BusyWaitRule)),
        ),
        descriptor(
            RuleMeta::new(
                "DoubleCheckedLocking",
                "org.codenarc.rule.concurrency.DoubleCheckedLockingRule",
                Category::Concurrency,
                Severity::Info,
                "DoubleCheckedLocking concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(DoubleCheckedLockingRule)),
        ),
        descriptor(
            RuleMeta::new(
                "InconsistentPropertyLocking",
                "org.codenarc.rule.concurrency.InconsistentPropertyLockingRule",
                Category::Concurrency,
                Severity::Info,
                "InconsistentPropertyLocking concurrency rule.",
            ),
            || RuleInstanceImpl::File(Box::new(InconsistentPropertyLockingRule)),
        ),
        descriptor(
            RuleMeta::new(
                "InconsistentPropertySynchronization",
                "org.codenarc.rule.concurrency.InconsistentPropertySynchronizationRule",
                Category::Concurrency,
                Severity::Info,
                "InconsistentPropertySynchronization concurrency rule.",
            ),
            || RuleInstanceImpl::File(Box::new(InconsistentPropertySynchronizationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "NestedSynchronization",
                "org.codenarc.rule.concurrency.NestedSynchronizationRule",
                Category::Concurrency,
                Severity::Info,
                "NestedSynchronization concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(NestedSynchronizationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "NoScriptBindings",
                "org.codenarc.rule.concurrency.NoScriptBindingsRule",
                Category::Concurrency,
                Severity::Info,
                "NoScriptBindings concurrency rule.",
            ),
            || RuleInstanceImpl::File(Box::new(NoScriptBindingsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticCalendarField",
                "org.codenarc.rule.concurrency.StaticCalendarFieldRule",
                Category::Concurrency,
                Severity::Info,
                "StaticCalendarField concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(StaticCalendarFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticConnection",
                "org.codenarc.rule.concurrency.StaticConnectionRule",
                Category::Concurrency,
                Severity::Info,
                "StaticConnection concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(StaticConnectionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticDateFormatField",
                "org.codenarc.rule.concurrency.StaticDateFormatFieldRule",
                Category::Concurrency,
                Severity::Info,
                "StaticDateFormatField concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(StaticDateFormatFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticMatcherField",
                "org.codenarc.rule.concurrency.StaticMatcherFieldRule",
                Category::Concurrency,
                Severity::Info,
                "StaticMatcherField concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(StaticMatcherFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticSimpleDateFormatField",
                "org.codenarc.rule.concurrency.StaticSimpleDateFormatFieldRule",
                Category::Concurrency,
                Severity::Info,
                "StaticSimpleDateFormatField concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(StaticSimpleDateFormatFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SynchronizedMethod",
                "org.codenarc.rule.concurrency.SynchronizedMethodRule",
                Category::Concurrency,
                Severity::Info,
                "SynchronizedMethod concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SynchronizedOnBoxedPrimitive",
                "org.codenarc.rule.concurrency.SynchronizedOnBoxedPrimitiveRule",
                Category::Concurrency,
                Severity::Info,
                "SynchronizedOnBoxedPrimitive concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedOnBoxedPrimitiveRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SynchronizedOnGetClass",
                "org.codenarc.rule.concurrency.SynchronizedOnGetClassRule",
                Category::Concurrency,
                Severity::Info,
                "SynchronizedOnGetClass concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedOnGetClassRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SynchronizedOnReentrantLock",
                "org.codenarc.rule.concurrency.SynchronizedOnReentrantLockRule",
                Category::Concurrency,
                Severity::Info,
                "SynchronizedOnReentrantLock concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedOnReentrantLockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SynchronizedOnString",
                "org.codenarc.rule.concurrency.SynchronizedOnStringRule",
                Category::Concurrency,
                Severity::Info,
                "SynchronizedOnString concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedOnStringRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SynchronizedReadObjectMethod",
                "org.codenarc.rule.concurrency.SynchronizedReadObjectMethodRule",
                Category::Concurrency,
                Severity::Info,
                "SynchronizedReadObjectMethod concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SynchronizedReadObjectMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SystemRunFinalizersOnExit",
                "org.codenarc.rule.concurrency.SystemRunFinalizersOnExitRule",
                Category::Concurrency,
                Severity::Info,
                "SystemRunFinalizersOnExit concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SystemRunFinalizersOnExitRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThisReferenceEscapesConstructor",
                "org.codenarc.rule.concurrency.ThisReferenceEscapesConstructorRule",
                Category::Concurrency,
                Severity::Info,
                "ThisReferenceEscapesConstructor concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThisReferenceEscapesConstructorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThreadGroup",
                "org.codenarc.rule.concurrency.ThreadGroupRule",
                Category::Concurrency,
                Severity::Info,
                "ThreadGroup concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThreadGroupRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThreadLocalNotStaticFinal",
                "org.codenarc.rule.concurrency.ThreadLocalNotStaticFinalRule",
                Category::Concurrency,
                Severity::Info,
                "ThreadLocalNotStaticFinal concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThreadLocalNotStaticFinalRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThreadYield",
                "org.codenarc.rule.concurrency.ThreadYieldRule",
                Category::Concurrency,
                Severity::Info,
                "ThreadYield concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThreadYieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseOfNotifyMethod",
                "org.codenarc.rule.concurrency.UseOfNotifyMethodRule",
                Category::Concurrency,
                Severity::Info,
                "UseOfNotifyMethod concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseOfNotifyMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "VolatileArrayField",
                "org.codenarc.rule.concurrency.VolatileArrayFieldRule",
                Category::Concurrency,
                Severity::Info,
                "VolatileArrayField concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(VolatileArrayFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "VolatileLongOrDoubleField",
                "org.codenarc.rule.concurrency.VolatileLongOrDoubleFieldRule",
                Category::Concurrency,
                Severity::Info,
                "VolatileLongOrDoubleField concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(VolatileLongOrDoubleFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "WaitOutsideOfWhileLoop",
                "org.codenarc.rule.concurrency.WaitOutsideOfWhileLoopRule",
                Category::Concurrency,
                Severity::Info,
                "WaitOutsideOfWhileLoop concurrency rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(WaitOutsideOfWhileLoopRule)),
        ),
    ]
}

/// Descriptors for the JDBC, serialization and remaining rules.
#[must_use]
pub fn misc_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "NonSerializableFieldInSerializableClass",
                "org.codenarc.rule.serialization.NonSerializableFieldInSerializableClass",
                Category::Serialization,
                Severity::Info,
                "NonSerializableFieldInSerializableClass rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(NonSerializableFieldInSerializableClassRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SerializableClassMustDefineSerialVersionUID",
                "org.codenarc.rule.serialization.SerializableClassMustDefineSerialVersionUIDRule",
                Category::Serialization,
                Severity::Info,
                "SerializableClassMustDefineSerialVersionUID rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SerializableClassMustDefineSerialVersionUIDRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnusedMethodParameter",
                "org.codenarc.rule.unused.UnusedMethodParameterRule",
                Category::Unused,
                Severity::Info,
                "UnusedMethodParameter rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnusedMethodParameterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CrapMetric",
                "org.codenarc.rule.size.CrapMetricRule",
                Category::Size,
                Severity::Info,
                "Methods with a high cyclomatic complexity.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CrapMetricRule)),
        ),
    ]
}
