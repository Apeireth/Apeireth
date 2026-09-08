//! Durable commitment tracking backed by the V11 memory schema.
//!
//! `commitments` and `commitment_events` are the canonical V11 tables. This
//! module does not create a shadow/legacy table. The small `ensure_schema`
//! bootstrap mirrors the V11 definitions for callers using a bare storage
//! pool, and validates an existing table before reads or writes so an
//! incompatible legacy table fails clearly instead of silently changing
//! semantics.

use apeireth_storage::{SqliteConnectionPool, StorageError};
use rusqlite::{params, types::Type, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentKind {
    User,
    Assistant,
    Shared,
    Deadline,
    FollowUp,
}

impl CommitmentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Shared => "shared",
            Self::Deadline => "deadline",
            Self::FollowUp => "follow_up",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "shared" => Some(Self::Shared),
            "deadline" => Some(Self::Deadline),
            "follow_up" => Some(Self::FollowUp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentStatus {
    Active,
    Completed,
    Cancelled,
    Expired,
    Superseded,
}

impl CommitmentStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "open",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            "superseded" => Some(Self::Superseded),
            "open" | "active" => Some(Self::Active),
            _ => None,
        }
    }
}

/// Public commitment model.
///
/// `subject_id` is required and is never inferred from scope or provenance.
/// The other V11 commitment columns are exposed here so reads do not discard
/// durable fields such as confidence, supersession, or retraction timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Commitment {
    pub id: String,
    pub subject_id: String,
    pub kind: CommitmentKind,
    pub status: CommitmentStatus,
    pub text: String,
    pub scope: Option<String>,
    pub provenance: Option<String>,
    pub source_episode: Option<String>,
    pub deadline: Option<i64>,
    pub confidence: f64,
    pub supersedes_id: Option<String>,
    pub retracted_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub revision: i64,
}

/// Compatibility event projection retained by [`SqliteCommitmentStore::events`].
/// The numeric `id` is only a historical projection; it is not a durable ID.
/// Use [`SqliteCommitmentStore::events_lossless`] for the canonical text ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentEvent {
    pub id: i64,
    pub commitment_id: String,
    pub from_status: Option<CommitmentStatus>,
    pub to_status: CommitmentStatus,
    pub at: i64,
    pub reason: Option<String>,
}

/// Lossless projection of a V11 `commitment_events` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentEventRecord {
    /// Exact text primary key from V11 `commitment_events.id`.
    pub event_id: String,
    pub commitment_id: String,
    pub event_type: String,
    pub event_at_ms: i64,
    pub source_episode_id: Option<String>,
    pub payload_json: String,
    pub created_at_ms: i64,
    pub from_status: Option<CommitmentStatus>,
    pub to_status: CommitmentStatus,
    pub reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CommitmentError {
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
    #[error("serialization: {0}")]
    Serialization(String),
    #[error("commitment `{0}` not found")]
    NotFound(String),
    #[error("commitment `{id}` revision conflict: expected {expected}, actual {actual}")]
    Cas {
        id: String,
        expected: i64,
        actual: i64,
    },
    #[error("invalid commitment: {0}")]
    Invalid(String),
}

impl From<rusqlite::Error> for CommitmentError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(StorageError::Db(error))
    }
}

enum TransitionOutcome {
    Updated(Commitment),
    NotFound,
    CasConflict { actual: i64 },
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ProvenanceEnvelope<'a> {
    kind: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
}

fn scope_db(value: Option<&str>) -> String {
    value
        .map(|value| {
            if serde_json::from_str::<serde_json::Value>(value).is_ok() {
                value.to_owned()
            } else {
                serde_json::Value::String(value.to_owned()).to_string()
            }
        })
        .unwrap_or_else(|| "{}".into())
}

fn scope_api(value: String) -> Option<String> {
    if value == "{}" {
        None
    } else if let Ok(serde_json::Value::String(value)) = serde_json::from_str(&value) {
        Some(value)
    } else {
        Some(value)
    }
}

fn provenance_db(kind: CommitmentKind, value: Option<&str>) -> String {
    serde_json::to_string(&ProvenanceEnvelope {
        kind: kind.as_str(),
        value,
    })
    .unwrap_or_else(|_| "{}".into())
}

