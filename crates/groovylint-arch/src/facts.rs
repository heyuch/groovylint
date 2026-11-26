//! Compact per-file facts used by the architecture engine.

use std::path::{Path, PathBuf};

use groovylint_core::Span;
use groovylint_syntax::ast::{
    Annotation, Block, ClassDecl, ClassKind as AstClassKind, Expr, ExprKind, Item, MethodDecl,
    Script, Stmt,
};

/// One import declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportFact {
    pub path: String,
    pub alias: Option<String>,
    pub is_static: bool,
    pub span: Span,
}

/// One annotation on a declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationFact {
    pub name: String,
    pub raw_arguments: Option<String>,
}

impl AnnotationFact {
    fn from_ast(annotation: &Annotation) -> Self {
        Self {
            name: annotation.name.clone(),
            raw_arguments: annotation.raw_arguments.clone(),
        }
    }
}

/// A field declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldFact {
    pub owner: String,
    pub name: String,
    pub type_name: Option<String>,
    pub annotations: Vec<AnnotationFact>,
    pub modifiers: Vec<String>,
    pub span: Span,
}

/// A method or constructor declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodFact {
    pub owner: String,
    pub name: String,
    pub return_type: Option<String>,
    pub annotations: Vec<AnnotationFact>,
    pub modifiers: Vec<String>,
    pub parameters: Vec<ParameterFact>,
    pub span: Span,
}

impl MethodFact {
    #[must_use]
    pub fn id(&self) -> String {
        format!("{}#{}/{}", self.owner, self.name, self.parameters.len())
    }
}

/// One method parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterFact {
    pub name: String,
    pub type_name: Option<String>,
    pub annotations: Vec<AnnotationFact>,
}

/// A class-like declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassFact {
    pub fqn: String,
    pub name: String,
    pub package: Option<String>,
    pub module: String,
    pub kind: AstClassKind,
    pub is_abstract: bool,
    pub is_nested: bool,
    pub annotations: Vec<AnnotationFact>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub methods: Vec<MethodFact>,
    pub fields: Vec<FieldFact>,
    pub span: Span,
}

/// A written type reference from a class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceFact {
    pub from: String,
    pub to: String,
    pub kind: ReferenceKind,
    pub span: Span,
}

/// Why a type name appeared in a class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceKind {
    Extends,
    Implements,
    FieldType,
    ReturnType,
    ParamType,
    LocalType,
    New,
    Cast,
    Annotation,
    Import,
}

/// Facts extracted from one file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileFacts {
    pub path: PathBuf,
    pub module: String,
    pub package: Option<String>,
    pub imports: Vec<ImportFact>,
    pub classes: Vec<ClassFact>,
    pub references: Vec<ReferenceFact>,
}

/// Extract facts from a parsed script.
#[must_use]
pub fn collect(path: &Path, module: &str, script: &Script) -> FileFacts {
    let package = script.package.as_ref().map(|package| package.name.clone());
    let imports = script
        .imports
        .iter()
        .map(|import| ImportFact {
            path: import.path.clone(),
            alias: import.alias.clone(),
            is_static: import.is_static,
            span: import.span,
        })
        .collect();
    let mut facts = FileFacts {
        path: path.to_path_buf(),
        module: module.to_string(),
        package: package.clone(),
        imports,
        classes: Vec::new(),
        references: Vec::new(),
    };
    for item in &script.items {
        collect_item(item, module, package.as_deref(), None, &mut facts);
    }
    facts
}

fn collect_item(
    item: &Item,
    module: &str,
    package: Option<&str>,
    outer: Option<&str>,
    facts: &mut FileFacts,
) {
    match item {
        Item::Class(class) => collect_class(class, module, package, outer, facts),
        Item::Method(method) => collect_top_method(method, module, package, facts),
        Item::Field(field) => collect_top_field(field, module, package, facts),
        Item::Group(items) => {
            for item in items {
                collect_item(item, module, package, outer, facts);
            }
        }
        Item::Initializer(_) | Item::Statement(_) => {}
    }
}

