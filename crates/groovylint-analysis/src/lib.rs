//! Semantic analysis.
//!
//! The first implementation is an annotation-free nullness analysis:
//! AST-to-CFG lowering, a nullness lattice/store, guard refinement, and an
//! intraprocedural worklist solver. Cross-method summaries and annotation
//! contracts are planned next; see `docs/NULLCHECK.md`.

#![doc(html_root_url = "https://docs.rs/groovylint-analysis")]

pub mod cfg;
pub mod contracts;
pub mod dataflow;
pub mod nullability;
pub mod nullcheck;
pub mod types;
