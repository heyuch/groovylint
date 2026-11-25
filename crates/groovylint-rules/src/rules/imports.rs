//! `CodeNarc` import rules.

use std::collections::BTreeSet;

use groovylint_core::Severity;
use groovylint_syntax::ast::ImportDecl;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    Category, DiagnosticSink, FileContext, FileRule, Rule, RuleMeta, RuleProperties,
};

/// Packages whose classes Groovy imports implicitly.
const AUTO_IMPORTED_PACKAGES: &[&str] = &[
    "java.lang",
    "java.util",
    "java.io",
    "java.net",
    "groovy.lang",
    "groovy.util",
];

/// Classes Groovy imports implicitly.
const AUTO_IMPORTED_CLASSES: &[&str] = &["java.math.BigDecimal", "java.math.BigInteger"];

/// Flags wildcard imports.
#[derive(Default)]
pub struct NoWildcardImportsRule {
    ignore_static_imports: bool,
    ignore_imports: bool,
}

impl Rule for NoWildcardImportsRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        self.ignore_static_imports = bool_property(properties, "ignoreStaticImports")?;
        self.ignore_imports = bool_property(properties, "ignoreImports")?;
        Ok(())
    }
}

impl FileRule for NoWildcardImportsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for import in &context.script.imports {
            if import.is_static && self.ignore_static_imports {
                continue;
            }
            if !import.is_static && self.ignore_imports {
                continue;
            }
            if !is_wildcard(import) {
                continue;
            }
            sink.report(import.span, "Wildcard (star) import");
        }
    }
}

/// Flags imports of classes Groovy imports by default.
pub struct UnnecessaryGroovyImportRule;

impl Rule for UnnecessaryGroovyImportRule {}

impl FileRule for UnnecessaryGroovyImportRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for import in &context.script.imports {
            if import.is_static {
                continue;
            }
            if !is_auto_imported(&import.path) {
                continue;
            }
            if has_renaming_alias(import) {
                continue;
            }
            sink.report(
                import.span,
                format!("The [{}] import is unnecessary", import.path),
            );
        }
    }
}

/// Flags imports from the package of the importing file.
pub struct ImportFromSamePackageRule;

impl Rule for ImportFromSamePackageRule {}

impl FileRule for ImportFromSamePackageRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let Some(package) = &context.script.package else {
            return;
        };
        for import in &context.script.imports {
            if import.is_static || is_wildcard(import) || has_renaming_alias(import) {
                continue;
            }
            if package_name(&import.path) != package.name {
                continue;
            }
            sink.report(
                import.span,
                format!("The [{}] import is from the same package", import.path),
            );
        }
    }
}

/// Flags imports from the `sun.*` packages.
pub struct ImportFromSunPackagesRule;

impl Rule for ImportFromSunPackagesRule {}

impl FileRule for ImportFromSunPackagesRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for import in &context.script.imports {
            if import.is_static {
                continue;
            }
            if import.path.starts_with("sun.") {
                sink.report(
                    import.span,
                    format!(
                        "The file imports {}, which is not portable and likely to change",
                        import.path
                    ),
                );
            }
        }
    }
}

/// Flags imports whose name never appears elsewhere in the file.
pub struct UnusedImportRule;

impl Rule for UnusedImportRule {}

impl FileRule for UnusedImportRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let lines: Vec<&str> = (1..=context.source.line_count())
            .filter_map(|line| context.source.line_text(line))
            .collect();

        for import in &context.script.imports {
            if is_wildcard(import) {
                continue;
            }
            if is_referenced(import, &lines) {
                continue;
            }
            sink.report(
                import.span,
                format!("The [{}] import is never referenced", import.path),
            );
        }
    }
}

fn is_referenced(import: &ImportDecl, lines: &[&str]) -> bool {
    let alias = alias_of(import);
    if referenced_name(&alias, lines) {
        return true;
    }
    if !import.is_static {
        return false;
    }
    match accessor_property(&alias) {
        Some(property) => referenced_name(&property, lines),
        None => false,
    }
}

