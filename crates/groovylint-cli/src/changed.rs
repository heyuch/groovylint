//! Restricting an analysis to the files a branch changed.
//!
//! Two sources are supported: a newline-separated list produced by the build (the
//! same file several tools can share), and `git` itself, which is what a CI job or
//! a `pre-commit` hook uses when there is no build script to ask.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;

/// Arguments that restrict the analyzed file set to what a branch changed.
#[derive(Debug, Args)]
#[allow(clippy::struct_field_names)]
pub struct ChangedArgs {
    /// File containing a newline-separated list of paths to analyze (`-` reads stdin).
    ///
    /// Blank lines and lines starting with `#` are ignored. Relative paths are
    /// resolved against the current directory.
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with_all = ["changed_since", "changed_baseline", "changed_commits"]
    )]
    pub changed_files: Option<PathBuf>,

    /// Three-point diff: files changed since the merge base with this ref, plus
    /// uncommitted and untracked files.
    ///
    /// `origin/develop` for "everything this branch changed".
    #[arg(
        long,
        value_name = "REF",
        conflicts_with_all = ["changed_files", "changed_commits"]
    )]
    pub changed_baseline: Option<String>,

    /// Two-point diff: files changed between this ref and the working tree,
    /// including uncommitted and untracked files.
    ///
    /// `HEAD^1` after a merge, to look only at what the merge brought in.
    #[arg(
        long,
        value_name = "REF",
        conflicts_with_all = ["changed_files", "changed_commits"]
    )]
    pub changed_since: Option<String>,

    /// Comma-separated commits: the files each one introduced, merge commits
    /// followed through their first parent. Uncommitted changes are ignored.
    #[arg(
        long,
        value_name = "REFS",
        conflicts_with_all = ["changed_files", "changed_since", "changed_baseline"]
    )]
    pub changed_commits: Option<String>,
}

/// The set of files to keep, with a description for the summary line.
pub struct ChangedSet {
    files: BTreeSet<PathBuf>,
    description: String,
    from_git: bool,
}

impl ChangedSet {
    /// Whether the file is part of the changed set.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        self.files.contains(&absolute(path))
    }

    /// Human-readable description of how the set was produced.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Whether the set came from a git range rather than from a list file.
    #[must_use]
    pub const fn is_git_range(&self) -> bool {
        self.from_git
    }
}

impl ChangedArgs {
    /// Builds the changed-file set, if one of the options was given.
    pub fn resolve(&self) -> Result<Option<ChangedSet>> {
        if let Some(path) = &self.changed_files {
            return Ok(Some(read_list(path)?));
        }
        let root = repository_root()?;
        if let Some(baseline) = &self.changed_baseline {
            let reference = validate_ref(baseline, "--changed-baseline")?;
            require_ref(&root, reference)?;
            let mut paths = git_paths(
                &root,
                &[
                    "diff",
                    "--name-only",
                    "--diff-filter=ACMR",
                    &format!("{reference}...HEAD"),
                ],
            )?;
            paths.extend(uncommitted_paths(&root)?);
            return Ok(Some(set_from_root(
                &root,
                paths,
                format!("{reference}...HEAD + working tree"),
            )));
        }
        if let Some(since) = &self.changed_since {
            let reference = validate_ref(since, "--changed-since")?;
            require_ref(&root, reference)?;
            let mut paths = git_paths(
                &root,
                &["diff", "--name-only", "--diff-filter=ACMR", reference],
            )?;
            paths.extend(uncommitted_paths(&root)?);
            return Ok(Some(set_from_root(
                &root,
                paths,
                format!("{reference} + working tree"),
            )));
        }
        let Some(commits) = &self.changed_commits else {
            return Ok(None);
        };
        let mut paths = Vec::new();
        for reference in commits
            .split(',')
            .map(str::trim)
            .filter(|ref_| !ref_.is_empty())
        {
            let reference = validate_ref(reference, "--changed-commits")?;
            require_ref(&root, reference)?;
            paths.extend(git_paths(
                &root,
                &[
                    "show",
                    "--first-parent",
                    "--root",
                    "--pretty=format:",
                    "--name-only",
                    "--diff-filter=ACMR",
                    reference,
                ],
            )?);
        }
        if paths.is_empty() {
            bail!("--changed-commits did not contain a valid commit");
        }
        Ok(Some(set_from_root(&root, paths, commits.clone())))
    }
}

