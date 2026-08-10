//! R22 ST-A2.4 — 6 历史流深度公共 API。
//!
//! **8 项承诺**: 全部遵守。**不假装**: 查询和写入均走真实 SQLite。
//! **不修改承诺 (LOCKED)**: 不改 workspace 版本、StreamKind 或锁定文档。

use std::str::FromStr;
use rusqlite::params;
use crate::{HistoryEntry, MemoryError, MemoryResult, SqliteMemoryStore, StreamKind};

/// 6 条 append-only 历史流的 SQL 深度访问接口。
pub struct StreamDepth;

impl StreamDepth {
    /// 按主体和可选起始时间倒序查询历史条目。
    pub fn query(store: &SqliteMemoryStore, kind: StreamKind, continuity_id: &str, limit: usize, since_ts: Option<i64>) -> MemoryResult<Vec<HistoryEntry>> {
        let conn = store.conn()?;
        let table = kind.table_name();
        let mut sql = format!("SELECT id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at FROM {table} WHERE subject_id = ?1");
        if since_ts.is_some() { sql.push_str(" AND created_at >= ?2"); }
        sql.push_str(" ORDER BY created_at DESC, id DESC");
        if limit > 0 { sql.push_str(&format!(" LIMIT {limit}")); }
        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(since) = since_ts {
            stmt.query_map(params![continuity_id, since], Self::row_to_entry)?
        } else {
            stmt.query_map(params![continuity_id], Self::row_to_entry)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(MemoryError::Sqlite)
    }

    /// 统计单条历史流的条目数。
    pub fn count(store: &SqliteMemoryStore, kind: StreamKind, continuity_id: &str) -> MemoryResult<u64> {
        let conn = store.conn()?;
        let sql = format!("SELECT COUNT(*) FROM {} WHERE subject_id = ?1", kind.table_name());
        let count: i64 = conn.query_row(&sql, params![continuity_id], |row| row.get(0))?;
        Ok(count.max(0) as u64)
    }

    /// 返回六条历史流的计数，顺序与 `StreamKind::ALL` 一致。
    pub fn count_all(store: &SqliteMemoryStore, continuity_id: &str) -> MemoryResult<[u64; 6]> {
        let mut counts = [0; 6];
        for (index, kind) in StreamKind::ALL.iter().enumerate() { counts[index] = Self::count(store, *kind, continuity_id)?; }
        Ok(counts)
    }

    /// 追加一条历史流记录。
    pub fn insert(store: &SqliteMemoryStore, kind: StreamKind, entry: &HistoryEntry) -> MemoryResult<()> {
        let conn = store.conn()?;
        let payload = serde_json::to_string(&entry.payload)?;
        let tags = serde_json::to_string(&entry.tags)?;
        let sql = format!("INSERT INTO {} (id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", kind.table_name());
        conn.execute(&sql, params![entry.id, entry.subject_id, entry.subject_rev, entry.session_id, entry.created_at, payload, entry.source, tags, entry.tombstoned_at])?;
        Ok(())
    }

    /// 按名称解析流类型后查询。
    pub fn query_by_name(store: &SqliteMemoryStore, kind_name: &str, continuity_id: &str, limit: usize, since_ts: Option<i64>) -> MemoryResult<Vec<HistoryEntry>> {
        Self::query(store, StreamKind::from_str(kind_name)?, continuity_id, limit, since_ts)
    }

    fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
        let payload_text: String = row.get(5)?;
        let tags_text: String = row.get(7)?;
        Ok(HistoryEntry {
            id: row.get(0)?, subject_id: row.get(1)?, subject_rev: row.get(2)?, session_id: row.get(3)?, created_at: row.get(4)?,
            payload: serde_json::from_str(&payload_text).map_err(|error| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error)))?,
            source: row.get(6)?, tags: serde_json::from_str(&tags_text).map_err(|error| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error)))?, tombstoned_at: row.get(8)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn store() -> SqliteMemoryStore { SqliteMemoryStore::open_in_memory().unwrap() }
    fn entry(id: &str, subject: &str, time: i64) -> HistoryEntry { HistoryEntry { id: id.into(), subject_id: subject.into(), subject_rev: 0, session_id: Some("session".into()), created_at: time, payload: json!({"id": id}), source: "test".into(), tags: vec!["test".into()], tombstoned_at: None } }
    #[test] fn all_streams_have_distinct_tables() { let names: std::collections::HashSet<_> = StreamKind::ALL.iter().map(|kind| kind.table_name()).collect(); assert_eq!(names.len(), 6); }
    #[test] fn insert_and_count() { let db = store(); StreamDepth::insert(&db, StreamKind::Thought, &entry("a", "c", 1)).unwrap(); assert_eq!(StreamDepth::count(&db, StreamKind::Thought, "c").unwrap(), 1); }
    #[test] fn query_is_newest_first() { let db = store(); StreamDepth::insert(&db, StreamKind::Action, &entry("a", "c", 1)).unwrap(); StreamDepth::insert(&db, StreamKind::Action, &entry("b", "c", 2)).unwrap(); let rows = StreamDepth::query(&db, StreamKind::Action, "c", 10, None).unwrap(); assert_eq!(rows[0].id, "b"); }
    #[test] fn query_filters_since() { let db = store(); StreamDepth::insert(&db, StreamKind::Action, &entry("old", "c", 1)).unwrap(); StreamDepth::insert(&db, StreamKind::Action, &entry("new", "c", 2)).unwrap(); assert_eq!(StreamDepth::query(&db, StreamKind::Action, "c", 10, Some(2)).unwrap().len(), 1); }
    #[test] fn query_respects_limit() { let db = store(); for index in 0..3 { StreamDepth::insert(&db, StreamKind::Action, &entry(&index.to_string(), "c", index)).unwrap(); } assert_eq!(StreamDepth::query(&db, StreamKind::Action, "c", 2, None).unwrap().len(), 2); }
    #[test] fn count_all_tracks_each_kind() { let db = store(); StreamDepth::insert(&db, StreamKind::Thought, &entry("t", "c", 1)).unwrap(); StreamDepth::insert(&db, StreamKind::Reflection, &entry("r", "c", 1)).unwrap(); assert_eq!(StreamDepth::count_all(&db, "c").unwrap(), [1,0,0,0,0,1]); }
    #[test] fn query_by_name_resolves_kind() { let db = store(); StreamDepth::insert(&db, StreamKind::Proposal, &entry("p", "c", 1)).unwrap(); assert_eq!(StreamDepth::query_by_name(&db, "proposal", "c", 1, None).unwrap().len(), 1); }
    #[test] fn other_subjects_are_excluded() { let db = store(); StreamDepth::insert(&db, StreamKind::Thought, &entry("a", "x", 1)).unwrap(); assert!(StreamDepth::query(&db, StreamKind::Thought, "y", 10, None).unwrap().is_empty()); }
}
