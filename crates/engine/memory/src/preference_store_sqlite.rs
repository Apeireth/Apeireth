//! P-arch (2026-08-27): v2.0.0-rc.1 RC-3 SQLitePreferenceStore impl (场景 D 例 1).
//!
//! **位置**: impl 在 `apeireth-memory` (engine), trait 在 `apeireth-plugin` (foundation).
//! 单向依赖: memory → plugin. 0 装诚实: trait 0 装 + NoopPreferenceStore (alpha 03f5ed71 已完成),
//! 本文件是真 SQLite impl 替换 Noop, 完成 v2.0.0-rc-roadmap.md §3 RC-3.
//!
//! **Schema** (per v2.0.0-rc-roadmap.md §3 RC-3):
//! - `user_preferences` (id PK, session_id, topic, stance TEXT, confidence REAL,
//!   evidence_refs TEXT (JSON array), tags TEXT (JSON array), created_at INTEGER)
//! - Index: `idx_user_prefs_session_confidence` on (session_id, confidence DESC)
//!   for `recall_for_context` 快速查
//!
//! **复用 SqliteConnectionPool** (per RC-1 模式): writer-async + reader-pool, 0 委托
//! `SqliteMemoryStore` (避免 Mutex<Connection> 串行).
//!
//! **3 阶审查** (O-6 锚 #9, commit message 必写明):
//! 1. 总体: 与 6 capability 抽象 (MemoryBackend/Experience/Perception/PreferenceStore/
//!    SelfAssessmentStore/CredentialResolver) 在 foundation 集中
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致);
//!    复用 SqliteConnectionPool (per RC-1 模式) 不开新 DB 连接管理
//! 3. 架构: runtime 拿 `Arc<dyn PreferenceStore>` 注入, 不直接 import impl crate
//!
//! **0 装 PASS**: trait 0 装 / NoopPreferenceStore 0 装 / 本文件真 SQLite impl
//! (替换 Noop). AI 写入 confidence 不假装 100% (UserPreference::confidence 字段).
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline
//! 全保持.
//!
//! **v1 compat**: 100+ consumer 0 破 (新 API, 0 改旧代码).

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_plugin::preference::{PreferenceStore, UserPreference};
use apeireth_storage::SqliteConnectionPool;

/// SQLitePreferenceStore — PreferenceStore trait 真 SQLite impl (RC-3)
///
/// 内部持 `Arc<SqliteConnectionPool>` (writer-async + reader-pool).
/// **Send + Sync**: `Arc<SqliteConnectionPool>` 本身是 Send+Sync, 本结构所有字段都是
/// `Send + Sync` 边界.
pub struct SQLitePreferenceStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SQLitePreferenceStore {
    /// 从 `SqliteConnectionPool` 创建 (注入 `Arc<SqliteConnectionPool>`).
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// 从 `Arc<SqliteConnectionPool>` 创建 (共享场景).
    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqliteConnectionPool {
        &self.pool
    }

