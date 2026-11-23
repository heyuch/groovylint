//! Class design rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{Item, MethodDecl};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};

fn is_object_param(method: &MethodDecl) -> bool {
    method.params.len() == 1
        && method.params[0]
            .type_name
            .as_deref()
            .is_some_and(|name| matches!(name, "Object" | "java.lang.Object"))
}

fn methods(class: &groovylint_syntax::ast::ClassDecl) -> impl Iterator<Item = &MethodDecl> {
    class.members.iter().filter_map(|member| match member {
        Item::Method(method) => Some(method),
        _ => None,
    })
}

/// Requires `equals` and `hashCode` to be overridden together.
pub struct EqualsAndHashCodeRule;

impl Rule for EqualsAndHashCodeRule {}

impl AstRule for EqualsAndHashCodeRule {
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
        let equals =
            methods(class).any(|method| method.name == "equals" && is_object_param(method));
        let hash_code =
            methods(class).any(|method| method.name == "hashCode" && method.params.is_empty());
        if equals && !hash_code {
            sink.report(
                class.span,
                format!(
                    "The class {} defines equals(Object) but not hashCode()",
                    class.name
                ),
            );
        } else if hash_code && !equals {
            sink.report(
                class.span,
                format!(
                    "The class {} defines hashCode() but not equals(Object)",
                    class.name
                ),
            );
        }
    }
}

/// Detects overloaded, non-Object `equals` methods.
pub struct EqualsOverloadedRule;

impl Rule for EqualsOverloadedRule {}

impl AstRule for EqualsOverloadedRule {
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
        if method.name == "equals" && !is_object_param(method) {
            sink.report(
                method.span,
                "The class overloads the equals method, it does not override it.",
            );
        }
    }
}

/// Descriptors for class design rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "EqualsAndHashCode",
                "org.codenarc.rule.basic.EqualsAndHashCodeRule",
                Category::Basic,
                Severity::Warning,
                "Checks that equals and hashCode are overridden together.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EqualsAndHashCodeRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EqualsOverloaded",
                "org.codenarc.rule.basic.EqualsOverloadedRule",
                Category::Basic,
                Severity::Warning,
                "Checks for overloaded equals methods.",
            ),
            || RuleInstanceImpl::Ast(Box::new(EqualsOverloadedRule)),
        ),
    ]
}
