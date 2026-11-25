//! `CodeNarc` naming rules.

use std::collections::{BTreeMap, BTreeSet};

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassDecl, ClassKind, ExprKind, Item, MethodDecl, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta, RuleProperties,
};
use crate::rules::util::{ClassStack, first_annotation_line};

/// The default identifier pattern `CodeNarc` uses for classes and methods.
const LOWER_CAMEL: &str = "[a-z]\\w*";
/// `CodeNarc`'s field/variable pattern excludes underscores.
const LOWER_CAMEL_FIELD: &str = "[a-z][a-zA-Z0-9]*";
const UPPER_CAMEL: &str = "([A-Z]\\w*\\$?)*";
const UPPER_SNAKE: &str = "[A-Z][A-Z0-9_]*";

/// Builds a rule pattern. `CodeNarc` matches names with `==~`, so the pattern
/// must cover the whole name. Lookarounds are common in configured patterns, so
/// a backtracking engine is used.
fn compile_anchored(pattern: &str) -> Result<Pattern, String> {
    let anchored = format!("^(?:{pattern})$");
    Pattern::new(&anchored).map_err(|error| format!("invalid regex {pattern}: {error}"))
}

fn regex_property(
    properties: &RuleProperties,
    name: &str,
    default: &Pattern,
) -> Result<Pattern, String> {
    match properties.get(name) {
        None => Ok(default.clone()),
        Some(pattern) => compile_anchored(pattern),
    }
}

fn ignored_names(properties: &RuleProperties, name: &str) -> Vec<String> {
    let Some(value) = properties.get(name) else {
        return Vec::new();
    };
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn simple_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// A rule pattern backed by a backtracking engine so that configured
/// lookarounds work like they do in `CodeNarc`.
#[derive(Clone)]
struct Pattern {
    regex: fancy_regex::Regex,
    text: String,
}

impl Pattern {
    fn new(pattern: &str) -> Result<Self, String> {
        let regex = fancy_regex::Regex::new(pattern)
            .map_err(|error| format!("invalid regex {pattern}: {error}"))?;
        Ok(Self {
            regex,
            text: pattern.to_string(),
        })
    }

    fn is_match(&self, name: &str) -> bool {
        self.regex.is_match(name).unwrap_or(false)
    }

    fn as_str(&self) -> &str {
        &self.text
    }

    fn default_upper_snake() -> Self {
        Self::new(&format!("^(?:{UPPER_SNAKE})$")).expect("default pattern must compile")
    }
}

/// Checks declared names against a regular expression.
pub struct NamePatternRule {
    pattern: Pattern,
    ignored: Vec<String>,
    /// Which declaration the pattern applies to.
    target: NameTarget,
    /// Pattern for `static final` declarations.
    static_final_pattern: Option<Pattern>,
    /// Pattern for `final` declarations, when configured.
    final_pattern: Option<Pattern>,
    /// Pattern for `static` declarations, when configured.
    static_pattern: Option<Pattern>,
    classes: ClassStack,
}

#[derive(Clone, Copy)]
enum NameTarget {
    Class,
    Interface,
    AbstractClass,
    Method,
    Parameter,
    Property,
    Variable,
    Field,
}

impl Rule for NamePatternRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.pattern = regex_property(properties, "regex", &self.pattern)?;
        self.ignored = match self.target {
            NameTarget::Method => ignored_names(properties, "ignoreMethodNames"),
            NameTarget::Parameter => ignored_names(properties, "ignoreParameterNames"),
            NameTarget::Property => ignored_names(properties, "ignorePropertyNames"),
            NameTarget::Variable => ignored_names(properties, "ignoreVariableNames"),
            NameTarget::Field => ignored_names(properties, "ignoreFieldNames"),
            _ => Vec::new(),
        };
        if matches!(
            self.target,
            NameTarget::Variable | NameTarget::Property | NameTarget::Field
        ) {
            self.static_final_pattern = Some(regex_property(
                properties,
                "staticFinalRegex",
                &Pattern::default_upper_snake(),
            )?);
            self.final_pattern = match properties.get("finalRegex") {
                None => None,
                Some(pattern) => Some(compile_anchored(pattern)?),
            };
            self.static_pattern = match properties.get("staticRegex") {
                None => None,
                Some(pattern) => Some(compile_anchored(pattern)?),
            };
        }
        Ok(())
    }
}

