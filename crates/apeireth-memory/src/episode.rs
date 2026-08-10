//! Episode 存储 (D2 §3 episode/episode query).
//!
//! - Append-only: 写入即不可变 (BEFORE UPDATE/DELETE trigger 拒绝)
//! - 索引: (session_id, timestamp), (continuity_id, timestamp), (timestamp)
//! - 查询: 按 session_id / time range / continuity_id / limit
//!
//! 公开 API:
//! - `EpisodeStore` trait
//! - `EpisodeQuery` 复合条件

use rusqlite::{params, Connection, OptionalExtension};

use apeireth_core::Episode;

use crate::{MemoryError, MemoryResult};

/// Episode 复合查询条件.
#[derive(Debug, Clone, Default)]
pub struct EpisodeQuery {
    /// 按 session_id 过滤.
    pub session_id: Option<String>,
    /// 按主体 continuity_id 过滤.
    pub continuity_id: Option<String>,
    /// 起始时间戳 (含, epoch seconds).
    pub since: Option<i64>,
    /// 结束时间戳 (含).
    pub until: Option<i64>,
    /// 角色 (user/assistant/system) 过滤.
    pub role: Option<String>,
    /// 最大返回条数.
    pub limit: Option<usize>,
}

impl EpisodeQuery {
    /// 构造空查询.
    pub fn new() -> Self {
        Self::default()
    }

    /// 链式: 限定 session_id.
    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 链式: 限定 continuity_id.
    pub fn for_continuity(mut self, cid: impl Into<String>) -> Self {
        self.continuity_id = Some(cid.into());
        self
    }

    /// 链式: 时间窗 `[since, until]`.
    pub fn in_range(mut self, since: Option<i64>, until: Option<i64>) -> Self {
        self.since = since;
        self.until = until;
        self
    }

    /// 链式: 限定角色.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// 链式: 限制最大返回条数.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }
}

/// Episode 存储 trait.
pub trait EpisodeStore {
    /// 写入一条 Episode (append-only).
    fn put_episode(&self, ep: &Episode) -> MemoryResult<()>;
    /// 按 id 读取.
    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>>;
    /// 检索某 session 的最近 N 条 (按时间升序, 取尾部 N 条).
    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>>;
    /// 复合条件查询.
    fn query(&self, q: &EpisodeQuery) -> MemoryResult<Vec<Episode>>;
    /// 统计某 session 的 Episode 数量.
    fn count_by_session(&self, session_id: &str) -> MemoryResult<i64>;
    /// 列出某 subject 的所有 Episode (跨 session).
    fn list_by_subject(&self, continuity_id: &str) -> MemoryResult<Vec<Episode>>;
}

impl crate::SqliteMemoryStore {
    fn validate_episode(ep: &Episode) -> MemoryResult<()> {
        if ep.id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode id is empty".into()));
        }
        if ep.session_id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode session_id is empty".into()));
        }
        if ep.role.trim().is_empty() {
            return Err(MemoryError::Invalid("episode role is empty".into()));
        }
        Ok(())
    }

    fn row_to_episode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Episode> {
        Ok(Episode {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            session_id: row.get(4)?,
        })
    }
}

impl EpisodeStore for crate::SqliteMemoryStore {
    fn put_episode(&self, ep: &Episode) -> MemoryResult<()> {
        Self::validate_episode(ep)?;
        let conn = self.conn()?;
        // Note: 现有 schema 没有 continuity_id 列在 episodes 表上, 续作兼容.
        // 主体引用通过 session_id 间接表达; 真正跨 session 的主体关联走 6 历史流.
        conn.execute(
            "INSERT OR IGNORE INTO episodes (id, continuity_id, session_id, timestamp, role, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                ep.id,
                "default", // continuity_id 占位, 真正场景由调用方先建 IdentityCard
                ep.session_id,
                ep.timestamp,
                ep.role,
                ep.content,
            ],
        )?;
        Ok(())
    }

    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>> {
        if id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode id is empty".into()));
        }
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT id, timestamp, role, content, session_id
                 FROM episodes WHERE id = ?1",
                params![id],
                Self::row_to_episode,
            )
            .optional()?;
        Ok(row)
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>> {
        if n == 0 {
            return Ok(Vec::new());
        }
        let conn = self.conn()?;
        // 取时间最大的 N 条, 然后按时间升序返回 (符合"对话流"语义).
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, role, content, session_id
             FROM episodes
             WHERE session_id = ?1
             ORDER BY timestamp DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![session_id, n as i64], Self::row_to_episode)?;
        let mut out: Vec<Episode> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        out.reverse();
        Ok(out)
    }

    fn query(&self, q: &EpisodeQuery) -> MemoryResult<Vec<Episode>> {
        let conn = self.conn()?;
        let mut sql =
            String::from("SELECT id, timestamp, role, content, session_id FROM episodes WHERE 1=1");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = &q.session_id {
            sql.push_str(" AND session_id = ?");
            args.push(Box::new(s.clone()));
        }
        if let Some(c) = &q.continuity_id {
            sql.push_str(" AND continuity_id = ?");
            args.push(Box::new(c.clone()));
        }
        if let Some(since) = q.since {
            sql.push_str(" AND timestamp >= ?");
            args.push(Box::new(since));
        }
        if let Some(until) = q.until {
            sql.push_str(" AND timestamp <= ?");
            args.push(Box::new(until));
        }
        if let Some(role) = &q.role {
            sql.push_str(" AND role = ?");
            args.push(Box::new(role.clone()));
        }
        sql.push_str(" ORDER BY timestamp ASC, id ASC");
        if let Some(n) = q.limit {
            sql.push_str(&format!(" LIMIT {}", n));
        }
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_episode)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn count_by_session(&self, session_id: &str) -> MemoryResult<i64> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM episodes WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    fn list_by_subject(&self, continuity_id: &str) -> MemoryResult<Vec<Episode>> {
        if continuity_id.trim().is_empty() {
            return Err(MemoryError::Invalid("continuity_id is empty".into()));
        }
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, role, content, session_id
             FROM episodes WHERE continuity_id = ?1
             ORDER BY timestamp ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![continuity_id], Self::row_to_episode)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

