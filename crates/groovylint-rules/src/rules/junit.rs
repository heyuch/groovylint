//! `CodeNarc` `JUnit` and Spock rules.
//!
//! Almost all of these only apply to test sources, which `CodeNarc` decides by
//! class name and we decide by file name.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassKind, Expr, ExprKind, Item, Literal, MethodDecl, Script, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{
    ClassStack, expr_text, has_annotation, is_constant, is_test_file, unparen,
};

const TEST_ANNOTATIONS: &[&str] = &[
    "After",
    "AfterAll",
    "AfterClass",
    "AfterEach",
    "Before",
    "BeforeAll",
    "BeforeClass",
    "BeforeEach",
    "Disabled",
    "Ignore",
    "Override",
    "Test",
    "ParameterizedTest",
];

/// Returns whether the file is a test source. `CodeNarc` decides by class name;
/// the file name is used as a fallback for scripts.
fn is_test_source(context: &FileContext<'_>) -> bool {
    if is_test_file(context.source.path().to_string_lossy().as_ref()) {
        return true;
    }
    context.script.items.iter().any(|item| match item {
        Item::Class(class) => is_test_class_name(&class.name),
        _ => false,
    })
}

/// Returns the receiver name for assertion helpers: `this`, `Assert` or none.
fn is_assert_receiver(receiver: Option<&Expr>) -> bool {
    match receiver {
        None => true,
        Some(expr) => match &unparen(expr).kind {
            ExprKind::This => true,
            ExprKind::Variable(name) => name == "Assert" || name == "assert",
            _ => false,
        },
    }
}

fn call_parts(expr: &Expr) -> Option<(Option<&Expr>, &str, &[Expr])> {
    let ExprKind::Call {
        receiver,
        name,
        args,
        safe: false,
    } = &expr.kind
    else {
        return None;
    };
    Some((receiver.as_deref(), name, args))
}

fn is_assert_call<'a>(
    expr: &'a Expr,
    names: &[&str],
    arities: &[usize],
) -> Option<(&'a str, &'a [Expr])> {
    let (receiver, name, args) = call_parts(expr)?;
    if !is_assert_receiver(receiver) {
        return None;
    }
    if !names.contains(&name) || !arities.contains(&args.len()) {
        return None;
    }
    Some((name, args))
}

fn bool_literal(expr: &Expr) -> Option<bool> {
    match &unparen(expr).kind {
        ExprKind::Literal(Literal::Boolean(value)) => Some(*value),
        _ => None,
    }
}

fn is_null(expr: &Expr) -> bool {
    matches!(&unparen(expr).kind, ExprKind::Literal(Literal::Null))
}

fn method_statements(method: &MethodDecl) -> &[Stmt] {
    match &method.body {
        Some(block) => &block.statements,
        None => &[],
    }
}

fn calls_super(method: &MethodDecl, name: &str) -> bool {
    method_statements(method).iter().any(|statement| {
        let Stmt::Expr(expr) = statement else {
            return false;
        };
        let ExprKind::Call {
            receiver: Some(receiver),
            name: method_name,
            args,
            ..
        } = &expr.kind
        else {
            return false;
        };
        matches!(receiver.kind, ExprKind::Super) && method_name == name && args.is_empty()
    })
}

/// Flags explicit calls to other test methods from within a test.
pub struct ChainedTestRule;

impl Rule for ChainedTestRule {}

impl AstRule for ChainedTestRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((receiver, name, args)) = call_parts(expr) else {
            return;
        };
        if args.is_empty() && name.starts_with("test") && is_assert_receiver(receiver) {
            sink.report(
                expr.span,
                format!(
                    "The test method {name}() is being invoked explicitly from within a unit test. Tests should be isolated and not depend on one another"
                ),
            );
        }
    }
}

/// Flags test cases that create or call other test cases.
#[derive(Default)]
pub struct CoupledTestCaseRule {
    classes: ClassStack,
}

impl Rule for CoupledTestCaseRule {}

