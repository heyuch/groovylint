//! `CodeNarc` import rules.

use std::collections::BTreeSet;

use groovylint_core::Severity;
use groovylint_syntax::ast::{Annotation, ExprKind, ImportDecl, Literal, Script, Stmt};
use groovylint_syntax::token::{Token, TokenKind};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::NodeRef;
use crate::rule::{
    Category, DiagnosticSink, FileContext, FileRule, Rule, RuleMeta, RuleProperties,
};
use crate::traverse::{self, AstVisitor};

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

/// Flags imports no reference in the file resolves to.
///
/// `CodeNarc` compares simple names textually, which counts a member call on
/// another object (`order.verify(..)`) as a use of `import static ...verify` and
/// misses property aliases. This resolves the identifiers the file actually
/// binds instead: a name counts only where it is used as a type, a bare
/// reference, or a member of an implicit receiver.
pub struct UnusedImportRule;

impl Rule for UnusedImportRule {}

impl FileRule for UnusedImportRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut collector = ReferenceCollector::default();
        traverse::walk_script(context.script, &mut collector);
        collector.collect_tokens(context.tokens, context.source.text(), context.script);

        for import in &context.script.imports {
            // A wildcard can only be judged against the target's members, which
            // a single file does not know about; `CodeNarc` skips them too.
            if is_wildcard(import) {
                continue;
            }
            if is_resolved(import, &collector) {
                continue;
            }
            sink.report(
                import.span,
                format!("The [{}] import is never referenced", import.path),
            );
        }
    }
}

/// Whether any name this import binds is referenced by the file.
fn is_resolved(import: &ImportDecl, collector: &ReferenceCollector) -> bool {
    let mut candidates = vec![alias_of(import)];
    if import.is_static
        && let Some(property) = accessor_property(&alias_of(import))
    {
        candidates.push(property);
    }
    candidates.iter().any(|candidate| {
        // A local declaration of the same name makes the reference ambiguous;
        // assuming the import is used keeps this rule free of false positives.
        collector.references.contains(candidate) || collector.declarations.contains(candidate)
    })
}

/// The names a file references and the names it declares.
#[derive(Default)]
struct ReferenceCollector {
    references: BTreeSet<String>,
    declarations: BTreeSet<String>,
}

impl ReferenceCollector {
    fn reference(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty() {
            self.references.insert(name.to_string());
        }
    }

