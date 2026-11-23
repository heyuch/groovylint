//! Source-file ownership and line/column lookups.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::span::Span;

/// A one-based line and column position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineCol {
    /// One-based line number.
    pub line: usize,
    /// One-based column number, counted in Unicode scalar values.
    pub column: usize,
}

/// An in-memory source file with a precomputed line index.
#[derive(Clone, Debug)]
pub struct SourceFile {
    path: PathBuf,
    text: Arc<str>,
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Creates a source file and builds its line index.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );

        Self {
            path: path.into(),
            text,
            line_starts,
        }
    }

    /// Returns the source path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the full source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the number of lines in the file.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns the one-based line and column for a byte offset.
    #[must_use]
    pub fn line_col(&self, offset: usize) -> LineCol {
        let offset = offset.min(self.text.len());
        let line_index = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = self.text[line_start..offset].chars().count() + 1;

        LineCol {
            line: line_index + 1,
            column,
        }
    }

    /// Returns the text of a one-based line without its line terminator.
    #[must_use]
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        let text = &self.text[start..end];
        let text = text.strip_suffix('\n').unwrap_or(text);
        Some(text.strip_suffix('\r').unwrap_or(text))
    }

    /// Returns the byte span of a one-based line, excluding its terminator.
    #[must_use]
    pub fn line_span(&self, line: usize) -> Option<Span> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        let mut actual_end = end;
        if self.text.as_bytes().get(actual_end.saturating_sub(1)) == Some(&b'\n') {
            actual_end = actual_end.saturating_sub(1);
        }
        if self.text.as_bytes().get(actual_end.saturating_sub(1)) == Some(&b'\r') {
            actual_end = actual_end.saturating_sub(1);
        }
        Some(Span::new(start, actual_end))
    }

    /// Returns a source slice for a span.
    #[must_use]
    pub fn slice(&self, span: Span) -> &str {
        self.text.get(span.range()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::SourceFile;

    #[test]
    fn maps_offsets_to_one_based_lines_and_columns() {
        let source = SourceFile::new("Example.groovy", "def x = 1\nif (x) {}\n");
        assert_eq!(source.line_col(0).line, 1);
        assert_eq!(source.line_col(0).column, 1);
        assert_eq!(source.line_col(10).line, 2);
        assert_eq!(source.line_col(10).column, 1);
    }

    #[test]
    fn exposes_line_text_without_terminator() {
        let source = SourceFile::new("Example.groovy", "one\r\ntwo\n");
        assert_eq!(source.line_text(1), Some("one"));
        assert_eq!(source.line_text(2), Some("two"));
        let span = source.line_span(1).expect("line span");
        assert_eq!(span.len(), 3);
    }
}
