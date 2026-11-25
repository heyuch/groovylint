use std::collections::BTreeSet;

use groovylint_core::SourceFile;
use groovylint_rules::{Engine, RuleRegistry};
use groovylint_syntax::parser::parse;

fn lint(source: &str, classes: &[&str]) -> Vec<groovylint_core::Diagnostic> {
    let source_file = SourceFile::new("Example.groovy", source);
    let parsed = parse(source).expect("source should parse");
    let registry = RuleRegistry::builtin();
    let class_names: Vec<String> = classes.iter().map(ToString::to_string).collect();
    let engine = Engine::from_codenarc_classes(&registry, &class_names);
    engine.lint(&source_file, &parsed)
}

fn rules(diagnostics: &[groovylint_core::Diagnostic]) -> BTreeSet<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule.as_str())
        .collect()
}

#[test]
fn reports_all_empty_block_rules() {
    let source = r"
class Example {
    void run() {
        if (true) {
        } else {
        }
        while (false) {
        }
        for (def i = 0; i < 3; i++) {
        }
        try {
        } catch (Exception error) {
        } finally {
        }
    }
}
";
    let diagnostics = lint(
        source,
        &[
            "org.codenarc.rule.basic.EmptyIfStatementRule",
            "org.codenarc.rule.basic.EmptyElseBlockRule",
            "org.codenarc.rule.basic.EmptyWhileStatementRule",
            "org.codenarc.rule.basic.EmptyForStatementRule",
            "org.codenarc.rule.basic.EmptyTryBlockRule",
            "org.codenarc.rule.basic.EmptyCatchBlockRule",
            "org.codenarc.rule.basic.EmptyFinallyBlockRule",
        ],
    );

    assert_eq!(diagnostics.len(), 7);
    assert_eq!(
        rules(&diagnostics),
        BTreeSet::from([
            "EmptyIfStatement",
            "EmptyElseBlock",
            "EmptyWhileStatement",
            "EmptyForStatement",
            "EmptyTryBlock",
            "EmptyCatchBlock",
            "EmptyFinallyBlock",
        ])
    );
}

#[test]
fn does_not_report_non_empty_blocks() {
    let source = r#"
if (true) {
    println "ok"
} else {
    println "no"
}
while (false) {
    println "loop"
}
for (def i = 0; i < 1; i++) {
    println i
}
try {
    println "try"
} catch (Exception error) {
    println error
} finally {
    println "done"
}
"#;
    let diagnostics = lint(
        source,
        &[
            "org.codenarc.rule.basic.EmptyIfStatementRule",
            "org.codenarc.rule.basic.EmptyElseBlockRule",
            "org.codenarc.rule.basic.EmptyWhileStatementRule",
            "org.codenarc.rule.basic.EmptyForStatementRule",
            "org.codenarc.rule.basic.EmptyTryBlockRule",
            "org.codenarc.rule.basic.EmptyCatchBlockRule",
            "org.codenarc.rule.basic.EmptyFinallyBlockRule",
        ],
    );
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
}

#[test]
fn unnecessary_semicolon_flags_line_endings_only() {
    let diagnostics = lint(
        "def value = 1;\nfor (def i = 0; i < 3; i++) { println i }\n",
        &["org.codenarc.rule.unnecessary.UnnecessarySemicolonRule"],
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule, "UnnecessarySemicolon");
    assert_eq!(diagnostics[0].span.start, 13);
}

#[test]
fn unknown_rules_are_reported_as_unsupported() {
    let registry = RuleRegistry::builtin();
    let class_names = vec!["org.codenarc.rule.basic.NotImplementedRule".to_string()];
    let engine = Engine::from_codenarc_classes(&registry, &class_names);
    assert_eq!(engine.rule_count(), 0);
    assert_eq!(engine.unsupported().len(), 1);
}