impl NamePatternRule {
    fn is_ignored(&self, name: &str) -> bool {
        self.ignored.iter().any(|ignored| ignored == name)
    }

    fn matches(&self, name: &str, is_static: bool, is_final: bool) -> bool {
        let selected = match (is_static, is_final) {
            (true, true) => self.static_final_pattern.as_ref(),
            (true, false) => self.static_pattern.as_ref(),
            (false, true) => self.final_pattern.as_ref(),
            (false, false) => None,
        };
        match selected {
            Some(pattern) => pattern.is_match(name),
            None => self.pattern.is_match(name),
        }
    }
}

impl FileRule for NamePatternRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        if matches!(self.target, NameTarget::Variable | NameTarget::Parameter) {
            let mut visitor = DeclarationNameVisitor { rule: self, sink };
            crate::traverse::walk_script(context.script, &mut visitor);
            return;
        }
        for item in &context.script.items {
            self.check_item(item, sink);
        }
    }
}

/// Reports variable and parameter names found anywhere in the file, including
/// inside closures.
struct DeclarationNameVisitor<'a, 'b> {
    rule: &'a NamePatternRule,
    sink: &'a mut DiagnosticSink<'b>,
}

impl crate::traverse::AstVisitor for DeclarationNameVisitor<'_, '_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Stmt(Stmt::VarDecl(decl)) => {
                if !matches!(self.rule.target, NameTarget::Variable) {
                    return;
                }
                if self.rule.is_ignored(&decl.name) || self.rule.matches(&decl.name, false, false) {
                    return;
                }
                self.sink.report(
                    decl.span,
                    format!(
                        "The variable name {} does not match {}",
                        decl.name,
                        self.rule.pattern.as_str()
                    ),
                );
            }
            NodeRef::Method(method) => self.check_params(&method.params),
            NodeRef::Expr(expr) => {
                if let ExprKind::Closure { params, .. } = &expr.kind {
                    self.check_params(params);
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

impl DeclarationNameVisitor<'_, '_> {
    fn check_params(&mut self, params: &[groovylint_syntax::ast::Param]) {
        if !matches!(self.rule.target, NameTarget::Parameter) {
            return;
        }
        for param in params {
            if self.rule.is_ignored(&param.name) || self.rule.matches(&param.name, false, false) {
                continue;
            }
            self.sink.report(
                param.span,
                format!(
                    "The parameter name {} does not match {}",
                    param.name,
                    self.rule.pattern.as_str()
                ),
            );
        }
    }
}

impl NamePatternRule {
    fn check_item(&mut self, item: &Item, sink: &mut DiagnosticSink<'_>) {
        match item {
            Item::Class(class) => {
                self.classes.push(&class.name);
                self.check_class(class, sink);
                for member in &class.members {
                    self.check_item(member, sink);
                }
                self.classes.pop();
            }
            Item::Method(method) => {
                self.check_method(method, sink);
                if let Some(body) = &method.body {
                    self.check_statements(&body.statements, sink);
                }
            }
            Item::Field(field) => {
                if matches!(self.target, NameTarget::Field) {
                    if self.is_ignored(&field.name) {
                        return;
                    }
                    if !self.matches(&field.name, field.is_static, field.is_final) {
                        sink.report(
                            field.span,
                            format!(
                                "The field name {} does not match {}",
                                field.name,
                                self.pattern.as_str()
                            ),
                        );
                    }
                    return;
                }
                // Groovy properties are the fields without an explicit
                // `public` or `private` modifier.
                if matches!(self.target, NameTarget::Property)
                    && !field.is_private
                    && !field.is_public
                    && !self.is_ignored(&field.name)
                    && !self.matches(&field.name, field.is_static, field.is_final)
                {
                    sink.report(
                        field.span,
                        format!(
                            "The property name {} in class is not a valid property name",
                            field.name
                        ),
                    );
                }
            }
            Item::Initializer(initializer) => {
                self.check_statements(&initializer.body.statements, sink);
            }
            Item::Statement(stmt) => self.check_statement(stmt, sink),
            Item::Group(items) => {
                for item in items {
                    self.check_item(item, sink);
                }
            }
        }
    }

    fn check_class(&self, class: &ClassDecl, sink: &mut DiagnosticSink<'_>) {
        let applies = match self.target {
            NameTarget::Class => matches!(class.kind, ClassKind::Class | ClassKind::Record),
            NameTarget::Interface => class.kind == ClassKind::Interface,
            NameTarget::AbstractClass => class.is_abstract && class.kind == ClassKind::Class,
            _ => false,
        };
        if !applies {
            return;
        }
        if self.matches(&class.name, false, false) {
            return;
        }
        sink.report(
            class.span,
            format!(
                "The {} name {} in class does not match {}",
                match self.target {
                    NameTarget::Interface => "interface",
                    NameTarget::AbstractClass => "abstract class",
                    _ => "class",
                },
                class.name,
                self.pattern.as_str()
            ),
        );
    }

    fn check_method(&mut self, method: &MethodDecl, sink: &mut DiagnosticSink<'_>) {
        if method.name == self.classes.current() {
            return;
        }
        if matches!(self.target, NameTarget::Method)
            && !self.is_ignored(&method.name)
            && !self.matches(&method.name, false, false)
        {
            sink.report(
                method.span,
                format!(
                    "The method name {} does not match {}",
                    method.name,
                    self.pattern.as_str()
                ),
            );
        }
    }

    fn check_statement(&mut self, stmt: &Stmt, sink: &mut DiagnosticSink<'_>) {
        match stmt {
            Stmt::Block(block) => {
                self.check_statements(&block.statements, sink);
            }
            Stmt::If(if_stmt) => {
                self.check_statement(&if_stmt.then_branch, sink);
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.check_statement(else_branch, sink);
                }
            }
            Stmt::While(while_stmt) => {
                self.check_statement(&while_stmt.body, sink);
            }
            Stmt::DoWhile(do_while) => {
                self.check_statement(&do_while.body, sink);
            }
            Stmt::For(for_stmt) => {
                self.check_statement(&for_stmt.body, sink);
            }
            Stmt::ForEach(for_each) => {
                self.check_statement(&for_each.body, sink);
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
                self.check_statement(&synchronized.body, sink);
            }
            Stmt::Switch(switch) => {
                for case in &switch.cases {
                    self.check_statements(&case.body, sink);
                }
            }
            _ => {}
        }
    }

    fn check_statements(&mut self, statements: &[Stmt], sink: &mut DiagnosticSink<'_>) {
        for statement in statements {
            self.check_statement(statement, sink);
        }
    }
}

/// Flags a single top-level class whose name differs from the file name.
pub struct ClassNameSameAsFilenameRule;

impl Rule for ClassNameSameAsFilenameRule {}

impl FileRule for ClassNameSameAsFilenameRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let Some(stem) = file_stem(context) else {
            return;
        };
        let classes: Vec<&ClassDecl> = context
            .script
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Class(class) => Some(class),
                _ => None,
            })
            .collect();
        if classes.len() != 1 || context.script.items.len() != 1 {
            return;
        }
        let class = classes[0];
        if class.name == stem {
            return;
        }
        let kind = match class.kind {
            ClassKind::Interface => "Interface",
            ClassKind::Enum => "Enum",
            _ => "Class",
        };
        // `CodeNarc` reports the first annotation line of the declaration.
        let line = first_annotation_line(context, context.source.line_col(class.span.start).line);
        let Some(span) = context.source.line_span(line) else {
            return;
        };
        sink.report(
            span,
            format!(
                "{kind} `{}` is the only class in `{stem}.groovy`. In such a case the file and the class should have the same name.",
                class.name
            ),
        );
    }
}

