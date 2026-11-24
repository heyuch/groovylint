//! `CodeNarc` comment and Javadoc rules.
//!
//! `CodeNarc` implements these on the raw source text, so the same approach is
//! used here: no lexer, just line scanning.

use groovylint_core::Severity;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};

/// One `/** ... */` block whose first line is `/**` on its own.
struct Javadoc {
    /// 1-based line of `/ **`.
    start: usize,
    /// Content lines between `/**` and `*/`, as `(line number, text after '*')`.
    lines: Vec<(usize, String)>,
    /// 1-based line of `*/`.
    end: usize,
}

fn javadocs(context: &FileContext<'_>) -> Vec<Javadoc> {
    let count = context.source.line_count();
    let mut blocks = Vec::new();
    let mut line = 1;

    while line <= count {
        let Some(text) = context.source.line_text(line) else {
            break;
        };
        if text.trim() != "/**" {
            line += 1;
            continue;
        }

        let start = line;
        let mut lines = Vec::new();
        let mut cursor = line + 1;
        let mut end = None;
        while cursor <= count {
            let Some(next) = context.source.line_text(cursor) else {
                break;
            };
            let trimmed = next.trim_start();
            if !trimmed.starts_with('*') {
                break;
            }
            if let Some(content) = trimmed.strip_prefix("*/") {
                let _ = content;
                end = Some(cursor);
                cursor += 1;
                break;
            }
            let after = trimmed[1..].trim();
            lines.push((cursor, after.to_string()));
            cursor += 1;
        }

        let Some(end) = end else {
            break;
        };
        blocks.push(Javadoc { start, lines, end });
        line = cursor;
    }

    blocks
}

fn report_at_line(
    context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    line: usize,
    message: &str,
) {
    let Some(span) = context.source.line_span(line) else {
        return;
    };
    sink.report(span, message.to_string());
}

/// Reports the first javadoc line of a block whose content satisfies `predicate`.
fn report_first_tag(
    context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    predicate: impl Fn(&str) -> bool,
    message: &str,
) {
    for block in javadocs(context) {
        let Some((line, _)) = block.lines.iter().find(|(_, text)| predicate(text)) else {
            continue;
        };
        report_at_line(context, sink, *line, message);
    }
}

fn tag_body<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(tag)?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
}

/// Requires a Javadoc comment on the main class of the file.
pub struct ClassJavadocRule;

impl Rule for ClassJavadocRule {}

impl AstRule for ClassJavadocRule {
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
        if class.kind == groovylint_syntax::ast::ClassKind::Enum {
            return;
        }
        let path = context.source.path().to_string_lossy();
        let Some(file_name) = path.rsplit('/').next() else {
            return;
        };
        let Some(stem) = file_name.strip_suffix(".groovy") else {
            return;
        };
        if stem != class.name {
            return;
        }

        // `CodeNarc` reports the first line of the declaration, annotations
        // included.
        let declaration_line =
            first_annotation_line(context, context.source.line_col(class.span.start).line);
        let Some(previous) = previous_content_line(context, declaration_line) else {
            return;
        };
        let Some(text) = context.source.line_text(previous) else {
            return;
        };
        if text.trim_start().starts_with("/**") {
            return;
        }
        report_at_line(
            context,
            sink,
            declaration_line,
            &format!("Class {} missing Javadoc", class.name),
        );
    }
}

/// Returns the first line of a run of annotations ending at `line`.
fn first_annotation_line(context: &FileContext<'_>, line: usize) -> usize {
    let mut first = line;
    while first > 1 {
        let Some(text) = context.source.line_text(first - 1) else {
            break;
        };
        if !text.trim_start().starts_with('@') {
            break;
        }
        first -= 1;
    }
    first
}

/// Walks back over blank lines, line comments, annotations and Javadoc lines.
fn previous_content_line(context: &FileContext<'_>, line: usize) -> Option<usize> {
    let mut cursor = line;
    while cursor > 1 {
        cursor -= 1;
        let text = context.source.line_text(cursor)?;
        let trimmed = text.trim();
        let is_skippable = trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('@')
            || trimmed.starts_with('*');
        if !is_skippable {
            return Some(cursor);
        }
    }
    None
}

