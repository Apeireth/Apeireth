//! Durable writeback plans in the owning memory database. Source transcripts
//! remain authoritative; this journal never acknowledges an unregistered page.
use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_plugin::experience::{GraphFact, GraphLink, WikiEntry};
use apeireth_storage::{SqliteConnectionPool, StorageError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{MemoryError, MemoryProvenance, MemoryResult, MemoryScope, MemoryTypedCandidate};

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memory_writeback_jobs (
 id TEXT PRIMARY KEY, namespace TEXT NOT NULL, source_session TEXT NOT NULL,
 source_request TEXT NOT NULL, source_hash TEXT NOT NULL, source_json TEXT NOT NULL,
 created_ms INTEGER NOT NULL, plan_json TEXT, next_operation INTEGER NOT NULL DEFAULT 0,
 state TEXT NOT NULL DEFAULT 'pending', attempts INTEGER NOT NULL DEFAULT 0,
 last_error TEXT, updated_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS memory_writeback_pending
 ON memory_writeback_jobs(namespace,state,created_ms,id);
CREATE TABLE IF NOT EXISTS memory_writeback_cursors (
 namespace TEXT PRIMARY KEY, cursor TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS memory_recovery_operation_receipts (
 operation_id TEXT PRIMARY KEY, payload_hash TEXT NOT NULL, applied_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS memory_writeback_dependencies (
 job_id TEXT NOT NULL, episode_id TEXT NOT NULL,
 PRIMARY KEY(job_id,episode_id)
);
CREATE INDEX IF NOT EXISTS memory_writeback_dependency_episode
 ON memory_writeback_dependencies(episode_id);
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WritebackOperation {
    Correction(crate::automatic_correction::AutomaticCorrectionPlan),
    Episode {
        episode: Episode,
        scope: MemoryScope,
        provenance: MemoryProvenance,
    },
    DerivedEpisode {
        episode: Episode,
        scope: MemoryScope,
        provenance: MemoryProvenance,
        fingerprint: String,
        metadata: serde_json::Value,
    },
    Typed {
        candidate: MemoryTypedCandidate,
        at_ms: i64,
        context: serde_json::Value,
    },
    Wiki(WikiEntry),
    Fact(GraphFact),
    Link(GraphLink),
    Association {
        from: String,
        to: String,
        episode_id: String,
    },
    PersonaConsolidation {
        extract: crate::persona_layer::PersonaExtract,
        at_secs: i64,
    },
    IngestGate {
        source: String,
        content: String,
        at_ms: i64,
        skipped: bool,
    },
    PolicySkipped {
        reason: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreparedWritebackPlan {
    pub operations: Vec<WritebackOperation>,
    pub source_episode_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryJob {
    pub id: String,
    pub namespace: String,
    pub source_session: String,
    pub source_request: String,
    pub source_hash: String,
    pub source: serde_json::Value,
    pub created_ms: i64,
    pub plan: Option<PreparedWritebackPlan>,
    pub next_operation: usize,
    pub state: String,
    pub attempts: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoveryStatus {
    pub pending: u64,
    pub done: u64,
    pub suppressed: u64,
    pub attempts: u64,
    pub last_error: Option<String>,
}

pub fn payload_hash(value: &impl Serialize) -> MemoryResult<String> {
    let canonical = serde_json::to_value(value)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical)?)
    ))
}

pub fn operation_id(job_id: &str, index: usize) -> String {
    format!("{job_id}/operation/{index}")
}

/// Call inside the same transaction as the owned domain mutation.
pub fn operation_applied(conn: &Connection, id: &str, hash: &str) -> MemoryResult<bool> {
    assert_operation_allowed(conn, id)?;
    let prior: Option<String> = conn
        .query_row(
            "SELECT payload_hash FROM memory_recovery_operation_receipts WHERE operation_id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    match prior {
        Some(prior) if prior != hash => Err(MemoryError::Invalid(
            "recovery operation identity conflict".into(),
        )),
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub(crate) fn has_table(conn: &Connection, name: &str) -> MemoryResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}

pub fn assert_operation_allowed(conn: &Connection, operation_id: &str) -> MemoryResult<()> {
    if let Some((job, _)) = operation_id.rsplit_once("/operation/") {
        let state: Option<String> = conn
            .query_row(
                "SELECT state FROM memory_writeback_jobs WHERE id=?1",
                [job],
                |r| r.get(0),
            )
            .optional()?;
        if state.is_none() {
            return Err(MemoryError::Invalid(
                "recovery operation has no owning journal in this database".into(),
            ));
        }
        if state.as_deref() == Some("suppressed") {
            return Err(MemoryError::Invalid("recovery source is suppressed".into()));
        }
        let forgotten:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM memory_writeback_dependencies d JOIN episode_governance g ON g.episode_id=d.episode_id WHERE d.job_id=?1 AND g.status='forgotten')",[job],|r|r.get(0))?;
        if forgotten {
            return Err(MemoryError::Invalid(
                "recovery dependency is forgotten".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn assert_episode_source_allowed(
    conn: &Connection,
    episode_id: &str,
) -> MemoryResult<()> {
    if has_table(conn, "episode_governance")? {
        let forgotten:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM episode_governance WHERE episode_id=?1 AND status='forgotten')",[episode_id],|r|r.get(0))?;
        if forgotten {
            return Err(MemoryError::Invalid("memory source is forgotten".into()));
        }
    }
    if has_table(conn, "memory_writeback_jobs")? {
        let suppressed:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM memory_writeback_dependencies d JOIN memory_writeback_jobs j ON j.id=d.job_id WHERE d.episode_id=?1 AND j.state='suppressed' AND COALESCE(j.last_error,'')<>'source_revision_changed')",[episode_id],|r|r.get(0))?;
        if suppressed {
            return Err(MemoryError::Invalid("memory source is suppressed".into()));
        }
        if has_table(conn, "episode_governance")? {
            let forgotten:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM memory_writeback_dependencies source JOIN memory_writeback_dependencies target ON target.job_id=source.job_id JOIN episode_governance g ON g.episode_id=target.episode_id WHERE source.episode_id=?1 AND g.status='forgotten' AND COALESCE(g.reason,'')<>'source_revision_changed')",[episode_id],|r|r.get(0))?;
            if forgotten {
                return Err(MemoryError::Invalid(
                    "memory dependency is forgotten".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn episode_source_timestamp(
    conn: &Connection,
    episode_id: &str,
) -> MemoryResult<Option<i64>> {
    if !has_table(conn, "episodes")? {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            "SELECT timestamp FROM episodes WHERE id=?1",
            [episode_id],
            |r| r.get(0),
        )
        .optional()?)
}

pub(crate) fn assert_metadata_source_allowed(
    conn: &Connection,
    metadata: &serde_json::Value,
) -> MemoryResult<()> {
    let (Some(request), Some(namespace)) = (
        metadata["provenance"]["source_request"].as_str(),
        metadata["scope"]["session_id"].as_str(),
    ) else {
        return Ok(());
    };
    if has_table(conn, "memory_writeback_jobs")? {
        let suppressed:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM memory_writeback_jobs WHERE namespace=?1 AND source_request=?2 AND state='suppressed')",params![namespace,request],|r|r.get(0))?;
        if suppressed {
            return Err(MemoryError::Invalid(
                "memory writeback source is suppressed".into(),
            ));
        }
        let forgotten:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM memory_writeback_jobs j JOIN memory_writeback_dependencies d ON d.job_id=j.id JOIN episode_governance g ON g.episode_id=d.episode_id WHERE j.namespace=?1 AND j.source_request=?2 AND g.status='forgotten')",params![namespace,request],|r|r.get(0))?;
        if forgotten {
            return Err(MemoryError::Invalid(
                "memory writeback dependency is forgotten".into(),
            ));
        }
    }
    Ok(())
}

pub fn record_operation_applied(
    conn: &Connection,
    id: &str,
    hash: &str,
    at_ms: i64,
) -> MemoryResult<()> {
    if !operation_applied(conn, id, hash)? {
        conn.execute(
            "INSERT INTO memory_recovery_operation_receipts VALUES (?1,?2,?3)",
            params![id, hash, at_ms],
        )?;
    }
    Ok(())
}

/// A forgotten source invalidates its entire frozen projection plan. Terminal
/// tombstones survive scans of the retained canonical transcript.
pub fn suppress_source_jobs(conn: &Connection, episode_id: &str) -> MemoryResult<()> {
    conn.execute(
        "UPDATE memory_writeback_jobs SET state='suppressed',last_error=NULL
         WHERE id IN (SELECT job_id FROM memory_writeback_dependencies WHERE episode_id=?1)",
        [episode_id],
    )?;
    Ok(())
}

pub struct SqliteWritebackJournal {
    pool: Arc<SqliteConnectionPool>,
}

impl crate::context_compiler::MemoryEvidenceSource for SqliteWritebackJournal {
    fn evidence_for(
        &self,
        id: &str,
        content: &str,
    ) -> MemoryResult<crate::context_compiler::MemorySourceEvidence> {
        self.pool
            .read(|conn| {
                crate::context_compiler::memory_source_evidence_on_connection(conn, id, content)
                    .map_err(|e| StorageError::Serialization(e.to_string()))
            })
            .map_err(|e| MemoryError::Other(e.to_string()))
    }
}

impl SqliteWritebackJournal {
    /// Recovery sinks must share this official writer pool. Separate pools,
    /// including independently opened handles to one path, are not certified.
    pub fn shares_pool(&self, pool: &SqliteConnectionPool) -> bool {
        std::ptr::eq(self.pool.as_ref(), pool)
    }
    pub fn has_pending_correction(&self, namespace: &str) -> MemoryResult<bool> {
        self.pool.read(|conn|Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM memory_writeback_jobs WHERE namespace=?1 AND state='pending' AND id LIKE 'correction/%')",[namespace],|r|r.get(0))?)).map_err(|e|MemoryError::Other(e.to_string()))
    }
    /// The caller applies the official memory migrations before construction.
    pub fn from_pool(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    pub fn dedup_committed(
        &self,
        namespace: &str,
        fingerprint: &str,
        at_ms: i64,
        window_ms: i64,
    ) -> MemoryResult<bool> {
        self.pool.read(|conn| {
            let prior:Option<i64>=conn.query_row("SELECT last_seen_ms FROM dedup_fingerprints WHERE namespace=?1 AND fingerprint=?2",params![namespace,fingerprint],|r|r.get(0)).optional()?;
            Ok(prior.is_some_and(|p|at_ms.saturating_sub(p)<window_ms))
        }).map_err(|e|MemoryError::Other(e.to_string()))
    }

    /// The caller invokes this only after durable episode acceptance. A failed
    /// fingerprint write leaves the operation pending for an idempotent replay.
    pub fn commit_dedup(&self, namespace: &str, fingerprint: &str, at_ms: i64) -> MemoryResult<()> {
        let namespace = namespace.to_owned();
        let fingerprint = fingerprint.to_owned();
        self.pool.write_sync(move|conn|{conn.execute("INSERT INTO dedup_fingerprints VALUES (?1,?2,?3) ON CONFLICT(namespace,fingerprint) DO UPDATE SET last_seen_ms=MAX(last_seen_ms,excluded.last_seen_ms)",params![namespace,fingerprint,at_ms])?;Ok(())}).map_err(|e|MemoryError::Other(e.to_string()))
    }

    pub fn turn_ordinal(&self, id: &str) -> MemoryResult<u64> {
        self.pool.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM memory_writeback_jobs WHERE namespace=(SELECT namespace FROM memory_writeback_jobs WHERE id=?1) AND rowid<=(SELECT rowid FROM memory_writeback_jobs WHERE id=?1) AND json_extract(source_json,'$.response_id') IS NOT NULL",[id],|r|r.get(0))?)).map_err(|e|MemoryError::Other(e.to_string()))
    }

    pub fn suppress(&self, id: &str) -> MemoryResult<()> {
        let id = id.to_owned();
        self.pool.write_sync(move|conn|{conn.execute("UPDATE memory_writeback_jobs SET state='suppressed',last_error=NULL WHERE id=?1",[id])?;Ok(())}).map_err(|e|MemoryError::Other(e.to_string()))
    }

    pub fn cursor(&self, namespace: &str) -> MemoryResult<Option<String>> {
        self.pool
            .read(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT cursor FROM memory_writeback_cursors WHERE namespace=?1",
                        [namespace],
                        |r| r.get(0),
                    )
                    .optional()?)
            })
            .map_err(|e| MemoryError::Other(e.to_string()))
    }

    /// Registers the complete page and advances its source cursor atomically.
    /// Concurrent consumers must reread the cursor after a conflict.
    pub fn register_page(
        &self,
        namespace: &str,
        expected: Option<String>,
        jobs: &[RecoveryJob],
        next: Option<String>,
    ) -> MemoryResult<()> {
        if jobs.iter().any(|j| j.namespace != namespace) {
            return Err(MemoryError::Invalid(
                "recovery page namespace mismatch".into(),
            ));
        }
        let namespace = namespace.to_owned();
        let jobs = jobs.to_vec();
        self.pool.write_sync(move |conn| {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let current: Option<String> = tx.query_row("SELECT cursor FROM memory_writeback_cursors WHERE namespace=?1", [&namespace], |r| r.get(0)).optional()?;
            if current != expected { return Err(StorageError::Serialization("recovery source cursor conflict".into())); }
            for job in jobs {
                let old: Option<String> = tx.query_row("SELECT source_hash FROM memory_writeback_jobs WHERE id=?1", [&job.id], |r| r.get(0)).optional()?;
                if old.as_ref().is_some_and(|old| old != &job.source_hash) {
                    return Err(StorageError::Serialization("recovery source identity conflict".into()));
                }
                tx.execute(
                    "INSERT OR IGNORE INTO memory_writeback_jobs
                     (id,namespace,source_session,source_request,source_hash,source_json,created_ms,updated_ms)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?7)",
                    params![job.id,job.namespace,job.source_session,job.source_request,job.source_hash,serde_json::to_string(&job.source).map_err(|e| StorageError::Serialization(e.to_string()))?,job.created_ms],
                )?;
            }
            if let Some(next) = next {
                tx.execute("INSERT INTO memory_writeback_cursors VALUES (?1,?2) ON CONFLICT(namespace) DO UPDATE SET cursor=excluded.cursor", params![namespace,next])?;
            }
            tx.commit()?;
            Ok(())
        }).map_err(|e| MemoryError::Other(e.to_string()))
    }

    pub fn pending(&self, namespace: &str, limit: usize) -> MemoryResult<Vec<RecoveryJob>> {
        self.pool.read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id,namespace,source_session,source_request,source_hash,source_json,created_ms,
                 plan_json,next_operation,state,attempts,last_error FROM memory_writeback_jobs
                 WHERE namespace=?1 AND state='pending' ORDER BY updated_ms,created_ms,id LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![namespace,limit.min(128) as i64], |r| {
                let source: String = r.get(5)?;
                let plan: Option<String> = r.get(7)?;
                Ok((RecoveryJob {
                    id:r.get(0)?, namespace:r.get(1)?, source_session:r.get(2)?, source_request:r.get(3)?, source_hash:r.get(4)?,
                    source:serde_json::Value::Null, created_ms:r.get(6)?, plan:None, next_operation:r.get::<_,i64>(8)?.max(0) as usize,
                    state:r.get(9)?, attempts:r.get::<_,i64>(10)?.max(0) as u64, last_error:r.get(11)?,
                },source,plan))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (mut job,source,plan) = row?;
                job.source=serde_json::from_str(&source).map_err(|e| StorageError::Serialization(e.to_string()))?;
                job.plan=plan.map(|p| serde_json::from_str(&p)).transpose().map_err(|e| StorageError::Serialization(e.to_string()))?;
                out.push(job);
            }
            Ok(out)
        }).map_err(|e| MemoryError::Other(e.to_string()))
    }

    pub fn save_plan(&self, job_id: &str, plan: &PreparedWritebackPlan) -> MemoryResult<()> {
        let id = job_id.to_owned();
        let plan = plan.clone();
        self.pool.write_sync(move |conn| {
            let tx=conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let serialized=serde_json::to_string(&plan).map_err(|e| StorageError::Serialization(e.to_string()))?;
            let prior: Option<String> = tx.query_row("SELECT plan_json FROM memory_writeback_jobs WHERE id=?1 AND state='pending'", [&id], |r| r.get(0)).optional()?.flatten();
            if prior.as_ref().is_some_and(|p| p != &serialized) { return Err(StorageError::Serialization("frozen recovery plan conflict".into())); }
            let changed=tx.execute("UPDATE memory_writeback_jobs SET plan_json=?1 WHERE id=?2 AND state='pending' AND plan_json IS NULL", params![serialized,id])?;
            if changed>0 {
                for episode in plan.source_episode_ids {
                    tx.execute("INSERT OR IGNORE INTO memory_writeback_dependencies VALUES (?1,?2)", params![id,episode])?;
                }
            }
            tx.commit()?; Ok(())
        }).map_err(|e| MemoryError::Other(e.to_string()))
    }

    /// Rechecks terminal suppression immediately before every operation.
    pub fn is_pending(&self, id: &str) -> MemoryResult<bool> {
        self.pool
            .read(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT state='pending' FROM memory_writeback_jobs WHERE id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?
                    .unwrap_or(false))
            })
            .map_err(|e| MemoryError::Other(e.to_string()))
    }

    pub fn advance(&self, id: &str, expected: usize, total: usize, at_ms: i64) -> MemoryResult<()> {
        let id = id.to_owned();
        self.pool.write_sync(move |conn| {
            conn.execute("UPDATE memory_writeback_jobs SET next_operation=?1,state=?2,last_error=NULL,updated_ms=?3 WHERE id=?4 AND next_operation=?5 AND state='pending'", params![expected+1,if expected+1>=total {"done"} else {"pending"},at_ms,id,expected])?;
            Ok(())
        }).map_err(|e| MemoryError::Other(e.to_string()))
    }

    pub fn finish_empty(&self, id: &str, at_ms: i64) -> MemoryResult<()> {
        let id = id.to_owned();
        self.pool.write_sync(move |conn| { conn.execute("UPDATE memory_writeback_jobs SET state='done',last_error=NULL,updated_ms=?1 WHERE id=?2 AND state='pending' AND plan_json IS NOT NULL", params![at_ms,id])?; Ok(()) })
            .map_err(|e| MemoryError::Other(e.to_string()))
    }

    pub fn failed(&self, id: &str, phase: &str, at_ms: i64) -> MemoryResult<()> {
        let id = id.to_owned();
        let phase = phase.to_owned();
        self.pool.write_sync(move |conn| { conn.execute("UPDATE memory_writeback_jobs SET attempts=attempts+1,last_error=?1,updated_ms=?2 WHERE id=?3 AND state='pending'", params![phase,at_ms,id])?; Ok(()) })
            .map_err(|e| MemoryError::Other(e.to_string()))
    }

    pub fn status(&self, namespace: &str) -> MemoryResult<RecoveryStatus> {
        self.pool.read(|conn| {
            let mut out=RecoveryStatus::default();
            let mut stmt=conn.prepare("SELECT state,COUNT(*),COALESCE(SUM(attempts),0) FROM memory_writeback_jobs WHERE namespace=?1 GROUP BY state")?;
            let rows=stmt.query_map([namespace], |r| Ok((r.get::<_,String>(0)?,r.get::<_,u64>(1)?,r.get::<_,u64>(2)?)))?;
            for row in rows { let (state,n,attempts)=row?; match state.as_str() {"pending"=>out.pending=n,"done"=>out.done=n,"suppressed"=>out.suppressed=n,_=>{}} out.attempts+=attempts; }
            out.last_error=conn.query_row("SELECT last_error FROM memory_writeback_jobs WHERE namespace=?1 AND last_error IS NOT NULL ORDER BY updated_ms DESC LIMIT 1", [namespace], |r| r.get(0)).optional()?.flatten();
            Ok(out)
        }).map_err(|e| MemoryError::Other(e.to_string()))
    }
}
