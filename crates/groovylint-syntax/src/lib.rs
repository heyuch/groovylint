//! Groovy syntax front end.
//!
//! This crate is intentionally a skeleton. The parser strategy is still an open
//! design decision: a hand-written recursive-descent/Pratt parser, a
//! tree-sitter grammar, or an existing Groovy parser crate.
//!
//! Planned responsibilities:
//!
//! - lexical analysis and token spans;
//! - lossless or near-lossless syntax representation;
//! - parser errors and recovery;
//! - lowering to a semantic tree used by analysis and rules.

#![doc(html_root_url = "https://docs.rs/groovylint-syntax")]

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;