    /// Records every identifier inside a written type or annotation argument.
    fn reference_words(&mut self, text: &str) {
        let words: Vec<String> = text
            .split(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '$'))
            .filter(|word| !word.is_empty())
            .map(ToString::to_string)
            .collect();
        for word in words {
            self.reference(&word);
        }
    }

    /// Collects the identifiers the tree does not model, such as generic
    /// arguments (`List<Foo>`) and DSL calls. Qualified member names
    /// (`other.name`) and map keys are skipped: they never refer to an import.
    fn collect_tokens(&mut self, tokens: &[Token], source: &str, script: &Script) {
        let import_ranges: Vec<std::ops::Range<usize>> = script
            .imports
            .iter()
            .map(|import| import.span.range())
            .collect();
        for (index, token) in tokens.iter().enumerate() {
            if token.kind != TokenKind::Ident {
                continue;
            }
            let range = token.span.range();
            if import_ranges
                .iter()
                .any(|import_range| import_range.contains(&range.start))
            {
                continue;
            }
            let previous = index.checked_sub(1).map(|index| tokens[index].kind);
            let next = tokens.get(index + 1).map(|token| token.kind);
            if previous == Some(TokenKind::Dot) {
                continue;
            }
            if next == Some(TokenKind::Colon)
                && matches!(
                    previous,
                    Some(
                        TokenKind::LeftBracket
                            | TokenKind::LeftParen
                            | TokenKind::Comma
                            | TokenKind::Newline
                    )
                )
            {
                continue;
            }
            self.reference(&source[range]);
        }
    }

    fn enter_annotations(&mut self, details: &[Annotation], names: &[String]) {
        for annotation in names {
            self.reference_words(annotation);
        }
        for annotation in details {
            self.reference_words(&annotation.name);
            if let Some(arguments) = &annotation.raw_arguments {
                self.reference_words(arguments);
            }
        }
    }

    fn enter_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => {
                self.declarations.insert(decl.name.clone());
                if let Some(type_name) = &decl.type_name {
                    self.reference_words(type_name);
                }
                self.enter_annotations(&decl.annotation_details, &decl.annotations);
            }
            Stmt::ForEach(for_each) => {
                self.declarations.insert(for_each.var_name.clone());
                if let Some(type_name) = &for_each.var_type {
                    self.reference_words(type_name);
                }
            }
            Stmt::Try(try_stmt) => {
                for catch in &try_stmt.catches {
                    if let Some(name) = &catch.param_name {
                        self.declarations.insert(name.clone());
                    }
                    for type_name in &catch.type_names {
                        self.reference_words(type_name);
                    }
                }
            }
            Stmt::Block(_)
            | Stmt::DoWhile(_)
            | Stmt::Expr(_)
            | Stmt::If(_)
            | Stmt::While(_)
            | Stmt::For(_)
            | Stmt::Synchronized(_)
            | Stmt::Switch(_)
            | Stmt::Return(_)
            | Stmt::Throw(_)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Assert(_)
            | Stmt::Empty(_) => {}
        }
    }

    fn enter_expr(&mut self, kind: &ExprKind) {
        match kind {
            ExprKind::Variable(name) => self.reference(name),
            ExprKind::Call {
                receiver: None,
                name,
                ..
            } => self.reference(name.trim_start_matches('&')),
            ExprKind::New { type_name, .. } | ExprKind::Cast { type_name, .. } => {
                self.reference_words(type_name);
            }
            ExprKind::Closure { params, .. } => {
                for param in params {
                    self.declarations.insert(param.name.clone());
                    if let Some(type_name) = &param.type_name {
                        self.reference_words(type_name);
                    }
                }
            }
            // Only an interpolated string refers to names; plain text does not.
            ExprKind::Literal(Literal::String(text)) if text.contains('$') => {
                self.reference_words(text);
            }
            ExprKind::Literal(_)
            | ExprKind::This
            | ExprKind::Super
            | ExprKind::Call { .. }
            | ExprKind::Property { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Assign { .. }
            | ExprKind::Ternary { .. }
            | ExprKind::List(_)
            | ExprKind::Map(_)
            | ExprKind::Index { .. }
            | ExprKind::NamedArg { .. }
            | ExprKind::Paren(_)
            | ExprKind::Switch { .. } => {}
        }
    }
}

impl AstVisitor for ReferenceCollector {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Expr(expr) => self.enter_expr(&expr.kind),
            NodeRef::Class(class) => {
                self.declarations.insert(class.name.clone());
                if let Some(parent) = &class.extends {
                    self.reference_words(parent);
                }
                for interface in &class.implements {
                    self.reference_words(interface);
                }
                self.enter_annotations(&class.annotation_details, &class.annotations);
            }
            NodeRef::Method(method) => {
                self.declarations.insert(method.name.clone());
                if let Some(return_type) = &method.return_type {
                    self.reference_words(return_type);
                }
                for param in &method.params {
                    self.declarations.insert(param.name.clone());
                    if let Some(type_name) = &param.type_name {
                        self.reference_words(type_name);
                    }
                    self.enter_annotations(&param.annotation_details, &param.annotations);
                }
                for thrown in &method.throws {
                    self.reference_words(thrown);
                }
                self.enter_annotations(&method.annotation_details, &method.annotations);
            }
            NodeRef::Field(field) => {
                self.declarations.insert(field.name.clone());
                if let Some(type_name) = &field.type_name {
                    self.reference_words(type_name);
                }
                self.enter_annotations(&field.annotation_details, &field.annotations);
            }
            NodeRef::Stmt(stmt) => self.enter_stmt(stmt),
            NodeRef::Script(_)
            | NodeRef::Initializer(_)
            | NodeRef::Block(_)
            | NodeRef::Finally(_) => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
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
    use super::{accessor_property, is_auto_imported, package_name};

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
    fn accessors_map_back_to_properties() {
        assert_eq!(accessor_property("getFoo"), Some("foo".to_string()));
        assert_eq!(accessor_property("isActive"), Some("active".to_string()));
        assert_eq!(accessor_property("run"), None);
        // `Introspector.decapitalize` leaves an all-caps property alone.
        assert_eq!(
            accessor_property("getDATA_TYPE"),
            Some("DATA_TYPE".to_string())
        );
        assert_eq!(
            accessor_property("getPLACEHOLDER"),
            Some("PLACEHOLDER".to_string())
        );
        assert_eq!(accessor_property("getXmlName"), Some("xmlName".to_string()));
    }
}
