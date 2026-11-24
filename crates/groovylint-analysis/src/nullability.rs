//! Nullness lattice and access paths.

use std::collections::BTreeMap;

use groovylint_core::Span;

/// The nullness abstract value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Nullness {
    /// Definitely null.
    Null,
    /// Definitely non-null.
    NonNull,
    /// At least one path may produce null.
    MaybeNull,
    /// No usable information. Usually an unresolved call or a dynamic value.
    Unknown,
}

impl Nullness {
    /// Joins two abstract values at a control-flow merge.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        use Nullness::{MaybeNull, NonNull, Null, Unknown};
        match (self, other) {
            (Null, Null) | (NonNull, NonNull) | (MaybeNull, MaybeNull) | (Unknown, Unknown) => self,
            (Null | MaybeNull, NonNull | Unknown)
            | (NonNull | MaybeNull, Null)
            | (Null | NonNull, MaybeNull)
            | (Unknown, Null | MaybeNull) => MaybeNull,
            // An unknown path joined with non-null remains unknown.
            (NonNull, Unknown) | (Unknown, NonNull) => Unknown,
        }
    }

    /// Returns whether this value is definitely or possibly null.
    #[must_use]
    pub const fn is_nullish(self) -> bool {
        matches!(self, Self::Null | Self::MaybeNull)
    }

    /// Returns whether this value is definitely null.
    #[must_use]
    pub const fn is_null(self) -> bool {
        matches!(self, Self::Null)
    }
}

/// A canonical access path used as a store key.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AccessPath {
    /// Local variable or parameter.
    Local(String),
    /// `this`.
    This,
    /// A field/property on another path.
    Field(Box<AccessPath>, String),
    /// A lookup by a literal key (`map.get('k')` / `map['k']`).
    Lookup(Box<AccessPath>, String),
}

impl AccessPath {
    /// Builds a field path.
    #[must_use]
    pub fn field(base: Self, name: impl Into<String>) -> Self {
        Self::Field(Box::new(base), name.into())
    }
}

/// A dataflow store for one program point.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Store {
    values: BTreeMap<AccessPath, Nullness>,
}

impl Store {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up a path, defaulting to unknown.
    #[must_use]
    pub fn get(&self, path: &AccessPath) -> Nullness {
        self.values.get(path).copied().unwrap_or(Nullness::Unknown)
    }

    /// Writes a path.
    pub fn set(&mut self, path: AccessPath, value: Nullness) {
        self.values.insert(path, value);
    }

    /// Removes a path.
    pub fn remove(&mut self, path: &AccessPath) {
        self.values.remove(path);
    }

    /// Returns all entries.
    #[must_use]
    pub fn values(&self) -> &BTreeMap<AccessPath, Nullness> {
        &self.values
    }

    /// Joins two stores.
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        let mut result = Self::new();
        let keys: std::collections::BTreeSet<&AccessPath> =
            self.values.keys().chain(other.values.keys()).collect();
        for key in keys {
            result
                .values
                .insert(key.clone(), self.get(key).join(other.get(key)));
        }
        result
    }

    /// Invalidates all fields rooted at `root`.
    ///
    /// An unknown call may replace a field with null, so the value becomes
    /// `MaybeNull` rather than disappearing from the store.
    pub fn invalidate_fields(&mut self, root: &AccessPath) {
        for (path, value) in &mut self.values {
            if path != root && is_field_rooted_at(path, root) {
                *value = Nullness::MaybeNull;
            }
        }
    }
}

fn is_field_rooted_at(path: &AccessPath, root: &AccessPath) -> bool {
    match path {
        AccessPath::Field(base, _) | AccessPath::Lookup(base, _) => {
            base.as_ref() == root || is_field_rooted_at(base, root)
        }
        AccessPath::Local(_) | AccessPath::This => false,
    }
}

/// A nullness finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// Source span.
    pub span: Span,
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
}

/// Default declaration nullness for a scope (package/class/compiler config).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Defaults {
    /// Unannotated parameters are non-null.
    pub params_nonnull: bool,
    /// Unannotated returns are non-null.
    pub returns_nonnull: bool,
    /// Unannotated fields are non-null.
    pub fields_nonnull: bool,
}

impl Defaults {
    /// Marked scopes cover parameters, returns, and fields.
    #[must_use]
    pub fn marked() -> Self {
        Self {
            params_nonnull: true,
            returns_nonnull: true,
            fields_nonnull: true,
        }
    }

    /// True when any default is enabled.
    #[must_use]
    pub fn any(self) -> bool {
        self.params_nonnull || self.returns_nonnull || self.fields_nonnull
    }
}

/// Reporting mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    /// Report only definitely-null dereferences.
    Conservative,
    /// Also report possible-null dereferences.
    Inferred,
    /// Inferred plus definite-initialisation checks for non-null fields.
    Strict,
}
