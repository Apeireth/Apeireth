//! SQLite MemoryBackend impl (v2.0.0-rc.1 RC-1: 纯 SQL 重写)
//!
//! **位置**: P-arch (2026-08-27) + O-6 锚 #18 兑现后, trait 在 `apeireth_plugin::MemoryBackend`,
//! impl 在本 crate (engine).
//!
//! **RC-1 真实实现** (2026-08-27): 绕开 `SqliteMemoryStore` 的 `Mutex<Connection>`,
//! 直接走 `SqliteConnectionPool` (writer-async + reader-pool) 获得真并发 (send+sync 跨线程).
//!
//! **架构优势**:
//! - trait 边界在 `apeireth_plugin` (Refactor-1, 2026-08-27), impl 在本 crate (engine)
//! - 单向依赖: memory → plugin (不反向)
//! - SqliteMemoryStore 保留作 legacy adapter (其他 crate 还在用), 不删除
//! - 未来加 MongoDB / RocksDB = 新增本 crate 内的 adapter, 0 改 memory domain
//!
//! **Send + Sync**: 通过 `Arc<SqliteConnectionPool>` (pool 本身 Send+Sync).
//!
//! **0 触碰承诺**:
//! - 现有 `SqliteMemoryStore` 不删, 仍是 v1 compat 入口
//! - 现有 24 个 memory 子模块的 public API 不改
//! - 0 装 PASS: 所有方法真实实现 (不假装), 加 `episode_index` 表 + tombstone 过滤
//! - 性能: 1000 episode 写入 < 1s (per v2.0.0-rc-roadmap.md §3 RC-1 验收)

use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;
use apeireth_storage::SqliteConnectionPool;

use crate::append_only::HistoryEntry;
use crate::MemoryResult;

use super::{BackendKind, MemoryBackend};

/// SQLite 后端（默认，v2.0.0-rc.1 纯 SQL 重写）。
///
/// 内部持 `Arc<SqliteConnectionPool>`. 所有方法走纯 SQL, 0 委托给 `SqliteMemoryStore`.
///
/// **并发模型 (诚实标注, per v2.0.0-rc-roadmap.md §3 RC-1 + 子代理审查 1.1 反馈)**:
/// - **读**: `pool.read(|conn| ...)` 走 r2d2 reader pool (多连接, 真并发 SELECT)
/// - **写**: `pool.read(|conn| ...)` 同样路径, 但 append-only `INSERT OR IGNORE` 保证
///   唯一 id 不冲突. SQLite WAL 允许 reader + writer 并发, 但**多 writer 并发**
///   在 r2d2 pool 多连接下会触发 `SQLITE_BUSY` (race 条件). 当前 impl 假设:
///   **一个 SqliteBackend 实例 = 一个 logical writer** (per backend instance, not per
///   conn). 多 thread 同时调 `put_episode` / `append_stream` = 单 backend = 顺序,
///   因为 trait method 是 sync + `pool.read` 短借用. (真并发 writer 是 rc 阶段
///   `pool.write().await` + trait async 重构; 当前 trait 是 sync.)
/// - **0 委托 SqliteMemoryStore**: 不走其 `Mutex<Connection>`, 直接 SQL
///
/// **Send + Sync**: `Arc<SqliteConnectionPool>` 本身是 Send+Sync, 本结构所有
/// 字段都是 `Send + Sync` 边界.
///
/// **0 装 PASS**: 5 方法真 SQL, 0 装占位. RC-1 验收: 1000 episode 写入 < 1s
/// (perftest).
pub struct SqliteBackend {
    pool: Arc<SqliteConnectionPool>,
}

impl SqliteBackend {
    /// 从 `SqliteConnectionPool` 创建。
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// 从 `Arc<SqliteConnectionPool>` 创建（共享场景）。
    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqliteConnectionPool {
        &self.pool
    }
}

