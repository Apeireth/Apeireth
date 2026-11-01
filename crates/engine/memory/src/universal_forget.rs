use std::sync::Arc;

use crate::{MemoryGovernanceError, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_storage::SqliteConnectionPool;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UniversalForgetOutcome {
    pub episode: bool,
    pub preference: bool,
    pub commitment: bool,
    pub temporal: bool,
    pub persona: bool,
}

/// Durable principal-scoped governance boundary. Legacy rows without an
/// explicit ownership mapping are intentionally untouched.
#[derive(Clone)]
pub struct UniversalForgetFacade {
    pub episodes: Arc<SqliteMemoryStore>,
}

impl UniversalForgetFacade {
    pub fn new(episodes: Arc<SqliteMemoryStore>) -> Self {
        Self { episodes }
    }

    pub fn is_episode_forgotten(&self, id: &str) -> bool {
        self.episodes
            .get_governed(id)
            .ok()
            .flatten()
            .is_some_and(|state| state.status.as_str() == "forgotten")
    }

    pub fn forget_episode(
        &self,
        id: &str,
        reason: Option<&str>,
        revision: i64,
    ) -> Result<UniversalForgetOutcome, MemoryGovernanceError> {
        self.episodes.forget_episode(id, reason, revision)?;
        Ok(UniversalForgetOutcome {
            episode: true,
            ..Default::default()
        })
    }

    /// Mutates all owned append-only/governed stores in one SQLite transaction.
    pub fn forget_principal_on_pool(
        pool: &SqliteConnectionPool,
        principal_id: &str,
        at_ms: i64,
        reason: Option<&str>,
    ) -> Result<UniversalForgetOutcome, String> {
        if principal_id.trim().is_empty() {
            return Err("principal_id is empty".into());
        }
        let principal = principal_id.to_owned();
        let reason = reason.unwrap_or("universal forget").to_owned();
        pool.write_sync(move |conn| {
            let tx = conn.transaction()?;
            let protected_personas: i64 = tx.query_row("SELECT COUNT(*) FROM persona_profile_governance WHERE subject_id=?1 AND protected=1 AND tombstoned_at_ms IS NULL", rusqlite::params![principal], |row| row.get(0))?;
            if protected_personas > 0 {
                return Err(apeireth_storage::StorageError::InvalidConfiguration("principal has protected persona memory".into()));
            }
            if table_exists(&tx, "user_preferences")? {
                tx.execute("DELETE FROM user_preferences WHERE session_id IN (SELECT session_id FROM memory_principal_sessions WHERE principal_id=?1)", rusqlite::params![principal])?;
            }
            tx.execute("INSERT OR IGNORE INTO episode_governance (episode_id,status,revision,reason,forgotten_at) SELECT e.id,'forgotten',0,?1,?2 FROM episodes e JOIN memory_principal_sessions ps ON ps.session_id=e.session_id WHERE ps.principal_id=?3", rusqlite::params![reason, at_ms, principal])?;
            tx.execute("UPDATE commitments SET status='cancelled', retracted_at_ms=?1, updated_at_ms=?1, revision=revision+1 WHERE subject_id=?2 AND status='open' AND retracted_at_ms IS NULL", rusqlite::params![at_ms, principal])?;
            tx.execute("INSERT INTO commitment_events (id,commitment_id,event_type,event_at_ms,payload_json,created_at_ms) SELECT printf('universal-forget:%s:%d',id,?1),id,'universal_forget',?1,json_object('reason',?2),?1 FROM commitments WHERE subject_id=?3 AND retracted_at_ms=?1", rusqlite::params![at_ms, reason, principal])?;
            let relation_rows: Vec<(String,String,String,String)> = {
                let mut stmt = tx.prepare("SELECT relation_key, subject_id, predicate, object_id FROM temporal_graph_facts WHERE subject_id=?1 AND retracted_at_ms IS NULL GROUP BY relation_key")?;
                let rows = stmt
                    .query_map(rusqlite::params![principal], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            };
            for (relation_key, subject, predicate, object) in relation_rows {
                let id = format!("universal-forget:{relation_key}:{at_ms}");
                tx.execute("INSERT OR IGNORE INTO temporal_graph_facts (id,relation_key,subject_id,predicate,object_id,valid_from_ms,valid_until_ms,believed_at_ms,scope_json,provenance_json,confidence,revision,retracted_at_ms,created_at_ms) SELECT ?1,relation_key,subject_id,predicate,object_id,?2,?2,?2,scope_json,json_object('reason',?3),confidence,COALESCE(MAX(revision)+1,0),?2,?2 FROM temporal_graph_facts WHERE relation_key=?4", rusqlite::params![id,at_ms,reason,relation_key])?;
                let _ = (subject,predicate,object);
            }
            tx.execute("UPDATE persona_profile_governance SET tombstoned_at_ms=?1,revision=revision+1,updated_at_ms=?1,reason=?2 WHERE subject_id=?3 AND protected=0 AND tombstoned_at_ms IS NULL", rusqlite::params![at_ms,reason,principal])?;
            tx.commit()?;
            Ok(UniversalForgetOutcome { episode:true, commitment:true, temporal:true, persona:true, preference:false })
        }).map_err(|error| error.to_string())
    }
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        rusqlite::params![table],
        |row| row.get(0),
    )
}
