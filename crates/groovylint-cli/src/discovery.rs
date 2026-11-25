//! File discovery.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;

/// Collects Groovy source files from explicit files and directories.
///
/// `excludes` drop every path that contains one of the given substrings, which is
/// how a run mirrors `CodeNarc`'s main-only setup: `--exclude /src/test/`.
pub fn collect_files(paths: &[PathBuf], excludes: &[String]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            files.push(path.clone());
            continue;
        }
        if !path.is_dir() {
            bail!("path does not exist: {}", path.display());
        }

        let walker = WalkBuilder::new(path).standard_filters(true).build();
        for entry in walker {
            let entry = entry.with_context(|| format!("failed to walk {}", path.display()))?;
            if entry.file_type().is_some_and(|kind| kind.is_file())
                && is_groovy_file(entry.path())
                && !is_excluded(entry.path(), excludes)
            {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_excluded(path: &Path, excludes: &[String]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let text = path.to_string_lossy().replace('\\', "/");
    excludes.iter().any(|pattern| text.contains(pattern))
}

fn is_groovy_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "Jenkinsfile" || name.starts_with("Jenkinsfile."))
    {
        return true;
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "groovy" | "gvy" | "gy" | "gradle"
            )
        })
}

/// Collects the changed files directly, without walking the tree.
///
/// Walking 24,000 files to keep three of them is where a changed-only run lost its
/// advantage: the walk and the per-file path resolution cost more than parsing the
/// whole project would. A changed set is small, so it is resolved directly: keep
/// the entries that are Groovy files, still exist, and live under one of the
/// requested paths.
pub fn collect_changed(
    paths: &[PathBuf],
    files: impl Iterator<Item = PathBuf>,
    excludes: &[String],
) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for path in paths {
        if !path.exists() {
            bail!("path does not exist: {}", path.display());
        }
        roots.push(std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()));
    }

    let cwd = std::env::current_dir().ok();
    let mut selected = Vec::new();
    for file in files {
        let resolved = std::fs::canonicalize(&file).unwrap_or_else(|_| file.clone());
        if !is_groovy_file(&resolved) || !resolved.is_file() {
            continue;
        }
        if !roots.is_empty() && !roots.iter().any(|root| resolved.starts_with(root)) {
            continue;
        }
        let displayed = match &cwd {
            Some(cwd) => resolved
                .strip_prefix(cwd)
                .map_or_else(|_| resolved.clone(), PathBuf::from),
            None => resolved,
        };
        if is_excluded(&displayed, excludes) {
            continue;
        }
        selected.push(displayed);
    }

    selected.sort();
    selected.dedup();
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::{is_excluded, is_groovy_file};
    use std::path::Path;

    #[test]
    fn excludes_paths_containing_a_pattern() {
        let patterns = vec!["/src/test/".to_string()];

        assert!(is_excluded(Path::new("a/src/test/b/C.groovy"), &patterns));
        assert!(!is_excluded(Path::new("a/src/main/b/C.groovy"), &patterns));
        assert!(!is_excluded(Path::new("a/src/test/b/C.groovy"), &[]));
    }

    #[test]
    fn recognizes_groovy_file_types() {
        assert!(is_groovy_file(Path::new("build.gradle")));
        assert!(is_groovy_file(Path::new("Jenkinsfile")));
        assert!(is_groovy_file(Path::new("Jenkinsfile.ci")));
        assert!(!is_groovy_file(Path::new("README.md")));
    }
}
