//! `CodeNarc` convention rules about declarations and characters.

use groovylint_core::Severity;
use groovylint_syntax::ast::{AssignOp, Expr, ExprKind, Literal, MethodDecl, Param, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta, RuleProperties,
};
use crate::rules::util::ClassStack;

/// Flags source files containing tab characters.
pub struct NoTabCharacterRule;

impl Rule for NoTabCharacterRule {}

impl FileRule for NoTabCharacterRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let text = context.source.text();
        let Some(index) = text.find('\t') else {
            return;
        };
        let Some(span) = context
            .source
            .line_span(context.source.line_col(index).line)
        else {
            return;
        };
        sink.report(span, "The tab character is not allowed in source files");
    }
}

/// Flags `long` literals written with a lower-case `l` suffix.
pub struct LongLiteralWithLowerCaseLRule;

impl Rule for LongLiteralWithLowerCaseLRule {}

impl AstRule for LongLiteralWithLowerCaseLRule {
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
        let ExprKind::Literal(Literal::Number(text)) = &expr.kind else {
            return;
        };
        let Some(value) = text.strip_suffix('l') else {
            return;
        };
        sink.report(
            expr.span,
            format!("The literal {value}l should be rewritten {value}L"),
        );
    }
}

/// Flags `def` on declarations, methods, parameters and fields.
#[derive(Default)]
pub struct NoDefRule {
    classes: ClassStack,
}

impl Rule for NoDefRule {}

impl AstRule for NoDefRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::VarDecl)
            .union(NodeMask::of(NodeKind::Method))
            .union(NodeMask::of(NodeKind::Field))
            .union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(_) => self.classes.enter(node),
            NodeRef::Stmt(Stmt::VarDecl(decl)) if decl.type_name.is_none() => {
                sink.report(decl.span, "def for declaration should not be used");
            }
            NodeRef::Field(field) if field.type_name.is_none() => {
                sink.report(field.span, "def should not be used for field type");
            }
            NodeRef::Method(method) => {
                if self.is_constructor(method) {
                    report_dynamic_parameters(method, sink);
                    return;
                }
                report_dynamic_method(method, sink);
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
            self.classes.exit(node);
        }
    }
}

impl NoDefRule {
    fn is_constructor(&self, method: &MethodDecl) -> bool {
        method.name == self.classes.current()
    }
}

fn report_dynamic_method(method: &MethodDecl, sink: &mut DiagnosticSink<'_>) {
    if method.return_type.is_none() && method.body.is_some() {
        sink.report(method.span, "def for method return type should not be used");
    }
    report_dynamic_parameters(method, sink);
}

fn report_dynamic_parameters(method: &MethodDecl, sink: &mut DiagnosticSink<'_>) {
    for param in &method.params {
        report_dynamic_parameter(param, sink);
    }
}

fn report_dynamic_parameter(param: &Param, sink: &mut DiagnosticSink<'_>) {
    if param.type_name.is_some() {
        return;
    }
    sink.report(
        param.span,
        "def for method parameter type should not be used",
    );
}

/// Flags declarations without an explicit type.
pub struct VariableTypeRequiredRule {
    ignored_names: WildcardList,
}

impl Default for VariableTypeRequiredRule {
    fn default() -> Self {
        Self {
            ignored_names: WildcardList::new(""),
        }
    }
}

impl Rule for VariableTypeRequiredRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let pattern = properties
            .get("ignoreVariableNames")
            .map_or("", String::as_str);
        self.ignored_names = WildcardList::new(pattern);
        Ok(())
    }
}

impl AstRule for VariableTypeRequiredRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::VarDecl)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(Stmt::VarDecl(decl)) = node else {
            return;
        };
        if decl.type_name.is_some() || self.ignored_names.matches(&decl.name) {
            return;
        }
        sink.report(
            decl.span,
            format!("The type is not specified for variable \"{}\"", decl.name),
        );
    }
}

/// Flags fields without an explicit type.
pub struct FieldTypeRequiredRule {
    ignored_names: WildcardList,
}

impl Default for FieldTypeRequiredRule {
    fn default() -> Self {
        Self {
            ignored_names: WildcardList::new(""),
        }
    }
}