/// Reads a newline-separated list of paths, resolving relatives against the cwd.
fn read_list(path: &Path) -> Result<ChangedSet> {
    let mut text = String::new();
    if path == Path::new("-") {
        std::io::stdin()
            .read_to_string(&mut text)
            .context("failed to read the changed-file list from stdin")?;
    } else {
        text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read the changed-file list {}", path.display()))?;
    }

    let mut files = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        files.insert(absolute(Path::new(line)));
    }
    Ok(ChangedSet {
        description: if path == Path::new("-") {
            "list from stdin".to_string()
        } else {
            path.display().to_string()
        },
        files,
        from_git: false,
    })
}

fn set_from_root(root: &Path, paths: Vec<String>, description: String) -> ChangedSet {
    let mut files = BTreeSet::new();
    for path in paths {
        files.insert(absolute(&root.join(path)));
    }
    ChangedSet {
        files,
        description,
        from_git: true,
    }
}

/// Files changed between `HEAD` and the working tree, plus untracked files.
fn uncommitted_paths(root: &Path) -> Result<Vec<String>> {
    let mut paths = git_paths(root, &["diff", "--name-only", "--diff-filter=ACMR", "HEAD"])?;
    paths.extend(git_paths(
        root,
        &["ls-files", "--others", "--exclude-standard"],
    )?);
    Ok(paths)
}

fn repository_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git; is it installed?")?;
    if !output.status.success() {
        bail!(
            "--changed requires a git repository: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let root = String::from_utf8(output.stdout).context("git printed a non-UTF-8 path")?;
    Ok(PathBuf::from(root.trim_end()))
}

fn require_ref(root: &Path, reference: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ])
        .output()
        .context("failed to run git")?;
    if output.status.success() {
        return Ok(());
    }
    bail!("cannot resolve {reference}: not a commit in this repository (run `git fetch` first)");
}

fn git_paths(root: &Path, args: &[&str]) -> Result<Vec<String>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let text = String::from_utf8(output.stdout).context("git printed a non-UTF-8 path")?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

/// Keeps only the files the branch changed, preserving the original order.
pub fn filter(files: Vec<PathBuf>, changed: &ChangedSet) -> Vec<PathBuf> {
    files
        .into_iter()
        .filter(|path| changed.contains(path))
        .collect()
}

/// Absolute form of a path, with symlinks resolved when the file exists.
fn absolute(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
}

/// Rejects anything git could read as an option or split into extra arguments.
fn validate_ref<'a>(reference: &'a str, option: &str) -> Result<&'a str> {
    let mut characters = reference.chars();
    let valid = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(
                    character,
                    '.' | '_' | '/' | '~' | '^' | '@' | '{' | '}' | ':' | '-'
                )
        });
    if !valid {
        bail!(
            "{option} {reference:?} is not a valid git ref: it must start with a letter or digit \
             and contain only letters, digits and . _ / ~ ^ @ {{ }} : -"
        );
    }
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::{absolute, validate_ref};
    use std::path::Path;

    #[test]
    fn accepts_ordinary_refs() {
        for reference in ["origin/develop", "HEAD^1", "main", "release/1.2", "a..b"] {
            assert!(
                validate_ref(reference, "--changed-since").is_ok(),
                "{reference}"
            );
        }
    }

    #[test]
    fn rejects_refs_that_git_would_read_as_options() {
        for reference in ["--output=/tmp/x", "-n", "", " ", "a b", "a;b", "a$(id)"] {
            assert!(
                validate_ref(reference, "--changed-since").is_err(),
                "{reference:?} should be rejected"
            );
        }
    }

    #[test]
    fn absolute_keeps_missing_relative_paths_under_the_cwd() {
        let path = absolute(Path::new("definitely/not/here.groovy"));
        assert!(path.is_absolute());
        assert!(path.ends_with("definitely/not/here.groovy"));
    }
}
