//! `CodeNarc` rules about unused declarations and allocations.

use std::collections::BTreeSet;

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassDecl, ClassKind, ExprKind, Item, MethodDecl, Script, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    Category, DiagnosticSink, FileContext, FileRule, NodeRef, Rule, RuleMeta, RuleProperties,
};
use crate::rules::util::is_test_file;
use crate::traverse::{self, AstVisitor};

/// Collects every name that is read, called or assigned.
struct NameCollector<'a> {
    names: BTreeSet<String>,
    /// Every member name, whichever receiver it is reached through. `CodeNarc`
    /// treats `getX`/`setX`/`isX` as used when `x` appears anywhere.
    member_names: BTreeSet<String>,
    class_names: &'a BTreeSet<String>,
}

impl AstVisitor for NameCollector<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        // An annotation argument refers to names like any other expression,
        // but the parser keeps it as raw text: `@MessageResource(LEVEL_CODE)`.
        let arguments = annotation_arguments(node);
        if !arguments.is_empty() {
            for name in identifier_words(&strip_quoted(&arguments)) {
                self.names.insert(name);
            }
        }

        let NodeRef::Expr(expr) = node else {
            return;
        };
        match &expr.kind {
            ExprKind::Variable(name) => {
                self.names.insert(name.clone());
                self.member_names.insert(name.clone());
            }
            // `CodeNarc` only counts a member as used when it is reached through
            // `this` (or implicitly) or through a class name declared in the file;
            // `other.field` and `it.method()` do not count. Mirroring that keeps
            // our findings a superset of CodeNarc's.
            ExprKind::Property { receiver, name, .. } => {
                self.member_names.insert(member_name(name).to_string());
                if self.reaches_this(receiver) {
                    self.names.insert(member_name(name).to_string());
                }
            }
            ExprKind::Call { receiver, name, .. } => {
                let reaches = match receiver {
                    None => true,
                    Some(receiver) => self.reaches_this(receiver),
                };
                self.member_names.insert(member_name(name).to_string());
                if reaches {
                    self.names.insert(member_name(name).to_string());
                }
            }
            ExprKind::Literal(groovylint_syntax::ast::Literal::String(text)) => {
                // Only interpolated strings reference names; a plain literal
                // (`'HTTP-NATIVE-ATTEMPT'`) must not count as a use of `ATTEMPT`.
                if text.contains('$') {
                    for name in identifier_words(text) {
                        self.names.insert(name);
                    }
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

impl NameCollector<'_> {
    /// Whether an expression is `this`, a bare name bound to a class, or a
    /// declared class reference.
    fn reaches_this(&self, expr: &groovylint_syntax::ast::Expr) -> bool {
        match &expr.kind {
            ExprKind::This => true,
            ExprKind::Variable(name) => name == "this" || self.class_names.contains(name.as_str()),
            _ => false,
        }
    }
}

/// The annotation arguments written on a declaration node.
fn annotation_arguments(node: NodeRef<'_>) -> String {
    let details = match node {
        NodeRef::Class(class) => &class.annotation_details,
        NodeRef::Method(method) => &method.annotation_details,
        NodeRef::Field(field) => &field.annotation_details,
        NodeRef::Stmt(Stmt::VarDecl(declaration)) => &declaration.annotation_details,
        _ => return String::new(),
    };
    let mut arguments = String::new();
    for annotation in details {
        if let Some(raw) = &annotation.raw_arguments {
            arguments.push(' ');
            arguments.push_str(raw);
        }
    }
    // A parameter annotation references names too: `@RequestHeader(HEADER) String x`.
    if let NodeRef::Method(method) = node {
        for param in &method.params {
            for annotation in &param.annotation_details {
                if let Some(raw) = &annotation.raw_arguments {
                    arguments.push(' ');
                    arguments.push_str(raw);
                }
            }
        }
    }
    arguments
}

/// Drops string literal bodies: they are values, not references.
fn strip_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote = None;
    for ch in text.chars() {
        match quote {
            Some(active) if ch == active => {
                quote = None;
                out.push(' ');
            }
            Some(_) => {}
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None => out.push(ch),
        }
    }
    out
}

/// Strips the operator that turns a name into a member reference: `&name`,
/// `::name`.
fn member_name(name: &str) -> &str {
    name.trim_start_matches('&').trim_start_matches("::")
}

/// Splits identifier words out of a string literal. Interpolated expressions
/// are plain text in the token stream, so every word may be a reference.
fn identifier_words(text: &str) -> Vec<String> {
    text.split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .filter(|word| !word.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn names_in_script(script: &Script) -> BTreeSet<String> {
    collect_names(script).0
}

/// The names reached through `this`/a class, and every member name.
fn collect_names(script: &Script) -> (BTreeSet<String>, BTreeSet<String>) {
    let class_names = declared_class_names(script);
    let mut collector = NameCollector {
        names: BTreeSet::new(),
        member_names: BTreeSet::new(),
        class_names: &class_names,
    };
    traverse::walk_script(script, &mut collector);
    (collector.names, collector.member_names)
}

fn names_in_method(method: &MethodDecl, class_names: &BTreeSet<String>) -> BTreeSet<String> {
    let mut collector = NameCollector {
        names: BTreeSet::new(),
        member_names: BTreeSet::new(),
        class_names,
    };
    if let Some(body) = &method.body {
        traverse::walk_block(body, &mut collector);
    }
    collector.names
}

fn declared_class_names(script: &Script) -> BTreeSet<String> {
    classes_in_script(script)
        .into_iter()
        .map(|class| class.name.clone())
        .collect()
}

fn classes_in_script(script: &Script) -> Vec<&ClassDecl> {
    let mut classes = Vec::new();
    for item in &script.items {
        if let Item::Class(class) = item {
            collect_classes(class, &mut classes);
        }
    }
    classes
}

fn collect_classes<'a>(class: &'a ClassDecl, classes: &mut Vec<&'a ClassDecl>) {
    classes.push(class);
    for member in &class.members {
        if let Item::Class(nested) = member {
            collect_classes(nested, classes);
        }
    }
}

fn qualified_class_name(script: &Script, class: &ClassDecl) -> String {
    match &script.package {
        Some(package) => format!("{}.{}", package.name, class.name),
        None => class.name.clone(),
    }
}

fn has_annotation(annotations: &[String], name: &str) -> bool {
    annotations
        .iter()
        .any(|annotation| annotation == name || annotation.ends_with(&format!(".{name}")))
}

/// Flags private fields that are never referenced in the file.
#[allow(clippy::struct_field_names)]
pub struct UnusedPrivateFieldRule {
    ignored_names: Vec<String>,
    ignored_class_annotations: Vec<String>,
    /// Annotations that mark a field the framework fills in, not the code.
    ///
    /// `CodeNarc` has no such property: any `@Mock`, `@InjectMocks` or
    /// `@Autowired` field it sees as unreferenced is reported. Leave it unset to
    /// keep `CodeNarc` behavior.
    ignored_field_annotations: Vec<String>,
}

impl Default for UnusedPrivateFieldRule {
    fn default() -> Self {
        Self {
            ignored_names: vec!["serialVersionUID".to_string()],
            ignored_class_annotations: vec!["Entity".to_string()],
            ignored_field_annotations: Vec::new(),
        }
    }
}

impl Rule for UnusedPrivateFieldRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("ignoreFieldNames") {
            self.ignored_names = split_names(value);
        }
        if let Some(value) = properties.get("ignoreClassesAnnotatedWithNames") {
            self.ignored_class_annotations = split_names(value);
        }
        if let Some(value) = properties.get("ignoreFieldsAnnotatedWith") {
            self.ignored_field_annotations = split_names(value);
        }
        Ok(())
    }
}

