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
    assert!(!stderr.contains("not implemented yet"));
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

/// A throwaway git repository with one file per source, each containing a `Println`.
struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new(sources: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let repo = Self { dir };
        repo.git(&["init", "--quiet", "--initial-branch=develop"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "user.name", "test"]);
        for (path, body) in sources {
            repo.write(path, body);
        }
        repo.git(&["add", "--all"]);
        repo.git(&["commit", "--quiet", "-m", "init"]);
        repo
    }

    fn root(&self) -> &std::path::Path {
        self.dir.path()
    }

    fn write(&self, path: &str, body: &str) {
        let target = self.root().join(path);
        std::fs::create_dir_all(target.parent().expect("parent")).expect("create dirs");
        std::fs::write(target, body).expect("write source");
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.root())
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn run(&self, args: &[&str]) -> Output {
        binary()
            .current_dir(self.root())
            .arg("check")
            .args(args)
            .output()
            .expect("run groovylint check")
    }
}

const RULESET: &str = "<ruleset><rule class='org.codenarc.rule.logging.PrintlnRule'/></ruleset>";
const PRINT: &str = "class Example {\n    void run() {\n        println 1\n    }\n}\n";
const CLEAN: &str = "class Example {\n    void run() {\n        assert 1 == 1\n    }\n}\n";

#[test]
fn changed_baseline_checks_only_the_branch_changes() {
    let repo = Repo::new(&[
        ("module-a/src/main/groovy/A.groovy", CLEAN),
        ("module-a/src/main/groovy/B.groovy", CLEAN),
    ]);
    let ruleset = repo.root().join("rules.xml");
    std::fs::write(&ruleset, RULESET).expect("write ruleset");

    repo.git(&["checkout", "--quiet", "-b", "feature"]);
    repo.write("module-a/src/main/groovy/B.groovy", PRINT);

    let ruleset = ruleset.to_str().expect("utf-8 path");
    let all = repo.run(&["--config", ruleset, "."]);
    assert_eq!(all.status.code(), Some(1));

    let changed = repo.run(&["--config", ruleset, "--changed-baseline", "develop", "."]);
    assert_eq!(changed.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&changed.stdout);
    assert!(stdout.contains("B.groovy"), "{stdout}");
    assert!(!stdout.contains("A.groovy"), "{stdout}");
    let stderr = String::from_utf8_lossy(&changed.stderr);
    assert!(stderr.contains("1 of 2 file(s) checked"), "{stderr}");
}

#[test]
fn changed_commits_ignores_uncommitted_work() {
    let repo = Repo::new(&[("src/main/groovy/A.groovy", CLEAN)]);
    let ruleset = repo.root().join("rules.xml");
    std::fs::write(&ruleset, RULESET).expect("write ruleset");

    repo.write("src/main/groovy/A.groovy", PRINT);
    repo.git(&["commit", "--quiet", "--all", "-m", "print"]);
    repo.write("src/main/groovy/New.groovy", PRINT);

    let ruleset = ruleset.to_str().expect("utf-8 path");
    let commits = repo.run(&["--config", ruleset, "--changed-commits", "HEAD", "."]);
    assert_eq!(commits.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&commits.stdout);
    assert!(stdout.contains("A.groovy"), "{stdout}");
    assert!(
        !stdout.contains("New.groovy"),
        "untracked file must be ignored: {stdout}"
    );
}

#[test]
fn changed_files_list_selects_exactly_those_files() {
    let repo = Repo::new(&[
        ("src/main/groovy/A.groovy", PRINT),
        ("src/main/groovy/B.groovy", PRINT),
    ]);
    let ruleset = repo.root().join("rules.xml");
    std::fs::write(&ruleset, RULESET).expect("write ruleset");

    std::fs::write(
        repo.root().join("changed.txt"),
        "src/main/groovy/A.groovy\n",
    )
    .expect("list");

    let ruleset = ruleset.to_str().expect("utf-8 path");
    let output = repo.run(&["--config", ruleset, "--changed-files", "changed.txt", "."]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A.groovy"), "{stdout}");
    assert!(!stdout.contains("B.groovy"), "{stdout}");
}

#[test]
fn an_empty_changed_set_passes_without_reading_anything() {
    let repo = Repo::new(&[("src/main/groovy/A.groovy", PRINT)]);
    let ruleset = repo.root().join("rules.xml");
    std::fs::write(&ruleset, RULESET).expect("write ruleset");

    let ruleset = ruleset.to_str().expect("utf-8 path");
    let output = repo.run(&["--config", ruleset, "--changed-baseline", "HEAD", "."]);
    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nothing to do"), "{stderr}");
}

#[test]
fn an_unknown_ref_fails_instead_of_checking_nothing() {
    let repo = Repo::new(&[("src/main/groovy/A.groovy", PRINT)]);
    let output = repo.run(&["--changed-since", "no-such-branch", "."]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot resolve no-such-branch"), "{stderr}");
}

#[test]
fn a_ref_that_looks_like_an_option_is_rejected() {
    let repo = Repo::new(&[("src/main/groovy/A.groovy", PRINT)]);
    let output = repo.run(&["--changed-since=--output=/tmp/x", "."]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a valid git ref"), "{stderr}");
}
