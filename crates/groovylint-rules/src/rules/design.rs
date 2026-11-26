//! `CodeNarc` design and exception rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{
    AssignOp, Block, ClassDecl, ClassKind, Expr, ExprKind, Item, Literal, MethodDecl, Stmt, UnaryOp,
};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};
use crate::rules::util::{ClassStack, expr_text, has_annotation, unparen, variable_name};

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
pub struct PrivateFieldCouldBeFinalRule {
    classes: ClassStack,
    ignored_fields: Vec<String>,
    ignored_classes: Vec<String>,
    ignore_jpa_entities: bool,
}

impl Default for PrivateFieldCouldBeFinalRule {
    fn default() -> Self {
        Self {
            classes: ClassStack::default(),
            ignored_fields: Vec::new(),
            ignored_classes: Vec::new(),
            ignore_jpa_entities: true,
        }
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Annotations that make a class a JPA entity, whose fields the rule leaves alone.
const JPA_ANNOTATIONS: &[&str] = &["Entity", "MappedSuperclass", "Embeddable"];

impl Rule for PrivateFieldCouldBeFinalRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("ignoreFieldNames") {
            self.ignored_fields = split_list(value);
        }
        if let Some(value) = properties.get("doNotApplyToClassNames") {
            self.ignored_classes = split_list(value);
        }
        if let Some(value) = properties.get("ignoreJpaEntities") {
            self.ignore_jpa_entities = value != "false";
        }
        Ok(())
    }
}

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

        if self.ignore_jpa_entities
            && class
                .annotations
                .iter()
                .any(|annotation| JPA_ANNOTATIONS.contains(&annotation.as_str()))
        {
            return;
        }
        if self
            .ignored_classes
            .iter()
            .any(|pattern| crate::rules::types::wildcard_matches(&class.name, pattern))
        {
            return;
        }

        let mut candidates: Vec<&str> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Field(field)
                    if field.is_private
                        && !field.is_final
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
        collect_assignments(&class.members, Some(&class.name), &mut disqualified);
        let _ = text;
        candidates.retain(|name| !disqualified.iter().any(|other| *other == *name));
        let ignored = self.ignored_fields.clone();
        candidates.retain(|name| {
            !ignored
                .iter()
                .any(|pattern| crate::rules::types::wildcard_matches(name, pattern))
        });

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
/// Records every field assignment that would stop the field from being final.
///
/// A write inside a constructor still allows `final`; a write in a method, a field
/// initializer or a closure does not, because it runs later.
fn collect_assignments(items: &[Item], class_name: Option<&str>, out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Method(method) => {
                let is_constructor = class_name == Some(method.name.as_str());
                if let Some(body) = &method.body {
                    for statement in &body.statements {
                        collect_statement_assignments(statement, is_constructor, out);
                    }
                }
            }
            Item::Field(field) => {
                if let Some(init) = &field.init {
                    collect_expression_assignments(init, false, out);
                }
            }
            Item::Initializer(initializer) => {
                for statement in &initializer.body.statements {
                    collect_statement_assignments(statement, false, out);
                }
            }
            Item::Class(class) => collect_assignments(&class.members, Some(&class.name), out),
            _ => {}
        }
    }
}

fn collect_expression_assignments(expr: &Expr, in_constructor: bool, out: &mut Vec<String>) {
    let mut finder = AssignmentFinder {
        in_constructor,
        closure_depth: 0,
        names: out,
    };
    crate::traverse::walk_expr(expr, &mut finder);
}

/// Collects assignment targets outside constructors.
struct AssignmentFinder<'a> {
    in_constructor: bool,
    closure_depth: usize,
    names: &'a mut Vec<String>,
}

impl AssignmentFinder<'_> {
    /// A write only initialises the field when it runs in the constructor itself:
    /// a closure runs later, so a write inside one disqualifies the field.
    fn mutates(&self) -> bool {
        !self.in_constructor || self.closure_depth > 0
    }
}