fn provenance_api(value: String) -> (CommitmentKind, Option<String>) {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&value) {
        if let Some(object) = parsed.as_object() {
            let kind = object
                .get("kind")
                .and_then(|value| value.as_str())
                .and_then(CommitmentKind::parse)
                .unwrap_or(CommitmentKind::User);
            let value = object
                .get("value")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            return (kind, value);
        }
    }
    (
        CommitmentKind::User,
        if value == "{}" { None } else { Some(value) },
    )
}

fn validate_text(value: &str, field: &str) -> Result<(), CommitmentError> {
    if value.trim().is_empty() {
        return Err(CommitmentError::Invalid(format!("{field} is empty")));
    }
    if value.contains('\0') {
        return Err(CommitmentError::Invalid(format!(
            "{field} contains a NUL byte"
        )));
    }
    Ok(())
}

fn validate_subject(subject_id: &str) -> Result<(), CommitmentError> {
    validate_text(subject_id, "subject_id")
}

fn validate_confidence(confidence: f64) -> Result<(), CommitmentError> {
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err(CommitmentError::Invalid(
            "confidence must be finite and between 0 and 1".into(),
        ));
    }
    Ok(())
}

fn embedded_subject(value: Option<&str>) -> Option<String> {
    value.and_then(|raw| {
        serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|value| {
                value
                    .get("subject_id")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            })
    })
}

fn validate_commitment(commitment: &Commitment) -> Result<(), CommitmentError> {
    validate_text(&commitment.id, "id")?;
    validate_subject(&commitment.subject_id)?;
    validate_text(&commitment.text, "text")?;
    validate_confidence(commitment.confidence)?;
    for embedded in [
        embedded_subject(commitment.scope.as_deref()),
        embedded_subject(commitment.provenance.as_deref()),
    ]
    .into_iter()
    .flatten()
    {
        if embedded.trim().is_empty() || embedded != commitment.subject_id {
            return Err(CommitmentError::Invalid(
                "scope/provenance subject_id does not match subject_id".into(),
            ));
        }
    }
    Ok(())
}

const REQUIRED_COMMITMENT_COLUMNS: &[&str] = &[
    "id",
    "subject_id",
    "statement",
    "status",
    "due_at_ms",
    "scope_json",
    "source_episode_id",
    "provenance_json",
    "confidence",
    "revision",
    "supersedes_id",
    "retracted_at_ms",
    "completed_at_ms",
    "created_at_ms",
    "updated_at_ms",
];
const REQUIRED_EVENT_COLUMNS: &[&str] = &[
    "id",
    "commitment_id",
    "event_type",
    "event_at_ms",
    "source_episode_id",
    "payload_json",
    "created_at_ms",
];
const SELECT_COMMITMENT: &str = "SELECT id,subject_id,statement,status,due_at_ms,scope_json,source_episode_id,provenance_json,confidence,revision,supersedes_id,retracted_at_ms,completed_at_ms,created_at_ms,updated_at_ms FROM commitments";

fn validate_schema_columns(
    connection: &rusqlite::Connection,
    table: &str,
    required: &[&str],
) -> Result<(), StorageError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let actual = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let missing: Vec<_> = required
        .iter()
        .copied()
        .filter(|column| !actual.iter().any(|actual| actual == column))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(StorageError::Serialization(format!(
            "incompatible legacy {table} table: missing V11 columns {}; use the V11 migration or an explicit data migration; no shadow table is supported",
            missing.join(", ")
        )))
    }
}

#[derive(Clone)]
pub struct SqliteCommitmentStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SqliteCommitmentStore {
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Opens a file-backed commitment store and validates the V11 schema.
    pub async fn open(path: impl AsRef<std::path::Path>) -> Result<Self, CommitmentError> {
        let store = Self::new(SqliteConnectionPool::open(path).await?);
        store.ensure_schema().await?;
        Ok(store)
    }

    /// Opens a shared in-memory commitment store and validates the V11 schema.
    pub async fn in_memory() -> Result<Self, CommitmentError> {
        let store = Self::new(SqliteConnectionPool::in_memory().await?);
        store.ensure_schema().await?;
        Ok(store)
    }

    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqliteConnectionPool {
        &self.pool
    }

