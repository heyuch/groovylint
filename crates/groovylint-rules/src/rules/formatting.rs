//! `CodeNarc` formatting rules.
//!
//! Most of these only look at source lines and brace positions, so they are
//! implemented on the raw text rather than the AST.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassDecl, ExprKind};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use groovylint_syntax::token::TokenKind;

use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta, TokenRule,
};

fn line_of(context: &FileContext<'_>, offset: usize) -> usize {
    context.source.line_col(offset).line
}

fn line_is_blank(context: &FileContext<'_>, line: usize) -> bool {
    context
        .source
        .line_text(line)
        .is_some_and(|text| text.trim().is_empty())
}

fn report_line(
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

/// Returns whether a class declaration fits on one line.
fn is_single_line_class(context: &FileContext<'_>, class: &ClassDecl) -> bool {
    line_of(context, class.span.start) == line_of(context, class.span.end)
}

/// Returns the line holding the opening brace of a class body.
fn class_brace_line(context: &FileContext<'_>, class: &ClassDecl) -> Option<usize> {
    let start = line_of(context, class.span.start);
    let end = line_of(context, class.span.end);
    for line in start..=end {
        if context.source.line_text(line)?.contains('{') {
            return Some(line);
        }
    }
    None
}

fn is_blank(context: &FileContext<'_>, line: usize) -> bool {
    context
        .source
        .line_text(line)
        .is_some_and(|text| text.trim().is_empty())
}

fn trimmed(context: &FileContext<'_>, line: usize) -> String {
    context
        .source
        .line_text(line)
        .map_or_default(|text| text.trim().to_string())
}

/// Requires a blank line before the closing brace of every class.
pub struct ClassEndsWithBlankLineRule;

impl Rule for ClassEndsWithBlankLineRule {}

impl AstRule for ClassEndsWithBlankLineRule {
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
        if is_single_line_class(context, class) {
            return;
        }
        let last = line_of(context, class.span.end);
        let before = last.saturating_sub(1);
        if trimmed(context, last) != "}" || !is_blank(context, before) {
            report_line(
                context,
                sink,
                last,
                "Class does not end with a blank line before the closing brace",
            );
        }
    }
}

/// Requires a blank line after the opening brace of every class.
pub struct ClassStartsWithBlankLineRule;

impl Rule for ClassStartsWithBlankLineRule {}

impl AstRule for ClassStartsWithBlankLineRule {
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
        if is_single_line_class(context, class) {
            return;
        }
        let Some(brace) = class_brace_line(context, class) else {
            return;
        };
        let after = brace + 1;
        if after >= line_of(context, class.span.end) {
            return;
        }
        if !is_blank(context, after) {
            report_line(
                context,
                sink,
                after,
                "Class does not start with a blank line after the opening brace",
            );
        }
    }
}

/// Flags a blank line directly after an opening block brace.
pub struct BlockStartsWithBlankLineRule;

impl Rule for BlockStartsWithBlankLineRule {}

impl AstRule for BlockStartsWithBlankLineRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Block(block) = node else {
            return;
        };
        let first = line_of(context, block.span.start);
        let last = line_of(context, block.span.end);
        if last <= first {
            return;
        }
        let after = first + 1;
        if after >= last {
            return;
        }
        if is_blank(context, after) {
            report_line(
                context,
                sink,
                after,
                "Block starts with a blank line after the opening brace",
            );
        }
    }
}

/// Flags a blank line directly before a closing block brace.
pub struct BlockEndsWithBlankLineRule;

impl Rule for BlockEndsWithBlankLineRule {}

impl AstRule for BlockEndsWithBlankLineRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Block(block) = node else {
            return;
        };
        let first = line_of(context, block.span.start);
        let last = line_of(context, block.span.end);
        if last <= first {
            return;
        }
        if !trimmed(context, last).starts_with('}') {
            return;
        }
        if is_blank(context, last - 1) {
            report_line(
                context,
                sink,
                last - 1,
                "Block ends with a blank line before the closing brace",
            );
        }
    }
}

/// Requires a blank line before annotated fields.
#[derive(Default)]
pub struct MissingBlankLineBeforeAnnotatedFieldRule {
    class_lines: Vec<usize>,
}

impl Rule for MissingBlankLineBeforeAnnotatedFieldRule {}

