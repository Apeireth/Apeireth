//! SQLite-backed persistence for persona profiles.
//!
//! The profile tables are also described by the memory migrations.  The local
//! [`SqlitePersonaProfileStore::ensure_schema`] is intentionally idempotent so
//! this store can be used before those migrations are wired into the storage
//! bootstrap.  Once bootstrap owns the schema, calling it remains harmless.

use std::sync::Arc;

use apeireth_core::kernel::Timestamp;
use apeireth_storage::{SqliteConnectionPool, StorageError};
use rusqlite::{types::Type, OptionalExtension, Row};
use serde::de::DeserializeOwned;

use crate::scope::{
    MemoryProvenance, PersonaMemoryProfile, PersonaProfileDelta, PersonaProfileStore,
};

/// Durable persona profile store using the storage crate's serialized writer.
///
/// A profile is keyed by `(persona_id, subject_id)`.  Every successful delta
/// creates a new immutable row in `persona_profile_history`; the current row
/// is updated only when its revision still equals the caller's expected
/// revision.  The read and history paths use pooled reader connections.
#[derive(Clone)]
pub struct SqlitePersonaProfileStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SqlitePersonaProfileStore {
    /// Creates a store from an owned connection pool.
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Creates a store sharing an existing connection pool.
    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    /// Returns the underlying pool for diagnostics and compatibility code.
    pub fn pool(&self) -> &SqliteConnectionPool {
        self.pool.as_ref()
    }

