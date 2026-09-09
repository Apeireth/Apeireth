//! Append-only memory access telemetry and ACT-R-style activation.
//!
//! V11 deliberately makes `memory_access_events` durable: its delete trigger
//! rejects physical deletion. The store therefore uses `cap` as a bounded
//! detailed-read/activation window and maintains one hourly row per memory in
//! `memory_access_aggregates`. Aggregates are updated in the same transaction
//! as each event, so retention never requires deleting an audit event.

use apeireth_storage::{SqliteConnectionPool, StorageError};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use async_trait::async_trait;

const HOUR_MS: i64 = 3_600_000;
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_METADATA_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessEvent {
    pub id: String,
    pub memory_id: String,
    pub subject_id: Option<String>,
    pub session_id: Option<String>,
    pub accessed_at_ms: i64,
    pub query: Option<String>,
    pub access_kind: String,
    pub rank: Option<i64>,
    pub score: Option<f64>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessAggregate {
    pub memory_id: String,
    pub bucket_start_ms: i64,
    pub access_count: i64,
    pub last_accessed_at_ms: Option<i64>,
    pub cumulative_score: f64,
    pub updated_at_ms: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AccessHistoryError {
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
    #[error("invalid access: {0}")]
    Invalid(String),
    #[error("metadata: {0}")]
    Metadata(#[from] serde_json::Error),
}

impl From<rusqlite::Error> for AccessHistoryError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(StorageError::Db(e))
    }
}

/// Supplies an optional activation value for a memory candidate.
///
/// `None` means that the coordinator should retain the candidate's existing
/// activation. This keeps activation opt-in and preserves the legacy ranking
/// behavior when no source is configured (or when a source has no value).
pub trait ActivationSource: Send + Sync {
    fn activation(&self, memory_id: &str, as_of_ms: i64) -> Option<f64>;
}

/// Adapter that exposes the existing access-history ACT-R API as an activation
/// source for the coordinator.
pub struct AccessHistoryActivationSource {
    store: Arc<SqliteAccessHistoryStore>,
    decay: f64,
    beta: f64,
}

impl AccessHistoryActivationSource {
    pub fn new(store: Arc<SqliteAccessHistoryStore>, decay: f64, beta: f64) -> Self {
        Self { store, decay, beta }
    }
}

impl ActivationSource for AccessHistoryActivationSource {
    fn activation(&self, memory_id: &str, as_of_ms: i64) -> Option<f64> {
        self.store
            .activation(memory_id, as_of_ms, self.decay, self.beta)
            .ok()
            .filter(|value| value.is_finite())
    }
}

pub struct SqliteAccessHistoryStore {
    pool: Arc<SqliteConnectionPool>,
    cap: usize,
}

impl SqliteAccessHistoryStore {
    pub fn new(pool: SqliteConnectionPool, cap: usize) -> Self {
        Self {
            pool: Arc::new(pool),
            cap,
        }
    }

    pub fn from_arc(pool: Arc<SqliteConnectionPool>, cap: usize) -> Self {
        Self { pool, cap }
    }

    pub fn pool(&self) -> &SqliteConnectionPool {
        &self.pool
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Ensures the V11 access tables exist for callers using a bare storage pool.
    ///
    /// Normal application startup should apply `migrations::V11` instead. This
    /// idempotent bootstrap is retained for isolated callers and tests, and
    /// intentionally creates the same columns and no shadow event table.
    pub async fn ensure_schema(&self) -> Result<(), AccessHistoryError> {
        let pool = self.pool.clone();
        pool.write(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS memory_access_events (
                    id TEXT PRIMARY KEY,
                    memory_id TEXT NOT NULL,
                    subject_id TEXT,
                    session_id TEXT,
                    accessed_at_ms INTEGER NOT NULL,
                    query TEXT,
                    access_kind TEXT NOT NULL DEFAULT 'recall',
                    rank INTEGER,
                    score REAL,
                    metadata_json TEXT NOT NULL DEFAULT '{}'
                );
                CREATE INDEX IF NOT EXISTS idx_memory_access_events_memory
                    ON memory_access_events(memory_id, accessed_at_ms DESC);
                CREATE INDEX IF NOT EXISTS idx_memory_access_events_subject_time
                    ON memory_access_events(subject_id, accessed_at_ms DESC);
                CREATE INDEX IF NOT EXISTS idx_memory_access_events_session_time
                    ON memory_access_events(session_id, accessed_at_ms DESC);
                CREATE TABLE IF NOT EXISTS memory_access_aggregates (
                    memory_id TEXT NOT NULL,
                    bucket_start_ms INTEGER NOT NULL,
                    access_count INTEGER NOT NULL DEFAULT 0,
                    last_accessed_at_ms INTEGER,
                    cumulative_score REAL NOT NULL DEFAULT 0.0,
                    updated_at_ms INTEGER NOT NULL,
                    PRIMARY KEY (memory_id, bucket_start_ms)
                );
                CREATE INDEX IF NOT EXISTS idx_memory_access_aggregates_recent
                    ON memory_access_aggregates(memory_id, last_accessed_at_ms DESC);
                CREATE TRIGGER IF NOT EXISTS memory_access_events_no_delete
                    BEFORE DELETE ON memory_access_events BEGIN
                    SELECT RAISE(ABORT, 'memory_access_events: hard DELETE forbidden');
                END;",
            )?;
            Ok(())
        })
        .await
        .map_err(Into::into)
    }

    /// Compatibility wrapper for the former four-argument API.
    pub async fn record_access(
        &self,
        memory_id: &str,
        selected_at_ms: i64,
        session_id: &str,
        source: &str,
    ) -> Result<AccessEvent, AccessHistoryError> {
        self.record_event(
            memory_id,
            None,
            (!session_id.trim().is_empty()).then(|| session_id.to_owned()),
            selected_at_ms,
            None,
            source,
            None,
            None,
            Value::Object(Default::default()),
        )
        .await
    }

    /// Records one V11 event and updates its hourly aggregate atomically.
    ///
    /// Event IDs are UUIDv5 values derived from the canonical event payload.
    /// Repeated identical events receive deterministic ordinal suffixes rather
    /// than being silently collapsed, preserving telemetry while avoiding
    /// random IDs. The writer queue serializes ordinal allocation.
    pub async fn record_event(
        &self,
        memory_id: &str,
        subject_id: Option<String>,
        session_id: Option<String>,
        accessed_at_ms: i64,
        query: Option<String>,
        access_kind: &str,
        rank: Option<i64>,
        score: Option<f64>,
        metadata: Value,
    ) -> Result<AccessEvent, AccessHistoryError> {
        validate_text(memory_id, "memory_id")?;
        validate_text(access_kind, "access_kind")?;
        validate_optional_text(subject_id.as_deref(), "subject_id")?;
        validate_optional_text(session_id.as_deref(), "session_id")?;
        validate_optional_text(query.as_deref(), "query")?;
        if rank.is_some_and(|value| value < 0) {
            return Err(AccessHistoryError::Invalid(
                "rank must be non-negative".into(),
            ));
        }
        if !score.map_or(true, f64::is_finite) {
            return Err(AccessHistoryError::Invalid("score must be finite".into()));
        }

        let metadata_json = serde_json::to_string(&metadata)?;
        if metadata_json.len() > MAX_METADATA_BYTES {
            return Err(AccessHistoryError::Invalid(format!(
                "metadata exceeds {MAX_METADATA_BYTES} bytes"
            )));
        }
        let identity = EventIdentity {
            memory_id,
            subject_id: subject_id.as_deref(),
            session_id: session_id.as_deref(),
            accessed_at_ms,
            query: query.as_deref(),
            access_kind,
            rank,
            score,
            metadata_json: &metadata_json,
        };
        let identity_json = serde_json::to_string(&identity)?;
        let bucket_start_ms = accessed_at_ms.div_euclid(HOUR_MS) * HOUR_MS;
        let event = AccessEvent {
            id: String::new(),
            memory_id: memory_id.to_owned(),
            subject_id,
            session_id,
            accessed_at_ms,
            query,
            access_kind: access_kind.to_owned(),
            rank,
            score,
            metadata,
        };
        let pool = self.pool.clone();
        pool.write(move |conn| {
            let tx = conn.transaction()?;
            let id = deterministic_event_id(&tx, &identity_json)?;
            let event = AccessEvent { id, ..event };
            tx.execute(
                "INSERT INTO memory_access_events
                 (id,memory_id,subject_id,session_id,accessed_at_ms,query,access_kind,rank,score,metadata_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    &event.id,
                    &event.memory_id,
                    event.subject_id.as_deref(),
                    event.session_id.as_deref(),
                    event.accessed_at_ms,
                    event.query.as_deref(),
                    &event.access_kind,
                    event.rank,
                    event.score,
                    &metadata_json
                ],
            )?;
            tx.execute(
                "INSERT INTO memory_access_aggregates
                 (memory_id,bucket_start_ms,access_count,last_accessed_at_ms,cumulative_score,updated_at_ms)
                 VALUES (?1,?2,1,?3,?4,?5)
                 ON CONFLICT(memory_id,bucket_start_ms) DO UPDATE SET
                   access_count=access_count+1,
                   last_accessed_at_ms=MAX(COALESCE(last_accessed_at_ms, excluded.last_accessed_at_ms), excluded.last_accessed_at_ms),
                   cumulative_score=cumulative_score+excluded.cumulative_score,
                   updated_at_ms=MAX(updated_at_ms, excluded.updated_at_ms)",
                params![
                    &event.memory_id,
                    bucket_start_ms,
                    event.accessed_at_ms,
                    event.score.unwrap_or(0.0),
                    event.accessed_at_ms
                ],
            )?;
            tx.commit()?;
            Ok(event)
        })
        .await
        .map_err(Into::into)
    }

    /// Records a retrieval result selected for context injection.
    pub async fn record_selected_context(
        &self,
        memory_id: &str,
        subject_id: Option<String>,
        session_id: Option<String>,
        accessed_at_ms: i64,
        query: Option<String>,
        rank: Option<i64>,
        score: Option<f64>,
        metadata: Value,
    ) -> Result<AccessEvent, AccessHistoryError> {
        self.record_event(
            memory_id,
            subject_id,
            session_id,
            accessed_at_ms,
            query,
            "selected_context",
            rank,
            score,
            metadata,
        )
        .await
    }

    /// Returns the bounded detailed event window for a memory.
    pub fn latest_accesses(
        &self,
        memory_id: &str,
        limit: usize,
    ) -> Result<Vec<AccessEvent>, AccessHistoryError> {
        validate_text(memory_id, "memory_id")?;
        if limit == 0 || self.cap == 0 {
            return Ok(Vec::new());
        }
        let limit = sqlite_limit(limit.min(self.cap))?;
        self.pool
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id,memory_id,subject_id,session_id,accessed_at_ms,query,access_kind,rank,score,metadata_json
                     FROM memory_access_events WHERE memory_id=?1
                     ORDER BY accessed_at_ms DESC,id DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![memory_id, limit], row_access_event)?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
            })
            .map_err(Into::into)
    }

    /// Returns bounded hourly summaries without touching append-only events.
    pub fn latest_aggregates(
        &self,
        memory_id: &str,
        limit: usize,
    ) -> Result<Vec<AccessAggregate>, AccessHistoryError> {
        validate_text(memory_id, "memory_id")?;
        if limit == 0 || self.cap == 0 {
            return Ok(Vec::new());
        }
        let limit = sqlite_limit(limit.min(self.cap))?;
        self.pool
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT memory_id,bucket_start_ms,access_count,last_accessed_at_ms,cumulative_score,updated_at_ms
                     FROM memory_access_aggregates WHERE memory_id=?1
                     ORDER BY bucket_start_ms DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![memory_id, limit], row_access_aggregate)?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
            })
            .map_err(Into::into)
    }

    pub fn activation(
        &self,
        memory_id: &str,
        as_of_ms: i64,
        decay: f64,
        beta: f64,
    ) -> Result<f64, AccessHistoryError> {
        validate_text(memory_id, "memory_id")?;
        validate_act_r(decay, beta)?;
        Ok(act_r_activation(
            &self.latest_accesses(memory_id, self.cap)?,
            as_of_ms,
            decay,
            beta,
        ))
    }
}

