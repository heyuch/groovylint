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
//! Inline tables are accepted but TOML requires them to be single-line. `kind`
//! is the built-in target type; `classKind` selects the class declaration kind,
//! and `type` remains the field type selector.

mod error;
mod model;
mod parse;

pub use error::{ParseError, Result};
pub use model::{
    AnnotationArgument, ArchConfig, ClassKind, ClassPredicate, ClassTarget, Constraint, Count,
    DependencyRef, FieldPredicate, MemberKind, MemberPredicate, MethodPredicate, MethodTarget,
    ModulePredicate, ModuleTarget, Must, PackagePredicate, PackageTarget, ParamPredicate,
    PatternList, ProjectConfig, Rule, Severity, SourceSet, Target, TargetRef, TargetType,
    TypeValue, Visibility,
};
pub use parse::parse;
