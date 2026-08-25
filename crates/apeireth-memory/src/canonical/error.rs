//! Canonical memory errors (M1B1).
//!
//! The high-level memory API deliberately does not expose `rusqlite::Error` or
//! `r2d2::Error` directly. Storage-level failures are wrapped in
//! [`MemoryError::Persistence`] via [`apeireth_storage::StorageError`].

use apeireth_storage::StorageError;

/// Errors produced by the canonical memory domain and repository.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// The requested memory item does not exist.
    #[error("memory item not found: {0}")]
    NotFound(String),

    /// Caller-supplied domain data failed validation.
    #[error("invalid memory data: {0}")]
    InvalidData(String),

    /// A write would violate a uniqueness/identity constraint.
    #[error("memory item conflict: {0}")]
    Conflict(String),

    /// The storage foundation failed.
    #[error("memory persistence error: {0}")]
    Persistence(#[from] StorageError),
}

/// Canonical memory result alias.
pub type Result<T> = std::result::Result<T, MemoryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_errors_wrap_as_persistence() {
        let storage = StorageError::InvalidConfiguration("max_connections = 0".into());
        let memory: MemoryError = storage.into();
        assert!(matches!(memory, MemoryError::Persistence(_)));
    }
}