impl crate::traverse::AstVisitor for AssignmentFinder<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        if matches!(expr.kind, ExprKind::Closure { .. }) {
            self.closure_depth += 1;
            return;
        }
        if !self.mutates() {
            return;
        }
        match &expr.kind {
            ExprKind::Assign { target, .. } => {
                if let Some(name) = assignment_target(target) {
                    self.names.push(name);
                }
            }
            ExprKind::Unary {
                op: UnaryOp::PreInc | UnaryOp::PreDec | UnaryOp::PostInc | UnaryOp::PostDec,
                operand,
            } => {
                if let Some(name) = assignment_target(operand) {
                    self.names.push(name);
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Expr(Expr {
            kind: ExprKind::Closure { .. },
            ..
        }) = node
        {
            self.closure_depth = self.closure_depth.saturating_sub(1);
        }
    }
}

/// The field a write targets: `x` or `this.x`.
fn assignment_target(expr: &Expr) -> Option<String> {
    if let Some(name) = variable_name(expr) {
        return Some(name.to_string());
    }
    let ExprKind::Property { receiver, name, .. } = &expr.kind else {
        return None;
    };
    let ExprKind::Variable(base) = &receiver.kind else {
        return None;
    };
    (base == "this").then(|| name.clone())
}

fn collect_statement_assignments(statement: &Stmt, in_constructor: bool, out: &mut Vec<String>) {
    let mut finder = AssignmentFinder {
        in_constructor,
        closure_depth: 0,
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
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::Try(try_stmt)) = node else {
            return;
        };
        for catch in &try_stmt.catches {
            let Some(Stmt::Expr(expr)) = catch.body.statements.last() else {
                continue;
            };
            let ExprKind::New { type_name, .. } = &expr.kind else {
                continue;
            };
            if simple_name(type_name).ends_with("Exception") {
                sink.report(
                    expr.span,
                    format!("The catch block constructs a {type_name} but does not throw it"),
                );
            }
        }
    }
}

/// Flags catching `Throwable`.
pub struct CatchThrowableRule;

impl Rule for CatchThrowableRule {}

impl AstRule for CatchThrowableRule {
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
            if catch
                .type_names
                .iter()
                .any(|name| simple_name(name) == "Throwable")
            {
                sink.report(catch.span, "Catching Throwable is dangerous");
            }
        }
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
        _context: &FileContext<'_>,
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
        if name.ends_with("Exception") || name.ends_with("Error") {
            sink.report(
                throw_stmt.span,
                format!("Did you forget the new keyword before {name}()?"),
            );
        }
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
            if !body.contains("throw") {
                sink.report(catch.span, "ThreadDeath should be rethrown when caught");
            }
        }
    }
}

/// Flags assignments to static fields from instance methods.
pub struct AssignmentToStaticFieldFromInstanceMethodRule;

impl Rule for AssignmentToStaticFieldFromInstanceMethodRule {}

impl FileRule for AssignmentToStaticFieldFromInstanceMethodRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            if let Item::Class(class) = item {
                check_static_field_assignments(class, sink);
            }
        }
    }
}

fn check_static_field_assignments(class: &ClassDecl, sink: &mut DiagnosticSink<'_>) {
    let static_fields: Vec<&str> = class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Field(field) if field.is_static => Some(field.name.as_str()),
            _ => None,
        })
        .collect();
    if !static_fields.is_empty() {
        for member in &class.members {
            let Item::Method(method) = member else {
                continue;
            };
            if method.is_static {
                continue;
            }
            let Some(body) = &method.body else {
                continue;
            };
            let mut checker = StaticFieldAssignment {
                fields: &static_fields,
                locals: Vec::new(),
                violations: Vec::new(),
            };
            crate::traverse::walk_block(body, &mut checker);
            for span in checker.violations {
                sink.report(
                    span,
                    format!(
                        "The instance method {} contains an assignment to a static field",
                        method.name
                    ),
                );
            }
        }
    }
    for member in &class.members {
        if let Item::Class(nested) = member {
            check_static_field_assignments(nested, sink);
        }
    }
}

struct StaticFieldAssignment<'a> {
    fields: &'a [&'a str],
    locals: Vec<String>,
    violations: Vec<groovylint_core::Span>,
}

impl crate::traverse::AstVisitor for StaticFieldAssignment<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Stmt(Stmt::VarDecl(decl)) => self.locals.push(decl.name.clone()),
            NodeRef::Expr(expr) => {
                let ExprKind::Assign {
                    target,
                    op: AssignOp::Assign,
                    ..
                } = &expr.kind
                else {
                    return;
                };
                let Some(name) = variable_name(target) else {
                    return;
                };
                if self.fields.contains(&name) && !self.locals.iter().any(|local| local == name) {
                    self.violations.push(expr.span);
                }
            }
            _ => {}
        }
    }
    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags Boolean methods that return null.
