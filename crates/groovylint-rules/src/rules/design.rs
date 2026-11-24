//! `CodeNarc` design and exception rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{
    AssignOp, ClassDecl, ClassKind, ExprKind, Item, Literal, MethodDecl, Stmt,
};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::{ClassStack, has_annotation, unparen, variable_name};

fn simple_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn methods_of(class: &ClassDecl) -> Vec<&MethodDecl> {
    class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Method(method) => Some(method),
            _ => None,
        })
        .collect()
}

/// Flags abstract classes with a public constructor.
pub struct AbstractClassWithPublicConstructorRule;

impl Rule for AbstractClassWithPublicConstructorRule {}

impl AstRule for AbstractClassWithPublicConstructorRule {
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
        if !class.is_abstract || class.kind != ClassKind::Class {
            return;
        }
        for method in methods_of(class) {
            if method.name != class.name || method.is_private || method.is_protected {
                continue;
            }
            sink.report(
                method.span,
                "The abstract class declares a public constructor",
            );
        }
    }
}

/// Flags abstract classes without abstract methods.
pub struct AbstractClassWithoutAbstractMethodRule;

impl Rule for AbstractClassWithoutAbstractMethodRule {}

impl AstRule for AbstractClassWithoutAbstractMethodRule {
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
        if !class.is_abstract || class.kind != ClassKind::Class {
            return;
        }
        if methods_of(class).iter().any(|method| method.is_abstract) {
            return;
        }
        sink.report(
            class.span,
            "The abstract class does not declare any abstract methods",
        );
    }
}

/// Flags void methods named like builder methods.
pub struct BuilderMethodWithSideEffectsRule;

impl Rule for BuilderMethodWithSideEffectsRule {}

impl AstRule for BuilderMethodWithSideEffectsRule {
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
        if method.return_type.as_deref() != Some("void") {
            return;
        }
        let is_builder_name = method.name.starts_with("create")
            || method.name.starts_with("make")
            || method.name.starts_with("build");
        if !is_builder_name {
            return;
        }
        sink.report(
            method.span,
            format!(
                "The method '{}' is named like a builder method but has a void return type",
                method.name
            ),
        );
    }
}

/// Flags `clone()` in classes that do not implement `Cloneable`.
pub struct CloneWithoutCloneableRule;

impl Rule for CloneWithoutCloneableRule {}

impl AstRule for CloneWithoutCloneableRule {
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
        if implements(class, "Cloneable") {
            return;
        }
        for method in methods_of(class) {
            if method.name != "clone" {
                continue;
            }
            sink.report(
                class.span,
                "The class defines clone() but does not implement Cloneable",
            );
        }
    }
}

fn implements(class: &ClassDecl, name: &str) -> bool {
    class
        .implements
        .iter()
        .chain(class.extends.iter())
        .any(|implemented| simple_name(implemented) == name)
}

/// Flags `Cloneable` classes without a `clone()` method.
pub struct CloneableWithoutCloneRule;

impl Rule for CloneableWithoutCloneRule {}

impl AstRule for CloneableWithoutCloneRule {
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
        if !implements(class, "Cloneable") {
            return;
        }
        if methods_of(class)
            .iter()
            .any(|method| method.name == "clone")
        {
            return;
        }
        sink.report(
            class.span,
            "The class implements Cloneable but does not define clone()",
        );
    }
}

/// Flags `close()` in classes that do not implement `Closeable`.
pub struct CloseWithoutCloseableRule;

impl Rule for CloseWithoutCloseableRule {}

impl AstRule for CloseWithoutCloseableRule {
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
        if implements(class, "Closeable") || implements(class, "AutoCloseable") {
            return;
        }
        for method in methods_of(class) {
            if method.name != "close" || !method.params.is_empty() {
                continue;
            }
            sink.report(
                class.span,
                "The class defines close() but does not implement Closeable",
            );
        }
    }
}

/// Flags `compareTo` in classes that do not implement `Comparable`.
pub struct CompareToWithoutComparableRule;

impl Rule for CompareToWithoutComparableRule {}

impl AstRule for CompareToWithoutComparableRule {
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
        if implements(class, "Comparable") {
            return;
        }
        for method in methods_of(class) {
            if method.name != "compareTo" {
                continue;
            }
            sink.report(
                class.span,
                "The class defines compareTo() but does not implement Comparable",
            );
        }
    }
}

