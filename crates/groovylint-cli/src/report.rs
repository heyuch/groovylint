//! Text and JSON reporting.

use std::fmt::Write as _;

use groovylint_core::{Diagnostic, Severity, SourceFile};
use serde_json::json;

/// Diagnostics for one analyzed file.
pub struct FileReport {
    /// Source file, retained for rendering.
    pub source: SourceFile,
    /// Findings.
    pub diagnostics: Vec<Diagnostic>,
}

/// Aggregate counts across files.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Summary {
    /// Number of analyzed files.
    pub files: usize,
    /// Number of error diagnostics.
    pub errors: usize,
    /// Number of warning diagnostics.
    pub warnings: usize,
    /// Number of info diagnostics.
    pub infos: usize,
}

impl Summary {
    /// Returns the total number of diagnostics.
    #[must_use]
    pub const fn total(self) -> usize {
        self.errors + self.warnings + self.infos
    }
}

/// Computes aggregate counts.
#[must_use]
pub fn summarize(reports: &[FileReport]) -> Summary {
    let mut summary = Summary {
        files: reports.len(),
        ..Summary::default()
    };
    for report in reports {
        for diagnostic in &report.diagnostics {
            match diagnostic.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Info => summary.infos += 1,
            }
        }
    }
    summary
}

/// Renders text diagnostics to stdout.
pub fn write_text(reports: &[FileReport]) {
    for report in reports {
        for diagnostic in &report.diagnostics {
            let start = report.source.line_col(diagnostic.span.start);
            let end = report.source.line_col(diagnostic.span.end);
            println!(
                "{}:{}:{}: {}[{}] {}",
                report.source.path().display(),
                start.line,
                start.column,
                diagnostic.severity,
                diagnostic.rule,
                diagnostic.message
            );

            if let Some(line) = report.source.line_text(start.line) {
                println!("  {:>4} | {line}", start.line);
                let caret_start = start.column.saturating_sub(1);
                let caret_len = if end.line == start.line {
                    end.column.saturating_sub(start.column).max(1)
                } else {
                    line.chars().count().saturating_sub(caret_start).max(1)
                };
                println!(
                    "       | {}{}",
                    " ".repeat(caret_start),
                    "^".repeat(caret_len)
                );
            }

            if let Some(help) = &diagnostic.help {
                println!("       = help: {help}");
            }
            println!();
        }
    }
}

/// Renders JSON diagnostics to stdout.
pub fn write_json(reports: &[FileReport], summary: Summary) -> serde_json::Result<String> {
    let files: Vec<_> = reports
        .iter()
        .map(|report| {
            let diagnostics: Vec<_> = report
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let start = report.source.line_col(diagnostic.span.start);
                    let end = report.source.line_col(diagnostic.span.end);
                    json!({
                        "rule": diagnostic.rule,
                        "severity": diagnostic.severity.as_str(),
                        "message": diagnostic.message,
                        "start": { "line": start.line, "column": start.column },
                        "end": { "line": end.line, "column": end.column },
                        "help": diagnostic.help,
                    })
                })
                .collect();
            json!({
                "path": report.source.path().display().to_string(),
                "diagnostics": diagnostics,
            })
        })
        .collect();

    let mut json = serde_json::to_string_pretty(&json!({
        "version": 1,
        "files": files,
        "summary": {
            "files": summary.files,
            "errors": summary.errors,
            "warnings": summary.warnings,
            "infos": summary.infos,
            "total": summary.total(),
        },
    }))?;
    json.push('\n');
    Ok(json)
}

/// Formats a one-line summary for stderr.
#[must_use]
pub fn summary_line(summary: Summary) -> String {
    let mut output = String::new();
    if summary.errors > 0 {
        let _ = write!(output, "{} error(s), ", summary.errors);
    }
    let _ = write!(
        output,
        "{} warning(s), {} info(s) in {} file(s)",
        summary.warnings, summary.infos, summary.files
    );
    output
}