/// Flags Javadoc blocks with more than one consecutive empty line.
pub struct JavadocConsecutiveEmptyLinesRule;

impl Rule for JavadocConsecutiveEmptyLinesRule {}

impl FileRule for JavadocConsecutiveEmptyLinesRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for block in javadocs(context) {
            for window in block.lines.windows(2) {
                if window[0].1.is_empty() && window[1].1.is_empty() {
                    report_at_line(
                        context,
                        sink,
                        window[1].0,
                        "The javadoc contains consecutive empty lines",
                    );
                    break;
                }
            }
        }
    }
}

/// Flags Javadoc blocks whose first content line is empty.
pub struct JavadocEmptyFirstLineRule;

impl Rule for JavadocEmptyFirstLineRule {}

impl FileRule for JavadocEmptyFirstLineRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for block in javadocs(context) {
            if block.lines.first().is_some_and(|(_, text)| text.is_empty()) {
                report_at_line(
                    context,
                    sink,
                    block.start,
                    "The first line of the javadoc is empty",
                );
            }
        }
    }
}

/// Flags Javadoc blocks whose last content line before `*/` is empty.
pub struct JavadocEmptyLastLineRule;

impl Rule for JavadocEmptyLastLineRule {}

impl FileRule for JavadocEmptyLastLineRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for block in javadocs(context) {
            if block.lines.last().is_some_and(|(_, text)| text.is_empty()) {
                report_at_line(
                    context,
                    sink,
                    block.end - 1,
                    "The last line of the javadoc is empty",
                );
            }
        }
    }
}

macro_rules! empty_tag_rule {
    ($struct_name:ident, $rule:literal, $class:literal, $tag:literal, $description:literal) => {
        #[doc = $description]
        pub struct $struct_name;

        impl Rule for $struct_name {}

        impl FileRule for $struct_name {
            fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
                report_first_tag(
                    context,
                    sink,
                    |text| tag_body(text, $tag).is_some_and(|body| body.is_empty()),
                    concat!("The javadoc ", $tag, " tag is empty"),
                );
            }
        }
    };
}

empty_tag_rule!(
    JavadocEmptyAuthorTagRule,
    "JavadocEmptyAuthorTag",
    "org.codenarc.rule.comments.JavadocEmptyAuthorTagRule",
    "@author",
    "Flags empty `@author` tags."
);
empty_tag_rule!(
    JavadocEmptyExceptionTagRule,
    "JavadocEmptyExceptionTag",
    "org.codenarc.rule.comments.JavadocEmptyExceptionTagRule",
    "@exception",
    "Flags empty `@exception` tags."
);
empty_tag_rule!(
    JavadocEmptyParamTagRule,
    "JavadocEmptyParamTag",
    "org.codenarc.rule.comments.JavadocEmptyParamTagRule",
    "@param",
    "Flags empty `@param` tags."
);
empty_tag_rule!(
    JavadocEmptyReturnTagRule,
    "JavadocEmptyReturnTag",
    "org.codenarc.rule.comments.JavadocEmptyReturnTagRule",
    "@return",
    "Flags empty `@return` tags."
);
empty_tag_rule!(
    JavadocEmptySeeTagRule,
    "JavadocEmptySeeTag",
    "org.codenarc.rule.comments.JavadocEmptySeeTagRule",
    "@see",
    "Flags empty `@see` tags."
);
empty_tag_rule!(
    JavadocEmptySinceTagRule,
    "JavadocEmptySinceTag",
    "org.codenarc.rule.comments.JavadocEmptySinceTagRule",
    "@since",
    "Flags empty `@since` tags."
);
empty_tag_rule!(
    JavadocEmptyThrowsTagRule,
    "JavadocEmptyThrowsTag",
    "org.codenarc.rule.comments.JavadocEmptyThrowsTagRule",
    "@throws",
    "Flags empty `@throws` tags."
);
empty_tag_rule!(
    JavadocEmptyVersionTagRule,
    "JavadocEmptyVersionTag",
    "org.codenarc.rule.comments.JavadocEmptyVersionTagRule",
    "@version",
    "Flags empty `@version` tags."
);