impl AstRule for CoupledTestCaseRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class).union(NodeMask::of(NodeKind::Expr))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Class(class) = node {
            self.classes.push(&class.name);
            return;
        }
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };

        if let ExprKind::New { type_name, .. } = &expr.kind {
            let simple = type_name.rsplit('.').next().unwrap_or(type_name);
            if is_test_class_name(simple) {
                sink.report(
                    expr.span,
                    format!(
                        "new {simple}() creates an instance of a test case. Test cases should not be coupled. Move this method to a helper object"
                    ),
                );
            }
            return;
        }

        let Some((Some(receiver), name, _)) = call_parts(expr) else {
            return;
        };
        let ExprKind::Variable(receiver_name) = &unparen(receiver).kind else {
            return;
        };
        if !is_test_class_name(receiver_name) || receiver_name == self.classes.current() {
            return;
        }
        sink.report(
            expr.span,
            format!(
                "{receiver_name}.{name}() invokes a method on another test case. Test cases should not be coupled. Move this method to a helper object"
            ),
        );
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

fn is_test_class_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase() && name.contains("Test")
}

/// Flags assertions that can never pass.
pub struct JUnitAssertAlwaysFailsRule;

impl Rule for JUnitAssertAlwaysFailsRule {}

impl AstRule for JUnitAssertAlwaysFailsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((name, args)) = is_assert_call(
            expr,
            &["assertTrue", "assertFalse", "assertNull", "assertNotNull"],
            &[1, 2],
        ) else {
            return;
        };
        let last = args.last().expect("arity checked");
        let fails = match name {
            "assertTrue" => bool_literal(last) == Some(false),
            "assertFalse" => bool_literal(last) == Some(true),
            "assertNull" => bool_literal(last).is_some() || is_non_null_constant(last),
            "assertNotNull" => is_null(last),
            _ => false,
        };
        if !fails {
            return;
        }
        let text = expr_text(expr, context.source.text());
        sink.report(
            expr.span,
            format!("The assertion {text} will always fail. Replace with a call to the fail(String) method"),
        );
    }
}

fn is_non_null_constant(expr: &Expr) -> bool {
    is_constant(expr) && !is_null(expr)
}

/// Flags assertions that always pass.
pub struct JUnitAssertAlwaysSucceedsRule;

impl Rule for JUnitAssertAlwaysSucceedsRule {}

impl AstRule for JUnitAssertAlwaysSucceedsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((name, args)) = is_assert_call(
            expr,
            &["assertTrue", "assertFalse", "assertNull", "assertNotNull"],
            &[1, 2],
        ) else {
            return;
        };
        let last = args.last().expect("arity checked");
        let succeeds = match name {
            "assertTrue" => bool_literal(last) == Some(true),
            "assertFalse" => bool_literal(last) == Some(false),
            "assertNull" => is_null(last),
            "assertNotNull" => bool_literal(last).is_some() || is_non_null_constant(last),
            _ => false,
        };
        if !succeeds {
            return;
        }
        let text = expr_text(expr, context.source.text());
        sink.report(
            expr.span,
            format!("The assertion {text} will always pass and is therefore pointless"),
        );
    }
}

/// Flags `assertEquals` calls whose actual value is a constant.
pub struct JUnitAssertEqualsConstantActualValueRule;

impl Rule for JUnitAssertEqualsConstantActualValueRule {}

impl AstRule for JUnitAssertEqualsConstantActualValueRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((_, args)) = is_assert_call(expr, &["assertEquals"], &[2, 3]) else {
            return;
        };
        let actual = args.last().expect("arity checked");
        if !is_constant(actual) {
            return;
        }
        let text = expr_text(expr, context.source.text());
        sink.report(
            expr.span,
            format!("The actual value in {text} is a constant and was probably intended to be the expected value"),
        );
    }
}

/// Flags `fail()` calls without a message.
pub struct JUnitFailWithoutMessageRule;

impl Rule for JUnitFailWithoutMessageRule {}

impl AstRule for JUnitFailWithoutMessageRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        if is_assert_call(expr, &["fail"], &[0]).is_none() {
            return;
        }
        sink.report(expr.span, "Pass a String parameter to the fail method");
    }
}

/// Flags `JUnit` 4 test methods that are missing the `@Test` annotation.
pub struct JUnitLostTestRule;

impl Rule for JUnitLostTestRule {}