impl AstRule for MissingBlankLineBeforeAnnotatedFieldRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field).union(NodeMask::of(NodeKind::Class))
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Class(_)) {
            self.class_lines.pop();
        }
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if let NodeRef::Class(class) = node {
            let start = line_of(context, class.span.start);
            self.class_lines.push(start);
            return;
        }
        let NodeRef::Field(field) = node else {
            return;
        };
        if field.annotations.is_empty() {
            return;
        }
        let line = line_of(context, field.span.start);
        if line <= 1 {
            return;
        }
        if self
            .class_lines
            .last()
            .is_some_and(|class_line| *class_line + 1 == line)
        {
            return;
        }
        let previous = trimmed(context, line - 1);
        // `CodeNarc` deliberately skips a field that sits directly below the
        // class declaration.
        if previous.is_empty()
            || previous.starts_with("//")
            || previous.starts_with("/*")
            || previous.starts_with('*')
        {
            return;
        }
        if previous.ends_with('{') {
            return;
        }
        report_line(
            context,
            sink,
            line,
            &format!(
                "There is no blank line before the declaration for field \"{}\" that has annotations",
                field.name
            ),
        );
    }
}

/// Flags files that do not end with a newline.
pub struct FileEndsWithoutNewlineRule;

impl Rule for FileEndsWithoutNewlineRule {}

impl FileRule for FileEndsWithoutNewlineRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let text = context.source.text();
        if text.is_empty() || text.ends_with('\n') {
            return;
        }
        let last = context.source.line_count().saturating_sub(1).max(1);
        report_line(context, sink, last, "File does not end with a newline");
    }
}

/// Flags consecutive blank lines.
pub struct ConsecutiveBlankLinesRule;

impl Rule for ConsecutiveBlankLinesRule {}

impl FileRule for ConsecutiveBlankLinesRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let count = context.source.line_count();
        for line in 2..=count {
            if line_is_blank(context, line) && line_is_blank(context, line - 1) {
                report_line(context, sink, line - 1, "File has consecutive blank lines");
            }
        }
    }
}

/// Requires a blank line before the package declaration.
pub struct BlankLineBeforePackageRule;

impl Rule for BlankLineBeforePackageRule {}

impl AstRule for BlankLineBeforePackageRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Script)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Script(script) = node else {
            return;
        };
        let Some(package) = &script.package else {
            return;
        };
        let line = line_of(context, package.span.start);
        if line <= 1 {
            return;
        }
        if !is_blank(context, line - 1) {
            report_line(
                context,
                sink,
                line,
                "The package declaration is not preceded by a blank line",
            );
        }
    }
}

/// Requires a blank line after the package declaration.
pub struct MissingBlankLineAfterPackageRule;

impl Rule for MissingBlankLineAfterPackageRule {}

impl AstRule for MissingBlankLineAfterPackageRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Script)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Script(script) = node else {
            return;
        };
        let Some(package) = &script.package else {
            return;
        };
        let package_line = line_of(context, package.span.start);
        let next = package_line + 1;
        if next > context.source.line_count() {
            return;
        }
        if !is_blank(context, next) {
            report_line(
                context,
                sink,
                package_line,
                "The package declaration is not followed by a blank line",
            );
        }
    }
}

/// Requires a blank line after the import block.
pub struct MissingBlankLineAfterImportsRule;

impl Rule for MissingBlankLineAfterImportsRule {}

impl AstRule for MissingBlankLineAfterImportsRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Script)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Script(script) = node else {
            return;
        };
        let Some(last) = script.imports.last() else {
            return;
        };
        let line = line_of(context, last.span.start) + 1;
        if line > context.source.line_count() {
            return;
        }
        if !is_blank(context, line) {
            report_line(
                context,
                sink,
                line,
                "The imports are not followed by a blank line",
            );
        }
    }
}

/// Flags a statement on the opening line of a multi-line closure.
pub struct ClosureStatementOnOpeningLineOfMultipleLineClosureRule;

impl Rule for ClosureStatementOnOpeningLineOfMultipleLineClosureRule {}

impl AstRule for ClosureStatementOnOpeningLineOfMultipleLineClosureRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Closure { body, .. } = &expr.kind else {
            return;
        };
        let Some(first) = body.statements.first() else {
            return;
        };
        let open = line_of(context, expr.span.start);
        let close = line_of(context, expr.span.end);
        if close <= open {
            return;
        }
        if line_of(context, first.span().start) == open {
            report_line(
                context,
                sink,
                open,
                "The multi-line closure contains a statement on the opening line of the closure.",
            );
        }
    }
}

/// Flags whitespace directly after a `!` operator.
pub struct SpaceAfterNotOperatorRule;

impl Rule for SpaceAfterNotOperatorRule {}

impl FileRule for SpaceAfterNotOperatorRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let text = context.source.text();
        let bytes = text.as_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            if *byte != b'!' {
                continue;
            }
            let Some(next) = bytes.get(index + 1) else {
                continue;
            };
            if *next == b'=' || *next == b'!' {
                continue;
            }
            if next.is_ascii_whitespace() {
                report_line(
                    context,
                    sink,
                    line_of(context, index),
                    "The not operator is followed by whitespace",
                );
            }
        }
    }
}

/// Requires whitespace around a closure arrow.
pub struct SpaceAroundClosureArrowRule;

impl Rule for SpaceAroundClosureArrowRule {}

impl AstRule for SpaceAroundClosureArrowRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        if !matches!(
            expr.kind,
            ExprKind::Closure {
                has_arrow: true,
                ..
            }
        ) {
            return;
        }
        let Some(text) = context.source.text().get(expr.span.range()) else {
            return;
        };
        let Some(index) = text.find("->") else {
            return;
        };
        let before = text[..index].chars().next_back();
        let after = text[index + 2..].chars().next();
        let before_ok = before.is_none_or(char::is_whitespace);
        let after_ok = after.is_none_or(|ch| ch.is_whitespace() || ch == '{');
        if before_ok && after_ok {
            return;
        }
        report_line(
            context,
            sink,
            line_of(context, expr.span.start + index),
            "The closure arrow is not surrounded by whitespace",
        );
    }
}

/// Flags whitespace between a method call name and its arguments.
pub struct SpaceAfterMethodCallNameRule;

impl Rule for SpaceAfterMethodCallNameRule {}

impl FileRule for SpaceAfterMethodCallNameRule {
    fn check(&mut self, _context: &FileContext<'_>, _sink: &mut DiagnosticSink<'_>) {}
}

/// Flags whitespace between a method declaration name and its parameter list.
pub struct SpaceAfterMethodDeclarationNameRule;

impl Rule for SpaceAfterMethodDeclarationNameRule {}

impl FileRule for SpaceAfterMethodDeclarationNameRule {
    fn check(&mut self, _context: &FileContext<'_>, _sink: &mut DiagnosticSink<'_>) {}
}

macro_rules! brace_placement_rule {
    ($struct_name:ident, $rule:literal, $class:literal, $description:literal, $kinds:expr) => {
        #[doc = $description]
        pub struct $struct_name;

        impl Rule for $struct_name {}

        impl AstRule for $struct_name {
            fn interests(&self) -> NodeMask {
                $kinds
            }

            fn enter(
                &mut self,
                node: NodeRef<'_>,
                context: &FileContext<'_>,
                sink: &mut DiagnosticSink<'_>,
            ) {
                let span = match node {
                    NodeRef::Class(class) => Some(class.span),
                    NodeRef::Method(method) => Some(method.span),
                    NodeRef::Stmt(stmt) => Some(stmt.span()),
                    _ => None,
                };
                let Some(span) = span else {
                    return;
                };
                let text = context.source.text();
                let start = line_of(context, span.start);
                let end = line_of(context, span.end);
                for line in start..=end {
                    let Some(content) = context.source.line_text(line) else {
                        break;
                    };
                    let trimmed = content.trim_start();
                    if !trimmed.starts_with('{') || line == start {
                        continue;
                    }
                    let _ = text;
                    report_line(
                        context,
                        sink,
                        line,
                        "Opening brace should be on the same line as the declaration",
                    );
                    break;
                }
            }
        }
    };
}

brace_placement_rule!(
    BracesForClassRule,
    "BracesForClass",
    "org.codenarc.rule.formatting.BracesForClassRule",
    "Requires the class opening brace on the declaration line.",
    NodeMask::of(NodeKind::Class)
);
brace_placement_rule!(
    BracesForMethodRule,
    "BracesForMethod",
    "org.codenarc.rule.formatting.BracesForMethodRule",
    "Requires the method opening brace on the declaration line.",
    NodeMask::of(NodeKind::Method)
);
brace_placement_rule!(
    BracesForForLoopRule,
    "BracesForForLoop",
    "org.codenarc.rule.formatting.BracesForForLoopRule",
    "Requires the loop opening brace on the statement line.",
    NodeMask::of(NodeKind::For).union(NodeMask::of(NodeKind::ForEach))
);
brace_placement_rule!(
    BracesForIfElseRule,
    "BracesForIfElse",
    "org.codenarc.rule.formatting.BracesForIfElseRule",
    "Requires the if opening brace on the statement line.",
    NodeMask::of(NodeKind::If)
);
brace_placement_rule!(
    BracesForTryCatchFinallyRule,
    "BracesForTryCatchFinally",
    "org.codenarc.rule.formatting.BracesForTryCatchFinallyRule",
    "Requires the try opening brace on the statement line.",
    NodeMask::of(NodeKind::Try)
);

