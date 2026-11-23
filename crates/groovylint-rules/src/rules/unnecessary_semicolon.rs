//! Token-level `CodeNarc` rule for line-ending semicolons.

use groovylint_core::Severity;
use groovylint_syntax::token::TokenKind;

use crate::rule::{Category, DiagnosticSink, FileContext, Rule, TokenRule};

/// Flags semicolons used as line terminators, matching `CodeNarc`'s
/// `UnnecessarySemicolon` behavior. Semicolons inside a `for` header are
/// ignored.
pub struct UnnecessarySemicolonRule;

impl Rule for UnnecessarySemicolonRule {}

impl TokenRule for UnnecessarySemicolonRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut paren_depth = 0_usize;
        let mut for_header_depth = None;
        let mut pending_for = false;

        for (index, token) in context.tokens.iter().enumerate() {
            match token.kind {
                TokenKind::For => pending_for = true,
                TokenKind::LeftParen => {
                    paren_depth += 1;
                    if pending_for {
                        for_header_depth = Some(paren_depth);
                        pending_for = false;
                    }
                }
                TokenKind::RightParen => {
                    if for_header_depth == Some(paren_depth) {
                        for_header_depth = None;
                    }
                    paren_depth = paren_depth.saturating_sub(1);
                }
                TokenKind::Semicolon if for_header_depth.is_none() => {
                    let next = context
                        .tokens
                        .get(index + 1)
                        .map_or(TokenKind::Eof, |next| next.kind);
                    if matches!(next, TokenKind::Newline | TokenKind::Eof) {
                        sink.report_with_help(
                            token.span,
                            "Semicolons as line endings can be removed safely",
                            "Remove the trailing semicolon.",
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// Metadata key for the rule.
pub const META: crate::rule::RuleMeta = crate::rule::RuleMeta::new(
    "UnnecessarySemicolon",
    "org.codenarc.rule.unnecessary.UnnecessarySemicolonRule",
    Category::Unnecessary,
    Severity::Info,
    "Checks for unnecessary line-ending semicolons.",
);