pub struct BooleanMethodReturnsNullRule;

impl Rule for BooleanMethodReturnsNullRule {}

impl AstRule for BooleanMethodReturnsNullRule {
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
        let is_boolean = method
            .return_type
            .as_deref()
            .is_some_and(|name| matches!(simple_name(name), "Boolean" | "boolean"));
        if !is_boolean {
            return;
        }
        let Some(body) = &method.body else {
            return;
        };
        for span in null_returns(body) {
            sink.report(
                span,
                "Returning null from a method that might return a Boolean",
            );
        }
    }
}

/// Flags instanceof checks.
pub struct InstanceofRule {
    ignore_type_names: Vec<String>,
}

impl Default for InstanceofRule {
    fn default() -> Self {
        Self {
            ignore_type_names: vec!["*Exception".to_string()],
        }
    }
}

impl Rule for InstanceofRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("ignoreTypeNames") {
            self.ignore_type_names = value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        Ok(())
    }
}

impl AstRule for InstanceofRule {
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
            op: groovylint_syntax::ast::BinaryOp::InstanceOf,
            right,
            ..
        } = &expr.kind
        else {
            return;
        };
        let type_name = expr_text(right, context.source.text());
        let simple = simple_name(type_name.trim());
        if self
            .ignore_type_names
            .iter()
            .any(|pattern| pattern == "*" || pattern.trim_start_matches('*') == simple)
        {
            return;
        }
        sink.report(
            expr.span,
            format!("The instanceof operator is used with {simple}"),
        );
    }
}

/// Flags nested for loops.
#[derive(Default)]
pub struct NestedForLoopRule {
    depth: usize,
}

impl Rule for NestedForLoopRule {}

impl AstRule for NestedForLoopRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::For).union(NodeMask::of(NodeKind::ForEach))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.depth += 1;
        if self.depth >= 2
            && let NodeRef::Stmt(stmt) = node
        {
            sink.report(stmt.span(), "Nested for loop");
        }
    }

    fn exit(
        &mut self,
        _node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        self.depth = self.depth.saturating_sub(1);
    }
}

/// Flags returning null from collection-returning methods.
pub struct ReturnsNullInsteadOfEmptyCollectionRule;

impl Rule for ReturnsNullInsteadOfEmptyCollectionRule {}

impl AstRule for ReturnsNullInsteadOfEmptyCollectionRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method).union(NodeMask::of(NodeKind::Expr))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Method(method) => {
                let Some(body) = &method.body else {
                    return;
                };
                let declared = method.return_type.as_deref().is_some_and(collection_type);
                if !declared && !returns_collection(body) {
                    return;
                }
                for span in null_returns(body) {
                    sink.report(
                        span,
                        "Returning null from a method that might return a Collection or Map",
                    );
                }
            }
            // A closure can return a collection too, and is then held to the same rule.
            NodeRef::Expr(Expr {
                kind: ExprKind::Closure { body, .. },
                ..
            }) => {
                if !returns_collection(body) {
                    return;
                }
                for span in null_returns(body) {
                    sink.report(
                        span,
                        "Returning null from a method that might return a Collection or Map",
                    );
                }
            }
            _ => {}
        }
    }
}

/// Collection types whose constructor or cast makes a method a collection producer.
fn collection_type(name: &str) -> bool {
    // A qualified type from another package is a different class: `java.awt.List`
    // is not a collection, and only the compiler could tell them apart.
    let package = name.rsplit_once('.').map(|(package, _)| package);
    if let Some(package) = package
        && package != "java.util"
        && package != "groovy"
    {
        return false;
    }
    matches!(
        simple_name(name),
        "Iterable"
            | "Collection"
            | "List"
            | "Map"
            | "Set"
            | "ArrayList"
            | "HashSet"
            | "LinkedList"
            | "TreeSet"
            | "HashMap"
            | "LinkedHashMap"
    )
}

