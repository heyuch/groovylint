//! `CodeNarc` rules for idiomatic Groovy code.

use groovylint_core::Severity;
use groovylint_syntax::ast::{Block, ClassDecl, Expr, ExprKind, Item, Literal, MethodDecl, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};
use crate::rules::util::{simple_name, unparen, variable_name};

fn is_false(expr: &Expr) -> bool {
    matches!(
        &unparen(expr).kind,
        ExprKind::Literal(Literal::Boolean(false))
    )
}

fn is_closure(expr: &Expr) -> bool {
    matches!(&unparen(expr).kind, ExprKind::Closure { .. })
}

fn call_parts(expr: &Expr) -> Option<(&Expr, &str, &[Expr])> {
    let ExprKind::Call {
        receiver: Some(receiver),
        name,
        args,
        ..
    } = &unparen(expr).kind
    else {
        return None;
    };
    Some((receiver, name, args))
}

fn is_call(expr: &Expr, method: &str, arity: usize) -> bool {
    matches!(
        &unparen(expr).kind,
        ExprKind::Call {
            receiver: Some(_),
            name,
            args,
            ..
        } if name == method && args.len() == arity
    )
}

fn assigned_value(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::VarDecl(decl) => decl.init.as_ref(),
        Stmt::Expr(expr) => match &expr.kind {
            ExprKind::Assign { value, .. } => Some(value),
            _ => None,
        },
        _ => None,
    }
}

fn method_is_getter(method: &MethodDecl) -> bool {
    let name = method.name.as_str();
    name.starts_with("get")
        && name.len() > 3
        && name.as_bytes()[3].is_ascii_uppercase()
        && method.params.is_empty()
}

fn single_return(block: &Block) -> Option<&Expr> {
    let statements = &block.statements;
    if statements.len() != 1 {
        return None;
    }
    match &statements[0] {
        Stmt::Return(ret) => ret.value.as_ref(),
        Stmt::Expr(expr) => Some(expr),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Explicit method-call rules

macro_rules! explicit_call_rule {
    ($struct:ident, $name:literal, $method:literal, $message:literal) => {
        /// Explicit operator-method call rule.
        pub struct $struct;

        impl Rule for $struct {}

        impl AstRule for $struct {
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
                let Some((receiver, name, args)) = call_parts(expr) else {
                    return;
                };
                if name != $method || args.len() != 1 {
                    return;
                }
                let receiver = crate::rules::util::expr_text(receiver, _context.source.text());
                sink.report(
                    expr.span,
                    format!(
                        "Explicit call to {}.{}(...) can be rewritten as {}; {}",
                        receiver, $method, $message, "use the corresponding Groovy operator"
                    ),
                );
            }
        }
    };
}

explicit_call_rule!(
    ExplicitCallToAndMethodRule,
    "ExplicitCallToAndMethod",
    "and",
    "&"
);
explicit_call_rule!(
    ExplicitCallToCompareToMethodRule,
    "ExplicitCallToCompareToMethod",
    "compareTo",
    "<=>"
);
explicit_call_rule!(
    ExplicitCallToDivMethodRule,
    "ExplicitCallToDivMethod",
    "div",
    "/"
);
explicit_call_rule!(
    ExplicitCallToEqualsMethodRule,
    "ExplicitCallToEqualsMethod",
    "equals",
    "=="
);
explicit_call_rule!(
    ExplicitCallToGetAtMethodRule,
    "ExplicitCallToGetAtMethod",
    "getAt",
    "[]"
);
explicit_call_rule!(
    ExplicitCallToLeftShiftMethodRule,
    "ExplicitCallToLeftShiftMethod",
    "leftShift",
    "<<"
);
explicit_call_rule!(
    ExplicitCallToMinusMethodRule,
    "ExplicitCallToMinusMethod",
    "minus",
    "-"
);
explicit_call_rule!(
    ExplicitCallToModMethodRule,
    "ExplicitCallToModMethod",
    "mod",
    "%"
);
explicit_call_rule!(
    ExplicitCallToMultiplyMethodRule,
    "ExplicitCallToMultiplyMethod",
    "multiply",
    "*"
);
explicit_call_rule!(
    ExplicitCallToOrMethodRule,
    "ExplicitCallToOrMethod",
    "or",
    "|"
);
explicit_call_rule!(
    ExplicitCallToPlusMethodRule,
    "ExplicitCallToPlusMethod",
    "plus",
    "+"
);
explicit_call_rule!(
    ExplicitCallToPowerMethodRule,
    "ExplicitCallToPowerMethod",
    "power",
    "**"
);
explicit_call_rule!(
    ExplicitCallToPutAtMethodRule,
    "ExplicitCallToPutAtMethod",
    "putAt",
    "[] ="
);
explicit_call_rule!(
    ExplicitCallToRightShiftMethodRule,
    "ExplicitCallToRightShiftMethod",
    "rightShift",
    ">>"
);
explicit_call_rule!(
    ExplicitCallToXorMethodRule,
    "ExplicitCallToXorMethod",
    "xor",
    "^"
);

// ---------------------------------------------------------------------------
// Explicit collection construction

macro_rules! explicit_type_rule {
    ($struct:ident, $name:literal, $type_name:literal, $replacement:literal) => {
        /// Explicit collection type construction rule.
        pub struct $struct;

        impl Rule for $struct {}

        impl AstRule for $struct {
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
                if !args.is_empty() || simple_name(type_name) != $type_name {
                    return;
                }
                sink.report(
                    expr.span,
                    format!(
                        "{} objects are normally instantiated using the form {}",
                        $type_name, $replacement
                    ),
                );
            }
        }
    };
}

