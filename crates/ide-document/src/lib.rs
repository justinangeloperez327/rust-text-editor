//! UI-independent, UTF-8-safe document model built on a rope buffer.

use ropey::Rope;
use std::ops::Range;

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
pub struct Document {
    text: Rope,
    version: DocumentVersion,
    saved_version: DocumentVersion,
}

impl Document {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            version: DocumentVersion::default(),
            saved_version: DocumentVersion::default(),
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
        self.text.remove(range.clone());
        self.text.insert(range.start, replacement);
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

    pub fn mark_saved(&mut self) {
        self.saved_version = self.version;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