impl Rule for FieldTypeRequiredRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let pattern = properties
            .get("ignoreFieldNames")
            .map_or("", String::as_str);
        self.ignored_names = WildcardList::new(pattern);
        Ok(())
    }
}

impl AstRule for FieldTypeRequiredRule {
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
        if field.type_name.is_some() || self.ignored_names.matches(&field.name) {
            return;
        }
        sink.report(
            field.span,
            format!("The type is not specified for field \"{}\"", field.name),
        );
    }
}

/// Flags methods without an explicit return type.
pub struct MethodReturnTypeRequiredRule {
    ignored_names: WildcardList,
    classes: ClassStack,
}

impl Default for MethodReturnTypeRequiredRule {
    fn default() -> Self {
        Self {
            ignored_names: WildcardList::new(""),
            classes: ClassStack::default(),
        }
    }
}

impl Rule for MethodReturnTypeRequiredRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let pattern = properties
            .get("ignoreMethodNames")
            .map_or("", String::as_str);
        self.ignored_names = WildcardList::new(pattern);
        Ok(())
    }
}

impl AstRule for MethodReturnTypeRequiredRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method).union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Class(_)) {
            self.classes.enter(node);
            return;
        }
        let NodeRef::Method(method) = node else {
            return;
        };
        if method.name == self.classes.current() {
            return;
        }
        if method.return_type.is_some() || self.ignored_names.matches(&method.name) {
            return;
        }
        sink.report(
            method.span,
            format!("Method \"{}\" has a dynamic return type", method.name),
        );
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Class(_)) {
            self.classes.exit(node);
        }
    }
}

/// Flags method parameters without an explicit type.
pub struct MethodParameterTypeRequiredRule {
    ignored_names: WildcardList,
}

impl Default for MethodParameterTypeRequiredRule {
    fn default() -> Self {
        Self {
            ignored_names: WildcardList::new(""),
        }
    }
}

impl Rule for MethodParameterTypeRequiredRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let pattern = properties
            .get("ignoreMethodParameterNames")
            .map_or("", String::as_str);
        self.ignored_names = WildcardList::new(pattern);
        Ok(())
    }
}

impl AstRule for MethodParameterTypeRequiredRule {
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
            if param.type_name.is_some() || self.ignored_names.matches(&param.name) {
                continue;
            }
            sink.report(
                param.span,
                format!(
                    "Parameter \"{}\" of method \"{}\" is dynamically typed",
                    param.name, method.name
                ),
            );
        }
    }
}

/// A comma-separated list of `*` wildcard patterns.
struct WildcardList {
    patterns: Vec<String>,
}

impl WildcardList {
    fn new(pattern: &str) -> Self {
        let patterns = pattern
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect();
        Self { patterns }
    }

    fn matches(&self, name: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| wildcard_match(pattern, name))
    }
}

/// Matches `name` against a `*` wildcard pattern.
fn wildcard_match(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }

    let Some(first) = parts.first() else {
        return false;
    };
    if !name.starts_with(first) {
        return false;
    }
    let Some(last) = parts.last() else {
        return false;
    };
    if !name.ends_with(last) {
        return false;
    }

    let mut remaining = &name[first.len()..];
    for part in &parts[1..parts.len() - 1] {
        let Some(index) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[index + part.len()..];
    }
    true
}

