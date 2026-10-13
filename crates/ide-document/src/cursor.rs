//! Cursor positions and selections use Unicode scalar-value offsets.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub anchor: usize,
    pub head: usize,
}

impl Cursor {
    #[must_use]
    pub const fn new(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    #[must_use]
    pub fn selection(self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    #[must_use]
    pub fn has_selection(self) -> bool {
        self.anchor != self.head
    }

    pub fn collapse_to(&mut self, offset: usize) {
        self.anchor = offset;
        self.head = offset;
    }

    pub fn extend_to(&mut self, offset: usize) {
        self.head = offset;
    }
}

/// Ordered cursor collection for eventual multi-cursor editing.
///
/// Identical anchor/head pairs are deduplicated. Overlapping selections are
/// retained until batched edit semantics are introduced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSet {
    cursors: Vec<Cursor>,
}

impl CursorSet {
    #[must_use]
    pub fn new(primary: Cursor) -> Self {
        Self {
            cursors: vec![primary],
        }
    }

    #[must_use]
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn add(&mut self, cursor: Cursor) {
        if !self.cursors.contains(&cursor) {
            self.cursors.push(cursor);
        }
    }

    /// Return ascending, non-overlapping selections for safe batched editing.
    ///
    /// Adjacent selections are allowed; overlapping selections and duplicate
    /// ranges are rejected instead of silently editing the same text twice.
    #[must_use]
    pub fn non_overlapping_ranges(&self) -> Option<Vec<Range<usize>>> {
        let mut ranges: Vec<_> = self
            .cursors
            .iter()
            .map(|cursor| cursor.selection())
            .collect();
        ranges.sort_by_key(|range| (range.start, range.end));
        if ranges
            .windows(2)
            .any(|pair| pair[0].end > pair[1].start || pair[0] == pair[1])
        {
            return None;
        }
        Some(ranges)
    }

    pub fn clear_secondary(&mut self) {
        self.cursors.truncate(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batched_ranges_reject_overlaps() {
        let mut cursors = CursorSet::new(Cursor { anchor: 7, head: 4 });
        cursors.add(Cursor { anchor: 1, head: 3 });
        assert_eq!(cursors.non_overlapping_ranges(), Some(vec![1..3, 4..7]));
        cursors.add(Cursor { anchor: 5, head: 8 });
        assert_eq!(cursors.non_overlapping_ranges(), None);
    }

    #[test]
    fn cursor_set_deduplicates_and_preserves_primary() {
        let mut cursors = CursorSet::new(Cursor::new(3));
        cursors.add(Cursor::new(3));
        cursors.add(Cursor::new(8));
        assert_eq!(cursors.cursors(), &[Cursor::new(3), Cursor::new(8)]);
        cursors.clear_secondary();
        assert_eq!(cursors.cursors(), &[Cursor::new(3)]);
    }

    #[test]
    fn selections_are_ordered_even_when_reversed() {
        let mut cursor = Cursor::new(8);
        cursor.extend_to(2);
        assert_eq!(cursor.selection(), 2..8);
        assert!(cursor.has_selection());
        cursor.collapse_to(3);
        assert_eq!(cursor.selection(), 3..3);
        assert!(!cursor.has_selection());
    }
}