impl AstRule for JUnitLostTestRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Script)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Script(script) = node else {
            return;
        };
        if !is_test_source(context) {
            return;
        }
        let imports_junit = script.imports.iter().any(|import| {
            let path = import.path.trim_end_matches(".*");
            path.starts_with("org.junit.")
        });
        if !imports_junit {
            return;
        }
        visit_methods(script, |method| {
            if !method.is_static
                && !method.is_abstract
                && method.return_type.as_deref() == Some("void")
                && method.params.is_empty()
                && method.name.starts_with("test")
                && !has_annotation(&method.annotations, "Test")
            {
                sink.report(
                    method.span,
                    format!(
                        "The method {} is a public, instance, void, no-arg method  named test*() that is not annotated with @Test.",
                        method.name
                    ),
                );
            }
        });
    }
}

fn visit_methods(script: &Script, mut visit: impl FnMut(&MethodDecl)) {
    fn walk(items: &[Item], visit: &mut impl FnMut(&MethodDecl)) {
        for item in items {
            match item {
                Item::Class(class) => walk(&class.members, visit),
                Item::Method(method) => visit(method),
                _ => {}
            }
        }
    }
    walk(&script.items, &mut visit);
}

/// Flags public fields on test classes.
pub struct JUnitPublicFieldRule;

impl Rule for JUnitPublicFieldRule {}

impl AstRule for JUnitPublicFieldRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Field(field) = node else {
            return;
        };
        if !field.is_public || field.is_static {
            return;
        }
        sink.report(
            field.span,
            format!(
                "The test class contains a public field {}. There is usually no reason to expose a public field on a test class.",
                field.name
            ),
        );
    }
}

/// Flags public methods on test classes that are not test methods.
pub struct JUnitPublicNonTestMethodRule;

impl Rule for JUnitPublicNonTestMethodRule {}

impl AstRule for JUnitPublicNonTestMethodRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.is_private || method.is_protected || method.is_static {
            return;
        }
        if is_test_method(method) || matches!(method.name.as_str(), "setUp" | "tearDown") {
            return;
        }
        if method
            .annotations
            .iter()
            .any(|annotation| TEST_ANNOTATIONS.contains(&annotation.as_str()))
        {
            return;
        }
        sink.report(
            method.span,
            format!("The method {} is public but not a test method", method.name),
        );
    }
}

fn is_test_method(method: &MethodDecl) -> bool {
    has_annotation(&method.annotations, "Test")
        || has_annotation(&method.annotations, "ParameterizedTest")
        || method.name.starts_with("test")
        || method.name.starts_with('`')
}

/// Flags public properties on test classes.
pub struct JUnitPublicPropertyRule;

impl Rule for JUnitPublicPropertyRule {}

impl AstRule for JUnitPublicPropertyRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Field(field) = node else {
            return;
        };
        if field.is_private || field.is_public || field.is_protected {
            return;
        }
        sink.report(
            field.span,
            format!(
                "The test class contains a public property {}. There is usually no reason to have a public property (even a constant) on a test class.",
                field.name
            ),
        );
    }
}

/// Flags `setUp()` methods that do not call `super.setUp()`.
pub struct JUnitSetUpCallsSuperRule;

impl Rule for JUnitSetUpCallsSuperRule {}

impl AstRule for JUnitSetUpCallsSuperRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.name != "setUp" || !method.params.is_empty() {
            return;
        }
        if calls_super(method, "setUp") {
            return;
        }
        sink.report(method.span, "The setUp() method should call super.setUp()");
    }
}

/// Flags `tearDown()` methods that do not call `super.tearDown()`.
pub struct JUnitTearDownCallsSuperRule;

impl Rule for JUnitTearDownCallsSuperRule {}

impl AstRule for JUnitTearDownCallsSuperRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.name != "tearDown" || !method.params.is_empty() {
            return;
        }
        if calls_super(method, "tearDown") {
            return;
        }
        sink.report(
            method.span,
            "The tearDown() method should call super.tearDown()",
        );
    }
}

/// Flags JUnit-style assertion methods in favour of Groovy power asserts.
pub struct JUnitStyleAssertionsRule;

impl Rule for JUnitStyleAssertionsRule {}

impl AstRule for JUnitStyleAssertionsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let matched = is_assert_call(
            expr,
            &["assertTrue", "assertFalse", "assertNull", "assertNotNull"],
            &[1, 2],
        )
        .or_else(|| is_assert_call(expr, &["assertEquals"], &[2, 3]));
        let Some((name, _)) = matched else {
            return;
        };
        sink.report(
            expr.span,
            format!("Replace {name} with an assert statement"),
        );
    }
}