explicit_type_rule!(
    ExplicitArrayListInstantiationRule,
    "ExplicitArrayListInstantiation",
    "ArrayList",
    "[]"
);
explicit_type_rule!(
    ExplicitHashMapInstantiationRule,
    "ExplicitHashMapInstantiation",
    "HashMap",
    "[:]"
);
explicit_type_rule!(
    ExplicitHashSetInstantiationRule,
    "ExplicitHashSetInstantiation",
    "HashSet",
    "[]"
);
explicit_type_rule!(
    ExplicitLinkedHashMapInstantiationRule,
    "ExplicitLinkedHashMapInstantiation",
    "LinkedHashMap",
    "[:]"
);
explicit_type_rule!(
    ExplicitLinkedListInstantiationRule,
    "ExplicitLinkedListInstantiation",
    "LinkedList",
    "[]"
);
explicit_type_rule!(
    ExplicitStackInstantiationRule,
    "ExplicitStackInstantiation",
    "Stack",
    "[]"
);
explicit_type_rule!(
    ExplicitTreeSetInstantiationRule,
    "ExplicitTreeSetInstantiation",
    "TreeSet",
    "[]"
);

// ---------------------------------------------------------------------------
// Assignment and closure rules

/// Flags assigning the result of a mutating `sort()` call.
pub struct AssignCollectionSortRule;

impl Rule for AssignCollectionSortRule {}

impl AstRule for AssignCollectionSortRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::VarDecl)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(stmt) = node else {
            return;
        };
        let Some(value) = assigned_value(stmt) else {
            return;
        };
        let Some((receiver, name, args)) = call_parts(value) else {
            return;
        };
        if name == "sort"
            && variable_name(receiver).is_some()
            && !matches!(args.first(), Some(arg) if is_false(arg))
        {
            sink.report(
                stmt.span(),
                "sort() mutates the original list; do not assign its return value",
            );
        }
    }
}

/// Flags assigning the result of a mutating `unique()` call.
pub struct AssignCollectionUniqueRule;

impl Rule for AssignCollectionUniqueRule {}

impl AstRule for AssignCollectionUniqueRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::VarDecl)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(stmt) = node else {
            return;
        };
        let Some(value) = assigned_value(stmt) else {
            return;
        };
        let Some((receiver, name, args)) = call_parts(value) else {
            return;
        };
        if name == "unique"
            && variable_name(receiver).is_some()
            && !matches!(args.first(), Some(arg) if is_false(arg))
        {
            sink.report(
                stmt.span(),
                "unique() mutates the original list; do not assign its return value",
            );
        }
    }
}

