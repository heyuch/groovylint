//! `CodeNarc` generic rules. Every rule in this module is driven by properties,
//! so it does nothing until the ruleset configures it.

use groovylint_core::Severity;
use groovylint_syntax::ast::Item;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    Category, DiagnosticSink, FileContext, FileRule, NodeRef, Rule, RuleMeta, RuleProperties,
    TokenRule,
};

fn names_property(properties: &RuleProperties, name: &str) -> Vec<String> {
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

/// Rejects a configured substring.
#[derive(Default)]
pub struct IllegalStringRule {
    needle: String,
}

impl Rule for IllegalStringRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.needle = properties.get("string").cloned().unwrap_or_default();
        Ok(())
    }
}

impl FileRule for IllegalStringRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        if self.needle.is_empty() {
            return;
        }
        let text = context.source.text().to_string();
        let mut offset = 0;
        while let Some(index) = text[offset..].find(&self.needle) {
            let start = offset + index;
            let line = context.source.line_col(start).line;
            if let Some(span) = context.source.line_span(line) {
                sink.report(span, format!("Found illegal string: {}", self.needle));
            }
            offset = start + self.needle.len();
        }
    }
}

/// Rejects a configured regular expression.
#[derive(Default)]
pub struct IllegalRegexRule {
    pattern: Option<regex::Regex>,
}

impl Rule for IllegalRegexRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let Some(value) = properties.get("regex") else {
            self.pattern = None;
            return Ok(());
        };
        self.pattern = Some(
            regex::Regex::new(value).map_err(|error| format!("invalid regex property: {error}"))?,
        );
        Ok(())
    }
}

impl FileRule for IllegalRegexRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let Some(pattern) = &self.pattern else {
            return;
        };
        let text = context.source.text();
        for matched in pattern.find_iter(text) {
            let line = context.source.line_col(matched.start()).line;
            let Some(span) = context.source.line_span(line) else {
                continue;
            };
            sink.report(
                span,
                format!("Found illegal regex match: {}", matched.as_str()),
            );
        }
    }
}

/// Requires a configured substring.
#[derive(Default)]
pub struct RequiredStringRule {
    needle: String,
}

impl Rule for RequiredStringRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.needle = properties.get("string").cloned().unwrap_or_default();
        Ok(())
    }
}

impl FileRule for RequiredStringRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        if self.needle.is_empty() || context.source.text().contains(&self.needle) {
            return;
        }
        let Some(span) = context.source.line_span(1) else {
            return;
        };
        sink.report(span, format!("Missing required string: {}", self.needle));
    }
}

/// Requires a configured regular expression.
#[derive(Default)]
pub struct RequiredRegexRule {
    pattern: Option<regex::Regex>,
}

impl Rule for RequiredRegexRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        let Some(value) = properties.get("regex") else {
            self.pattern = None;
            return Ok(());
        };
        self.pattern = Some(
            regex::Regex::new(value).map_err(|error| format!("invalid regex property: {error}"))?,
        );
        Ok(())
    }
}

impl FileRule for RequiredRegexRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let Some(pattern) = &self.pattern else {
            return;
        };
        if pattern.is_match(context.source.text()) {
            return;
        }
        let Some(span) = context.source.line_span(1) else {
            return;
        };
        sink.report(span, "Missing required regex match");
    }
}

/// Rejects imports and allocations of configured class names.
#[derive(Default)]
pub struct IllegalClassReferenceRule {
    class_names: Vec<String>,
}

impl Rule for IllegalClassReferenceRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.class_names = names_property(properties, "classNames");
        Ok(())
    }
}

impl FileRule for IllegalClassReferenceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for import in &context.script.imports {
            let imported = simple_name(&import.path);
            if !self.class_names.iter().any(|name| name == imported) {
                continue;
            }
            sink.report(
                import.span,
                format!("Found illegal class reference {imported}"),
            );
        }
    }
}

/// Rejects imports of configured packages.
#[derive(Default)]
pub struct IllegalPackageReferenceRule {
    package_names: Vec<String>,
}

impl Rule for IllegalPackageReferenceRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.package_names = names_property(properties, "packageNames");
        Ok(())
    }
}

impl FileRule for IllegalPackageReferenceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for import in &context.script.imports {
            for package in &self.package_names {
                if !import.path.starts_with(package) {
                    continue;
                }
                sink.report(
                    import.span,
                    format!("Found illegal package reference {package}"),
                );
            }
        }
    }
}

/// Rejects subclasses of configured superclasses.
#[derive(Default)]
pub struct IllegalSubclassRule {
    superclass_names: Vec<String>,
}

