use std::fmt;

/// A configuration parse error with a TOML-ish path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    /// Where the error occurred, such as `targets.domain-class.in`.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl ParseError {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            formatter.write_str(&self.message)
        } else {
            write!(formatter, "{}: {}", self.path, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, ParseError>;
