//! SQLite SessionStore adapter for the production assembly.

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_runtime::canonical::{RuntimeError, RuntimeResult, Session, SessionStore};
use apeireth_storage::{run_migrations, SqliteConnectionPool, StorageError};
use async_trait::async_trait;

/// Durable SQLite implementation of the kernel's SessionStore port.
pub struct SqliteSessionStore {
    pool: SqliteConnectionPool,
}

impl SqliteSessionStore {
    /// Open a file-backed store and apply storage migrations.
    pub async fn open(path: impl AsRef<std::path::Path>) -> RuntimeResult<Self> {
        let pool = SqliteConnectionPool::open(path.as_ref())
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        pool.write(|connection| run_migrations(connection))
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        Ok(Self { pool })
    }

    /// Open a shared in-memory SQLite store.
    pub async fn in_memory() -> RuntimeResult<Self> {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        pool.write(|connection| run_migrations(connection))
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        Ok(Self { pool })
    }

    fn storage_error(
        session: SessionId,
        operation: &'static str,
        error: StorageError,
    ) -> RuntimeError {
        match operation {
            "load" => RuntimeError::session_load(session, error.to_string()),
            _ => RuntimeError::session_save(session, error.to_string()),
        }
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>> {
        use rusqlite::OptionalExtension;

        let id = *id;
        self.pool
            .read(move |connection| {
                let data: Option<String> = connection
                    .prepare("SELECT data FROM sessions WHERE id = ?1")?
                    .query_row([id.to_string()], |row| row.get(0))
                    .optional()?;
                data.map(|json| {
                    serde_json::from_str::<Session>(&json)
                        .map_err(|error| StorageError::Serialization(error.to_string()))
                })
                .transpose()
            })
            .map_err(|error| Self::storage_error(id, "load", error))
    }

    async fn save(&self, session: &Session) -> RuntimeResult<()> {
        let id = session.id;
        let data = serde_json::to_string(session)
            .map_err(|error| RuntimeError::session_save(id, error.to_string()))?;
        self.pool
            .write(move |connection| {
                connection.execute(
                    "INSERT INTO sessions (id, data) VALUES (?1, ?2)
                     ON CONFLICT(id) DO UPDATE SET data = excluded.data",
                    rusqlite::params![id.to_string(), data],
                )?;
                Ok(())
            })
            .await
            .map_err(|error| Self::storage_error(id, "save", error))
    }

    async fn list(&self) -> RuntimeResult<Vec<Session>> {
        let mut sessions = self
            .pool
            .read(|connection| {
                let mut statement = connection.prepare("SELECT data FROM sessions")?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
                rows.map(|row| {
                    let json =
                        row.map_err(|error| StorageError::Serialization(error.to_string()))?;
                    serde_json::from_str::<Session>(&json)
                        .map_err(|error| StorageError::Serialization(error.to_string()))
                })
                .collect::<Result<Vec<_>, StorageError>>()
            })
            .map_err(|error| RuntimeError::session_store_open(format!("list: {error}")))?;
        sessions.sort_by(|left, right| {
            right
                .updated_at
                .epoch_millis()
                .cmp(&left.updated_at.epoch_millis())
        });
        Ok(sessions)
    }
}

/// Convenience conversion for assembly callers.
pub fn as_session_store(store: SqliteSessionStore) -> Arc<dyn SessionStore> {
    Arc::new(store)
}