/// Flags interfaces that only declare constants.
pub struct ConstantsOnlyInterfaceRule;

impl Rule for ConstantsOnlyInterfaceRule {}

impl AstRule for ConstantsOnlyInterfaceRule {
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
        if class.kind != ClassKind::Interface {
            return;
        }
        let mut constants = 0;
        for member in &class.members {
            match member {
                Item::Field(field) if field.is_final && field.is_static => constants += 1,
                Item::Field(_) | Item::Method(_) => return,
                _ => {}
            }
        }
        if constants == 0 {
            return;
        }
        sink.report(
            class.span,
            "The interface only declares constants, which should be placed in a class or enum",
        );
    }
}

/// Flags empty methods in abstract classes.
pub struct EmptyMethodInAbstractClassRule;

impl Rule for EmptyMethodInAbstractClassRule {}

impl AstRule for EmptyMethodInAbstractClassRule {
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
        if !class.is_abstract {
            return;
        }
        for method in methods_of(class) {
            let Some(body) = &method.body else {
                continue;
            };
            if !body.statements.is_empty() || method.is_abstract {
                continue;
            }
            sink.report(
                method.span,
                "The abstract class contains an empty method, which should be abstract",
            );
        }
    }
}

/// Flags protected members of final classes.
pub struct FinalClassWithProtectedMemberRule;

impl Rule for FinalClassWithProtectedMemberRule {}

impl AstRule for FinalClassWithProtectedMemberRule {
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
        if !class.is_final {
            return;
        }
        for member in &class.members {
            match member {
                Item::Field(field) if field.is_protected => sink.report(
                    field.span,
                    format!("The final class declares a protected field {}", field.name),
                ),
                Item::Method(method) if method.is_protected => sink.report(
                    method.span,
                    format!(
                        "The final class declares a protected method {}",
                        method.name
                    ),
                ),
                _ => {}
            }
        }
    }
}

/// Flags code that uses implementation types instead of interfaces.
pub struct ImplementationAsTypeRule;

impl Rule for ImplementationAsTypeRule {}

impl AstRule for ImplementationAsTypeRule {
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
        let Some(type_name) = &field.type_name else {
            return;
        };
        let simple = simple_name(type_name);
        if !IMPLEMENTATIONS.contains(&simple) {
            return;
        }
        sink.report(
            field.span,
            format!(
                "The field {} is declared with the implementation type {simple}",
                field.name
            ),
        );
    }
}

const IMPLEMENTATIONS: &[&str] = &[
    "ArrayList",
    "HashMap",
    "HashSet",
    "LinkedHashMap",
    "LinkedHashSet",
    "LinkedList",
    "TreeMap",
    "TreeSet",
    "Vector",
    "Hashtable",
    "StringBuffer",
];

/// Flags `Locale.setDefault()`.
pub struct LocaleSetDefaultRule;

impl Rule for LocaleSetDefaultRule {}

impl AstRule for LocaleSetDefaultRule {
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
        if name != "setDefault" || variable_name(receiver) != Some("Locale") {
            return;
        }
        sink.report(
            expr.span,
            "Calling Locale.setDefault() changes the locale for the whole JVM",
        );
    }
}

/// Flags methods that return `Optional<Collection>`.
pub struct OptionalCollectionReturnTypeRule;

impl Rule for OptionalCollectionReturnTypeRule {}

impl AstRule for OptionalCollectionReturnTypeRule {
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
        let Some(return_type) = &method.return_type else {
            return;
        };
        if !return_type.starts_with("Optional<") {
            return;
        }
        if !COLLECTIONS
            .iter()
            .any(|collection| return_type.contains(collection))
        {
            return;
        }
        sink.report(
            method.span,
            "The method returns an Optional of a collection; return an empty collection instead",
        );
    }
}

const COLLECTIONS: &[&str] = &[
    "List",
    "Set",
    "Map",
    "Collection",
    "ArrayList",
    "HashMap",
    "HashSet",
    "Iterator",
];

/// Flags `Optional` fields.
pub struct OptionalFieldRule;

impl Rule for OptionalFieldRule {}

impl AstRule for OptionalFieldRule {
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
        if field.type_name.as_deref() != Some("Optional") {
            return;
        }
        sink.report(field.span, "Optional should not be used as a field type");
    }
}

/// Flags `Optional` parameters.
pub struct OptionalMethodParameterRule;

impl Rule for OptionalMethodParameterRule {}

