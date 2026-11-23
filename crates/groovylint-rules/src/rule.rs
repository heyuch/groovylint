//! Rule contracts exposed to rule implementations.

use std::collections::BTreeMap;

use groovylint_core::{Diagnostic, Severity, SourceFile, Span};
use groovylint_syntax::ast::{
    ClassDecl, Expr, FieldDecl, InitializerDecl, MethodDecl, Script, Stmt,
};
use groovylint_syntax::token::Token;

/// Rule configuration properties, currently represented as strings for
/// compatibility with `CodeNarc` XML properties.
pub type RuleProperties = BTreeMap<String, String>;

/// Broad rule categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Category {
    /// Basic correctness rules.
    Basic,
    /// Unnecessary-code rules.
    Unnecessary,
}

impl Category {
    /// Returns the lower-case category name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Unnecessary => "unnecessary",
        }
    }
}

/// Static metadata for a rule.
#[derive(Clone, Copy, Debug)]
pub struct RuleMeta {
    /// Canonical `CodeNarc`-compatible rule name.
    pub name: &'static str,
    /// Fully qualified `CodeNarc` rule class.
    pub codenarc_class: &'static str,
    /// Rule category.
    pub category: Category,
    /// Default severity.
    pub default_severity: Severity,
    /// One-line description.
    pub description: &'static str,
}

impl RuleMeta {
    /// Creates rule metadata.
    #[must_use]
    pub const fn new(
        name: &'static str,
        codenarc_class: &'static str,
        category: Category,
        default_severity: Severity,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            codenarc_class,
            category,
            default_severity,
            description,
        }
    }
}

/// Base behavior shared by every rule capability.
///
/// Rules are instantiated per file so they may keep traversal state without
/// sharing mutable state across threads.
pub trait Rule: Send {
    /// Applies configured properties.
    fn configure(&mut self, _properties: &RuleProperties) -> Result<(), String> {
        Ok(())
    }
}

/// A rule that consumes the token stream.
pub trait TokenRule: Rule {
    /// Called once per file with the full context.
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>);
}

/// A rule that consumes the AST through a single shared traversal.
pub trait AstRule: Rule {
    /// Returns the node kinds this rule wants to observe.
    fn interests(&self) -> NodeMask;

    /// Called before a node's children are visited.
    fn enter(
        &mut self,
        _node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
    }

    /// Called after a node's children are visited.
    fn exit(
        &mut self,
        _node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
    }
}

/// A rule that runs after the AST and file-level facts are available.
pub trait FileRule: Rule {
    /// Called once per file.
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>);
}

/// The context available to rules for one file.
#[derive(Clone, Copy)]
pub struct FileContext<'a> {
    /// Source file.
    pub source: &'a SourceFile,
    /// Parsed syntax tree.
    pub script: &'a Script,
    /// Token stream.
    pub tokens: &'a [Token],
}

/// A diagnostic emitter bound to one rule and severity.
pub struct DiagnosticSink<'a> {
    rule: &'a str,
    severity: Severity,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> DiagnosticSink<'a> {
    /// Creates a sink.
    pub fn new(rule: &'a str, severity: Severity, diagnostics: &'a mut Vec<Diagnostic>) -> Self {
        Self {
            rule,
            severity,
            diagnostics,
        }
    }

    /// Reports a finding.
    pub fn report(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(self.rule, self.severity, message, span));
    }

    /// Reports a finding with a remediation hint.
    pub fn report_with_help(
        &mut self,
        span: Span,
        message: impl Into<String>,
        help: impl Into<String>,
    ) {
        self.diagnostics
            .push(Diagnostic::new(self.rule, self.severity, message, span).with_help(help));
    }
}

