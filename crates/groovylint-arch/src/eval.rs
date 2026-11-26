//! Architecture rule evaluation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use groovylint_core::{Severity, Span};
use regex::Regex;

use crate::facts::{AnnotationFact, ClassFact, FieldFact, MethodFact, ParameterFact};
use crate::model::{
    ArchConfig, ClassKind, ClassPredicate, Constraint, Count, DependencyRef, FieldPredicate,
    MemberKind, MemberPredicate, MethodPredicate, ModulePredicate, PackagePredicate, PatternList,
    Rule, Severity as ArchSeverity, Target, TargetRef, TargetType, TypeValue, Visibility,
};
use crate::project::{Edge, Project};

/// One architecture finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchFinding {
    pub path: PathBuf,
    pub span: Span,
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub help: Option<String>,
}

/// Evaluates the configured rules against a project.
#[must_use]
pub fn check(config: &ArchConfig, project: &Project) -> Vec<ArchFinding> {
    let mut evaluator = Evaluator {
        config,
        project,
        target_cache: BTreeMap::new(),
        target_stack: BTreeSet::new(),
        findings: Vec::new(),
    };
    for rule in &config.rules {
        evaluator.evaluate_rule(rule);
    }
    evaluator.findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.span.start.cmp(&right.span.start))
            .then_with(|| left.rule.cmp(&right.rule))
            .then_with(|| left.message.cmp(&right.message))
    });
    evaluator.findings
}

struct Evaluator<'a> {
    config: &'a ArchConfig,
    project: &'a Project,
    target_cache: BTreeMap<String, BTreeSet<String>>,
    target_stack: BTreeSet<String>,
    findings: Vec<ArchFinding>,
}

