//! In-memory architecture model.
//!
//! The model deliberately keeps target definitions, selectors, and constraints
//! separate. `in` is structural containment only; narrowing a target by another
//! target is expressed by repeating the owner predicate (`in = "domain-module"`)
//! or, if added later, by explicit set intersection.

use std::collections::BTreeMap;

/// The kind of a target node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetType {
    Module,
    Package,
    Class,
    Method,
    Field,
}

impl TargetType {
    /// Parses the `type` field used in target declarations.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "module" => Some(Self::Module),
            "package" => Some(Self::Package),
            "class" => Some(Self::Class),
            "method" => Some(Self::Method),
            "field" => Some(Self::Field),
            _ => None,
        }
    }

    /// Stable spelling used in diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Package => "package",
            Self::Class => "class",
            Self::Method => "method",
            Self::Field => "field",
        }
    }

    /// Whether a target of `self` can structurally contain `child`.
    #[must_use]
    pub const fn can_contain(self, child: Self) -> bool {
        match self {
            Self::Module => matches!(child, Self::Package | Self::Class),
            Self::Package => matches!(child, Self::Package | Self::Class),
            Self::Class => matches!(child, Self::Method | Self::Field),
            Self::Method | Self::Field => false,
        }
    }
}

/// One selector predicate. Predicates in a target are `AND`ed; a `name` list is
/// `OR`ed, and `anyOf` is an explicit union of target selectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Predicate {
    /// Simple-name pattern list. A list is a union.
    Name(PatternList),
    /// Anchored regular expression over the simple name.
    NameMatches(String),
    /// Structural containment (`in`).
    In(TargetRef),
    /// Union of target selectors.
    AnyOf(Vec<TargetRef>),
    /// Negated selector of the same target type.
    Not(Box<TargetSpec>),
    /// Module Gradle path pattern list.
    Path(PatternList),
    /// Class-like kind.
    Kind(ClassKind),
    /// Class is abstract.
    Abstract(bool),
    /// Class is nested.
    Nested(bool),
    /// Direct superclass.
    Extends(TypeValue),
    /// Direct interfaces.
    Implements(Vec<TypeValue>),
    /// Any of these annotations.
    AnnotatedWith(Vec<String>),
    /// Declaration modifiers; all listed modifiers must be present.
    Modifiers(Vec<Modifier>),
    /// Method parameter count.
    ParamCount(usize),
    /// Method raw return type.
    RawReturnType(TypeValue),
    /// Field type.
    FieldType(TypeValue),
}

/// A named target definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetSpec {
    pub target_type: TargetType,
    pub predicates: Vec<Predicate>,
}

/// A reference to a target, by name or inline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetRef {
    Named(String),
    Inline(Box<TargetSpec>),
}

/// A simple-name, path, or literal pattern list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternList {
    pub patterns: Vec<String>,
}

/// The declaration kind of a class-like target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassKind {
    Class,
    Interface,
    Trait,
    Enum,
    Annotation,
    Record,
}

impl ClassKind {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "class" => Some(Self::Class),
            "interface" => Some(Self::Interface),
            "trait" => Some(Self::Trait),
            "enum" => Some(Self::Enum),
            "annotation" => Some(Self::Annotation),
            "record" => Some(Self::Record),
            _ => None,
        }
    }
}

/// A declaration modifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Modifier {
    Static,
    Public,
    Protected,
    Private,
    Abstract,
    Final,
}

impl Modifier {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "static" => Some(Self::Static),
            "public" => Some(Self::Public),
            "protected" => Some(Self::Protected),
            "private" => Some(Self::Private),
            "abstract" => Some(Self::Abstract),
            "final" => Some(Self::Final),
            _ => None,
        }
    }
}

/// A type value: a written type name or a type target name, resolved later.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeValue(pub String);

/// A dependency edge endpoint relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyRef {
    /// Any module: the default when no prefix is written.
    Any(TargetRef),
    /// Same module as the source.
    Self_(TargetRef),
    /// Another module (alias for `any:`).
    Other(TargetRef),
    /// No target (`none`, only used by `visible`).
    None,
}

/// Visibility used by a method constraint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
    PackagePrivate,
}

impl Visibility {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "public" => Some(Self::Public),
            "protected" => Some(Self::Protected),
            "private" => Some(Self::Private),
            "package" | "packagePrivate" => Some(Self::PackagePrivate),
            _ => None,
        }
    }
}

/// A count expression for `counts`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Count {
    AtLeast(usize),
    AtMost(usize),
    Any,
    Some,
}

/// An annotation argument comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArgument {
    pub annotation: String,
    pub name: String,
    pub value: String,
}

/// A rule constraint. The valid variants are checked against the resolved target
/// type after parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Constraint {
    DependOnlyOn(Vec<DependencyRef>),
    NotDependOn(Vec<DependencyRef>),
    Visible(Vec<DependencyRef>),
    AnnotatedWith(Vec<String>),
    NotAnnotatedWith(Vec<String>),
    AnnotationArgument(AnnotationArgument),
    Extend(TypeValue),
    Implement(Vec<TypeValue>),
    Exist(bool),
    AllMembers(Vec<Predicate>),
    AllMethods(Vec<Predicate>),
    AllFields(Vec<Predicate>),
    RawReturnType(TypeValue),
    NotRawReturnType(Vec<TypeValue>),
    AllParams(Vec<Predicate>),
    Visibility(Visibility),
    FieldType(TypeValue),
    NotFieldType(Vec<TypeValue>),
    Children(Vec<String>),
    Counts(BTreeMap<String, Count>),
    Acyclic(bool),
}

/// The `must` map of a rule. Several entries are a conjunction.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Must {
    pub constraints: Vec<Constraint>,
}

/// Source-set scope.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SourceSet {
    #[default]
    Main,
    Test,
}

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Severity {
    #[default]
    Error,
    Warn,
}

/// One architecture rule block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub id: String,
    pub target: TargetRef,
    pub must: Must,
    pub because: Option<String>,
    pub severity: Severity,
    pub on: SourceSet,
    pub except: Vec<String>,
}

/// Project-level configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectConfig {
    pub severity: Severity,
}

/// A parsed architecture configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArchConfig {
    pub version: u32,
    pub project: ProjectConfig,
    pub targets: BTreeMap<String, TargetSpec>,
    pub rules: Vec<Rule>,
}