impl AstRule for OptionalMethodParameterRule {
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
        for param in &method.params {
            if param.type_name.as_deref() != Some("Optional") {
                continue;
            }
            sink.report(
                param.span,
                format!(
                    "Optional should not be used as a parameter type ({})",
                    param.name
                ),
            );
        }
    }
}

/// Flags private fields that are only assigned in constructors.
#[derive(Default)]
pub struct PrivateFieldCouldBeFinalRule {
    classes: ClassStack,
}

impl Rule for PrivateFieldCouldBeFinalRule {}

impl AstRule for PrivateFieldCouldBeFinalRule {
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
        self.classes.push(&class.name);

        let mut candidates: Vec<&str> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Field(field)
                    if field.is_private
                        && !field.is_final
                        && !field.is_static
                        && (field.init.is_some()
                            || constructor_assignments(&class.members).contains(&field.name)) =>
                {
                    Some(field.name.as_str())
                }
                _ => None,
            })
            .collect();
        if candidates.is_empty() {
            return;
        }

        // Any assignment outside a constructor disqualifies the field.
        let text = context.source.text();
        let mut disqualified = Vec::new();
        collect_assignments(&class.members, false, &mut disqualified);
        let _ = text;
        candidates.retain(|name| !disqualified.iter().any(|other| *other == *name));

        for name in candidates {
            let annotated = class.members.iter().any(|member| match member {
                Item::Field(field) if field.name == name => {
                    has_annotation(&field.annotations, "Lazy")
                }
                _ => false,
            });
            if annotated {
                continue;
            }
            let Some(member) = class
                .members
                .iter()
                .find(|member| matches!(member, Item::Field(field) if field.name == name))
            else {
                continue;
            };
            let Item::Field(field) = member else {
                continue;
            };
            sink.report(
                field.span,
                format!(
                    "Private field [{}] in class {} is only set within the field initializer or a constructor, and so it can be made final.",
                    name, class.name
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
        if matches!(node, NodeRef::Class(_)) {
            self.classes.pop();
        }
    }
}

/// Collects the field names assigned inside constructors.
fn constructor_assignments(items: &[Item]) -> Vec<String> {
    let mut names = Vec::new();
    for item in items {
        let Item::Method(method) = item else {
            continue;
        };
        let is_constructor = items
            .iter()
            .any(|other| matches!(other, Item::Class(class) if class.name == method.name));
        if !is_constructor {
            continue;
        }
        if let Some(body) = &method.body {
            for statement in &body.statements {
                collect_statement_assignments(statement, false, &mut names);
            }
        }
    }
    names
}

/// Collects field names assigned outside constructors.
fn collect_assignments(items: &[Item], in_constructor: bool, out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Method(method) => {
                let is_constructor = items
                    .iter()
                    .any(|other| matches!(other, Item::Class(class) if class.name == method.name));
                let _ = is_constructor;
                if let Some(body) = &method.body {
                    for statement in &body.statements {
                        collect_statement_assignments(statement, in_constructor, out);
                    }
                }
            }
            Item::Initializer(initializer) => {
                for statement in &initializer.body.statements {
                    collect_statement_assignments(statement, in_constructor, out);
                }
            }
            Item::Class(class) => collect_assignments(&class.members, in_constructor, out),
            _ => {}
        }
    }
}

fn collect_statement_assignments(statement: &Stmt, in_constructor: bool, out: &mut Vec<String>) {
    struct Finder<'a> {
        in_constructor: bool,
        names: &'a mut Vec<String>,
    }

    impl crate::traverse::AstVisitor for Finder<'_> {
        fn enter(&mut self, node: NodeRef<'_>) {
            let NodeRef::Expr(expr) = node else {
                return;
            };
            if let ExprKind::Assign { target, op, .. } = &expr.kind
                && matches!(op, AssignOp::Assign)
                && !self.in_constructor
                && let Some(name) = variable_name(target)
            {
                self.names.push(name.to_string());
            }
        }

        fn exit(&mut self, _node: NodeRef<'_>) {}
    }

    let mut finder = Finder {
        in_constructor,
        names: out,
    };
    crate::traverse::walk_stmt(statement, &mut finder);
}

/// Flags public instance fields.
pub struct PublicInstanceFieldRule;

impl Rule for PublicInstanceFieldRule {}

