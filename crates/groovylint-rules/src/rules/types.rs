//! Type references, as `CodeNarc`'s `ClassReferenceAstVisitor` sees them.
//!
//! The class-reference rules (`JdbcConnectionReference`, `IllegalPackageReference`,
//! `ImplementationAsType`, ...) all ask the same question: which types does this
//! file name, and where? One walker answers it.

use groovylint_core::Span;
use groovylint_syntax::ast::{Expr, ExprKind, Script, Stmt};

use crate::rule::NodeRef;

/// A type the file names, with the span it was named at.
pub struct TypeReference {
    /// Where the reference sits.
    pub span: Span,
    /// The type as written, possibly qualified.
    pub name: String,
}

/// Collects every type the file names: supertypes, signatures, declarations,
/// casts, constructor calls and class literals.
#[must_use]
pub fn type_references(script: &Script) -> Vec<TypeReference> {
    let mut collector = Collector { found: Vec::new() };
    crate::traverse::walk_script(script, &mut collector);
    collector.found
}

struct Collector {
    found: Vec<TypeReference>,
}

impl Collector {
    fn record(&mut self, span: Span, name: &str) {
        self.found.push(TypeReference {
            span,
            name: name.to_string(),
        });
    }
}

impl crate::traverse::AstVisitor for Collector {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Class(class) => {
                let mut names = Vec::new();
                if let Some(parent) = &class.extends {
                    names.push(parent.clone());
                }
                names.extend(class.implements.iter().cloned());
                for name in names {
                    self.record(class.span, &name);
                }
            }
            NodeRef::Field(field) => {
                if let Some(type_name) = &field.type_name {
                    self.record(field.span, type_name);
                }
            }
            NodeRef::Method(method) => {
                if let Some(return_type) = &method.return_type {
                    self.record(method.span, return_type);
                }
                for param in &method.params {
                    if let Some(type_name) = &param.type_name {
                        self.record(param.span, type_name);
                    }
                }
            }
            NodeRef::Stmt(Stmt::VarDecl(decl)) => {
                if let Some(type_name) = &decl.type_name {
                    self.record(decl.span, type_name);
                }
            }
            NodeRef::Stmt(Stmt::Try(try_stmt)) => {
                for catch in &try_stmt.catches {
                    for type_name in &catch.type_names {
                        self.record(catch.span, type_name);
                    }
                }
            }
            NodeRef::Expr(expr) => match &expr.kind {
                ExprKind::New { type_name, .. } | ExprKind::Cast { type_name, .. } => {
                    self.record(expr.span, type_name);
                }
                ExprKind::Property {
                    receiver,
                    name,
                    safe: false,
                } if name == "class" => {
                    if let ExprKind::Variable(base) = &receiver.kind {
                        self.record(expr.span, base);
                    }
                }
                // `org.bad.Helper.getCount()`, `value instanceof org.bad.Other` and
                // `new Handler(org.bad.Request)`: a qualified name used as a value.
                ExprKind::Property { .. } | ExprKind::Variable(_) => {
                    if let Some(name) = dotted_name(expr) {
                        self.record(expr.span, &name);
                    }
                }
                ExprKind::Closure { params, .. } => {
                    for param in params {
                        if let Some(type_name) = &param.type_name {
                            self.record(param.span, type_name);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Whether a type's package matches one of the configured wildcard patterns.
///
/// `CodeNarc` compares the package the type belongs to: `*` stays inside one
/// dotted segment, `**` crosses them, `?` is a single character.
#[must_use]
pub fn package_matches(type_name: &str, pattern: &str) -> bool {
    let Some(index) = type_name.rfind('.') else {
        return false;
    };
    let package = &type_name[..index];
    wildcard_matches(package, pattern.trim())
}

/// The package an import belongs to: `org.bad.Bad` and `org.bad.*` both give
/// `org.bad`, because the imported name itself is dropped.
#[must_use]
pub fn import_package(path: &str) -> &str {
    let trimmed = path.trim_end_matches('*');
    let trimmed = trimmed.trim_end_matches('.');
    match trimmed.rfind('.') {
        Some(index) => &trimmed[..index],
        None => "",
    }
}

/// Whether `value` matches a `CodeNarc` wildcard pattern.
#[must_use]
pub fn wildcard_matches(value: &str, pattern: &str) -> bool {
    if !pattern.contains('*') && !pattern.contains('?') {
        return value == pattern;
    }
    let mut regex = String::from("^");
    let mut characters = pattern.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '*' if characters.peek() == Some(&'*') => {
                characters.next();
                regex.push_str(".*");
            }
            '*' => regex.push_str("[^/]*"),
            '?' => regex.push('.'),
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    match regex::Regex::new(&regex) {
        Ok(compiled) => compiled.is_match(value),
        Err(_) => false,
    }
}

/// The dotted text of an expression built only from names, which is how
/// `CodeNarc` reads a qualified type used as a value.
fn dotted_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Variable(name) => Some(name.clone()),
        ExprKind::Property {
            receiver,
            name,
            safe: false,
        } => {
            let base = dotted_name(receiver)?;
            Some(format!("{base}.{name}"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{package_matches, wildcard_matches};

    #[test]
    fn matches_exact_and_wildcard_packages() {
        assert!(package_matches("org.bad.Widget", "org.bad"));
        assert!(!package_matches("org.bad.other.Widget", "org.bad"));
        assert!(package_matches("org.bad.other.Widget", "org.*"));
        assert!(package_matches("org.bad.other.Widget", "org.**"));
        assert!(!package_matches("Widget", "org.bad"));
    }

    #[test]
    fn star_crosses_dots_but_not_slashes() {
        // Package names have no separators for the wildcard to respect: only `/`
        // stops a `*`, which is why `org.*` covers every subpackage.
        assert!(wildcard_matches("org.bad", "org.*"));
        assert!(wildcard_matches("org.bad.other", "org.*"));
        assert!(!wildcard_matches("org/bad", "org*"));
        assert!(wildcard_matches("com.example", "com.ex?mple"));
    }
}
