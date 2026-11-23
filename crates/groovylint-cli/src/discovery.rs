//! File discovery.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;

/// Collects Groovy source files from explicit files and directories.
pub fn collect_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
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
            if entry.file_type().is_some_and(|kind| kind.is_file()) && is_groovy_file(entry.path())
            {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
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

#[cfg(test)]
mod tests {
    use super::is_groovy_file;
    use std::path::Path;

    #[test]
    fn recognizes_groovy_file_types() {
        assert!(is_groovy_file(Path::new("build.gradle")));
        assert!(is_groovy_file(Path::new("Jenkinsfile")));
        assert!(is_groovy_file(Path::new("Jenkinsfile.ci")));
        assert!(!is_groovy_file(Path::new("README.md")));
    }
}