    /// Ensures the canonical V11 tables exist, then rejects incompatible
    /// pre-V11/legacy tables rather than adapting them implicitly.
    pub async fn ensure_schema(&self) -> Result<(), CommitmentError> {
        let pool = self.pool.clone();
        pool.write(|connection| {
            let commitments_exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='commitments')",
                [],
                |row| row.get(0),
            )?;
            if commitments_exists {
                validate_schema_columns(connection, "commitments", REQUIRED_COMMITMENT_COLUMNS)?;
            }
            let events_exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='commitment_events')",
                [],
                |row| row.get(0),
            )?;
            if events_exists {
                validate_schema_columns(connection, "commitment_events", REQUIRED_EVENT_COLUMNS)?;
            }
            connection.execute_batch(
                r#"
CREATE TABLE IF NOT EXISTS commitments (
    id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL,
    statement TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    due_at_ms INTEGER,
    scope_json TEXT NOT NULL DEFAULT '{}',
    source_episode_id TEXT,
    provenance_json TEXT NOT NULL DEFAULT '{}',
    confidence REAL NOT NULL DEFAULT 0.5,
    revision INTEGER NOT NULL DEFAULT 0,
    supersedes_id TEXT,
    retracted_at_ms INTEGER,
    completed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_commitments_subject_status
    ON commitments(subject_id,status,due_at_ms);
CREATE INDEX IF NOT EXISTS idx_commitments_due
    ON commitments(status,due_at_ms);
CREATE INDEX IF NOT EXISTS idx_commitments_source
    ON commitments(source_episode_id);
CREATE TABLE IF NOT EXISTS commitment_events (
    id TEXT PRIMARY KEY,
    commitment_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_at_ms INTEGER NOT NULL,
    source_episode_id TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_commitment_events_commitment
    ON commitment_events(commitment_id,event_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_commitment_events_type_time
    ON commitment_events(event_type,event_at_ms DESC);
CREATE TRIGGER IF NOT EXISTS commitment_events_no_delete
BEFORE DELETE ON commitment_events BEGIN
    SELECT RAISE(ABORT,'commitment_events: hard DELETE forbidden');
END;
"#,
            )?;
            validate_schema_columns(connection, "commitments", REQUIRED_COMMITMENT_COLUMNS)?;
            validate_schema_columns(connection, "commitment_events", REQUIRED_EVENT_COLUMNS)
        })
        .await
        .map_err(Into::into)
    }

    pub async fn create(&self, mut commitment: Commitment) -> Result<Commitment, CommitmentError> {
        validate_commitment(&commitment)?;
        commitment.status = CommitmentStatus::Active;
        commitment.revision = 0;
        commitment.retracted_at_ms = None;
        commitment.completed_at_ms = None;
        let value = commitment.clone();

        self.pool
            .write(move |connection| {
                let transaction = connection.transaction()?;
                transaction.execute(
                    "INSERT INTO commitments (id,subject_id,statement,status,due_at_ms,scope_json,source_episode_id,provenance_json,confidence,revision,supersedes_id,retracted_at_ms,completed_at_ms,created_at_ms,updated_at_ms) VALUES (?1,?2,?3,'open',?4,?5,?6,?7,?8,0,?9,NULL,NULL,?10,?11)",
                    params![
                        value.id,
                        value.subject_id,
                        value.text,
                        value.deadline,
                        scope_db(value.scope.as_deref()),
                        value.source_episode,
                        provenance_db(value.kind, value.provenance.as_deref()),
                        value.confidence,
                        value.supersedes_id,
                        value.created_at,
                        value.updated_at
                    ],
                )?;
                let payload = serde_json::json!({
                    "from_status": serde_json::Value::Null,
                    "to_status": CommitmentStatus::Active.as_str(),
                    "reason": "created",
                });
                transaction.execute(
                    "INSERT INTO commitment_events (id,commitment_id,event_type,event_at_ms,source_episode_id,payload_json,created_at_ms) VALUES (?1,?2,'created',?3,?4,?5,?3)",
                    params![
                        format!("{}:0", value.id),
                        value.id,
                        value.created_at,
                        value.source_episode,
                        payload.to_string()
                    ],
                )?;
                transaction.commit()?;
                Ok(())
            })
            .await?;
        Ok(commitment)
    }

    pub fn get(&self, id: &str) -> Result<Option<Commitment>, CommitmentError> {
        self.pool
            .read(|connection| {
                connection
                    .query_row(
                        &format!("{SELECT_COMMITMENT} WHERE id=?1"),
                        params![id],
                        row_commitment,
                    )
                    .optional()
                    .map_err(Into::into)
            })
            .map_err(Into::into)
    }

    /// Subject-scoped read. A mismatch returns no row, avoiding existence
    /// leakage across principals.
    pub fn get_for_subject(
        &self,
        subject_id: &str,
        id: &str,
    ) -> Result<Option<Commitment>, CommitmentError> {
        validate_subject(subject_id)?;
        let subject_id = subject_id.to_owned();
        self.pool
            .read(|connection| {
                connection
                    .query_row(
                        &format!("{SELECT_COMMITMENT} WHERE id=?1 AND subject_id=?2"),
                        params![id, subject_id],
                        row_commitment,
                    )
                    .optional()
                    .map_err(Into::into)
            })
            .map_err(Into::into)
    }

    pub fn list_active(
        &self,
        scope: Option<&str>,
        now_ms: Option<i64>,
    ) -> Result<Vec<Commitment>, CommitmentError> {
        self.list_active_query(None, scope, now_ms)
    }

    /// Lists active commitments for one explicit subject.
    pub fn list_active_for_subject(
        &self,
        subject_id: &str,
        scope: Option<&str>,
        now_ms: Option<i64>,
    ) -> Result<Vec<Commitment>, CommitmentError> {
        validate_subject(subject_id)?;
        self.list_active_query(Some(subject_id), scope, now_ms)
    }

    fn list_active_query(
        &self,
        subject_id: Option<&str>,
        scope: Option<&str>,
        now_ms: Option<i64>,
    ) -> Result<Vec<Commitment>, CommitmentError> {
        let subject_id = subject_id.map(str::to_owned);
        let scope_json = scope.map(|value| scope_db(Some(value)));
        self.pool
            .read(|connection| {
                let mut statement = connection.prepare(&format!(
                    "{SELECT_COMMITMENT} WHERE status='open' AND (?1 IS NULL OR subject_id=?1) AND (?2 IS NULL OR scope_json=?3 OR scope_json=json(?3)) AND (?4 IS NULL OR due_at_ms IS NULL OR due_at_ms>=?4) ORDER BY COALESCE(due_at_ms,9223372036854775807),created_at_ms,id"
                ))?;
                let rows = statement.query_map(
                    params![subject_id, scope, scope_json, now_ms],
                    row_commitment,
                )?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
            })
            .map_err(Into::into)
    }

    pub async fn transition(
        &self,
        id: &str,
        expected_revision: i64,
        to: CommitmentStatus,
        at: i64,
        reason: Option<&str>,
    ) -> Result<Commitment, CommitmentError> {
        self.transition_inner(None, id, expected_revision, to, at, reason)
            .await
    }

    /// Subject-scoped CAS transition. A subject mismatch is reported as
    /// not-found, avoiding cross-subject existence leakage.
    pub async fn transition_for_subject(
        &self,
        subject_id: &str,
        id: &str,
        expected_revision: i64,
        to: CommitmentStatus,
        at: i64,
        reason: Option<&str>,
    ) -> Result<Commitment, CommitmentError> {
        validate_subject(subject_id)?;
        self.transition_inner(Some(subject_id), id, expected_revision, to, at, reason)
            .await
    }

    async fn transition_inner(
        &self,
        subject_id: Option<&str>,
        id: &str,
        expected_revision: i64,
        to: CommitmentStatus,
        at: i64,
        reason: Option<&str>,
    ) -> Result<Commitment, CommitmentError> {
        validate_text(id, "id")?;
        if expected_revision < 0 {
            return Err(CommitmentError::Invalid(
                "expected_revision must not be negative".into(),
            ));
        }
        if to == CommitmentStatus::Active {
            return Err(CommitmentError::Invalid(
                "active is the creation state and cannot be restored by transition".into(),
            ));
        }
        let id = id.to_owned();
        let subject_id = subject_id.map(str::to_owned);
        let reason = reason.map(str::to_owned);
        let lookup = id.clone();
        let outcome = self
            .pool
            .write(move |connection| {
                let transaction = connection.transaction()?;
                let current: Option<(String, String, i64, Option<String>)> = match subject_id {
                    Some(ref subject_id) => transaction
                        .query_row(
                            "SELECT status,subject_id,revision,source_episode_id FROM commitments WHERE id=?1 AND subject_id=?2",
                            params![&lookup, subject_id],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                        )
                        .optional()?,
                    None => transaction
                        .query_row(
                            "SELECT status,subject_id,revision,source_episode_id FROM commitments WHERE id=?1",
                            params![&lookup],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                        )
                        .optional()?,
                };
                let Some((from_raw, _subject, revision, source_episode_id)) = current else {
                    return Ok(TransitionOutcome::NotFound);
                };
                if revision != expected_revision {
                    return Ok(TransitionOutcome::CasConflict { actual: revision });
                }
                let Some(from) = CommitmentStatus::parse(&from_raw) else {
                    return Ok(TransitionOutcome::Invalid(format!(
                        "stored commitment status `{from_raw}` is invalid"
                    )));
                };
                if from != CommitmentStatus::Active {
                    return Ok(TransitionOutcome::Invalid(format!(
                        "cannot transition terminal commitment from {}",
                        from.as_str()
                    )));
                }

                let completed_at = (to == CommitmentStatus::Completed).then_some(at);
                let retracted_at = matches!(
                    to,
                    CommitmentStatus::Cancelled
                        | CommitmentStatus::Expired
                        | CommitmentStatus::Superseded
                )
                .then_some(at);
                let next_revision = expected_revision + 1;
                let changed = transaction.execute(
                    "UPDATE commitments SET status=?1,updated_at_ms=?2,revision=revision+1,completed_at_ms=?3,retracted_at_ms=?4 WHERE id=?5 AND revision=?6",
                    params![to.as_str(), at, completed_at, retracted_at, &lookup, expected_revision],
                )?;
                if changed != 1 {
                    let actual = transaction
                        .query_row(
                            "SELECT revision FROM commitments WHERE id=?1",
                            params![&lookup],
                            |row| row.get(0),
                        )
                        .optional()?
                        .unwrap_or(expected_revision);
                    return Ok(TransitionOutcome::CasConflict { actual });
                }
                let payload = serde_json::json!({
                    "from_status": from.as_str(),
                    "to_status": to.as_str(),
                    "reason": reason,
                });
                transaction.execute(
                    "INSERT INTO commitment_events (id,commitment_id,event_type,event_at_ms,source_episode_id,payload_json,created_at_ms) VALUES (?1,?2,'status_changed',?3,?4,?5,?3)",
                    params![
                        format!("{}:{}", lookup, next_revision),
                        lookup,
                        at,
                        source_episode_id,
                        payload.to_string()
                    ],
                )?;
                let updated = transaction.query_row(
                    &format!("{SELECT_COMMITMENT} WHERE id=?1"),
                    params![&lookup],
                    row_commitment,
                )?;
                transaction.commit()?;
                Ok(TransitionOutcome::Updated(updated))
            })
            .await?;
        match outcome {
            TransitionOutcome::Updated(commitment) => Ok(commitment),
            TransitionOutcome::NotFound => Err(CommitmentError::NotFound(id)),
            TransitionOutcome::CasConflict { actual } => Err(CommitmentError::Cas {
                id,
                expected: expected_revision,
                actual,
            }),
            TransitionOutcome::Invalid(reason) => Err(CommitmentError::Invalid(reason)),
        }
    }

    /// Compatibility projection retaining the historical numeric event ID.
    pub fn events(&self, id: &str) -> Result<Vec<CommitmentEvent>, CommitmentError> {
        Ok(self
            .events_lossless(id)?
            .into_iter()
            .map(|event| CommitmentEvent {
                id: event_numeric(&event.event_id),
                commitment_id: event.commitment_id,
                from_status: event.from_status,
                to_status: event.to_status,
                at: event.event_at_ms,
                reason: event.reason,
            })
            .collect())
    }

    /// Returns exact V11 event IDs and every persisted event column.
    pub fn events_lossless(&self, id: &str) -> Result<Vec<CommitmentEventRecord>, CommitmentError> {
        self.pool
            .read(|connection| {
                let mut statement = connection.prepare(
                    "SELECT id,commitment_id,event_type,event_at_ms,source_episode_id,payload_json,created_at_ms FROM commitment_events WHERE commitment_id=?1 ORDER BY event_at_ms,id",
                )?;
                let rows = statement.query_map(params![id], row_event)?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
            })
            .map_err(Into::into)
    }

    /// Alias for callers preferring an explicit name over `events_lossless`.
    pub fn events_with_ids(&self, id: &str) -> Result<Vec<CommitmentEventRecord>, CommitmentError> {
        self.events_lossless(id)
    }
}