macro_rules! missing_description_rule {
    ($struct_name:ident, $rule:literal, $class:literal, $tag:literal, $has_argument:literal, $description:literal) => {
        #[doc = $description]
        pub struct $struct_name;

        impl Rule for $struct_name {}

        impl FileRule for $struct_name {
            fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
                report_first_tag(
                    context,
                    sink,
                    |text| {
                        let Some(body) = tag_body(text, $tag) else {
                            return false;
                        };
                        if body.is_empty() {
                            return false;
                        }
                        // `@param name` and `@throws Type` need an argument before
                        // the description; `@return` does not.
                        let rest = if $has_argument {
                            match body.split_once(char::is_whitespace) {
                                Some((_, rest)) => rest.trim(),
                                None => return true,
                            }
                        } else {
                            body
                        };
                        rest.is_empty()
                    },
                    concat!("The description for the javadoc ", $tag, " tag is missing"),
                );
            }
        }
    };
}

missing_description_rule!(
    JavadocMissingParamDescriptionRule,
    "JavadocMissingParamDescription",
    "org.codenarc.rule.comments.JavadocMissingParamDescriptionRule",
    "@param",
    true,
    "Flags `@param` tags without a description."
);
missing_description_rule!(
    JavadocMissingExceptionDescriptionRule,
    "JavadocMissingExceptionDescription",
    "org.codenarc.rule.comments.JavadocMissingExceptionDescriptionRule",
    "@exception",
    true,
    "Flags `@exception` tags without a description."
);
missing_description_rule!(
    JavadocMissingThrowsDescriptionRule,
    "JavadocMissingThrowsDescription",
    "org.codenarc.rule.comments.JavadocMissingThrowsDescriptionRule",
    "@throws",
    true,
    "Flags `@throws` tags without a description."
);

/// Requires whitespace after a comment delimiter.
pub struct SpaceAfterCommentDelimiterRule;

impl Rule for SpaceAfterCommentDelimiterRule {}

impl FileRule for SpaceAfterCommentDelimiterRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let text = context.source.text();
        let bytes = text.as_bytes();
        let mut index = 0;
        while index + 1 < bytes.len() {
            if bytes[index] != b'/' {
                index += 1;
                continue;
            }
            let Some(next) = bytes.get(index + 1) else {
                break;
            };
            if *next != b'/' && *next != b'*' {
                index += 1;
                continue;
            }
            let delimiter_length = if *next == b'*' && bytes.get(index + 2) == Some(&b'*') {
                3
            } else {
                2
            };
            let after = bytes.get(index + delimiter_length).copied();
            let previous = index.checked_sub(1).and_then(|at| bytes.get(at)).copied();
            let is_word =
                |byte: Option<u8>| byte.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_');
            let skip_for_string = matches!(previous, Some(b':' | b'$')) && delimiter_length == 2;

            if is_word(after) && !skip_for_string {
                let line = context.source.line_col(index).line;
                let quoted = context
                    .source
                    .line_text(line)
                    .is_some_and(|text| text.contains('\'') || text.contains('"'));
                if !quoted {
                    report_at_line(
                        context,
                        sink,
                        line,
                        "The comment does not begin with space or whitespace",
                    );
                }
            }
            index += delimiter_length;
        }
    }
}

/// Requires whitespace before a comment delimiter.
pub struct SpaceBeforeCommentDelimiterRule;

impl Rule for SpaceBeforeCommentDelimiterRule {}