/// Flags indentation that is not a multiple of the configured width.
#[derive(Default)]
pub struct IndentationRule {
    spaces_per_indent_level: Option<usize>,
}

impl Rule for IndentationRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        self.spaces_per_indent_level = match properties.get("spacesPerIndentLevel") {
            None => None,
            Some(value) => Some(
                value
                    .parse()
                    .map_err(|_| "spacesPerIndentLevel must be a number".to_string())?,
            ),
        };
        Ok(())
    }
}

impl FileRule for IndentationRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let width = self.spaces_per_indent_level.unwrap_or(4);
        if width == 0 {
            return;
        }
        for line in 1..=context.source.line_count() {
            let Some(text) = context.source.line_text(line) else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            let mut indent = 0;
            for ch in text.chars() {
                match ch {
                    ' ' => indent += 1,
                    '\t' => indent += width,
                    _ => break,
                }
            }
            if indent % width != 0
                && let Some(span) = context.source.line_span(line)
            {
                sink.report(
                    span,
                    format!("The indentation is not a multiple of {width} spaces"),
                );
            }
        }
    }
}

/// Flags lines longer than the configured maximum.
pub struct LineLengthRule {
    length: usize,
    ignore_imports: bool,
    ignore_packages: bool,
    ignore_line_regex: Option<regex::Regex>,
}

impl Default for LineLengthRule {
    fn default() -> Self {
        Self {
            length: 120,
            ignore_imports: true,
            ignore_packages: true,
            ignore_line_regex: None,
        }
    }
}

impl Rule for LineLengthRule {
    fn configure(&mut self, properties: &crate::rule::RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("length") {
            self.length = value
                .parse()
                .map_err(|_| "length must be a number".to_string())?;
        }
        if let Some(value) = properties.get("ignoreImportStatements") {
            self.ignore_imports = value != "false";
        }
        if let Some(value) = properties.get("ignorePackageStatements") {
            self.ignore_packages = value != "false";
        }
        if let Some(value) = properties.get("ignoreLineRegex") {
            self.ignore_line_regex = Some(
                regex::Regex::new(value)
                    .map_err(|error| format!("invalid ignoreLineRegex: {error}"))?,
            );
        }
        Ok(())
    }
}

impl FileRule for LineLengthRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for line in 1..=context.source.line_count() {
            let Some(text) = context.source.line_text(line) else {
                continue;
            };
            if text.chars().count() <= self.length {
                continue;
            }
            let trimmed = text.trim_start();
            if (trimmed.starts_with("import ") && self.ignore_imports)
                || (trimmed.starts_with("package ") && self.ignore_packages)
            {
                continue;
            }
            if self
                .ignore_line_regex
                .as_ref()
                .is_some_and(|pattern| pattern.is_match(text))
            {
                continue;
            }
            if let Some(span) = context.source.line_span(line) {
                sink.report(
                    span,
                    format!(
                        "The line exceeds {} characters. The line is {} characters.",
                        self.length,
                        text.chars().count()
                    ),
                );
            }
        }
    }
}

/// Flags map entries without spaces around the colon.
pub struct SpaceAroundMapEntryColonRule;

impl Rule for SpaceAroundMapEntryColonRule {}

impl AstRule for SpaceAroundMapEntryColonRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Map(entries) = &expr.kind else {
            return;
        };
        for (key, value) in entries {
            let start = key.span.end;
            let end = value.span.start;
            if start >= end {
                continue;
            }
            let Some(between) = context.source.text().get(start..end) else {
                continue;
            };
            let Some(colon) = between.find(':') else {
                continue;
            };
            let before = between[..colon].chars().next_back();
            let after = between[colon + 1..].chars().next();
            if before.is_some_and(|ch| !ch.is_whitespace())
                || after.is_some_and(|ch| !ch.is_whitespace())
            {
                sink.report(expr.span, "Map entry colon should be surrounded by spaces");
            }
        }
    }
}

