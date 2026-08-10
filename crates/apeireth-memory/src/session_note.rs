//! Session + Note 存储.
//!
//! - Session: 可变 (last_active_at 自动更新, closed_at 一次性设置)
//! - Note: 可变 (apeireth_core::Note 文档明确 "可更新/合并/遗忘")
//! - 与 6 历史流 + Episode 的差异: 这两个不是 append-only (按 D2 §5.4)
//!
//! 与 `apeireth_core` 类型关系:
//! - `Session` <-> `SessionRecord` (多一个 `closed_at` 字段)
//! - `Note` <-> `NoteRecord` (几乎一一对应, 走 JSON 缓存便于 SQLite 直接读)

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use apeireth_core::{Note, Session};

use crate::append_only::now_unix;
use crate::{MemoryError, MemoryResult};

/// Session 存储层记录.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRecord {
    /// session ID.
    pub id: String,
    /// 启动时间戳.
    pub started_at: i64,
    /// 最后活跃时间戳.
    pub last_active_at: i64,
    /// 关闭时间戳 (`None` = 仍在进行).
    pub closed_at: Option<i64>,
}

impl SessionRecord {
    /// 从 `apeireth_core::Session` 构造.
    pub fn from_core(s: &Session) -> Self {
        Self {
            id: s.id.clone(),
            started_at: s.started_at,
            last_active_at: s.last_active_at,
            closed_at: None,
        }
    }

    /// 转回 `apeireth_core::Session` (丢弃 `closed_at`).
    pub fn into_core(self) -> Session {
        Session {
            id: self.id,
            started_at: self.started_at,
            last_active_at: self.last_active_at,
        }
    }
}

/// Session 存储 trait.
pub trait SessionStore {
    /// 插入或更新一个 Session (upsert).
    fn upsert_session(&self, session: &Session) -> MemoryResult<()>;
    /// 读取 session.
    fn get_session(&self, id: &str) -> MemoryResult<Option<SessionRecord>>;
    /// 关闭一个 session (一次性设置 `closed_at`).
    fn close_session(&self, id: &str, at: i64) -> MemoryResult<()>;
    /// 列出"未关闭"的 sessions.
    fn list_open_sessions(&self) -> MemoryResult<Vec<SessionRecord>>;
    /// 列出全部 sessions.
    fn list_all_sessions(&self) -> MemoryResult<Vec<SessionRecord>>;
}

impl crate::SqliteMemoryStore {
    fn validate_session(s: &Session) -> MemoryResult<()> {
        if s.id.trim().is_empty() {
            return Err(MemoryError::Invalid("session id is empty".into()));
        }
        Ok(())
    }
}