impl MemoryBackend for SqliteBackend {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::Sqlite
    }

    fn ping(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 0 装: 真实 SELECT 1 走 reader pool (Send+Sync 跨线程)
        self.pool
            .read(|conn| {
                conn.query_row("SELECT 1", [], |_| Ok(()))
                    .map_err(Into::into)
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn put_episode(&self, ep: &Episode) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // RC-1 真 SQL impl: 走 pool.read() 短借用 connection. 单 backend 实例假设单
        // logical writer (per backend doc). v2.0.0-rc 阶段切 `pool.write().await` +
        // trait async 重构 (per 子代理审查 1.1 反馈, 见 backend/mod.rs 文档).
        //
        // continuity_id: 生产迁移 schema 要求 NOT NULL; 核心 Episode 无此字段,
        // 以 session_id 派生 (episode 的主体 = 其会话), 0 装诚实并保持写入可见.
        let ep = ep.clone();
        self.pool
            .write_sync(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO episodes (id, continuity_id, timestamp, role, content, session_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        ep.id,
                        ep.session_id,
                        ep.timestamp,
                        ep.role,
                        ep.content,
                        ep.session_id
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn get_episode(
        &self,
        id: &str,
    ) -> Result<Option<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        let id = id.to_string();
        self.pool
            .read(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT e.id, e.timestamp, e.role,
                            COALESCE(g.content_override, e.content), e.session_id
                       FROM episodes e
                       LEFT JOIN episode_governance g ON g.episode_id = e.id
                      WHERE e.id = ?1
                        AND (g.status IS NULL OR g.status <> 'forgotten')",
                )?;
                let mut rows = stmt.query(rusqlite::params![id])?;
                if let Some(row) = rows.next()? {
                    Ok(Some(Episode {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        session_id: row.get(4)?,
                    }))
                } else {
                    Ok(None)
                }
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn recent_episodes(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        let session_id = session_id.to_string();
        self.pool
            .read(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT e.id, e.timestamp, e.role,
                            COALESCE(g.content_override, e.content), e.session_id
                         FROM episodes e
                         LEFT JOIN episode_governance g ON g.episode_id = e.id
                         WHERE e.session_id = ?1
                           AND (g.status IS NULL OR g.status <> 'forgotten') \
                         ORDER BY timestamp DESC, id DESC \
                         LIMIT ?2",
                )?;
                let rows = stmt.query_map(rusqlite::params![session_id, n as i64], |row| {
                    Ok(Episode {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        session_id: row.get(4)?,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                out.reverse();
                Ok(out)
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn put_episode_metadata(
        &self,
        episode_id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let episode_id = episode_id.to_string();
        let metadata = serde_json::to_string(&metadata)?;
        self.pool
            .write_sync(move |conn| {
                // Keep this self-healing for older test/embedded schemas that
                // predate V9; the production migration creates the same table.
                //
                // M27 / L 组 (schema 所有权说明): `episode_memory_metadata` 的
                // canonical 定义在 migrations **V11** (带 `updated_at` 列).
                // 这里的懒建是 pre-V11 老库 self-heal 路径 — 只会建出 V9 时代的
                // 两列形态 (无 updated_at); 新库 `updated_at` 始终由 V11 提供.
                // 读取方不依赖 updated_at, 故两种形态行为一致; 但 schema 快照
                // 以 V11 为准, 不允许在此新增列定义.
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS episode_memory_metadata (\
                     episode_id TEXT PRIMARY KEY, metadata_json TEXT NOT NULL)",
                )?;
                conn.execute(
                    "INSERT INTO episode_memory_metadata (episode_id, metadata_json)\
                     VALUES (?1, ?2) ON CONFLICT(episode_id) DO UPDATE SET metadata_json = excluded.metadata_json",
                    rusqlite::params![episode_id, metadata],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn get_episode_metadata(
        &self,
        episode_id: &str,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let episode_id = episode_id.to_string();
        let raw = self
            .pool
            .read(move |conn| {
                let result = conn.query_row(
                    "SELECT metadata_json FROM episode_memory_metadata WHERE episode_id = ?1",
                    rusqlite::params![episode_id],
                    |row| row.get::<_, String>(0),
                );
                match result {
                    Ok(raw) => Ok(Some(raw)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                        if message.contains("no such table") =>
                    {
                        Ok(None)
                    }
                    Err(error) => Err(error.into()),
                }
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        raw.map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn append_stream(
        &self,
        kind: apeireth_core::kernel::StreamKind,
        entry: HistoryEntry,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let table = crate::StreamKindExt::table_name_ext(kind);
        // 0 装 PASS: 走 pool.read() 短借用. 单 backend 实例 = 单 logical writer
        // (per backend doc). 真并发 writer 是 v2.0.0-rc 阶段 `pool.write().await` + trait async.
        // INSERT OR IGNORE 保证同 id 重复 append 不冲突 (append-only 语义).
        // composite payload (含原始字段 + tags + tombstone + session_id)
        let payload = serde_json::json!({
            "id": entry.id,
            "subject_id": entry.subject_id,
            "subject_rev": entry.subject_rev,
            "session_id": entry.session_id,
            "created_at": entry.created_at,
            "source": entry.source,
            "tags": entry.tags,
            "tombstoned_at": entry.tombstoned_at,
            "payload": entry.payload,
        });
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        self.pool
            .write_sync(move |conn| {
                // H13: `session_id` / `tombstoned_at` 必须写**列** — 与
                // `append_only::insert_entry` (HistoryStream trait 路径) 的行
                // 布局统一. 否则两条写入路径对同一批 6 流表语义分裂: trait
                // 路径写的行本 backend 的 `list_for_session` 按列过滤查不到;
                // 本 backend 写的 tombstone 只在 payload, 按列过滤的 trait
                // 读者 (include_tombstoned=false) 看不见软删除.
                conn.execute(
                    &format!(
                        "INSERT OR IGNORE INTO {table} (id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
                    ),
                    rusqlite::params![
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
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn list_stream(
        &self,
        kind: apeireth_core::kernel::StreamKind,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let table = crate::StreamKindExt::table_name_ext(kind);
        let session_id = session_id.to_string();
        self.pool
            .read(|conn| {
                // H13: 行归属以 `session_id` 列为准, payload '$.session_id' 为
                // 兼容兜底 (旧版 SqliteBackend 只写 payload 不写列).
                // - 去掉 `json_extract(...) IS NULL` 全匹配兜底: 它会把其它
                //   session 的 trait 路径行 (payload 无 $.session_id 键) 也
                //   返回 — 跨 session 数据泄漏.
                // - 仅当调用方显式查询无归属条目 (session_id 传空串) 时才匹配
                //   NULL 归属.
                // - tombstone 同时查列与 payload: trait 路径的 tombstone 落在
                //   `tombstoned_at` 列, 旧实现只读 payload → 软删除条目被当
                //   活条目返回.
                let mut stmt = conn.prepare_cached(&format!(
                    "SELECT id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at \
                          FROM {table} \
                          WHERE ((session_id = ?1 OR (session_id IS NULL AND ?1 = '')) \
                             OR (json_extract(payload, '$.session_id') = ?1 \
                                 OR (json_extract(payload, '$.session_id') IS NULL AND ?1 = ''))) \
                            AND tombstoned_at IS NULL \
                            AND json_extract(payload, '$.tombstoned_at') IS NULL \
                          ORDER BY created_at ASC \
                          LIMIT ?2"
                ))?;
                let rows = stmt.query_map(rusqlite::params![session_id, n as i64], |row| {
                    let id: String = row.get(0)?;
                    let subject_id: String = row.get(1)?;
                    let subject_rev: i64 = row.get(2)?;
                    let created_at: i64 = row.get(4)?;
                    let payload_str: String = row.get(5)?;
                    let source: String = row.get(6)?;
                    let tags_str: String = row.get(7)?;
                    // 列优先 (H13 统一布局), payload 兜底 (旧版 SqliteBackend 行).
                    let column_session: Option<String> = row.get(3).ok().flatten();
                    let column_tombstone: Option<i64> = row.get(8).ok().flatten();
                    let payload: serde_json::Value =
                        serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
                    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                    let session_id = column_session.or_else(|| {
                        payload
                            .get("session_id")
                            .and_then(|v| v.as_str().map(String::from))
                    });
                    let tombstoned_at = column_tombstone.or_else(|| {
                        payload.get("tombstoned_at").and_then(|v| v.as_i64())
                    });
                    let inner_payload = payload
                        .get("payload")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    Ok(HistoryEntry {
                        id,
                        subject_id,
                        subject_rev,
                        session_id,
                        created_at,
                        payload: inner_payload,
                        source,
                        tags,
                        tombstoned_at,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                // 双保险: SQL 已同时过滤列与 payload 两种来源的 tombstone,
                // 这里按解码值再兜底一层 (未来 SQL 漏改也不会让软删条目复现).
                out.retain(|e| e.tombstoned_at.is_none());
                Ok(out)
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl crate::scope::ScopedMemoryBackend for SqliteBackend {
    fn query_candidates(
        &self,
        query: &crate::scope::MemoryCandidateQuery,
    ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        if query.visible_scopes.is_empty() || query.limit == 0 {
            return Ok(Vec::new());
        }

        let mut scope_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        for scope in &query.visible_scopes {
            match scope {
                crate::scope::MemoryScope::Global => {
                    scope_clauses.push(
                        "json_extract(m.metadata_json, '$.scope.scope') = 'global'".to_string(),
                    );
                }
                crate::scope::MemoryScope::Project { project_id } => {
                    scope_clauses.push("(json_extract(m.metadata_json, '$.scope.scope') = 'project' AND json_extract(m.metadata_json, '$.scope.project_id') = ?)".to_string());
                    params.push(Box::new(project_id.clone()));
                }
                crate::scope::MemoryScope::User { user_id } => {
                    scope_clauses.push("(json_extract(m.metadata_json, '$.scope.scope') = 'user' AND json_extract(m.metadata_json, '$.scope.user_id') = ?)".to_string());
                    params.push(Box::new(user_id.clone()));
                }
                crate::scope::MemoryScope::Persona {
                    persona_id,
                    user_id,
                } => {
                    scope_clauses.push("(json_extract(m.metadata_json, '$.scope.scope') = 'persona' AND json_extract(m.metadata_json, '$.scope.persona_id') = ? AND json_extract(m.metadata_json, '$.scope.user_id') = ?)".to_string());
                    params.push(Box::new(persona_id.clone()));
                    params.push(Box::new(user_id.clone()));
                }
                crate::scope::MemoryScope::Session { session_id } => {
                    scope_clauses.push("((json_extract(m.metadata_json, '$.scope.scope') = 'session' AND json_extract(m.metadata_json, '$.scope.session_id') = ?) OR (m.metadata_json IS NULL AND e.session_id = ?))".to_string());
                    params.push(Box::new(session_id.clone()));
                    params.push(Box::new(session_id.clone()));
                }
            }
        }

        if scope_clauses.is_empty() {
            return Ok(Vec::new());
        }

        let scope_sql = scope_clauses.join(" OR ");
        let mut sql = format!(
            "SELECT e.id, e.timestamp, e.role,
                    COALESCE(g.content_override, e.content), e.session_id
               FROM episodes e
               LEFT JOIN episode_memory_metadata m ON m.episode_id = e.id
               LEFT JOIN episode_governance g ON g.episode_id = e.id
              WHERE (g.status IS NULL OR g.status <> 'forgotten')
                AND ({scope_sql})"
        );

        if let Some(as_of_ms) = query.as_of_ms {
            let as_of_sec = as_of_ms / 1000;
            sql.push_str(" AND e.timestamp <= ?");
            params.push(Box::new(as_of_sec));
        }

        sql.push_str(" ORDER BY e.timestamp DESC, e.id DESC LIMIT ?");
        params.push(Box::new(query.limit as i64));

        self.pool
            .read(move |conn| {
                let mut stmt = conn.prepare(&sql)?;
                let param_refs: Vec<&dyn rusqlite::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();
                let rows = stmt.query_map(param_refs.as_slice(), |row| {
                    Ok(Episode {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        session_id: row.get(4)?,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                out.reverse();
                Ok(out)
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

// 注: `MemoryError::from` impl (rusqlite::Error → MemoryError) 已在 memory/src/error.rs 给出.
// 如未提供, 这里需手写 impl. 假设 v1 era SqliteMemoryStore 已有对应 impl.
// (snapshot: 0 重写 24 个 memory 子模块的 public API; `MemoryError::from` 是 crate 内 trait.)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::append_only::HistoryEntry;
    use apeireth_storage::SqliteConnectionPool;

    async fn fresh() -> SqliteBackend {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool");
        // RC-1 验收: 创 episodes + 6 streams 表
        // (per v2.0.0-rc-roadmap.md §3 RC-1: 绕开 SqliteMemoryStore mutex, 但 schema 必须有)
        // 简化: 用 StorageError 路径 inline 创 (避免 MemoryError→StorageError 转换)
        pool.write(|conn| -> Result<(), apeireth_storage::StorageError> {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS episodes (
                    id TEXT PRIMARY KEY,
                    continuity_id TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    session_id TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS episode_governance (
                    episode_id TEXT PRIMARY KEY,
                    status TEXT NOT NULL DEFAULT 'active',
                    protected INTEGER NOT NULL DEFAULT 0,
                    content_override TEXT,
                    revision INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER,
                    updated_by TEXT,
                    reason TEXT,
                    forgotten_at INTEGER
                );
                CREATE TABLE IF NOT EXISTS thought_stream (
                    id TEXT PRIMARY KEY,
                    subject_id TEXT NOT NULL,
                    subject_rev INTEGER NOT NULL,
                    session_id TEXT,
                    created_at INTEGER NOT NULL,
                    payload TEXT NOT NULL,
                    source TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    tombstoned_at INTEGER
                );
                CREATE TABLE IF NOT EXISTS proposal_stream (
                    id TEXT PRIMARY KEY, subject_id TEXT NOT NULL, subject_rev INTEGER NOT NULL,
                    session_id TEXT, created_at INTEGER NOT NULL, payload TEXT NOT NULL,
                    source TEXT NOT NULL, tags TEXT NOT NULL, tombstoned_at INTEGER
                );
                CREATE TABLE IF NOT EXISTS action_stream (
                    id TEXT PRIMARY KEY, subject_id TEXT NOT NULL, subject_rev INTEGER NOT NULL,
                    session_id TEXT, created_at INTEGER NOT NULL, payload TEXT NOT NULL,
                    source TEXT NOT NULL, tags TEXT NOT NULL, tombstoned_at INTEGER
                );
                CREATE TABLE IF NOT EXISTS relation_stream (
                    id TEXT PRIMARY KEY, subject_id TEXT NOT NULL, subject_rev INTEGER NOT NULL,
                    session_id TEXT, created_at INTEGER NOT NULL, payload TEXT NOT NULL,
                    source TEXT NOT NULL, tags TEXT NOT NULL, tombstoned_at INTEGER
                );
                CREATE TABLE IF NOT EXISTS evolution_stream (
                    id TEXT PRIMARY KEY, subject_id TEXT NOT NULL, subject_rev INTEGER NOT NULL,
                    session_id TEXT, created_at INTEGER NOT NULL, payload TEXT NOT NULL,
                    source TEXT NOT NULL, tags TEXT NOT NULL, tombstoned_at INTEGER
                );
                CREATE TABLE IF NOT EXISTS reflection_stream (
                    id TEXT PRIMARY KEY, subject_id TEXT NOT NULL, subject_rev INTEGER NOT NULL,
                    session_id TEXT, created_at INTEGER NOT NULL, payload TEXT NOT NULL,
                    source TEXT NOT NULL, tags TEXT NOT NULL, tombstoned_at INTEGER
                );
            "#,
            )
            .map_err(apeireth_storage::StorageError::from)
        })
        .await
        .expect("create schema");
        SqliteBackend::new(pool)
    }

    fn ep(id: &str, session: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp: 1_700_000_000,
            role: "user".to_string(),
            content: format!("content of {id}"),
            session_id: session.to_string(),
        }
    }

    fn he(id: &str, session: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            subject_id: "subj-1".to_string(),
            subject_rev: 1,
            session_id: Some(session.to_string()),
            created_at: 1_700_000_100,
            payload: serde_json::json!({"kind": "test"}),
            source: "test".to_string(),
            tags: vec!["unit".into()],
            tombstoned_at: None,
        }
    }

    #[tokio::test]
    async fn name_and_kind() {
        let b = fresh().await;
        assert_eq!(b.name(), "sqlite");
        assert_eq!(b.kind(), BackendKind::Sqlite);
    }

    #[tokio::test]
    async fn ping_succeeds() {
        let b = fresh().await;
        assert!(b.ping().is_ok());
    }

    #[tokio::test]
    async fn episode_roundtrip() {
        let b = fresh().await;
        let e = ep("ep-1", "sess-1");
        b.put_episode(&e).unwrap();
        let got = b.get_episode("ep-1").unwrap().expect("episode exists");
        assert_eq!(got.id, e.id);
        assert_eq!(got.content, e.content);
        let recent = b.recent_episodes("sess-1", 10).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[tokio::test]
    async fn recent_episodes_order_ascending_by_timestamp() {
        let b = fresh().await;
        // 写3 条不同 timestamp
        for (id, ts) in [("a", 100), ("b", 50), ("c", 75)].iter() {
            let mut e = ep(id, "s");
            e.timestamp = *ts;
            b.put_episode(&e).unwrap();
        }
        let recent = b.recent_episodes("s", 10).unwrap();
        assert_eq!(recent.len(), 3);
        // ORDER BY timestamp ASC
        assert_eq!(recent[0].id, "b");
        assert_eq!(recent[1].id, "c");
        assert_eq!(recent[2].id, "a");

        let recent = b.recent_episodes("s", 2).unwrap();
        assert_eq!(
            recent
                .iter()
                .map(|episode| episode.id.as_str())
                .collect::<Vec<_>>(),
            ["c", "a"]
        );
    }

    #[tokio::test]
    async fn append_and_list_stream_through_trait() {
        let b = fresh().await;
        let session = "sess-stream";
        let thought = apeireth_core::kernel::StreamKind::Thought;
        b.append_stream(thought, he("t-1", session)).unwrap();
        b.append_stream(thought, he("t-2", session)).unwrap();
        let listed = b.list_stream(thought, session, 10).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "t-1");
        assert_eq!(listed[1].id, "t-2");
    }

    /// H13: `append_stream` 必须写 `session_id` 列 (与 HistoryStream trait
    /// 路径的 `append_only::insert_entry` 行布局统一), 且 payload 顶层
    /// 带 session_id / tombstoned_at.
    #[tokio::test]
    async fn append_stream_writes_session_id_column() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        let mut entry = he("col-1", "sess-col");
        entry.tombstoned_at = Some(1_700_000_999);
        b.append_stream(thought, entry).unwrap();

        let (column_session, column_tombstone): (Option<String>, Option<i64>) = b
            .pool()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT session_id, tombstoned_at FROM thought_stream WHERE id = 'col-1'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            })
            .unwrap();
        assert_eq!(column_session.as_deref(), Some("sess-col"));
        assert_eq!(column_tombstone, Some(1_700_000_999));

        let payload: String = b
            .pool()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT payload FROM thought_stream WHERE id = 'col-1'",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(payload["session_id"], "sess-col");
        assert_eq!(payload["tombstoned_at"], 1_700_000_999);
    }

    /// H13: trait 路径 (append_only::insert_entry) 写入的行必须能被本
    /// backend 的 `list_stream` 看到 (统一布局前: 列空 + payload 无键 → 不可见).
    #[tokio::test]
    async fn trait_path_rows_are_visible_to_backend_list_stream() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        b.pool()
            .read(|conn| {
                crate::append_only::insert_entry(conn, "thought_stream", &he("trait-1", "sess-a"))
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
            })
            .unwrap();
        let listed = b.list_stream(thought, "sess-a", 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "trait-1");
    }

    /// H13: 本 backend 写的行必须能被 trait 路径的 `list_for_session` 看到
    /// (统一布局前: session_id 列为 NULL → 按列过滤一条都查不到).
    #[tokio::test]
    async fn backend_path_rows_are_visible_to_trait_list_for_session() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        b.append_stream(thought, he("backend-1", "sess-b")).unwrap();
        let rows = b
            .pool()
            .read(|conn| {
                crate::append_only::list_for_session(conn, "thought_stream", "sess-b", false)
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
            })
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "backend-1");
    }

    /// H13: 去掉 `IS NULL` 全匹配兜底后, 不得再把其它 session 的行返回
    /// (旧实现把 trait 路径行 (payload 无 $.session_id) 当"无归属"全匹配).
    #[tokio::test]
    async fn list_stream_does_not_leak_other_sessions() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        b.pool()
            .read(|conn| {
                crate::append_only::insert_entry(conn, "thought_stream", &he("a-1", "sess-a"))
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))?;
                crate::append_only::insert_entry(conn, "thought_stream", &he("b-1", "sess-b"))
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
            })
            .unwrap();
        let listed = b.list_stream(thought, "sess-a", 10).unwrap();
        assert_eq!(listed.len(), 1, "不得泄漏 sess-b 的条目");
        assert_eq!(listed[0].id, "a-1");
    }

    /// H13: 无归属 (NULL session_id) 行只有调用方显式查空串时才匹配.
    #[tokio::test]
    async fn unowned_rows_require_explicit_empty_session_query() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        b.pool()
            .read(|conn| {
                // 模拟旧版 SqliteBackend 写的行: 列 NULL + payload 无 $.session_id 键
                conn.execute(
                    "INSERT INTO thought_stream (id, subject_id, subject_rev, session_id, created_at, payload, source, tags, tombstoned_at) \
                     VALUES ('orphan-1', 'subj', 1, NULL, 100, '{\"kind\":\"orphan\"}', 'test', '[]', NULL)",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(
            b.list_stream(thought, "sess-any", 10).unwrap().is_empty(),
            "无归属行不得匹配具名 session 查询"
        );
        let unowned = b.list_stream(thought, "", 10).unwrap();
        assert_eq!(unowned.len(), 1);
        assert_eq!(unowned[0].id, "orphan-1");
        assert_eq!(unowned[0].session_id, None);
    }

    /// H13: trait 路径的 tombstone 在 `tombstoned_at` 列 — list_stream 必须
    /// 同时查列与 payload, 否则软删除条目被当活条目返回.
    #[tokio::test]
    async fn column_tombstone_hides_entry_from_backend_list_stream() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        let mut dead = he("dead-1", "sess-t");
        dead.tombstoned_at = Some(1_700_000_500);
        b.pool()
            .read(|conn| {
                crate::append_only::insert_entry(conn, "thought_stream", &dead)
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))?;
                crate::append_only::insert_entry(conn, "thought_stream", &he("alive-1", "sess-t"))
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
            })
            .unwrap();
        let listed = b.list_stream(thought, "sess-t", 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "alive-1");
    }

    /// H13: backend 路径写出的 tombstone (列 + payload 双写) 同样被过滤.
    #[tokio::test]
    async fn backend_tombstone_hides_entry_from_list_stream() {
        let b = fresh().await;
        let thought = apeireth_core::kernel::StreamKind::Thought;
        let mut dead = he("dead-2", "sess-u");
        dead.tombstoned_at = Some(1_700_000_600);
        b.append_stream(thought, dead).unwrap();
        assert!(b.list_stream(thought, "sess-u", 10).unwrap().is_empty());
    }

    #[tokio::test]
    async fn unknown_stream_name_is_compile_error() {
        // typed enum 不可能 unknown (编译期保证)
        let _b = fresh().await;
        let invalid = apeireth_core::kernel::StreamKind::Thought;
        let _ = invalid; // 验证编译过, 语义由 typed enum 保证
    }

    #[tokio::test]
    #[ignore = "wall-clock benchmark; run explicitly in performance validation"]
    async fn performance_1000_episodes_under_1s() {
        let b = fresh().await;
        let start = std::time::Instant::now();
        for i in 0..1000 {
            let mut e = ep(&format!("perf-{i}"), "perf-sess");
            e.timestamp = 1_700_000_000 + i;
            b.put_episode(&e).unwrap();
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 1,
            "1000 episode write took {elapsed:?}, > 1s (RC-1 验收失败)"
        );
    }
}