/// Whether the block returns a collection, ignoring nested closures.
fn returns_collection(block: &Block) -> bool {
    let mut finder = ReturnFinder {
        closure_depth: 0,
        returns: Vec::new(),
    };
    crate::traverse::walk_block(block, &mut finder);
    finder.returns.iter().any(expression_returns_collection)
}

/// Whether a returned expression produces a collection.
fn expression_returns_collection(expr: &Expr) -> bool {
    match &unparen(expr).kind {
        ExprKind::List(_) | ExprKind::Map(_) => true,
        ExprKind::New { type_name, .. } | ExprKind::Cast { type_name, .. } => {
            collection_type(type_name)
        }
        ExprKind::Ternary {
            then_expr,
            else_expr,
            ..
        } => expression_returns_collection(then_expr) || expression_returns_collection(else_expr),
        _ => false,
    }
}

/// The `return null` statements of a block, ignoring nested closures.
fn null_returns(block: &Block) -> Vec<groovylint_core::Span> {
    let mut finder = ReturnFinder {
        closure_depth: 0,
        returns: Vec::new(),
    };
    crate::traverse::walk_block(block, &mut finder);
    finder
        .returns
        .iter()
        .filter(|value| returns_null(value))
        .map(|value| value.span)
        .collect()
}

/// Whether a returned expression can evaluate to `null`.
fn returns_null(expr: &Expr) -> bool {
    match &unparen(expr).kind {
        ExprKind::Literal(Literal::Null) => true,
        ExprKind::Ternary {
            then_expr,
            else_expr,
            ..
        } => returns_null(then_expr) || returns_null(else_expr),
        _ => false,
    }
}

/// Collects returned expressions, skipping the bodies of nested closures.
struct ReturnFinder {
    closure_depth: usize,
    returns: Vec<Expr>,
}

impl crate::traverse::AstVisitor for ReturnFinder {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Expr(Expr {
                kind: ExprKind::Closure { .. } | ExprKind::New { body: Some(_), .. },
                ..
            }) => self.closure_depth += 1,
            NodeRef::Stmt(Stmt::Return(ret)) if self.closure_depth == 0 => {
                if let Some(value) = &ret.value {
                    self.returns.push(value.clone());
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self, node: NodeRef<'_>) {
        let nested = matches!(
            node,
            NodeRef::Expr(Expr {
                kind: ExprKind::Closure { .. } | ExprKind::New { body: Some(_), .. },
                ..
            })
        );
        if nested {
            self.closure_depth = self.closure_depth.saturating_sub(1);
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
        descriptor(
            RuleMeta::new(
                "AssignmentToStaticFieldFromInstanceMethod",
                "org.codenarc.rule.design.AssignmentToStaticFieldFromInstanceMethodRule",
                Category::Design,
                Severity::Warning,
                "Assignments to static fields from instance methods are error-prone.",
            ),
            || RuleInstanceImpl::File(Box::new(AssignmentToStaticFieldFromInstanceMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BooleanMethodReturnsNull",
                "org.codenarc.rule.design.BooleanMethodReturnsNullRule",
                Category::Design,
                Severity::Warning,
                "Boolean methods should not return null.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BooleanMethodReturnsNullRule)),
        ),
        descriptor(
            RuleMeta::new(
                "Instanceof",
                "org.codenarc.rule.design.InstanceofRule",
                Category::Design,
                Severity::Warning,
                "Avoid instanceof checks.",
            ),
            || RuleInstanceImpl::Ast(Box::new(InstanceofRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "NestedForLoop",
                "org.codenarc.rule.design.NestedForLoopRule",
                Category::Design,
                Severity::Warning,
                "Avoid nested for loops.",
            ),
            || RuleInstanceImpl::Ast(Box::new(NestedForLoopRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ReturnsNullInsteadOfEmptyCollection",
                "org.codenarc.rule.design.ReturnsNullInsteadOfEmptyCollectionRule",
                Category::Design,
                Severity::Warning,
                "Return an empty collection instead of null.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ReturnsNullInsteadOfEmptyCollectionRule)),
        ),
    ]
}

/// Descriptors for the exception rules.
#[must_use]
pub fn exception_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "CatchThrowable",
                "org.codenarc.rule.exceptions.CatchThrowableRule",
                Category::Exceptions,
                Severity::Warning,
                "Catching Throwable is dangerous.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CatchThrowableRule)),
        ),
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
