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

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::system_clock;
    use apeireth_protocol::canonical::{ContentPart, NormalizedMessage};
    use apeireth_runtime::canonical::{PermissionPreset, Session, SessionSettings};
    use apeireth_storage::SqliteConnectionPool;

    #[tokio::test]
    async fn legacy_session_json_without_settings_opens_with_defaults_and_no_data_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite3");

        // Simulate a pre-settings database: create only the `sessions` table
        // (as migration v1 did) and store a session JSON with no `settings`
        // key — the on-disk shape this feature must migrate from.
        let pool = SqliteConnectionPool::open(&path).await.unwrap();
        let clock = system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("hello"), clock.as_ref());
        let session_id = session.id;
        let mut legacy = serde_json::to_value(&session).unwrap();
        legacy
            .as_object_mut()
            .expect("session serializes as object")
            .remove("settings");
        let data = serde_json::to_string(&legacy).unwrap();

        pool.write(move |conn| {
            conn.execute_batch("CREATE TABLE sessions (id TEXT PRIMARY KEY, data TEXT);")?;
            conn.execute(
                "INSERT INTO sessions (id, data) VALUES (?1, ?2)",
                rusqlite::params![session_id.to_string(), data],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        drop(pool);

        // Reopen through the production store. `run_migrations` is idempotent
        // against the existing table and must not drop or rewrite the row.
        let store = SqliteSessionStore::open(&path).await.unwrap();
        let loaded = store.load(&session_id).await.unwrap().unwrap();

        assert_eq!(loaded.settings, SessionSettings::default());
        assert_eq!(loaded.settings.model, None);
        assert_eq!(
            loaded.settings.permission_preset,
            PermissionPreset::Standard
        );
        assert_eq!(loaded.messages.len(), 1, "transcript must survive migration");
        assert_eq!(ContentPart::join_text(&loaded.messages[0].content), "hello");
    }
}