impl FileRule for UnusedPrivateFieldRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let used = names_in_script(context.script);
        for class in classes_in_script(context.script) {
            if self.should_ignore_class(class) {
                continue;
            }
            for field in private_fields(class) {
                if self.ignored_names.contains(&field.name) {
                    continue;
                }
                if has_annotation(&field.annotations, "Delegate") {
                    continue;
                }
                let injected = self
                    .ignored_field_annotations
                    .iter()
                    .any(|name| has_annotation(&field.annotations, name));
                if injected {
                    continue;
                }
                if used.contains(&field.name) {
                    continue;
                }
                sink.report(
                    field.span,
                    format!(
                        "The field {} is not used within the class {}",
                        field.name,
                        qualified_class_name(context.script, class)
                    ),
                );
            }
        }
    }
}

impl UnusedPrivateFieldRule {
    fn should_ignore_class(&self, class: &ClassDecl) -> bool {
        self.ignored_class_annotations
            .iter()
            .any(|name| has_annotation(&class.annotations, name))
    }
}

fn private_fields(class: &ClassDecl) -> Vec<&groovylint_syntax::ast::FieldDecl> {
    class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Field(field) if field.is_private => Some(field),
            _ => None,
        })
        .collect()
}

fn private_methods(class: &ClassDecl) -> Vec<&MethodDecl> {
    class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Method(method) => {
                // A constructor without a modifier is private in Java semantics,
                // which is what `CodeNarc` sees for enum constructors too.
                let private = method.is_private
                    || (method.name == class.name && class.kind == ClassKind::Enum);
                (private && method.body.is_some()).then_some(method)
            }
            _ => None,
        })
        .collect()
}