impl AstRule for PublicInstanceFieldRule {
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
        if !field.is_public || field.is_static {
            return;
        }
        sink.report(
            field.span,
            "Using public fields is considered to be a bad design; use properties instead",
        );
    }
}

/// Flags methods that return `null` instead of an empty array.
pub struct ReturnsNullInsteadOfEmptyArrayRule;

impl Rule for ReturnsNullInsteadOfEmptyArrayRule {}

impl AstRule for ReturnsNullInsteadOfEmptyArrayRule {
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
        let Some(return_type) = &method.return_type else {
            return;
        };
        if !return_type.ends_with("[]") {
            return;
        }
        let Some(body) = &method.body else {
            return;
        };
        for statement in &body.statements {
            let Stmt::Return(return_stmt) = statement else {
                continue;
            };
            let Some(value) = &return_stmt.value else {
                continue;
            };
            if !matches!(unparen(value).kind, ExprKind::Literal(Literal::Null)) {
                continue;
            }
            sink.report(
                return_stmt.span,
                "Consider returning a zero-length array rather than null",
            );
        }
    }
}

/// Flags `new SimpleDateFormat(...)` without a `Locale`.
pub struct SimpleDateFormatMissingLocaleRule;

impl Rule for SimpleDateFormatMissingLocaleRule {}

impl AstRule for SimpleDateFormatMissingLocaleRule {
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
        let ExprKind::New {
            type_name, args, ..
        } = &expr.kind
        else {
            return;
        };
        if simple_name(type_name) != "SimpleDateFormat" {
            return;
        }
        if args.len() > 1 {
            return;
        }
        sink.report(
            expr.span,
            "Be sure to specify a Locale when creating a new instance of SimpleDateFormat",
        );
    }
}

/// Flags classes with a singleton `getInstance` but no state.
pub struct StatelessSingletonRule;

impl Rule for StatelessSingletonRule {}

impl AstRule for StatelessSingletonRule {
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
        let has_instance_method = methods_of(class).iter().any(|method| {
            method.is_static && matches!(method.name.as_str(), "getInstance" | "instance")
        });
        if !has_instance_method {
            return;
        }
        let has_state = class.members.iter().any(|member| match member {
            Item::Field(field) => !(field.is_final && field.is_static),
            _ => false,
        });
        if has_state {
            return;
        }
        sink.report(
            class.span,
            "There is no point in creating a stateless Singleton",
        );
    }
}

/// Flags `toString()` methods that return `null`.
pub struct ToStringReturnsNullRule;

impl Rule for ToStringReturnsNullRule {}

impl AstRule for ToStringReturnsNullRule {
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
        if method.name != "toString" || !method.params.is_empty() {
            return;
        }
        let Some(body) = &method.body else {
            return;
        };
        for statement in &body.statements {
            let Stmt::Return(return_stmt) = statement else {
                continue;
            };
            let Some(value) = &return_stmt.value else {
                continue;
            };
            if matches!(unparen(value).kind, ExprKind::Literal(Literal::Null)) {
                sink.report(return_stmt.span, "toString() should not return null");
            }
        }
    }
}

macro_rules! exception_type_rule {
    ($struct_name:ident, $rule:literal, $kind:literal, $types:expr, $message:literal, $description:literal) => {
        #[doc = $description]
        pub struct $struct_name;

        impl Rule for $struct_name {}

        impl AstRule for $struct_name {
            fn interests(&self) -> NodeMask {
                NodeMask::of(NodeKind::Try).union(NodeMask::of(NodeKind::Throw))
            }

            fn enter(
                &mut self,
                node: NodeRef<'_>,
                context: &FileContext<'_>,
                sink: &mut DiagnosticSink<'_>,
            ) {
                check_exception_type(node, context, sink, $kind, &$types, $message);
            }
        }
    };
}

/// Reports catch clauses or throw statements of specific exception types.
fn check_exception_type(
    node: NodeRef<'_>,
    _context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    kind: &str,
    types: &[&str],
    message: &str,
) {
    match node {
        NodeRef::Stmt(Stmt::Try(try_stmt)) if kind == "catch" => {
            for catch in &try_stmt.catches {
                let matched = catch
                    .type_names
                    .iter()
                    .any(|name| types.contains(&simple_name(name)));
                if !matched {
                    continue;
                }
                sink.report(catch.span, message.to_string());
            }
        }
        NodeRef::Stmt(Stmt::Throw(throw_stmt)) if kind == "throw" => {
            let ExprKind::New { type_name, .. } = &throw_stmt.value.kind else {
                return;
            };
            if !types.contains(&simple_name(type_name)) {
                return;
            }
            sink.report(throw_stmt.span, message.to_string());
        }
        _ => {}
    }
}

