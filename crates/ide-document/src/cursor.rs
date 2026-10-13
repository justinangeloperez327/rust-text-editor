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

#[cfg(test)]
mod tests {
    use super::*;

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