fn split_names(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Flags private methods that are never called in the file.
#[derive(Default)]
pub struct UnusedPrivateMethodRule {
    ignored_annotations: Vec<String>,
}

impl Rule for UnusedPrivateMethodRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let value = properties
            .get("ignoreMethodsWithAnnotationNames")
            .map_or("", String::as_str);
        self.ignored_annotations = split_names(value);
        Ok(())
    }
}

impl FileRule for UnusedPrivateMethodRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let (used, member_names) = collect_names(context.script);
        let mut reported = BTreeSet::new();
        for class in classes_in_script(context.script) {
            for method in private_methods(class) {
                if method.name == class.name {
                    continue;
                }
                if reported.contains(&method.name) {
                    continue;
                }
                if self.is_ignored(method) {
                    continue;
                }
                if used.contains(&method.name) || Self::is_used_as_property(method, &member_names) {
                    continue;
                }
                reported.insert(method.name.clone());
                sink.report(
                    method.span,
                    format!(
                        "The method {} is not used within the class {}",
                        method.name,
                        qualified_class_name(context.script, class)
                    ),
                );
            }
        }
    }
}

impl UnusedPrivateMethodRule {
    /// Groovy property syntax calls `getX`/`setX`/`isX` through `x`.
    fn is_used_as_property(method: &MethodDecl, used: &BTreeSet<String>) -> bool {
        match super::imports::accessor_property(&method.name) {
            Some(property) => used.contains(&property),
            None => false,
        }
    }

    fn is_ignored(&self, method: &MethodDecl) -> bool {
        self.ignored_annotations
            .iter()
            .any(|name| has_annotation(&method.annotations, name))
    }
}

/// Flags parameters of private methods that the body never reads.
pub struct UnusedPrivateMethodParameterRule {
    ignored_pattern: regex::Regex,
}

impl Default for UnusedPrivateMethodParameterRule {
    fn default() -> Self {
        Self {
            ignored_pattern: regex::Regex::new("ignore|ignored").expect("valid default regex"),
        }
    }
}

impl Rule for UnusedPrivateMethodParameterRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let Some(value) = properties.get("ignoreRegex") else {
            return Ok(());
        };
        self.ignored_pattern =
            regex::Regex::new(value).map_err(|error| format!("invalid ignoreRegex: {error}"))?;
        Ok(())
    }
}

impl FileRule for UnusedPrivateMethodParameterRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes_in_script(context.script) {
            for method in private_methods(class) {
                let used = names_in_method(method, &declared_class_names(context.script));
                for param in &method.params {
                    if used.contains(&param.name) {
                        continue;
                    }
                    if self.ignored_pattern.is_match(&param.name) {
                        continue;
                    }
                    sink.report(
                        method.span,
                        format!(
                            "Method parameter [{}] is never referenced in the method {} of class {}",
                            param.name,
                            method.name,
                            qualified_class_name(context.script, class)
                        ),
                    );
                }
            }
        }
    }
}

/// Flags allocations whose result is discarded.
pub struct UnusedObjectRule {
    is_array: bool,
    skip_test_files: bool,
}

impl Rule for UnusedObjectRule {}

impl FileRule for UnusedObjectRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        if self.skip_test_files && is_test_file(context.source.path().to_string_lossy().as_ref()) {
            return;
        }
        for item in &context.script.items {
            self.check_item(item, sink);
        }
        // `CodeNarc` sees a single-expression lambda body as a bare expression
        // statement (not a block), so `() -> new X()` is reported as unused.
        // Brace closures (`{ -> new X() }`) are blocks and are not reported.
        if !self.is_array {
            let mut collector = ArrowLambdaCollector {
                spans: Vec::new(),
                source: context.source.text(),
            };
            traverse::walk_script(context.script, &mut collector);
            for span in collector.spans {
                sink.report(span, "The instantiated object is not used");
            }
        }
    }
}

/// Finds constructor calls that are the sole statement of an arrow lambda.
struct ArrowLambdaCollector<'a> {
    spans: Vec<groovylint_core::Span>,
    source: &'a str,
}