/// Flags test methods without assertions.
pub struct JUnitTestMethodWithoutAssertRule;

impl Rule for JUnitTestMethodWithoutAssertRule {}

impl AstRule for JUnitTestMethodWithoutAssertRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if !is_test_method(method) {
            return;
        }
        if has_test_assertion(method) {
            return;
        }
        sink.report(
            method.span,
            format!("Test method '{}' makes no assertions", method.name),
        );
    }
}

fn has_test_assertion(method: &MethodDecl) -> bool {
    let Some(body) = &method.body else {
        return false;
    };
    let mut finder = AssertionFinder { found: false };
    for statement in &body.statements {
        crate::traverse::walk_stmt(statement, &mut finder);
        if finder.found {
            return true;
        }
    }
    false
}

struct AssertionFinder {
    found: bool,
}

impl crate::traverse::AstVisitor for AssertionFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Stmt(Stmt::Assert(_)) => self.found = true,
            NodeRef::Expr(expr) => {
                let Some((_, name, _)) = call_parts(expr) else {
                    return;
                };
                if name.starts_with("assert")
                    || name.starts_with("should")
                    || name.starts_with("fail")
                    || name.starts_with("verify")
                    || name.starts_with("expect")
                {
                    self.found = true;
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags `setUp()` methods that only call `super.setUp()`.
pub struct JUnitUnnecessarySetUpRule;

impl Rule for JUnitUnnecessarySetUpRule {}

impl AstRule for JUnitUnnecessarySetUpRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.name != "setUp" || !method.params.is_empty() {
            return;
        }
        let statements = method_statements(method);
        if statements.len() != 1 {
            return;
        }
        if !calls_super(method, "setUp") {
            return;
        }
        sink.report(
            method.span,
            "The setUp() method contains no logic and can be removed",
        );
    }
}

/// Flags `tearDown()` methods that only call `super.tearDown()`.
pub struct JUnitUnnecessaryTearDownRule;

impl Rule for JUnitUnnecessaryTearDownRule {}

impl AstRule for JUnitUnnecessaryTearDownRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.name != "tearDown" || !method.params.is_empty() {
            return;
        }
        if method_statements(method).len() != 1 {
            return;
        }
        if !calls_super(method, "tearDown") {
            return;
        }
        sink.report(
            method.span,
            "The tearDown() method contains no logic and can be removed",
        );
    }
}

/// Flags `throws` clauses on `JUnit` test methods.
pub struct JUnitUnnecessaryThrowsExceptionRule;

impl Rule for JUnitUnnecessaryThrowsExceptionRule {}

impl AstRule for JUnitUnnecessaryThrowsExceptionRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.throws.is_empty() || method.is_static || !method.params.is_empty() {
            return;
        }
        let junit4 = [
            "Test",
            "Before",
            "BeforeClass",
            "AfterClass",
            "After",
            "Ignore",
        ]
        .iter()
        .any(|annotation| has_annotation(&method.annotations, annotation));
        let junit3 =
            method.name.starts_with("test") || matches!(method.name.as_str(), "setUp" | "tearDown");
        if !junit4 && !junit3 {
            return;
        }
        sink.report(
            method.span,
            format!(
                "The {} method declares thrown exceptions, which is not necessary",
                method.name
            ),
        );
    }
}

/// Flags `@IgnoreRest` on Spock feature methods.
pub struct SpockIgnoreRestUsedRule;

impl Rule for SpockIgnoreRestUsedRule {}

impl AstRule for SpockIgnoreRestUsedRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !context.source.text().contains("spock.lang") && !is_test_source(context) {
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if !has_annotation(&method.annotations, "IgnoreRest") {
            return;
        }
        sink.report(
            method.span,
            format!("The method '{}' uses @IgnoreRest", method.name),
        );
    }
}

/// Flags boolean expressions nested inside `then:`/`expect:` blocks.
pub struct SpockMissingAssertRule;

impl Rule for SpockMissingAssertRule {}

impl AstRule for SpockMissingAssertRule {
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
        if !context.source.text().contains("spock.lang") {
            return;
        }
        let Some(body) = &method.body else {
            return;
        };

