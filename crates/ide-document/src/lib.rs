//! UI-independent document model primitives.
//! The rope buffer and versioned snapshots are introduced in Group 4.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocumentVersion(pub u64);

impl DocumentVersion {
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentVersion;

    #[test]
    fn version_advances() {
        assert_eq!(DocumentVersion::default().next(), DocumentVersion(1));
    }
}