/// Descriptors for convention rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "NoTabCharacter",
                "org.codenarc.rule.convention.NoTabCharacterRule",
                Category::Convention,
                Severity::Info,
                "Checks for tab characters in source files.",
            ),
            || RuleInstanceImpl::File(Box::new(NoTabCharacterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "LongLiteralWithLowerCaseL",
                "org.codenarc.rule.convention.LongLiteralWithLowerCaseLRule",
                Category::Convention,
                Severity::Warning,
                "Checks for long literals written with a lower case `l`.",
            ),
            || RuleInstanceImpl::Ast(Box::new(LongLiteralWithLowerCaseLRule)),
        ),
        descriptor(
            RuleMeta::new(
                "NoDef",
                "org.codenarc.rule.convention.NoDefRule",
                Category::Convention,
                Severity::Info,
                "Checks for `def` declarations.",
            ),
            || RuleInstanceImpl::Ast(Box::new(NoDefRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "VariableTypeRequired",
                "org.codenarc.rule.convention.VariableTypeRequiredRule",
                Category::Convention,
                Severity::Info,
                "Checks for variables without a declared type.",
            ),
            || RuleInstanceImpl::Ast(Box::new(VariableTypeRequiredRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "FieldTypeRequired",
                "org.codenarc.rule.convention.FieldTypeRequiredRule",
                Category::Convention,
                Severity::Info,
                "Checks for fields without a declared type.",
            ),
            || RuleInstanceImpl::Ast(Box::new(FieldTypeRequiredRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "MethodReturnTypeRequired",
                "org.codenarc.rule.convention.MethodReturnTypeRequiredRule",
                Category::Convention,
                Severity::Info,
                "Checks for methods without a declared return type.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MethodReturnTypeRequiredRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "MethodParameterTypeRequired",
                "org.codenarc.rule.convention.MethodParameterTypeRequired",
                Category::Convention,
                Severity::Info,
                "Checks for method parameters without a declared type.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MethodParameterTypeRequiredRule::default())),
        ),
    ]
}

/// Flags non-void methods that do not end in an explicit `return`.
#[derive(Default)]
pub struct ImplicitReturnStatementRule {
    classes: ClassStack,
}

impl Rule for ImplicitReturnStatementRule {}

impl AstRule for ImplicitReturnStatementRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Method).union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(_) => self.classes.enter(node),
            NodeRef::Method(method) => {
                let Some(return_type) = &method.return_type else {
                    return;
                };
                if return_type == "void" {
                    return;
                }
                let Some(body) = &method.body else {
                    return;
                };
                if ends_with_explicit_exit(&body.statements) {
                    return;
                }
                sink.report(
                    method.span,
                    format!(
                        "The method {} in class {} is missing an explicit return",
                        method.name,
                        self.classes.current()
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
            self.classes.exit(node);
        }
    }
}

/// Returns whether a block's statements end in something that exits the method.
fn ends_with_explicit_exit(statements: &[Stmt]) -> bool {
    let Some(last) = statements.last() else {
        return false;
    };
    match last {
        Stmt::Return(_)
        | Stmt::Throw(_)
        | Stmt::If(_)
        | Stmt::For(_)
        | Stmt::ForEach(_)
        | Stmt::While(_)
        | Stmt::DoWhile(_)
        | Stmt::Switch(_)
        | Stmt::Synchronized(_)
        | Stmt::Try(_) => true,
        Stmt::Block(block) => {
            block.statements.is_empty() || matches!(block.statements.last(), Some(Stmt::Try(_)))
        }
        _ => false,
    }
}

/// Flags assignments to a method or closure parameter.
#[derive(Default)]
pub struct ParameterReassignmentRule {
    classes: ClassStack,
    method_params: Vec<String>,
    closure_params: Vec<Vec<String>>,
}

impl Rule for ParameterReassignmentRule {}

impl ParameterReassignmentRule {
    fn is_parameter(&self, name: &str) -> bool {
        if self.method_params.iter().any(|param| param == name) {
            return true;
        }
        self.closure_params
            .iter()
            .any(|params| params.iter().any(|param| param == name))
    }
}

impl AstRule for ParameterReassignmentRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
            .union(NodeMask::of(NodeKind::Method))
            .union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(_) => self.classes.enter(node),
            NodeRef::Method(method) => {
                self.closure_params.clear();
                self.method_params = method
                    .params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect();
            }
            NodeRef::Expr(expr) => match &expr.kind {
                ExprKind::Closure { params, .. } => {
                    let names = params.iter().map(|param| param.name.clone()).collect();
                    self.closure_params.push(names);
                }
                ExprKind::Assign {
                    target,
                    op: AssignOp::Assign,
                    ..
                } => {
                    let ExprKind::Variable(name) = &target.kind else {
                        return;
                    };
                    if !self.is_parameter(name) {
                        return;
                    }
                    sink.report(
                        expr.span,
                        format!(
                            "The method parameter [{}] in class {} was reassigned. Use a temporary variable instead.",
                            name,
                            self.classes.current()
                        ),
                    );
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(_) => self.classes.exit(node),
            NodeRef::Method(_) => {
                self.method_params.clear();
                self.closure_params.clear();
            }
            NodeRef::Expr(expr) => {
                if matches!(expr.kind, ExprKind::Closure { .. }) {
                    self.closure_params.pop();
                }
            }
            _ => {}
        }
    }
}

