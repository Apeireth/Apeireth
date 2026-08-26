//! Storage-level errors for the canonical storage foundation.

use std::path::PathBuf;

/// Errors produced by the storage foundation.
///
/// The public storage contract never uses `Box<dyn Error>` or bare strings.
/// Callers can distinguish open, pool, database, migration, channel, and
/// configuration failures.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The database could not be opened or created.
    #[error("failed to open sqlite database `{path}`: {message}")]
    Open {
        /// Path (or logical name) of the database.
        path: PathBuf,
        /// Underlying open failure.
        message: String,
    },

    /// A SQLite operation failed.
    #[error("sqlite database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A connection could not be obtained from the read pool.
    #[error("sqlite connection pool error: {0}")]
    Pool(#[from] r2d2::Error),

    /// The single-writer channel is closed or a writer task vanished.
    #[error("storage write channel error: {0}")]
    WriteQueue(String),

    /// A schema migration failed.
    #[error("storage migration `{name}` failed at version {version}: {message}")]
    Migration {
        /// Migration version that was being applied.
        version: i64,
        /// Migration name.
        name: &'static str,
        /// Underlying failure.
        message: String,
    },

    /// Configuration failed validation.
    #[error("invalid storage configuration: {0}")]
    InvalidConfiguration(String),

    /// A serialized payload could not be encoded or decoded.
    #[error("storage serialization error: {0}")]
    Serialization(String),
}

impl StorageError {
    /// Constructs a migration error with the supplied migration metadata.
    pub(crate) fn migration(version: i64, name: &'static str, message: impl Into<String>) -> Self {
        Self::Migration {
            version,
            name,
            message: message.into(),
        }
    }
}