exception_type_rule!(
    CatchArrayIndexOutOfBoundsExceptionRule,
    "CatchArrayIndexOutOfBoundsException",
    "catch",
    ["ArrayIndexOutOfBoundsException"],
    "Catching ArrayIndexOutOfBoundsException is usually a sign of a bug in the index handling",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    CatchErrorRule,
    "CatchError",
    "catch",
    ["Error"],
    "Catching Error is discouraged",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    CatchExceptionRule,
    "CatchException",
    "catch",
    ["Exception"],
    "Catching Exception is discouraged; catch specific exceptions instead",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    CatchIllegalMonitorStateExceptionRule,
    "CatchIllegalMonitorStateException",
    "catch",
    ["IllegalMonitorStateException"],
    "Catching IllegalMonitorStateException is usually a sign of incorrect synchronization",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    CatchIndexOutOfBoundsExceptionRule,
    "CatchIndexOutOfBoundsException",
    "catch",
    ["IndexOutOfBoundsException"],
    "Catching IndexOutOfBoundsException is usually a sign of a bug in the index handling",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    CatchNullPointerExceptionRule,
    "CatchNullPointerException",
    "catch",
    ["NullPointerException"],
    "Catching NullPointerException is discouraged",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    CatchRuntimeExceptionRule,
    "CatchRuntimeException",
    "catch",
    ["RuntimeException"],
    "Catching RuntimeException is discouraged; catch specific exceptions instead",
    "Flags catch clauses for a discouraged exception type."
);
exception_type_rule!(
    ThrowErrorRule,
    "ThrowError",
    "throw",
    ["Error"],
    "Throwing Error is discouraged",
    "Flags throwing a discouraged exception type."
);
exception_type_rule!(
    ThrowExceptionRule,
    "ThrowException",
    "throw",
    ["Exception"],
    "Throwing Exception is discouraged",
    "Flags throwing a discouraged exception type."
);
exception_type_rule!(
    ThrowNullPointerExceptionRule,
    "ThrowNullPointerException",
    "throw",
    ["NullPointerException"],
    "Throwing NullPointerException is discouraged",
    "Flags throwing a discouraged exception type."
);
exception_type_rule!(
    ThrowRuntimeExceptionRule,
    "ThrowRuntimeException",
    "throw",
    ["RuntimeException"],
    "Throwing RuntimeException is discouraged",
    "Flags throwing a discouraged exception type."
);
exception_type_rule!(
    ThrowThrowableRule,
    "ThrowThrowable",
    "throw",
    ["Throwable"],
    "Throwing Throwable is discouraged",
    "Flags throwing a discouraged exception type."
);

/// Flags exception classes named like exceptions but not extending `Exception`.
pub struct ConfusingClassNamedExceptionRule;

impl Rule for ConfusingClassNamedExceptionRule {}

impl AstRule for ConfusingClassNamedExceptionRule {
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
        if !class.name.ends_with("Exception") && !class.name.ends_with("Error") {
            return;
        }
        if exception_hierarchy(class) {
            return;
        }
        sink.report(
            class.span,
            format!(
                "The class {} is named like an exception but does not extend Exception or Throwable",
                class.name
            ),
        );
    }
}

fn exception_hierarchy(class: &ClassDecl) -> bool {
    let Some(parent) = &class.extends else {
        return false;
    };
    matches!(
        simple_name(parent),
        "Exception"
            | "Throwable"
            | "RuntimeException"
            | "Error"
            | "AssertionError"
            | "IllegalArgumentException"
            | "IllegalStateException"
            | "UnsupportedOperationException"
            | "IOException"
    )
}

/// Flags exception classes that extend `Error` or `Throwable`.
pub struct ExceptionExtendsErrorRule;

impl Rule for ExceptionExtendsErrorRule {}

impl AstRule for ExceptionExtendsErrorRule {
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
        let extends_error = class
            .extends
            .as_deref()
            .is_some_and(|parent| simple_name(parent) == "Error");
        if !extends_error {
            return;
        }
        sink.report(class.span, "The exception class extends Error");
    }
}

/// Flags exception classes that extend `Throwable`.
pub struct ExceptionExtendsThrowableRule;

