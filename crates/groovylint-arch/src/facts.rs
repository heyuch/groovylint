//! Compact per-file facts used by the architecture engine.

use std::path::{Path, PathBuf};

use groovylint_core::Span;
use groovylint_syntax::ast::{
    Annotation, Block, ClassDecl, ClassKind as AstClassKind, Expr, ExprKind, Item, MethodDecl,
    Script, Stmt,
};

use crate::model::{ArchConfig, ClassPredicate, Constraint, MethodPredicate, Target};

/// Which fact categories a configuration needs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FactPlan(u16);

impl FactPlan {
    pub const IMPORTS: Self = Self(1 << 0);
    pub const CLASSES: Self = Self(1 << 1);
    pub const METHODS: Self = Self(1 << 2);
    pub const FIELDS: Self = Self(1 << 3);
    pub const ANNOTATIONS: Self = Self(1 << 4);
    pub const TYPE_REFS: Self = Self(1 << 5);
    pub const PARAMS: Self = Self(1 << 6);

    #[must_use]
    pub const fn all() -> Self {
        Self(
            Self::IMPORTS.0
                | Self::CLASSES.0
                | Self::METHODS.0
                | Self::FIELDS.0
                | Self::ANNOTATIONS.0
                | Self::TYPE_REFS.0
                | Self::PARAMS.0,
        )
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Derives a plan from targets and constraints.
    #[must_use]
    pub fn from_config(config: &ArchConfig) -> Self {
        let mut plan = Self::default();
        for target in config.targets.values() {
            plan = plan.union(target_requirements(target));
        }
        for rule in &config.rules {
            plan = plan.union(rule_target_requirements(&rule.target, config));
            for constraint in &rule.must.constraints {
                plan = plan.union(constraint_requirements(constraint));
            }
        }
        plan
    }
}

fn rule_target_requirements(reference: &crate::model::TargetRef, config: &ArchConfig) -> FactPlan {
    match reference {
        crate::model::TargetRef::Named(name) => config
            .targets
            .get(name)
            .map_or_else(FactPlan::default, target_requirements),
        crate::model::TargetRef::Inline(target) => target_requirements(target),
    }
}

fn target_requirements(target: &Target) -> FactPlan {
    match target {
        Target::Module(_) => FactPlan::default(),
        Target::Package(_) => FactPlan::CLASSES,
        Target::Class(target) => {
            let mut plan = FactPlan::CLASSES;
            for predicate in &target.predicates {
                if matches!(predicate, ClassPredicate::AnnotatedWith(_)) {
                    plan = plan.union(FactPlan::ANNOTATIONS);
                }
            }
            plan
        }
        Target::Method(target) => {
            let mut plan = FactPlan::METHODS.union(FactPlan::CLASSES);
            for predicate in &target.predicates {
                match predicate {
                    MethodPredicate::AnnotatedWith(_) => {
                        plan = plan.union(FactPlan::ANNOTATIONS);
                    }
                    MethodPredicate::ParamCount(_) => {
                        plan = plan.union(FactPlan::PARAMS);
                    }
                    _ => {}
                }
            }
            plan
        }
    }
}

fn constraint_requirements(constraint: &Constraint) -> FactPlan {
    match constraint {
        Constraint::DependOnlyOn(_) | Constraint::NotDependOn(_) | Constraint::Visible(_) => {
            FactPlan::IMPORTS
                .union(FactPlan::TYPE_REFS)
                .union(FactPlan::CLASSES)
        }
        Constraint::AnnotatedWith(_)
        | Constraint::NotAnnotatedWith(_)
        | Constraint::AnnotationArgument(_) => FactPlan::ANNOTATIONS,
        Constraint::Extend(_) | Constraint::Implement(_) | Constraint::Exist(_) => {
            FactPlan::CLASSES
        }
        Constraint::AllMembers(_) => FactPlan::CLASSES
            .union(FactPlan::METHODS)
            .union(FactPlan::FIELDS)
            .union(FactPlan::ANNOTATIONS)
            .union(FactPlan::PARAMS),
        Constraint::AllMethods(_) => FactPlan::METHODS
            .union(FactPlan::ANNOTATIONS)
            .union(FactPlan::PARAMS),
        Constraint::AllFields(_) => FactPlan::FIELDS.union(FactPlan::ANNOTATIONS),
        Constraint::RawReturnType(_)
        | Constraint::NotRawReturnType(_)
        | Constraint::Visibility(_) => FactPlan::METHODS,
        Constraint::AllParams(_) => FactPlan::METHODS
            .union(FactPlan::PARAMS)
            .union(FactPlan::ANNOTATIONS),
        Constraint::Children(_) | Constraint::Counts(_) => FactPlan::default(),
        Constraint::Acyclic(_) => FactPlan::CLASSES.union(FactPlan::TYPE_REFS),
    }
}

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
    pub plan: FactPlan,
    pub imports: Vec<ImportFact>,
    pub classes: Vec<ClassFact>,
    pub references: Vec<ReferenceFact>,
}