    /// Creates the profile tables when the central memory migration has not
    /// run yet.  This is deliberately kept local and idempotent; it does not
    /// replace the migration-owned schema.
    pub async fn ensure_schema(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.pool
            .write(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS persona_profiles (
                        persona_id TEXT NOT NULL,
                        subject_id TEXT NOT NULL,
                        portrait TEXT NOT NULL DEFAULT '',
                        traits_json TEXT NOT NULL DEFAULT '[]',
                        known_facts_json TEXT NOT NULL DEFAULT '[]',
                        shared_experiences_json TEXT NOT NULL DEFAULT '[]',
                        revision INTEGER NOT NULL DEFAULT 0,
                        provenance_json TEXT NOT NULL DEFAULT '{}',
                        created_at_ms INTEGER NOT NULL,
                        updated_at_ms INTEGER NOT NULL,
                        PRIMARY KEY (persona_id, subject_id)
                    );
                    CREATE INDEX IF NOT EXISTS idx_persona_profiles_subject
                        ON persona_profiles(subject_id, updated_at_ms DESC);

                    CREATE TABLE IF NOT EXISTS persona_profile_history (
                        id TEXT PRIMARY KEY,
                        persona_id TEXT NOT NULL,
                        subject_id TEXT NOT NULL,
                        revision INTEGER NOT NULL,
                        portrait TEXT NOT NULL DEFAULT '',
                        traits_json TEXT NOT NULL DEFAULT '[]',
                        known_facts_json TEXT NOT NULL DEFAULT '[]',
                        shared_experiences_json TEXT NOT NULL DEFAULT '[]',
                        provenance_json TEXT NOT NULL DEFAULT '{}',
                        changed_at_ms INTEGER NOT NULL,
                        change_reason TEXT
                    );
                    CREATE UNIQUE INDEX IF NOT EXISTS uq_persona_profile_history_revision
                        ON persona_profile_history(persona_id, subject_id, revision);
                    CREATE INDEX IF NOT EXISTS idx_persona_profile_history_subject
                        ON persona_profile_history(subject_id, changed_at_ms DESC);

                    CREATE TRIGGER IF NOT EXISTS persona_profile_history_no_delete
                    BEFORE DELETE ON persona_profile_history BEGIN
                        SELECT RAISE(ABORT, 'persona_profile_history: hard DELETE forbidden');
                    END;
                    CREATE TRIGGER IF NOT EXISTS persona_profile_history_no_update
                    BEFORE UPDATE ON persona_profile_history BEGIN
                        SELECT RAISE(ABORT, 'persona_profile_history: UPDATE forbidden');
                    END;
                    "#,
                )
                .map_err(StorageError::from)
            })
            .await
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Returns all immutable snapshots in revision order.
    pub async fn history(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Vec<PersonaMemoryProfile>, String> {
        self.pool
            .read(|conn| {
                let mut statement = conn.prepare_cached(
                    "SELECT persona_id, subject_id, revision, portrait, traits_json, \
                            known_facts_json, shared_experiences_json, provenance_json, changed_at_ms \
                       FROM persona_profile_history \
                      WHERE persona_id = ?1 AND subject_id = ?2 \
                      ORDER BY revision ASC",
                )?;
                let rows = statement.query_map(rusqlite::params![persona_id, subject_id], |row| {
                    profile_from_history_row(row)
                })?;
                Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .map_err(|error| error.to_string())
    }

    /// Alias for callers that prefer an explicit getter name.
    pub async fn get_history(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Vec<PersonaMemoryProfile>, String> {
        self.history(persona_id, subject_id).await
    }
}

#[async_trait::async_trait]
impl PersonaProfileStore for SqlitePersonaProfileStore {
    async fn get_profile(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Option<PersonaMemoryProfile>, String> {
        self.pool
            .read(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT persona_id, subject_id, portrait, traits_json, known_facts_json, \
                            shared_experiences_json, revision, provenance_json, updated_at_ms \
                       FROM persona_profiles \
                      WHERE persona_id = ?1 AND subject_id = ?2",
                        rusqlite::params![persona_id, subject_id],
                        profile_from_current_row,
                    )
                    .optional()?)
            })
            .map_err(|error| error.to_string())
    }

    async fn apply_delta(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String> {
        let persona_id = persona_id.to_owned();
        let subject_id = subject_id.to_owned();
        let changed_at_ms = updated_at.epoch_millis();

        self.pool
            .write(move |conn| {
                let transaction = conn.transaction()?;
                let current = transaction
                    .query_row(
                        "SELECT persona_id, subject_id, portrait, traits_json, known_facts_json, \
                                shared_experiences_json, revision, provenance_json, updated_at_ms \
                           FROM persona_profiles \
                          WHERE persona_id = ?1 AND subject_id = ?2",
                        rusqlite::params![&persona_id, &subject_id],
                        profile_from_current_row,
                    )
                    .optional()?;

                let has_current = current.is_some();
                let mut profile = current.unwrap_or_else(|| PersonaMemoryProfile {
                    persona_id: persona_id.clone(),
                    subject_id: subject_id.clone(),
                    portrait: String::new(),
                    traits: Vec::new(),
                    known_facts: Vec::new(),
                    shared_experiences: Vec::new(),
                    revision: 0,
                    provenance: MemoryProvenance::default(),
                    updated_at,
                });
                let current_revision = profile.revision;
                profile
                    .apply_delta(&delta, expected_revision, updated_at)
                    .map_err(StorageError::InvalidConfiguration)?;
                if profile.revision <= current_revision {
                    return Err(StorageError::InvalidConfiguration(
                        "persona profile revision overflow".to_string(),
                    ));
                }
                let revision = i64::try_from(profile.revision).map_err(|_| {
                    StorageError::InvalidConfiguration(
                        "persona profile revision exceeds sqlite integer range".to_string(),
                    )
                })?;
                let expected_revision_i64 = i64::try_from(expected_revision).map_err(|_| {
                    StorageError::InvalidConfiguration(
                        "expected persona profile revision exceeds sqlite integer range".to_string(),
                    )
                })?;
                let traits_json = serde_json::to_string(&profile.traits)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let known_facts_json = serde_json::to_string(&profile.known_facts)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let shared_experiences_json = serde_json::to_string(&profile.shared_experiences)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let provenance_json = serde_json::to_string(&profile.provenance)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;

                let changed = if has_current {
                    transaction.execute(
                        "UPDATE persona_profiles SET portrait = ?1, traits_json = ?2, \
                                known_facts_json = ?3, shared_experiences_json = ?4, \
                                revision = ?5, provenance_json = ?6, updated_at_ms = ?7 \
                           WHERE persona_id = ?8 AND subject_id = ?9 AND revision = ?10",
                        rusqlite::params![
                            &profile.portrait,
                            traits_json,
                            known_facts_json,
                            shared_experiences_json,
                            revision,
                            provenance_json,
                            changed_at_ms,
                            &persona_id,
                            &subject_id,
                            expected_revision_i64,
                        ],
                    )?
                } else {
                    transaction.execute(
                        "INSERT INTO persona_profiles \
                            (persona_id, subject_id, portrait, traits_json, known_facts_json, \
                             shared_experiences_json, revision, provenance_json, created_at_ms, updated_at_ms) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                        rusqlite::params![
                            &persona_id,
                            &subject_id,
                            &profile.portrait,
                            traits_json,
                            known_facts_json,
                            shared_experiences_json,
                            revision,
                            provenance_json,
                            changed_at_ms,
                        ],
                    )?
                };

                if changed != 1 {
                    return Err(StorageError::InvalidConfiguration(format!(
                        "persona profile revision conflict: expected {expected_revision}, current {current_revision}"
                    )));
                }

                let snapshot_id = history_id(&persona_id, &subject_id, revision);
                transaction.execute(
                    "INSERT INTO persona_profile_history \
                        (id, persona_id, subject_id, revision, portrait, traits_json, \
                         known_facts_json, shared_experiences_json, provenance_json, changed_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        snapshot_id,
                        &persona_id,
                        &subject_id,
                        revision,
                        &profile.portrait,
                        traits_json,
                        known_facts_json,
                        shared_experiences_json,
                        provenance_json,
                        changed_at_ms,
                    ],
                )?;
                transaction.commit()?;
                Ok(profile)
            })
            .await
            .map_err(|error| error.to_string())
    }
}

