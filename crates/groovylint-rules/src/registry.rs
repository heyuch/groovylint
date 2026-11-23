//! Built-in rule registry.

use crate::rule::{AstRule, Category, FileRule, RuleMeta, RuleProperties, TokenRule};
use crate::rules::empty_blocks::{
    EmptyCatchBlockRule, EmptyElseBlockRule, EmptyFinallyBlockRule, EmptyForStatementRule,
    EmptyIfStatementRule, EmptyTryBlockRule, EmptyWhileStatementRule,
};
use crate::rules::unnecessary_semicolon::{
    META as UNNECESSARY_SEMICOLON_META, UnnecessarySemicolonRule,
};

/// An instance of one rule capability for a single file.
pub enum RuleInstanceImpl {
    /// AST rule instance.
    Ast(Box<dyn AstRule>),
    /// Token rule instance.
    Token(Box<dyn TokenRule>),
    /// File rule instance.
    File(Box<dyn FileRule>),
}

impl RuleInstanceImpl {
    /// Applies properties to the underlying rule.
    pub fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        match self {
            Self::Ast(rule) => rule.configure(properties),
            Self::Token(rule) => rule.configure(properties),
            Self::File(rule) => rule.configure(properties),
        }
    }
}

/// A registered rule and its factory.
#[derive(Clone, Copy)]
pub struct RuleDescriptor {
    /// Static metadata.
    pub meta: RuleMeta,
    /// Creates a fresh per-file instance.
    pub factory: fn() -> RuleInstanceImpl,
}

/// The built-in rule registry.
pub struct RuleRegistry {
    descriptors: Vec<RuleDescriptor>,
}

impl RuleRegistry {
    /// Creates the built-in registry.
    #[must_use]
    pub fn builtin() -> Self {
        let descriptors = vec![
            descriptor(
                RuleMeta::new(
                    "EmptyIfStatement",
                    "org.codenarc.rule.basic.EmptyIfStatementRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty if statements.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyIfStatementRule)),
            ),
            descriptor(
                RuleMeta::new(
                    "EmptyElseBlock",
                    "org.codenarc.rule.basic.EmptyElseBlockRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty else blocks.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyElseBlockRule)),
            ),
            descriptor(
                RuleMeta::new(
                    "EmptyWhileStatement",
                    "org.codenarc.rule.basic.EmptyWhileStatementRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty while statements.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyWhileStatementRule)),
            ),
            descriptor(
                RuleMeta::new(
                    "EmptyForStatement",
                    "org.codenarc.rule.basic.EmptyForStatementRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty for statements.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyForStatementRule)),
            ),
            descriptor(
                RuleMeta::new(
                    "EmptyTryBlock",
                    "org.codenarc.rule.basic.EmptyTryBlockRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty try blocks.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyTryBlockRule)),
            ),
            descriptor(
                RuleMeta::new(
                    "EmptyCatchBlock",
                    "org.codenarc.rule.basic.EmptyCatchBlockRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty catch blocks.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyCatchBlockRule)),
            ),
            descriptor(
                RuleMeta::new(
                    "EmptyFinallyBlock",
                    "org.codenarc.rule.basic.EmptyFinallyBlockRule",
                    Category::Basic,
                    groovylint_core::Severity::Warning,
                    "Checks for empty finally blocks.",
                ),
                || RuleInstanceImpl::Ast(Box::new(EmptyFinallyBlockRule)),
            ),
            descriptor(UNNECESSARY_SEMICOLON_META, || {
                RuleInstanceImpl::Token(Box::new(UnnecessarySemicolonRule))
            }),
        ];

        Self { descriptors }
    }

    /// Returns all known descriptors.
    #[must_use]
    pub fn descriptors(&self) -> &[RuleDescriptor] {
        &self.descriptors
    }

    /// Finds a rule by canonical name.
    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&RuleDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.meta.name == name)
    }

    /// Finds a rule by its `CodeNarc` class name.
    #[must_use]
    pub fn find_codenarc_class(&self, class_name: &str) -> Option<&RuleDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.meta.codenarc_class == class_name)
    }
}

fn descriptor(meta: RuleMeta, factory: fn() -> RuleInstanceImpl) -> RuleDescriptor {
    RuleDescriptor { meta, factory }
}
