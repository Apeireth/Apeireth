//! Append-only Log 公共类型 + 6 历史流 trait 共享设施.
//!
//! 6 流 trait 各自只暴露"追加 + 软删除(tombstone) + 按主体/时间窗查询"三组操作,
//! 严格禁止 `UPDATE` / `DELETE` (由 SQLite trigger 在 schema 层强制).

use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{MemoryError, MemoryResult, StreamKind};

/// Append-only 约束违反.
#[derive(Debug, Error)]
pub enum AppendOnlyError {
    /// 试图 UPDATE / DELETE 一条已落库的流条目.
    #[error("append-only violation on `{table}`: {action}")]
    Violation {
        /// 表名.
        table: &'static str,
        /// 违反的操作.
        action: &'static str,
    },
}

/// 软删除 tombstone 标记 (D2 §5.3 #1: 软删除 = 标记 `tombstoned_at`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tombstone {
    /// 被软删除条目 id.
    pub id: String,
    /// 软删除时间戳.
    pub tombstoned_at: i64,
    /// 软删除原因.
    pub reason: String,
}

/// 一条历史流条目 (D2 §5.3 #2: 每条必须含 subject_id + subject_rev).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    /// 条目 id.
    pub id: String,
    /// 主体 ID.
    pub subject_id: String,
    /// 主体版本号.
    pub subject_rev: i64,
    /// 可选 session 关联.
    pub session_id: Option<String>,
    /// 创建时间 (unix seconds).
    pub created_at: i64,
    /// 自由结构化 payload (JSON 序列化).
    pub payload: serde_json::Value,
    /// 来源 (`ai_generated` / `human_overridden` / `council_synthesized`).
    pub source: String,
    /// 标签.
    pub tags: Vec<String>,
    /// 软删除标记; `None` = 未删除.
    pub tombstoned_at: Option<i64>,
}

/// 6 历史流共享 trait: 6 张表都遵循同一组操作签名.
pub trait HistoryStream {
    /// 流种类.
    const KIND: StreamKind;

    /// 追加一条历史流条目. `subject_id` 必填, 不允许 `None` (D2 §5.3 #2).
    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()>;

    /// 软删除一条历史流条目 (D2 §5.3 #1: 软删除 = 设置 `tombstoned_at`).
    ///
    /// 真正的 `DELETE` 由 trigger 拒绝; 此处通过 `UPDATE tombstoned_at = ?` 表达
    /// "已废弃", 因此仍可被检索 (默认过滤掉 `tombstoned_at IS NOT NULL`).
    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()>;

    /// 按主体 ID + 时间窗查询历史流条目 (默认排除 tombstoned).
    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>>;

    /// 按 session_id 查询 (部分流可能会话绑定, 例如 action / relation).
    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>>;

    /// 取最近 N 条 (审计/摘要用; 时间升序返回, 最新在末尾).
    fn list_recent(
        &self,
        limit: usize,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>>;
}

// ============================================
// 共享 SQL 工具
// ============================================

pub(crate) fn validate_entry(entry: &HistoryEntry) -> MemoryResult<()> {
    if entry.id.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "history entry id must not be empty".into(),
        ));
    }
    if entry.subject_id.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "subject_id is required (D2 §5.3 #2)".into(),
        ));
    }
    if entry.subject_rev < 0 {
        return Err(MemoryError::Invalid(
            "subject_rev must be non-negative".into(),
        ));
    }
    Ok(())
}

pub(crate) fn insert_entry(
    conn: &Connection,
    table: &'static str,
    entry: &HistoryEntry,
) -> MemoryResult<()> {
    validate_entry(entry)?;
    let tags_json = serde_json::to_string(&entry.tags)?;
    let payload_str = serde_json::to_string(&entry.payload)?;
    let sql = format!(
        "INSERT INTO {table} (id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    );
    conn.execute(
        &sql,
        params![
            entry.id,
            entry.subject_id,
            entry.subject_rev,
            entry.session_id,
            entry.created_at,
            payload_str,
            entry.source,
            tags_json,
            entry.tombstoned_at,
        ],
    )?;
    Ok(())
}

pub(crate) fn mark_tombstone(
    conn: &Connection,
    table: &'static str,
    id: &str,
    at: i64,
) -> MemoryResult<()> {
    if id.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "tombstone id must not be empty".into(),
        ));
    }
    // 软删除: 走 UPDATE tombstoned_at, triggers 保护其它列.
    let sql =
        format!("UPDATE {table} SET tombstoned_at = ?1 WHERE id = ?2 AND tombstoned_at IS NULL");
    let updated = conn.execute(&sql, params![at, id])?;
    if updated == 0 {
        // 已经 tombstoned 或 id 不存在.
        let exists = conn
            .query_row(
                &format!("SELECT 1 FROM {table} WHERE id = ?1"),
                params![id],
                |_row| Ok(()),
            )
            .optional()?;
        if exists.is_none() {
            return Err(MemoryError::Invalid(format!(
                "cannot tombstone missing {table} entry: {id}"
            )));
        }
    }
    Ok(())
}

fn decode_row(
    table: &str,
    id: String,
    subject_id: String,
    subject_rev: i64,
    session_id: Option<String>,
    created_at: i64,
    payload: String,
    source: String,
    tags: String,
    tombstoned_at: Option<i64>,
) -> MemoryResult<HistoryEntry> {
    let payload_v: serde_json::Value = serde_json::from_str(&payload).map_err(|e| {
        MemoryError::Invalid(format!("invalid JSON payload in {table} entry {id}: {e}"))
    })?;
    let tags_v: Vec<String> = serde_json::from_str(&tags).map_err(|e| {
        MemoryError::Invalid(format!("invalid JSON tags in {table} entry {id}: {e}"))
    })?;
    Ok(HistoryEntry {
        id,
        subject_id,
        subject_rev,
        session_id,
        created_at,
        payload: payload_v,
        source,
        tags: tags_v,
        tombstoned_at,
    })
}