#[derive(Serialize)]
struct EventIdentity<'a> {
    memory_id: &'a str,
    subject_id: Option<&'a str>,
    session_id: Option<&'a str>,
    accessed_at_ms: i64,
    query: Option<&'a str>,
    access_kind: &'a str,
    rank: Option<i64>,
    score: Option<f64>,
    metadata_json: &'a str,
}

fn deterministic_event_id(
    conn: &rusqlite::Connection,
    identity_json: &str,
) -> rusqlite::Result<String> {
    for ordinal in 0_u64.. {
        let name = format!("apeireth-memory-access-v11\0{identity_json}\0{ordinal}");
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
            .simple()
            .to_string();
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM memory_access_events WHERE id=?1)",
            params![&id],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(id);
        }
    }
    unreachable!("u64 ordinal exhausted")
}

fn validate_text(value: &str, field: &str) -> Result<(), AccessHistoryError> {
    if value.trim().is_empty() {
        return Err(AccessHistoryError::Invalid(format!("{field} is empty")));
    }
    if value.len() > MAX_TEXT_BYTES {
        return Err(AccessHistoryError::Invalid(format!(
            "{field} exceeds {MAX_TEXT_BYTES} bytes"
        )));
    }
    if value.contains('\0') {
        return Err(AccessHistoryError::Invalid(format!(
            "{field} contains a NUL byte"
        )));
    }
    Ok(())
}