impl Evaluator<'_> {
    fn evaluate_rule(&mut self, rule: &Rule) {
        let target_type = self.target_type(&rule.target);
        let Some(target_type) = target_type else {
            return;
        };
        let nodes = self.target_nodes(&rule.target);
        if nodes.is_empty() {
            for constraint in &rule.must.constraints {
                if matches!(constraint, Constraint::Exist(true)) {
                    self.push_root(rule, "target matches no node");
                }
            }
            return;
        }
        for constraint in &rule.must.constraints {
            match constraint {
                Constraint::DependOnlyOn(dependencies) => {
                    self.evaluate_dependencies(rule, target_type, &nodes, dependencies, false);
                }
                Constraint::NotDependOn(dependencies) => {
                    self.evaluate_dependencies(rule, target_type, &nodes, dependencies, true);
                }
                Constraint::Visible(dependencies) => {
                    self.evaluate_visible(rule, target_type, &nodes, dependencies);
                }
                Constraint::AnnotatedWith(annotations) => {
                    self.evaluate_annotations(rule, target_type, &nodes, annotations, false);
                }
                Constraint::NotAnnotatedWith(annotations) => {
                    self.evaluate_annotations(rule, target_type, &nodes, annotations, true);
                }
                Constraint::AnnotationArgument(argument) => {
                    self.evaluate_annotation_argument(rule, target_type, &nodes, argument);
                }
                Constraint::Extend(supertype) => {
                    self.evaluate_extend(rule, target_type, &nodes, supertype);
                }
                Constraint::Implement(supertypes) => {
                    self.evaluate_implement(rule, target_type, &nodes, supertypes);
                }
                Constraint::Exist(exist) => self.evaluate_exist(rule, target_type, &nodes, *exist),
                Constraint::AllMembers(predicates) => {
                    self.evaluate_all_members(rule, target_type, &nodes, predicates);
                }
                Constraint::AllMethods(predicates) => {
                    self.evaluate_all_methods(rule, target_type, &nodes, predicates);
                }
                Constraint::AllFields(predicates) => {
                    self.evaluate_all_fields(rule, target_type, &nodes, predicates);
                }
                Constraint::RawReturnType(type_name) => {
                    self.evaluate_method_signature(rule, target_type, &nodes, type_name, false);
                }
                Constraint::NotRawReturnType(type_names) => {
                    self.evaluate_method_signature_not(rule, target_type, &nodes, type_names);
                }
                Constraint::AllParams(predicates) => {
                    self.evaluate_all_params(rule, target_type, &nodes, predicates);
                }
                Constraint::Visibility(visibility) => {
                    self.evaluate_visibility(rule, target_type, &nodes, *visibility);
                }
                Constraint::Children(children) => {
                    self.evaluate_children(rule, target_type, &nodes, children);
                }
                Constraint::Counts(counts) => {
                    self.evaluate_counts(rule, target_type, &nodes, counts);
                }
                Constraint::Acyclic(acyclic) => {
                    self.evaluate_acyclic(rule, target_type, &nodes, *acyclic);
                }
            }
        }
    }

    fn target_type(&self, reference: &TargetRef) -> Option<TargetType> {
        match reference {
            TargetRef::Named(name) => self.config.targets.get(name).map(Target::target_type),
            TargetRef::Inline(target) => Some(target.target_type()),
        }
    }

    fn target_nodes(&mut self, reference: &TargetRef) -> BTreeSet<String> {
        match reference {
            TargetRef::Named(name) => {
                if let Some(cached) = self.target_cache.get(name) {
                    return cached.clone();
                }
                if !self.target_stack.insert(name.clone()) {
                    return BTreeSet::new();
                }
                let nodes = if let Some(target) = self.config.targets.get(name) {
                    self.evaluate_target(target)
                } else {
                    BTreeSet::new()
                };
                self.target_stack.remove(name);
                self.target_cache.insert(name.clone(), nodes.clone());
                nodes
            }
            TargetRef::Inline(target) => self.evaluate_target(target),
        }
    }

    fn evaluate_target(&mut self, target: &Target) -> BTreeSet<String> {
        let candidates = self.nodes_of_type(target.target_type());
        candidates
            .into_iter()
            .filter(|node| self.matches_target(target, node))
            .collect()
    }

    fn nodes_of_type(&self, target_type: TargetType) -> Vec<String> {
        match target_type {
            TargetType::Module => self.project.module_ids(),
            TargetType::Package => self.project.packages.iter().cloned().collect(),
            TargetType::Class => self.project.classes.keys().cloned().collect(),
            TargetType::Method => self
                .project
                .classes
                .values()
                .flat_map(|class| class.methods.iter().map(MethodFact::id))
                .collect(),
        }
    }

    fn matches_target(&mut self, target: &Target, node: &str) -> bool {
        match target {
            Target::Module(target) => {
                let Some(module) = self.project.modules.get(node) else {
                    return false;
                };
                target.predicates.iter().all(|predicate| match predicate {
                    ModulePredicate::Name(patterns) => matches_name(patterns, &module.name),
                    ModulePredicate::NameMatches(pattern) => matches_regex(pattern, &module.name),
                    ModulePredicate::Path(patterns) => matches_name(patterns, &module.id),
                    ModulePredicate::AnyOf(refs) => self.target_nodes_any(refs, node),
                    ModulePredicate::Not(inner) => {
                        !self.matches_target(&Target::Module((**inner).clone()), node)
                    }
                })
            }
            Target::Package(target) => target.predicates.iter().all(|predicate| match predicate {
                PackagePredicate::Name(patterns) => matches_name(patterns, node),
                PackagePredicate::NameMatches(pattern) => matches_regex(pattern, node),
                PackagePredicate::In(reference) => self.node_in_target(node, reference),
                PackagePredicate::AnyOf(refs) => self.target_nodes_any(refs, node),
                PackagePredicate::Not(inner) => {
                    !self.matches_target(&Target::Package((**inner).clone()), node)
                }
            }),
            Target::Class(target) => {
                let Some(class) = self.project.class(node) else {
                    return false;
                };
                target
                    .predicates
                    .iter()
                    .all(|predicate| self.matches_class_predicate(class, node, predicate))
            }
            Target::Method(target) => {
                let Some(method) = self.method(node).cloned() else {
                    return false;
                };
                target
                    .predicates
                    .iter()
                    .all(|predicate| self.matches_method_predicate(&method, node, predicate))
            }
        }
    }

    fn matches_class_predicate(
        &mut self,
        class: &ClassFact,
        node: &str,
        predicate: &ClassPredicate,
    ) -> bool {
        match predicate {
            ClassPredicate::Name(patterns) => matches_name(patterns, &class.name),
            ClassPredicate::NameMatches(pattern) => matches_regex(pattern, &class.name),
            ClassPredicate::In(reference) => self.node_in_target(node, reference),
            ClassPredicate::AnyOf(refs) => self.target_nodes_any(refs, node),
            ClassPredicate::Not(inner) => {
                !self.matches_target(&Target::Class((**inner).clone()), node)
            }
            ClassPredicate::Kind(kind) => class.kind == ast_kind(*kind),
            ClassPredicate::Abstract(abstract_) => class.is_abstract == *abstract_,
            ClassPredicate::Nested(nested) => class.is_nested == *nested,
            ClassPredicate::Extends(supertype) => class
                .extends
                .as_deref()
                .is_some_and(|name| type_matches(name, supertype)),
            ClassPredicate::Implements(supertypes) => supertypes.iter().all(|supertype| {
                class
                    .implements
                    .iter()
                    .any(|name| type_matches(name, supertype))
            }),
            ClassPredicate::AnnotatedWith(annotations) => {
                has_annotations(&class.annotations, annotations)
            }
        }
    }

    fn matches_method_predicate(
        &mut self,
        method: &MethodFact,
        node: &str,
        predicate: &MethodPredicate,
    ) -> bool {
        match predicate {
            MethodPredicate::Name(patterns) => matches_name(patterns, &method.name),
            MethodPredicate::NameMatches(pattern) => matches_regex(pattern, &method.name),
            MethodPredicate::In(reference) => self.node_in_target(&method.owner, reference),
            MethodPredicate::AnyOf(refs) => self.target_nodes_any(refs, node),
            MethodPredicate::Not(inner) => {
                !self.matches_target(&Target::Method((**inner).clone()), node)
            }
            MethodPredicate::AnnotatedWith(annotations) => {
                has_annotations(&method.annotations, annotations)
            }
            MethodPredicate::Modifiers(modifiers) => modifiers
                .iter()
                .all(|modifier| method.modifiers.contains(&modifier_name(*modifier))),
            MethodPredicate::ParamCount(count) => method.parameters.len() == *count,
            MethodPredicate::RawReturnType(type_name) => method
                .return_type
                .as_deref()
                .is_some_and(|return_type| type_matches(return_type, type_name)),
        }
    }

    fn node_in_target(&mut self, node: &str, reference: &TargetRef) -> bool {
        let Some(target_type) = self.target_type(reference) else {
            return false;
        };
        let owner = if let Some(class) = self.project.class(node) {
            match target_type {
                TargetType::Module => Some(class.module.clone()),
                TargetType::Package => Some(
                    class
                        .package
                        .clone()
                        .unwrap_or_else(|| "(default)".to_string()),
                ),
                TargetType::Class => Some(class.fqn.clone()),
                TargetType::Method => None,
            }
        } else if self.project.packages.contains(node) {
            match target_type {
                TargetType::Module => self.project.package_module(node).map(ToString::to_string),
                TargetType::Package => Some(node.to_string()),
                TargetType::Class | TargetType::Method => None,
            }
        } else if let Some(method) = self.method(node) {
            match target_type {
                TargetType::Class => Some(method.owner.clone()),
                TargetType::Module => self
                    .project
                    .class(&method.owner)
                    .map(|class| class.module.clone()),
                TargetType::Package => self.project.class(&method.owner).map(|class| {
                    class
                        .package
                        .clone()
                        .unwrap_or_else(|| "(default)".to_string())
                }),
                TargetType::Method => None,
            }
        } else {
            None
        };
        owner.is_some_and(|owner| self.target_nodes(reference).contains(&owner))
    }

    fn target_nodes_any(&mut self, refs: &[TargetRef], node: &str) -> bool {
        refs.iter()
            .any(|reference| self.target_nodes(reference).contains(node))
    }

    fn method(&self, node: &str) -> Option<&MethodFact> {
        let (owner, rest) = node.split_once('#')?;
        let (name, arity) = rest.split_once('/')?;
        let arity: usize = arity.parse().ok()?;
        self.project
            .classes
            .get(owner)?
            .methods
            .iter()
            .find(|method| method.name == name && method.parameters.len() == arity)
    }

    fn evaluate_dependencies(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        dependencies: &[DependencyRef],
        forbidden: bool,
    ) {
        for edge in &self.project.edges {
            let Some(source) = self.source_node(target_type, edge) else {
                continue;
            };
            if !nodes.contains(&source) {
                continue;
            }
            // `dependOnlyOn` describes the project's own architecture; JDK and
            // language runtime types are not something every rule should list.
            // Explicit `notDependOn` rules still see them.
            if !forbidden && is_platform_dependency(&edge.to) {
                continue;
            }
            let matched = dependencies
                .iter()
                .any(|dependency| self.dependency_matches(dependency, edge, &edge.from));
            if (forbidden && matched) || (!forbidden && !matched) {
                let (verb, relation) = if forbidden {
                    ("must not depend on", "forbidden dependency")
                } else {
                    ("depends on a target outside", "dependency not allowed")
                };
                self.push_edge(rule, edge, format!("{verb} `{}` ({relation})", edge.to));
            }
        }
    }

    fn evaluate_visible(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        dependencies: &[DependencyRef],
    ) {
        if target_type != TargetType::Package {
            return;
        }
        for edge in &self.project.edges {
            let target_package = self.project.package_of_type(&edge.to);
            if !nodes.contains(&target_package) {
                continue;
            }
            if self.project.class(&edge.from).is_none() {
                continue;
            }
            let allowed = dependencies
                .iter()
                .any(|dependency| self.dependency_matches(dependency, edge, &edge.from));
            if !allowed {
                self.push_edge(
                    rule,
                    edge,
                    format!("`{}` must not depend on this package", edge.from),
                );
            }
        }
    }

    fn dependency_matches(
        &mut self,
        dependency: &DependencyRef,
        edge: &Edge,
        source_node: &str,
    ) -> bool {
        match dependency {
            DependencyRef::None => false,
            DependencyRef::Any(reference) | DependencyRef::Other(reference) => {
                self.edge_in_target(edge, reference, source_node, false)
            }
            DependencyRef::Self_(reference) => {
                self.edge_in_target(edge, reference, source_node, true)
            }
        }
    }

    fn edge_in_target(
        &mut self,
        edge: &Edge,
        reference: &TargetRef,
        source_node: &str,
        same_module: bool,
    ) -> bool {
        let Some(target_type) = self.target_type(reference) else {
            return false;
        };
        let target_node = match target_type {
            TargetType::Class => edge.to.clone(),
            TargetType::Package => self.project.package_of_type(&edge.to),
            TargetType::Module => self
                .project
                .class(&edge.to)
                .map_or_else(String::new, |class| class.module.clone()),
            TargetType::Method => return false,
        };
        if !self.target_nodes(reference).contains(&target_node) {
            return false;
        }
        if !same_module {
            return true;
        }
        let source_module = self
            .project
            .class(source_node)
            .map(|class| class.module.as_str());
        let target_module = match target_type {
            TargetType::Class => self.project.class_module(&edge.to),
            TargetType::Package => Some(
                self.project
                    .package_module(&target_node)
                    .unwrap_or_default(),
            ),
            TargetType::Module => Some(target_node.as_str()),
            TargetType::Method => None,
        };
        source_module.is_some() && source_module == target_module
    }

    fn source_node(&self, target_type: TargetType, edge: &Edge) -> Option<String> {
        let class = self.project.class(&edge.from)?;
        match target_type {
            TargetType::Class => Some(class.fqn.clone()),
            TargetType::Package => Some(
                class
                    .package
                    .clone()
                    .unwrap_or_else(|| "(default)".to_string()),
            ),
            TargetType::Module => Some(class.module.clone()),
            TargetType::Method => None,
        }
    }

    fn evaluate_annotations(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        annotations: &[String],
        negated: bool,
    ) {
        for node in nodes {
            let present = match target_type {
                TargetType::Class => self
                    .project
                    .class(node)
                    .is_some_and(|class| has_annotations(&class.annotations, annotations)),
                TargetType::Method => self
                    .method(node)
                    .is_some_and(|method| has_annotations(&method.annotations, annotations)),
                TargetType::Module | TargetType::Package => false,
            };
            if present == negated {
                self.push_node(
                    rule,
                    target_type,
                    node,
                    if negated {
                        format!("must not be annotated with one of {annotations:?}")
                    } else {
                        format!("must be annotated with one of {annotations:?}")
                    },
                );
            }
        }
    }

    fn evaluate_annotation_argument(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        argument: &crate::model::AnnotationArgument,
    ) {
        for node in nodes {
            let annotations: Option<&[AnnotationFact]> = match target_type {
                TargetType::Class => self
                    .project
                    .class(node)
                    .map(|class| class.annotations.as_slice()),
                TargetType::Method => self
                    .method(node)
                    .map(|method| method.annotations.as_slice()),
                TargetType::Module | TargetType::Package => None,
            };
            let Some(annotations) = annotations else {
                continue;
            };
            let matches = annotations.iter().any(|annotation| {
                annotation.name == argument.annotation
                    && annotation.raw_arguments.as_deref().is_some_and(|raw| {
                        normalize(raw).contains(&normalize(&argument.name))
                            && normalize(raw).contains(&normalize(&argument.value))
                    })
            });
            if !matches {
                self.push_node(
                    rule,
                    target_type,
                    node,
                    format!(
                        "annotation {} must have {} = {}",
                        argument.annotation, argument.name, argument.value
                    ),
                );
            }
        }
    }

    fn evaluate_extend(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        supertype: &TypeValue,
    ) {
        if target_type != TargetType::Class {
            return;
        }
        for node in nodes {
            let Some(class) = self.project.class(node) else {
                continue;
            };
            if !class
                .extends
                .as_deref()
                .is_some_and(|name| type_matches(name, supertype))
            {
                self.push_node(
                    rule,
                    target_type,
                    node,
                    format!("must extend {}", supertype.0),
                );
            }
        }
    }

    fn evaluate_implement(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        supertypes: &[TypeValue],
    ) {
        if target_type != TargetType::Class {
            return;
        }
        for node in nodes {
            let Some(class) = self.project.class(node) else {
                continue;
            };
            if !supertypes.iter().all(|supertype| {
                class
                    .implements
                    .iter()
                    .any(|name| type_matches(name, supertype))
            }) {
                self.push_node(
                    rule,
                    target_type,
                    node,
                    "does not implement all required interfaces".to_string(),
                );
            }
        }
    }

    fn evaluate_exist(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        exist: bool,
    ) {
        if exist == nodes.is_empty() {
            if exist {
                self.push_root(rule, "target matches no node");
            } else {
                for node in nodes {
                    self.push_node(rule, target_type, node, "target must not exist".to_string());
                }
            }
        }
    }

    fn evaluate_all_members(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        predicates: &[MemberPredicate],
    ) {
        if target_type != TargetType::Class {
            return;
        }
        for node in nodes {
            let Some(class) = self.project.class(node).cloned() else {
                continue;
            };
            for method in &class.methods {
                if !matches_member(
                    MemberKind::Method,
                    &method.name,
                    &method.annotations,
                    &method.modifiers,
                    predicates,
                ) {
                    self.push_method(rule, &class, method, "member does not satisfy allMembers");
                }
            }
            for field in &class.fields {
                if !matches_member(
                    MemberKind::Field,
                    &field.name,
                    &field.annotations,
                    &field.modifiers,
                    predicates,
                ) {
                    self.push_field(rule, &class, field, "member does not satisfy allMembers");
                }
            }
        }
    }

    fn evaluate_all_methods(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        predicates: &[MethodPredicate],
    ) {
        if target_type != TargetType::Class {
            return;
        }
        for node in nodes {
            let Some(class) = self.project.class(node).cloned() else {
                continue;
            };
            for method in &class.methods {
                let method_id = method.id();
                if !predicates
                    .iter()
                    .all(|predicate| self.matches_method_predicate(method, &method_id, predicate))
                {
                    self.push_method(rule, &class, method, "method does not satisfy allMethods");
                }
            }
        }
    }

    fn evaluate_all_fields(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        predicates: &[FieldPredicate],
    ) {
        if target_type != TargetType::Class {
            return;
        }
        for node in nodes {
            let Some(class) = self.project.class(node).cloned() else {
                continue;
            };
            for field in &class.fields {
                if !predicates
                    .iter()
                    .all(|predicate| matches_field_predicate(field, predicate))
                {
                    self.push_field(rule, &class, field, "field does not satisfy allFields");
                }
            }
        }
    }

    fn evaluate_method_signature(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        type_name: &TypeValue,
        negated: bool,
    ) {
        if target_type != TargetType::Method {
            return;
        }
        for node in nodes {
            let Some(method) = self.method(node).cloned() else {
                continue;
            };
            let matches = method
                .return_type
                .as_deref()
                .is_some_and(|return_type| type_matches(return_type, type_name));
            if matches == negated {
                let class = self.project.class(&method.owner).cloned();
                if let Some(class) = class {
                    self.push_method(
                        rule,
                        &class,
                        &method,
                        if negated {
                            "return type must not match".to_string()
                        } else {
                            format!("return type must be {}", type_name.0)
                        },
                    );
                }
            }
        }
    }

    fn evaluate_method_signature_not(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        type_names: &[TypeValue],
    ) {
        if target_type != TargetType::Method {
            return;
        }
        for node in nodes {
            let Some(method) = self.method(node).cloned() else {
                continue;
            };
            let bad = type_names.iter().any(|type_name| {
                method
                    .return_type
                    .as_deref()
                    .is_some_and(|return_type| type_matches(return_type, type_name))
            });
            if bad && let Some(class) = self.project.class(&method.owner).cloned() {
                self.push_method(rule, &class, &method, "return type is forbidden");
            }
        }
    }

    fn evaluate_all_params(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        predicates: &[crate::model::ParamPredicate],
    ) {
        if target_type != TargetType::Method {
            return;
        }
        for node in nodes {
            let Some(method) = self.method(node).cloned() else {
                continue;
            };
            for parameter in &method.parameters {
                if !predicates
                    .iter()
                    .all(|predicate| matches_param_predicate(parameter, predicate))
                    && let Some(class) = self.project.class(&method.owner).cloned()
                {
                    self.push_method(
                        rule,
                        &class,
                        &method,
                        "parameter does not satisfy allParams",
                    );
                }
            }
        }
    }

    fn evaluate_visibility(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        visibility: Visibility,
    ) {
        if target_type != TargetType::Method {
            return;
        }
        for node in nodes {
            let Some(method) = self.method(node).cloned() else {
                continue;
            };
            let expected = match visibility {
                Visibility::Public => "public",
                Visibility::Protected => "protected",
                Visibility::Private => "private",
                Visibility::PackagePrivate => "",
            };
            let actual = if method.modifiers.iter().any(|modifier| modifier == "public") {
                "public"
            } else if method
                .modifiers
                .iter()
                .any(|modifier| modifier == "protected")
            {
                "protected"
            } else if method
                .modifiers
                .iter()
                .any(|modifier| modifier == "private")
            {
                "private"
            } else {
                ""
            };
            if actual != expected
                && let Some(class) = self.project.class(&method.owner).cloned()
            {
                self.push_method(rule, &class, &method, "method visibility does not match");
            }
        }
    }

    fn evaluate_children(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        children: &[String],
    ) {
        if target_type != TargetType::Module {
            return;
        }
        let allowed: BTreeSet<String> = children
            .iter()
            .flat_map(|name| {
                self.config
                    .targets
                    .get(name)
                    .map_or_else(BTreeSet::new, |target| self.evaluate_target(target))
            })
            .collect();
        for node in nodes {
            let Some(module) = self.project.modules.get(node) else {
                continue;
            };
            for child in &module.children {
                if !allowed.contains(child) {
                    self.push_root(
                        rule,
                        format!("module `{node}` has an unexpected child `{child}`"),
                    );
                }
            }
        }
    }

    fn evaluate_counts(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        counts: &BTreeMap<String, Count>,
    ) {
        if target_type != TargetType::Module {
            return;
        }
        for node in nodes {
            let Some(module) = self.project.modules.get(node) else {
                continue;
            };
            for (name, count) in counts {
                let Some(target) = self.config.targets.get(name) else {
                    continue;
                };
                let set = self.evaluate_target(target);
                let actual = module
                    .children
                    .iter()
                    .filter(|child| set.contains(*child))
                    .count();
                if !count_matches(*count, actual) {
                    self.push_root(
                        rule,
                        format!("module `{node}` has {actual} `{name}` children"),
                    );
                }
            }
        }
    }

    fn evaluate_acyclic(
        &mut self,
        rule: &Rule,
        target_type: TargetType,
        nodes: &BTreeSet<String>,
        acyclic: bool,
    ) {
        if target_type != TargetType::Module || !acyclic {
            return;
        }
        let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for edge in &self.project.edges {
            let Some(source) = self.project.class(&edge.from) else {
                continue;
            };
            let Some(target) = self.project.class(&edge.to) else {
                continue;
            };
            if source.module != target.module {
                adjacency
                    .entry(source.module.clone())
                    .or_default()
                    .insert(target.module.clone());
            }
        }
        let mut visited = BTreeSet::new();
        let mut stack = Vec::new();
        for node in nodes {
            self.detect_cycle(node, &adjacency, &mut visited, &mut stack, rule);
        }
    }

    fn detect_cycle(
        &mut self,
        node: &str,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
        visited: &mut BTreeSet<String>,
        stack: &mut Vec<String>,
        rule: &Rule,
    ) {
        if stack.iter().any(|item| item == node) {
            self.push_root(rule, format!("module cycle contains `{node}`"));
            return;
        }
        if !visited.insert(node.to_string()) {
            return;
        }
        stack.push(node.to_string());
        if let Some(next) = adjacency.get(node) {
            for child in next {
                self.detect_cycle(child, adjacency, visited, stack, rule);
            }
        }
        stack.pop();
    }

    fn push_root(&mut self, rule: &Rule, message: impl Into<String>) {
        self.findings.push(ArchFinding {
            path: self.project.root.clone(),
            span: Span::empty(0),
            rule: rule.id.clone(),
            severity: severity(rule.severity),
            message: message.into(),
            help: rule.because.clone(),
        });
    }

    fn push_node(&mut self, rule: &Rule, target_type: TargetType, node: &str, message: String) {
        let (path, span) = self.node_location(target_type, node);
        if Self::is_excepted(rule, node) {
            return;
        }
        self.findings.push(ArchFinding {
            path,
            span,
            rule: rule.id.clone(),
            severity: severity(rule.severity),
            message,
            help: rule.because.clone(),
        });
    }

    fn push_edge(&mut self, rule: &Rule, edge: &Edge, message: String) {
        if Self::is_excepted(rule, &edge.from) {
            return;
        }
        self.findings.push(ArchFinding {
            path: edge.path.clone(),
            span: edge.span,
            rule: rule.id.clone(),
            severity: severity(rule.severity),
            message,
            help: rule.because.clone(),
        });
    }

    fn push_method(
        &mut self,
        rule: &Rule,
        class: &ClassFact,
        method: &MethodFact,
        message: impl Into<String>,
    ) {
        if Self::is_excepted(rule, &class.fqn) {
            return;
        }
        let (path, span) = self.class_location(&class.fqn, method.span);
        self.findings.push(ArchFinding {
            path,
            span,
            rule: rule.id.clone(),
            severity: severity(rule.severity),
            message: message.into(),
            help: rule.because.clone(),
        });
    }

    fn push_field(
        &mut self,
        rule: &Rule,
        class: &ClassFact,
        field: &FieldFact,
        message: impl Into<String>,
    ) {
        if Self::is_excepted(rule, &class.fqn) {
            return;
        }
        let (path, span) = self.class_location(&class.fqn, field.span);
        self.findings.push(ArchFinding {
            path,
            span,
            rule: rule.id.clone(),
            severity: severity(rule.severity),
            message: message.into(),
            help: rule.because.clone(),
        });
    }

    fn node_location(&self, target_type: TargetType, node: &str) -> (PathBuf, Span) {
        match target_type {
            TargetType::Class => self.class_location(node, Span::empty(0)),
            TargetType::Method => self.method(node).map_or_else(
                || (self.project.root.clone(), Span::empty(0)),
                |method| self.class_location(&method.owner, method.span),
            ),
            TargetType::Package => self
                .project
                .classes
                .values()
                .find(|class| class.package.as_deref().unwrap_or("(default)") == node)
                .map_or_else(
                    || (self.project.root.clone(), Span::empty(0)),
                    |class| self.class_location(&class.fqn, class.span),
                ),
            TargetType::Module => self
                .project
                .classes
                .values()
                .find(|class| class.module == node)
                .map_or_else(
                    || (self.project.root.clone(), Span::empty(0)),
                    |class| self.class_location(&class.fqn, class.span),
                ),
        }
    }

    fn class_location(&self, class: &str, span: Span) -> (PathBuf, Span) {
        self.project.path_for_class(class).map_or_else(
            || (self.project.root.clone(), span),
            |path| (path.to_path_buf(), span),
        )
    }

    fn is_excepted(rule: &Rule, node: &str) -> bool {
        rule.except.iter().any(|excluded| excluded == node)
    }
}