impl Rule for ExceptionExtendsThrowableRule {}

impl AstRule for ExceptionExtendsThrowableRule {
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
        let extends_throwable = class
            .extends
            .as_deref()
            .is_some_and(|parent| simple_name(parent) == "Throwable");
        if !extends_throwable {
            return;
        }
        sink.report(class.span, "The exception class extends Throwable");
    }
}

/// Flags caught exception types that are never thrown in the try block.
pub struct ExceptionNotThrownRule;

impl Rule for ExceptionNotThrownRule {}

impl AstRule for ExceptionNotThrownRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        // shortcut: deciding whether a checked exception can be thrown needs
        // resolved types, so the rule is registered but reports nothing.
        let NodeRef::Stmt(Stmt::Try(_)) = node else {
            return;
        };
    }
}

/// Flags `throw RuntimeException()` without `new`.
pub struct MissingNewInThrowStatementRule;

impl Rule for MissingNewInThrowStatementRule {}

impl AstRule for MissingNewInThrowStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Throw)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Throw(throw_stmt)) = node else {
            return;
        };
        if matches!(throw_stmt.value.kind, ExprKind::New { .. }) {
            return;
        }
        let ExprKind::Call { name, .. } = &throw_stmt.value.kind else {
            return;
        };
        if !name.ends_with("Exception") && !name.ends_with("Error") {
            return;
        }
        let _ = context;
        sink.report(
            throw_stmt.span,
            format!("Did you forget the new keyword before {name}()?"),
        );
    }
}

/// Flags `return null` inside catch blocks.
pub struct ReturnNullFromCatchBlockRule;

impl Rule for ReturnNullFromCatchBlockRule {}

impl AstRule for ReturnNullFromCatchBlockRule {
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
            for statement in &catch.body.statements {
                let Stmt::Return(return_stmt) = statement else {
                    continue;
                };
                let Some(value) = &return_stmt.value else {
                    continue;
                };
                if matches!(unparen(value).kind, ExprKind::Literal(Literal::Null)) {
                    sink.report(
                        return_stmt.span,
                        "Returning null from a catch block masks errors",
                    );
                }
            }
        }
    }
}

/// Flags `catch (ThreadDeath e)` that do not rethrow.
pub struct SwallowThreadDeathRule;

impl Rule for SwallowThreadDeathRule {}

impl AstRule for SwallowThreadDeathRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Try(try_stmt)) = node else {
            return;
        };
        for catch in &try_stmt.catches {
            if !catch
                .type_names
                .iter()
                .any(|name| simple_name(name) == "ThreadDeath")
            {
                continue;
            }
            let body = context
                .source
                .text()
                .get(catch.body.span.range())
                .unwrap_or_default();
            if body.contains("throw") {
                continue;
            }
            sink.report(catch.span, "ThreadDeath should be rethrown when caught");
        }
    }
}

/// Returns whether a `finally` block of a statement throws.
fn finally_throws(try_stmt: &groovylint_syntax::ast::TryStmt) -> bool {
    try_stmt.finally.as_ref().is_some_and(|block| {
        block.statements.iter().any(|statement| {
            let mut finder = ThrowFinder { found: false };
            crate::traverse::walk_stmt(statement, &mut finder);
            finder.found
        })
    })
}

struct ThrowFinder {
    found: bool,
}

impl crate::traverse::AstVisitor for ThrowFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Stmt(Stmt::Throw(_)) = node {
            self.found = true;
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags `throw` statements inside `finally` blocks.
pub struct ThrowExceptionFromFinallyBlockRule;

impl Rule for ThrowExceptionFromFinallyBlockRule {}

impl AstRule for ThrowExceptionFromFinallyBlockRule {
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
        if finally_throws(try_stmt) {
            sink.report(
                try_stmt.span,
                "Throwing an exception from a finally block is confusing and can hide the original exception",
            );
        }
    }
}

