//! Groovy syntax front end.
//!
//! This crate is the parser/syntax boundary and depends only on
//! `groovylint-core`. It does not depend on rules, configuration, analysis, or
//! reporting.
//!
//! The semantic AST in [`ast`] is the contract used by rules and analysis. It
//! intentionally stores structure and source spans, not comments, trivia,
//! formatting policy, or rule metadata.
//!
//! A formatter and safe autofix need lossless source information. The planned
//! next layer is a trivia-preserving token stream and a concrete syntax tree
//! (CST) built alongside the AST; see `docs/SYNTAX.md` and `docs/AUTOFIX.md`.
//!
//! Responsibilities:
//!
//! - lexical analysis and token spans;
//! - parser errors and recovery;
//! - the semantic AST;
//! - a lossless CST for formatting and source edits (planned);
//! - no formatting decisions and no rule-specific logic.

#![doc(html_root_url = "https://docs.rs/groovylint-syntax")]

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;