fn file_stem(context: &FileContext<'_>) -> Option<String> {
    let path = context.source.path().to_string_lossy().to_string();
    let name = path.rsplit('/').next()?;
    name.strip_suffix(".groovy").map(ToString::to_string)
}

/// Flags classes or interfaces whose simple name equals their supertype's.
pub struct SameNameAsSuperRule {
    interface_only: bool,
}

impl Rule for SameNameAsSuperRule {}

impl FileRule for SameNameAsSuperRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            let is_interface = class.kind == ClassKind::Interface;
            if self.interface_only != is_interface {
                continue;
            }
            let Some(parent) = &class.extends else {
                continue;
            };
            if simple_name(parent) != class.name {
                continue;
            }
            sink.report(
                class.span,
                format!(
                    "The type {} has the same simple name as its supertype {}",
                    class.name, parent
                ),
            );
        }
    }
}

/// Flags methods that almost override an `Object` method.
pub struct ObjectOverrideMisspelledMethodNameRule;

impl Rule for ObjectOverrideMisspelledMethodNameRule {}

impl FileRule for ObjectOverrideMisspelledMethodNameRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            for member in &class.members {
                let Item::Method(method) = member else {
                    continue;
                };
                check_object_override(method, sink);
            }
        }
    }
}

fn check_object_override(method: &MethodDecl, sink: &mut DiagnosticSink<'_>) {
    let name = method.name.as_str();
    let arity = method.params.len();
    if name == "equal" && arity == 1 {
        sink.report(
            method.span,
            "Trying to override the equals method using the name [equal]",
        );
        return;
    }
    let lower = name.to_ascii_lowercase();
    let expected = match (lower.as_str(), arity) {
        ("equals", 1) => "equals",
        ("hashcode", 0) => "hashCode",
        ("tostring", 0) => "toString",
        _ => return,
    };
    if name == expected {
        return;
    }
    sink.report(
        method.span,
        format!("Trying to override the {expected} method using the name [{name}]"),
    );
}

