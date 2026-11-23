//! Byte spans in source text.

use std::ops::Range;

/// A half-open byte range in a source file.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Span {
    /// Start byte offset, inclusive.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
}

impl Span {
    /// Creates a span from byte offsets.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns an empty span at `offset`.
    #[must_use]
    pub const fn empty(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Returns the length of the span in bytes.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns whether the span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    /// Returns the smallest span covering both inputs.
    #[must_use]
    pub fn cover(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns the span as a `Range<usize>`.
    #[must_use]
    pub const fn range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Returns whether `offset` is inside the span.
    #[must_use]
    pub const fn contains(self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::Span;

    #[test]
    fn cover_uses_outer_bounds() {
        let span = Span::new(4, 8).cover(Span::new(2, 6));
        assert_eq!(span, Span::new(2, 8));
    }

    #[test]
    fn empty_span_contains_nothing() {
        let span = Span::empty(3);
        assert!(span.is_empty());
        assert!(!span.contains(3));
    }
}
