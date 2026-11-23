//! Diagnostics and severities.

use std::fmt;

use crate::span::Span;

/// The severity of a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    /// Informational finding.
    Info,
    /// Warning-level finding.
    Warning,
    /// Error-level finding.
    Error,
}

impl Severity {
    /// Returns the lowercase name used in machine-readable output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A single rule finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Stable rule name, for example `EmptyIfStatement`.
    pub rule: String,
    /// Finding severity.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Source location.
    pub span: Span,
    /// Optional remediation hint.
    pub help: Option<String>,
}

impl Diagnostic {
    /// Creates a diagnostic without a help message.
    #[must_use]
    pub fn new(
        rule: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            rule: rule.into(),
            severity,
            message: message.into(),
            span,
            help: None,
        }
    }

    /// Adds a remediation hint.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
