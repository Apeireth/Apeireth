//! Durable append-only temporal graph storage backed by the V11 schema.
//!
//! The store deliberately owns no connection management. All mutations are sent
//! through [`SqliteConnectionPool::write`], and all reads use its reader pool.
//! The schema is the existing `temporal_graph_facts` V11 table; no shadow table
//! or ORM is introduced here.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use apeireth_storage::{SqliteConnectionPool, StorageError};
use rusqlite::{params, types::Type, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A single immutable revision of a directed temporal relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalGraphFact {
    pub id: String,
    /// Stable identity of the logical relation across all revisions.
    pub relation_key: String,
    pub subject_id: String,
    pub predicate: String,
    pub object_id: String,
    /// Valid time is a half-open interval: `[valid_from_ms, valid_until_ms)`.
    pub valid_from_ms: i64,
    pub valid_until_ms: Option<i64>,
    /// Transaction/belief time at which this revision became known.
    pub believed_at_ms: i64,
    pub source_episode_id: Option<String>,
    pub scope: Value,
    pub provenance: Value,
    pub confidence: f64,
    /// Assigned by the store; revisions start at zero for each relation key.
    pub revision: i64,
    pub supersedes_id: Option<String>,
    /// A non-null value marks a retraction tombstone. Tombstones are retained.
    pub retracted_at_ms: Option<i64>,
    pub created_at_ms: i64,
}

impl TemporalGraphFact {
    /// Returns whether this row is a retained retraction tombstone.
    pub fn is_retraction(&self) -> bool {
        self.retracted_at_ms.is_some()
    }
}

/// Errors returned by the temporal graph store.
#[derive(Debug, Error)]
pub enum TemporalGraphError {
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
    #[error("invalid temporal graph input: {0}")]
    Invalid(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<rusqlite::Error> for TemporalGraphError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(StorageError::Db(error))
    }
}

/// Deterministic as-of graph query.
///
/// `as_of_ms` is the valid-time probe. Unless explicitly overridden,
/// `belief_as_of_ms` uses the same timestamp, so late-arriving revisions are
/// not visible before they were believed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TemporalGraphQuery {
    pub subject_id: Option<String>,
    pub predicate: Option<String>,
    pub object_id: Option<String>,
    pub relation_key: Option<String>,
    pub scope: Option<Value>,
    pub as_of_ms: i64,
    pub belief_as_of_ms: Option<i64>,
    pub limit: Option<usize>,
}

impl TemporalGraphQuery {
    pub fn new(as_of_ms: i64) -> Self {
        Self {
            as_of_ms,
            ..Self::default()
        }
    }

    pub fn with_as_of_ms(mut self, value: i64) -> Self {
        self.as_of_ms = value;
        self
    }

    pub fn with_belief_as_of_ms(mut self, value: i64) -> Self {
        self.belief_as_of_ms = Some(value);
        self
    }

    pub fn subject(mut self, value: impl Into<String>) -> Self {
        self.subject_id = Some(value.into());
        self
    }

    pub fn predicate(mut self, value: impl Into<String>) -> Self {
        self.predicate = Some(value.into());
        self
    }

    pub fn object(mut self, value: impl Into<String>) -> Self {
        self.object_id = Some(value.into());
        self
    }

    pub fn relation(mut self, value: impl Into<String>) -> Self {
        self.relation_key = Some(value.into());
        self
    }

    pub fn in_scope(mut self, value: Value) -> Self {
        self.scope = Some(value);
        self
    }

    pub fn limit(mut self, value: usize) -> Self {
        self.limit = Some(value);
        self
    }
}

/// Bounds for cycle-safe breadth-first traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalBudget {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
}

impl Default for TraversalBudget {
    fn default() -> Self {
        Self {
            max_depth: 16,
            max_nodes: 1_000,
            max_edges: 5_000,
        }
    }
}

/// Traversal output. `facts` contains each traversed effective edge once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalResult {
    pub nodes: Vec<String>,
    pub facts: Vec<TemporalGraphFact>,
    pub truncated: bool,
}