/// Flags a closure passed inside parentheses as the last argument.
pub struct ClosureAsLastMethodParameterRule;

impl Rule for ClosureAsLastMethodParameterRule {}

impl AstRule for ClosureAsLastMethodParameterRule {
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
        let Some(last) = args.last() else {
            return;
        };
        if !is_closure(last) {
            return;
        }
        let text = context.source.text();
        let before = text[..last.span.start].trim_end();
        let after = text[last.span.end..].trim_start();
        if before.ends_with('(') && after.starts_with(')') {
            sink.report(expr.span, format!("The last parameter to '{name}' is a closure and can appear outside the parentheses"));
        }
    }
}

/// Flags calls to deprecated `collectAll`.
pub struct CollectAllIsDeprecatedRule;

impl Rule for CollectAllIsDeprecatedRule {}

impl AstRule for CollectAllIsDeprecatedRule {
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
        if is_call(expr, "collectAll", 1) || is_call(expr, "collectAll", 2) {
            sink.report(
                expr.span,
                "collectAll{} is deprecated; use collectNested instead",
            );
        }
    }
}

/// Flags chained `collect { ... }.flatten()`.
pub struct UseCollectManyRule;

impl Rule for UseCollectManyRule {}

impl AstRule for UseCollectManyRule {
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
        let Some((receiver, name, args)) = call_parts(expr) else {
            return;
        };
        if name != "flatten" || !args.is_empty() {
            return;
        }
        if is_call(receiver, "collect", 1) {
            sink.report(
                expr.span,
                "collect{}.flatten() can be rewritten as collectMany{}",
            );
        }
    }
}

/// Flags nested collect calls on the outer closure parameter.
pub struct UseCollectNestedRule;

impl Rule for UseCollectNestedRule {}

impl AstRule for UseCollectNestedRule {
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
        let Some((receiver, name, args)) = call_parts(expr) else {
            return;
        };
        if name != "collect" || args.len() != 1 {
            return;
        }
        let ExprKind::Closure { params, body, .. } = &unparen(receiver).kind else {
            return;
        };
        let param = params.first().map_or("it", |param| param.name.as_str());
        if closure_uses_collect(body, param) {
            sink.report(
                expr.span,
                "Instead of nested collect{}-calls use collectNested{}",
            );
        }
    }
}

fn closure_uses_collect(block: &Block, param: &str) -> bool {
    block.statements.iter().any(|statement| {
        let mut finder = CollectOnParam {
            param,
            found: false,
        };
        crate::traverse::walk_stmt(statement, &mut finder);
        finder.found
    })
}

struct CollectOnParam<'a> {
    param: &'a str,
    found: bool,
}

impl crate::traverse::AstVisitor for CollectOnParam<'_> {
    fn enter(&mut self, node: NodeRef<'_>) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let Some((receiver, name, args)) = call_parts(expr) else {
            return;
        };
        if name == "collect" && args.len() == 1 && variable_name(receiver) == Some(self.param) {
            self.found = true;
        }
    }
    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags `GString` keys in map literals.
pub struct GStringAsMapKeyRule;

impl Rule for GStringAsMapKeyRule {}

impl AstRule for GStringAsMapKeyRule {
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
        let ExprKind::Map(entries) = &expr.kind else {
            return;
        };
        for (key, _) in entries {
            if let ExprKind::Literal(Literal::String(text)) = &unparen(key).kind
                && text.contains('$')
            {
                sink.report(key.span, "GString as a key in a map is unsafe");
            }
        }
    }
}

/// Flags `${...}` inside a normal quoted string.
pub struct GStringExpressionWithinStringRule;

impl Rule for GStringExpressionWithinStringRule {}

impl AstRule for GStringExpressionWithinStringRule {
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
        let ExprKind::Literal(Literal::String(raw)) = &expr.kind else {
            return;
        };
        let is_single_quoted = raw.starts_with('\'') && !raw.starts_with("\"\"\"");
        if is_single_quoted && raw.contains("${") {
            sink.report(expr.span, "The string contains a GString-type expression");
        }
    }
}

