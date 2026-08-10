//! # SqliteProvider — 嵌入式 SQLite (R20 完整实现, 用 workspace 锁定的 rusqlite 0.32)
//!
//! 1:1 翻译 Golutra `local_sqlite` 商业版 (embedded SQLite memory provider).
//!
//! ## 实现
//!
//! - `Arc<parking_lot::Mutex<rusqlite::Connection>>` — 进程内单连接 (足够 memory use case)
//! - 表 schema: `memory_entries(id PK, content NOT NULL, metadata JSON NOT NULL, created_at INTEGER NOT NULL)`
//! - `capture` — INSERT OR REPLACE (覆写已存在 id)
//! - `query` — 按 id 精确 / content LIKE 子串 + ORDER BY created_at DESC + LIMIT
//! - `clear` — DELETE WHERE id=? / DELETE FROM table
//!
//! ## K-1 强校验
//!
//! - `query` 无过滤: 返 `Err(InvalidQuery)`
//! - `clear(Some(id))` id 不存在: 返 `Err(NotFound)`
//! - SQLite I/O 失败: 返 `Err(BackendIoError)` (K-1 强校验, 不假装成功)
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use rusqlite::{params, Connection};

use super::error::{
    MemoryProviderError, MemoryProviderResult, MEMORY_PROVIDER_ERROR_VARIANT_COUNT,
};
use super::provider_kind::{ProviderKind, PROVIDER_KIND_VARIANT_COUNT};
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 SqliteProvider 结构
// ============================================================================

/// SQLite memory provider (R20 完整实现).
///
/// `Send + Sync` (Arc + Mutex). 单连接足够 memory use case (中等 QPS).
#[derive(Clone)]
pub struct SqliteProvider {
    /// 单连接 (rusqlite::Connection 非 Send/Sync, 用 Mutex 保护).
    conn: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for SqliteProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteProvider").finish_non_exhaustive()
    }
}

impl SqliteProvider {
    /// 打开 / 创建一个 SQLite 文件 (`:memory:` 走 in-memory mode, 不落盘).
    ///
    /// - `path` 存在: 打开并确保 schema
    /// - `path` 不存在: 创建并初始化 schema
    /// - `:memory:`: 内存数据库 (测试用)
    ///
    /// K-1 强校验: SQLite I/O 失败返 `BackendIoError`.
    pub fn open<P: AsRef<Path>>(path: P) -> MemoryProviderResult<Self> {
        let conn = Connection::open(path).map_err(|e| {
            MemoryProviderError::BackendIoError(format!("sqlite open failed: {e}"))
        })?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 内存 SQLite 便捷构造 (测试用).
    pub fn in_memory() -> MemoryProviderResult<Self> {
        Self::open(":memory:")
    }

    /// 初始化 schema (idempotent, IF NOT EXISTS).
    fn init_schema(conn: &Connection) -> MemoryProviderResult<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memory_entries (
                id          TEXT PRIMARY KEY,
                content     TEXT NOT NULL,
                metadata    TEXT NOT NULL DEFAULT '{}',
                created_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memory_created_at
                ON memory_entries(created_at);
            "#,
        )
        .map_err(|e| {
            MemoryProviderError::BackendIoError(format!("sqlite init_schema failed: {e}"))
        })?;
        Ok(())
    }

    /// 当前 entry 数 (sync).
    pub fn len(&self) -> MemoryProviderResult<usize> {
        let guard = self.conn.lock();
        let n: i64 = guard
            .query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))
            .map_err(|e| {
                MemoryProviderError::BackendIoError(format!("sqlite count failed: {e}"))
            })?;
        Ok(n as usize)
    }

    /// 是否空.
    pub fn is_empty(&self) -> MemoryProviderResult<bool> {
        Ok(self.len()? == 0)
    }
}

