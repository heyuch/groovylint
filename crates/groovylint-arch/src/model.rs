//! In-memory architecture model.
//!
//! Target types are built in, so they are represented as an enum rather than a
//! `type` field plus a bag of predicates. Each member of [`Target`] owns exactly
//! the predicates that are legal for that target type.

use std::collections::BTreeMap;

/// The kind of a target node, kept for diagnostics and type checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetType {
    Module,
    Package,
    Class,
    Method,
}

impl TargetType {
    /// Parses the target kind used by target declarations and inline targets.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "module" => Some(Self::Module),
            "package" => Some(Self::Package),
            "class" => Some(Self::Class),
            "method" => Some(Self::Method),
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
        }
    }

    /// Whether a target of `self` can structurally contain `child`.
    #[must_use]
    pub const fn can_contain(self, child: Self) -> bool {
        match self {
            Self::Module => matches!(child, Self::Package | Self::Class),
            Self::Package => matches!(child, Self::Package | Self::Class),
            Self::Class => matches!(child, Self::Method),
            Self::Method => false,
        }
    }
}

/// A parsed target declaration or inline target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Target {
    Module(ModuleTarget),
    Package(PackageTarget),
    Class(ClassTarget),
    Method(MethodTarget),
}

impl Target {
    /// The target's built-in type.
    #[must_use]
    pub const fn target_type(&self) -> TargetType {
        match self {
            Self::Module(_) => TargetType::Module,
            Self::Package(_) => TargetType::Package,
            Self::Class(_) => TargetType::Class,
            Self::Method(_) => TargetType::Method,
        }
    }
}

/// A module target. Modules are roots: they have no `in` selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleTarget {
    pub predicates: Vec<ModulePredicate>,
}

/// Predicates legal on a module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModulePredicate {
    Name(PatternList),
    NameMatches(String),
    Path(PatternList),
    AnyOf(Vec<TargetRef>),
    Not(Box<ModuleTarget>),
}

/// A package target. Packages may be contained by a module or another package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageTarget {
    pub predicates: Vec<PackagePredicate>,
}

/// Predicates legal on a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackagePredicate {
    Name(PatternList),
    NameMatches(String),
    In(TargetRef),
    AnyOf(Vec<TargetRef>),
    Not(Box<PackageTarget>),
}

/// A class target. Classes may be contained by a module, package, or another
/// class target only through a valid structural owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassTarget {
    pub predicates: Vec<ClassPredicate>,
}

/// Predicates legal on a class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassPredicate {
    Name(PatternList),
    NameMatches(String),
    In(TargetRef),
    AnyOf(Vec<TargetRef>),
    Not(Box<ClassTarget>),
    Kind(ClassKind),
    Abstract(bool),
    Nested(bool),
    Extends(TypeValue),
    Implements(Vec<TypeValue>),
    AnnotatedWith(Vec<String>),
}

/// A method target. Methods are contained by a class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodTarget {
    pub predicates: Vec<MethodPredicate>,
}

/// Predicates legal on a method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MethodPredicate {
    Name(PatternList),
    NameMatches(String),
    In(TargetRef),
    AnyOf(Vec<TargetRef>),
    Not(Box<MethodTarget>),
    AnnotatedWith(Vec<String>),
    Modifiers(Vec<Modifier>),
    ParamCount(usize),
    RawReturnType(TypeValue),
}

/// Predicates legal on a field member (used by `allFields`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldPredicate {
    Name(PatternList),
    NameMatches(String),
    AnnotatedWith(Vec<String>),
    Modifiers(Vec<Modifier>),
    Type(TypeValue),
}

/// A reference to a target, by name or inline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetRef {
    Named(String),
    Inline(Box<Target>),
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

/// The kind of a class member in `allMembers`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberKind {
    Field,
    Method,
}

impl MemberKind {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "field" => Some(Self::Field),
            "method" => Some(Self::Method),
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

/// A selector over members in `allMembers`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberPredicate {
    Name(PatternList),
    NameMatches(String),
    AnnotatedWith(Vec<String>),
    Modifiers(Vec<Modifier>),
    Kind(MemberKind),
    Not(Box<MemberPredicate>),
}

/// A selector over method parameters in `allParams`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParamPredicate {
    Name(PatternList),
    NameMatches(String),
    AnnotatedWith(Vec<String>),
    Type(TypeValue),
    Not(Box<ParamPredicate>),
}

/// A rule constraint. The variant set is checked against the resolved target
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
    AllMembers(Vec<MemberPredicate>),
    AllMethods(Vec<MethodPredicate>),
    AllFields(Vec<FieldPredicate>),
    RawReturnType(TypeValue),
    NotRawReturnType(Vec<TypeValue>),
    AllParams(Vec<ParamPredicate>),
    Visibility(Visibility),
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
    pub targets: BTreeMap<String, Target>,
    pub rules: Vec<Rule>,
}