fn event_numeric(id: &str) -> i64 {
    id.bytes().fold(1469598103934665603i64, |hash, byte| {
        hash.wrapping_mul(1099511628211)
            .wrapping_add(i64::from(byte))
    })
}

fn row_commitment(row: &rusqlite::Row<'_>) -> rusqlite::Result<Commitment> {
    let status_raw: String = row.get(3)?;
    let status = CommitmentStatus::parse(&status_raw).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            format!("invalid commitment status `{status_raw}`").into(),
        )
    })?;
    let subject_id: String = row.get(1)?;
    if subject_id.trim().is_empty() {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            1,
            Type::Text,
            "commitment subject_id is empty".into(),
        ));
    }
    let confidence: f64 = row.get(8)?;
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            8,
            Type::Real,
            "commitment confidence is outside [0, 1]".into(),
        ));
    }
    let (kind, provenance) = provenance_api(row.get(7)?);
    Ok(Commitment {
        id: row.get(0)?,
        subject_id,
        kind,
        status,
        text: row.get(2)?,
        scope: scope_api(row.get(5)?),
        provenance,
        source_episode: row.get(6)?,
        deadline: row.get(4)?,
        confidence,
        supersedes_id: row.get(10)?,
        retracted_at_ms: row.get(11)?,
        completed_at_ms: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        revision: row.get(9)?,
    })
}

