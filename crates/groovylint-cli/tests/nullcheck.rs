use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_groovylint"))
}

#[test]
fn reports_nullcheck_findings_and_mode() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("Example.groovy");
    fs::write(
        &file,
        "def run() {\n    String value = null\n    return value.length()\n}\n",
    )
    .expect("write fixture");

    let output = binary()
        .arg("nullcheck")
        .arg("--format")
        .arg("json")
        .arg(&file)
        .output()
        .expect("run nullcheck");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(json["summary"]["total"], 1);
    assert_eq!(
        json["files"][0]["diagnostics"][0]["rule"],
        "NullDereference"
    );
}

#[test]
fn inferred_mode_reports_possible_null_from_a_branch() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("Example.groovy");
    fs::write(
        &file,
        "def run(boolean c) {\n    String value = null\n    if (c) { value = 'x' }\n    return value.length()\n}\n",
    )
    .expect("write fixture");

    let output = binary()
        .arg("nullcheck")
        .arg("--format")
        .arg("json")
        .arg("--mode")
        .arg("inferred")
        .arg(&file)
        .output()
        .expect("run nullcheck");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        json["files"][0]["diagnostics"][0]["rule"],
        "PossibleNullDereference"
    );
}

#[test]
fn loads_toml_contracts() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("Example.groovy");
    let contracts = temp.path().join("contracts.toml");
    fs::write(
        &file,
        "def maybe() { 'body is non-null' }\ndef run() { return maybe().length() }\n",
    )
    .expect("write fixture");
    fs::write(
        &contracts,
        "[[contracts]]\nmethod = \"maybe\"\narity = 0\nreturns = \"nullable\"\n",
    )
    .expect("write contracts");

    let output = binary()
        .arg("nullcheck")
        .arg("--format")
        .arg("json")
        .arg("--mode")
        .arg("inferred")
        .arg("--contracts")
        .arg(&contracts)
        .arg(&file)
        .output()
        .expect("run nullcheck");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        json["files"][0]["diagnostics"][0]["rule"],
        "PossibleNullDereference"
    );
}

#[test]
fn strict_mode_reports_uninitialized_nonnull_fields() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("Strict.groovy");
    fs::write(&file, "class Strict {\n    @NonNull String value\n}\n").expect("write fixture");

    let output = binary()
        .arg("nullcheck")
        .arg("--format")
        .arg("json")
        .arg("--mode")
        .arg("strict")
        .arg(&file)
        .output()
        .expect("run nullcheck");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        json["files"][0]["diagnostics"][0]["rule"],
        "FieldNotInitialized"
    );
}

#[test]
fn reads_package_info_defaults() {
    let temp = TempDir::new().expect("tempdir");
    let package_info = temp.path().join("package-info.java");
    let file = temp.path().join("Marked.groovy");
    fs::write(&package_info, "@NullMarked\npackage demo;\n").expect("write package-info");
    fs::write(
        &file,
        "package demo\nclass Marked {\n    String returnsNull() { return null }\n}\n",
    )
    .expect("write fixture");

    let output = binary()
        .arg("nullcheck")
        .arg("--format")
        .arg("json")
        .arg("--mode")
        .arg("inferred")
        .arg(&file)
        .output()
        .expect("run nullcheck");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(json["files"][0]["diagnostics"][0]["rule"], "NullReturn");
}