/// Flags uses of `double`/`Double`, or of `float`/`Float`.
pub struct FloatingPointTypeRule {
    type_names: &'static [&'static str],
    short_name: &'static str,
    classes: ClassStack,
}

impl Rule for FloatingPointTypeRule {}

impl FloatingPointTypeRule {
    fn matches(&self, type_name: Option<&str>) -> bool {
        let Some(type_name) = type_name else {
            return false;
        };
        self.type_names.contains(&type_name)
    }
}

impl AstRule for FloatingPointTypeRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
            .union(NodeMask::of(NodeKind::Method))
            .union(NodeMask::of(NodeKind::VarDecl))
            .union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let name = self.short_name;
        match node {
            NodeRef::Class(_) => self.classes.enter(node),
            NodeRef::Field(field) => {
                if self.matches(field.type_name.as_deref()) {
                    sink.report(
                        field.span,
                        format!(
                            "The field {} in class {} is of type {name}. Prefer using BigDecimal.",
                            field.name,
                            self.classes.current()
                        ),
                    );
                }
            }
            NodeRef::Stmt(Stmt::VarDecl(decl)) => {
                if self.matches(decl.type_name.as_deref()) {
                    sink.report(
                        decl.span,
                        format!(
                            "The variable {} in class {} is of type {name}. Prefer using BigDecimal.",
                            decl.name,
                            self.classes.current()
                        ),
                    );
                }
            }
            NodeRef::Method(method) => {
                if self.matches(method.return_type.as_deref()) {
                    sink.report(
                        method.span,
                        format!(
                            "The method {} in class {} has a return type of {name}. Prefer using BigDecimal.",
                            method.name,
                            self.classes.current()
                        ),
                    );
                }
                for param in &method.params {
                    if !self.matches(param.type_name.as_deref()) {
                        continue;
                    }
                    sink.report(
                        param.span,
                        format!(
                            "The parameter named {} in method {} of class {} is of type {name}. Prefer using BigDecimal.",
                            param.name,
                            method.name,
                            self.classes.current()
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
            self.classes.exit(node);
        }
    }
}

/// Flags public methods declared after non-public methods.
pub struct PublicMethodsBeforeNonPublicMethodsRule;

impl Rule for PublicMethodsBeforeNonPublicMethodsRule {}

impl FileRule for PublicMethodsBeforeNonPublicMethodsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            check_public_method_order(item, sink);
        }
    }
}

fn check_public_method_order(item: &groovylint_syntax::ast::Item, sink: &mut DiagnosticSink<'_>) {
    if let groovylint_syntax::ast::Item::Class(class) = item {
        let mut saw_non_public = false;
        for member in &class.members {
            if let groovylint_syntax::ast::Item::Method(method) = member {
                let public = method.is_public || (!method.is_private && !method.is_protected);
                if public && saw_non_public {
                    sink.report(
                        method.span,
                        format!(
                            "The public method {} is declared after a non-public method",
                            method.name
                        ),
                    );
                } else if !public {
                    saw_non_public = true;
                }
            }
        }
        for member in &class.members {
            check_public_method_order(member, sink);
        }
    }
}

/// Flags static fields declared after instance fields.
pub struct StaticFieldsBeforeInstanceFieldsRule;

impl Rule for StaticFieldsBeforeInstanceFieldsRule {}

impl FileRule for StaticFieldsBeforeInstanceFieldsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            check_static_field_order(item, sink);
        }
    }
}

fn check_static_field_order(item: &groovylint_syntax::ast::Item, sink: &mut DiagnosticSink<'_>) {
    if let groovylint_syntax::ast::Item::Class(class) = item {
        let mut saw_instance = false;
        for member in &class.members {
            if let groovylint_syntax::ast::Item::Field(field) = member {
                if !field.is_static {
                    saw_instance = true;
                } else if saw_instance {
                    sink.report(
                        field.span,
                        format!(
                            "The static field {} is declared after an instance field",
                            field.name
                        ),
                    );
                }
            }
        }
        for member in &class.members {
            check_static_field_order(member, sink);
        }
    }
}

