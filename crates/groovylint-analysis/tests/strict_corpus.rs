//! Optional black-box corpus check.
//!
//! The public repository does not vendor groovy-nullchecker. Set
//! `GROOVYLINT_NULLCHECKER_CORPUS` to its `nullchecker/corpus` directory to run
//! this test locally:
//!
//! ```text
//! GROOVYLINT_NULLCHECKER_CORPUS=/path/to/groovy-nullchecker/nullchecker/corpus \
//!   cargo test -p groovylint-analysis --test strict_corpus
//! ```

use std::env;
use std::path::PathBuf;

use groovylint_analysis::nullability::Mode;
use groovylint_analysis::nullcheck;
use groovylint_syntax::parser::parse;

#[test]
fn strict_flow_corpus_matches_expected_counts() {
    let Some(root) = env::var_os("GROOVYLINT_NULLCHECKER_CORPUS").map(PathBuf::from) else {
        return;
    };
    let expected = [
        ("flow_access_path.groovy", 2),
        ("flow_branch_merge.groovy", 3),
        ("flow_closure_capture.groovy", 2),
        ("flow_for_nullable_element.groovy", 1),
        ("flow_local_null_deref.groovy", 1),
        ("flow_loop_report_once.groovy", 1),
        ("flow_loop_switch.groovy", 3),
        ("flow_reassigned_null.groovy", 1),
        ("flow_switch_fallthrough.groovy", 0),
        ("flow_try_catch.groovy", 1),
    ];
    for (name, expected_count) in expected {
        let path = root.join("strict").join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let parsed = parse(&text).unwrap_or_else(|error| {
            panic!("failed to parse {}: {}", path.display(), error.message)
        });
        let findings = nullcheck::analyze(&parsed.script, Mode::Inferred);
        assert_eq!(
            findings.len(),
            expected_count,
            "{}: {findings:?}",
            path.display()
        );
    }

    let annotation_expected = [
        ("nullable_deref_param.groovy", 1),
        ("nullable_return_deref.groovy", 1),
        ("nonnull_param_null.groovy", 1),
        ("nonnull_return.groovy", 1),
        ("nonnull_return_nullable_value.groovy", 1),
        ("optional_or_else.groovy", 2),
        ("safe_nav_result_deref.groovy", 1),
        ("deref_null_literal.groovy", 1),
    ];
    for (name, expected_count) in annotation_expected {
        let path = root.join("positive").join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let parsed = parse(&text).unwrap_or_else(|error| {
            panic!("failed to parse {}: {}", path.display(), error.message)
        });
        let findings = nullcheck::analyze(&parsed.script, Mode::Inferred);
        assert_eq!(
            findings.len(),
            expected_count,
            "{}: {findings:?}",
            path.display()
        );
    }

    let clean = [
        "guards_null_comparison.groovy",
        "guards_while.groovy",
        "guards_short_circuit.groovy",
        "guards_safe_navigation.groovy",
        "guards_groovy_truth.groovy",
        "guards_instanceof.groovy",
        "guards_elvis_ternary.groovy",
        "guards_early_exit.groovy",
        "guards_assert.groovy",
        "guards_test_assertions.groovy",
        "guards_validators.groovy",
        "no_false_positive_clean.groovy",
        "guards_contract.groovy",
        "guards_ensures_nonnull.groovy",
        "guards_ensures_nonnull_if.groovy",
        "assertion_contract.groovy",
    ];
    for name in clean {
        let path = root.join("negative").join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let parsed = parse(&text).unwrap_or_else(|error| {
            panic!("failed to parse {}: {}", path.display(), error.message)
        });
        let findings = nullcheck::analyze(&parsed.script, Mode::Inferred);
        assert!(findings.is_empty(), "{}: {findings:?}", path.display());
    }
}
