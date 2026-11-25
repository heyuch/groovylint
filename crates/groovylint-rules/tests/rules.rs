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