/// Flags static methods declared after instance methods of the same visibility.
pub struct StaticMethodsBeforeInstanceMethodsRule;

impl Rule for StaticMethodsBeforeInstanceMethodsRule {}

impl FileRule for StaticMethodsBeforeInstanceMethodsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            check_static_method_order(item, sink);
        }
    }
}

fn visibility(method: &MethodDecl) -> u8 {
    if method.is_private {
        0
    } else if method.is_protected {
        1
    } else {
        2
    }
}

fn check_static_method_order(item: &groovylint_syntax::ast::Item, sink: &mut DiagnosticSink<'_>) {
    if let groovylint_syntax::ast::Item::Class(class) = item {
        let mut saw_instance = [false; 3];
        for member in &class.members {
            if let groovylint_syntax::ast::Item::Method(method) = member {
                let level = visibility(method) as usize;
                if method.is_static {
                    if saw_instance[level] {
                        sink.report(
                            method.span,
                            format!(
                                "The static method {} is declared after an instance method",
                                method.name
                            ),
                        );
                    }
                } else {
                    saw_instance[level] = true;
                }
            }
        }
        for member in &class.members {
            check_static_method_order(member, sink);
        }
    }
}

/// Flags if statements whose branches are simple return/expression values.
pub struct IfStatementCouldBeTernaryRule;

impl Rule for IfStatementCouldBeTernaryRule {}

impl AstRule for IfStatementCouldBeTernaryRule {
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
        let Some(else_branch) = &if_stmt.else_branch else {
            return;
        };
        if branch_value(&if_stmt.then_branch).is_some() && branch_value(else_branch).is_some() {
            sink.report(
                if_stmt.span,
                "The if statement can be rewritten using the ternary operator",
            );
        }
    }
}

fn branch_value(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Return(ret) => ret.value.as_ref(),
        Stmt::Expr(expr) => Some(expr),
        Stmt::Block(block) if block.statements.len() == 1 => branch_value(&block.statements[0]),
        _ => None,
    }
}

/// Flags if/else-if chains that compare one value for equality.
pub struct CouldBeSwitchStatementRule;

impl Rule for CouldBeSwitchStatementRule {}

impl AstRule for CouldBeSwitchStatementRule {
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
        let Some(subject) = equality_subject(&if_stmt.condition, context.source.text()) else {
            return;
        };
        let mut count = 1;
        let mut current = if_stmt.else_branch.as_deref();
        while let Some(Stmt::If(next)) = current {
            if equality_subject(&next.condition, context.source.text()).as_deref()
                == Some(subject.as_str())
            {
                count += 1;
                current = next.else_branch.as_deref();
            } else {
                break;
            }
        }
        if count >= 2 {
            sink.report(
                if_stmt.span,
                "The if/else-if chain can be rewritten as a switch statement",
            );
        }
    }
}

fn equality_subject(expr: &Expr, source: &str) -> Option<String> {
    let ExprKind::Binary {
        op: groovylint_syntax::ast::BinaryOp::Eq,
        left,
        ..
    } = &expr.kind
    else {
        return None;
    };
    Some(crate::rules::util::expr_text(left, source))
}

/// Descriptors for the remaining convention rules.
#[must_use]
pub fn more_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "ImplicitReturnStatement",
                "org.codenarc.rule.convention.ImplicitReturnStatementRule",
                Category::Convention,
                Severity::Info,
                "Checks for methods without an explicit return statement.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ImplicitReturnStatementRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ParameterReassignment",
                "org.codenarc.rule.convention.ParameterReassignmentRule",
                Category::Convention,
                Severity::Info,
                "Checks for reassigned method parameters.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ParameterReassignmentRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "NoDouble",
                "org.codenarc.rule.convention.NoDoubleRule",
                Category::Convention,
                Severity::Warning,
                "Checks for double and Double types.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(FloatingPointTypeRule {
                    type_names: &["double", "Double", "java.lang.Double"],
                    short_name: "double/Double",
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "NoJavaUtilDate",
                "org.codenarc.rule.convention.NoJavaUtilDateRule",
                Category::Convention,
                Severity::Warning,
                "Checks for java.util.Date usage.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(FloatingPointTypeRule {
                    type_names: &["Date", "java.util.Date"],
                    short_name: "java.util.Date",
                    classes: ClassStack::default(),
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "NoFloat",
                "org.codenarc.rule.convention.NoFloatRule",
                Category::Convention,
                Severity::Warning,
                "Checks for float and Float types.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(FloatingPointTypeRule {
                    type_names: &["float", "Float", "java.lang.Float"],
                    short_name: "float/Float",
                    classes: ClassStack::default(),
                }))
            },
        ),
    ]
}