/// Descriptors for the formatting rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "LineLength",
                "org.codenarc.rule.formatting.LineLengthRule",
                Category::Formatting,
                Severity::Warning,
                "Lines should not exceed the configured length.",
            ),
            || RuleInstanceImpl::File(Box::new(LineLengthRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAroundMapEntryColon",
                "org.codenarc.rule.formatting.SpaceAroundMapEntryColonRule",
                Category::Formatting,
                Severity::Info,
                "Map entry colons should be surrounded by spaces.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpaceAroundMapEntryColonRule)),
        ),
        descriptor(
            RuleMeta::new(
                "TrailingWhitespace",
                "org.codenarc.rule.formatting.TrailingWhitespaceRule",
                Category::Formatting,
                Severity::Info,
                "Checks for lines that end with whitespace characters.",
            ),
            || RuleInstanceImpl::File(Box::new(TrailingWhitespaceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterCatch",
                "org.codenarc.rule.formatting.SpaceAfterCatchRule",
                Category::Formatting,
                Severity::Info,
                "Keywords should be followed by exactly one space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterCatchRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterFor",
                "org.codenarc.rule.formatting.SpaceAfterForRule",
                Category::Formatting,
                Severity::Info,
                "Keywords should be followed by exactly one space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterForRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterIf",
                "org.codenarc.rule.formatting.SpaceAfterIfRule",
                Category::Formatting,
                Severity::Info,
                "Keywords should be followed by exactly one space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterIfRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterSwitch",
                "org.codenarc.rule.formatting.SpaceAfterSwitchRule",
                Category::Formatting,
                Severity::Info,
                "Keywords should be followed by exactly one space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterSwitchRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterWhile",
                "org.codenarc.rule.formatting.SpaceAfterWhileRule",
                Category::Formatting,
                Severity::Info,
                "Keywords should be followed by exactly one space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterWhileRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterComma",
                "org.codenarc.rule.formatting.SpaceAfterCommaRule",
                Category::Formatting,
                Severity::Info,
                "Commas should be followed by a space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterCommaRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterSemicolon",
                "org.codenarc.rule.formatting.SpaceAfterSemicolonRule",
                Category::Formatting,
                Severity::Info,
                "Semicolons should be followed by a space.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAfterSemicolonRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterNotOperator",
                "org.codenarc.rule.formatting.SpaceAfterNotOperatorRule",
                Category::Formatting,
                Severity::Info,
                "The `!` operator should not be followed by whitespace.",
            ),
            || RuleInstanceImpl::File(Box::new(SpaceAfterNotOperatorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterOpeningBrace",
                "org.codenarc.rule.formatting.SpaceAfterOpeningBraceRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces should be followed by a space.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpaceAfterOpeningBraceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterClosingBrace",
                "org.codenarc.rule.formatting.SpaceAfterClosingBraceRule",
                Category::Formatting,
                Severity::Info,
                "Closing braces should be followed by a space.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpaceAfterClosingBraceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceBeforeClosingBrace",
                "org.codenarc.rule.formatting.SpaceBeforeClosingBraceRule",
                Category::Formatting,
                Severity::Info,
                "Closing braces should be preceded by a space.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpaceBeforeClosingBraceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceBeforeOpeningBrace",
                "org.codenarc.rule.formatting.SpaceBeforeOpeningBraceRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces should be preceded by a space.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpaceBeforeOpeningBraceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceInsideParentheses",
                "org.codenarc.rule.formatting.SpaceInsideParenthesesRule",
                Category::Formatting,
                Severity::Info,
                "Parentheses should not contain inner whitespace.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceInsideParenthesesRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAroundOperator",
                "org.codenarc.rule.formatting.SpaceAroundOperatorRule",
                Category::Formatting,
                Severity::Info,
                "Binary operators should be surrounded by whitespace.",
            ),
            || RuleInstanceImpl::Token(Box::new(SpaceAroundOperatorRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterMethodCallName",
                "org.codenarc.rule.formatting.SpaceAfterMethodCallNameRule",
                Category::Formatting,
                Severity::Info,
                "Method calls should not have space before arguments.",
            ),
            || RuleInstanceImpl::File(Box::new(SpaceAfterMethodCallNameRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAfterMethodDeclarationName",
                "org.codenarc.rule.formatting.SpaceAfterMethodDeclarationNameRule",
                Category::Formatting,
                Severity::Info,
                "Method declarations should not have space before the parameter list.",
            ),
            || RuleInstanceImpl::File(Box::new(SpaceAfterMethodDeclarationNameRule)),
        ),
        descriptor(
            RuleMeta::new(
                "Indentation",
                "org.codenarc.rule.formatting.IndentationRule",
                Category::Formatting,
                Severity::Info,
                "Lines should be indented by a multiple of the configured width.",
            ),
            || RuleInstanceImpl::File(Box::new(IndentationRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ConsecutiveBlankLines",
                "org.codenarc.rule.formatting.ConsecutiveBlankLinesRule",
                Category::Formatting,
                Severity::Info,
                "Files should not contain consecutive blank lines.",
            ),
            || RuleInstanceImpl::File(Box::new(ConsecutiveBlankLinesRule)),
        ),
        descriptor(
            RuleMeta::new(
                "FileEndsWithoutNewline",
                "org.codenarc.rule.formatting.FileEndsWithoutNewlineRule",
                Category::Formatting,
                Severity::Info,
                "Files should end with a newline.",
            ),
            || RuleInstanceImpl::File(Box::new(FileEndsWithoutNewlineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SpaceAroundClosureArrow",
                "org.codenarc.rule.formatting.SpaceAroundClosureArrowRule",
                Category::Formatting,
                Severity::Info,
                "The closure arrow should be surrounded by whitespace.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SpaceAroundClosureArrowRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BracesForClass",
                "org.codenarc.rule.formatting.BracesForClassRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces for classes belong on the declaration line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BracesForClassRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BracesForMethod",
                "org.codenarc.rule.formatting.BracesForMethodRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces for methods belong on the declaration line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BracesForMethodRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BracesForForLoop",
                "org.codenarc.rule.formatting.BracesForForLoopRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces for loops belong on the statement line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BracesForForLoopRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BracesForIfElse",
                "org.codenarc.rule.formatting.BracesForIfElseRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces for if statements belong on the statement line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BracesForIfElseRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BracesForTryCatchFinally",
                "org.codenarc.rule.formatting.BracesForTryCatchFinallyRule",
                Category::Formatting,
                Severity::Info,
                "Opening braces for try statements belong on the statement line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BracesForTryCatchFinallyRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BlockEndsWithBlankLine",
                "org.codenarc.rule.formatting.BlockEndsWithBlankLineRule",
                Category::Formatting,
                Severity::Info,
                "Blocks should not end with a blank line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BlockEndsWithBlankLineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BlockStartsWithBlankLine",
                "org.codenarc.rule.formatting.BlockStartsWithBlankLineRule",
                Category::Formatting,
                Severity::Info,
                "Blocks should not start with a blank line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BlockStartsWithBlankLineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ClassEndsWithBlankLine",
                "org.codenarc.rule.formatting.ClassEndsWithBlankLineRule",
                Category::Formatting,
                Severity::Info,
                "Classes should end with a blank line before the closing brace.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ClassEndsWithBlankLineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ClassStartsWithBlankLine",
                "org.codenarc.rule.formatting.ClassStartsWithBlankLineRule",
                Category::Formatting,
                Severity::Info,
                "Classes should start with a blank line after the opening brace.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ClassStartsWithBlankLineRule)),
        ),
        descriptor(
            RuleMeta::new(
                "BlankLineBeforePackage",
                "org.codenarc.rule.formatting.BlankLineBeforePackageRule",
                Category::Formatting,
                Severity::Info,
                "The package declaration should be preceded by a blank line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(BlankLineBeforePackageRule)),
        ),
        descriptor(
            RuleMeta::new(
                "MissingBlankLineAfterImports",
                "org.codenarc.rule.formatting.MissingBlankLineAfterImportsRule",
                Category::Formatting,
                Severity::Info,
                "The import block should be followed by a blank line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MissingBlankLineAfterImportsRule)),
        ),
        descriptor(
            RuleMeta::new(
                "MissingBlankLineAfterPackage",
                "org.codenarc.rule.formatting.MissingBlankLineAfterPackageRule",
                Category::Formatting,
                Severity::Info,
                "The package declaration should be followed by a blank line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MissingBlankLineAfterPackageRule)),
        ),
        descriptor(
            RuleMeta::new(
                "MissingBlankLineBeforeAnnotatedField",
                "org.codenarc.rule.formatting.MissingBlankLineBeforeAnnotatedFieldRule",
                Category::Formatting,
                Severity::Info,
                "Annotated fields should be preceded by a blank line.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MissingBlankLineBeforeAnnotatedFieldRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ClosureStatementOnOpeningLineOfMultipleLineClosure",
                "org.codenarc.rule.formatting.ClosureStatementOnOpeningLineOfMultipleLineClosureRule",
                Category::Formatting,
                Severity::Info,
                "Multi-line closures should not start on the opening line.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(
                    ClosureStatementOnOpeningLineOfMultipleLineClosureRule,
                ))
            },
        ),
    ]
}

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
            report_line(context, sink, line, "Line ends with whitespace characters");
        }
    }
}

/// Returns the gap between two offsets as a string slice.
fn gap(source: &str, from: usize, to: usize) -> &str {
    if to <= from {
        return "";
    }
    source.get(from..to).unwrap_or_default()
}

/// Checks that a keyword is followed by exactly one space and `(`.
fn check_keyword_spacing(
    context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    kind: TokenKind,
    keyword: &str,
) {
    let source = context.source.text();
    for (index, token) in context.tokens.iter().enumerate() {
        if token.kind != kind {
            continue;
        }
        let Some(next) = context.tokens.get(index + 1) else {
            continue;
        };
        if next.kind != TokenKind::LeftParen {
            continue;
        }
        if gap(source, token.span.end, next.span.start) == " " {
            continue;
        }
        report_line(
            context,
            sink,
            line_of(context, token.span.start),
            &format!(
                "The {keyword} keyword is not followed by exactly one space and an opening parenthesis"
            ),
        );
    }
}

macro_rules! keyword_space_rule {
    ($struct_name:ident, $rule:literal, $keyword:literal, $kind:expr, $description:literal) => {
        #[doc = $description]
        pub struct $struct_name;

        impl Rule for $struct_name {}

        impl TokenRule for $struct_name {
            fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
                check_keyword_spacing(context, sink, $kind, $keyword);
            }
        }
    };
}

keyword_space_rule!(
    SpaceAfterIfRule,
    "SpaceAfterIf",
    "if",
    TokenKind::If,
    "Requires exactly one space between `if` and `(`."
);
keyword_space_rule!(
    SpaceAfterForRule,
    "SpaceAfterFor",
    "for",
    TokenKind::For,
    "Requires exactly one space between `for` and `(`."
);
keyword_space_rule!(
    SpaceAfterWhileRule,
    "SpaceAfterWhile",
    "while",
    TokenKind::While,
    "Requires exactly one space between `while` and `(`."
);
keyword_space_rule!(
    SpaceAfterSwitchRule,
    "SpaceAfterSwitch",
    "switch",
    TokenKind::Switch,
    "Requires exactly one space between `switch` and `(`."
);
keyword_space_rule!(
    SpaceAfterCatchRule,
    "SpaceAfterCatch",
    "catch",
    TokenKind::Catch,
    "Requires exactly one space between `catch` and `(`."
);

/// Flags a comma that is not followed by whitespace.
pub struct SpaceAfterCommaRule;

impl Rule for SpaceAfterCommaRule {}

impl TokenRule for SpaceAfterCommaRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_space_after_token(context, sink, TokenKind::Comma, "comma");
    }
}

/// Flags a semicolon that is not followed by whitespace.
pub struct SpaceAfterSemicolonRule;

impl Rule for SpaceAfterSemicolonRule {}

impl TokenRule for SpaceAfterSemicolonRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        check_space_after_token(context, sink, TokenKind::Semicolon, "semicolon");
    }
}