/// Event-driven fact collector used by the single AST traversal.
pub struct FactCollector {
    plan: FactPlan,
    path: PathBuf,
    module: String,
    package: Option<String>,
    imports: Vec<ImportFact>,
    classes: Vec<ClassFact>,
    references: Vec<ReferenceFact>,
    class_stack: Vec<usize>,
}

impl FactCollector {
    #[must_use]
    pub fn new(path: &Path, module: &str, plan: FactPlan) -> Self {
        Self {
            plan,
            path: path.to_path_buf(),
            module: module.to_string(),
            package: None,
            imports: Vec::new(),
            classes: Vec::new(),
            references: Vec::new(),
            class_stack: Vec::new(),
        }
    }

    pub fn enter_script(&mut self, script: &Script) {
        self.package = script.package.as_ref().map(|package| package.name.clone());
        if self.plan.contains(FactPlan::IMPORTS) {
            self.imports = script
                .imports
                .iter()
                .map(|import| ImportFact {
                    path: import.path.clone(),
                    alias: import.alias.clone(),
                    is_static: import.is_static,
                    span: import.span,
                })
                .collect();
        }
    }

    pub fn enter_class(&mut self, class: &ClassDecl) {
        if !self.plan.contains(FactPlan::CLASSES) {
            return;
        }
        let outer = self
            .class_stack
            .last()
            .and_then(|index| self.classes.get(*index))
            .map(|class| class.fqn.clone());
        let fqn = match &outer {
            Some(outer) => format!("{outer}${}", class.name),
            None => match &self.package {
                Some(package) => format!("{package}.{}", class.name),
                None => class.name.clone(),
            },
        };
        let annotations = if self.plan.contains(FactPlan::ANNOTATIONS) {
            class
                .annotation_details
                .iter()
                .map(AnnotationFact::from_ast)
                .collect()
        } else {
            Vec::new()
        };
        let fact = ClassFact {
            fqn: fqn.clone(),
            name: class.name.clone(),
            package: self.package.clone(),
            module: self.module.clone(),
            kind: class.kind,
            is_abstract: class.is_abstract,
            is_nested: outer.is_some(),
            annotations,
            extends: class.extends.clone(),
            implements: class.implements.clone(),
            methods: Vec::new(),
            fields: Vec::new(),
            span: class.span,
        };
        if self.plan.contains(FactPlan::TYPE_REFS) {
            if let Some(extends) = &class.extends {
                self.references.push(ReferenceFact {
                    from: fqn.clone(),
                    to: extends.clone(),
                    kind: ReferenceKind::Extends,
                    span: class.span,
                });
            }
            for interface in &class.implements {
                self.references.push(ReferenceFact {
                    from: fqn.clone(),
                    to: interface.clone(),
                    kind: ReferenceKind::Implements,
                    span: class.span,
                });
            }
            if self.plan.contains(FactPlan::ANNOTATIONS) {
                for annotation in &class.annotation_details {
                    self.references.push(ReferenceFact {
                        from: fqn.clone(),
                        to: annotation.name.clone(),
                        kind: ReferenceKind::Annotation,
                        span: class.span,
                    });
                }
            }
        }
        let index = self.classes.len();
        self.classes.push(fact);
        self.class_stack.push(index);
    }

    pub fn exit_class(&mut self) {
        self.class_stack.pop();
    }

    pub fn enter_field(&mut self, field: &groovylint_syntax::ast::FieldDecl) {
        let Some(index) = self.class_stack.last().copied() else {
            return;
        };
        if !self.plan.contains(FactPlan::FIELDS) {
            return;
        }
        let owner = self.classes[index].fqn.clone();
        let fact = self.field_fact(field, &owner);
        self.classes[index].fields.push(fact);
    }

    pub fn enter_method(&mut self, method: &MethodDecl) {
        let Some(index) = self.class_stack.last().copied() else {
            return;
        };
        if !self.plan.contains(FactPlan::METHODS) {
            return;
        }
        let owner = self.classes[index].fqn.clone();
        let fact = self.method_fact(method, &owner);
        self.classes[index].methods.push(fact);
    }

    pub fn enter_stmt(&mut self, stmt: &Stmt) {
        if !self.plan.contains(FactPlan::TYPE_REFS) {
            return;
        }
        if let Some(owner) = self.owner() {
            if let Stmt::VarDecl(decl) = stmt
                && let Some(type_name) = &decl.type_name
            {
                self.references.push(ReferenceFact {
                    from: owner.to_string(),
                    to: type_name.clone(),
                    kind: ReferenceKind::LocalType,
                    span: decl.span,
                });
            } else if let Stmt::ForEach(for_each) = stmt
                && let Some(type_name) = &for_each.var_type
            {
                self.references.push(ReferenceFact {
                    from: owner.to_string(),
                    to: type_name.clone(),
                    kind: ReferenceKind::LocalType,
                    span: for_each.span,
                });
            }
        }
    }