fn event_status(
    value: Option<&serde_json::Value>,
    field: &str,
) -> rusqlite::Result<Option<CommitmentStatus>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(raw) = value.as_str() else {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            format!("event `{field}` is not text").into(),
        ));
    };
    CommitmentStatus::parse(raw).map(Some).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            format!("invalid event {field} `{raw}`").into(),
        )
    })
}

fn row_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommitmentEventRecord> {
    let payload_json: String = row.get(5)?;
    let payload: serde_json::Value = serde_json::from_str(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, Type::Text, Box::new(error))
    })?;
    let from_status = event_status(payload.get("from_status"), "from_status")?;
    let to_status = event_status(payload.get("to_status"), "to_status")?.unwrap_or_else(|| {
        // Historical `created` events can omit to_status; creation is the only
        // event whose implicit target is open.
        CommitmentStatus::Active
    });
    Ok(CommitmentEventRecord {
        event_id: row.get(0)?,
        commitment_id: row.get(1)?,
        event_type: row.get(2)?,
        event_at_ms: row.get(3)?,
        source_episode_id: row.get(4)?,
        payload_json,
        created_at_ms: row.get(6)?,
        from_status,
        to_status,
        reason: payload
            .get("reason")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    async fn store() -> SqliteCommitmentStore {
        SqliteCommitmentStore::in_memory().await.unwrap()
    }

    fn commitment() -> Commitment {
        Commitment {
            id: "c1".into(),
            subject_id: "subject-1".into(),
            kind: CommitmentKind::Shared,
            status: CommitmentStatus::Active,
            text: "do it".into(),
            scope: Some("s".into()),
            provenance: Some("dialog".into()),
            source_episode: Some("e1".into()),
            deadline: Some(20),
            confidence: 0.8,
            supersedes_id: Some("old-c".into()),
            retracted_at_ms: None,
            completed_at_ms: None,
            created_at: 10,
            updated_at: 10,
            revision: 0,
        }
    }

    #[tokio::test]
    async fn lifecycle_cas_and_lossless_audit_are_atomic() {
        let store = store().await;
        store.create(commitment()).await.unwrap();
        let completed = store
            .transition("c1", 0, CommitmentStatus::Completed, 11, Some("done"))
            .await
            .unwrap();
        assert_eq!(completed.status, CommitmentStatus::Completed);
        assert_eq!(completed.completed_at_ms, Some(11));
        assert_eq!(store.events("c1").unwrap().len(), 2);
        let events = store.events_lossless("c1").unwrap();
        assert_eq!(events[0].event_id, "c1:0");
        assert_eq!(events[1].event_id, "c1:1");
        assert_eq!(events[1].event_type, "status_changed");
        assert!(events[1].payload_json.contains("done"));
        assert!(matches!(
            store
                .transition("c1", 0, CommitmentStatus::Cancelled, 12, None)
                .await,
            Err(CommitmentError::Cas { actual: 1, .. })
        ));
        assert!(matches!(
            store
                .transition("c1", 1, CommitmentStatus::Cancelled, 12, None)
                .await,
            Err(CommitmentError::Invalid(_))
        ));
        assert_eq!(store.events("c1").unwrap().len(), 2);
    }

    #[tokio::test]
    async fn subject_is_required_and_v11_fields_round_trip() {
        let store = store().await;
        let mut missing = commitment();
        missing.subject_id.clear();
        assert!(matches!(
            store.create(missing).await,
            Err(CommitmentError::Invalid(message)) if message.contains("subject_id")
        ));

        let created = store.create(commitment()).await.unwrap();
        assert_eq!(created.subject_id, "subject-1");
        let loaded = store.get("c1").unwrap().unwrap();
        assert_eq!(loaded.subject_id, "subject-1");
        assert_eq!(loaded.confidence, 0.8);
        assert_eq!(loaded.supersedes_id.as_deref(), Some("old-c"));
        assert_eq!(loaded.source_episode.as_deref(), Some("e1"));
        assert_eq!(loaded.deadline, Some(20));
        assert_eq!(
            store
                .list_active_for_subject("subject-1", None, None)
                .unwrap()
                .len(),
            1
        );
        assert!(store
            .list_active_for_subject("other-subject", None, None)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn restart_preserves_state_and_event_ids() {
        let file = NamedTempFile::new().unwrap();
        let first = SqliteCommitmentStore::open(file.path()).await.unwrap();
        first.create(commitment()).await.unwrap();
        first
            .transition("c1", 0, CommitmentStatus::Cancelled, 11, None)
            .await
            .unwrap();
        drop(first);

        let reopened = SqliteCommitmentStore::open(file.path()).await.unwrap();
        let loaded = reopened.get("c1").unwrap().unwrap();
        assert_eq!(loaded.status, CommitmentStatus::Cancelled);
        assert_eq!(loaded.retracted_at_ms, Some(11));
        assert_eq!(loaded.revision, 1);
        assert_eq!(
            reopened
                .events_lossless("c1")
                .unwrap()
                .into_iter()
                .map(|event| event.event_id)
                .collect::<Vec<_>>(),
            vec!["c1:0", "c1:1"]
        );
    }

    #[tokio::test]
    async fn incompatible_legacy_table_fails_clearly() {
        let pool = SqliteConnectionPool::in_memory().await.unwrap();
        pool.write(|connection| {
            connection
                .execute_batch(
                    "CREATE TABLE commitments (id TEXT PRIMARY KEY, statement TEXT NOT NULL);",
                )
                .map_err(StorageError::from)
        })
        .await
        .unwrap();
        let store = SqliteCommitmentStore::new(pool);
        let error = store.ensure_schema().await.unwrap_err().to_string();
        assert!(error.contains("incompatible legacy commitments table"));
        assert!(error.contains("subject_id"));
    }
}