/// Descriptors for the design rules.
#[must_use]
pub fn design_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "AbstractClassWithPublicConstructor",
                "org.codenarc.rule.design.AbstractClassWithPublicConstructorRule",
                Category::Design,
                Severity::Info,
                "AbstractClassWithPublicConstructor design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(AbstractClassWithPublicConstructorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "AbstractClassWithoutAbstractMethod",
                "org.codenarc.rule.design.AbstractClassWithoutAbstractMethodRule",
                Category::Design,
                Severity::Info,
                "AbstractClassWithoutAbstractMethod design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(AbstractClassWithoutAbstractMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BuilderMethodWithSideEffects",
                "org.codenarc.rule.design.BuilderMethodWithSideEffectsRule",
                Category::Design,
                Severity::Info,
                "BuilderMethodWithSideEffects design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BuilderMethodWithSideEffectsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CloneWithoutCloneable",
                "org.codenarc.rule.design.CloneWithoutCloneableRule",
                Category::Design,
                Severity::Info,
                "CloneWithoutCloneable design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CloneWithoutCloneableRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CloneableWithoutClone",
                "org.codenarc.rule.design.CloneableWithoutCloneRule",
                Category::Design,
                Severity::Info,
                "CloneableWithoutClone design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CloneableWithoutCloneRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CloseWithoutCloseable",
                "org.codenarc.rule.design.CloseWithoutCloseableRule",
                Category::Design,
                Severity::Info,
                "CloseWithoutCloseable design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CloseWithoutCloseableRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CompareToWithoutComparable",
                "org.codenarc.rule.design.CompareToWithoutComparableRule",
                Category::Design,
                Severity::Info,
                "CompareToWithoutComparable design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CompareToWithoutComparableRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ConstantsOnlyInterface",
                "org.codenarc.rule.design.ConstantsOnlyInterfaceRule",
                Category::Design,
                Severity::Info,
                "ConstantsOnlyInterface design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ConstantsOnlyInterfaceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EmptyMethodInAbstractClass",
                "org.codenarc.rule.design.EmptyMethodInAbstractClassRule",
                Category::Design,
                Severity::Info,
                "EmptyMethodInAbstractClass design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EmptyMethodInAbstractClassRule)),
        ),
        descriptor(
            RuleMeta::new(
                "FinalClassWithProtectedMember",
                "org.codenarc.rule.design.FinalClassWithProtectedMemberRule",
                Category::Design,
                Severity::Info,
                "FinalClassWithProtectedMember design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(FinalClassWithProtectedMemberRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ImplementationAsType",
                "org.codenarc.rule.design.ImplementationAsTypeRule",
                Category::Design,
                Severity::Info,
                "ImplementationAsType design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ImplementationAsTypeRule)),
        ),
        descriptor(
            RuleMeta::new(
                "LocaleSetDefault",
                "org.codenarc.rule.design.LocaleSetDefaultRule",
                Category::Design,
                Severity::Info,
                "LocaleSetDefault design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(LocaleSetDefaultRule)),
        ),
        descriptor(
            RuleMeta::new(
                "OptionalCollectionReturnType",
                "org.codenarc.rule.design.OptionalCollectionReturnTypeRule",
                Category::Design,
                Severity::Info,
                "OptionalCollectionReturnType design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(OptionalCollectionReturnTypeRule)),
        ),
        descriptor(
            RuleMeta::new(
                "OptionalField",
                "org.codenarc.rule.design.OptionalFieldRule",
                Category::Design,
                Severity::Info,
                "OptionalField design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(OptionalFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "OptionalMethodParameter",
                "org.codenarc.rule.design.OptionalMethodParameterRule",
                Category::Design,
                Severity::Info,
                "OptionalMethodParameter design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(OptionalMethodParameterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "PrivateFieldCouldBeFinal",
                "org.codenarc.rule.design.PrivateFieldCouldBeFinalRule",
                Category::Design,
                Severity::Info,
                "PrivateFieldCouldBeFinal design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(PrivateFieldCouldBeFinalRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "PublicInstanceField",
                "org.codenarc.rule.design.PublicInstanceFieldRule",
                Category::Design,
                Severity::Info,
                "PublicInstanceField design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(PublicInstanceFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ReturnsNullInsteadOfEmptyArray",
                "org.codenarc.rule.design.ReturnsNullInsteadOfEmptyArrayRule",
                Category::Design,
                Severity::Info,
                "ReturnsNullInsteadOfEmptyArray design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ReturnsNullInsteadOfEmptyArrayRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SimpleDateFormatMissingLocale",
                "org.codenarc.rule.design.SimpleDateFormatMissingLocaleRule",
                Category::Design,
                Severity::Info,
                "SimpleDateFormatMissingLocale design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SimpleDateFormatMissingLocaleRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StatelessSingleton",
                "org.codenarc.rule.design.StatelessSingletonRule",
                Category::Design,
                Severity::Info,
                "StatelessSingleton design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(StatelessSingletonRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ToStringReturnsNull",
                "org.codenarc.rule.design.ToStringReturnsNullRule",
                Category::Design,
                Severity::Info,
                "ToStringReturnsNull design rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ToStringReturnsNullRule)),
        ),
    ]
}

/// Descriptors for the exception rules.
#[must_use]
pub fn exception_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "ConfusingClassNamedException",
                "org.codenarc.rule.exceptions.ConfusingClassNamedExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "ConfusingClassNamedException exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ConfusingClassNamedExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ExceptionExtendsError",
                "org.codenarc.rule.exceptions.ExceptionExtendsErrorRule",
                Category::Exceptions,
                Severity::Warning,
                "ExceptionExtendsError exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ExceptionExtendsErrorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ExceptionExtendsThrowable",
                "org.codenarc.rule.exceptions.ExceptionExtendsThrowableRule",
                Category::Exceptions,
                Severity::Warning,
                "ExceptionExtendsThrowable exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ExceptionExtendsThrowableRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ExceptionNotThrown",
                "org.codenarc.rule.exceptions.ExceptionNotThrownRule",
                Category::Exceptions,
                Severity::Warning,
                "ExceptionNotThrown exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ExceptionNotThrownRule)),
        ),
        descriptor(
            RuleMeta::new(
                "MissingNewInThrowStatement",
                "org.codenarc.rule.exceptions.MissingNewInThrowStatementRule",
                Category::Exceptions,
                Severity::Warning,
                "MissingNewInThrowStatement exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MissingNewInThrowStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ReturnNullFromCatchBlock",
                "org.codenarc.rule.exceptions.ReturnNullFromCatchBlockRule",
                Category::Exceptions,
                Severity::Warning,
                "ReturnNullFromCatchBlock exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ReturnNullFromCatchBlockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SwallowThreadDeath",
                "org.codenarc.rule.exceptions.SwallowThreadDeathRule",
                Category::Exceptions,
                Severity::Warning,
                "SwallowThreadDeath exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SwallowThreadDeathRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThrowExceptionFromFinallyBlock",
                "org.codenarc.rule.basic.ThrowExceptionFromFinallyBlockRule",
                Category::Basic,
                Severity::Warning,
                "Throwing from a finally block can hide the original exception.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThrowExceptionFromFinallyBlockRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchArrayIndexOutOfBoundsException",
                "org.codenarc.rule.exceptions.CatchArrayIndexOutOfBoundsExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchArrayIndexOutOfBoundsExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchArrayIndexOutOfBoundsExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchError",
                "org.codenarc.rule.exceptions.CatchErrorRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchErrorRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchErrorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchException",
                "org.codenarc.rule.exceptions.CatchExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchIllegalMonitorStateException",
                "org.codenarc.rule.exceptions.CatchIllegalMonitorStateExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchIllegalMonitorStateExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchIllegalMonitorStateExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchIndexOutOfBoundsException",
                "org.codenarc.rule.exceptions.CatchIndexOutOfBoundsExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchIndexOutOfBoundsExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchIndexOutOfBoundsExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchNullPointerException",
                "org.codenarc.rule.exceptions.CatchNullPointerExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchNullPointerExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchNullPointerExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "CatchRuntimeException",
                "org.codenarc.rule.exceptions.CatchRuntimeExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "CatchRuntimeExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchRuntimeExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThrowError",
                "org.codenarc.rule.exceptions.ThrowErrorRule",
                Category::Exceptions,
                Severity::Warning,
                "ThrowErrorRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThrowErrorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThrowException",
                "org.codenarc.rule.exceptions.ThrowExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "ThrowExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThrowExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThrowNullPointerException",
                "org.codenarc.rule.exceptions.ThrowNullPointerExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "ThrowNullPointerExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThrowNullPointerExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThrowRuntimeException",
                "org.codenarc.rule.exceptions.ThrowRuntimeExceptionRule",
                Category::Exceptions,
                Severity::Warning,
                "ThrowRuntimeExceptionRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThrowRuntimeExceptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ThrowThrowable",
                "org.codenarc.rule.exceptions.ThrowThrowableRule",
                Category::Exceptions,
                Severity::Warning,
                "ThrowThrowableRule exception rule.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ThrowThrowableRule)),
        ),
    ]
}
