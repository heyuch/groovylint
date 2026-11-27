//! Project index and resolved dependency graph.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use groovylint_core::Span;

use crate::facts::{ClassFact, FileFacts, ReferenceKind};

/// A Gradle module (or a directory-derived module when no build file exists).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleFact {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub children: BTreeSet<String>,
}

/// A resolved or symbolic dependency edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: ReferenceKind,
    pub span: Span,
    pub path: PathBuf,
}

/// A resolved project index.
#[derive(Clone, Debug, Default)]
pub struct Project {
    pub root: PathBuf,
    pub modules: BTreeMap<String, ModuleFact>,
    pub packages: BTreeSet<String>,
    pub classes: BTreeMap<String, ClassFact>,
    pub edges: Vec<Edge>,
    pub files: BTreeMap<PathBuf, FileFacts>,
    /// Package to module, resolved once: a package is looked up for every edge a
    /// `self:` dependency is evaluated against, and finding it by scanning the
    /// classes made that a full pass over the project per edge.
    package_modules: BTreeMap<String, String>,
}

impl Project {
    /// Builds an index from already-extracted file facts.
    #[must_use]
    pub fn new(root: &Path, files: Vec<FileFacts>) -> Self {
        let mut project = Self {
            root: root.to_path_buf(),
            ..Self::default()
        };
        for file in &files {
            project.add_module(&file.module);
            for class in &file.classes {
                project.packages.insert(
                    class
                        .package
                        .clone()
                        .unwrap_or_else(|| "(default)".to_string()),
                );
                project.classes.insert(class.fqn.clone(), class.clone());
            }
        }
        for class in project.classes.values() {
            let package = class
                .package
                .clone()
                .unwrap_or_else(|| "(default)".to_string());
            project
                .package_modules
                .entry(package)
                .or_insert_with(|| class.module.clone());
        }
        for file in &files {
            for import in &file.imports {
                for class in &file.classes {
                    project.edges.push(Edge {
                        from: class.fqn.clone(),
                        to: import.path.clone(),
                        kind: ReferenceKind::Import,
                        span: import.span,
                        path: file.path.clone(),
                    });
                }
            }
            for reference in &file.references {
                let Some(class) = project.classes.get(&reference.from) else {
                    continue;
                };
                let resolved = resolve_type(
                    &reference.to,
                    class.package.as_deref(),
                    &file.imports,
                    &project.classes,
                );
                if let Some(package) = package_of(&resolved) {
                    project.packages.insert(package);
                }
                project.edges.push(Edge {
                    from: reference.from.clone(),
                    to: resolved,
                    kind: reference.kind,
                    span: reference.span,
                    path: file.path.clone(),
                });
            }
        }
        project.files = files
            .into_iter()
            .map(|file| (file.path.clone(), file))
            .collect();
        project
    }

    fn add_module(&mut self, id: &str) {
        if self.modules.contains_key(id) {
            return;
        }
        let parent = id.rsplit_once(':').map(|(parent, _)| parent.to_string());
        if let Some(parent) = &parent {
            self.add_module(parent);
            if let Some(module) = self.modules.get_mut(parent) {
                module.children.insert(id.to_string());
            }
        }
        let name = id.rsplit(':').next().unwrap_or(id).to_string();
        self.modules.insert(
            id.to_string(),
            ModuleFact {
                id: id.to_string(),
                name,
                parent,
                children: BTreeSet::new(),
            },
        );
    }

    /// Returns the class's module id.
    #[must_use]
    pub fn class_module(&self, class: &str) -> Option<&str> {
        self.classes.get(class).map(|class| class.module.as_str())
    }

    /// Returns the class's package, or `(default)`.
    #[must_use]
    pub fn class_package(&self, class: &str) -> Option<&str> {
        self.classes
            .get(class)
            .map(|class| class.package.as_deref().unwrap_or("(default)"))
    }

    /// Returns the package of a symbolic or resolved type.
    #[must_use]
    pub fn package_of_type(&self, name: &str) -> String {
        package_of(name).unwrap_or_else(|| {
            if name.contains('.') {
                name.rsplit_once('.').map_or_else(
                    || "(default)".to_string(),
                    |(package, _)| package.to_string(),
                )
            } else {
                "(default)".to_string()
            }
        })
    }

    /// Returns module ids in stable order.
    #[must_use]
    pub fn module_ids(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Returns classes in a module.
    #[must_use]
    pub fn classes_in_module<'a>(&'a self, module: &str) -> Vec<&'a ClassFact> {
        self.classes
            .values()
            .filter(|class| class.module == module)
            .collect()
    }
}

fn package_of(name: &str) -> Option<String> {
    if name.contains('.') && !name.starts_with("java.") {
        name.rsplit_once('.')
            .map(|(package, _)| package.to_string())
    } else if name.contains('.') {
        name.rsplit_once('.')
            .map(|(package, _)| package.to_string())
    } else {
        None
    }
}

fn resolve_type(
    written: &str,
    package: Option<&str>,
    imports: &[crate::facts::ImportFact],
    classes: &BTreeMap<String, ClassFact>,
) -> String {
    let written = written.trim();
    if written.is_empty() {
        return String::new();
    }
    if classes.contains_key(written) {
        return written.to_string();
    }
    for import in imports {
        if import.is_static {
            continue;
        }
        if import.alias.as_deref() == Some(written) {
            return import.path.clone();
        }
        if import.path == written {
            return import.path.clone();
        }
        if import.path.ends_with(&format!(".{written}")) {
            return import.path.clone();
        }
    }
    if let Some(package) = package {
        let candidate = format!("{package}.{written}");
        if classes.contains_key(&candidate) {
            return candidate;
        }
    }
    for import in imports {
        if import.is_static || !import.path.ends_with(".*") {
            continue;
        }
        let candidate = format!("{}.{}", import.path.trim_end_matches(".*"), written);
        if classes.contains_key(&candidate) {
            return candidate;
        }
    }
    let java_lang = format!("java.lang.{written}");
    if classes.contains_key(&java_lang) || written.chars().next().is_some_and(char::is_uppercase) {
        return java_lang;
    }
    if written.contains('.') {
        return written.to_string();
    }
    package.map_or_else(
        || written.to_string(),
        |package| format!("{package}.{written}"),
    )
}

impl Project {
    /// Returns the first source path for a class.
    #[must_use]
    pub fn path_for_class(&self, class: &str) -> Option<&Path> {
        self.files.values().find_map(|file| {
            file.classes
                .iter()
                .any(|candidate| candidate.fqn == class)
                .then_some(file.path.as_path())
        })
    }

    /// Returns a module for a package declared by the project.
    #[must_use]
    pub fn package_module(&self, package: &str) -> Option<&str> {
        self.package_modules.get(package).map(String::as_str)
    }

    /// Returns a class fact by fully-qualified name.
    #[must_use]
    pub fn class(&self, name: &str) -> Option<&ClassFact> {
        self.classes.get(name)
    }
}
