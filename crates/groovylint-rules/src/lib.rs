//! Rule model, registry, and execution engine.
//!
//! Rules implement one or more capability traits. A single shared AST traversal
//! dispatches syntax-rule events, while token rules and file-level rules have
//! their own explicit phases.

pub mod engine;
pub mod registry;
pub mod rule;
pub mod rules;
pub mod traverse;

pub use engine::{Engine, FileAnalysis};
pub use registry::{RuleDescriptor, RuleInstanceImpl, RuleRegistry};
pub use rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta, RuleProperties, TokenRule,
};