fn check_space_after_token(
    context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    kind: TokenKind,
    label: &str,
) {
    let source = context.source.text();
    for (index, token) in context.tokens.iter().enumerate() {
        if token.kind != kind {
            continue;
        }
        let Some(next) = context.tokens.get(index + 1) else {
            continue;
        };
        if next.kind == TokenKind::Newline || next.kind == TokenKind::Eof {
            continue;
        }
        if !gap(source, token.span.end, next.span.start).is_empty() {
            continue;
        }
        report_line(
            context,
            sink,
            line_of(context, token.span.start),
            &format!("The {label} is not followed by a space or whitespace"),
        );
    }
}

/// Flags block braces that are not followed by whitespace.
pub struct SpaceAfterOpeningBraceRule;

impl Rule for SpaceAfterOpeningBraceRule {}

impl AstRule for SpaceAfterOpeningBraceRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Block(block) = node else {
            return;
        };
        let source = context.source.text();
        let Some(after) = source
            .get(block.span.start + 1..)
            .and_then(|rest| rest.chars().next())
        else {
            return;
        };
        if after.is_whitespace() || after == '}' {
            return;
        }
        report_line(
            context,
            sink,
            line_of(context, block.span.start),
            "The opening brace is not followed by a space or whitespace",
        );
    }
}