#[async_trait]
impl MemoryProvider for SqliteProvider {
    async fn capture(&self, entry: MemoryEntry) -> MemoryProviderResult<String> {
        let id = entry.id.clone();
        let metadata_json = serde_json::to_string(&entry.metadata).map_err(|e| {
            MemoryProviderError::SerializationError(format!("metadata encode: {e}"))
        })?;
        let conn = self.conn.clone();
        let id_for_task = id.clone();
        // tokio::task::spawn_blocking: rusqlite 阻塞调用, 不阻塞 runtime
        tokio::task::spawn_blocking(move || -> MemoryProviderResult<()> {
            let guard = conn.lock();
            guard
                .execute(
                    "INSERT OR REPLACE INTO memory_entries (id, content, metadata, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![id_for_task, entry.content, metadata_json, entry.created_at_secs as i64],
                )
                .map_err(|e| {
                    MemoryProviderError::BackendIoError(format!("sqlite insert failed: {e}"))
                })?;
            Ok(())
        })
        .await
        .map_err(|e| {
            MemoryProviderError::BackendIoError(format!("spawn_blocking join failed: {e}"))
        })??;
        Ok(id)
    }

    async fn query(&self, q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // K-1 强校验
        if !q.has_any_filter() {
            return Err(MemoryProviderError::InvalidQuery(
                "at least one of `id` or `content_contains` must be set".to_string(),
            ));
        }
        let limit = q.effective_limit() as i64;

        // 构造 SQL
        let (where_clause, has_id, has_contains) = match (&q.id, &q.content_contains) {
            (Some(_), Some(_)) => ("id = ?1 AND content LIKE ?2", true, true),
            (Some(_), None) => ("id = ?1", true, false),
            (None, Some(_)) => ("content LIKE ?1", false, true),
            (None, None) => unreachable!("guarded by has_any_filter"),
        };
        let sql = format!(
            "SELECT id, content, metadata, created_at FROM memory_entries WHERE {where_clause} ORDER BY created_at DESC LIMIT {limit}"
        );

        let conn = self.conn.clone();
        let q_id = q.id.clone();
        let q_contains = q.content_contains.clone();
        let result: Vec<MemoryEntry> = tokio::task::spawn_blocking(move || -> MemoryProviderResult<Vec<MemoryEntry>> {
            let guard = conn.lock();
            let mut stmt = guard.prepare(&sql).map_err(|e| {
                MemoryProviderError::BackendIoError(format!("sqlite prepare failed: {e}"))
            })?;
            let rows = match (has_id, has_contains) {
                (true, true) => stmt
                    .query_map(
                        params![q_id.unwrap(), format!("%{}%", q_contains.unwrap())],
                        row_to_entry,
                    )
                    .map_err(|e| {
                        MemoryProviderError::BackendIoError(format!(
                            "sqlite query_map failed: {e}"
                        ))
                    })?,
                (true, false) => stmt
                    .query_map(params![q_id.unwrap()], row_to_entry)
                    .map_err(|e| {
                        MemoryProviderError::BackendIoError(format!(
                            "sqlite query_map failed: {e}"
                        ))
                    })?,
                (false, true) => stmt
                    .query_map(params![format!("%{}%", q_contains.unwrap())], row_to_entry)
                    .map_err(|e| {
                        MemoryProviderError::BackendIoError(format!(
                            "sqlite query_map failed: {e}"
                        ))
                    })?,
                _ => unreachable!(),
            };
            let mut out = Vec::new();
            for row in rows {
                let e = row.map_err(|e| {
                    MemoryProviderError::BackendIoError(format!("sqlite row read failed: {e}"))
                })?;
                out.push(e);
            }
            Ok(out)
        })
        .await
        .map_err(|e| {
            MemoryProviderError::BackendIoError(format!("spawn_blocking join failed: {e}"))
        })??;
        Ok(result)
    }