        let mut label: Option<String> = None;
        for statement in &body.statements {
            if let Some(name) = block_label(statement, context.source.text()) {
                label = Some(name);
                continue;
            }
            let Some(name) = &label else {
                continue;
            };
            if !matches!(name.as_str(), "then" | "expect") {
                continue;
            }
            for nested in nested_comparisons(statement) {
                sink.report(
                    nested,
                    format!(
                        "'{name}:' might contain a boolean expression in a nested statement, which is not implicitly asserted"
                    ),
                );
            }
        }
    }
}

/// Returns the label of a Spock block statement such as `then:`.
fn block_label(statement: &Stmt, source: &str) -> Option<String> {
    let text = source.get(statement.span().range())?.trim();
    let name = text.strip_suffix(':')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name.to_string())
}

/// Finds comparison expressions nested inside control statements.
fn nested_comparisons(statement: &Stmt) -> Vec<groovylint_core::Span> {
    let mut finder = ComparisonFinder { spans: Vec::new() };
    crate::traverse::walk_stmt(statement, &mut finder);
    finder.spans
}

struct ComparisonFinder {
    spans: Vec<groovylint_core::Span>,
}

impl crate::traverse::AstVisitor for ComparisonFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Binary { op, .. } = &expr.kind else {
            return;
        };
        if crate::rules::util::is_comparison(*op)
            && matches!(op, groovylint_syntax::ast::BinaryOp::Eq)
        {
            self.spans.push(expr.span);
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags `catch` blocks that only call `fail()`.
pub struct UnnecessaryFailRule;

impl Rule for UnnecessaryFailRule {}

impl AstRule for UnnecessaryFailRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Stmt(Stmt::Try(try_stmt)) = node else {
            return;
        };
        for catch in &try_stmt.catches {
            for statement in &catch.body.statements {
                let Stmt::Expr(expr) = statement else {
                    continue;
                };
                if is_assert_call(expr, &["fail"], &[0]).is_none() {
                    continue;
                }
                sink.report(
                    expr.span,
                    "Catching an exception and failing will hide the stack trace. It is better to rethrow the exception",
                );
            }
        }
    }
}

/// Flags `assertEquals` calls that compare against a `true`/`false` literal.
pub struct UseAssertTrueInsteadOfAssertEqualsRule;

impl Rule for UseAssertTrueInsteadOfAssertEqualsRule {}

impl AstRule for UseAssertTrueInsteadOfAssertEqualsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((_, args)) = is_assert_call(expr, &["assertEquals"], &[2, 3]) else {
            return;
        };
        let has_boolean = args.iter().any(|arg| bool_literal(arg).is_some());
        if !has_boolean {
            return;
        }
        sink.report(
            expr.span,
            "assertEquals can be simplified using assertTrue or assertFalse",
        );
    }
}

/// Flags `assertTrue`/`assertFalse` with a negation.
pub struct UseAssertFalseInsteadOfNegationRule;

impl Rule for UseAssertFalseInsteadOfNegationRule {}

impl AstRule for UseAssertFalseInsteadOfNegationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((_, args)) = is_assert_call(expr, &["assertTrue"], &[1, 2]) else {
            return;
        };
        let last = args.last().expect("arity checked");
        let ExprKind::Unary {
            op: groovylint_syntax::ast::UnaryOp::Not,
            operand,
        } = &unparen(last).kind
        else {
            return;
        };
        let right = expr_text(operand, context.source.text());
        sink.report(
            expr.span,
            format!("assertTrue(!{right}) can be simplified to assertFalse({right})"),
        );
    }
}

/// Flags `assertFalse` with a negation.
pub struct UseAssertTrueInsteadOfNegationRule;

impl Rule for UseAssertTrueInsteadOfNegationRule {}

impl AstRule for UseAssertTrueInsteadOfNegationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((_, args)) = is_assert_call(expr, &["assertFalse"], &[1, 2]) else {
            return;
        };
        let last = args.last().expect("arity checked");
        let ExprKind::Unary {
            op: groovylint_syntax::ast::UnaryOp::Not,
            operand,
        } = &unparen(last).kind
        else {
            return;
        };
        let right = expr_text(operand, context.source.text());
        sink.report(
            expr.span,
            format!("assertFalse(!{right}) can be simplified to assertTrue({right})"),
        );
    }
}