/// Flags methods whose name differs only by capitalization.
pub struct ConfusingMethodNameRule;

impl Rule for ConfusingMethodNameRule {}

impl AstRule for ConfusingMethodNameRule {
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
        check_confusing_names(class, sink);
    }
}

fn check_confusing_names(class: &ClassDecl, sink: &mut DiagnosticSink<'_>) {
    let mut closures: BTreeSet<String> = BTreeSet::new();
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    for member in &class.members {
        let Item::Field(field) = member else {
            continue;
        };
        let lower = field.name.to_ascii_lowercase();
        fields.insert(lower.clone(), field.name.clone());
        let is_closure = matches!(
            field.init.as_ref().map(|init| &init.kind),
            Some(ExprKind::Closure { .. })
        );
        if is_closure {
            closures.insert(lower);
        }
    }

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for member in &class.members {
        let Item::Method(method) = member else {
            continue;
        };
        if method.name == class.name {
            continue;
        }
        let lower = method.name.to_ascii_lowercase();
        let signature = format!("{lower}{}", parameter_signature(method));
        if closures.contains(&lower) {
            sink.report(
                method.span,
                format!(
                    "Found very confusing method name. Conflicts with a similar closure name. Found method : {} {}",
                    method.name,
                    parameter_signature(method)
                ),
            );
        } else if seen.contains(&signature) {
            sink.report(
                method.span,
                format!(
                    "Found very confusing method name: {} {}",
                    method.name,
                    parameter_signature(method)
                ),
            );
        } else if let Some(field) = fields.get(&lower) {
            sink.report(
                method.span,
                format!(
                    "The method name {} is similar to the field name {}",
                    method.name, field
                ),
            );
        }
        seen.insert(signature);
    }
}

fn parameter_signature(method: &MethodDecl) -> String {
    let types: Vec<String> = method
        .params
        .iter()
        .map(|param| {
            param
                .type_name
                .clone()
                .unwrap_or_else(|| "java.lang.Object".to_string())
        })
        .collect();
    format!("({})", types.join(", "))
}

fn default_pattern(pattern: &str) -> Pattern {
    compile_anchored(pattern).expect("built-in pattern must compile")
}

fn lower_camel() -> Pattern {
    default_pattern(LOWER_CAMEL)
}

fn lower_camel_field() -> Pattern {
    default_pattern(LOWER_CAMEL_FIELD)
}

fn upper_camel() -> Pattern {
    default_pattern(UPPER_CAMEL)
}

fn upper_snake() -> Pattern {
    default_pattern(UPPER_SNAKE)
}