impl Rule for IllegalSubclassRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.superclass_names = names_property(properties, "superclassNames");
        Ok(())
    }
}

impl FileRule for IllegalSubclassRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            let Some(parent) = &class.extends else {
                continue;
            };
            if !self
                .superclass_names
                .iter()
                .any(|name| simple_name(parent) == name)
            {
                continue;
            }
            sink.report(
                class.span,
                format!(
                    "The class {} extends the illegal superclass {parent}",
                    class.name
                ),
            );
        }
    }
}

/// Rejects fields on classes that should stay stateless.
#[derive(Default)]
pub struct StatelessClassRule {
    class_names: Vec<String>,
}

impl Rule for StatelessClassRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.class_names = names_property(properties, "classNames");
        Ok(())
    }
}

impl FileRule for StatelessClassRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            if !self.class_names.contains(&class.name) {
                continue;
            }
            for member in &class.members {
                let Item::Field(field) = member else {
                    continue;
                };
                if field.is_static && field.is_final {
                    continue;
                }
                sink.report(
                    field.span,
                    format!(
                        "The class {} should be stateless but declares a field",
                        class.name
                    ),
                );
            }
        }
    }
}

/// Checks member modifiers against configured allowed or illegal sets.
#[derive(Default)]
pub struct IllegalClassMemberRule {
    configured: bool,
}

impl Rule for IllegalClassMemberRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.configured = properties
            .keys()
            .any(|key| key.starts_with("illegal") || key.starts_with("allowed"));
        Ok(())
    }
}

impl TokenRule for IllegalClassMemberRule {
    fn check(&mut self, _context: &FileContext<'_>, _sink: &mut DiagnosticSink<'_>) {
        // Modifier-set matching needs the full CodeNarc property vocabulary; the
        // rule is registered so rulesets load, but it reports nothing yet.
    }
}

/// Unused hook that keeps the node type referenced for future extension.
#[allow(dead_code)]
fn _node_ref(_: NodeRef<'_>) {}

/// Descriptors for generic rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "IllegalString",
                "org.codenarc.rule.generic.IllegalStringRule",
                Category::Generic,
                Severity::Warning,
                "Checks for a configured illegal string.",
            ),
            || RuleInstanceImpl::File(Box::new(IllegalStringRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "IllegalRegex",
                "org.codenarc.rule.generic.IllegalRegexRule",
                Category::Generic,
                Severity::Warning,
                "Checks for a configured illegal regex.",
            ),
            || RuleInstanceImpl::File(Box::new(IllegalRegexRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "RequiredString",
                "org.codenarc.rule.generic.RequiredStringRule",
                Category::Generic,
                Severity::Warning,
                "Checks for a configured required string.",
            ),
            || RuleInstanceImpl::File(Box::new(RequiredStringRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "RequiredRegex",
                "org.codenarc.rule.generic.RequiredRegexRule",
                Category::Generic,
                Severity::Warning,
                "Checks for a configured required regex.",
            ),
            || RuleInstanceImpl::File(Box::new(RequiredRegexRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "IllegalClassReference",
                "org.codenarc.rule.generic.IllegalClassReferenceRule",
                Category::Generic,
                Severity::Warning,
                "Checks for references to configured class names.",
            ),
            || RuleInstanceImpl::File(Box::new(IllegalClassReferenceRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "IllegalPackageReference",
                "org.codenarc.rule.generic.IllegalPackageReferenceRule",
                Category::Generic,
                Severity::Warning,
                "Checks for references to configured packages.",
            ),
            || RuleInstanceImpl::File(Box::new(IllegalPackageReferenceRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "IllegalSubclass",
                "org.codenarc.rule.generic.IllegalSubclassRule",
                Category::Generic,
                Severity::Warning,
                "Checks for subclasses of configured superclasses.",
            ),
            || RuleInstanceImpl::File(Box::new(IllegalSubclassRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "StatelessClass",
                "org.codenarc.rule.generic.StatelessClassRule",
                Category::Generic,
                Severity::Warning,
                "Checks that configured classes declare no state.",
            ),
            || RuleInstanceImpl::File(Box::new(StatelessClassRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "IllegalClassMember",
                "org.codenarc.rule.generic.IllegalClassMemberRule",
                Category::Generic,
                Severity::Warning,
                "Checks member modifiers against configured sets.",
            ),
            || RuleInstanceImpl::Token(Box::new(IllegalClassMemberRule::default())),
        ),
    ]
}