/// Flags `assertEquals` with a `null` argument.
pub struct UseAssertNullInsteadOfAssertEqualsRule;

impl Rule for UseAssertNullInsteadOfAssertEqualsRule {}

impl AstRule for UseAssertNullInsteadOfAssertEqualsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((_, args)) = is_assert_call(expr, &["assertEquals"], &[2, 3]) else {
            return;
        };
        if !args.iter().skip(args.len() - 2).any(is_null) {
            return;
        }
        sink.report(expr.span, "assertEquals can be simplified using assertNull");
    }
}

/// Flags `assertTrue(x.is(y))`.
pub struct UseAssertSameInsteadOfAssertTrueRule;

impl Rule for UseAssertSameInsteadOfAssertTrueRule {}

impl AstRule for UseAssertSameInsteadOfAssertTrueRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((_, args)) = is_assert_call(expr, &["assertTrue", "assertFalse"], &[1, 2]) else {
            return;
        };
        let last = args.last().expect("arity checked");
        let Some((_, name, _)) = call_parts(unparen(last)) else {
            return;
        };
        if name != "is" {
            return;
        }
        sink.report(
            expr.span,
            "assert method can be simplified using the assertSame method",
        );
    }
}

/// Flags `assertEquals(boolean, boolean)` that should use `assertTrue`.
pub struct UseAssertEqualsInsteadOfAssertTrueRule;

impl Rule for UseAssertEqualsInsteadOfAssertTrueRule {}

impl AstRule for UseAssertEqualsInsteadOfAssertTrueRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if !is_test_source(context) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((receiver, name, args)) = call_parts(expr) else {
            return;
        };
        if name != "assertTrue" || args.len() != 2 || !is_assert_receiver(receiver) {
            return;
        }
        let text = expr_text(expr, context.source.text());
        sink.report(
            expr.span,
            format!("{text} can be simplified to use assertEquals twice or assertTrue"),
        );
    }
}

