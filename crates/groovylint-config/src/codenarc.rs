//! Minimal `CodeNarc` XML ruleset reader.
//!
//! Supported today:
//!
//! - `<ruleset>` documents with `<rule class='...'/>` entries;
//! - `<description>` is accepted and ignored.
//!
//! Rule properties and `<ruleset-ref>` are intentionally deferred until the
//! first rule slice is stable.

use std::fmt;
use std::path::{Path, PathBuf};

/// A `CodeNarc` rule entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeNarcRule {
    /// Fully qualified `CodeNarc` rule class.
    pub class_name: String,
}

/// A parsed `CodeNarc` ruleset.
#[derive(Clone, Debug, Default)]
pub struct Ruleset {
    /// Rules in document order.
    pub rules: Vec<CodeNarcRule>,
    /// Constructs that were accepted but not interpreted.
    pub warnings: Vec<String>,
}

impl Ruleset {
    /// Returns the `CodeNarc` class names in this ruleset.
    #[must_use]
    pub fn class_names(&self) -> Vec<String> {
        self.rules
            .iter()
            .map(|rule| rule.class_name.clone())
            .collect()
    }
}

/// A configuration error.
#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// Loads a `CodeNarc` XML ruleset from disk.
pub fn load(path: &Path) -> Result<Ruleset, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        ConfigError::new(format!(
            "failed to read ruleset `{}`: {error}",
            path.display()
        ))
    })?;
    parse(&text).map_err(|error| {
        ConfigError::new(format!(
            "failed to parse ruleset `{}`: {error}",
            path.display()
        ))
    })
}

/// Returns the vendored `CodeNarc` 4.0.0 basic ruleset.
#[must_use]
pub fn default_basic() -> Ruleset {
    parse(include_str!("../rulesets/basic.xml")).expect("vendored CodeNarc basic ruleset is valid")
}

/// Parses a `CodeNarc` XML ruleset.
pub fn parse(xml: &str) -> Result<Ruleset, ConfigError> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| ConfigError::new(format!("invalid XML: {error}")))?;
    let root = document.root_element();

    if root.tag_name().name() != "ruleset" {
        return Err(ConfigError::new(format!(
            "expected a <ruleset> root element, found <{}>",
            root.tag_name().name()
        )));
    }

    let mut rules = Vec::new();
    let mut warnings = Vec::new();

    for child in root.children().filter(roxmltree::Node::is_element) {
        match child.tag_name().name() {
            "rule" => {
                let class_name = child
                    .attribute("class")
                    .ok_or_else(|| ConfigError::new("<rule> is missing the `class` attribute"))?;
                rules.push(CodeNarcRule {
                    class_name: class_name.to_string(),
                });
            }
            "description" => {}
            "ruleset-ref" => warnings.push(
                "<ruleset-ref> is not supported yet; the referenced ruleset was ignored"
                    .to_string(),
            ),
            other => warnings.push(format!("ignored unsupported element <{other}>")),
        }
    }

    Ok(Ruleset { rules, warnings })
}

/// Returns the path used for display of the vendored default.
#[must_use]
pub fn default_ruleset_display_path() -> PathBuf {
    PathBuf::from("rulesets/basic.xml (CodeNarc 4.0.0)")
}

#[cfg(test)]
mod tests {
    use super::{default_basic, parse};

    #[test]
    fn default_ruleset_contains_code_narc_rules() {
        let ruleset = default_basic();
        assert_eq!(ruleset.rules.len(), 44);
        assert!(
            ruleset
                .class_names()
                .contains(&"org.codenarc.rule.basic.EmptyIfStatementRule".to_string())
        );
    }

    #[test]
    fn parses_rule_entries() {
        let ruleset = parse(
            r#"<ruleset><rule class="org.codenarc.rule.basic.EmptyIfStatementRule"/></ruleset>"#,
        )
        .expect("ruleset should parse");
        assert_eq!(ruleset.rules.len(), 1);
    }

    #[test]
    fn rejects_missing_class_attribute() {
        assert!(parse("<ruleset><rule /></ruleset>").is_err());
    }
}
