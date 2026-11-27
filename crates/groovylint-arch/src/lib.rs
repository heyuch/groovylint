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
//! is the built-in target type and `kind` selects the class declaration kind.
//! Fields are not targets; `type` is a field member selector inside
//! `allFields`.

mod error;
mod eval;
mod facts;
mod model;
mod parse;
mod project;

pub use error::{ParseError, Result};
pub use eval::{ArchFinding, check};
pub use facts::{
    ClassFact, FactCollector, FactPlan, FieldFact, FileFacts, ImportFact, MethodFact,
    ReferenceKind, collect, module_for,
};
pub use model::{
    AnnotationArgument, ArchConfig, ClassKind, ClassPredicate, ClassTarget, Constraint, Count,
    DependencyRef, FieldPredicate, MemberKind, MemberPredicate, MethodPredicate, MethodTarget,
    ModulePredicate, ModuleTarget, Must, PackagePredicate, PackageTarget, ParamPredicate,
    PatternList, ProjectConfig, Rule, Severity, SourceSet, Target, TargetRef, TargetType,
    TypeValue, Visibility,
};
pub use parse::parse;
pub use project::Project;