/// Checks the package name against a regular expression.
pub struct PackageNameRule;

impl Rule for PackageNameRule {}

impl FileRule for PackageNameRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let Some(package) = &context.script.package else {
            return;
        };
        let Ok(pattern) = fancy_regex::Regex::new(r"^(?:[a-z]+[a-z0-9]*(\.[a-z0-9]+)*)$") else {
            return;
        };
        if pattern.is_match(&package.name).unwrap_or(false) {
            return;
        }
        sink.report(
            package.span,
            format!("The package name {} is not valid", package.name),
        );
    }
}

/// Checks that the declared package matches the file path below `groupId`.
#[derive(Default)]
pub struct PackageNameMatchesFilePathRule {
    group_id: String,
}

impl Rule for PackageNameMatchesFilePathRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.group_id = properties.get("groupId").cloned().unwrap_or_default();
        Ok(())
    }
}

impl FileRule for PackageNameMatchesFilePathRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        if self.group_id.is_empty() {
            return;
        }
        let Some(package) = &context.script.package else {
            return;
        };
        let path = context.source.path().to_string_lossy().to_string();
        let Some(index) = path.find(&self.group_id) else {
            sink.report(
                package.span,
                format!(
                    "Could not find groupId '{}' in the file path",
                    self.group_id
                ),
            );
            return;
        };
        let tail = &path[index + self.group_id.len()..];
        let folder = tail.rsplit_once('/').map_or("", |(folder, _)| folder);
        let folders = folder.trim_matches('/').replace('/', ".");
        let package_tail = package
            .name
            .split_once(&self.group_id)
            .map_or(package.name.as_str(), |(_, tail)| tail)
            .trim_matches('.');
        if folders == package_tail {
            return;
        }
        sink.report(
            package.span,
            format!(
                "The package {} does not match the file path ({folders})",
                package.name
            ),
        );
    }
}

/// Flags factory methods outside builder classes.
pub struct FactoryMethodNameRule {
    pattern: regex::Regex,
}

impl Default for FactoryMethodNameRule {
    fn default() -> Self {
        Self {
            pattern: regex::Regex::new("^(build|create).*").expect("valid default regex"),
        }
    }
}

impl Rule for FactoryMethodNameRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("regex") {
            self.pattern = regex::Regex::new(value)
                .map_err(|error| format!("invalid factory method regex: {error}"))?;
        }
        Ok(())
    }
}

impl AstRule for FactoryMethodNameRule {
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
        if !self.pattern.is_match(&method.name)
            || crate::rules::util::has_annotation(&method.annotations, "Override")
        {
            return;
        }
        sink.report(
            method.span,
            format!(
                "The method {} matches the factory-method naming pattern",
                method.name
            ),
        );
    }
}

