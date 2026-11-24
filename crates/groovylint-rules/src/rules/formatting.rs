//! `CodeNarc` formatting rules that only need the source lines.

use groovylint_core::Severity;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{Category, DiagnosticSink, FileContext, FileRule, Rule, RuleMeta};

/// Flags lines that end in whitespace.
pub struct TrailingWhitespaceRule;

impl Rule for TrailingWhitespaceRule {}

impl FileRule for TrailingWhitespaceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for line in 1..=context.source.line_count() {
            let Some(text) = context.source.line_text(line) else {
                continue;
            };
            if !text.ends_with([' ', '\t']) {
                continue;
            }
            let Some(span) = context.source.line_span(line) else {
                continue;
            };
            sink.report(span, "Line ends with whitespace characters");
        }
    }
}

/// Descriptors for formatting rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![descriptor(
        RuleMeta::new(
            "TrailingWhitespace",
            "org.codenarc.rule.formatting.TrailingWhitespaceRule",
            Category::Formatting,
            Severity::Info,
            "Checks for lines that end with whitespace characters.",
        ),
        || RuleInstanceImpl::File(Box::new(TrailingWhitespaceRule)),
    )]
}
