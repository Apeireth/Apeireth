//! R22 ST-A2.5 — 主体连续性全链路。
//!
//! **8 项承诺**: 全部遵守。**不假装**: continuity_id 仍由 IdentityCard 唯一约束守护。
//! **不修改承诺 (LOCKED)**: 不改 workspace 版本、锁定 StreamKind 或锁定文档。

use crate::{IdentityCardStore, MemoryResult, SqliteMemoryStore};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// 跨会话主体连续性的可审计快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuityLink {
    /// 唯一主体 ID。
    pub continuity_id: String,
    /// IdentityCard 当前载体。
    pub carriers: Vec<String>,
    /// 主体诞生时间。
    pub birth_time: i64,
    /// 最近一次会话时间。
    pub last_active_at: i64,
    /// 会话总数。
    pub total_sessions: u64,
    /// 事件总数。
    pub total_episodes: u64,
}

/// 最近会话的可召回引用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRef {
    /// 会话 ID。
    pub session_id: String,
    /// 最近活动时间。
    pub ts: i64,
    /// 该主体在会话中的事件数。
    pub episode_count: u64,
}

/// 确保扩展表存在；它只补充现有 sessions 表缺失的主体关联。
fn ensure_table(store: &SqliteMemoryStore) -> MemoryResult<()> {
    let conn = store.conn()?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS continuity_sessions (continuity_id TEXT NOT NULL, session_id TEXT NOT NULL PRIMARY KEY, recorded_at INTEGER NOT NULL)")?;
    Ok(())
}

/// 从 IdentityCard 和真实 episode/session 记录解析主体连续性。
pub fn resolve_continuity(
    store: &SqliteMemoryStore,
    continuity_id: &str,
) -> MemoryResult<ContinuityLink> {
    ensure_table(store)?;
    let identity = store.get(continuity_id)?.ok_or_else(|| {
        crate::MemoryError::Invalid(format!("continuity_id `{continuity_id}` not found"))
    })?;
    let conn = store.conn()?;
    let total_episodes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM episodes WHERE continuity_id = ?1",
        params![continuity_id],
        |row| row.get(0),
    )?;
    let total_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM continuity_sessions WHERE continuity_id = ?1",
        params![continuity_id],
        |row| row.get(0),
    )?;
    let last_active_at: i64 = conn.query_row(
        "SELECT COALESCE(MAX(recorded_at), ?2) FROM continuity_sessions WHERE continuity_id = ?1",
        params![continuity_id, identity.birth_time],
        |row| row.get(0),
    )?;
    Ok(ContinuityLink {
        continuity_id: identity.continuity_id,
        carriers: identity.carriers,
        birth_time: identity.birth_time,
        last_active_at,
        total_sessions: total_sessions.max(0) as u64,
        total_episodes: total_episodes.max(0) as u64,
    })
}

/// 记录一个跨会话 recall 锚点，并确保 sessions 表有对应会话。
pub fn record_session(
    store: &SqliteMemoryStore,
    continuity_id: &str,
    session_id: &str,
    ts: i64,
) -> MemoryResult<()> {
    if session_id.trim().is_empty() {
        return Err(crate::MemoryError::Invalid("session_id is empty".into()));
    }
    let _ = store.get(continuity_id)?.ok_or_else(|| {
        crate::MemoryError::Invalid(format!("continuity_id `{continuity_id}` not found"))
    })?;
    ensure_table(store)?;
    let conn = store.conn()?;
    conn.execute("INSERT OR IGNORE INTO sessions (id, started_at, last_active_at, closed_at) VALUES (?1, ?2, ?2, NULL)", params![session_id, ts])?;
    conn.execute("INSERT INTO continuity_sessions (continuity_id, session_id, recorded_at) VALUES (?1, ?2, ?3) ON CONFLICT(session_id) DO UPDATE SET continuity_id=excluded.continuity_id, recorded_at=excluded.recorded_at", params![continuity_id, session_id, ts])?;
    conn.execute(
        "UPDATE sessions SET last_active_at = MAX(last_active_at, ?2) WHERE id = ?1",
        params![session_id, ts],
    )?;
    Ok(())
}

/// 召回主体最近的 N 个会话及其 episode 数量。
pub fn recall_recent(
    store: &SqliteMemoryStore,
    continuity_id: &str,
    limit: usize,
) -> MemoryResult<Vec<SessionRef>> {
    ensure_table(store)?;
    let conn = store.conn()?;
    let sql = format!("SELECT cs.session_id, cs.recorded_at, (SELECT COUNT(*) FROM episodes e WHERE e.continuity_id = cs.continuity_id AND e.session_id = cs.session_id) FROM continuity_sessions cs WHERE cs.continuity_id = ?1 ORDER BY cs.recorded_at DESC, cs.session_id DESC{}", if limit > 0 { format!(" LIMIT {limit}") } else { String::new() });
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![continuity_id], |row| {
        Ok(SessionRef {
            session_id: row.get(0)?,
            ts: row.get(1)?,
            episode_count: row.get::<_, i64>(2)?.max(0) as u64,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(crate::MemoryError::Sqlite)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::IdentityCard;
    fn setup() -> (SqliteMemoryStore, String) {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        let id = "continuity-test".to_string();
        db.create(&IdentityCard {
            continuity_id: id.clone(),
            birth_time: 10,
            carriers: vec!["host".into()],
            migration_history: vec![],
        })
        .unwrap();
        (db, id)
    }
    #[test]
    fn resolves_identity() {
        let (db, id) = setup();
        let link = resolve_continuity(&db, &id).unwrap();
        assert_eq!(link.birth_time, 10);
    }
    #[test]
    fn records_and_recalls_session() {
        let (db, id) = setup();
        record_session(&db, &id, "s1", 20).unwrap();
        let rows = recall_recent(&db, &id, 5).unwrap();
        assert_eq!(rows[0].session_id, "s1");
    }
    #[test]
    fn updates_existing_session_without_duplicate() {
        let (db, id) = setup();
        record_session(&db, &id, "s1", 20).unwrap();
        record_session(&db, &id, "s1", 30).unwrap();
        assert_eq!(recall_recent(&db, &id, 10).unwrap().len(), 1);
        assert_eq!(resolve_continuity(&db, &id).unwrap().total_sessions, 1);
    }
    #[test]
    fn limit_is_enforced() {
        let (db, id) = setup();
        for n in 0..3 {
            record_session(&db, &id, &format!("s{n}"), n).unwrap();
        }
        assert_eq!(recall_recent(&db, &id, 2).unwrap().len(), 2);
    }
    #[test]
    fn unknown_identity_is_rejected() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        assert!(resolve_continuity(&db, "missing").is_err());
    }
    #[test]
    fn empty_session_is_rejected() {
        let (db, id) = setup();
        assert!(record_session(&db, &id, " ", 1).is_err());
    }
    #[test]
    fn episodes_are_counted() {
        let (db, id) = setup();
        record_session(&db, &id, "s1", 20).unwrap();
        db.conn().unwrap().execute("INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content) VALUES ('e1', ?1, 's1', 21, 'user', 'hello')", [&id]).unwrap();
        assert_eq!(resolve_continuity(&db, &id).unwrap().total_episodes, 1);
        assert_eq!(recall_recent(&db, &id, 1).unwrap()[0].episode_count, 1);
    }
}