/// Finds `alias` outside import statements, ignoring `aliasLongerName` and
/// `otherAlias`. This mirrors `CodeNarc`'s textual check, so a reference that
/// only appears inside a longer word does not count.
fn referenced_name(name: &str, lines: &[&str]) -> bool {
    for line in lines {
        if is_import_line(line) {
            continue;
        }
        if contains_name(line, name) {
            return true;
        }
    }
    false
}

fn is_import_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("import ") || trimmed.starts_with("import\t")
}

fn contains_name(line: &str, name: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = line[offset..].find(name) {
        let start = offset + index;
        let preceded_by_name_char = start
            .checked_sub(1)
            .and_then(|previous| line.as_bytes().get(previous))
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'));
        let followed_by_name_char = line
            .as_bytes()
            .get(start + name.len())
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'));
        // A qualified reference (`a.b.Foo`) does not need the import.
        let qualified = start
            .checked_sub(1)
            .and_then(|previous| line.as_bytes().get(previous))
            == Some(&b'.');
        if !preceded_by_name_char && !followed_by_name_char && !qualified {
            return true;
        }
        offset = start + name.len();
    }
    false
}

/// Maps a static import of a getter/setter back to the property it accesses.
fn accessor_property(alias: &str) -> Option<String> {
    let getter = alias.strip_prefix("get");
    let setter = alias.strip_prefix("set");
    let test = alias.strip_prefix("is");
    let property = getter.or(setter).or(test)?;
    let mut chars = property.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    // `Introspector.decapitalize`: `getDATA_TYPE` is the property `DATA_TYPE`,
    // `getValue` is `value`. Decapitalizing blindly turns the first into
    // `dATA_TYPE` and hides every use of the property.
    let second_is_uppercase = rest.chars().next().is_some_and(char::is_uppercase);
    if first.is_uppercase() && second_is_uppercase {
        return Some(property.to_string());
    }
    Some(format!("{}{}", first.to_ascii_lowercase(), rest))
}

fn alias_of(import: &ImportDecl) -> String {
    match &import.alias {
        Some(alias) => alias.clone(),
        None => simple_name(&import.path).to_string(),
    }
}

fn has_renaming_alias(import: &ImportDecl) -> bool {
    let Some(alias) = &import.alias else {
        return false;
    };
    alias != simple_name(&import.path)
}

fn is_wildcard(import: &ImportDecl) -> bool {
    import.path.ends_with(".*")
}

fn is_auto_imported(path: &str) -> bool {
    AUTO_IMPORTED_CLASSES.contains(&path) || AUTO_IMPORTED_PACKAGES.contains(&package_name(path))
}

fn package_name(path: &str) -> &str {
    if let Some(package) = path.strip_suffix(".*") {
        return package;
    }
    match path.rfind('.') {
        Some(index) => &path[..index],
        None => "",
    }
}

fn simple_name(path: &str) -> &str {
    match path.rfind('.') {
        Some(index) => &path[index + 1..],
        None => path,
    }
}

fn bool_property(properties: &RuleProperties, name: &str) -> Result<bool, String> {
    match properties.get(name) {
        None => Ok(false),
        Some(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        Some(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        Some(value) => Err(format!(
            "property `{name}` must be true or false, got `{value}`"
        )),
    }
}

/// Flags repeated import statements.
pub struct DuplicateImportRule;

impl Rule for DuplicateImportRule {}

impl FileRule for DuplicateImportRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut seen = BTreeSet::new();
        for import in &context.script.imports {
            // `CodeNarc` compares the imported class, so two aliases of the same
            // class are a duplicate too.
            let key = (import.path.clone(), import.is_static);
            if seen.insert(key) {
                continue;
            }
            sink.report(
                import.span,
                format!("The import [{}] is duplicated", import.path),
            );
        }
    }
}

/// Flags static imports that come after normal imports.
pub struct MisorderedStaticImportsRule {
    comes_before: bool,
}

impl Default for MisorderedStaticImportsRule {
    fn default() -> Self {
        Self { comes_before: true }
    }
}