/// Flags block braces that are not followed by whitespace.
pub struct SpaceAfterClosingBraceRule;

impl Rule for SpaceAfterClosingBraceRule {}

impl AstRule for SpaceAfterClosingBraceRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block).union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let end = match node {
            NodeRef::Block(block) => block.span.end,
            NodeRef::Class(class) => class.span.end,
            _ => return,
        };
        let source = context.source.text();
        let Some(after) = source.get(end..).and_then(|rest| rest.chars().next()) else {
            return;
        };
        if after.is_whitespace() || after == ')' || after == ';' || after == ',' {
            return;
        }
        report_line(
            context,
            sink,
            line_of(context, end.saturating_sub(1)),
            "The closing brace is not followed by a space or whitespace",
        );
    }
}

/// Flags block braces that are not preceded by whitespace.
pub struct SpaceBeforeClosingBraceRule;

impl Rule for SpaceBeforeClosingBraceRule {}

impl AstRule for SpaceBeforeClosingBraceRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Block(block) = node else {
            return;
        };
        let source = context.source.text();
        let Some(before) = source
            .get(..block.span.end.saturating_sub(1))
            .and_then(|text| text.chars().next_back())
        else {
            return;
        };
        if before.is_whitespace() || before == '{' {
            return;
        }
        report_line(
            context,
            sink,
            line_of(context, block.span.end),
            "The closing brace is not preceded by a space or whitespace",
        );
    }
}