/// 公共 helper: 按角色/时间窗组合查询 (供 `mvp/memory/store.py` 兼容层使用).
pub fn query_recent(conn: &Connection, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>> {
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, role, content, session_id
         FROM episodes WHERE session_id = ?1
         ORDER BY timestamp DESC, id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![session_id, n as i64], |row| {
        Ok(Episode {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            session_id: row.get(4)?,
        })
    })?;
    let mut out: Vec<Episode> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    out.reverse();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteMemoryStore;

    fn make_episode(id: &str, session: &str, ts: i64, role: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: ts,
            role: role.into(),
            content: format!("content of {id}"),
            session_id: session.into(),
        }
    }

    #[test]
    fn put_and_get_roundtrip() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let ep = make_episode("ep-1", "s-1", 1000, "user");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();
        let got = <SqliteMemoryStore as EpisodeStore>::get_episode(&store, "ep-1")
            .unwrap()
            .unwrap();
        assert_eq!(got.id, "ep-1");
        assert_eq!(got.content, "content of ep-1");
    }

    #[test]
    fn append_only_blocks_update_and_delete() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let ep = make_episode("ep-a", "s-a", 1, "user");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();
        let conn = store.conn().unwrap();
        let upd = conn
            .execute("UPDATE episodes SET content='x' WHERE id='ep-a'", [])
            .unwrap_err();
        assert!(upd.to_string().contains("append-only"), "got {upd}");
        let del = conn
            .execute("DELETE FROM episodes WHERE id='ep-a'", [])
            .unwrap_err();
        assert!(del.to_string().contains("append-only"), "got {del}");
    }

    #[test]
    fn recent_episodes_returns_in_chronological_order() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        // 故意乱序插入
        for (id, ts) in [("e3", 3000_i64), ("e1", 1000), ("e2", 2000)] {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &store,
                &make_episode(id, "sess-A", ts, "user"),
            )
            .unwrap();
        }
        let recent =
            <SqliteMemoryStore as EpisodeStore>::recent_episodes(&store, "sess-A", 3).unwrap();
        assert_eq!(
            recent.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["e1", "e2", "e3"]
        );
    }

    #[test]
    fn query_combines_filters() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "sess-X", 100, "user"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e2", "sess-X", 200, "assistant"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e3", "sess-Y", 300, "user"),
        )
        .unwrap();
        let q = EpisodeQuery::new()
            .for_session("sess-X")
            .in_range(Some(150), Some(250));
        let rows = <SqliteMemoryStore as EpisodeStore>::query(&store, &q).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "e2");
    }

    #[test]
    fn count_by_session_reflects_inserts() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        for i in 0..5 {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &store,
                &make_episode(&format!("e{i}"), "sess-C", 100 + i, "user"),
            )
            .unwrap();
        }
        let c = <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, "sess-C").unwrap();
        assert_eq!(c, 5);
    }

    #[test]
    fn list_by_subject_returns_all_timestamps() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "sess-1", 1, "user"),
        )
        .unwrap();
        let rows = <SqliteMemoryStore as EpisodeStore>::list_by_subject(&store, "default").unwrap();
        assert!(rows.len() >= 1);
    }

    #[test]
    fn empty_validation_errors() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let mut bad = make_episode("e1", "sess-1", 1, "user");
        bad.id = "  ".into();
        let err = <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &bad).unwrap_err();
        assert!(err.to_string().contains("id is empty"));
    }
}