impl Rule for MisorderedStaticImportsRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("comesBefore") {
            self.comes_before = value != "false";
        }
        Ok(())
    }
}

impl FileRule for MisorderedStaticImportsRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut normal_found = false;
        for import in &context.script.imports {
            if self.comes_before {
                if !import.is_static {
                    normal_found = true;
                } else if normal_found {
                    sink.report(
                        import.span,
                        "Static imports should appear before normal imports",
                    );
                }
            } else if import.is_static {
                normal_found = true;
            } else if normal_found {
                sink.report(
                    import.span,
                    "Normal imports should appear before static imports",
                );
            }
        }
    }
}

/// Descriptors for import rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "MisorderedStaticImports",
                "org.codenarc.rule.imports.MisorderedStaticImportsRule",
                Category::Imports,
                Severity::Info,
                "Static and normal imports should be ordered consistently.",
            ),
            || RuleInstanceImpl::File(Box::new(MisorderedStaticImportsRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "NoWildcardImports",
                "org.codenarc.rule.imports.NoWildcardImportsRule",
                Category::Imports,
                Severity::Info,
                "Checks for wildcard (star) imports.",
            ),
            || RuleInstanceImpl::File(Box::new(NoWildcardImportsRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "UnnecessaryGroovyImport",
                "org.codenarc.rule.imports.UnnecessaryGroovyImportRule",
                Category::Imports,
                Severity::Info,
                "Checks for imports of packages Groovy imports by default.",
            ),
            || RuleInstanceImpl::File(Box::new(UnnecessaryGroovyImportRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ImportFromSamePackage",
                "org.codenarc.rule.imports.ImportFromSamePackageRule",
                Category::Imports,
                Severity::Info,
                "Checks for imports from the package of the importing file.",
            ),
            || RuleInstanceImpl::File(Box::new(ImportFromSamePackageRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ImportFromSunPackages",
                "org.codenarc.rule.imports.ImportFromSunPackagesRule",
                Category::Imports,
                Severity::Warning,
                "Checks for imports from the `sun.*` packages.",
            ),
            || RuleInstanceImpl::File(Box::new(ImportFromSunPackagesRule)),
        ),
        descriptor(
            RuleMeta::new(
                "DuplicateImport",
                "org.codenarc.rule.imports.DuplicateImportRule",
                Category::Imports,
                Severity::Warning,
                "Checks for duplicate imports.",
            ),
            || RuleInstanceImpl::File(Box::new(DuplicateImportRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnusedImport",
                "org.codenarc.rule.imports.UnusedImportRule",
                Category::Imports,
                Severity::Info,
                "Checks for imports that are never referenced.",
            ),
            || RuleInstanceImpl::File(Box::new(UnusedImportRule)),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{accessor_property, contains_name, is_auto_imported, package_name};

    #[test]
    fn package_name_ignores_wildcards_and_classes() {
        assert_eq!(package_name("foo.bar.Baz"), "foo.bar");
        assert_eq!(package_name("foo.bar.*"), "foo.bar");
        assert_eq!(package_name("Baz"), "");
    }

    #[test]
    fn auto_imported_covers_packages_and_single_classes() {
        assert!(is_auto_imported("java.util.List"));
        assert!(is_auto_imported("java.math.BigDecimal"));
        assert!(!is_auto_imported("java.math.RoundingMode"));
        assert!(!is_auto_imported("foo.Bar"));
    }

    #[test]
    fn name_matches_only_at_a_word_boundary() {
        assert!(contains_name("def value = Bar.create()", "Bar"));
        assert!(contains_name("def value = ${Bar.x}", "Bar"));
        assert!(!contains_name("def value = BarFactory.create()", "Bar"));
        assert!(!contains_name("def value = myBar.create()", "Bar"));
    }

    #[test]
    fn accessors_map_back_to_properties() {
        assert_eq!(accessor_property("getFoo"), Some("foo".to_string()));
        assert_eq!(accessor_property("isActive"), Some("active".to_string()));
        assert_eq!(accessor_property("run"), None);
    }
}