#[derive(Clone)]
pub struct SqliteTemporalGraphStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SqliteTemporalGraphStore {
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn from_pool(pool: SqliteConnectionPool) -> Self {
        Self::new(pool)
    }

    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    /// Opens a database and ensures the V11 graph table is available.
    pub async fn open(path: impl AsRef<std::path::Path>) -> Result<Self, TemporalGraphError> {
        let store = Self::new(SqliteConnectionPool::open(path).await?);
        store.ensure_schema().await?;
        Ok(store)
    }

    /// Opens a shared in-memory database and ensures the V11 graph table is available.
    pub async fn in_memory() -> Result<Self, TemporalGraphError> {
        let store = Self::new(SqliteConnectionPool::in_memory().await?);
        store.ensure_schema().await?;
        Ok(store)
    }

    pub fn pool(&self) -> &SqliteConnectionPool {
        self.pool.as_ref()
    }

    /// Creates (or upgrades) the V11 table and read indexes for callers whose
    /// migration bootstrap has not run. Existing V11 columns and rows are
    /// untouched. The triggers are deliberately installed separately so an
    /// existing database cannot retain an UPDATE escape hatch.
    pub async fn ensure_schema(&self) -> Result<(), TemporalGraphError> {
        self.pool
            .write(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS temporal_graph_facts (
                        id TEXT PRIMARY KEY,
                        relation_key TEXT NOT NULL,
                        subject_id TEXT NOT NULL,
                        predicate TEXT NOT NULL,
                        object_id TEXT NOT NULL,
                        valid_from_ms INTEGER NOT NULL,
                        valid_until_ms INTEGER,
                        believed_at_ms INTEGER NOT NULL,
                        source_episode_id TEXT,
                        scope_json TEXT NOT NULL DEFAULT '{}',
                        provenance_json TEXT NOT NULL DEFAULT '{}',
                        confidence REAL NOT NULL DEFAULT 0.5,
                        revision INTEGER NOT NULL DEFAULT 0,
                        supersedes_id TEXT,
                        retracted_at_ms INTEGER,
                        created_at_ms INTEGER NOT NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_relation_revision
                        ON temporal_graph_facts(relation_key, revision DESC, id ASC);
                    CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_subject
                        ON temporal_graph_facts(subject_id, predicate, valid_from_ms);
                    CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_object
                        ON temporal_graph_facts(object_id, predicate, valid_from_ms);
                    CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_active
                        ON temporal_graph_facts(subject_id, valid_until_ms, retracted_at_ms);
                    CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_source
                        ON temporal_graph_facts(source_episode_id);
                    CREATE TRIGGER IF NOT EXISTS temporal_graph_facts_no_delete
                    BEFORE DELETE ON temporal_graph_facts BEGIN
                        SELECT RAISE(ABORT, 'temporal_graph_facts: hard DELETE forbidden');
                    END;
                    CREATE TRIGGER IF NOT EXISTS temporal_graph_facts_no_update
                    BEFORE UPDATE ON temporal_graph_facts BEGIN
                        SELECT RAISE(ABORT, 'temporal_graph_facts: UPDATE forbidden');
                    END;
                    "#,
                )
                .map_err(StorageError::from)
            })
            .await
            .map_err(Into::into)
    }

    /// Appends a new revision. Existing rows are never updated or deleted.
    /// An empty id is replaced with a generated UUID; the assigned revision is
    /// returned in the value rather than trusting the caller's revision field.
    pub async fn append(
        &self,
        mut fact: TemporalGraphFact,
    ) -> Result<TemporalGraphFact, TemporalGraphError> {
        validate_fact(&fact)?;
        if fact.id.trim().is_empty() {
            fact.id = deterministic_fact_id(&fact);
        }
        let scope_json = canonical_json(&fact.scope)?;
        let provenance_json = canonical_json(&fact.provenance)?;
        let stored = fact.clone();
        let revision = self
            .pool
            .write(move |conn| {
                let tx = conn.transaction()?;
                let revision = append_in_transaction(&tx, &stored, &scope_json, &provenance_json)?;
                tx.commit()?;
                Ok(revision)
            })
            .await?;
        fact.revision = revision;
        Ok(fact)
    }

    /// Compatibility spelling for callers that call an append an insertion.
    pub async fn insert(
        &self,
        fact: TemporalGraphFact,
    ) -> Result<TemporalGraphFact, TemporalGraphError> {
        self.append(fact).await
    }

    pub async fn append_fact(
        &self,
        fact: TemporalGraphFact,
    ) -> Result<TemporalGraphFact, TemporalGraphError> {
        self.append(fact).await
    }

    /// Appends a correction linked to an existing revision by `supersedes_id`.
    /// The logical relation key is immutable across the chain.
    pub async fn supersede(
        &self,
        prior_id: &str,
        mut replacement: TemporalGraphFact,
    ) -> Result<TemporalGraphFact, TemporalGraphError> {
        if prior_id.trim().is_empty() {
            return Err(TemporalGraphError::Invalid("prior_id is empty".into()));
        }
        let relation_key: Option<String> = self.pool.read(|conn| {
            conn.query_row(
                "SELECT relation_key FROM temporal_graph_facts WHERE id = ?1",
                params![prior_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(StorageError::from)
        })?;
        let Some(relation_key) = relation_key else {
            return Err(TemporalGraphError::Invalid(format!(
                "cannot supersede unknown fact `{prior_id}`"
            )));
        };
        if replacement.relation_key.trim().is_empty() {
            replacement.relation_key = relation_key.clone();
        } else if replacement.relation_key != relation_key {
            return Err(TemporalGraphError::Invalid(
                "superseding fact must retain relation_key".into(),
            ));
        }
        replacement.supersedes_id = Some(prior_id.to_owned());
        self.append(replacement).await
    }

    /// Appends a zero-length tombstone revision, preserving prior history.
    /// The lookup and insert execute in one serialized writer transaction.
    pub async fn retract(
        &self,
        relation_key: &str,
        at_ms: i64,
        created_at_ms: i64,
    ) -> Result<TemporalGraphFact, TemporalGraphError> {
        if relation_key.trim().is_empty() {
            return Err(TemporalGraphError::Invalid("relation_key is empty".into()));
        }
        let relation_key = relation_key.to_owned();
        let id = uuid::Uuid::new_v4().to_string();
        self.pool
            .write(move |conn| {
                let tx = conn.transaction()?;
                let prior: Option<(String, String, String, String, String, f64, String)> = tx
                    .query_row(
                        "SELECT subject_id, predicate, object_id, scope_json,
                                provenance_json, confidence, id
                           FROM temporal_graph_facts
                          WHERE relation_key = ?1
                            AND retracted_at_ms IS NULL
                          ORDER BY revision DESC, id ASC
                          LIMIT 1",
                        params![relation_key],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                                row.get(5)?,
                                row.get(6)?,
                            ))
                        },
                    )
                    .optional()?;
                let Some((
                    subject_id,
                    predicate,
                    object_id,
                    scope,
                    provenance,
                    confidence,
                    prior_id,
                )) = prior
                else {
                    return Err(StorageError::Serialization(
                        "cannot retract unknown relation_key".into(),
                    ));
                };
                let tombstone = TemporalGraphFact {
                    id,
                    relation_key,
                    subject_id,
                    predicate,
                    object_id,
                    valid_from_ms: at_ms,
                    valid_until_ms: Some(at_ms),
                    believed_at_ms: at_ms,
                    source_episode_id: None,
                    scope: parse_json_or_quarantine(&scope, "scope_json")
                        .map_err(|error| StorageError::Serialization(error.to_string()))?,
                    provenance: parse_json_or_quarantine(&provenance, "provenance_json")
                        .map_err(|error| StorageError::Serialization(error.to_string()))?,
                    confidence,
                    revision: 0,
                    supersedes_id: Some(prior_id),
                    retracted_at_ms: Some(at_ms),
                    created_at_ms,
                };
                let scope_json = serde_json::to_string(&tombstone.scope)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let provenance_json = serde_json::to_string(&tombstone.provenance)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let revision =
                    append_in_transaction(&tx, &tombstone, &scope_json, &provenance_json)?;
                tx.commit()?;
                let mut result = tombstone;
                result.revision = revision;
                Ok(result)
            })
            .await
            .map_err(Into::into)
    }

    pub async fn retract_relation(
        &self,
        relation_key: &str,
        at_ms: i64,
        created_at_ms: i64,
    ) -> Result<TemporalGraphFact, TemporalGraphError> {
        self.retract(relation_key, at_ms, created_at_ms).await
    }

    /// Returns the effective revision for every matching relation at the
    /// requested valid and belief times. Results are sorted by stable key.
    pub fn query(
        &self,
        query: &TemporalGraphQuery,
    ) -> Result<Vec<TemporalGraphFact>, TemporalGraphError> {
        if query.limit == Some(0) {
            return Ok(Vec::new());
        }
        let belief_as_of = query.belief_as_of_ms.unwrap_or(query.as_of_ms);
        self.pool
            .read(|conn| {
                // Candidate rows are grouped by relation_key in the host
                // language. This is intentional: SQLite's window functions
                // cannot express the tombstone/latest-revision rule as clearly,
                // and this keeps selection deterministic on older SQLite builds.
                let mut statement = conn.prepare(
                    "SELECT id, relation_key, subject_id, predicate, object_id,
                            valid_from_ms, valid_until_ms, believed_at_ms,
                            source_episode_id, scope_json, provenance_json, confidence,
                            revision, supersedes_id, retracted_at_ms, created_at_ms
                       FROM temporal_graph_facts
                      WHERE (?1 IS NULL OR relation_key = ?1)
                        AND believed_at_ms <= ?2
                      ORDER BY relation_key ASC, revision DESC, id ASC",
                )?;
                let rows = statement.query_map(
                    params![query.relation_key.as_deref(), belief_as_of],
                    row_fact,
                )?;
                let mut revisions = Vec::new();
                for row in rows {
                    revisions.push(row?);
                }

                let mut output = Vec::new();
                let mut index = 0;
                while index < revisions.len() {
                    let key = revisions[index].relation_key.clone();
                    let start = index;
                    while index < revisions.len() && revisions[index].relation_key == key {
                        index += 1;
                    }
                    let group = &revisions[start..index];
                    // A tombstone is effective only at/after its own valid and
                    // belief time. A future retraction must not hide an older
                    // revision from an earlier as-of query.
                    if group.first().is_some_and(|fact| {
                        fact.retracted_at_ms.is_some()
                            && fact.valid_from_ms <= query.as_of_ms
                            && fact.believed_at_ms <= belief_as_of
                    }) {
                        continue;
                    }
                    let Some(fact) = group.iter().find(|fact| {
                        fact.retracted_at_ms.is_none()
                            && fact.valid_from_ms <= query.as_of_ms
                            && fact
                                .valid_until_ms
                                .is_none_or(|until| query.as_of_ms < until)
                    }) else {
                        continue;
                    };
                    if query
                        .subject_id
                        .as_ref()
                        .is_some_and(|value| value != &fact.subject_id)
                        || query
                            .predicate
                            .as_ref()
                            .is_some_and(|value| value != &fact.predicate)
                        || query
                            .object_id
                            .as_ref()
                            .is_some_and(|value| value != &fact.object_id)
                        || query
                            .scope
                            .as_ref()
                            .is_some_and(|value| value != &fact.scope)
                    {
                        continue;
                    }
                    output.push(fact.clone());
                    if query.limit.is_some_and(|limit| output.len() >= limit) {
                        break;
                    }
                }
                Ok(output)
            })
            .map_err(Into::into)
    }

    pub fn query_facts(
        &self,
        query: &TemporalGraphQuery,
    ) -> Result<Vec<TemporalGraphFact>, TemporalGraphError> {
        self.query(query)
    }

    pub fn query_as_of(&self, as_of_ms: i64) -> Result<Vec<TemporalGraphFact>, TemporalGraphError> {
        self.query(&TemporalGraphQuery::new(as_of_ms))
    }

    pub fn facts_as_of(
        &self,
        query: &TemporalGraphQuery,
    ) -> Result<Vec<TemporalGraphFact>, TemporalGraphError> {
        self.query(query)
    }

    /// Returns all immutable revisions for one stable relation key.
    pub fn history(
        &self,
        relation_key: &str,
    ) -> Result<Vec<TemporalGraphFact>, TemporalGraphError> {
        self.pool
            .read(|conn| {
                let mut statement = conn.prepare(
                    "SELECT id, relation_key, subject_id, predicate, object_id,
                            valid_from_ms, valid_until_ms, believed_at_ms,
                            source_episode_id, scope_json, provenance_json, confidence,
                            revision, supersedes_id, retracted_at_ms, created_at_ms
                       FROM temporal_graph_facts
                      WHERE relation_key = ?1
                      ORDER BY revision ASC, id ASC",
                )?;
                let rows = statement.query_map(params![relation_key], row_fact)?;
                Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .map_err(Into::into)
    }

    /// Performs a bounded, cycle-safe breadth-first traversal over effective
    /// as-of edges. Query predicates, scope, and belief cut-offs are retained.
    pub fn traverse(
        &self,
        start: &str,
        query: &TemporalGraphQuery,
        budget: TraversalBudget,
    ) -> Result<TraversalResult, TemporalGraphError> {
        if start.trim().is_empty() {
            return Err(TemporalGraphError::Invalid("start is empty".into()));
        }
        if budget.max_nodes == 0 || budget.max_edges == 0 {
            return Ok(TraversalResult {
                nodes: Vec::new(),
                facts: Vec::new(),
                truncated: true,
            });
        }

        let mut nodes = vec![start.to_owned()];
        let mut seen = HashSet::from([start.to_owned()]);
        let mut queue = VecDeque::from([(start.to_owned(), 0usize)]);
        let mut facts = Vec::new();
        let mut truncated = false;

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= budget.max_depth {
                truncated = true;
                continue;
            }
            let node_query = query.clone().subject(node);
            for fact in self.query(&node_query)? {
                let is_new_node = !seen.contains(&fact.object_id);
                if facts.len() >= budget.max_edges {
                    truncated = true;
                    break;
                }
                if is_new_node && nodes.len() >= budget.max_nodes {
                    truncated = true;
                    break;
                }
                facts.push(fact.clone());
                if seen.insert(fact.object_id.clone()) {
                    nodes.push(fact.object_id.clone());
                    queue.push_back((fact.object_id, depth + 1));
                }
            }
            if truncated {
                break;
            }
        }

        Ok(TraversalResult {
            nodes,
            facts,
            truncated,
        })
    }

    pub fn traverse_from(
        &self,
        start: &str,
        query: &TemporalGraphQuery,
        budget: TraversalBudget,
    ) -> Result<TraversalResult, TemporalGraphError> {
        self.traverse(start, query, budget)
    }
}

