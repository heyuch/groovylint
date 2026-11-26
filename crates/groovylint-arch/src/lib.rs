//! Architecture rule model and TOML parser.
//!
//! This crate exists to validate the DSL by forcing it into Rust types. It is
//! intentionally independent of the analyzer: the parser only builds an
//! `ArchConfig`; project evaluation can resolve targets and facts later.
//!
//! Canonical TOML uses nested tables, not multiline inline tables:
//!
//! ```toml
//! [targets.domain-module]
//! type = "module"
//! name = "*-domain"
//!
//! [targets.domain-class]
//! type = "class"
//! in = "domain-module"
//! ```
//!
//! Inline tables are accepted but TOML requires them to be single-line. `type`
//! is the target kind; a field's type selector is `fieldType`.

mod error;
mod model;
mod parse;

pub use error::{ParseError, Result};
pub use model::{
    AnnotationArgument, ArchConfig, ClassKind, Constraint, Count, DependencyRef, Must, PatternList,
    Predicate, ProjectConfig, Rule, Severity, SourceSet, TargetRef, TargetSpec, TargetType,
    TypeValue, Visibility,
};
pub use parse::parse;
