use std::path::PathBuf;
use std::process::{Command, Output};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_groovylint"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run_check(arguments: &[&str]) -> Output {
    binary()
        .arg("check")
        .args(arguments)
        .output()
        .expect("run groovylint check")
}

#[test]
fn top_level_help_lists_check_command() {
    let output = binary().arg("--help").output().expect("run --help");

    assert!(output.status.success(), "--help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Static analysis for Groovy"));
    assert!(stdout.contains("check"));
}

#[test]
fn version_is_reported() {
    let output = binary().arg("--version").output().expect("run --version");

    assert!(output.status.success(), "--version should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("groovylint"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn check_reports_code_narc_basic_violations() {
    let path = fixture("basic.groovy");
    let output = run_check(&[path.to_str().expect("utf-8 fixture path"), "--threads", "2"]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EmptyIfStatement"));
    assert!(stdout.contains("EmptyElseBlock"));
    assert!(stdout.contains("EmptyTryBlock"));
    assert!(stdout.contains("EmptyCatchBlock"));
    assert!(stdout.contains("EmptyFinallyBlock"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not implemented yet"));
}

#[test]
fn json_output_is_machine_readable() {
    let path = fixture("basic.groovy");
    let output = run_check(&[
        path.to_str().expect("utf-8 fixture path"),
        "--format",
        "json",
        "--threads",
        "4",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("JSON output should parse");
    assert_eq!(json["version"], 1);
    assert_eq!(json["summary"]["files"], 1);
    assert!(json["summary"]["warnings"].as_u64().unwrap_or(0) >= 7);
    assert_eq!(
        json["files"][0]["diagnostics"][0]["rule"],
        "EmptyIfStatement"
    );
}

#[test]
fn custom_ruleset_enables_token_rule() {
    let source = fixture("basic.groovy");
    let config = fixture("strict.xml");
    let output = run_check(&[
        source.to_str().expect("utf-8 fixture path"),
        "--config",
        config.to_str().expect("utf-8 config path"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("UnnecessarySemicolon"));
    assert!(stdout.contains("info[UnnecessarySemicolon]"));
}

#[test]
fn parallel_runs_are_deterministic() {
    let path = fixture("basic.groovy");
    let path = path.to_str().expect("utf-8 fixture path");
    let single = run_check(&[path, "--format", "json", "--threads", "1"]);
    let parallel = run_check(&[path, "--format", "json", "--threads", "4"]);
    assert_eq!(single.stdout, parallel.stdout);
}

#[test]
fn clean_file_exits_successfully() {
    let path = fixture("clean.groovy");
    let output = run_check(&[path.to_str().expect("utf-8 fixture path")]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, [] as [u8; 0]);
}

#[test]
fn parse_errors_are_reported_as_diagnostics() {
    let path = fixture("parse-error.groovy");
    let output = run_check(&[path.to_str().expect("utf-8 fixture path")]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ParseError"));
}

#[test]
fn missing_path_is_a_usage_error() {
    let output = run_check(&["/definitely/not/a/real/path"]);
    assert_eq!(output.status.code(), Some(2));
}