    pub fn enter_expr(&mut self, expr: &Expr) {
        if !self.plan.contains(FactPlan::TYPE_REFS) {
            return;
        }
        let Some(owner) = self.owner().map(ToString::to_string) else {
            return;
        };
        match &expr.kind {
            ExprKind::New { type_name, .. } => self.references.push(ReferenceFact {
                from: owner,
                to: type_name.clone(),
                kind: ReferenceKind::New,
                span: expr.span,
            }),
            ExprKind::Cast { type_name, .. } => self.references.push(ReferenceFact {
                from: owner,
                to: type_name.clone(),
                kind: ReferenceKind::Cast,
                span: expr.span,
            }),
            _ => {}
        }
    }

    #[must_use]
    pub fn finish(self) -> FileFacts {
        FileFacts {
            path: self.path,
            module: self.module,
            package: self.package,
            plan: self.plan,
            imports: self.imports,
            classes: self.classes,
            references: self.references,
        }
    }

    fn owner(&self) -> Option<&str> {
        self.class_stack
            .last()
            .and_then(|index| self.classes.get(*index))
            .map(|class| class.fqn.as_str())
    }

    fn field_fact(&mut self, field: &groovylint_syntax::ast::FieldDecl, owner: &str) -> FieldFact {
        if self.plan.contains(FactPlan::TYPE_REFS) {
            if let Some(type_name) = &field.type_name {
                self.references.push(ReferenceFact {
                    from: owner.to_string(),
                    to: type_name.clone(),
                    kind: ReferenceKind::FieldType,
                    span: field.span,
                });
            }
            if self.plan.contains(FactPlan::ANNOTATIONS) {
                for annotation in &field.annotation_details {
                    self.references.push(ReferenceFact {
                        from: owner.to_string(),
                        to: annotation.name.clone(),
                        kind: ReferenceKind::Annotation,
                        span: field.span,
                    });
                }
            }
        }
        FieldFact {
            owner: owner.to_string(),
            name: field.name.clone(),
            type_name: field.type_name.clone(),
            annotations: if self.plan.contains(FactPlan::ANNOTATIONS) {
                field
                    .annotation_details
                    .iter()
                    .map(AnnotationFact::from_ast)
                    .collect()
            } else {
                Vec::new()
            },
            modifiers: field_modifiers(field),
            span: field.span,
        }
    }

    fn method_fact(&mut self, method: &MethodDecl, owner: &str) -> MethodFact {
        if self.plan.contains(FactPlan::TYPE_REFS) {
            if let Some(return_type) = &method.return_type {
                self.references.push(ReferenceFact {
                    from: owner.to_string(),
                    to: return_type.clone(),
                    kind: ReferenceKind::ReturnType,
                    span: method.span,
                });
            }
            for param in &method.params {
                if let Some(type_name) = &param.type_name {
                    self.references.push(ReferenceFact {
                        from: owner.to_string(),
                        to: type_name.clone(),
                        kind: ReferenceKind::ParamType,
                        span: param.span,
                    });
                }
            }
            if self.plan.contains(FactPlan::ANNOTATIONS) {
                for annotation in &method.annotation_details {
                    self.references.push(ReferenceFact {
                        from: owner.to_string(),
                        to: annotation.name.clone(),
                        kind: ReferenceKind::Annotation,
                        span: method.span,
                    });
                }
            }
        }
        let parameters = if self.plan.contains(FactPlan::PARAMS) {
            method
                .params
                .iter()
                .map(|param| ParameterFact {
                    name: param.name.clone(),
                    type_name: param.type_name.clone(),
                    annotations: if self.plan.contains(FactPlan::ANNOTATIONS) {
                        param
                            .annotation_details
                            .iter()
                            .map(AnnotationFact::from_ast)
                            .collect()
                    } else {
                        Vec::new()
                    },
                })
                .collect()
        } else {
            Vec::new()
        };
        MethodFact {
            owner: owner.to_string(),
            name: method.name.clone(),
            return_type: method.return_type.clone(),
            annotations: if self.plan.contains(FactPlan::ANNOTATIONS) {
                method
                    .annotation_details
                    .iter()
                    .map(AnnotationFact::from_ast)
                    .collect()
            } else {
                Vec::new()
            },
            modifiers: method_modifiers(method),
            parameters,
            span: method.span,
        }
    }
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
        plan: FactPlan::all(),
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
