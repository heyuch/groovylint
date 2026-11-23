//! Configuration model and `CodeNarc` ruleset compatibility.
//!
//! The first vertical slice supports reading `CodeNarc` XML `<rule>` entries and
//! uses the vendored `CodeNarc` 4.0.0 `rulesets/basic.xml` as its default.

pub mod codenarc;

pub use codenarc::{CodeNarcRule, ConfigError, Ruleset, default_basic, load};