/// Flags getter-shaped methods that return a constant.
pub struct GetterMethodCouldBePropertyRule;

impl Rule for GetterMethodCouldBePropertyRule {}

impl AstRule for GetterMethodCouldBePropertyRule {
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
        if !method_is_getter(method) || method.is_static {
            return;
        }
        let Some(body) = &method.body else {
            return;
        };
        let Some(value) = single_return(body) else {
            return;
        };
        if is_literalish(value) {
            let property = getter_property(&method.name);
            sink.report(
                method.span,
                format!(
                    "The method '{}' can be expressed as property '{}'{}",
                    method.name, property, ""
                ),
            );
        }
        let _ = context;
    }
}

fn is_literalish(expr: &Expr) -> bool {
    match &unparen(expr).kind {
        ExprKind::Literal(_) | ExprKind::Property { .. } => true,
        ExprKind::Variable(name) => name.chars().next().is_some_and(char::is_uppercase),
        _ => false,
    }
}

fn getter_property(name: &str) -> String {
    let tail = &name[3..];
    let mut chars = tail.chars();
    chars.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_ascii_lowercase(), chars.as_str())
    })
}

/// Flags deprecated `groovy.lang.Immutable`.
pub struct GroovyLangImmutableRule;

impl Rule for GroovyLangImmutableRule {}

impl AstRule for GroovyLangImmutableRule {
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
        let import_text = context
            .script
            .imports
            .iter()
            .map(|import| import.path.as_str())
            .collect::<Vec<_>>();
        let transform_imported = import_text.contains(&"groovy.transform.Immutable")
            || import_text.contains(&"groovy.transform.*")
            || import_text.contains(&"javax.annotation.concurrent.Immutable");
        for annotation in &class.annotations {
            if annotation == "groovy.lang.Immutable"
                || (annotation == "Immutable" && !transform_imported)
                || import_text.iter().any(|path| {
                    path.starts_with("groovy.lang.Immutable")
                        && annotation.ends_with(path.split('.').next_back().unwrap_or(""))
                })
            {
                sink.report(
                    class.span,
                    "groovy.lang.Immutable is deprecated; use groovy.transform.Immutable",
                );
                return;
            }
        }
    }
}

/// Flags repeated number literals.
pub struct DuplicateNumberLiteralRule {
    seen: std::collections::BTreeSet<String>,
    ignored: Vec<String>,
    minimum: Option<f64>,
}

impl Default for DuplicateNumberLiteralRule {
    fn default() -> Self {
        Self {
            seen: std::collections::BTreeSet::new(),
            ignored: vec!["0".to_string(), "1".to_string()],
            minimum: None,
        }
    }
}

impl Rule for DuplicateNumberLiteralRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("ignoreNumbers") {
            self.ignored = value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        if let Some(value) = properties.get("duplicateNumberMinimumValue") {
            self.minimum = Some(
                value
                    .trim()
                    .parse()
                    .map_err(|_| "duplicateNumberMinimumValue must be a number".to_string())?,
            );
        }
        Ok(())
    }
}

impl AstRule for DuplicateNumberLiteralRule {
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
        let ExprKind::Literal(Literal::Number(value)) = &expr.kind else {
            return;
        };
        let value = value.trim_end_matches(['d', 'D', 'f', 'F', 'g', 'G', 'l', 'L']);
        if self.ignored.iter().any(|ignored| ignored == value) {
            return;
        }
        if let Some(minimum) = self.minimum
            && value.parse::<f64>().is_ok_and(|number| number < minimum)
        {
            return;
        }
        if !self.seen.insert(value.to_string()) {
            sink.report(
                expr.span,
                format!("The number literal {value} is duplicated"),
            );
        }
    }
}

/// Flags repeated string literals.
pub struct DuplicateStringLiteralRule {
    seen: std::collections::BTreeSet<String>,
    ignored: Vec<String>,
    minimum_length: Option<usize>,
}

impl Default for DuplicateStringLiteralRule {
    fn default() -> Self {
        Self {
            seen: std::collections::BTreeSet::new(),
            ignored: vec![String::new()],
            minimum_length: None,
        }
    }
}