fn severity(severity: ArchSeverity) -> Severity {
    match severity {
        ArchSeverity::Error => Severity::Error,
        ArchSeverity::Warn => Severity::Warning,
    }
}

fn ast_kind(kind: ClassKind) -> groovylint_syntax::ast::ClassKind {
    match kind {
        ClassKind::Class => groovylint_syntax::ast::ClassKind::Class,
        ClassKind::Interface => groovylint_syntax::ast::ClassKind::Interface,
        ClassKind::Trait => groovylint_syntax::ast::ClassKind::Trait,
        ClassKind::Enum => groovylint_syntax::ast::ClassKind::Enum,
        ClassKind::Annotation => groovylint_syntax::ast::ClassKind::Annotation,
        ClassKind::Record => groovylint_syntax::ast::ClassKind::Record,
    }
}

fn modifier_name(modifier: crate::model::Modifier) -> String {
    match modifier {
        crate::model::Modifier::Static => "static",
        crate::model::Modifier::Public => "public",
        crate::model::Modifier::Protected => "protected",
        crate::model::Modifier::Private => "private",
        crate::model::Modifier::Abstract => "abstract",
        crate::model::Modifier::Final => "final",
    }
    .to_string()
}

fn matches_name(patterns: &PatternList, name: &str) -> bool {
    patterns.patterns.iter().any(|pattern| glob(pattern, name))
}