    async fn clear(&self, id: Option<&str>) -> MemoryProviderResult<()> {
        let conn = self.conn.clone();
        let id_owned = id.map(|s| s.to_string());
        let affected: usize = tokio::task::spawn_blocking(move || -> MemoryProviderResult<usize> {
            let guard = conn.lock();
            match &id_owned {
                Some(id) => {
                    let n = guard
                        .execute("DELETE FROM memory_entries WHERE id = ?1", params![id])
                        .map_err(|e| {
                            MemoryProviderError::BackendIoError(format!("sqlite delete failed: {e}"))
                        })?;
                    Ok(n)
                }
                None => {
                    let n = guard.execute("DELETE FROM memory_entries", []).map_err(|e| {
                        MemoryProviderError::BackendIoError(format!("sqlite delete all failed: {e}"))
                    })?;
                    Ok(n)
                }
            }
        })
        .await
        .map_err(|e| {
            MemoryProviderError::BackendIoError(format!("spawn_blocking join failed: {e}"))
        })??;
        if id.is_some() && affected == 0 {
            return Err(MemoryProviderError::NotFound(id.unwrap().to_string()));
        }
        Ok(())
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Sqlite
    }
}

/// row → MemoryEntry (rusqlite 辅助闭包).
fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryEntry> {
    let id: String = row.get(0)?;
    let content: String = row.get(1)?;
    let metadata_json: String = row.get(2)?;
    let created_at: i64 = row.get(3)?;
    let metadata: std::collections::HashMap<String, String> =
        serde_json::from_str(&metadata_json).unwrap_or_default();
    Ok(MemoryEntry {
        id,
        content,
        metadata,
        created_at_secs: created_at as u64,
    })
}

// ============================================================================
// §2 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 守门 #1: 跨子模块 variant 计数守门.
    #[test]
    fn cross_module_variant_counts() {
        assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 8);
        assert_eq!(PROVIDER_KIND_VARIANT_COUNT, 7);
    }

    /// 守门 #2: in_memory 模式空.
    #[test]
    fn in_memory_starts_empty() {
        let p = SqliteProvider::in_memory().unwrap();
        assert!(p.is_empty().unwrap());
        assert_eq!(p.len().unwrap(), 0);
        assert_eq!(p.kind(), ProviderKind::Sqlite);
        assert!(p.is_implemented());
    }

    /// 守门 #3: capture → len = 1.
    #[tokio::test]
    async fn capture_in_memory() {
        let p = SqliteProvider::in_memory().unwrap();
        let id = p
            .capture(MemoryEntry::with_id_and_content("x", "sqlite hello"))
            .await
            .unwrap();
        assert_eq!(id, "x");
        assert_eq!(p.len().unwrap(), 1);
    }

    /// 守门 #4: query 无过滤返 InvalidQuery.
    #[tokio::test]
    async fn query_no_filter_returns_invalid() {
        let p = SqliteProvider::in_memory().unwrap();
        let err = p.query(MemoryQuery::new()).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::InvalidQuery(_)));
    }

    /// 守门 #5: clear 单条不存在返 NotFound.
    #[tokio::test]
    async fn clear_unknown_id_returns_not_found() {
        let p = SqliteProvider::in_memory().unwrap();
        let err = p.clear(Some("nope")).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::NotFound(_)));
    }

    /// 守门 #6: 端到端 — capture / query / clear.
    #[tokio::test]
    async fn end_to_end_capture_query_clear() {
        let p = SqliteProvider::in_memory().unwrap();
        let mut md = HashMap::new();
        md.insert("tag".to_string(), "test".to_string());

        p.capture(MemoryEntry::new("a", "first", md.clone()))
            .await
            .unwrap();
        p.capture(MemoryEntry::new("b", "second", HashMap::new()))
            .await
            .unwrap();
        assert_eq!(p.len().unwrap(), 2);

        // query by id
        let r = p.query(MemoryQuery::by_id("a")).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "first");
        assert_eq!(r[0].metadata.get("tag").map(|s| s.as_str()), Some("test"));

        // query by content_contains
        let r = p
            .query(MemoryQuery::by_content_contains("sec"))
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "b");

        // clear 单条
        p.clear(Some("a")).await.unwrap();
        assert_eq!(p.len().unwrap(), 1);

        // clear 全部
        p.clear(None).await.unwrap();
        assert!(p.is_empty().unwrap());
    }
}
