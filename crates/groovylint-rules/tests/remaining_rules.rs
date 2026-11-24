#![allow(clippy::needless_raw_string_hashes)]

use std::collections::BTreeSet;

use groovylint_core::SourceFile;
use groovylint_rules::{Engine, RuleRegistry};
use groovylint_syntax::parser::parse;

fn lint_with(
    source: &str,
    entries: &[(&str, &[(&str, &str)])],
) -> Vec<groovylint_core::Diagnostic> {
    let source_file = SourceFile::new("Example.groovy", source);
    let parsed = parse(source).expect("source should parse");
    let registry = RuleRegistry::builtin();
    let entries: Vec<(String, std::collections::BTreeMap<String, String>)> = entries
        .iter()
        .map(|(class, properties)| {
            (
                (*class).to_string(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            )
        })
        .collect();
    let engine = Engine::from_codenarc_entries(&registry, &entries);
    engine.lint(&source_file, &parsed)
}

fn lint(source: &str, classes: &[&str]) -> Vec<groovylint_core::Diagnostic> {
    let entries: Vec<(&str, &[(&str, &str)])> =
        classes.iter().map(|class| (*class, &[][..])).collect();
    lint_with(source, &entries)
}

fn names(diagnostics: &[groovylint_core::Diagnostic]) -> BTreeSet<&str> {
    diagnostics.iter().map(|d| d.rule.as_str()).collect()
}

#[test]
fn all_in_scope_codenarc_classes_are_registered() {
    let registry = RuleRegistry::builtin();
    assert_eq!(registry.descriptors().len(), 384);
    let unique: BTreeSet<_> = registry
        .descriptors()
        .iter()
        .map(|descriptor| descriptor.meta.codenarc_class)
        .collect();
    assert_eq!(unique.len(), 384);
}

#[test]
fn groovyism_rules_cover_calls_collections_and_gstrings() {
    let source = r#"
def a = left.plus(right)
def b = new ArrayList()
def c = ["${key}": 1]
def d = obj.getName()
def e = list.collect { it.name }
"#;
    let diagnostics = lint(
        source,
        &[
            "org.codenarc.rule.groovyism.ExplicitCallToPlusMethodRule",
            "org.codenarc.rule.groovyism.ExplicitArrayListInstantiationRule",
            "org.codenarc.rule.groovyism.GStringAsMapKeyRule",
            "org.codenarc.rule.groovyism.UnnecessaryGetterRule",
            "org.codenarc.rule.unnecessary.UnnecessaryGetterRule",
            "org.codenarc.rule.unnecessary.UnnecessaryCollectCallRule",
        ],
    );
    let rules = names(&diagnostics);
    assert!(rules.contains("ExplicitCallToPlusMethod"));
    assert!(rules.contains("ExplicitArrayListInstantiation"));
    assert!(rules.contains("GStringAsMapKey"));
    assert!(rules.contains("UnnecessaryGetter"));
    assert!(rules.contains("UnnecessaryCollectCall"));
}

#[test]
fn unnecessary_rules_report_redundant_code() {
    let source = r#"
def a = "hello"
def b = "hello".toString()
def c = (1 as Integer)
def d = obj.setName("x")
def run() { return 1 }
"#;
    let diagnostics = lint(
        source,
        &[
            "org.codenarc.rule.unnecessary.UnnecessaryGStringRule",
            "org.codenarc.rule.unnecessary.UnnecessaryToStringRule",
            "org.codenarc.rule.unnecessary.UnnecessaryCastRule",
            "org.codenarc.rule.unnecessary.UnnecessarySetterRule",
            "org.codenarc.rule.unnecessary.UnnecessaryReturnKeywordRule",
        ],
    );
    let rules = names(&diagnostics);
    for expected in [
        "UnnecessaryGString",
        "UnnecessaryToString",
        "UnnecessaryCast",
        "UnnecessarySetter",
        "UnnecessaryReturnKeyword",
    ] {
        assert!(rules.contains(expected), "missing {expected}: {rules:?}");
    }
}

#[test]
fn convention_rules_report_order_and_ternary_candidates() {
    let source = r#"
class Example {
    public void first() {}
    private void second() {}
    public void third() {}
}
"#;
    let diagnostics = lint(
        source,
        &["org.codenarc.rule.convention.PublicMethodsBeforeNonPublicMethodsRule"],
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule, "PublicMethodsBeforeNonPublicMethods");

    let ternaries = r#"
def value = 1
if (value) {
    return 'yes'
} else {
    return 'no'
}
"#;
    let diagnostics = lint(
        ternaries,
        &["org.codenarc.rule.convention.IfStatementCouldBeTernaryRule"],
    );
    assert_eq!(
        names(&diagnostics),
        BTreeSet::from(["IfStatementCouldBeTernary"])
    );
}

#[test]
fn design_rules_report_boolean_null_and_nested_loops() {
    let source = r#"
class Example {
    Boolean enabled() {
        if (ready) return true
        return null
    }
    void run() {
        for (i in 1..2) {
            for (j in 1..2) { println i + j }
        }
    }
}
"#;
    let diagnostics = lint(
        source,
        &[
            "org.codenarc.rule.design.BooleanMethodReturnsNullRule",
            "org.codenarc.rule.design.NestedForLoopRule",
        ],
    );
    let rules = names(&diagnostics);
    assert!(rules.contains("BooleanMethodReturnsNull"));
    assert!(rules.contains("NestedForLoop"));
}

#[test]
fn formatting_and_import_rules_are_configurable() {
    let source = "import java.util.List\nimport static java.util.Collections.emptyList\n";
    let diagnostics = lint(
        source,
        &["org.codenarc.rule.imports.MisorderedStaticImportsRule"],
    );
    assert_eq!(
        names(&diagnostics),
        BTreeSet::from(["MisorderedStaticImports"])
    );

    let long = "def value = 'abcdefghijklmnopqrstuvwxyz'\n";
    let diagnostics = lint_with(
        long,
        &[(
            "org.codenarc.rule.formatting.LineLengthRule",
            &[("length", "20")],
        )],
    );
    assert_eq!(names(&diagnostics), BTreeSet::from(["LineLength"]));
}

#[test]
fn concurrency_rules_report_common_antipatterns() {
    let source = r#"
class Example {
    void run() {
        synchronized (this) { println 'x' }
        while (ready) { sleep(100) }
    }
}
"#;
    let diagnostics = lint(
        source,
        &[
            "org.codenarc.rule.concurrency.SynchronizedOnThisRule",
            "org.codenarc.rule.concurrency.BusyWaitRule",
        ],
    );
    let rules = names(&diagnostics);
    assert!(rules.contains("SynchronizedOnThis"));
    assert!(rules.contains("BusyWait"));
}

#[test]
fn generic_illegal_class_member_honours_properties() {
    let source = "class Example { public static int value = 1 }\n";
    let diagnostics = lint_with(
        source,
        &[(
            "org.codenarc.rule.generic.IllegalClassMemberRule",
            &[("illegalFieldModifiers", "static")],
        )],
    );
    assert_eq!(names(&diagnostics), BTreeSet::from(["IllegalClassMember"]));
}

#[test]
fn size_rules_report_parameter_count_and_complexity() {
    let source = r#"
class Example {
    void run(a, b, c, d, e, f) {
        if (a) { println a }
        while (b) { println b }
    }
}
"#;
    let diagnostics = lint_with(
        source,
        &[
            (
                "org.codenarc.rule.size.ParameterCountRule",
                &[("maxParameters", "5")],
            ),
            (
                "org.codenarc.rule.size.CyclomaticComplexityRule",
                &[("maxMethodComplexity", "1")],
            ),
        ],
    );
    let rules = names(&diagnostics);
    assert!(rules.contains("ParameterCount"));
    assert!(rules.contains("CyclomaticComplexity"));
}

#[test]
fn grouped_declarations_report_confusing_multiple_returns() {
    let source = "def first, second = [1, 2]\n";
    let diagnostics = lint(
        source,
        &["org.codenarc.rule.groovyism.ConfusingMultipleReturnsRule"],
    );
    assert_eq!(
        names(&diagnostics),
        BTreeSet::from(["ConfusingMultipleReturns"])
    );
}

#[test]
fn exception_not_thrown_and_indentation_are_reported() {
    let source = "def run() {\n    try {\n        println 'x'\n    } catch (Exception error) {\n        new RuntimeException(error)\n    }\n}\n";
    let diagnostics = lint(
        source,
        &["org.codenarc.rule.exceptions.ExceptionNotThrownRule"],
    );
    assert_eq!(names(&diagnostics), BTreeSet::from(["ExceptionNotThrown"]));

    let indentation = "def run() {\n   println 'x'\n}\n";
    let diagnostics = lint_with(
        indentation,
        &[(
            "org.codenarc.rule.formatting.IndentationRule",
            &[("spacesPerIndentLevel", "4")],
        )],
    );
    assert_eq!(names(&diagnostics), BTreeSet::from(["Indentation"]));
}

#[test]
fn inconsistent_property_synchronization_is_reported() {
    let source = r#"
class Example {
    String value
    synchronized String getValue() { value }
    void setValue(String next) { value = next }
}
"#;
    let diagnostics = lint(
        source,
        &["org.codenarc.rule.concurrency.InconsistentPropertySynchronizationRule"],
    );
    assert_eq!(
        names(&diagnostics),
        BTreeSet::from(["InconsistentPropertySynchronization"])
    );
}