impl Rule for DuplicateStringLiteralRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("ignoreStrings") {
            let delimiter = properties
                .get("ignoreStringsDelimiter")
                .and_then(|delimiter| delimiter.chars().next())
                .unwrap_or(',');
            self.ignored = value
                .split(delimiter)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        if let Some(value) = properties.get("duplicateStringMinimumLength") {
            self.minimum_length = Some(
                value
                    .trim()
                    .parse()
                    .map_err(|_| "duplicateStringMinimumLength must be a number".to_string())?,
            );
        }
        Ok(())
    }
}

impl AstRule for DuplicateStringLiteralRule {
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
        let ExprKind::Literal(Literal::String(value)) = &expr.kind else {
            return;
        };
        let value = value.trim_matches('"').trim_matches('\'');
        if self.ignored.iter().any(|ignored| ignored == value) {
            return;
        }
        if let Some(minimum) = self.minimum_length
            && value.chars().count() < minimum
        {
            return;
        }
        if !self.seen.insert(value.to_string()) {
            sink.report(
                expr.span,
                format!("The string literal \"{value}\" is duplicated"),
            );
        }
    }
}

/// Flags comma-separated declarations where only the last name gets a value.
pub struct ConfusingMultipleReturnsRule;

impl Rule for ConfusingMultipleReturnsRule {}

impl FileRule for ConfusingMultipleReturnsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            check_declaration_group(item, sink);
        }
    }
}

fn check_declaration_group(item: &Item, sink: &mut DiagnosticSink<'_>) {
    match item {
        Item::Group(items) | Item::Class(ClassDecl { members: items, .. }) => {
            if matches!(item, Item::Group(_)) {
                check_group(items, sink);
            }
            for member in items {
                check_declaration_group(member, sink);
            }
        }
        Item::Initializer(initializer) => {
            for statement in &initializer.body.statements {
                check_group_statements(statement, sink);
            }
        }
        _ => {}
    }
}

fn check_group(items: &[Item], sink: &mut DiagnosticSink<'_>) {
    if items.len() < 2 {
        return;
    }
    let (_, last_span, last_init) = declaration_parts(&items[items.len() - 1]);
    if last_init.is_none() {
        return;
    }
    for item in &items[..items.len() - 1] {
        if let (name, span, None) = declaration_parts(item) {
            sink.report(
                span,
                format!(
                    "The declaration for {name} is initialized to null before the last initializer"
                ),
            );
        }
    }
    let _ = last_span;
}

fn declaration_parts(item: &Item) -> (String, groovylint_core::Span, Option<&Expr>) {
    match item {
        Item::Field(field) => (field.name.clone(), field.span, field.init.as_ref()),
        Item::Statement(Stmt::VarDecl(decl)) => (decl.name.clone(), decl.span, decl.init.as_ref()),
        _ => (String::new(), groovylint_core::Span::empty(0), None),
    }
}

fn check_group_statements(statement: &Stmt, sink: &mut DiagnosticSink<'_>) {
    if let Stmt::Block(block) = statement {
        let declarations: Vec<&Stmt> = block
            .statements
            .iter()
            .filter(|statement| matches!(statement, Stmt::VarDecl(_)))
            .collect();
        if declarations.len() > 1
            && declarations
                .last()
                .is_some_and(|last| matches!(last, Stmt::VarDecl(decl) if decl.init.is_some()))
        {
            for declaration in &declarations[..declarations.len() - 1] {
                if let Stmt::VarDecl(decl) = declaration
                    && decl.init.is_none()
                {
                    sink.report(
                        decl.span,
                        format!("The declaration for {} is initialized to null before the last initializer", decl.name),
                    );
                }
            }
        }
        for statement in &block.statements {
            check_group_statements(statement, sink);
        }
    }
}

// ---------------------------------------------------------------------------
// Descriptors

macro_rules! d {
    ($vec:ident, $name:literal, $class:literal, $rule:ident, $desc:literal) => {
        $vec.push(descriptor(
            RuleMeta::new($name, $class, Category::Groovyism, Severity::Info, $desc),
            || RuleInstanceImpl::Ast(Box::new($rule)),
        ));
    };
}