/// Descriptors for the rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "ChainedTest",
                "org.codenarc.rule.junit.ChainedTestRule",
                Category::Junit,
                Severity::Warning,
                "Test methods should not call other test methods.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ChainedTestRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CoupledTestCase",
                "org.codenarc.rule.junit.CoupledTestCaseRule",
                Category::Junit,
                Severity::Warning,
                "Test cases should not depend on other test cases.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CoupledTestCaseRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitAssertAlwaysFails",
                "org.codenarc.rule.junit.JUnitAssertAlwaysFailsRule",
                Category::Junit,
                Severity::Warning,
                "Assertions with constant arguments that always fail.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitAssertAlwaysFailsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitAssertAlwaysSucceeds",
                "org.codenarc.rule.junit.JUnitAssertAlwaysSucceedsRule",
                Category::Junit,
                Severity::Warning,
                "Assertions with constant arguments that always pass.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitAssertAlwaysSucceedsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitAssertEqualsConstantActualValue",
                "org.codenarc.rule.junit.JUnitAssertEqualsConstantActualValueRule",
                Category::Junit,
                Severity::Warning,
                "assertEquals with a constant actual value.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitAssertEqualsConstantActualValueRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitFailWithoutMessage",
                "org.codenarc.rule.junit.JUnitFailWithoutMessageRule",
                Category::Junit,
                Severity::Warning,
                "fail() should be passed a message.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitFailWithoutMessageRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitLostTest",
                "org.codenarc.rule.junit.JUnitLostTestRule",
                Category::Junit,
                Severity::Warning,
                "JUnit 4 test methods that are missing @Test.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitLostTestRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitPublicField",
                "org.codenarc.rule.junit.JUnitPublicFieldRule",
                Category::Junit,
                Severity::Warning,
                "Test classes should not declare public fields.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitPublicFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitPublicNonTestMethod",
                "org.codenarc.rule.junit.JUnitPublicNonTestMethodRule",
                Category::Junit,
                Severity::Warning,
                "Test classes should not declare public non-test methods.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitPublicNonTestMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitPublicProperty",
                "org.codenarc.rule.junit.JUnitPublicPropertyRule",
                Category::Junit,
                Severity::Warning,
                "Test classes should not declare public properties.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitPublicPropertyRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitSetUpCallsSuper",
                "org.codenarc.rule.junit.JUnitSetUpCallsSuperRule",
                Category::Junit,
                Severity::Warning,
                "setUp() must call super.setUp().",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitSetUpCallsSuperRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitStyleAssertions",
                "org.codenarc.rule.junit.JUnitStyleAssertionsRule",
                Category::Junit,
                Severity::Info,
                "Prefer Groovy power asserts over JUnit assertions.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitStyleAssertionsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitTearDownCallsSuper",
                "org.codenarc.rule.junit.JUnitTearDownCallsSuperRule",
                Category::Junit,
                Severity::Warning,
                "tearDown() must call super.tearDown().",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitTearDownCallsSuperRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitTestMethodWithoutAssert",
                "org.codenarc.rule.junit.JUnitTestMethodWithoutAssertRule",
                Category::Junit,
                Severity::Warning,
                "Test methods should assert something.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitTestMethodWithoutAssertRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitUnnecessarySetUp",
                "org.codenarc.rule.junit.JUnitUnnecessarySetUpRule",
                Category::Junit,
                Severity::Info,
                "setUp() with only a super call is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitUnnecessarySetUpRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitUnnecessaryTearDown",
                "org.codenarc.rule.junit.JUnitUnnecessaryTearDownRule",
                Category::Junit,
                Severity::Info,
                "tearDown() with only a super call is unnecessary.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitUnnecessaryTearDownRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JUnitUnnecessaryThrowsException",
                "org.codenarc.rule.junit.JUnitUnnecessaryThrowsExceptionRule",
                Category::Junit,
                Severity::Info,
                "Test methods do not need throws clauses.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JUnitUnnecessaryThrowsExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpockIgnoreRestUsed",
                "org.codenarc.rule.junit.SpockIgnoreRestUsedRule",
                Category::Junit,
                Severity::Warning,
                "@IgnoreRest should not be committed.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpockIgnoreRestUsedRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpockMissingAssert",
                "org.codenarc.rule.junit.SpockMissingAssertRule",
                Category::Junit,
                Severity::Info,
                "Comparisons nested in then:/expect: blocks are not asserted.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpockMissingAssertRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryFail",
                "org.codenarc.rule.junit.UnnecessaryFailRule",
                Category::Junit,
                Severity::Warning,
                "Catching an exception and calling fail() hides the stack trace.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnnecessaryFailRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseAssertEqualsInsteadOfAssertTrue",
                "org.codenarc.rule.junit.UseAssertEqualsInsteadOfAssertTrueRule",
                Category::Junit,
                Severity::Info,
                "Use assertEquals for two-argument equality assertions.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseAssertEqualsInsteadOfAssertTrueRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseAssertFalseInsteadOfNegation",
                "org.codenarc.rule.junit.UseAssertFalseInsteadOfNegationRule",
                Category::Junit,
                Severity::Warning,
                "assertTrue(!x) should be assertFalse(x).",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseAssertFalseInsteadOfNegationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseAssertNullInsteadOfAssertEquals",
                "org.codenarc.rule.junit.UseAssertNullInsteadOfAssertEqualsRule",
                Category::Junit,
                Severity::Info,
                "assertEquals against null should be assertNull.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseAssertNullInsteadOfAssertEqualsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseAssertSameInsteadOfAssertTrue",
                "org.codenarc.rule.junit.UseAssertSameInsteadOfAssertTrueRule",
                Category::Junit,
                Severity::Info,
                "assertTrue(x.is(y)) should be assertSame.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseAssertSameInsteadOfAssertTrueRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseAssertTrueInsteadOfAssertEquals",
                "org.codenarc.rule.junit.UseAssertTrueInsteadOfAssertEqualsRule",
                Category::Junit,
                Severity::Info,
                "assertEquals against a boolean should be assertTrue or assertFalse.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseAssertTrueInsteadOfAssertEqualsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UseAssertTrueInsteadOfNegation",
                "org.codenarc.rule.junit.UseAssertTrueInsteadOfNegationRule",
                Category::Junit,
                Severity::Warning,
                "assertFalse(!x) should be assertTrue(x).",
            ),
            || RuleInstanceImpl::Ast(Box::new(UseAssertTrueInsteadOfNegationRule)),
        ),
    ]
}

#[allow(dead_code)]
fn unused(_: ClassKind) {}