fn profile_from_current_row(row: &Row<'_>) -> rusqlite::Result<PersonaMemoryProfile> {
    let persona_id: String = row.get(0)?;
    let subject_id: String = row.get(1)?;
    let portrait: String = row.get(2)?;
    let traits_json: String = row.get(3)?;
    let known_facts_json: String = row.get(4)?;
    let shared_experiences_json: String = row.get(5)?;
    let revision = non_negative_revision(row.get::<_, i64>(6)?, 6)?;
    let provenance_json: String = row.get(7)?;
    let updated_at_ms: i64 = row.get(8)?;

    Ok(PersonaMemoryProfile {
        persona_id,
        subject_id,
        portrait,
        traits: decode_json(&traits_json, 3)?,
        known_facts: decode_json(&known_facts_json, 4)?,
        shared_experiences: decode_json(&shared_experiences_json, 5)?,
        revision,
        provenance: decode_provenance(&provenance_json, 7)?,
        updated_at: timestamp_from_millis(updated_at_ms, 8)?,
    })
}

fn profile_from_history_row(row: &Row<'_>) -> rusqlite::Result<PersonaMemoryProfile> {
    let persona_id: String = row.get(0)?;
    let subject_id: String = row.get(1)?;
    let revision = non_negative_revision(row.get::<_, i64>(2)?, 2)?;
    let portrait: String = row.get(3)?;
    let traits_json: String = row.get(4)?;
    let known_facts_json: String = row.get(5)?;
    let shared_experiences_json: String = row.get(6)?;
    let provenance_json: String = row.get(7)?;
    let changed_at_ms: i64 = row.get(8)?;

    Ok(PersonaMemoryProfile {
        persona_id,
        subject_id,
        portrait,
        traits: decode_json(&traits_json, 4)?,
        known_facts: decode_json(&known_facts_json, 5)?,
        shared_experiences: decode_json(&shared_experiences_json, 6)?,
        revision,
        provenance: decode_provenance(&provenance_json, 7)?,
        updated_at: timestamp_from_millis(changed_at_ms, 8)?,
    })
}

fn decode_json<T: DeserializeOwned>(raw: &str, column: usize) -> rusqlite::Result<T> {
    serde_json::from_str(raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

fn decode_provenance(raw: &str, column: usize) -> rusqlite::Result<MemoryProvenance> {
    // V11's column default is `{}`. Treat that legacy/default value as the
    // domain default, but reject every other malformed payload explicitly.
    if raw.trim() == "{}" {
        return Ok(MemoryProvenance::default());
    }
    serde_json::from_str(raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

fn non_negative_revision(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, Box::new(error))
    })
}

fn timestamp_from_millis(value: i64, column: usize) -> rusqlite::Result<Timestamp> {
    Timestamp::from_epoch_millis(value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            Type::Integer,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid timestamp milliseconds: {value}"),
            )),
        )
    })
}