/// Returns whether an expression is a boolean negation.
fn is_negation(expr: &Expr) -> bool {
    use groovylint_syntax::ast::{BinaryOp, UnaryOp};
    matches!(
        &expr.kind,
        ExprKind::Unary {
            op: UnaryOp::Not,
            ..
        } | ExprKind::Binary {
            op: BinaryOp::NotEq,
            ..
        }
    )
}

/// Returns whether an expression compares against `null`.
fn is_null_comparison(expr: &Expr) -> bool {
    let ExprKind::Binary { left, right, .. } = &expr.kind else {
        return false;
    };
    is_null_literal(left) || is_null_literal(right)
}

fn is_null_literal(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Literal(Literal::Null))
}

/// Flags negated conditions inside ternary expressions.
pub struct ConfusingTernaryRule;

impl Rule for ConfusingTernaryRule {}

impl AstRule for ConfusingTernaryRule {
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
            is_elvis: _,
            condition,
            ..
        } = &expr.kind
        else {
            return;
        };
        if !is_negation(condition) {
            return;
        }
        if is_null_comparison(condition) {
            return;
        }
        let text = crate::rules::util::expr_text(condition, context.source.text());
        sink.report(
            expr.span,
            format!("({text}) is a confusing negation in a ternary expression"),
        );
    }
}

/// Returns whether an expression is a unary `!` negation.
fn is_unary_negation(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Unary {
            op: groovylint_syntax::ast::UnaryOp::Not,
            ..
        }
    )
}

/// Flags `if (!condition) ... else ...` statements.
pub struct InvertedIfElseRule;

impl Rule for InvertedIfElseRule {}

impl AstRule for InvertedIfElseRule {
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
        if if_stmt.else_branch.is_none() {
            return;
        }
        if !is_unary_negation(&if_stmt.condition) {
            return;
        }
        sink.report(
            if_stmt.span,
            "The if statement is inverted; rewrite it with the positive condition",
        );
    }
}

/// Flags uses of an obsolete collection type.
pub struct ObsoleteTypeRule {
    type_name: &'static str,
    replacement: &'static str,
}

impl Rule for ObsoleteTypeRule {}

impl ObsoleteTypeRule {
    fn matches(&self, type_name: &str) -> bool {
        type_name.rsplit('.').next() == Some(self.type_name)
    }
}

