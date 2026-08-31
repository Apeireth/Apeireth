//! Rolling cross-frontend context ledger (salvage of companion `onering`).
//!
//! Donor behaviour recovered:
//! - Unified timeline keyed by `continuity_id` (SSE / Web / CLI / … share one ledger).
//! - Monotonic `seq` (AUTOINCREMENT) as the sort key, not wall-clock.
//! - Count-based prune: keep the most recent `max_records` rows per continuity.
//! - Ledger is **not** the episode pipeline — table `onering_messages` is a
//!   sidecar. Extract / dream / reflection keep reading `episodes`.
//!
//! Discarded donor shortcuts:
//! - VCP fuzzy-diff timeline insertion (explicitly not absorbed).
//! - Hard DELETE as the only retention (see [`crate::retention`] for the
//!   policy object; this module still prunes the rolling window because a
//!   ledger is a recent-window, not an archive).
//!
//! Persistence reuses [`crate::SqliteMemoryStore::conn`]. No second store.

use rusqlite::params;

use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// Default per-anchor retention (donor DEFAULT_MAX_RECORDS).
pub const DEFAULT_MAX_RECORDS: usize = 200;

/// Legal ledger roles.
pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";

/// One ledger row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// Monotonic sequence (sort key).
    pub seq: i64,
    /// Continuity anchor.
    pub continuity_id: String,
    /// `"user"` | `"assistant"`.
    pub role: String,
    /// Sender label (may be empty).
    pub sender: String,
    /// Frontend origin (`web` / `cli` / `openai-compat` / `proactive` / …).
    pub frontend: String,
    /// Utterance body.
    pub content: String,
    /// Wall-clock epoch seconds (audit; sort uses `seq`).
    pub ts: i64,
}

/// Rolling context ledger bound to one continuity anchor.
pub struct OneRingLedger<'a> {
    store: &'a SqliteMemoryStore,
    continuity: String,
    max_records: usize,
}

impl<'a> OneRingLedger<'a> {
    /// Open a ledger for `continuity`. Empty anchor is rejected.
    pub fn new(store: &'a SqliteMemoryStore, continuity: impl Into<String>) -> MemoryResult<Self> {
        let continuity = continuity.into().trim().to_string();
        if continuity.is_empty() {
            return Err(MemoryError::Invalid(
                "continuity 锚点为空, 无法打开账本".into(),
            ));
        }
        let this = Self {
            store,
            continuity,
            max_records: DEFAULT_MAX_RECORDS,
        };
        this.ensure_table()?;
        Ok(this)
    }

    /// Override the per-anchor retention cap (minimum 1).
    pub fn with_max_records(mut self, n: usize) -> Self {
        self.max_records = n.max(1);
        self
    }

    pub fn continuity(&self) -> &str {
        &self.continuity
    }

    pub fn max_records(&self) -> usize {
        self.max_records
    }

    /// Record an utterance on this ledger's continuity.
    pub fn record(
        &self,
        role: &str,
        sender: Option<&str>,
        frontend: &str,
        content: &str,
        ts: i64,
    ) -> MemoryResult<LedgerEntry> {
        self.record_as(&self.continuity, role, sender, frontend, content, ts)
    }

    /// Record onto an explicit continuity (multi-anchor HTTP header case).
    pub fn record_as(
        &self,
        continuity: &str,
        role: &str,
        sender: Option<&str>,
        frontend: &str,
        content: &str,
        ts: i64,
    ) -> MemoryResult<LedgerEntry> {
        let continuity = continuity.trim();
        if continuity.is_empty() {
            return Err(MemoryError::Invalid("continuity 锚点为空, 拒绝留痕".into()));
        }
        if role != ROLE_USER && role != ROLE_ASSISTANT {
            return Err(MemoryError::Invalid(format!(
                "非法角色 `{role}` (账本只留 user/assistant)"
            )));
        }
        let frontend = frontend.trim();
        if frontend.is_empty() {
            return Err(MemoryError::Invalid(
                "前端来源为空, 拒绝留痕 (OneRing 必须可溯源)".into(),
            ));
        }
        let content = content.trim();
        if content.is_empty() {
            return Err(MemoryError::Invalid("发言内容为空, 拒绝留痕".into()));
        }
        let sender = sender.map(|s| s.trim().to_string()).unwrap_or_default();

        self.ensure_table()?;
        let conn = self.store.conn()?;
        conn.execute(
            "INSERT INTO onering_messages (continuity_id, role, sender, frontend, content, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![continuity, role, sender, frontend, content, ts],
        )?;
        let seq = conn.last_insert_rowid();
        conn.execute(
            "DELETE FROM onering_messages
              WHERE continuity_id = ?1
                AND seq NOT IN (
                    SELECT seq FROM onering_messages
                     WHERE continuity_id = ?1
                     ORDER BY seq DESC LIMIT ?2
                )",
            params![continuity, self.max_records as i64],
        )?;
        Ok(LedgerEntry {
            seq,
            continuity_id: continuity.to_string(),
            role: role.to_string(),
            sender,
            frontend: frontend.to_string(),
            content: content.to_string(),
            ts,
        })
    }

