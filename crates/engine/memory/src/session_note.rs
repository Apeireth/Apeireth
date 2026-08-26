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
    /// 有效期起点 (epoch seconds; `None` = 无下界, 一直有效). M5.
    /// `serde(default)`: 旧缓存 JSON (无此字段) 反序列化为 None = 永久有效, 向后兼容.
    #[serde(default)]
    pub valid_from: Option<i64>,
    /// 有效期终点 (epoch seconds; `None` = 永久有效). M5: 存量条目缺省即 None.
    #[serde(default)]
    pub valid_until: Option<i64>,
}

impl NoteRecord {
    /// 从 `apeireth_core::Note` 构造 (核心类型无时间有效性字段 → 缺省永久有效).
    pub fn from_core(n: &Note) -> Self {
        Self {
            id: n.id.clone(),
            timestamp: n.timestamp,
            content: n.content.clone(),
            source_episode_ids: n.source_episode_ids.clone(),
            confidence: n.confidence,
            tags: n.tags.clone(),
            valid_from: None,
            valid_until: None,
        }
    }

    /// 转回 `apeireth_core::Note` (核心类型不含有效性窗口, 此处丢弃两字段;
    /// M5 边界: 不改 apeireth_core, 接口稳定为先).
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

/// Note 时间有效性过滤模式 (M5).
///
/// `None`/缺省 = 永久有效 (向后兼容铁律: 存量条目零迁移).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ValidityFilter {
    /// 不过滤有效性: 永久有效 + 未过期 + 已过期 全返回 (缺省, 保守).
    #[default]
    All,
    /// 只返回在 `as_of` 时刻当前有效的条目
    /// (valid_from IS NULL OR valid_from <= as_of) AND (valid_until IS NULL OR valid_until > as_of).
    CurrentOnly,
}