fn matches_regex(pattern: &str, name: &str) -> bool {
    Regex::new(pattern).is_ok_and(|regex| regex.is_match(name))
}

fn glob(pattern: &str, value: &str) -> bool {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^:]*");
                }
            }
            '?' => regex.push('.'),
            '{' => {
                for next in chars.by_ref() {
                    if next == '}' {
                        break;
                    }
                }
                regex.push_str("[^:]*");
            }
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    Regex::new(&regex).is_ok_and(|regex| regex.is_match(value))
}

fn is_platform_dependency(name: &str) -> bool {
    [
        "java.",
        "javax.",
        "jakarta.",
        "groovy.",
        "org.codehaus.groovy.",
        "kotlin.",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

fn type_matches(written: &str, expected: &TypeValue) -> bool {
    written == expected.0
        || written.ends_with(&format!(".{}", expected.0))
        || expected.0.ends_with(&format!(".{written}"))
}

fn has_annotations(annotations: &[AnnotationFact], expected: &[String]) -> bool {
    expected.iter().any(|expected| {
        annotations.iter().any(|annotation| {
            annotation.name == *expected || annotation.name.ends_with(&format!(".{expected}"))
        })
    })
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && !matches!(character, '\'' | '"'))
        .collect()
}

fn matches_member(
    kind: MemberKind,
    name: &str,
    annotations: &[AnnotationFact],
    modifiers: &[String],
    predicates: &[MemberPredicate],
) -> bool {
    predicates.iter().all(|predicate| match predicate {
        MemberPredicate::Name(patterns) => matches_name(patterns, name),
        MemberPredicate::NameMatches(pattern) => matches_regex(pattern, name),
        MemberPredicate::AnnotatedWith(expected) => has_annotations(annotations, expected),
        MemberPredicate::Modifiers(expected) => expected
            .iter()
            .all(|modifier| modifiers.contains(&modifier_name(*modifier))),
        MemberPredicate::Kind(expected) => *expected == kind,
        MemberPredicate::Not(inner) => !matches_member(
            kind,
            name,
            annotations,
            modifiers,
            std::slice::from_ref(inner.as_ref()),
        ),
    })
}

fn matches_field_predicate(field: &FieldFact, predicate: &FieldPredicate) -> bool {
    match predicate {
        FieldPredicate::Name(patterns) => matches_name(patterns, &field.name),
        FieldPredicate::NameMatches(pattern) => matches_regex(pattern, &field.name),
        FieldPredicate::AnnotatedWith(expected) => has_annotations(&field.annotations, expected),
        FieldPredicate::Modifiers(expected) => expected
            .iter()
            .all(|modifier| field.modifiers.contains(&modifier_name(*modifier))),
        FieldPredicate::Type(type_name) => field
            .type_name
            .as_deref()
            .is_some_and(|actual| type_matches(actual, type_name)),
    }
}

fn matches_param_predicate(
    parameter: &ParameterFact,
    predicate: &crate::model::ParamPredicate,
) -> bool {
    match predicate {
        crate::model::ParamPredicate::Name(patterns) => matches_name(patterns, &parameter.name),
        crate::model::ParamPredicate::NameMatches(pattern) => {
            matches_regex(pattern, &parameter.name)
        }
        crate::model::ParamPredicate::AnnotatedWith(expected) => {
            has_annotations(&parameter.annotations, expected)
        }
        crate::model::ParamPredicate::Type(type_name) => parameter
            .type_name
            .as_deref()
            .is_some_and(|actual| type_matches(actual, type_name)),
        crate::model::ParamPredicate::Not(inner) => !matches_param_predicate(parameter, inner),
    }
}

fn count_matches(count: Count, actual: usize) -> bool {
    match count {
        Count::AtLeast(minimum) => actual >= minimum,
        Count::AtMost(maximum) => actual <= maximum,
        Count::Any => true,
        Count::Some => actual > 0,
    }
}
