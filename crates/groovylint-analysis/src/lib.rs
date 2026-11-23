//! Semantic analysis foundation.
//!
//! This crate is intentionally a skeleton. It will host control-flow graphs,
//! data-flow frameworks, type inference, and nullability analysis once the
//! front end and compatibility scope are settled.
//!
//! Planned responsibilities:
//!
//! - lowering syntax into an analysis-friendly representation;
//! - control-flow graph construction;
//! - forward/backward data-flow solvers;
//! - type and nullability lattices;
//! - checker-framework-inspired nullness checks.

#![doc(html_root_url = "https://docs.rs/groovylint-analysis")]

pub mod cfg;
pub mod dataflow;
pub mod nullability;
pub mod types;