impl SessionStore for crate::SqliteMemoryStore {
    fn upsert_session(&self, session: &Session) -> MemoryResult<()> {
        Self::validate_session(session)?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO sessions (id, started_at, last_active_at, closed_at)
             VALUES (?1, ?2, ?3, NULL)
             ON CONFLICT(id) DO UPDATE SET
                 last_active_at = excluded.last_active_at",
            params![session.id, session.started_at, session.last_active_at],
        )?;
        Ok(())
    }

    fn get_session(&self, id: &str) -> MemoryResult<Option<SessionRecord>> {
        if id.trim().is_empty() {
            return Err(MemoryError::Invalid("session id is empty".into()));
        }
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT id, started_at, last_active_at, closed_at
                 FROM sessions WHERE id = ?1",
                params![id],
                |row| {
                    Ok(SessionRecord {
                        id: row.get(0)?,
                        started_at: row.get(1)?,
                        last_active_at: row.get(2)?,
                        closed_at: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    fn close_session(&self, id: &str, at: i64) -> MemoryResult<()> {
        if id.trim().is_empty() {
            return Err(MemoryError::Invalid("session id is empty".into()));
        }
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE sessions SET closed_at = ?1
             WHERE id = ?2 AND closed_at IS NULL",
            params![at, id],
        )?;
        if updated == 0 {
            // 可能不存在 / 已关闭
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(MemoryError::Invalid(format!("session `{id}` not found")));
            }
            // 已关闭视为幂等成功.
        }
        Ok(())
    }

    fn list_open_sessions(&self) -> MemoryResult<Vec<SessionRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, started_at, last_active_at, closed_at
             FROM sessions WHERE closed_at IS NULL
             ORDER BY last_active_at DESC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionRecord {
                id: row.get(0)?,
                started_at: row.get(1)?,
                last_active_at: row.get(2)?,
                closed_at: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn list_all_sessions(&self) -> MemoryResult<Vec<SessionRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, started_at, last_active_at, closed_at
             FROM sessions ORDER BY started_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionRecord {
                id: row.get(0)?,
                started_at: row.get(1)?,
                last_active_at: row.get(2)?,
                closed_at: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

/// Note 存储层记录.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteRecord {
    /// note ID.
    pub id: String,
    /// 提炼时间戳.
    pub timestamp: i64,
    /// 知识内容.
    pub content: String,
    /// 来源 episode IDs.
    pub source_episode_ids: Vec<String>,
    /// 置信度 (0.0 - 1.0).
    pub confidence: f64,
    /// 标签.
    pub tags: Vec<String>,
}

impl NoteRecord {
    /// 从 `apeireth_core::Note` 构造.
    pub fn from_core(n: &Note) -> Self {
        Self {
            id: n.id.clone(),
            timestamp: n.timestamp,
            content: n.content.clone(),
            source_episode_ids: n.source_episode_ids.clone(),
            confidence: n.confidence,
            tags: n.tags.clone(),
        }
    }

    /// 转回 `apeireth_core::Note`.
    pub fn into_core(self) -> Note {
        Note {
            id: self.id,
            timestamp: self.timestamp,
            content: self.content,
            source_episode_ids: self.source_episode_ids,
            confidence: self.confidence,
            tags: self.tags,
        }
    }
}

/// Note 复合查询条件.
#[derive(Debug, Clone, Default)]
pub struct NoteQuery {
    /// 最小置信度 (含).
    pub min_confidence: Option<f64>,
    /// 起始时间戳.
    pub since: Option<i64>,
    /// 结束时间戳.
    pub until: Option<i64>,
    /// 单标签过滤.
    pub tag: Option<String>,
    /// 限制返回条数.
    pub limit: Option<usize>,
}

impl NoteQuery {
    /// 构造空查询.
    pub fn new() -> Self {
        Self::default()
    }
    /// 链式: 最小置信度.
    pub fn min_confidence(mut self, c: f64) -> Self {
        self.min_confidence = Some(c);
        self
    }
    /// 链式: 时间窗.
    pub fn in_range(mut self, since: Option<i64>, until: Option<i64>) -> Self {
        self.since = since;
        self.until = until;
        self
    }
    /// 链式: 单标签.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }
    /// 链式: limit.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }
}

/// Note 存储 trait.
pub trait NoteStore {
    /// 写入一个 Note (首次创建).
    fn put_note(&self, note: &Note) -> MemoryResult<()>;
    /// 按 id 读取.
    fn get_note(&self, id: &str) -> MemoryResult<Option<NoteRecord>>;
    /// 更新 content (partial).
    fn update_note_content(&self, id: &str, content: &str) -> MemoryResult<()>;
    /// 更新 confidence (partial).
    fn update_note_confidence(&self, id: &str, confidence: f64) -> MemoryResult<()>;
    /// 替换 tags (full).
    fn replace_note_tags(&self, id: &str, tags: &[String]) -> MemoryResult<()>;
    /// 物理删除一个 Note (Note 允许遗忘, D2 §5.4).
    fn delete_note(&self, id: &str) -> MemoryResult<()>;
    /// 复合条件查询.
    fn query(&self, q: &NoteQuery) -> MemoryResult<Vec<NoteRecord>>;
}

impl crate::SqliteMemoryStore {
    fn validate_note(n: &Note) -> MemoryResult<()> {
        if n.id.trim().is_empty() {
            return Err(MemoryError::Invalid("note id is empty".into()));
        }
        if !(0.0..=1.0).contains(&n.confidence) {
            return Err(MemoryError::Invalid(format!(
                "note confidence must be in [0, 1], got {}",
                n.confidence
            )));
        }
        Ok(())
    }
}

impl NoteStore for crate::SqliteMemoryStore {
    fn put_note(&self, note: &Note) -> MemoryResult<()> {
        Self::validate_note(note)?;
        let conn = self.conn()?;
        let source_json = serde_json::to_string(&note.source_episode_ids)?;
        let tags_json = serde_json::to_string(&note.tags)?;
        conn.execute(
            "INSERT INTO notes (id, timestamp, content, source_episode_ids_json, confidence, tags_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                note.id,
                note.timestamp,
                note.content,
                source_json,
                note.confidence,
                tags_json,
            ],
        )?;
        Ok(())
    }

    fn get_note(&self, id: &str) -> MemoryResult<Option<NoteRecord>> {
        if id.trim().is_empty() {
            return Err(MemoryError::Invalid("note id is empty".into()));
        }
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT id, timestamp, content, source_episode_ids_json, confidence, tags_json
                 FROM notes WHERE id = ?1",
                params![id],
                |row| {
                    let source_json: String = row.get(3)?;
                    let tags_json: String = row.get(5)?;
                    let source: Vec<String> = serde_json::from_str(&source_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                    let tags: Vec<String> = serde_json::from_str(&tags_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                    Ok(NoteRecord {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        content: row.get(2)?,
                        source_episode_ids: source,
                        confidence: row.get(4)?,
                        tags,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    fn update_note_content(&self, id: &str, content: &str) -> MemoryResult<()> {
        let conn = self.conn()?;
        let n = conn.execute(
            "UPDATE notes SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        if n == 0 {
            return Err(MemoryError::Invalid(format!("note `{id}` not found")));
        }
        Ok(())
    }

    fn update_note_confidence(&self, id: &str, confidence: f64) -> MemoryResult<()> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(MemoryError::Invalid(format!(
                "confidence must be in [0, 1], got {confidence}"
            )));
        }
        let conn = self.conn()?;
        let n = conn.execute(
            "UPDATE notes SET confidence = ?1 WHERE id = ?2",
            params![confidence, id],
        )?;
        if n == 0 {
            return Err(MemoryError::Invalid(format!("note `{id}` not found")));
        }
        Ok(())
    }

    fn replace_note_tags(&self, id: &str, tags: &[String]) -> MemoryResult<()> {
        let tags_json = serde_json::to_string(tags)?;
        let conn = self.conn()?;
        let n = conn.execute(
            "UPDATE notes SET tags_json = ?1 WHERE id = ?2",
            params![tags_json, id],
        )?;
        if n == 0 {
            return Err(MemoryError::Invalid(format!("note `{id}` not found")));
        }
        Ok(())
    }

    fn delete_note(&self, id: &str) -> MemoryResult<()> {
        let conn = self.conn()?;
        let n = conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(MemoryError::Invalid(format!("note `{id}` not found")));
        }
        Ok(())
    }

    fn query(&self, q: &NoteQuery) -> MemoryResult<Vec<NoteRecord>> {
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT id, timestamp, content, source_episode_ids_json, confidence, tags_json
             FROM notes WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(c) = q.min_confidence {
            sql.push_str(" AND confidence >= ?");
            args.push(Box::new(c));
        }
        if let Some(s) = q.since {
            sql.push_str(" AND timestamp >= ?");
            args.push(Box::new(s));
        }
        if let Some(u) = q.until {
            sql.push_str(" AND timestamp <= ?");
            args.push(Box::new(u));
        }
        if let Some(tag) = &q.tag {
            sql.push_str(" AND tags_json LIKE ?");
            args.push(Box::new(format!("%\"{}\"%", tag)));
        }
        sql.push_str(" ORDER BY timestamp ASC, id ASC");
        if let Some(n) = q.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let source_json: String = row.get(3)?;
            let tags_json: String = row.get(5)?;
            let source: Vec<String> = serde_json::from_str(&source_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(NoteRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                content: row.get(2)?,
                source_episode_ids: source,
                confidence: row.get(4)?,
                tags,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

/// 公共 helper: 写入 session (供测试或外部直接使用).
pub fn upsert_session(conn: &Connection, session: &Session) -> MemoryResult<()> {
    if session.id.trim().is_empty() {
        return Err(MemoryError::Invalid("session id is empty".into()));
    }
    conn.execute(
        "INSERT INTO sessions (id, started_at, last_active_at, closed_at)
         VALUES (?1, ?2, ?3, NULL)
         ON CONFLICT(id) DO UPDATE SET
             last_active_at = excluded.last_active_at",
        params![session.id, session.started_at, session.last_active_at],
    )?;
    Ok(())
}

/// 公共 helper: 写入 note (供测试或外部直接使用).
pub fn put_note(conn: &Connection, note: &Note) -> MemoryResult<()> {
    if note.id.trim().is_empty() {
        return Err(MemoryError::Invalid("note id is empty".into()));
    }
    let source_json = serde_json::to_string(&note.source_episode_ids)?;
    let tags_json = serde_json::to_string(&note.tags)?;
    conn.execute(
        "INSERT INTO notes (id, timestamp, content, source_episode_ids_json, confidence, tags_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            note.id,
            note.timestamp,
            note.content,
            source_json,
            note.confidence,
            tags_json
        ],
    )?;
    Ok(())
}

/// 公共 helper: 取当前 epoch seconds (供 `created_at` 字段默认值).
pub fn now() -> i64 {
    now_unix()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteMemoryStore;

    fn make_session(id: &str, started: i64) -> Session {
        Session {
            id: id.into(),
            started_at: started,
            last_active_at: started,
        }
    }

    fn make_note(id: &str, content: &str) -> Note {
        Note {
            id: id.into(),
            timestamp: 1_700_000_000,
            content: content.into(),
            source_episode_ids: vec!["ep-1".into()],
            confidence: 0.5,
            tags: vec!["tag-a".into()],
        }
    }

    #[test]
    fn session_upsert_and_get() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as SessionStore>::upsert_session(&store, &make_session("s1", 1000))
            .unwrap();
        let got = <SqliteMemoryStore as SessionStore>::get_session(&store, "s1")
            .unwrap()
            .unwrap();
        assert_eq!(got.started_at, 1000);
        assert!(got.closed_at.is_none());
    }

    #[test]
    fn session_upsert_updates_last_active() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as SessionStore>::upsert_session(&store, &make_session("s1", 1000))
            .unwrap();
        let mut s = make_session("s1", 1000);
        s.last_active_at = 5000;
        <SqliteMemoryStore as SessionStore>::upsert_session(&store, &s).unwrap();
        let got = <SqliteMemoryStore as SessionStore>::get_session(&store, "s1")
            .unwrap()
            .unwrap();
        assert_eq!(got.started_at, 1000);
        assert_eq!(got.last_active_at, 5000);
    }

    #[test]
    fn session_close_idempotent() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as SessionStore>::upsert_session(&store, &make_session("s1", 1000))
            .unwrap();
        <SqliteMemoryStore as SessionStore>::close_session(&store, "s1", 2000).unwrap();
        // 重复关闭不报错
        <SqliteMemoryStore as SessionStore>::close_session(&store, "s1", 3000).unwrap();
        let got = <SqliteMemoryStore as SessionStore>::get_session(&store, "s1")
            .unwrap()
            .unwrap();
        // 第一次的关闭时间
        assert_eq!(got.closed_at, Some(2000));
        let open = <SqliteMemoryStore as SessionStore>::list_open_sessions(&store).unwrap();
        assert!(open.is_empty());
    }

    #[test]
    fn note_put_get_update_delete() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as NoteStore>::put_note(&store, &make_note("n1", "hello")).unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n1")
            .unwrap()
            .unwrap();
        assert_eq!(got.content, "hello");

        <SqliteMemoryStore as NoteStore>::update_note_content(&store, "n1", "world").unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n1")
            .unwrap()
            .unwrap();
        assert_eq!(got.content, "world");

        <SqliteMemoryStore as NoteStore>::update_note_confidence(&store, "n1", 0.9).unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n1")
            .unwrap()
            .unwrap();
        assert!((got.confidence - 0.9).abs() < 1e-9);

        <SqliteMemoryStore as NoteStore>::replace_note_tags(&store, "n1", &["x".into()]).unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n1")
            .unwrap()
            .unwrap();
        assert_eq!(got.tags, vec!["x".to_string()]);

        <SqliteMemoryStore as NoteStore>::delete_note(&store, "n1").unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n1").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn note_query_filters() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        for i in 0..5 {
            let mut n = make_note(&format!("n{i}"), &format!("c{i}"));
            n.timestamp = 1000 + i * 100;
            n.confidence = 0.3 + (i as f64) * 0.1;
            n.tags = if i % 2 == 0 {
                vec!["even".into()]
            } else {
                vec!["odd".into()]
            };
            <SqliteMemoryStore as NoteStore>::put_note(&store, &n).unwrap();
        }
        let q = NoteQuery::new()
            .min_confidence(0.5)
            .in_range(Some(1100), Some(1400))
            .limit(2);
        let rows = <SqliteMemoryStore as NoteStore>::query(&store, &q).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.confidence >= 0.5));

        let q2 = NoteQuery::new().with_tag("even");
        let rows2 = <SqliteMemoryStore as NoteStore>::query(&store, &q2).unwrap();
        assert_eq!(rows2.len(), 3); // n0, n2, n4
    }

    #[test]
    fn note_confidence_validation() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let mut bad = make_note("n", "c");
        bad.confidence = 1.5;
        let err = <SqliteMemoryStore as NoteStore>::put_note(&store, &bad).unwrap_err();
        assert!(err.to_string().contains("confidence"));
    }

    #[test]
    fn session_record_into_core_drops_closed_at() {
        let rec = SessionRecord {
            id: "x".into(),
            started_at: 1,
            last_active_at: 2,
            closed_at: Some(3),
        };
        let core = rec.into_core();
        assert_eq!(core.id, "x");
        assert_eq!(core.started_at, 1);
        assert_eq!(core.last_active_at, 2);
    }
}