fn validate_optional_text(value: Option<&str>, field: &str) -> Result<(), AccessHistoryError> {
    if let Some(value) = value {
        validate_text(value, field)?;
    }
    Ok(())
}

fn sqlite_limit(limit: usize) -> Result<i64, AccessHistoryError> {
    i64::try_from(limit).map_err(|_| AccessHistoryError::Invalid("limit is too large".into()))
}

fn row_access_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessEvent> {
    let metadata_json: String = row.get(9)?;
    let metadata = serde_json::from_str(&metadata_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(AccessEvent {
        id: row.get(0)?,
        memory_id: row.get(1)?,
        subject_id: row.get(2)?,
        session_id: row.get(3)?,
        accessed_at_ms: row.get(4)?,
        query: row.get(5)?,
        access_kind: row.get(6)?,
        rank: row.get(7)?,
        score: row.get(8)?,
        metadata,
    })
}

fn row_access_aggregate(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessAggregate> {
    Ok(AccessAggregate {
        memory_id: row.get(0)?,
        bucket_start_ms: row.get(1)?,
        access_count: row.get(2)?,
        last_accessed_at_ms: row.get(3)?,
        cumulative_score: row.get(4)?,
        updated_at_ms: row.get(5)?,
    })
}

fn validate_act_r(decay: f64, beta: f64) -> Result<(), AccessHistoryError> {
    if !decay.is_finite() || decay <= 0.0 || !beta.is_finite() {
        return Err(AccessHistoryError::Invalid(
            "decay must be finite and positive; beta finite".into(),
        ));
    }
    Ok(())
}

/// Calculates ACT-R base-level activation from the supplied access events.
///
/// Events newer than `as_of_ms` are clipped to a one-second age. This keeps
/// clock skew from producing an unbounded activation while retaining the
/// canonical ACT-R logarithmic sum and beta offset.
pub fn act_r_activation(accesses: &[AccessEvent], as_of_ms: i64, decay: f64, beta: f64) -> f64 {
    debug_assert!(decay.is_finite() && decay > 0.0 && beta.is_finite());
    let sum: f64 = accesses
        .iter()
        .map(|access| {
            let age_ms = as_of_ms.saturating_sub(access.accessed_at_ms).max(1000);
            (age_ms as f64 / 1000.0).powf(-decay)
        })
        .sum();
    if sum > 0.0 {
        sum.ln() + beta
    } else {
        beta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store(cap: usize) -> SqliteAccessHistoryStore {
        let store =
            SqliteAccessHistoryStore::new(SqliteConnectionPool::in_memory().await.unwrap(), cap);
        store.ensure_schema().await.unwrap();
        store
    }

    #[tokio::test]
    async fn activation_source_uses_access_history_api() {
        let store = Arc::new(store(4).await);
        store
            .record_access("memory-a", 1_000, "session", "search")
            .await
            .unwrap();
        let source = AccessHistoryActivationSource::new(store, 0.5, 0.25);
        let value = source.activation("memory-a", 2_000).unwrap();
        assert!((value - 0.25).abs() < f64::EPSILON);
        assert!(source.activation("missing", 2_000).is_some());
    }

    struct FakeActivationSource {
        value: Option<f64>,
    }

    impl ActivationSource for FakeActivationSource {
        fn activation(&self, _memory_id: &str, _as_of_ms: i64) -> Option<f64> {
            self.value
        }
    }

    #[test]
    fn fake_activation_source_preserves_none_contract() {
        let source = FakeActivationSource { value: None };
        assert_eq!(source.activation("memory-a", 1_000), None);
        let source = FakeActivationSource { value: Some(0.75) };
        assert_eq!(source.activation("memory-a", 1_000), Some(0.75));
    }

    #[tokio::test]
    async fn cap_is_a_read_limit_and_aggregate_is_atomic() {
        let store = store(2).await;
        store.record_access("m", 1000, "a", "search").await.unwrap();
        store.record_access("m", 2000, "b", "search").await.unwrap();
        store.record_access("m", 3000, "c", "search").await.unwrap();

        assert_eq!(store.latest_accesses("m", 9).unwrap().len(), 2);
        assert_eq!(store.latest_aggregates("m", 9).unwrap()[0].access_count, 3);
        assert!(store.activation("m", 4000, 0.5, 0.0).unwrap().is_finite());
        let count: i64 = store
            .pool
            .read(|connection| {
                Ok(connection.query_row(
                    "SELECT COUNT(*) FROM memory_access_events",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn ids_are_deterministic_and_ties_have_stable_order() {
        let first = store(10).await;
        let second = store(10).await;
        let metadata = serde_json::json!({"x": 1});
        let first_event = first
            .record_selected_context(
                "m",
                Some("u".into()),
                Some("s".into()),
                1000,
                Some("q".into()),
                Some(1),
                Some(0.5),
                metadata.clone(),
            )
            .await
            .unwrap();
        let second_event = second
            .record_selected_context(
                "m",
                Some("u".into()),
                Some("s".into()),
                1000,
                Some("q".into()),
                Some(1),
                Some(0.5),
                metadata,
            )
            .await
            .unwrap();
        assert_eq!(first_event.id, second_event.id);

        let duplicate = first
            .record_selected_context(
                "m",
                Some("u".into()),
                Some("s".into()),
                1000,
                Some("q".into()),
                Some(1),
                Some(0.5),
                serde_json::json!({"x": 1}),
            )
            .await
            .unwrap();
        assert_ne!(first_event.id, duplicate.id);
        let events = first.latest_accesses("m", 10).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[0].id > events[1].id);
    }

    #[tokio::test]
    async fn validation_rejects_invalid_values() {
        let store = store(2).await;
        assert!(matches!(
            store.record_access(" ", 1, "s", "search").await,
            Err(AccessHistoryError::Invalid(_))
        ));
        assert!(matches!(
            store
                .record_event(
                    "m",
                    None,
                    None,
                    1,
                    Some("".into()),
                    "recall",
                    None,
                    None,
                    Value::Null,
                )
                .await,
            Err(AccessHistoryError::Invalid(_))
        ));
        assert!(matches!(
            store
                .record_event(
                    "m",
                    None,
                    None,
                    1,
                    None,
                    "recall",
                    Some(-1),
                    None,
                    Value::Null,
                )
                .await,
            Err(AccessHistoryError::Invalid(_))
        ));
        assert!(matches!(
            store.latest_accesses("", 1),
            Err(AccessHistoryError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn v11_delete_trigger_protects_events() {
        let store = store(2).await;
        store.record_access("m", 1000, "s", "search").await.unwrap();
        let result = store
            .pool
            .write(|connection| {
                connection.execute("DELETE FROM memory_access_events", [])?;
                Ok(())
            })
            .await;
        assert!(result.is_err());
        assert_eq!(store.latest_accesses("m", 2).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn selected_context_uses_v11_fields() {
        let store = store(2).await;
        let event = store
            .record_selected_context(
                "m",
                Some("u".into()),
                None,
                1,
                Some("q".into()),
                Some(1),
                Some(0.5),
                serde_json::json!({"x": 1}),
            )
            .await
            .unwrap();
        assert_eq!(event.access_kind, "selected_context");
        assert_eq!(event.id.len(), 32);
        assert_eq!(event.subject_id.as_deref(), Some("u"));
        assert_eq!(event.query.as_deref(), Some("q"));
    }

    #[tokio::test]
    async fn histories_are_isolated() {
        let store = store(2).await;
        store.record_access("a", 1, "s", "search").await.unwrap();
        assert!(store.latest_accesses("b", 10).unwrap().is_empty());
    }
}