    /// Most recent `limit` rows in seq-ascending (timeline) order. `limit=0` → empty.
    pub fn recent(&self, limit: usize) -> MemoryResult<Vec<LedgerEntry>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.ensure_table()?;
        let conn = self.store.conn()?;
        let mut stmt = conn.prepare(
            "SELECT seq, continuity_id, role, sender, frontend, content, ts
               FROM onering_messages
              WHERE continuity_id = ?1
              ORDER BY seq DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![self.continuity, limit as i64], map_row)?;
        let mut out: Vec<LedgerEntry> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        out.reverse();
        Ok(out)
    }

    /// Row count for this continuity.
    pub fn len(&self) -> MemoryResult<usize> {
        self.ensure_table()?;
        let conn = self.store.conn()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM onering_messages WHERE continuity_id = ?1",
            params![self.continuity],
            |r| r.get(0),
        )?;
        Ok(n.max(0) as usize)
    }

    pub fn is_empty(&self) -> MemoryResult<bool> {
        Ok(self.len()? == 0)
    }

    fn ensure_table(&self) -> MemoryResult<()> {
        ensure_onering_table(self.store)
    }
}

/// Create the sidecar table (idempotent). Shared with continuity migrate.
pub(crate) fn ensure_onering_table(store: &SqliteMemoryStore) -> MemoryResult<()> {
    let conn = store.conn()?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS onering_messages (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            continuity_id TEXT NOT NULL,
            role TEXT NOT NULL,
            sender TEXT NOT NULL DEFAULT '',
            frontend TEXT NOT NULL,
            content TEXT NOT NULL,
            ts INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_onering_continuity_seq
            ON onering_messages(continuity_id, seq);",
    )?;
    Ok(())
}

pub(crate) fn onering_table_exists(conn: &rusqlite::Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'onering_messages'",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<LedgerEntry> {
    Ok(LedgerEntry {
        seq: r.get(0)?,
        continuity_id: r.get(1)?,
        role: r.get(2)?,
        sender: r.get(3)?,
        frontend: r.get(4)?,
        content: r.get(5)?,
        ts: r.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EpisodeStore;

    fn store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().unwrap()
    }

    #[test]
    fn records_and_replays_in_order() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main").unwrap();
        l.record("user", Some("master"), "web", "你好", 10).unwrap();
        l.record("assistant", Some("apeireth"), "web", "主人好", 11)
            .unwrap();
        let evs = l.recent(10).unwrap();
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].role, "user");
        assert_eq!(evs[1].role, "assistant");
        assert!(evs[0].seq < evs[1].seq);
        assert_eq!(evs[0].frontend, "web");
        assert_eq!(l.len().unwrap(), 2);
    }

    #[test]
    fn cross_frontend_same_timeline() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main").unwrap();
        l.record("user", Some("master"), "web", "网页问的", 1)
            .unwrap();
        l.record("user", Some("master"), "openai-compat", "SSE 问的", 2)
            .unwrap();
        l.record("assistant", Some("apeireth"), "proactive", "主动问候", 3)
            .unwrap();
        l.record("user", Some("master"), "cli", "终端问的", 4)
            .unwrap();
        let evs = l.recent(10).unwrap();
        assert_eq!(evs.len(), 4);
        let frontends: Vec<&str> = evs.iter().map(|e| e.frontend.as_str()).collect();
        assert_eq!(frontends, vec!["web", "openai-compat", "proactive", "cli"]);
    }

    #[test]
    fn multi_anchor_isolated() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main").unwrap();
        l.record("user", None, "web", "A 的话", 1).unwrap();
        l.record_as("c-other", "user", None, "web", "B 的话", 2)
            .unwrap();
        assert_eq!(l.len().unwrap(), 1);
        assert_eq!(l.recent(5).unwrap()[0].content, "A 的话");
    }

    #[test]
    fn rejects_invalid_role_content_frontend_anchor() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main").unwrap();
        assert!(l.record("system", None, "web", "x", 1).is_err());
        assert!(l.record("user", None, "web", "   ", 1).is_err());
        assert!(l.record("user", None, "  ", "内容", 1).is_err());
        assert!(l.record_as(" ", "user", None, "web", "内容", 1).is_err());
        assert!(OneRingLedger::new(&st, "  ").is_err());
    }

    #[test]
    fn prunes_to_max_records() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main")
            .unwrap()
            .with_max_records(3);
        for i in 0..10 {
            l.record("user", None, "web", &format!("第{i}条"), i)
                .unwrap();
        }
        assert_eq!(l.len().unwrap(), 3);
        let evs = l.recent(10).unwrap();
        assert_eq!(evs[0].content, "第7条");
        assert_eq!(evs[2].content, "第9条");
    }

    #[test]
    fn recent_limit_zero_is_empty() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main").unwrap();
        l.record("user", None, "web", "x", 1).unwrap();
        assert!(l.recent(0).unwrap().is_empty());
    }

    #[test]
    fn ledger_does_not_pollute_episodes() {
        let st = store();
        let l = OneRingLedger::new(&st, "c-main").unwrap();
        l.record("user", None, "web", "账本条目", 1).unwrap();
        assert_eq!(
            <SqliteMemoryStore as EpisodeStore>::count_by_session(&st, "c-main").unwrap(),
            0,
            "账本与记忆管线分流"
        );
    }
}
