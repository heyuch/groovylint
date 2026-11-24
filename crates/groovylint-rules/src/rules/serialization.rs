//! `CodeNarc` serialization rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassDecl, ClassKind, FieldDecl, Item};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{Category, DiagnosticSink, FileContext, FileRule, Rule, RuleMeta};

fn is_serializable(class: &ClassDecl) -> bool {
    class
        .implements
        .iter()
        .any(|name| name.rsplit('.').next() == Some("Serializable"))
}

fn fields<'a>(class: &'a ClassDecl, name: &str) -> Vec<&'a FieldDecl> {
    class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Field(field) if field.name == name => Some(field),
            _ => None,
        })
        .collect()
}

fn top_level_classes<'a>(context: &'a FileContext<'a>) -> Vec<&'a ClassDecl> {
    context
        .script
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Class(class) => Some(class),
            _ => None,
        })
        .collect()
}

/// Checks the `serialVersionUID` field of serializable classes.
pub struct SerialVersionUIDRule;

impl Rule for SerialVersionUIDRule {}

impl FileRule for SerialVersionUIDRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in top_level_classes(context) {
            if !is_serializable(class) {
                continue;
            }
            for field in fields(class, "serialVersionUID") {
                if !field.is_private && !field.is_public {
                    sink.report(
                        field.span,
                        "The serialVersionUID field is a property. It should have an explicit visibility",
                    );
                }
                if !field.is_private {
                    sink.report(field.span, "serialVersionUID found that is not private");
                }
                if !field.is_static {
                    sink.report(field.span, "serialVersionUID found that is not static");
                }
                if !field.is_final {
                    sink.report(field.span, "serialVersionUID found that is not final");
                }
                let is_long = matches!(field.type_name.as_deref(), Some("long" | "Long"));
                if !is_long {
                    sink.report(
                        field.span,
                        format!(
                            "serialVersionUID found that is not long. Found: {}",
                            field.type_name.as_deref().unwrap_or("def")
                        ),
                    );
                }
            }
        }
    }
}

/// Checks the `serialPersistentFields` field of serializable classes.
pub struct SerialPersistentFieldsRule;

impl Rule for SerialPersistentFieldsRule {}

impl FileRule for SerialPersistentFieldsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in top_level_classes(context) {
            if !is_serializable(class) {
                continue;
            }
            for field in fields(class, "serialPersistentFields") {
                if field.is_private && field.is_static && field.is_final {
                    continue;
                }
                sink.report(
                    field.span,
                    "The class is Serializable and defines a field named serialPersistentFields which is not private, static, and final",
                );
            }
        }
    }
}

/// Flags serialization members that enums ignore.
pub struct EnumCustomSerializationIgnoredRule;

impl Rule for EnumCustomSerializationIgnoredRule {}

impl FileRule for EnumCustomSerializationIgnoredRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in top_level_classes(context) {
            if class.kind != ClassKind::Enum {
                continue;
            }
            for member in &class.members {
                match member {
                    Item::Method(method)
                        if method.name == "writeObject" || method.name == "writeReplace" =>
                    {
                        sink.report(
                            method.span,
                            format!("Serialization method {} is ignored for enums", method.name),
                        );
                    }
                    Item::Field(field)
                        if field.name == "serialVersionUID"
                            || field.name == "serialPersistentFields" =>
                    {
                        sink.report(
                            field.span,
                            format!("Serialization field {} is ignored for enums", field.name),
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Descriptors for serialization rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "SerialVersionUID",
                "org.codenarc.rule.serialization.SerialVersionUIDRule",
                Category::Serialization,
                Severity::Warning,
                "Checks that serialVersionUID is private, static and final long.",
            ),
            || RuleInstanceImpl::File(Box::new(SerialVersionUIDRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SerialPersistentFields",
                "org.codenarc.rule.serialization.SerialPersistentFieldsRule",
                Category::Serialization,
                Severity::Warning,
                "Checks that serialPersistentFields is private, static and final.",
            ),
            || RuleInstanceImpl::File(Box::new(SerialPersistentFieldsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "EnumCustomSerializationIgnored",
                "org.codenarc.rule.serialization.EnumCustomSerializationIgnoredRule",
                Category::Serialization,
                Severity::Warning,
                "Checks for serialization members on enums.",
            ),
            || RuleInstanceImpl::File(Box::new(EnumCustomSerializationIgnoredRule)),
        ),
    ]
}