impl FileRule for SpaceBeforeCommentDelimiterRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let text = context.source.text();
        let bytes = text.as_bytes();
        let mut index = 1;
        while index + 1 < bytes.len() {
            if bytes[index] != b'/' || bytes[index - 1].is_ascii_whitespace() {
                index += 1;
                continue;
            }
            let Some(next) = bytes.get(index + 1) else {
                break;
            };
            let is_comment =
                *next == b'*' || (*next == b'/' && !matches!(bytes[index - 1], b':' | b'$'));
            if !is_comment {
                index += 1;
                continue;
            }
            let line = context.source.line_col(index).line;
            let skip = context
                .source
                .line_text(line)
                .is_some_and(|text| text.contains('\'') || text.contains('"'));
            if !skip {
                report_at_line(
                    context,
                    sink,
                    line,
                    "The comment is not preceded by a space or whitespace",
                );
            }
            index += 2;
        }
    }
}

/// Descriptors for the comment rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "ClassJavadoc",
                "org.codenarc.rule.comments.ClassJavadocRule",
                Category::Comments,
                Severity::Warning,
                "The main class of a file should have a Javadoc comment.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ClassJavadocRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocConsecutiveEmptyLines",
                "org.codenarc.rule.comments.JavadocConsecutiveEmptyLinesRule",
                Category::Comments,
                Severity::Info,
                "Javadoc comments should not contain consecutive empty lines.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocConsecutiveEmptyLinesRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyFirstLine",
                "org.codenarc.rule.comments.JavadocEmptyFirstLineRule",
                Category::Comments,
                Severity::Info,
                "The first line of a Javadoc comment should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyFirstLineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyLastLine",
                "org.codenarc.rule.comments.JavadocEmptyLastLineRule",
                Category::Comments,
                Severity::Info,
                "The last line of a Javadoc comment should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyLastLineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyAuthorTag",
                "org.codenarc.rule.comments.JavadocEmptyAuthorTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @author tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyAuthorTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyExceptionTag",
                "org.codenarc.rule.comments.JavadocEmptyExceptionTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @exception tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyExceptionTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyParamTag",
                "org.codenarc.rule.comments.JavadocEmptyParamTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @param tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyParamTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyReturnTag",
                "org.codenarc.rule.comments.JavadocEmptyReturnTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @return tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyReturnTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptySeeTag",
                "org.codenarc.rule.comments.JavadocEmptySeeTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @see tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptySeeTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptySinceTag",
                "org.codenarc.rule.comments.JavadocEmptySinceTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @since tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptySinceTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyThrowsTag",
                "org.codenarc.rule.comments.JavadocEmptyThrowsTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @throws tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyThrowsTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocEmptyVersionTag",
                "org.codenarc.rule.comments.JavadocEmptyVersionTagRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @version tags should not be empty.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocEmptyVersionTagRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocMissingParamDescription",
                "org.codenarc.rule.comments.JavadocMissingParamDescriptionRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @param tags should have a description.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocMissingParamDescriptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocMissingExceptionDescription",
                "org.codenarc.rule.comments.JavadocMissingExceptionDescriptionRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @exception tags should have a description.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocMissingExceptionDescriptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "JavadocMissingThrowsDescription",
                "org.codenarc.rule.comments.JavadocMissingThrowsDescriptionRule",
                Category::Comments,
                Severity::Info,
                "Javadoc @throws tags should have a description.",
            ),
            || RuleInstanceImpl::File(Box::new(JavadocMissingThrowsDescriptionRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterCommentDelimiter",
                "org.codenarc.rule.comments.SpaceAfterCommentDelimiterRule",
                Category::Comments,
                Severity::Info,
                "Comments should start with a space after the delimiter.",
            ),
            || RuleInstanceImpl::File(Box::new(SpaceAfterCommentDelimiterRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceBeforeCommentDelimiter",
                "org.codenarc.rule.comments.SpaceBeforeCommentDelimiterRule",
                Category::Comments,
                Severity::Info,
                "Comments should be preceded by a space.",
            ),
            || RuleInstanceImpl::File(Box::new(SpaceBeforeCommentDelimiterRule)),
        ),
    ]
}