/// Descriptors for naming rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "FactoryMethodName",
                "org.codenarc.rule.naming.FactoryMethodNameRule",
                Category::Naming,
                Severity::Warning,
                "Factory methods should follow the configured naming convention.",
            ),
            || RuleInstanceImpl::Ast(Box::new(FactoryMethodNameRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ClassName",
                "org.codenarc.rule.naming.ClassNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks class names against a regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: upper_camel(),
                    ignored: Vec::new(),
                    target: NameTarget::Class,
                    static_final_pattern: None,
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "InterfaceName",
                "org.codenarc.rule.naming.InterfaceNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks interface names against a configured regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: upper_camel(),
                    ignored: Vec::new(),
                    target: NameTarget::Interface,
                    static_final_pattern: None,
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "AbstractClassName",
                "org.codenarc.rule.naming.AbstractClassNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks abstract class names against a configured regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: upper_camel(),
                    ignored: Vec::new(),
                    target: NameTarget::AbstractClass,
                    static_final_pattern: None,
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "MethodName",
                "org.codenarc.rule.naming.MethodNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks method names against a regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: lower_camel(),
                    ignored: Vec::new(),
                    target: NameTarget::Method,
                    static_final_pattern: None,
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "ParameterName",
                "org.codenarc.rule.naming.ParameterNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks parameter names against a regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: lower_camel_field(),
                    ignored: Vec::new(),
                    target: NameTarget::Parameter,
                    static_final_pattern: None,
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "PropertyName",
                "org.codenarc.rule.naming.PropertyNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks property names against a regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: lower_camel_field(),
                    ignored: Vec::new(),
                    target: NameTarget::Property,
                    static_final_pattern: Some(upper_snake()),
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "FieldName",
                "org.codenarc.rule.naming.FieldNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks field names against a regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: lower_camel_field(),
                    ignored: vec!["serialVersionUID".to_string()],
                    target: NameTarget::Field,
                    static_final_pattern: Some(upper_snake()),
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "PackageName",
                "org.codenarc.rule.naming.PackageNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks the package name against a regular expression.",
            ),
            || RuleInstanceImpl::File(Box::new(PackageNameRule)),
        ),
        descriptor(
            RuleMeta::new(
                "VariableName",
                "org.codenarc.rule.naming.VariableNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks variable names against a regular expression.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(NamePatternRule {
                    pattern: lower_camel_field(),
                    ignored: Vec::new(),
                    target: NameTarget::Variable,
                    static_final_pattern: Some(upper_snake()),
                    final_pattern: None,
                    static_pattern: None,
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "ClassNameSameAsFilename",
                "org.codenarc.rule.naming.ClassNameSameAsFilenameRule",
                Category::Naming,
                Severity::Warning,
                "Checks that the only class in a file has the file's name.",
            ),
            || RuleInstanceImpl::File(Box::new(ClassNameSameAsFilenameRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ClassNameSameAsSuperclass",
                "org.codenarc.rule.naming.ClassNameSameAsSuperclassRule",
                Category::Naming,
                Severity::Warning,
                "Checks classes named like their superclass.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(SameNameAsSuperRule {
                    interface_only: false,
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "InterfaceNameSameAsSuperInterface",
                "org.codenarc.rule.naming.InterfaceNameSameAsSuperInterfaceRule",
                Category::Naming,
                Severity::Warning,
                "Checks interfaces named like their super-interface.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(SameNameAsSuperRule {
                    interface_only: true,
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "ObjectOverrideMisspelledMethodName",
                "org.codenarc.rule.naming.ObjectOverrideMisspelledMethodNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks for misspelled Object method overrides.",
            ),
            || RuleInstanceImpl::File(Box::new(ObjectOverrideMisspelledMethodNameRule)),
        ),
        descriptor(
            RuleMeta::new(
                "PackageNameMatchesFilePath",
                "org.codenarc.rule.naming.PackageNameMatchesFilePathRule",
                Category::Naming,
                Severity::Warning,
                "Checks that the package matches the file path.",
            ),
            || RuleInstanceImpl::File(Box::new(PackageNameMatchesFilePathRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ConfusingMethodName",
                "org.codenarc.rule.naming.ConfusingMethodNameRule",
                Category::Naming,
                Severity::Warning,
                "Checks for method names that differ only by capitalization.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ConfusingMethodNameRule)),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::parameter_signature;
    use groovylint_core::Span;
    use groovylint_syntax::ast::{MethodDecl, Param};

    #[test]
    fn parameter_signature_lists_declared_types() {
        let method = MethodDecl {
            name: "run".to_string(),
            return_type: None,
            is_static: false,
            is_private: false,
            is_public: false,
            is_protected: false,
            is_final: false,
            is_abstract: false,
            is_synchronized: false,
            throws: Vec::new(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    type_name: Some("String".to_string()),
                    annotations: Vec::new(),
                    annotation_details: Vec::new(),
                    type_annotations: Vec::new(),
                    default_value: None,
                    span: Span::empty(0),
                },
                Param {
                    name: "b".to_string(),
                    type_name: None,
                    annotations: Vec::new(),
                    annotation_details: Vec::new(),
                    type_annotations: Vec::new(),
                    default_value: None,
                    span: Span::empty(0),
                },
            ],
            annotations: Vec::new(),
            annotation_details: Vec::new(),
            suppressed_rules: Vec::new(),
            body: None,
            span: Span::empty(0),
        };
        assert_eq!(parameter_signature(&method), "(String, java.lang.Object)");
    }
}