#[test]
fn interface_fields_are_implicitly_static_and_final() {
    // Interface members are constants, so the property-name rule must use the
    // constant pattern instead of the lowercase one.
    let diagnostics = lint(
        "interface Names {\n    String NAMESPACE = 'x'\n}\n",
        &["org.codenarc.rule.naming.PropertyNameRule"],
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn unsafe_implementation_as_map_skips_test_sources() {
    let source = "class Example {\n    void run() {\n        def statement = [setString: { a, b -> }] as PreparedStatement\n    }\n}\n";
    let parsed = parse(source).expect("source should parse");
    let registry = RuleRegistry::builtin();
    let engine = Engine::from_codenarc_classes(
        &registry,
        &["org.codenarc.rule.security.UnsafeImplementationAsMapRule".to_string()],
    );

    let test_file = SourceFile::new("src/test/groovy/ExampleTest.groovy", source);
    assert_eq!(engine.lint(&test_file, &parsed).len(), 0);

    let main_file = SourceFile::new("src/main/groovy/Example.groovy", source);
    assert_eq!(engine.lint(&main_file, &parsed).len(), 1);
}

#[test]
fn parity_regressions_from_the_corpus_comparison() {
    // Map literals with shorthand keys (`[legacy: 'kept']`).
    let duplicates = lint(
        "class Example {\n    void run() {\n        def a = [legacy: 'kept']\n        def b = [legacy: 'kept']\n    }\n}\n",
        &["org.codenarc.rule.dry.DuplicateMapLiteralRule"],
    );
    assert_eq!(duplicates.len(), 1, "{duplicates:?}");

    // Wildcard imports of default packages are unnecessary.
    let wildcard = lint(
        "import java.util.*\n\nclass Example {\n}\n",
        &["org.codenarc.rule.imports.UnnecessaryGroovyImportRule"],
    );
    assert_eq!(wildcard.len(), 1, "{wildcard:?}");

    // Two aliases of one class are a duplicate import.
    let aliased = lint(
        "import a.Foo\nimport a.Foo as Bar\n\nclass Example {\n}\n",
        &["org.codenarc.rule.imports.DuplicateImportRule"],
    );
    assert_eq!(aliased.len(), 1, "{aliased:?}");

    // `super.finalize()` is a call to finalize().
    let finalize = lint(
        "class Example {\n    protected void finalize() {\n        super.finalize()\n    }\n}\n",
        &["org.codenarc.rule.security.ObjectFinalizeRule"],
    );
    assert_eq!(finalize.len(), 1, "{finalize:?}");

    // Abstract/interface methods without a return type use `def`.
    let nodef = lint(
        "interface Example {\n    toDO(@MappingTarget Object target, Object param)\n}\n",
        &["org.codenarc.rule.convention.NoDefRule"],
    );
    assert_eq!(nodef.len(), 1, "{nodef:?}");

    // Enum constructors are implicitly private.
    let enum_ctor = lint(
        "enum Example {\n    A('a', 'aa')\n\n    String value\n    String desc\n\n    Example(String value, String desc) {\n    }\n}\n",
        &["org.codenarc.rule.unused.UnusedPrivateMethodParameterRule"],
    );
    assert!(!enum_ctor.is_empty(), "{enum_ctor:?}");

    // A constructor call as the body of an arrow lambda is reported.
    let lambda = lint(
        "class Example {\n    void run() {\n        def f = () -> new RuntimeException('x')\n    }\n}\n",
        &["org.codenarc.rule.unused.UnusedObjectRule"],
    );
    assert_eq!(lambda.len(), 1, "{lambda:?}");

    // JDBC references include the import line.
    let jdbc = lint(
        "import java.sql.Connection\n\nclass Example {\n}\n",
        &["org.codenarc.rule.jdbc.JdbcConnectionReferenceRule"],
    );
    assert_eq!(jdbc.len(), 1, "{jdbc:?}");
}

#[test]
fn static_getter_import_is_used_by_its_property() {
    // `getDATA_TYPE` is the property `DATA_TYPE` (JavaBeans does not
    // decapitalize when the second character is uppercase).
    let findings = lint(
        "import static a.Consts.getDATA_TYPE\n\nclass Example {\n    String run() {\n        return message(DATA_TYPE)\n    }\n}\n",
        &["org.codenarc.rule.imports.UnusedImportRule"],
    );
    assert!(findings.is_empty(), "{findings:?}");

    let unused = lint(
        "import static a.Consts.getDATA_TYPE\n\nclass Example {\n}\n",
        &["org.codenarc.rule.imports.UnusedImportRule"],
    );
    assert_eq!(unused.len(), 1, "{unused:?}");
}

#[test]
fn unused_import_resolves_names_instead_of_matching_text() {
    let unused_import = &["org.codenarc.rule.imports.UnusedImportRule"];

    // A name only mentioned in a comment or a plain string does not use it.
    let commented = lint(
        "import a.b.Foo\n\nclass Example {\n    // Foo is great\n    String run() {\n        return 'Foo'\n    }\n}\n",
        unused_import,
    );
    assert_eq!(commented.len(), 1, "{commented:?}");

    // A member call on another object is not a use of a static import.
    let member = lint(
        "import static org.mockito.Mockito.verify\n\nclass Example {\n    void run() {\n        order.verify(attempts)\n    }\n}\n",
        unused_import,
    );
    assert_eq!(member.len(), 1, "{member:?}");

    // Generic arguments, annotations and interpolated strings do use names.
    let resolved = lint(
        "import a.b.Foo\nimport a.b.Anno\nimport a.b.Bar\n\nclass Example {\n    @Anno\n    List<Foo> run() {\n        return \"${Bar.NAME}\".length()\n    }\n}\n",
        unused_import,
    );
    assert!(resolved.is_empty(), "{resolved:?}");

    // A wildcard can be satisfied by any member of the target.
    let wildcard = lint("import a.b.*\n\nclass Example {\n}\n", unused_import);
    assert!(wildcard.is_empty(), "{wildcard:?}");
}
