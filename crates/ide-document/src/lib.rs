//! UI-independent, UTF-8-safe document model built on a rope buffer.

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
}

#[derive(Debug, Clone)]
struct EditRecord {
    start: usize,
    removed: String,
    inserted: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    text: Rope,
    version: DocumentVersion,
    saved_version: DocumentVersion,
    undo: Vec<EditRecord>,
    redo: Vec<EditRecord>,
}

impl Document {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            version: DocumentVersion::default(),
            saved_version: DocumentVersion::default(),
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
        self.version != self.saved_version
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
        self.undo.push(EditRecord {
            start: range.start,
            removed,
            inserted: replacement.to_owned(),
        });
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
        let end = record.start + record.inserted.chars().count();
        self.text.remove(record.start..end);
        self.text.insert(record.start, &record.removed);
        self.version = self.version.next();
        self.redo.push(record);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(record) = self.redo.pop() else {
            return false;
        };
        let end = record.start + record.removed.chars().count();
        self.text.remove(record.start..end);
        self.text.insert(record.start, &record.inserted);
        self.version = self.version.next();
        self.undo.push(record);
        true
    }

    pub fn mark_saved(&mut self) {
        self.saved_version = self.version;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
