use groovylint_arch::{FactPlan, ReferenceKind};
use groovylint_core::SourceFile;
use groovylint_rules::Engine;
use groovylint_syntax::parser::parse;

#[test]
fn engine_collects_facts_in_the_same_traversal() {
    let text = "package demo\nimport com.acme.Other\nclass A {\n  Other other\n  void run() { new StringBuilder() }\n}\n";
    let source = SourceFile::new("A.groovy", text);
    let parsed = parse(text).expect("source parses");
    let analysis =
        Engine::empty().lint_with_facts(&source, &parsed, "demo-module", FactPlan::all());
    let facts = analysis.facts.expect("facts");
    assert_eq!(facts.module, "demo-module");
    assert!(facts.classes.iter().any(|class| class.fqn == "demo.A"));
    assert!(
        facts
            .references
            .iter()
            .any(|reference| reference.kind == ReferenceKind::FieldType)
    );
    assert!(
        facts
            .references
            .iter()
            .any(|reference| reference.kind == ReferenceKind::New)
    );
}

#[test]
fn fact_plan_can_skip_expensive_references() {
    let text = "package demo\nimport com.acme.Other\nclass A {\n  Other other\n}\n";
    let source = SourceFile::new("A.groovy", text);
    let parsed = parse(text).expect("source parses");
    let analysis =
        Engine::empty().lint_with_facts(&source, &parsed, "demo-module", FactPlan::CLASSES);
    let facts = analysis.facts.expect("facts");
    assert_eq!(facts.classes.len(), 1);
    assert_eq!(facts.imports.len(), 0);
    assert_eq!(facts.references.len(), 0);
}
