//! Built-in rule registry.

use crate::rule::{AstRule, FileRule, RuleMeta, RuleProperties, TokenRule};
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
        let mut descriptors = vec![descriptor(UNNECESSARY_SEMICOLON_META, || {
            RuleInstanceImpl::Token(Box::new(UnnecessarySemicolonRule))
        })];
        descriptors.extend(crate::rules::empty_blocks::descriptors());
        descriptors.extend(crate::rules::conditionals::descriptors());
        descriptors.extend(crate::rules::finally::descriptors());
        descriptors.extend(crate::rules::api_usage::descriptors());
        descriptors.extend(crate::rules::empty_declarations::descriptors());
        descriptors.extend(crate::rules::unary::descriptors());
        descriptors.extend(crate::rules::class_design::descriptors());
        descriptors.extend(crate::rules::casts::descriptors());
        descriptors.extend(crate::rules::filter_closure::descriptors());

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

pub(crate) fn descriptor(meta: RuleMeta, factory: fn() -> RuleInstanceImpl) -> RuleDescriptor {
    RuleDescriptor { meta, factory }
}