    /// 创 `user_preferences` schema + 索引 (幂等, IF NOT EXISTS)
    /// 与 RC-1 SqliteBackend::fresh() 同样 inline 创 schema (test 简洁)
    /// 0 装诚实: 不依赖 migrations 系统的 Migration trait (避免 MemoryError→StorageError 转换)
    pub async fn ensure_schema(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.pool.clone();
        pool.write(|conn| -> Result<(), apeireth_storage::StorageError> {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS user_preferences (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    topic TEXT NOT NULL,
                    stance TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    evidence_refs TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_user_prefs_session_confidence
                    ON user_preferences(session_id, confidence DESC);
            "#,
            )
            .map_err(apeireth_storage::StorageError::from)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl PreferenceStore for SQLitePreferenceStore {
    fn record(
        &self,
        pref: &UserPreference,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // INSERT OR REPLACE: 同一 id 覆盖 (id 是 PK, scene-d §2.1 v1 实践: SHA-256
        // session_id + topic 派生, 同 session + topic 多次 record 是更新不是新增)
        let evidence_refs_json = serde_json::to_string(&pref.evidence_refs)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let tags_json = serde_json::to_string(&pref.tags)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let session_id_str = pref.session_id.to_string();
        // 同步读 — RC-3 trait method 是 sync, 用 reader pool
        self.pool
            .read(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO user_preferences \
                     (id, session_id, topic, stance, confidence, evidence_refs, tags, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        pref.id,
                        session_id_str,
                        pref.topic,
                        pref.stance,
                        pref.confidence,
                        evidence_refs_json,
                        tags_json,
                        pref.created_at,
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn recall_for_context(
        &self,
        session_id: &SessionId,
        current_topic: &str,
        limit: u32,
    ) -> Result<Vec<UserPreference>, Box<dyn std::error::Error + Send + Sync>> {
        let session_id_str = session_id.to_string();
        // SELECT 路径: 同 session 下, topic LIKE current_topic (前缀匹配)
        // + ORDER BY confidence DESC, created_at DESC (新近优先)
        // + LIMIT ?N
        self.pool
            .read(|conn| {
                let mut stmt = conn
                    .prepare_cached(
                        "SELECT id, session_id, topic, stance, confidence, evidence_refs, tags, created_at \
                         FROM user_preferences \
                         WHERE session_id = ?1 \
                           AND (?2 = '' OR topic LIKE '%' || ?2 || '%') \
                         ORDER BY confidence DESC, created_at DESC \
                         LIMIT ?3",
                    )?;
                let rows = stmt
                    .query_map(rusqlite::params![session_id_str, current_topic, i64::from(limit)], |row| {
                        let id: String = row.get(0)?;
                        let session_id_str: String = row.get(1)?;
                        let topic: String = row.get(2)?;
                        let stance: String = row.get(3)?;
                        let confidence: f64 = row.get(4)?;
                        let evidence_refs_str: String = row.get(5)?;
                        let tags_str: String = row.get(6)?;
                        let created_at: i64 = row.get(7)?;
                        let evidence_refs: Vec<String> =
                            serde_json::from_str(&evidence_refs_str).unwrap_or_default();
                        let tags: Vec<String> =
                            serde_json::from_str(&tags_str).unwrap_or_default();
                        // session_id 解析: 字符串 → SessionId
                        let session_id_parsed = session_id_str
                            .parse::<SessionId>()
                            .map_err(|e| rusqlite::Error::InvalidParameterName(format!("SessionId parse: {e}")),
                            )?;
                        Ok(UserPreference {
                            id,
                            session_id: session_id_parsed,
                            topic,
                            stance,
                            confidence,
                            evidence_refs,
                            tags,
                            created_at,
                        })
                    })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn forget(&self, pref_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 0 装诚实: 真删, 不留 tombstone (preference 是事实记录, 不是 audit log)
        self.pool
            .read(|conn| {
                conn.execute(
                    "DELETE FROM user_preferences WHERE id = ?1",
                    rusqlite::params![pref_id],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn list_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<UserPreference>, Box<dyn std::error::Error + Send + Sync>> {
        let session_id_str = session_id.to_string();
        // 不 LIMIT, 列全部 (主人查看 / 导出用, 通常量小)
        // 0 装诚实: 如果一个 session 累积 10000 preferences, 这会慢.
        // rc 阶段加 pagination (limit + offset)
        self.pool
            .read(|conn| {
                let mut stmt = conn
                    .prepare_cached(
                        "SELECT id, session_id, topic, stance, confidence, evidence_refs, tags, created_at \
                         FROM user_preferences \
                         WHERE session_id = ?1 \
                         ORDER BY confidence DESC, created_at DESC",
                    )?;
                let rows = stmt.query_map(rusqlite::params![session_id_str], |row| {
                    let id: String = row.get(0)?;
                    let session_id_str: String = row.get(1)?;
                    let topic: String = row.get(2)?;
                    let stance: String = row.get(3)?;
                    let confidence: f64 = row.get(4)?;
                    let evidence_refs_str: String = row.get(5)?;
                    let tags_str: String = row.get(6)?;
                    let created_at: i64 = row.get(7)?;
                    let evidence_refs: Vec<String> =
                        serde_json::from_str(&evidence_refs_str).unwrap_or_default();
                    let tags: Vec<String> =
                        serde_json::from_str(&tags_str).unwrap_or_default();
                    let session_id_parsed = session_id_str
                        .parse::<SessionId>()
                        .map_err(|e| rusqlite::Error::InvalidParameterName(format!("SessionId parse: {e}")),
                        )?;
                    Ok(UserPreference {
                        id,
                        session_id: session_id_parsed,
                        topic,
                        stance,
                        confidence,
                        evidence_refs,
                        tags,
                        created_at,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    async fn fresh() -> SQLitePreferenceStore {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool");
        let store = SQLitePreferenceStore::new(pool);
        store.ensure_schema().await.expect("ensure_schema");
        store
    }

    fn pref(id: &str, sid: SessionId, topic: &str, confidence: f64) -> UserPreference {
        UserPreference {
            id: id.to_string(),
            session_id: sid,
            topic: topic.to_string(),
            stance: format!("主人偏好 {topic} 因为 reasons"),
            evidence_refs: vec!["ep-1".to_string()],
            created_at: 1_700_000_000,
            confidence,
            tags: vec!["test".to_string()],
        }
    }

    /// RC-3 验收: record + recall roundtrip
    #[tokio::test]
    async fn record_and_recall_roundtrip() {
        let store = fresh().await;
        let sid = SessionId::new();
        let p = pref("pref-1", sid, "Rust language", 0.85);
        store.record(&p).expect("record");

        let recalled = store.recall_for_context(&sid, "Rust", 10).expect("recall");
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].id, "pref-1");
        assert_eq!(recalled[0].confidence, 0.85);
    }

    /// RC-3 验收: recall_for_context 按 confidence desc 排序 (top-N)
    #[tokio::test]
    async fn recall_orders_by_confidence_desc() {
        let store = fresh().await;
        let sid = SessionId::new();
        // 3 pref 同样 topic, confidence 不同
        store.record(&pref("low", sid, "topic", 0.3)).unwrap();
        store.record(&pref("high", sid, "topic", 0.9)).unwrap();
        store.record(&pref("mid", sid, "topic", 0.6)).unwrap();

        let recalled = store.recall_for_context(&sid, "topic", 10).expect("recall");
        assert_eq!(recalled.len(), 3);
        assert_eq!(recalled[0].id, "high");
        assert_eq!(recalled[1].id, "mid");
        assert_eq!(recalled[2].id, "low");
    }

    /// RC-3 验收: recall_for_context topic 过滤 (LIKE 匹配)
    #[tokio::test]
    async fn recall_filters_by_topic() {
        let store = fresh().await;
        let sid = SessionId::new();
        store
            .record(&pref("rust-1", sid, "Rust language", 0.9))
            .unwrap();
        store
            .record(&pref("py-1", sid, "Python language", 0.7))
            .unwrap();
        store
            .record(&pref("rust-2", sid, "Rust tooling", 0.8))
            .unwrap();

        let rust_only = store.recall_for_context(&sid, "Rust", 10).expect("recall");
        assert_eq!(rust_only.len(), 2);
        let ids: Vec<&str> = rust_only.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"rust-1"));
        assert!(ids.contains(&"rust-2"));
        assert!(!ids.contains(&"py-1"));
    }

    /// RC-3 验收: forget 真删 (不假装)
    #[tokio::test]
    async fn forget_actually_deletes() {
        let store = fresh().await;
        let sid = SessionId::new();
        store.record(&pref("p1", sid, "topic", 0.8)).unwrap();
        let before = store.list_for_session(&sid).expect("list");
        assert_eq!(before.len(), 1);

        store.forget("p1").expect("forget");
        let after = store.list_for_session(&sid).expect("list");
        assert_eq!(after.len(), 0, "forget 真删, 不假装");
    }

    /// RC-3 验收: list_for_session 列全部 (无 LIMIT, 主人查看用)
    #[tokio::test]
    async fn list_for_session_returns_all() {
        let store = fresh().await;
        let sid = SessionId::new();
        for i in 0..5 {
            store
                .record(&pref(&format!("p{i}"), sid, "topic", 0.5))
                .unwrap();
        }
        let list = store.list_for_session(&sid).expect("list");
        assert_eq!(list.len(), 5);
    }

    /// RC-3 验收: 不同 session 隔离
    #[tokio::test]
    async fn session_isolation() {
        let store = fresh().await;
        let sid1 = SessionId::new();
        let sid2 = SessionId::new();
        store.record(&pref("p1", sid1, "topic", 0.9)).unwrap();
        store.record(&pref("p2", sid2, "topic", 0.5)).unwrap();

        let r1 = store.recall_for_context(&sid1, "topic", 10).expect("r1");
        let r2 = store.recall_for_context(&sid2, "topic", 10).expect("r2");
        assert_eq!(r1.len(), 1);
        assert_eq!(r2.len(), 1);
        assert_eq!(r1[0].id, "p1");
        assert_eq!(r2[0].id, "p2");
    }

    /// RC-3 验收: record 覆盖 (INSERT OR REPLACE on PK id 冲突)
    #[tokio::test]
    async fn record_upserts_on_id_conflict() {
        let store = fresh().await;
        let sid = SessionId::new();
        let mut p = pref("p1", sid, "topic1", 0.5);
        store.record(&p).unwrap();
        // 同 id, 改 topic + confidence
        p.topic = "topic2".to_string();
        p.confidence = 0.9;
        store.record(&p).unwrap();
        let list = store.list_for_session(&sid).expect("list");
        assert_eq!(list.len(), 1, "PK 冲突 → UPSERT, 不重复");
        assert_eq!(list[0].topic, "topic2");
        assert_eq!(list[0].confidence, 0.9);
    }
}