/// 问法感知: 从查询文本推导有效性过滤模式 (M5, 确定性关键词规则, 0 LLM 0 装).
///
/// 规则 (先判当前类, 命中即 CurrentOnly; 否则历史类命中 → All 含过期; 都不命中 → All):
/// - 当前类词: 现在/当前/目前/如今/眼下/此刻/现阶段/近来 → 只要当前有效条目
/// - 历史类词: 以前/曾经/过去/当时/从前/往昔/昔日/早先 → 含已过期条目
/// 边界: 两类同时出现以当前类优先 (如 "现在和以前比" 主问在现在);
/// 关键词为子串匹配, 接受少量误报 (启发式规则, 升级路径: 词表外置/分词, 见台账 M5).
pub fn validity_from_query_text(text: &str) -> ValidityFilter {
    const CURRENT_MARKERS: &[&str] = &[
        "现在",
        "当前",
        "目前",
        "如今",
        "眼下",
        "此刻",
        "现阶段",
        "近来",
    ];
    // 历史类词 (以前/曾经/过去/当时/从前/往昔/昔日/早先) 无需显式匹配:
    // 其语义 = "含已过期条目", 与缺省 All 一致, 故只对当前类词做判定.
    if CURRENT_MARKERS.iter().any(|m| text.contains(m)) {
        return ValidityFilter::CurrentOnly;
    }
    ValidityFilter::All
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
    /// 时间有效性过滤 (M5; 缺省 All 不过滤, 向后兼容).
    pub validity: ValidityFilter,
    /// 有效性判定参考时刻 (epoch seconds; `None` = 用当前墙钟). M5.
    pub as_of: Option<i64>,
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
    /// 链式: 时间有效性过滤 (M5).
    pub fn validity(mut self, v: ValidityFilter) -> Self {
        self.validity = v;
        self
    }
    /// 链式: 有效性判定参考时刻 (M5, 测试/确定性场景用; 缺省 = 当前墙钟).
    pub fn as_of(mut self, ts: i64) -> Self {
        self.as_of = Some(ts);
        self
    }
    /// 链式: 问法感知 (M5) — 用确定性关键词规则从查询文本推导 validity.
    pub fn with_query_text(mut self, text: &str) -> Self {
        self.validity = validity_from_query_text(text);
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
                "SELECT id, timestamp, content, source_episode_ids_json, confidence, tags_json,
                        valid_from, valid_until
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
                        valid_from: row.get(6)?,
                        valid_until: row.get(7)?,
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
            "SELECT id, timestamp, content, source_episode_ids_json, confidence, tags_json,
                    valid_from, valid_until
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
        // M5 时间有效性过滤: NULL 边界 = 无限制 (存量行两列皆 NULL → 恒通过, 向后兼容).
        // 半开区间 [valid_from, valid_until): 终点用 > 判定, 过期当刻即不再返回.
        if q.validity == ValidityFilter::CurrentOnly {
            let now = q.as_of.unwrap_or_else(now_unix);
            sql.push_str(
                " AND (valid_from IS NULL OR valid_from <= ?) \
                 AND (valid_until IS NULL OR valid_until > ?)",
            );
            args.push(Box::new(now));
            args.push(Box::new(now));
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
                valid_from: row.get(6)?,
                valid_until: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

impl crate::SqliteMemoryStore {
    /// M5 写入侧: 带时间有效性窗口写入 Note (提炼/存入时来源可标注有效期则标).
    ///
    /// - `valid_from`/`valid_until` 均 `None` = 永久有效 (与 `put_note` 等价, 0 装缺省)
    /// - 校验: 两者都给时必须 `valid_from <= valid_until` (单点/反向窗口拒绝)
    /// - 不改 `apeireth_core::Note` (核心类型无此字段, 接口稳定为先)
    pub fn put_note_with_validity(
        &self,
        note: &Note,
        valid_from: Option<i64>,
        valid_until: Option<i64>,
    ) -> MemoryResult<()> {
        Self::validate_note(note)?;
        if let (Some(f), Some(u)) = (valid_from, valid_until) {
            if f > u {
                return Err(MemoryError::Invalid(format!(
                    "note validity window reversed: valid_from {f} > valid_until {u}"
                )));
            }
        }
        let conn = self.conn()?;
        let source_json = serde_json::to_string(&note.source_episode_ids)?;
        let tags_json = serde_json::to_string(&note.tags)?;
        conn.execute(
            "INSERT INTO notes (id, timestamp, content, source_episode_ids_json, confidence, tags_json, valid_from, valid_until)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                note.id,
                note.timestamp,
                note.content,
                source_json,
                note.confidence,
                tags_json,
                valid_from,
                valid_until,
            ],
        )?;
        Ok(())
    }

    /// M5 写入侧: 为已有 Note 设置/更新有效性窗口 (事后得知有效期时用).
    ///
    /// 传 `(None, None)` = 恢复永久有效. note 不存在时返回 `Invalid`.
    pub fn set_note_validity(
        &self,
        id: &str,
        valid_from: Option<i64>,
        valid_until: Option<i64>,
    ) -> MemoryResult<()> {
        if id.trim().is_empty() {
            return Err(MemoryError::Invalid("note id is empty".into()));
        }
        if let (Some(f), Some(u)) = (valid_from, valid_until) {
            if f > u {
                return Err(MemoryError::Invalid(format!(
                    "note validity window reversed: valid_from {f} > valid_until {u}"
                )));
            }
        }
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE notes SET valid_from = ?1, valid_until = ?2 WHERE id = ?3",
            params![valid_from, valid_until, id],
        )?;
        if updated == 0 {
            return Err(MemoryError::Invalid(format!("note not found: {id}")));
        }
        Ok(())
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

    // ===== M5: 通用记忆层时间有效性 (valid_from/valid_until + 问法感知过滤) =====

    const M5_AS_OF: i64 = 2_000_000_000;

    #[test]
    fn m5_default_permanent_and_legacy_row_compat() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        // 常规写入 → 缺省永久有效 (valid_from/valid_until = None).
        let n = make_note("perm", "永久事实");
        <SqliteMemoryStore as NoteStore>::put_note(&store, &n).unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "perm")
            .unwrap()
            .unwrap();
        assert_eq!(got.valid_from, None);
        assert_eq!(got.valid_until, None);
        // 存量兼容: 旧式 6 列 INSERT (不含 valid_* 列) → 读出仍 None, 零迁移负担.
        let conn = store.conn().unwrap();
        conn.execute(
            "INSERT INTO notes (id, timestamp, content, source_episode_ids_json, confidence, tags_json)
             VALUES ('legacy-1', 42, '老条目', '[]', 0.5, '[]')",
            [],
        )
        .unwrap();
        drop(conn);
        let legacy = <SqliteMemoryStore as NoteStore>::get_note(&store, "legacy-1")
            .unwrap()
            .unwrap();
        assert_eq!(legacy.valid_from, None);
        assert_eq!(legacy.valid_until, None);
        // CurrentOnly 过滤下永久条目必须保留.
        let rows = <SqliteMemoryStore as NoteStore>::query(
            &store,
            &NoteQuery::new()
                .validity(ValidityFilter::CurrentOnly)
                .as_of(M5_AS_OF),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        // V3 migration 已应用.
        assert!(store.applied_migrations().unwrap().contains(&3));
    }

    #[test]
    fn m5_current_only_filters_expired_and_future() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        // a) 永久有效 b) 当前有效 c) 已过期 d) 尚未生效
        store
            .put_note_with_validity(&make_note("a-perm", "永久"), None, None)
            .unwrap();
        store
            .put_note_with_validity(
                &make_note("b-active", "当前有效"),
                Some(M5_AS_OF - 100),
                Some(M5_AS_OF + 100),
            )
            .unwrap();
        store
            .put_note_with_validity(
                &make_note("c-expired", "已过期"),
                Some(M5_AS_OF - 500),
                Some(M5_AS_OF - 100),
            )
            .unwrap();
        store
            .put_note_with_validity(
                &make_note("d-future", "尚未生效"),
                Some(M5_AS_OF + 100),
                None,
            )
            .unwrap();
        let current = <SqliteMemoryStore as NoteStore>::query(
            &store,
            &NoteQuery::new()
                .validity(ValidityFilter::CurrentOnly)
                .as_of(M5_AS_OF),
        )
        .unwrap();
        let ids: Vec<&str> = current.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["a-perm", "b-active"]);
        // All 模式: 4 条全返回 (缺省行为不变, 向后兼容).
        let all = <SqliteMemoryStore as NoteStore>::query(&store, &NoteQuery::new()).unwrap();
        assert_eq!(all.len(), 4);
        // 半开区间边界: valid_until == as_of 视为已过期 (> 判定).
        let edge = <SqliteMemoryStore as NoteStore>::query(
            &store,
            &NoteQuery::new()
                .validity(ValidityFilter::CurrentOnly)
                .as_of(M5_AS_OF + 100),
        )
        .unwrap();
        let edge_ids: Vec<&str> = edge.iter().map(|r| r.id.as_str()).collect();
        assert!(edge_ids.contains(&"d-future"));
        assert!(!edge_ids.contains(&"b-active"));
    }

    #[test]
    fn m5_write_side_validation_and_update() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        // 反向窗口拒绝.
        let err = store
            .put_note_with_validity(&make_note("bad", "x"), Some(200), Some(100))
            .unwrap_err();
        assert!(err.to_string().contains("reversed"));
        let err2 = store
            .set_note_validity("bad", Some(200), Some(100))
            .unwrap_err();
        assert!(err2.to_string().contains("reversed"));
        // 正常写入 + 事后更新窗口 + 恢复永久.
        store
            .put_note_with_validity(&make_note("w1", "x"), None, None)
            .unwrap();
        store.set_note_validity("w1", Some(100), Some(200)).unwrap();
        let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "w1")
            .unwrap()
            .unwrap();
        assert_eq!(got.valid_from, Some(100));
        assert_eq!(got.valid_until, Some(200));
        store.set_note_validity("w1", None, None).unwrap();
        let got2 = <SqliteMemoryStore as NoteStore>::get_note(&store, "w1")
            .unwrap()
            .unwrap();
        assert_eq!(got2.valid_from, None);
        assert_eq!(got2.valid_until, None);
        // 不存在的 note → Invalid.
        let err3 = store.set_note_validity("ghost", None, None).unwrap_err();
        assert!(err3.to_string().contains("not found"));
    }

    #[test]
    fn m5_query_text_awareness_rules() {
        // 当前类词 → CurrentOnly.
        for t in [
            "现在的偏好",
            "当前状态",
            "目前的地址",
            "如今住哪",
            "眼下的工作",
        ] {
            assert_eq!(
                validity_from_query_text(t),
                ValidityFilter::CurrentOnly,
                "should be CurrentOnly: {t}"
            );
        }
        // 历史类词 / 中性 / 空 → All (含已过期, 保守缺省).
        for t in ["以前喜欢什么", "曾经说过", "过去的住址", "随便聊聊", ""] {
            assert_eq!(
                validity_from_query_text(t),
                ValidityFilter::All,
                "should be All: {t}"
            );
        }
        // 两类同时出现 → 当前类优先 (主问在现在).
        assert_eq!(
            validity_from_query_text("现在和以前的偏好对比"),
            ValidityFilter::CurrentOnly
        );
        // builder 集成: with_query_text.
        let q = NoteQuery::new().with_query_text("你现在的职业是什么");
        assert_eq!(q.validity, ValidityFilter::CurrentOnly);
        let q2 = NoteQuery::new().with_query_text("你以前养过宠物吗");
        assert_eq!(q2.validity, ValidityFilter::All);
    }

    #[test]
    fn m5_serde_default_for_legacy_json() {
        // 旧缓存 JSON (无 valid_* 字段) 反序列化 → None = 永久有效.
        let legacy_json = r#"{"id":"x","timestamp":1,"content":"c","source_episode_ids":[],"confidence":0.5,"tags":[]}"#;
        let rec: NoteRecord = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(rec.valid_from, None);
        assert_eq!(rec.valid_until, None);
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