fn collect_class(
    class: &ClassDecl,
    module: &str,
    package: Option<&str>,
    outer: Option<&str>,
    facts: &mut FileFacts,
) {
    let fqn = match outer {
        Some(outer) => format!("{outer}${}", class.name),
        None => match package {
            Some(package) => format!("{package}.{}", class.name),
            None => class.name.clone(),
        },
    };
    let is_nested = outer.is_some();
    let mut class_fact = ClassFact {
        fqn: fqn.clone(),
        name: class.name.clone(),
        package: package.map(ToString::to_string),
        module: module.to_string(),
        kind: class.kind,
        is_abstract: class.is_abstract,
        is_nested,
        annotations: class
            .annotation_details
            .iter()
            .map(AnnotationFact::from_ast)
            .collect(),
        extends: class.extends.clone(),
        implements: class.implements.clone(),
        methods: Vec::new(),
        fields: Vec::new(),
        span: class.span,
    };
    if let Some(extends) = &class.extends {
        facts.references.push(ReferenceFact {
            from: fqn.clone(),
            to: extends.clone(),
            kind: ReferenceKind::Extends,
            span: class.span,
        });
    }
    for interface in &class.implements {
        facts.references.push(ReferenceFact {
            from: fqn.clone(),
            to: interface.clone(),
            kind: ReferenceKind::Implements,
            span: class.span,
        });
    }
    for annotation in &class.annotation_details {
        facts.references.push(ReferenceFact {
            from: fqn.clone(),
            to: annotation.name.clone(),
            kind: ReferenceKind::Annotation,
            span: class.span,
        });
    }
    for member in &class.members {
        match member {
            Item::Field(field) => {
                let fact = field_fact(field, &fqn, facts);
                class_fact.fields.push(fact);
            }
            Item::Method(method) => {
                let fact = method_fact(method, &fqn, facts);
                class_fact.methods.push(fact);
            }
            Item::Class(nested) => collect_class(nested, module, package, Some(&fqn), facts),
            Item::Group(items) => {
                for item in items {
                    match item {
                        Item::Field(field) => {
                            let fact = field_fact(field, &fqn, facts);
                            class_fact.fields.push(fact);
                        }
                        Item::Method(method) => {
                            let fact = method_fact(method, &fqn, facts);
                            class_fact.methods.push(fact);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    facts.classes.push(class_fact);
}

fn collect_top_method(
    method: &MethodDecl,
    module: &str,
    package: Option<&str>,
    facts: &mut FileFacts,
) {
    let owner = package.unwrap_or_default().to_string();
    let _ = module;
    let _ = method_fact(method, &owner, facts);
}

fn collect_top_field(
    field: &groovylint_syntax::ast::FieldDecl,
    module: &str,
    package: Option<&str>,
    facts: &mut FileFacts,
) {
    let owner = package.unwrap_or_default().to_string();
    let _ = module;
    let _ = field_fact(field, &owner, facts);
}

fn field_fact(
    field: &groovylint_syntax::ast::FieldDecl,
    owner: &str,
    facts: &mut FileFacts,
) -> FieldFact {
    if let Some(type_name) = &field.type_name {
        facts.references.push(ReferenceFact {
            from: owner.to_string(),
            to: type_name.clone(),
            kind: ReferenceKind::FieldType,
            span: field.span,
        });
    }
    for annotation in &field.annotation_details {
        facts.references.push(ReferenceFact {
            from: owner.to_string(),
            to: annotation.name.clone(),
            kind: ReferenceKind::Annotation,
            span: field.span,
        });
    }
    FieldFact {
        owner: owner.to_string(),
        name: field.name.clone(),
        type_name: field.type_name.clone(),
        annotations: field
            .annotation_details
            .iter()
            .map(AnnotationFact::from_ast)
            .collect(),
        modifiers: field_modifiers(field),
        span: field.span,
    }
}

fn method_fact(method: &MethodDecl, owner: &str, facts: &mut FileFacts) -> MethodFact {
    if let Some(return_type) = &method.return_type {
        facts.references.push(ReferenceFact {
            from: owner.to_string(),
            to: return_type.clone(),
            kind: ReferenceKind::ReturnType,
            span: method.span,
        });
    }
    let mut parameters = Vec::new();
    for param in &method.params {
        if let Some(type_name) = &param.type_name {
            facts.references.push(ReferenceFact {
                from: owner.to_string(),
                to: type_name.clone(),
                kind: ReferenceKind::ParamType,
                span: param.span,
            });
        }
        for annotation in &param.annotation_details {
            facts.references.push(ReferenceFact {
                from: owner.to_string(),
                to: annotation.name.clone(),
                kind: ReferenceKind::Annotation,
                span: param.span,
            });
        }
        parameters.push(ParameterFact {
            name: param.name.clone(),
            type_name: param.type_name.clone(),
            annotations: param
                .annotation_details
                .iter()
                .map(AnnotationFact::from_ast)
                .collect(),
        });
    }
    for annotation in &method.annotation_details {
        facts.references.push(ReferenceFact {
            from: owner.to_string(),
            to: annotation.name.clone(),
            kind: ReferenceKind::Annotation,
            span: method.span,
        });
    }
    if let Some(body) = &method.body {
        collect_block_references(body, owner, facts);
    }
    MethodFact {
        owner: owner.to_string(),
        name: method.name.clone(),
        return_type: method.return_type.clone(),
        annotations: method
            .annotation_details
            .iter()
            .map(AnnotationFact::from_ast)
            .collect(),
        modifiers: method_modifiers(method),
        parameters,
        span: method.span,
    }
}

fn collect_block_references(block: &Block, owner: &str, facts: &mut FileFacts) {
    for statement in &block.statements {
        collect_stmt_references(statement, owner, facts);
    }
}

fn collect_stmt_references(stmt: &Stmt, owner: &str, facts: &mut FileFacts) {
    match stmt {
        Stmt::Block(block) => collect_block_references(block, owner, facts),
        Stmt::VarDecl(decl) => {
            if let Some(type_name) = &decl.type_name {
                facts.references.push(ReferenceFact {
                    from: owner.to_string(),
                    to: type_name.clone(),
                    kind: ReferenceKind::LocalType,
                    span: decl.span,
                });
            }
            if let Some(init) = &decl.init {
                collect_expr_references(init, owner, facts);
            }
        }
        Stmt::Expr(expr)
        | Stmt::Return(groovylint_syntax::ast::ReturnStmt {
            value: Some(expr), ..
        })
        | Stmt::Throw(groovylint_syntax::ast::ThrowStmt { value: expr, .. }) => {
            collect_expr_references(expr, owner, facts);
        }
        Stmt::If(branch) => {
            collect_expr_references(&branch.condition, owner, facts);
            collect_stmt_references(&branch.then_branch, owner, facts);
            if let Some(else_branch) = &branch.else_branch {
                collect_stmt_references(else_branch, owner, facts);
            }
        }
        Stmt::While(statement) => {
            collect_expr_references(&statement.condition, owner, facts);
            collect_stmt_references(&statement.body, owner, facts);
        }
        Stmt::DoWhile(statement) => {
            collect_stmt_references(&statement.body, owner, facts);
            collect_expr_references(&statement.condition, owner, facts);
        }
        Stmt::For(statement) => {
            if let Some(init) = &statement.init {
                collect_stmt_references(init, owner, facts);
            }
            if let Some(condition) = &statement.condition {
                collect_expr_references(condition, owner, facts);
            }
            if let Some(update) = &statement.update {
                collect_expr_references(update, owner, facts);
            }
            collect_stmt_references(&statement.body, owner, facts);
        }
        Stmt::ForEach(statement) => {
            collect_expr_references(&statement.iterable, owner, facts);
            collect_stmt_references(&statement.body, owner, facts);
        }
        Stmt::Try(statement) => {
            collect_block_references(&statement.body, owner, facts);
            for catch in &statement.catches {
                collect_block_references(&catch.body, owner, facts);
            }
            if let Some(finally) = &statement.finally {
                collect_block_references(finally, owner, facts);
            }
        }
        Stmt::Synchronized(statement) => {
            collect_expr_references(&statement.expression, owner, facts);
            collect_stmt_references(&statement.body, owner, facts);
        }
        Stmt::Switch(statement) => {
            collect_expr_references(&statement.expression, owner, facts);
            for case in &statement.cases {
                for statement in &case.body {
                    collect_stmt_references(statement, owner, facts);
                }
            }
        }
        Stmt::Assert(statement) => {
            collect_expr_references(&statement.condition, owner, facts);
            if let Some(message) = &statement.message {
                collect_expr_references(message, owner, facts);
            }
        }
        _ => {}
    }
}

fn collect_expr_references(expr: &Expr, owner: &str, facts: &mut FileFacts) {
    match &expr.kind {
        ExprKind::New {
            type_name,
            args,
            body,
        } => {
            facts.references.push(ReferenceFact {
                from: owner.to_string(),
                to: type_name.clone(),
                kind: ReferenceKind::New,
                span: expr.span,
            });
            for arg in args {
                collect_expr_references(arg, owner, facts);
            }
            if let Some(items) = body {
                for item in items {
                    if let Item::Field(field) = item {
                        let _ = field_fact(field, owner, facts);
                    }
                }
            }
        }
        ExprKind::Cast { expr, type_name } => {
            facts.references.push(ReferenceFact {
                from: owner.to_string(),
                to: type_name.clone(),
                kind: ReferenceKind::Cast,
                span: expr.span,
            });
            collect_expr_references(expr, owner, facts);
        }
        ExprKind::Call { receiver, args, .. } => {
            if let Some(receiver) = receiver {
                collect_expr_references(receiver, owner, facts);
            }
            for arg in args {
                collect_expr_references(arg, owner, facts);
            }
        }
        ExprKind::Property { receiver, .. }
        | ExprKind::Index {
            target: receiver, ..
        } => {
            collect_expr_references(receiver, owner, facts);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_references(left, owner, facts);
            collect_expr_references(right, owner, facts);
        }
        ExprKind::Unary { operand, .. } => collect_expr_references(operand, owner, facts),
        ExprKind::Assign { target, value, .. } => {
            collect_expr_references(target, owner, facts);
            collect_expr_references(value, owner, facts);
        }
        ExprKind::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_expr_references(condition, owner, facts);
            collect_expr_references(then_expr, owner, facts);
            collect_expr_references(else_expr, owner, facts);
        }
        ExprKind::List(items) => {
            for item in items {
                collect_expr_references(item, owner, facts);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_expr_references(key, owner, facts);
                collect_expr_references(value, owner, facts);
            }
        }
        ExprKind::Closure { body, .. } => collect_block_references(body, owner, facts),
        ExprKind::NamedArg { value, .. } => collect_expr_references(value, owner, facts),
        _ => {}
    }
}

fn method_modifiers(method: &MethodDecl) -> Vec<String> {
    let mut modifiers = Vec::new();
    push_modifier(&mut modifiers, method.is_static, "static");
    push_modifier(&mut modifiers, method.is_public, "public");
    push_modifier(&mut modifiers, method.is_protected, "protected");
    push_modifier(&mut modifiers, method.is_private, "private");
    push_modifier(&mut modifiers, method.is_abstract, "abstract");
    push_modifier(&mut modifiers, method.is_final, "final");
    modifiers
}

fn field_modifiers(field: &groovylint_syntax::ast::FieldDecl) -> Vec<String> {
    let mut modifiers = Vec::new();
    push_modifier(&mut modifiers, field.is_static, "static");
    push_modifier(&mut modifiers, field.is_public, "public");
    push_modifier(&mut modifiers, field.is_protected, "protected");
    push_modifier(&mut modifiers, field.is_private, "private");
    push_modifier(&mut modifiers, field.is_final, "final");
    modifiers
}

fn push_modifier(modifiers: &mut Vec<String>, present: bool, name: &str) {
    if present {
        modifiers.push(name.to_string());
    }
}

/// Returns the module directory for a source path by looking for the nearest
/// build file, falling back to the first directory under `root`.
#[must_use]
pub fn module_for(path: &Path, root: &Path) -> String {
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory.join("build.gradle").is_file() || directory.join("build.gradle.kts").is_file()
        {
            return relative_module(directory, root);
        }
        if directory == root {
            break;
        }
        current = directory.parent();
    }
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.components().next().map_or_else(
        || "root".to_string(),
        |component| component.as_os_str().to_string_lossy().to_string(),
    )
}

fn relative_module(directory: &Path, root: &Path) -> String {
    let relative = directory.strip_prefix(root).unwrap_or(directory);
    let parts: Vec<String> = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();
    if parts.is_empty() {
        "root".to_string()
    } else {
        parts.join(":")
    }
}