/// A coarse AST node category used for dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeKind {
    /// Whole script.
    Script,
    /// Class-like declaration.
    Class,
    /// Method declaration.
    Method,
    /// Field declaration.
    Field,
    /// Block.
    Block,
    /// `if`.
    If,
    /// `while`.
    While,
    /// Classic `for`.
    For,
    /// `for (x in y)`.
    ForEach,
    /// `try`.
    Try,
    /// `synchronized`.
    Synchronized,
    /// `finally` block.
    Finally,
    /// `switch`.
    Switch,
    /// Static or instance initializer.
    Initializer,
    /// Variable declaration.
    VarDecl,
    /// Expression statement.
    ExprStmt,
    /// Return statement.
    Return,
    /// Throw statement.
    Throw,
    /// Break statement.
    Break,
    /// Continue statement.
    Continue,
    /// Assert statement.
    Assert,
    /// Empty statement.
    Empty,
    /// Any expression.
    Expr,
}

impl NodeKind {
    /// Returns the dispatch bit for this kind.
    #[must_use]
    pub const fn bit(self) -> u64 {
        1_u64 << (self as u8)
    }
}

/// A set of [`NodeKind`] values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeMask(u64);

impl NodeMask {
    /// An empty mask.
    pub const EMPTY: Self = Self(0);
    /// A mask containing every node kind.
    pub const ALL: Self = Self(u64::MAX);

    /// Creates a mask containing one kind.
    #[must_use]
    pub const fn of(kind: NodeKind) -> Self {
        Self(kind.bit())
    }

    /// Creates a mask from multiple kinds.
    #[must_use]
    pub fn from_kinds(kinds: &[NodeKind]) -> Self {
        kinds
            .iter()
            .fold(Self::EMPTY, |mask, kind| mask.union(Self::of(*kind)))
    }

    /// Returns the union of two masks.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns whether a kind is in the mask.
    #[must_use]
    pub const fn contains(self, kind: NodeKind) -> bool {
        self.0 & kind.bit() != 0
    }
}

/// A borrowed AST node passed to [`AstRule`] callbacks.
#[derive(Clone, Copy, Debug)]
pub enum NodeRef<'a> {
    /// Whole script.
    Script(&'a Script),
    /// Class declaration.
    Class(&'a ClassDecl),
    /// Method declaration.
    Method(&'a MethodDecl),
    /// Field declaration.
    Field(&'a FieldDecl),
    /// Block.
    Block(&'a groovylint_syntax::ast::Block),
    /// Finally block (a context marker around a block).
    Finally(&'a groovylint_syntax::ast::Block),
    /// Static or instance initializer.
    Initializer(&'a InitializerDecl),
    /// Statement.
    Stmt(&'a Stmt),
    /// Expression.
    Expr(&'a Expr),
}

impl NodeRef<'_> {
    /// Returns the node kind used for dispatch.
    #[must_use]
    pub const fn kind(self) -> NodeKind {
        match self {
            Self::Script(_) => NodeKind::Script,
            Self::Class(_) => NodeKind::Class,
            Self::Method(_) => NodeKind::Method,
            Self::Field(_) => NodeKind::Field,
            Self::Block(_) => NodeKind::Block,
            Self::Finally(_) => NodeKind::Finally,
            Self::Initializer(_) => NodeKind::Initializer,
            Self::Stmt(stmt) => match stmt {
                Stmt::Block(_) => NodeKind::Block,
                Stmt::VarDecl(_) => NodeKind::VarDecl,
                Stmt::Expr(_) => NodeKind::ExprStmt,
                Stmt::If(_) => NodeKind::If,
                Stmt::While(_) => NodeKind::While,
                Stmt::For(_) => NodeKind::For,
                Stmt::ForEach(_) => NodeKind::ForEach,
                Stmt::Try(_) => NodeKind::Try,
                Stmt::Synchronized(_) => NodeKind::Synchronized,
                Stmt::Switch(_) => NodeKind::Switch,
                Stmt::Return(_) => NodeKind::Return,
                Stmt::Throw(_) => NodeKind::Throw,
                Stmt::Break(_) => NodeKind::Break,
                Stmt::Continue(_) => NodeKind::Continue,
                Stmt::Assert(_) => NodeKind::Assert,
                Stmt::Empty(_) => NodeKind::Empty,
            },
            Self::Expr(_) => NodeKind::Expr,
        }
    }
}