impl AstRule for ObsoleteTypeRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
            .union(NodeMask::of(NodeKind::Field))
            .union(NodeMask::of(NodeKind::VarDecl))
            .union(NodeMask::of(NodeKind::Method))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let message = format!(
            "The java.util.{} class is effectively obsolete. Use {} instead",
            self.type_name, self.replacement
        );
        match node {
            NodeRef::Expr(expr) => {
                if let ExprKind::New { type_name, .. } = &expr.kind
                    && self.matches(type_name)
                {
                    sink.report(expr.span, message);
                }
            }
            NodeRef::Field(field) => {
                if let Some(type_name) = &field.type_name
                    && self.matches(type_name)
                {
                    sink.report(field.span, message);
                }
            }
            NodeRef::Stmt(Stmt::VarDecl(decl)) => {
                if let Some(type_name) = &decl.type_name
                    && self.matches(type_name)
                {
                    sink.report(decl.span, message);
                }
            }
            NodeRef::Method(method) => {
                if let Some(type_name) = &method.return_type
                    && self.matches(type_name)
                {
                    sink.report(method.span, message.clone());
                }
                for param in &method.params {
                    if let Some(type_name) = &param.type_name
                        && self.matches(type_name)
                    {
                        sink.report(param.span, message.clone());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Descriptors for the remaining convention rules.
#[must_use]
pub fn extra_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "ConfusingTernary",
                "org.codenarc.rule.convention.ConfusingTernaryRule",
                Category::Convention,
                Severity::Warning,
                "Checks for negated conditions in ternary expressions.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ConfusingTernaryRule)),
        ),
        descriptor(
            RuleMeta::new(
                "InvertedIfElse",
                "org.codenarc.rule.convention.InvertedIfElseRule",
                Category::Convention,
                Severity::Warning,
                "Checks for inverted if-else statements.",
            ),
            || RuleInstanceImpl::Ast(Box::new(InvertedIfElseRule)),
        ),
        descriptor(
            RuleMeta::new(
                "HashtableIsObsolete",
                "org.codenarc.rule.convention.HashtableIsObsoleteRule",
                Category::Convention,
                Severity::Warning,
                "Checks for java.util.Hashtable usage.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(ObsoleteTypeRule {
                    type_name: "Hashtable",
                    replacement: "HashMap or ConcurrentHashMap",
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "VectorIsObsolete",
                "org.codenarc.rule.convention.VectorIsObsoleteRule",
                Category::Convention,
                Severity::Warning,
                "Checks for java.util.Vector usage.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(ObsoleteTypeRule {
                    type_name: "Vector",
                    replacement: "ArrayList or Collections.synchronizedList()",
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "CouldBeSwitchStatement",
                "org.codenarc.rule.convention.CouldBeSwitchStatementRule",
                Category::Convention,
                Severity::Warning,
                "If/else-if chains comparing one value can be a switch.",
            ),
            || RuleInstanceImpl::Ast(Box::new(CouldBeSwitchStatementRule)),
        ),
        descriptor(
            RuleMeta::new(
                "IfStatementCouldBeTernary",
                "org.codenarc.rule.convention.IfStatementCouldBeTernaryRule",
                Category::Convention,
                Severity::Warning,
                "If statements returning values can be ternaries.",
            ),
            || RuleInstanceImpl::Ast(Box::new(IfStatementCouldBeTernaryRule)),
        ),
        descriptor(
            RuleMeta::new(
                "PublicMethodsBeforeNonPublicMethods",
                "org.codenarc.rule.convention.PublicMethodsBeforeNonPublicMethodsRule",
                Category::Convention,
                Severity::Info,
                "Public methods should appear before non-public methods.",
            ),
            || RuleInstanceImpl::File(Box::new(PublicMethodsBeforeNonPublicMethodsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticFieldsBeforeInstanceFields",
                "org.codenarc.rule.convention.StaticFieldsBeforeInstanceFieldsRule",
                Category::Convention,
                Severity::Info,
                "Static fields should appear before instance fields.",
            ),
            || RuleInstanceImpl::File(Box::new(StaticFieldsBeforeInstanceFieldsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "StaticMethodsBeforeInstanceMethods",
                "org.codenarc.rule.convention.StaticMethodsBeforeInstanceMethodsRule",
                Category::Convention,
                Severity::Info,
                "Static methods should appear before instance methods.",
            ),
            || RuleInstanceImpl::File(Box::new(StaticMethodsBeforeInstanceMethodsRule)),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{WildcardList, wildcard_match};

    #[test]
    fn wildcard_matching_handles_prefix_suffix_and_infix() {
        assert!(wildcard_match("id", "id"));
        assert!(!wildcard_match("id", "id2"));
        assert!(wildcard_match("*Id", "policyId"));
        assert!(!wildcard_match("*Id", "policyId2"));
        assert!(wildcard_match("get*By*", "getPolicyById"));
        assert!(wildcard_match("*", "anything"));
    }

    #[test]
    fn wildcard_list_parses_several_patterns() {
        let list = WildcardList::new("id, *Lock, temp");
        assert!(list.matches("id"));
        assert!(list.matches("writeLock"));
        assert!(!list.matches("name"));
        assert!(!WildcardList::new("").matches("name"));
    }
}