fn append_in_transaction(
    tx: &Transaction<'_>,
    fact: &TemporalGraphFact,
    scope_json: &str,
    provenance_json: &str,
) -> Result<i64, StorageError> {
    let revision: i64 = tx.query_row(
        "SELECT COALESCE(MAX(revision), -1) + 1
           FROM temporal_graph_facts
          WHERE relation_key = ?1",
        params![fact.relation_key],
        |row| row.get(0),
    )?;

    if let Some(prior_id) = &fact.supersedes_id {
        let prior: Option<(String, i64, Option<String>)> = tx
            .query_row(
                "SELECT relation_key, revision, supersedes_id
                   FROM temporal_graph_facts WHERE id = ?1",
                params![prior_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((prior_key, prior_revision, _)) = prior else {
            return Err(StorageError::Serialization(format!(
                "supersedes_id `{prior_id}` does not exist"
            )));
        };
        if prior_key != fact.relation_key {
            return Err(StorageError::Serialization(
                "supersedes_id belongs to another relation_key".into(),
            ));
        }
        if prior_revision >= revision {
            return Err(StorageError::Serialization(
                "supersedes_id must reference an earlier revision".into(),
            ));
        }
    }

    tx.execute(
        "INSERT INTO temporal_graph_facts
            (id, relation_key, subject_id, predicate, object_id,
             valid_from_ms, valid_until_ms, believed_at_ms, source_episode_id,
             scope_json, provenance_json, confidence, revision, supersedes_id,
             retracted_at_ms, created_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            fact.id,
            fact.relation_key,
            fact.subject_id,
            fact.predicate,
            fact.object_id,
            fact.valid_from_ms,
            fact.valid_until_ms,
            fact.believed_at_ms,
            fact.source_episode_id,
            scope_json,
            provenance_json,
            fact.confidence,
            revision,
            fact.supersedes_id,
            fact.retracted_at_ms,
            fact.created_at_ms,
        ],
    )?;
    Ok(revision)
}

fn deterministic_fact_id(fact: &TemporalGraphFact) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"apeireth.temporal_graph.fact.v1\0");
    for value in [
        fact.relation_key.as_str(),
        fact.subject_id.as_str(),
        fact.predicate.as_str(),
        fact.object_id.as_str(),
        &fact.valid_from_ms.to_string(),
        &fact
            .valid_until_ms
            .map_or_else(String::new, |value| value.to_string()),
        &fact.believed_at_ms.to_string(),
        fact.source_episode_id.as_deref().unwrap_or(""),
        &canonical_json(&fact.scope).unwrap_or_else(|_| "<invalid>".into()),
        &canonical_json(&fact.provenance).unwrap_or_else(|_| "<invalid>".into()),
        &fact.confidence.to_bits().to_string(),
        &fact.revision.to_string(),
        fact.supersedes_id.as_deref().unwrap_or(""),
        &fact
            .retracted_at_ms
            .map_or_else(String::new, |value| value.to_string()),
        &fact.created_at_ms.to_string(),
    ] {
        hasher.update(value.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(value.as_bytes());
        hasher.update(b"\0");
    }
    format!("tg_{}", hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn canonical_json(value: &Value) -> Result<String, TemporalGraphError> {
    serde_json::to_string(value).map_err(TemporalGraphError::from)
}

fn parse_json_or_quarantine(raw: &str, column: &str) -> Result<Value, TemporalGraphError> {
    serde_json::from_str(raw).map_err(|error| {
        TemporalGraphError::Invalid(format!(
            "malformed {column}; row quarantined from visible results: {error}"
        ))
    })
}

fn validate_fact(fact: &TemporalGraphFact) -> Result<(), TemporalGraphError> {
    if fact.relation_key.trim().is_empty()
        || fact.subject_id.trim().is_empty()
        || fact.predicate.trim().is_empty()
        || fact.object_id.trim().is_empty()
    {
        return Err(TemporalGraphError::Invalid(
            "relation_key, subject_id, predicate, and object_id are required".into(),
        ));
    }
    if !fact.confidence.is_finite() || !(0.0..=1.0).contains(&fact.confidence) {
        return Err(TemporalGraphError::Invalid(
            "confidence must be finite and between 0 and 1".into(),
        ));
    }
    match fact.valid_until_ms {
        Some(until) if fact.retracted_at_ms.is_some() && until != fact.valid_from_ms => {
            return Err(TemporalGraphError::Invalid(
                "retraction must have an empty valid interval".into(),
            ));
        }
        Some(until) if fact.retracted_at_ms.is_none() && until <= fact.valid_from_ms => {
            return Err(TemporalGraphError::Invalid(
                "valid_until_ms must be after valid_from_ms".into(),
            ));
        }
        None if fact.retracted_at_ms.is_some() => {
            return Err(TemporalGraphError::Invalid(
                "retraction must have valid_until_ms".into(),
            ));
        }
        _ => {}
    }
    if fact.supersedes_id.as_deref().is_some_and(str::is_empty) {
        return Err(TemporalGraphError::Invalid(
            "supersedes_id cannot be empty".into(),
        ));
    }
    Ok(())
}

fn row_fact(row: &Row<'_>) -> rusqlite::Result<TemporalGraphFact> {
    Ok(TemporalGraphFact {
        id: row.get(0)?,
        relation_key: row.get(1)?,
        subject_id: row.get(2)?,
        predicate: row.get(3)?,
        object_id: row.get(4)?,
        valid_from_ms: row.get(5)?,
        valid_until_ms: row.get(6)?,
        believed_at_ms: row.get(7)?,
        source_episode_id: row.get(8)?,
        scope: json_column(row, 9)?,
        provenance: json_column(row, 10)?,
        confidence: row.get(11)?,
        revision: row.get(12)?,
        supersedes_id: row.get(13)?,
        retracted_at_ms: row.get(14)?,
        created_at_ms: row.get(15)?,
    })
}

fn json_column(row: &Row<'_>, index: usize) -> rusqlite::Result<Value> {
    let raw: String = row.get(index)?;
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> SqliteTemporalGraphStore {
        SqliteTemporalGraphStore::in_memory().await.unwrap()
    }

    fn fact(key: &str, subject: &str, object: &str, at_ms: i64) -> TemporalGraphFact {
        TemporalGraphFact {
            id: String::new(),
            relation_key: key.into(),
            subject_id: subject.into(),
            predicate: "knows".into(),
            object_id: object.into(),
            valid_from_ms: at_ms,
            valid_until_ms: None,
            believed_at_ms: at_ms,
            source_episode_id: Some("episode-1".into()),
            scope: serde_json::json!({"scope": "global"}),
            provenance: serde_json::json!({"kind": "test"}),
            confidence: 0.9,
            revision: 0,
            supersedes_id: None,
            retracted_at_ms: None,
            created_at_ms: at_ms,
        }
    }

    #[tokio::test]
    async fn revisions_are_append_only_and_as_of_is_deterministic() {
        let store = store().await;
        let first = store.append(fact("relation", "a", "b", 10)).await.unwrap();
        let mut replacement = fact("relation", "a", "c", 20);
        replacement.supersedes_id = Some(first.id.clone());
        let second = store.supersede(&first.id, replacement).await.unwrap();
        assert_eq!(second.revision, 1);
        assert_eq!(store.history("relation").unwrap().len(), 2);
        assert_eq!(
            store.query(&TemporalGraphQuery::new(15)).unwrap()[0].object_id,
            "b"
        );
        assert_eq!(
            store.query(&TemporalGraphQuery::new(25)).unwrap()[0].object_id,
            "c"
        );
        let count: i64 = store
            .pool()
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COUNT(*) FROM temporal_graph_facts", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn late_belief_cutoff_and_retraction_are_respected() {
        let store = store().await;
        let first = store.append(fact("relation", "a", "b", 10)).await.unwrap();
        let mut late = fact("relation", "a", "c", 20);
        late.believed_at_ms = 100;
        store.supersede(&first.id, late).await.unwrap();
        assert_eq!(
            store.query(&TemporalGraphQuery::new(25)).unwrap()[0].object_id,
            "b"
        );
        assert_eq!(
            store
                .query(&TemporalGraphQuery::new(25).with_belief_as_of_ms(100))
                .unwrap()[0]
                .object_id,
            "c"
        );
        let tombstone = store.retract("relation", 110, 110).await.unwrap();
        assert!(tombstone.is_retraction());
        assert!(store
            .query(&TemporalGraphQuery::new(110))
            .unwrap()
            .is_empty());
        assert_eq!(store.history("relation").unwrap().len(), 3);
    }

    #[tokio::test]
    async fn traversal_is_cycle_safe_and_bounded() {
        let store = store().await;
        store.append(fact("ab", "a", "b", 1)).await.unwrap();
        store.append(fact("bc", "b", "c", 1)).await.unwrap();
        store.append(fact("ca", "c", "a", 1)).await.unwrap();
        let result = store
            .traverse("a", &TemporalGraphQuery::new(2), TraversalBudget::default())
            .unwrap();
        assert_eq!(result.nodes, vec!["a", "b", "c"]);
        assert_eq!(result.facts.len(), 3);
        assert!(!result.truncated);

        let bounded = store
            .traverse(
                "a",
                &TemporalGraphQuery::new(2),
                TraversalBudget {
                    max_depth: 10,
                    max_nodes: 2,
                    max_edges: 10,
                },
            )
            .unwrap();
        assert_eq!(bounded.nodes.len(), 2);
        assert!(bounded.truncated);
    }

    #[tokio::test]
    async fn wuhan_to_shanghai_supersede_has_current_history_and_as_of_views() {
        let store = store().await;
        let wuhan = store
            .append(fact("city", "traveller", "Wuhan", 100))
            .await
            .unwrap();
        let mut correction = fact("city", "traveller", "Shanghai", 200);
        correction.believed_at_ms = 250;
        let shanghai = store.supersede(&wuhan.id, correction).await.unwrap();

        assert_eq!(shanghai.revision, 1);
        assert_eq!(shanghai.supersedes_id.as_deref(), Some(wuhan.id.as_str()));
        assert_eq!(
            store.history("city").unwrap(),
            vec![wuhan.clone(), shanghai.clone()]
        );
        assert_eq!(
            store.query(&TemporalGraphQuery::new(150)).unwrap()[0].object_id,
            "Wuhan"
        );
        assert_eq!(
            store.query(&TemporalGraphQuery::new(200)).unwrap()[0].object_id,
            "Wuhan",
            "late-believed supersede must not change the default as-of view"
        );
        assert_eq!(
            store
                .query(&TemporalGraphQuery::new(200).with_belief_as_of_ms(250))
                .unwrap()[0]
                .object_id,
            "Shanghai"
        );
        assert_eq!(
            store
                .query(&TemporalGraphQuery::new(250).with_belief_as_of_ms(250))
                .unwrap()[0]
                .object_id,
            "Shanghai"
        );
    }

    #[tokio::test]
    async fn traversal_respects_as_of_and_budgets_without_following_cycles_forever() {
        let store = store().await;
        store
            .append(fact("ab", "Wuhan", "Shanghai", 10))
            .await
            .unwrap();
        store
            .append(fact("bc", "Shanghai", "Tokyo", 10))
            .await
            .unwrap();
        store
            .append(fact("ca", "Tokyo", "Wuhan", 10))
            .await
            .unwrap();
        store
            .append(fact("future", "Wuhan", "Osaka", 100))
            .await
            .unwrap();

        let result = store
            .traverse(
                "Wuhan",
                &TemporalGraphQuery::new(20),
                TraversalBudget {
                    max_depth: 10,
                    max_nodes: 3,
                    max_edges: 3,
                },
            )
            .unwrap();
        assert_eq!(result.nodes, vec!["Wuhan", "Shanghai", "Tokyo"]);
        assert_eq!(result.facts.len(), 3);
        assert!(!result.facts.iter().any(|fact| fact.object_id == "Osaka"));
        assert!(!result.truncated);

        let depth_limited = store
            .traverse(
                "Wuhan",
                &TemporalGraphQuery::new(20),
                TraversalBudget {
                    max_depth: 1,
                    max_nodes: 10,
                    max_edges: 10,
                },
            )
            .unwrap();
        assert_eq!(depth_limited.nodes, vec!["Wuhan", "Shanghai"]);
        assert!(depth_limited.truncated);
    }

    #[tokio::test]
    async fn supersede_rejects_cross_relation_and_unknown_predecessors() {
        let store = store().await;
        let original = store
            .append(fact("city", "traveller", "Wuhan", 1))
            .await
            .unwrap();
        let other = fact("other-city", "traveller", "Beijing", 2);
        assert!(store.supersede("missing", other.clone()).await.is_err());

        let mut wrong_relation = fact("other-city", "traveller", "Shanghai", 3);
        wrong_relation.relation_key = "other-city".into();
        assert!(store.supersede(&original.id, wrong_relation).await.is_err());
    }

    #[tokio::test]
    async fn invalid_rows_are_rejected_and_hard_delete_is_blocked() {
        let store = store().await;
        assert!(store.append(fact("", "a", "b", 1)).await.is_err());
        assert!(store
            .query(&TemporalGraphQuery::new(1).limit(0))
            .unwrap()
            .is_empty());
        let inserted = store.append(fact("r", "a", "b", 1)).await.unwrap();
        let deleted = store
            .pool()
            .write(move |conn| {
                conn.execute(
                    "DELETE FROM temporal_graph_facts WHERE id = ?1",
                    params![inserted.id],
                )?;
                Ok(())
            })
            .await;
        assert!(deleted.is_err());
    }
}