/// Descriptors for Groovy idiom rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    let mut vec = Vec::new();
    d!(
        vec,
        "AssignCollectionSort",
        "org.codenarc.rule.groovyism.AssignCollectionSortRule",
        AssignCollectionSortRule,
        "sort() mutates lists."
    );
    d!(
        vec,
        "AssignCollectionUnique",
        "org.codenarc.rule.groovyism.AssignCollectionUniqueRule",
        AssignCollectionUniqueRule,
        "unique() mutates lists."
    );
    d!(
        vec,
        "ClosureAsLastMethodParameter",
        "org.codenarc.rule.groovyism.ClosureAsLastMethodParameterRule",
        ClosureAsLastMethodParameterRule,
        "Move trailing closures outside parentheses."
    );
    d!(
        vec,
        "CollectAllIsDeprecated",
        "org.codenarc.rule.groovyism.CollectAllIsDeprecatedRule",
        CollectAllIsDeprecatedRule,
        "collectAll is deprecated."
    );
    d!(
        vec,
        "ExplicitArrayListInstantiation",
        "org.codenarc.rule.groovyism.ExplicitArrayListInstantiationRule",
        ExplicitArrayListInstantiationRule,
        "Use [] for ArrayList."
    );
    d!(
        vec,
        "ExplicitCallToAndMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToAndMethodRule",
        ExplicitCallToAndMethodRule,
        "Use & instead of and()."
    );
    d!(
        vec,
        "ExplicitCallToCompareToMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToCompareToMethodRule",
        ExplicitCallToCompareToMethodRule,
        "Use comparison operators."
    );
    d!(
        vec,
        "ExplicitCallToDivMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToDivMethodRule",
        ExplicitCallToDivMethodRule,
        "Use / instead of div()."
    );
    d!(
        vec,
        "ExplicitCallToEqualsMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToEqualsMethodRule",
        ExplicitCallToEqualsMethodRule,
        "Use == instead of equals()."
    );
    d!(
        vec,
        "ExplicitCallToGetAtMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToGetAtMethodRule",
        ExplicitCallToGetAtMethodRule,
        "Use [] instead of getAt()."
    );
    d!(
        vec,
        "ExplicitCallToLeftShiftMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToLeftShiftMethodRule",
        ExplicitCallToLeftShiftMethodRule,
        "Use << instead of leftShift()."
    );
    d!(
        vec,
        "ExplicitCallToMinusMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToMinusMethodRule",
        ExplicitCallToMinusMethodRule,
        "Use - instead of minus()."
    );
    d!(
        vec,
        "ExplicitCallToModMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToModMethodRule",
        ExplicitCallToModMethodRule,
        "Use % instead of mod()."
    );
    d!(
        vec,
        "ExplicitCallToMultiplyMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToMultiplyMethodRule",
        ExplicitCallToMultiplyMethodRule,
        "Use * instead of multiply()."
    );
    d!(
        vec,
        "ExplicitCallToOrMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToOrMethodRule",
        ExplicitCallToOrMethodRule,
        "Use | instead of or()."
    );
    d!(
        vec,
        "ExplicitCallToPlusMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToPlusMethodRule",
        ExplicitCallToPlusMethodRule,
        "Use + instead of plus()."
    );
    d!(
        vec,
        "ExplicitCallToPowerMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToPowerMethodRule",
        ExplicitCallToPowerMethodRule,
        "Use ** instead of power()."
    );
    d!(
        vec,
        "ExplicitCallToPutAtMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToPutAtMethodRule",
        ExplicitCallToPutAtMethodRule,
        "Use [] = instead of putAt()."
    );
    d!(
        vec,
        "ExplicitCallToRightShiftMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToRightShiftMethodRule",
        ExplicitCallToRightShiftMethodRule,
        "Use >> instead of rightShift()."
    );
    d!(
        vec,
        "ExplicitCallToXorMethod",
        "org.codenarc.rule.groovyism.ExplicitCallToXorMethodRule",
        ExplicitCallToXorMethodRule,
        "Use ^ instead of xor()."
    );
    d!(
        vec,
        "ExplicitHashMapInstantiation",
        "org.codenarc.rule.groovyism.ExplicitHashMapInstantiationRule",
        ExplicitHashMapInstantiationRule,
        "Use [:] for HashMap."
    );
    d!(
        vec,
        "ExplicitHashSetInstantiation",
        "org.codenarc.rule.groovyism.ExplicitHashSetInstantiationRule",
        ExplicitHashSetInstantiationRule,
        "Use a set literal."
    );
    d!(
        vec,
        "ExplicitLinkedHashMapInstantiation",
        "org.codenarc.rule.groovyism.ExplicitLinkedHashMapInstantiationRule",
        ExplicitLinkedHashMapInstantiationRule,
        "Use [:] for LinkedHashMap."
    );
    d!(
        vec,
        "ExplicitLinkedListInstantiation",
        "org.codenarc.rule.groovyism.ExplicitLinkedListInstantiationRule",
        ExplicitLinkedListInstantiationRule,
        "Use [] for LinkedList."
    );
    d!(
        vec,
        "ExplicitStackInstantiation",
        "org.codenarc.rule.groovyism.ExplicitStackInstantiationRule",
        ExplicitStackInstantiationRule,
        "Use [] for Stack."
    );
    d!(
        vec,
        "ExplicitTreeSetInstantiation",
        "org.codenarc.rule.groovyism.ExplicitTreeSetInstantiationRule",
        ExplicitTreeSetInstantiationRule,
        "Use [] for TreeSet."
    );
    d!(
        vec,
        "GStringAsMapKey",
        "org.codenarc.rule.groovyism.GStringAsMapKeyRule",
        GStringAsMapKeyRule,
        "GString map keys are unsafe."
    );
    d!(
        vec,
        "GStringExpressionWithinString",
        "org.codenarc.rule.groovyism.GStringExpressionWithinStringRule",
        GStringExpressionWithinStringRule,
        "Single-quoted strings cannot interpolate."
    );
    d!(
        vec,
        "GetterMethodCouldBeProperty",
        "org.codenarc.rule.groovyism.GetterMethodCouldBePropertyRule",
        GetterMethodCouldBePropertyRule,
        "Getter methods returning constants can be properties."
    );
    d!(
        vec,
        "GroovyLangImmutable",
        "org.codenarc.rule.groovyism.GroovyLangImmutableRule",
        GroovyLangImmutableRule,
        "groovy.lang.Immutable is deprecated."
    );
    d!(
        vec,
        "UseCollectMany",
        "org.codenarc.rule.groovyism.UseCollectManyRule",
        UseCollectManyRule,
        "collect{}.flatten() can be collectMany{}."
    );
    d!(
        vec,
        "UseCollectNested",
        "org.codenarc.rule.groovyism.UseCollectNestedRule",
        UseCollectNestedRule,
        "Use collectNested for nested collect calls."
    );
    vec.push(descriptor(
        RuleMeta::new(
            "ConfusingMultipleReturns",
            "org.codenarc.rule.groovyism.ConfusingMultipleReturnsRule",
            Category::Groovyism,
            Severity::Warning,
            "Multiple comma-separated declarations only initialize the last value.",
        ),
        || RuleInstanceImpl::File(Box::new(ConfusingMultipleReturnsRule)),
    ));
    vec.push(descriptor(
        RuleMeta::new(
            "DuplicateNumberLiteral",
            "org.codenarc.rule.dry.DuplicateNumberLiteralRule",
            Category::Dry,
            Severity::Info,
            "Duplicate numeric literals.",
        ),
        || RuleInstanceImpl::Ast(Box::new(DuplicateNumberLiteralRule::default())),
    ));
    vec.push(descriptor(
        RuleMeta::new(
            "DuplicateStringLiteral",
            "org.codenarc.rule.dry.DuplicateStringLiteralRule",
            Category::Dry,
            Severity::Info,
            "Duplicate string literals.",
        ),
        || RuleInstanceImpl::Ast(Box::new(DuplicateStringLiteralRule::default())),
    ));
    vec
}
