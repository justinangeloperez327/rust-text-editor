//! UI-independent, UTF-8-safe document model built on a rope buffer.

pub mod cursor;

use cursor::{Cursor, CursorSet, TextPosition};
use ropey::Rope;
use std::{fs, io, ops::Range, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocumentVersion(pub u64);

impl DocumentVersion {
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidRange,
    InvalidCharacterOffset,
    OverlappingSelections,
}

#[derive(Debug, Clone)]
struct TextChange {
    start: usize,
    removed: String,
    inserted: String,
}

#[derive(Debug, Clone)]
struct EditRecord {
    changes: Vec<TextChange>,
    before_state: u64,
    after_state: u64,
}

#[derive(Debug, Clone)]
pub struct Document {
    text: Rope,
    version: DocumentVersion,
    saved_state: u64,
    current_state: u64,
    next_state: u64,
    undo: Vec<EditRecord>,
    redo: Vec<EditRecord>,
}

impl Document {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            version: DocumentVersion::default(),
            saved_state: 0,
            current_state: 0,
            next_state: 1,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    #[must_use]
    pub fn version(&self) -> DocumentVersion {
        self.version
    }

    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.current_state != self.saved_state
    }

    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.text.len_lines()
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    pub fn line(&self, index: usize) -> Option<String> {
        if index >= self.text.len_lines() {
            return None;
        }
        Some(self.text.line(index).to_string())
    }

    /// Convert a Unicode character offset into a zero-based line and column.
    pub fn position_at(&self, offset: usize) -> Result<TextPosition, EditError> {
        if offset > self.len_chars() {
            return Err(EditError::InvalidCharacterOffset);
        }
        let line = self.text.char_to_line(offset);
        Ok(TextPosition {
            line,
            column: offset - self.text.line_to_char(line),
        })
    }

    /// Convert a zero-based line and character column into an offset.
    /// Columns count Unicode scalar values, including line-ending characters.
    pub fn offset_at(&self, position: TextPosition) -> Result<usize, EditError> {
        if position.line >= self.len_lines() {
            return Err(EditError::InvalidCharacterOffset);
        }
        let start = self.text.line_to_char(position.line);
        let line_length = self.text.line(position.line).len_chars();
        if position.column > line_length {
            return Err(EditError::InvalidCharacterOffset);
        }
        Ok(start + position.column)
    }

    /// Replace the given range of Unicode scalar-value offsets.
    pub fn replace(&mut self, range: Range<usize>, replacement: &str) -> Result<(), EditError> {
        if range.start > range.end || range.end > self.text.len_chars() {
            return Err(EditError::InvalidRange);
        }
        let removed = self.text.slice(range.clone()).to_string();
        if removed == replacement {
            return Ok(());
        }
        self.text.remove(range.clone());
        self.text.insert(range.start, replacement);
        let after_state = self.next_state;
        self.next_state = self.next_state.saturating_add(1);
        self.undo.push(EditRecord {
            changes: vec![TextChange {
                start: range.start,
                removed,
                inserted: replacement.to_owned(),
            }],
            before_state: self.current_state,
            after_state,
        });
        self.current_state = after_state;
        self.redo.clear();
        self.version = self.version.next();
        Ok(())
    }

    pub fn insert(&mut self, at: usize, text: &str) -> Result<(), EditError> {
        if at > self.len_chars() {
            return Err(EditError::InvalidCharacterOffset);
        }
        self.replace(at..at, text)
    }

    pub fn delete(&mut self, range: Range<usize>) -> Result<(), EditError> {
        self.replace(range, "")
    }

    /// Apply non-overlapping selections as one undoable transaction.
    /// All ranges refer to the document state before the edit.
    pub fn replace_cursors(
        &mut self,
        cursors: &CursorSet,
        replacement: &str,
    ) -> Result<(), EditError> {
        let ranges = cursors
            .non_overlapping_ranges()
            .ok_or(EditError::OverlappingSelections)?;
        if ranges.iter().any(|range| range.end > self.len_chars()) {
            return Err(EditError::InvalidRange);
        }
        let mut changes = Vec::new();
        for range in ranges.into_iter().rev() {
            let removed = self.text.slice(range.clone()).to_string();
            if removed == replacement {
                continue;
            }
            self.text.remove(range.clone());
            self.text.insert(range.start, replacement);
            changes.push(TextChange {
                start: range.start,
                removed,
                inserted: replacement.to_owned(),
            });
        }
        if changes.is_empty() {
            return Ok(());
        }
        let after_state = self.next_state;
        self.next_state = self.next_state.saturating_add(1);
        self.undo.push(EditRecord {
            changes,
            before_state: self.current_state,
            after_state,
        });
        self.current_state = after_state;
        self.redo.clear();
        self.version = self.version.next();
        Ok(())
    }

    /// Replace the cursor's selection, or insert at its head.
    /// On success the cursor collapses after the inserted text.
    pub fn insert_at_cursor(&mut self, cursor: &mut Cursor, text: &str) -> Result<(), EditError> {
        let selection = cursor.selection();
        if selection.end > self.len_chars() {
            return Err(EditError::InvalidCharacterOffset);
        }
        let next = selection.start + text.chars().count();
        self.replace(selection, text)?;
        cursor.collapse_to(next);
        Ok(())
    }

    /// Move by one Unicode scalar value; optionally preserve the selection anchor.
    pub fn move_cursor_horizontal(&self, cursor: &mut Cursor, right: bool, extend_selection: bool) {
        let next = if !extend_selection && cursor.has_selection() {
            let range = cursor.selection();
            if right { range.end } else { range.start }
        } else if right {
            cursor.head.saturating_add(1).min(self.len_chars())
        } else {
            cursor.head.saturating_sub(1)
        };
        if extend_selection {
            cursor.extend_to(next);
        } else {
            cursor.collapse_to(next);
        }
    }

    fn line_content_length(&self, line_index: usize) -> usize {
        let line = self.text.line(line_index);
        let len = line.len_chars();
        if len == 0 {
            return 0;
        }
        if line.char(len - 1) == '\n' {
            if len > 1 && line.char(len - 2) == '\r' {
                return len - 2;
            }
            return len - 1;
        }
        if line.char(len - 1) == '\r' {
            return len - 1;
        }
        len
    }

    /// Move to the start or end of the current line, excluding line endings.
    pub fn move_cursor_line_boundary(
        &self,
        cursor: &mut Cursor,
        to_end: bool,
        extend_selection: bool,
    ) -> Result<(), EditError> {
        let position = self.position_at(cursor.head)?;
        let start = self.text.line_to_char(position.line);
        let content_length = self.line_content_length(position.line);
        let target = if to_end {
            start + content_length
        } else {
            start
        };
        if extend_selection {
            cursor.extend_to(target);
        } else {
            cursor.collapse_to(target);
        }
        Ok(())
    }

    /// Move one logical line while preserving the current character column.
    pub fn move_cursor_vertical(
        &self,
        cursor: &mut Cursor,
        down: bool,
        extend_selection: bool,
    ) -> Result<(), EditError> {
        let position = self.position_at(cursor.head)?;
        let target_line = if down {
            (position.line + 1).min(self.len_lines() - 1)
        } else {
            position.line.saturating_sub(1)
        };
        let start = self.text.line_to_char(target_line);
        let content_length = self.line_content_length(target_line);
        let target = start + position.column.min(content_length);
        if extend_selection {
            cursor.extend_to(target);
        } else {
            cursor.collapse_to(target);
        }
        Ok(())
    }

    /// Load UTF-8 text. Invalid UTF-8 and I/O failures are returned to the caller.
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self::new(&fs::read_to_string(path)?))
    }

    /// Save UTF-8 text and update the clean checkpoint only after successful I/O.
    pub fn save(&mut self, path: &Path) -> io::Result<()> {
        fs::write(path, self.text())?;
        self.mark_saved();
        Ok(())
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(record) = self.undo.pop() else {
            return false;
        };
        for change in record.changes.iter().rev() {
            let end = change.start + change.inserted.chars().count();
            self.text.remove(change.start..end);
            self.text.insert(change.start, &change.removed);
        }
        self.current_state = record.before_state;
        self.version = self.version.next();
        self.redo.push(record);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(record) = self.redo.pop() else {
            return false;
        };
        for change in &record.changes {
            let end = change.start + change.removed.chars().count();
            self.text.remove(change.start..end);
            self.text.insert(change.start, &change.inserted);
        }
        self.current_state = record.after_state;
        self.version = self.version.next();
        self.undo.push(record);
        true
    }

    pub fn mark_saved(&mut self) {
        self.saved_state = self.current_state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_and_home_end_navigation() {
        let doc = Document::new("abc\r\nx\nlong");
        let mut cursor = Cursor::new(2);
        doc.move_cursor_vertical(&mut cursor, true, false).unwrap();
        assert_eq!(cursor, Cursor::new(6));
        doc.move_cursor_line_boundary(&mut cursor, true, false)
            .unwrap();
        assert_eq!(cursor, Cursor::new(6));
        doc.move_cursor_vertical(&mut cursor, true, true).unwrap();
        assert_eq!(cursor.selection(), 6..8);
        doc.move_cursor_line_boundary(&mut cursor, false, false)
            .unwrap();
        assert_eq!(cursor, Cursor::new(7));
    }

    #[test]
    fn multi_cursor_replacement_is_single_undo_transaction() {
        let mut doc = Document::new("abc def ghi");
        let mut cursors = CursorSet::new(Cursor { anchor: 0, head: 3 });
        cursors.add(Cursor {
            anchor: 8,
            head: 11,
        });
        doc.replace_cursors(&cursors, "🚀").unwrap();
        assert_eq!(doc.text(), "🚀 def 🚀");
        assert_eq!(doc.version(), DocumentVersion(1));
        assert!(doc.undo());
        assert_eq!(doc.text(), "abc def ghi");
        assert!(doc.redo());
        assert_eq!(doc.text(), "🚀 def 🚀");
    }

    #[test]
    fn overlapping_selections_do_not_change_text() {
        let mut doc = Document::new("abcdef");
        let mut cursors = CursorSet::new(Cursor { anchor: 1, head: 4 });
        cursors.add(Cursor { anchor: 3, head: 5 });
        assert_eq!(
            doc.replace_cursors(&cursors, "x"),
            Err(EditError::OverlappingSelections)
        );
        assert_eq!(doc.text(), "abcdef");
        assert!(!doc.can_undo());
    }

    #[test]
    fn selection_replacement_collapses_cursor() {
        let mut doc = Document::new("a🚀b");
        let mut cursor = Cursor::new(3);
        cursor.extend_to(1);
        doc.insert_at_cursor(&mut cursor, "é").unwrap();
        assert_eq!(doc.text(), "aé");
        assert_eq!(cursor, Cursor::new(2));
        assert!(doc.undo());
        assert_eq!(doc.text(), "a🚀b");
    }

    #[test]
    fn cursor_movement_handles_selection_and_boundaries() {
        let doc = Document::new("é🚀");
        let mut cursor = Cursor::new(1);
        doc.move_cursor_horizontal(&mut cursor, true, true);
        assert_eq!(cursor.selection(), 1..2);
        doc.move_cursor_horizontal(&mut cursor, false, false);
        assert_eq!(cursor, Cursor::new(1));
        doc.move_cursor_horizontal(&mut cursor, false, false);
        doc.move_cursor_horizontal(&mut cursor, false, false);
        assert_eq!(cursor, Cursor::new(0));
    }

    #[test]
    fn positions_round_trip_across_unicode_and_newlines() {
        let doc = Document::new("a🚀\r\nbé");
        for offset in 0..=doc.len_chars() {
            let pos = doc.position_at(offset).unwrap();
            assert_eq!(doc.offset_at(pos).unwrap(), offset);
        }
        assert_eq!(
            doc.position_at(4).unwrap(),
            TextPosition { line: 1, column: 0 }
        );
        assert_eq!(
            doc.offset_at(TextPosition {
                line: 10,
                column: 0
            }),
            Err(EditError::InvalidCharacterOffset)
        );
    }

    #[test]
    fn undo_redo_unicode_edits() {
        let mut doc = Document::new("abc");
        doc.replace(1..2, "🚀").unwrap();
        assert_eq!(doc.text(), "a🚀c");
        assert!(doc.undo());
        assert_eq!(doc.text(), "abc");
        assert!(doc.redo());
        assert_eq!(doc.text(), "a🚀c");
        assert!(doc.can_undo());
    }

    #[test]
    fn undo_to_saved_checkpoint_clears_dirty_flag() {
        let mut doc = Document::new("abc");
        doc.insert(3, "!").unwrap();
        doc.mark_saved();
        doc.insert(4, "?").unwrap();
        assert!(doc.is_modified());
        assert!(doc.undo());
        assert!(!doc.is_modified());
        assert!(doc.redo());
        assert!(doc.is_modified());
        assert!(doc.undo());
        doc.insert(4, "~").unwrap();
        assert!(doc.is_modified());
    }

    #[test]
    fn new_edit_clears_redo_history() {
        let mut doc = Document::new("text");
        doc.insert(4, "!").unwrap();
        assert!(doc.undo());
        doc.insert(4, "?").unwrap();
        assert!(!doc.can_redo());
        assert!(!doc.redo());
    }

    #[test]
    fn no_op_edit_preserves_version() {
        let mut doc = Document::new("abc");
        doc.replace(0..3, "abc").unwrap();
        assert_eq!(doc.version(), DocumentVersion(0));
        assert!(!doc.can_undo());
    }

    #[test]
    fn version_advances() {
        assert_eq!(DocumentVersion::default().next(), DocumentVersion(1));
    }

    #[test]
    fn unicode_edits_use_character_offsets() {
        let mut doc = Document::new("a🚀b\nsecond");
        assert_eq!(doc.len_chars(), 10);
        assert_eq!(doc.len_lines(), 2);
        doc.replace(1..2, "é").unwrap();
        assert_eq!(doc.text(), "aéb\nsecond");
        assert_eq!(doc.version(), DocumentVersion(1));
        assert!(doc.is_modified());
        doc.mark_saved();
        assert!(!doc.is_modified());
    }

    #[test]
    fn invalid_ranges_do_not_mutate_document() {
        let mut doc = Document::new("hello");
        assert_eq!(doc.delete(4..8), Err(EditError::InvalidRange));
        assert_eq!(doc.insert(100, "x"), Err(EditError::InvalidCharacterOffset));
        assert_eq!(doc.text(), "hello");
        assert_eq!(doc.version(), DocumentVersion(0));
    }

    #[test]
    fn lines_are_accessible_without_flattening_buffer() {
        let doc = Document::new("alpha\nbeta");
        assert_eq!(doc.line(0).as_deref(), Some("alpha\n"));
        assert_eq!(doc.line(1).as_deref(), Some("beta"));
        assert_eq!(doc.line(2), None);
    }
}