impl AstVisitor for ArrowLambdaCollector<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Closure {
            body, has_arrow, ..
        } = &expr.kind
        else {
            return;
        };
        if !*has_arrow {
            return;
        }
        let is_lambda = self
            .source
            .get(expr.span.range())
            .is_some_and(|text| !text.trim_start().starts_with('{'));
        if !is_lambda {
            return;
        }
        let [Stmt::Expr(inner)] = body.statements.as_slice() else {
            return;
        };
        if matches!(inner.kind, ExprKind::New { .. }) {
            self.spans.push(inner.span);
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

impl UnusedObjectRule {
    fn check_item(&mut self, item: &Item, sink: &mut DiagnosticSink<'_>) {
        match item {
            Item::Class(class) => {
                for member in &class.members {
                    self.check_item(member, sink);
                }
            }
            Item::Method(method) => {
                if let Some(body) = &method.body {
                    self.check_statements(&body.statements, sink);
                }
            }
            Item::Initializer(initializer) => {
                self.check_statements(&initializer.body.statements, sink);
            }
            Item::Statement(stmt) => self.check_single_statement(stmt, true, sink),
            Item::Field(_) => {}
            Item::Group(items) => {
                for item in items {
                    self.check_item(item, sink);
                }
            }
        }
    }

    fn check_statements(&mut self, statements: &[Stmt], sink: &mut DiagnosticSink<'_>) {
        for (index, stmt) in statements.iter().enumerate() {
            let is_last = index + 1 == statements.len();
            self.check_single_statement(stmt, is_last, sink);
        }
    }

    fn check_single_statement(
        &mut self,
        stmt: &Stmt,
        is_last: bool,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match stmt {
            Stmt::Expr(expr) => {
                if is_last {
                    return;
                }
                let is_allocation = match &expr.kind {
                    ExprKind::New { type_name, .. } => type_name.ends_with("[]") == self.is_array,
                    _ => false,
                };
                if !is_allocation {
                    return;
                }
                if self.is_array {
                    sink.report(expr.span, "The result of the array expression is ignored");
                } else {
                    sink.report(expr.span, "The instantiated object is not used");
                }
            }
            Stmt::Block(block) => {
                self.check_statements(&block.statements, sink);
            }
            Stmt::If(if_stmt) => {
                self.check_single_statement(&if_stmt.then_branch, false, sink);
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.check_single_statement(else_branch, false, sink);
                }
            }
            Stmt::While(while_stmt) => {
                self.check_single_statement(&while_stmt.body, false, sink);
            }
            Stmt::DoWhile(do_while) => {
                self.check_single_statement(&do_while.body, false, sink);
            }
            Stmt::For(for_stmt) => {
                self.check_single_statement(&for_stmt.body, false, sink);
            }
            Stmt::ForEach(for_each) => {
                self.check_single_statement(&for_each.body, false, sink);
            }
            Stmt::Try(try_stmt) => {
                self.check_statements(&try_stmt.body.statements, sink);
                for catch in &try_stmt.catches {
                    self.check_statements(&catch.body.statements, sink);
                }
                if let Some(finally) = &try_stmt.finally {
                    self.check_statements(&finally.statements, sink);
                }
            }
            Stmt::Synchronized(synchronized) => {
                self.check_single_statement(&synchronized.body, false, sink);
            }
            Stmt::Switch(switch) => {
                for case in &switch.cases {
                    for statement in &case.body {
                        self.check_single_statement(statement, false, sink);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Flags local variables that the file never reads.
pub struct UnusedVariableRule;

impl Rule for UnusedVariableRule {}

impl FileRule for UnusedVariableRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let used = names_in_script(context.script);
        let mut declarations = Vec::new();
        let mut collector = DeclarationCollector {
            declarations: &mut declarations,
        };
        traverse::walk_script(context.script, &mut collector);

        for (name, span) in declarations {
            if used.contains(&name) {
                continue;
            }
            sink.report(span, format!("Variable [{name}] is never referenced"));
        }
    }
}

/// Collects local variable declarations.
struct DeclarationCollector<'a> {
    declarations: &'a mut Vec<(String, groovylint_core::Span)>,
}

impl AstVisitor for DeclarationCollector<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        if let NodeRef::Stmt(Stmt::VarDecl(decl)) = node {
            self.declarations.push((decl.name.clone(), decl.span));
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Descriptors for unused rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "UnusedPrivateField",
                "org.codenarc.rule.unused.UnusedPrivateFieldRule",
                Category::Unused,
                Severity::Warning,
                "Checks for private fields that are never referenced.",
            ),
            || RuleInstanceImpl::File(Box::new(UnusedPrivateFieldRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "UnusedPrivateMethod",
                "org.codenarc.rule.unused.UnusedPrivateMethodRule",
                Category::Unused,
                Severity::Warning,
                "Checks for private methods that are never called.",
            ),
            || RuleInstanceImpl::File(Box::new(UnusedPrivateMethodRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "UnusedPrivateMethodParameter",
                "org.codenarc.rule.unused.UnusedPrivateMethodParameterRule",
                Category::Unused,
                Severity::Warning,
                "Checks for unused private method parameters.",
            ),
            || RuleInstanceImpl::File(Box::new(UnusedPrivateMethodParameterRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "UnusedVariable",
                "org.codenarc.rule.unused.UnusedVariableRule",
                Category::Unused,
                Severity::Warning,
                "Checks for local variables that are never read.",
            ),
            || RuleInstanceImpl::File(Box::new(UnusedVariableRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnusedObject",
                "org.codenarc.rule.unused.UnusedObjectRule",
                Category::Unused,
                Severity::Warning,
                "Checks for discarded object allocations.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(UnusedObjectRule {
                    is_array: false,
                    skip_test_files: true,
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "UnusedArray",
                "org.codenarc.rule.unused.UnusedArrayRule",
                Category::Unused,
                Severity::Warning,
                "Checks for discarded array allocations.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(UnusedObjectRule {
                    is_array: true,
                    skip_test_files: false,
                }))
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::is_test_file;

    #[test]
    fn detects_test_file_names() {
        assert!(is_test_file("a/b/PolicyServiceTest.groovy"));
        assert!(is_test_file("a/PolicySpec.groovy"));
        assert!(!is_test_file("a/PolicyService.groovy"));
        assert!(!is_test_file("a/PolicyTestHelper.groovy"));
    }
}
