//! `CodeNarc` enhanced rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{Item, MethodDecl};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{Category, DiagnosticSink, FileContext, FileRule, Rule, RuleMeta};

fn has_annotation(annotations: &[String], name: &str) -> bool {
    annotations
        .iter()
        .any(|annotation| annotation == name || annotation.ends_with(&format!(".{name}")))
}

/// Flags `Object` method overrides that lack `@Override`.
pub struct MissingOverrideAnnotationRule;

impl Rule for MissingOverrideAnnotationRule {}

impl FileRule for MissingOverrideAnnotationRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            for member in &class.members {
                let Item::Method(method) = member else {
                    continue;
                };
                check_method(method, sink);
            }
        }
    }
}

/// Reports `equals`, `hashCode`, `toString`, `clone` and `finalize` overrides
/// that are missing `@Override`.
fn check_method(method: &MethodDecl, sink: &mut DiagnosticSink<'_>) {
    if has_annotation(&method.annotations, "Override") {
        return;
    }
    if method.is_static {
        return;
    }
    let overrides = match method.name.as_str() {
        "equals" => method.params.len() == 1 && takes_object(&method.params[0]),
        "hashCode" | "toString" | "clone" | "finalize" => method.params.is_empty(),
        _ => false,
    };
    if !overrides {
        return;
    }
    sink.report(
        method.span,
        format!(
            "Method '{}' is overriding a method in 'java.lang.Object' but is not annotated with @Override.",
            method.name
        ),
    );
}

fn takes_object(param: &groovylint_syntax::ast::Param) -> bool {
    match &param.type_name {
        None => true,
        Some(type_name) => matches!(type_name.as_str(), "Object" | "java.lang.Object"),
    }
}

/// Descriptors for enhanced rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![descriptor(
        RuleMeta::new(
            "MissingOverrideAnnotation",
            "org.codenarc.rule.enhanced.MissingOverrideAnnotationRule",
            Category::Enhanced,
            Severity::Info,
            "Checks for overrides without @Override.",
        ),
        || RuleInstanceImpl::File(Box::new(MissingOverrideAnnotationRule)),
    )]
}
