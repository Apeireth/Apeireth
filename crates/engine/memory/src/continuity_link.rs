//! R22 ST-A2.5 — 主体连续性全链路。
//!
//! **8 项承诺**: 全部遵守。**不假装**: continuity_id 仍由 IdentityCard 唯一约束守护。
//! **不修改承诺 (LOCKED)**: 不改 workspace 版本、锁定 StreamKind 或锁定文档。

use crate::{
    onering, EpisodeQuery, EpisodeStore, IdentityCardStore, MemoryError, MemoryResult,
    SqliteMemoryStore,
};
use apeireth_core::kernel::memory::{IdentityCard, Migration};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// Default continuity anchor (single-subject deploy).
pub const DEFAULT_CONTINUITY_ID: &str = "companion-main";

/// Environment variable that overrides the process-level continuity anchor.
pub const CONTINUITY_ENV_VAR: &str = "APEIRETH_CONTINUITY_ID";

/// Lineage prefix for copy-forward migrated episode ids (`mig-{original}`).
pub const MIGRATED_ID_PREFIX: &str = "mig-";

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

/// Trim a raw continuity value; empty → `fallback`. Never returns an empty string
/// if `fallback` itself is non-empty.
pub fn normalize_continuity(raw: &str, fallback: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// `APEIRETH_CONTINUITY_ID` (trim, non-empty) or `default`.
pub fn continuity_id_from_env(default: &str) -> String {
    std::env::var(CONTINUITY_ENV_VAR)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Process-level continuity: env override or [`DEFAULT_CONTINUITY_ID`].
pub fn current_continuity_id() -> String {
    continuity_id_from_env(DEFAULT_CONTINUITY_ID)
}

/// Ensure an IdentityCard exists for `continuity_id` (idempotent). Empty id is rejected.
pub fn ensure_identity(
    store: &SqliteMemoryStore,
    continuity_id: &str,
    carrier: &str,
    birth_time: i64,
) -> MemoryResult<()> {
    let cid = continuity_id.trim();
    if cid.is_empty() {
        return Err(MemoryError::Invalid(
            "continuity_id 为空, 无法登记 IdentityCard".into(),
        ));
    }
    if store.exists(cid)? {
        return Ok(());
    }
    let card = IdentityCard {
        continuity_id: cid.to_string(),
        birth_time,
        carriers: vec![carrier.trim().to_string()],
        migration_history: Vec::new(),
    };
    match store.create(&card) {
        Ok(_) => Ok(()),
        Err(MemoryError::Identity(crate::IdentityConflict::AlreadyExists(_))) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Honest report of one append-only copy-forward subject migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReport {
    pub from: String,
    pub to: String,
    /// Episodes copied forward (originals kept).
    pub episodes_copied: usize,
    /// Already-migrated rows skipped (idempotent INSERT OR IGNORE).
    pub episodes_skipped: usize,
    /// OneRing ledger rows re-keyed (0 = table missing or no matching rows).
    pub ledger_rekeyed: usize,
    pub executed_at: i64,
}

/// Copy-forward migrate episodes from anchor `from` to `to`.
///
/// Append-only safety: originals stay. Copies get id `mig-{original}`,
/// `continuity_id = to`, `session_id = to`, preserved timestamp/role/content.
/// OneRing ledger (non-append-only sidecar) is UPDATE-rekeyed when present.
pub fn migrate_subject(
    store: &SqliteMemoryStore,
    from: &str,
    to: &str,
    executed_at: i64,
) -> MemoryResult<MigrationReport> {
    let from = from.trim().to_string();
    let to = to.trim().to_string();
    if from.is_empty() || to.is_empty() {
        return Err(MemoryError::Invalid(
            "迁移锚点不能为空 (from/to 均须非空)".into(),
        ));
    }
    if from == to {
        return Err(MemoryError::Invalid(format!(
            "迁移锚点相同 (from == to == {from}), 无需迁移"
        )));
    }

    // Query without holding conn (Mutex is not re-entrant).
    let olds = store.query(&EpisodeQuery::new().for_session(&from))?;

    let conn = store.conn()?;
    let mut copied = 0usize;
    let mut skipped = 0usize;
    for ep in &olds {
        let n = conn.execute(
            "INSERT OR IGNORE INTO episodes (id, continuity_id, session_id, timestamp, role, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                format!("{MIGRATED_ID_PREFIX}{}", ep.id),
                to,
                to,
                ep.timestamp,
                ep.role,
                ep.content,
            ],
        )?;
        if n > 0 {
            copied += 1;
        } else {
            skipped += 1;
        }
    }

    let ledger_rekeyed = if onering::onering_table_exists(&conn) {
        conn.execute(
            "UPDATE onering_messages SET continuity_id = ?1 WHERE continuity_id = ?2",
            params![to, from],
        )?
    } else {
        0
    };

    Ok(MigrationReport {
        from,
        to,
        episodes_copied: copied,
        episodes_skipped: skipped,
        ledger_rekeyed,
        executed_at,
    })
}

/// Record a carrier hop on the IdentityCard (same continuity, different carrier).
/// Distinct from [`migrate_subject`] (anchor re-key).
pub fn record_carrier_migration(
    store: &SqliteMemoryStore,
    continuity_id: &str,
    from_carrier: &str,
    to_carrier: &str,
    timestamp: i64,
) -> MemoryResult<()> {
    let m = Migration {
        from_carrier: from_carrier.trim().to_string(),
        to_carrier: to_carrier.trim().to_string(),
        timestamp,
    };
    store.record_migration(continuity_id, &m).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::memory::IdentityCard;
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

    #[test]
    fn normalize_trims_and_falls_back() {
        assert_eq!(normalize_continuity("  c1  ", "fb"), "c1");
        assert_eq!(normalize_continuity("   ", "fb"), "fb");
        assert_eq!(normalize_continuity("", "fb"), "fb");
    }

    #[test]
    fn current_continuity_never_empty() {
        assert!(!current_continuity_id().trim().is_empty());
    }

    #[test]
    fn ensure_identity_is_idempotent() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        ensure_identity(&db, "c-main", "carrier-a", 10).unwrap();
        ensure_identity(&db, "c-main", "carrier-a", 10).unwrap();
        assert!(db.exists("c-main").unwrap());
        assert!(ensure_identity(&db, "  ", "carrier-a", 10).is_err());
    }

    #[test]
    fn migrate_copies_forward_and_keeps_originals() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        db.put_episode(&apeireth_core::kernel::memory::Episode {
            id: "ep-0".into(),
            timestamp: 100,
            role: "user".into(),
            content: "事实一".into(),
            session_id: "me".into(),
        })
        .unwrap();
        db.put_episode(&apeireth_core::kernel::memory::Episode {
            id: "ep-1".into(),
            timestamp: 101,
            role: "assistant".into(),
            content: "事实二".into(),
            session_id: "me".into(),
        })
        .unwrap();
        let r = migrate_subject(&db, "me", "c-main", 200).unwrap();
        assert_eq!(r.episodes_copied, 2);
        assert_eq!(r.episodes_skipped, 0);
        assert_eq!(db.count_by_session("me").unwrap(), 2);
        let news = db.recent_episodes("c-main", 10).unwrap();
        assert_eq!(news.len(), 2);
        assert!(news.iter().all(|e| e.id.starts_with(MIGRATED_ID_PREFIX)));
        assert_eq!(news[0].content, "事实一");
        assert_eq!(news[0].timestamp, 100);
        let by_cont = db
            .query(&EpisodeQuery::new().for_continuity("c-main"))
            .unwrap();
        assert_eq!(by_cont.len(), 2, "迁移副本应写入真实 continuity_id");
    }

    #[test]
    fn migrate_is_idempotent() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        db.put_episode(&apeireth_core::kernel::memory::Episode {
            id: "ep-0".into(),
            timestamp: 100,
            role: "user".into(),
            content: "事实一".into(),
            session_id: "me".into(),
        })
        .unwrap();
        let r1 = migrate_subject(&db, "me", "c-main", 200).unwrap();
        assert_eq!(r1.episodes_copied, 1);
        let r2 = migrate_subject(&db, "me", "c-main", 201).unwrap();
        assert_eq!(r2.episodes_copied, 0);
        assert_eq!(r2.episodes_skipped, 1);
        assert_eq!(db.count_by_session("c-main").unwrap(), 1);
    }

    #[test]
    fn migrate_rejects_empty_or_same_anchor() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        assert!(migrate_subject(&db, "", "c", 1).is_err());
        assert!(migrate_subject(&db, "me", "  ", 1).is_err());
        assert!(migrate_subject(&db, "same", "same", 1).is_err());
    }

    #[test]
    fn migrate_rekeys_onering_ledger_when_present() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        let ledger = crate::onering::OneRingLedger::new(&db, "me")
            .unwrap()
            .with_max_records(10);
        ledger
            .record("user", None, "web", "账本旧锚", 50)
            .unwrap();
        let r = migrate_subject(&db, "me", "c-main", 200).unwrap();
        assert_eq!(r.ledger_rekeyed, 1);
        let moved = crate::onering::OneRingLedger::new(&db, "c-main").unwrap();
        assert_eq!(moved.len().unwrap(), 1);
        assert_eq!(moved.recent(1).unwrap()[0].content, "账本旧锚");
    }

    #[test]
    fn record_carrier_migration_appends_history() {
        let db = SqliteMemoryStore::open_in_memory().unwrap();
        ensure_identity(&db, "c-main", "disk-a", 10).unwrap();
        record_carrier_migration(&db, "c-main", "disk-a", "disk-b", 20).unwrap();
        let card = db.get("c-main").unwrap().unwrap();
        assert!(card.carriers.iter().any(|c| c == "disk-b"));
        assert_eq!(card.migration_history.len(), 1);
        assert_eq!(card.migration_history[0].from_carrier, "disk-a");
        assert_eq!(card.migration_history[0].to_carrier, "disk-b");
    }
}