fn history_id(persona_id: &str, subject_id: &str, revision: i64) -> String {
    // Length-prefixing makes the identifier unambiguous even when either key
    // contains punctuation used by a human-readable delimiter.
    format!(
        "persona-profile:{:08x}:{}:{:08x}:{}:{revision}",
        persona_id.len(),
        persona_id,
        subject_id.len(),
        subject_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::PersonaProfileStore;

    async fn fresh() -> SqlitePersonaProfileStore {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool");
        let store = SqlitePersonaProfileStore::new(pool);
        store.ensure_schema().await.expect("ensure_schema");
        store
    }

    fn timestamp(ms: i64) -> Timestamp {
        Timestamp::from_epoch_millis(ms).expect("valid timestamp")
    }

    fn delta(provenance_source: &str) -> PersonaProfileDelta {
        PersonaProfileDelta {
            portrait_replace: Some("a calm guide".to_string()),
            traits_add: vec!["patient".to_string(), "precise".to_string()],
            facts_add: vec!["uses Rust".to_string()],
            experiences_add: vec!["built a store".to_string()],
            provenance: MemoryProvenance {
                source: provenance_source.to_string(),
                source_session: Some("session-1".to_string()),
                source_trace: Some("trace-1".to_string()),
                source_request: None,
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn profile_and_history_round_trip() {
        let store = fresh().await;
        let updated_at = timestamp(1_700_000_000_000);
        let expected = store
            .apply_delta("persona-a", "subject-a", 0, delta("test"), updated_at)
            .await
            .expect("apply delta");

        assert_eq!(expected.revision, 1);
        assert_eq!(expected.updated_at, updated_at);
        assert_eq!(
            store
                .get_profile("persona-a", "subject-a")
                .await
                .expect("get profile"),
            Some(expected.clone())
        );

        let history = store
            .history("persona-a", "subject-a")
            .await
            .expect("history");
        assert_eq!(history, vec![expected]);
    }

    #[tokio::test]
    async fn revision_conflict_does_not_write_current_or_history() {
        let store = fresh().await;
        let first = store
            .apply_delta(
                "persona-a",
                "subject-a",
                0,
                delta("first"),
                timestamp(1_700_000_000_000),
            )
            .await
            .expect("first delta");

        let conflict = store
            .apply_delta(
                "persona-a",
                "subject-a",
                0,
                delta("stale"),
                timestamp(1_700_000_000_001),
            )
            .await
            .expect_err("stale revision must fail");
        assert!(conflict.contains("persona profile revision conflict"));
        assert_eq!(
            store
                .get_profile("persona-a", "subject-a")
                .await
                .expect("get profile"),
            Some(first)
        );
        assert_eq!(
            store
                .history("persona-a", "subject-a")
                .await
                .expect("history")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn profile_delta_matches_in_memory_dedup_and_updates() {
        let store = fresh().await;
        let first = store
            .apply_delta(
                "persona-a",
                "subject-a",
                0,
                delta("first"),
                timestamp(1_700_000_000_000),
            )
            .await
            .expect("first delta");
        let second_delta = PersonaProfileDelta {
            portrait_replace: None,
            traits_add: vec!["patient".into(), "kind".into()],
            traits_remove: vec!["precise".into()],
            facts_update: vec![("uses Rust".into(), "uses stable Rust".into())],
            experiences_add: vec!["built a store".into(), "reviewed history".into()],
            provenance: MemoryProvenance {
                source: "second".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let second = store
            .apply_delta(
                "persona-a",
                "subject-a",
                first.revision,
                second_delta,
                timestamp(1_700_000_000_100),
            )
            .await
            .expect("second delta");

        assert_eq!(second.revision, 2);
        assert_eq!(second.traits, vec!["patient", "kind"]);
        assert_eq!(second.known_facts, vec!["uses stable Rust"]);
        assert_eq!(
            second.shared_experiences,
            vec!["built a store", "reviewed history"]
        );
        assert_eq!(
            store.history("persona-a", "subject-a").await.unwrap().len(),
            2
        );
    }

    #[tokio::test]
    async fn malformed_history_json_is_reported() {
        let store = fresh().await;
        store
            .pool()
            .write(|conn| {
                conn.execute(
                    "INSERT INTO persona_profile_history \
                        (id, persona_id, subject_id, revision, portrait, traits_json, \
                         known_facts_json, shared_experiences_json, provenance_json, changed_at_ms) \
                     VALUES ('bad-history', 'bad-persona', 'bad-subject', 1, '', '[]', '[]', '[]', '{', 1)",
                    [],
                )?;
                Ok(())
            })
            .await
            .expect("insert malformed history row");

        assert!(store.history("bad-persona", "bad-subject").await.is_err());
    }

    #[tokio::test]
    async fn history_rows_are_immutable() {
        let store = fresh().await;
        store
            .apply_delta(
                "persona-a",
                "subject-a",
                0,
                delta("first"),
                timestamp(1_700_000_000_000),
            )
            .await
            .expect("apply delta");

        let update = store
            .pool()
            .write(|conn| {
                conn.execute(
                    "UPDATE persona_profile_history SET portrait = 'tampered' \
                     WHERE persona_id = 'persona-a' AND subject_id = 'subject-a' AND revision = 1",
                    [],
                )?;
                Ok(())
            })
            .await;
        assert!(update.is_err(), "history snapshots must reject updates");
        assert_eq!(
            store.history("persona-a", "subject-a").await.unwrap()[0].portrait,
            "a calm guide"
        );
    }

    #[tokio::test]
    async fn concurrent_same_revision_has_one_winner() {
        let store = fresh().await;
        let left = store.clone();
        let right = store.clone();
        let timestamp = timestamp(1_700_000_000_000);
        let (left, right) = tokio::join!(
            left.apply_delta("persona-a", "subject-a", 0, delta("left"), timestamp),
            right.apply_delta("persona-a", "subject-a", 0, delta("right"), timestamp),
        );
        assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
        assert_eq!(
            store.history("persona-a", "subject-a").await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn profile_and_history_survive_reopening_file_pool() {
        let path = tempfile::NamedTempFile::new().expect("temp db");
        let pool = SqliteConnectionPool::open(path.path())
            .await
            .expect("open pool");
        let store = SqlitePersonaProfileStore::new(pool);
        store.ensure_schema().await.expect("ensure schema");
        let expected = store
            .apply_delta(
                "persona-a",
                "subject-a",
                0,
                delta("restart"),
                timestamp(1_700_000_000_000),
            )
            .await
            .expect("apply delta");
        drop(store);

        let reopened_pool = SqliteConnectionPool::open(path.path())
            .await
            .expect("reopen pool");
        let reopened = SqlitePersonaProfileStore::new(reopened_pool);
        assert_eq!(
            reopened
                .get_profile("persona-a", "subject-a")
                .await
                .unwrap(),
            Some(expected.clone())
        );
        assert_eq!(
            reopened.history("persona-a", "subject-a").await.unwrap(),
            vec![expected]
        );
    }

    #[tokio::test]
    async fn malformed_current_row_is_reported() {
        let store = fresh().await;
        store
            .pool()
            .write(|conn| {
                conn.execute(
                    "INSERT INTO persona_profiles \
                        (persona_id, subject_id, portrait, traits_json, known_facts_json, \
                         shared_experiences_json, revision, provenance_json, created_at_ms, updated_at_ms) \
                     VALUES ('bad-persona', 'bad-subject', '', 'not-json', '[]', '[]', 0, '{}', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .await
            .expect("insert malformed row");

        assert!(store
            .get_profile("bad-persona", "bad-subject")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn ensure_schema_is_idempotent_and_missing_profile_is_none() {
        let store = fresh().await;
        store.ensure_schema().await.expect("second ensure_schema");
        assert_eq!(store.get_profile("missing", "subject").await.unwrap(), None);
    }

    #[test]
    fn store_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SqlitePersonaProfileStore>();
    }
}
