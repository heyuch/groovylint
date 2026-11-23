//! Shared, analysis-agnostic domain types for `groovylint`.

pub mod diagnostic;
pub mod source;
pub mod span;

pub use diagnostic::{Diagnostic, Severity};
pub use source::{LineCol, SourceFile};
pub use span::Span;