/// Flags a block brace that is not preceded by whitespace.
pub struct SpaceBeforeOpeningBraceRule;

impl Rule for SpaceBeforeOpeningBraceRule {}

impl AstRule for SpaceBeforeOpeningBraceRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Block).union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let start = match node {
            NodeRef::Block(block) => block.span.start,
            NodeRef::Class(class) => match class.span.range().end.checked_sub(1) {
                Some(_) => match brace_offset(context, class.span.start, class.span.end) {
                    Some(offset) => offset,
                    None => return,
                },
                None => return,
            },
            _ => return,
        };
        let source = context.source.text();
        let Some(before) = source
            .get(..start)
            .and_then(|text| text.chars().next_back())
        else {
            return;
        };
        if before.is_whitespace() || before == '(' || before == '{' || before == '$' {
            return;
        }
        report_line(
            context,
            sink,
            line_of(context, start),
            "The opening brace is not preceded by a space or whitespace",
        );
    }
}

/// Returns the offset of the first `{` in a span.
fn brace_offset(context: &FileContext<'_>, start: usize, end: usize) -> Option<usize> {
    let text = context.source.text().get(start..end)?;
    text.find('{').map(|index| start + index)
}

/// Flags unnecessary whitespace inside parentheses.
#[derive(Default)]
pub struct SpaceInsideParenthesesRule {
    reported: std::collections::BTreeSet<usize>,
}

impl Rule for SpaceInsideParenthesesRule {}

impl TokenRule for SpaceInsideParenthesesRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let source = context.source.text();
        for (index, token) in context.tokens.iter().enumerate() {
            let (neighbour, message) = match token.kind {
                TokenKind::LeftParen => (context.tokens.get(index + 1), "after"),
                TokenKind::RightParen => (
                    index.checked_sub(1).and_then(|at| context.tokens.get(at)),
                    "before",
                ),
                _ => continue,
            };
            let Some(neighbour) = neighbour else {
                continue;
            };
            if matches!(neighbour.kind, TokenKind::Newline | TokenKind::Eof) {
                continue;
            }
            let spacing = if message == "after" {
                gap(source, token.span.end, neighbour.span.start)
            } else {
                gap(source, neighbour.span.end, token.span.start)
            };
            if spacing.is_empty() {
                continue;
            }
            if !self.reported.insert(line_of(context, token.span.start)) {
                continue;
            }
            report_line(
                context,
                sink,
                line_of(context, token.span.start),
                "The parentheses contain whitespace directly inside the delimiter",
            );
        }
    }
}

/// Flags binary operators without surrounding whitespace.
pub struct SpaceAroundOperatorRule;

impl Rule for SpaceAroundOperatorRule {}

impl TokenRule for SpaceAroundOperatorRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let source = context.source.text();
        for (index, token) in context.tokens.iter().enumerate() {
            if !is_binary_operator(token.kind) {
                continue;
            }
            let (Some(previous), Some(next)) = (
                index.checked_sub(1).and_then(|at| context.tokens.get(at)),
                context.tokens.get(index + 1),
            ) else {
                continue;
            };
            let before = gap(source, previous.span.end, token.span.start);
            let after = gap(source, token.span.end, next.span.start);
            let before_ok = !before.is_empty()
                || previous.kind == TokenKind::Newline
                || previous.span.end == token.span.start;
            let after_ok = !after.is_empty()
                || next.kind == TokenKind::Newline
                || next.span.start == token.span.end;
            if before_ok && after_ok {
                continue;
            }
            report_line(
                context,
                sink,
                line_of(context, token.span.start),
                "The operator is not surrounded by whitespace",
            );
        }
    }
}

fn is_binary_operator(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Assign
            | TokenKind::Eq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::Le
            | TokenKind::Gt
            | TokenKind::Ge
            | TokenKind::AndAnd
            | TokenKind::OrOr
    )
}