pub(crate) fn list_for_subject(
    conn: &Connection,
    table: &'static str,
    subject_id: &str,
    since: Option<i64>,
    until: Option<i64>,
    include_tombstoned: bool,
) -> MemoryResult<Vec<HistoryEntry>> {
    let mut sql = format!(
        "SELECT id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at
         FROM {table} WHERE subject_id = ?1"
    );
    if !include_tombstoned {
        sql.push_str(" AND tombstoned_at IS NULL");
    }
    if since.is_some() {
        sql.push_str(" AND created_at >= ?2");
    }
    if until.is_some() {
        let ph = if since.is_some() { "?3" } else { "?2" };
        sql.push_str(&format!(" AND created_at <= {ph}"));
    }
    sql.push_str(" ORDER BY created_at ASC, id ASC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = match (since, until) {
        (Some(s), Some(u)) => stmt.query_map(params![subject_id, s, u], row_mapper(table))?,
        (Some(s), None) => stmt.query_map(params![subject_id, s], row_mapper(table))?,
        (None, Some(u)) => stmt.query_map(params![subject_id, u], row_mapper(table))?,
        (None, None) => stmt.query_map(params![subject_id], row_mapper(table))?,
    };
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub(crate) fn list_for_session(
    conn: &Connection,
    table: &'static str,
    session_id: &str,
    include_tombstoned: bool,
) -> MemoryResult<Vec<HistoryEntry>> {
    let mut sql = format!(
        "SELECT id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at
         FROM {table} WHERE session_id = ?1"
    );
    if !include_tombstoned {
        sql.push_str(" AND tombstoned_at IS NULL");
    }
    sql.push_str(" ORDER BY created_at ASC, id ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![session_id], row_mapper(table))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub(crate) fn list_recent_entries(
    conn: &Connection,
    table: &'static str,
    limit: usize,
    include_tombstoned: bool,
) -> MemoryResult<Vec<HistoryEntry>> {
    let mut sql = format!(
        "SELECT id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at
         FROM {table}"
    );
    if !include_tombstoned {
        sql.push_str(" WHERE tombstoned_at IS NULL");
    }
    sql.push_str(" ORDER BY created_at DESC, id DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit as i64], row_mapper(table))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    out.reverse(); // 时间升序, 最新在末尾 (与其余查询一致)
    Ok(out)
}

fn row_mapper<'a>(
    table: &'a str,
) -> impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> + 'a {
    move |row| {
        let id: String = row.get(0)?;
        let subject_id: String = row.get(1)?;
        let subject_rev: i64 = row.get(2)?;
        let session_id: Option<String> = row.get(3)?;
        let created_at: i64 = row.get(4)?;
        let payload: String = row.get(5)?;
        let source: String = row.get(6)?;
        let tags: String = row.get(7)?;
        let tombstoned_at: Option<i64> = row.get(8)?;
        decode_row(
            table,
            id,
            subject_id,
            subject_rev,
            session_id,
            created_at,
            payload,
            source,
            tags,
            tombstoned_at,
        )
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
    }
}

/// 一键导出所有 6 历史流 (按时间排序, JSON Lines 友好的结构).
pub fn export_all_streams(conn: &Connection) -> MemoryResult<Vec<HistoryEntry>> {
    let mut out = Vec::new();
    for kind in [
        StreamKind::Thought,
        StreamKind::Proposal,
        StreamKind::Action,
        StreamKind::Relation,
        StreamKind::Evolution,
        StreamKind::Reflection,
    ] {
        let table = kind.table_name();
        let mut stmt = conn.prepare(&format!(
            "SELECT id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at
             FROM {table} ORDER BY created_at ASC, id ASC"
        ))?;
        let rows = stmt.query_map([], row_mapper(table))?;
        for r in rows {
            out.push(r?);
        }
    }
    Ok(out)
}

/// 给 streams 模块的 FromStr helper.
pub fn kind_from_str(s: &str) -> MemoryResult<StreamKind> {
    StreamKind::from_str(s)
}

/// 公共 helper: 取当前 epoch seconds.
pub(crate) fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_entry(id: &str, subject: &str, rev: i64, at: i64) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            subject_id: subject.into(),
            subject_rev: rev,
            session_id: Some("sess-1".into()),
            created_at: at,
            payload: json!({"note": "test"}),
            source: "unit".into(),
            tags: vec!["t1".into()],
            tombstoned_at: None,
        }
    }

    #[test]
    fn validate_entry_rejects_empty_fields() {
        let mut e = sample_entry("x", "s", 1, 0);
        e.id = "  ".into();
        assert!(validate_entry(&e).is_err());
        e.id = "x".into();
        e.subject_id = " ".into();
        assert!(validate_entry(&e).is_err());
        e.subject_id = "s".into();
        e.subject_rev = -1;
        assert!(validate_entry(&e).is_err());
    }

    #[test]
    fn export_all_streams_returns_empty_on_fresh_db() {
        // 使用 SqliteMemoryStore (已应用 migration) 而非裸 connection.
        let store = crate::SqliteMemoryStore::open_in_memory().unwrap();
        let conn = store.conn().expect("store conn");
        let out = export_all_streams(&conn).unwrap();
        assert!(out.is_empty());
    }
}
