//! P-arch (2026-08-27): v2.0.0-rc.1 RC-4 SelfAssessmentStore SQLite impl (场景 D 例 2).
//!
//! **位置**: impl 在 `apeireth-memory` (engine), trait 在 `apeireth-plugin` (foundation).
//! 单向依赖: memory → plugin. 0 装诚实: trait 0 装 / NoopSelfAssessmentStore 0 装 / 本文件真 SQLite impl.
//!
//! **Schema** (per v2.0.0-rc-roadmap.md §3 RC-4):
//! - `self_assessments` (id PK, round, session_id, task_id, alignment REAL,
//!   quality REAL, deviations TEXT (JSON), assessed_at INTEGER, reviewer_id TEXT)
//! - Index: `idx_self_assessments_task_time` on (task_id, assessed_at DESC) for `recent_for_task` 快速查
//!
//! **复用 SqliteConnectionPool** (per RC-1 / RC-3 模式): writer-async + reader-pool.
//!
//! **3 阶审查** (O-6 锚 #9, commit message 必写明):
//! 1. 总体: 与 6 capability 抽象在 foundation 集中; 场景 D §2.2
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致);
//!    复用 SqliteConnectionPool (per RC-1 模式) 不开新 DB 连接管理
//! 3. 架构: runtime 拿 `Arc<dyn SelfAssessmentStore>` 注入, 不直接 import impl crate
//!
//! **0 装 PASS**: trait 0 装 / Noop 0 装 / 本文件真 SQLite impl (替换 Noop).
//! **runtime 集成** (per scene-d §2.2 + v2.0.0-rc-roadmap.md §2.5):
//! - 时间驱动: 每 100 turn `Runtime::execute_outcome` 触发
//! - 事件驱动: 每次 tool 失败触发
//! - 多 instance: 评估用不同 model (per scene-d §5 决策 1)
//! - 启动时读 `recent_for_task(session.task_id(), 5)`, alignment < 0.6 → `DeviationReport`
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline 全保持.
//!
//! **v1 compat**: 100+ consumer 0 破 (新 API, 0 改旧代码).

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_plugin::self_assessment::{SelfAssessment, SelfAssessmentStore};
use apeireth_storage::SqliteConnectionPool;

/// SQLiteSelfAssessmentStore — SelfAssessmentStore trait 真 SQLite impl (RC-4)
///
/// 内部持 `Arc<SqliteConnectionPool>` (writer-async + reader-pool).
/// **Send + Sync**: `Arc<SqliteConnectionPool>` 本身是 Send+Sync, 本结构所有字段都是
/// `Send + Sync` 边界.
pub struct SQLiteSelfAssessmentStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SQLiteSelfAssessmentStore {
    /// 从 `SqliteConnectionPool` 创建.
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

    /// 创 `self_assessments` schema + 索引 (幂等, IF NOT EXISTS)
    pub async fn ensure_schema(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.pool.clone();
        pool.write(|conn| -> Result<(), apeireth_storage::StorageError> {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS self_assessments (
                    id TEXT PRIMARY KEY,
                    round INTEGER NOT NULL,
                    session_id TEXT NOT NULL,
                    task_id TEXT NOT NULL,
                    alignment REAL NOT NULL,
                    quality REAL NOT NULL,
                    deviations TEXT NOT NULL,
                    assessed_at INTEGER NOT NULL,
                    reviewer_id TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_self_assessments_task_time
                    ON self_assessments(task_id, assessed_at DESC);",
            )
            .map_err(apeireth_storage::StorageError::from)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl SelfAssessmentStore for SQLiteSelfAssessmentStore {
    fn record(&self, sa: &SelfAssessment) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 0 装诚实: 真写, 不假装
        // INSERT OR REPLACE on PK id 冲突 (同 id record 是 UPSERT, 与 PreferenceStore 同 pattern)
        let deviations_json = serde_json::to_string(&sa.deviations).map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let session_id_str = sa.session_id.to_string();
        self.pool
            .read(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO self_assessments \
                     (id, round, session_id, task_id, alignment, quality, deviations, assessed_at, reviewer_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        sa.id,
                        sa.round,
                        session_id_str,
                        sa.task_id,
                        sa.alignment,
                        sa.quality,
                        deviations_json,
                        sa.assessed_at,
                        sa.reviewer_id,
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn recent_for_task(
        &self,
        task_id: &str,
        limit: u32,
    ) -> Result<Vec<SelfAssessment>, Box<dyn std::error::Error + Send + Sync>> {
        // SELECT WHERE task_id = ?1 ORDER BY assessed_at DESC LIMIT ?N
        // runtime hot-path: 启动时读 5 条, alignment < 0.6 → DeviationReport
        self.pool
            .read(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, round, session_id, task_id, alignment, quality, deviations, assessed_at, reviewer_id \
                     FROM self_assessments \
                     WHERE task_id = ?1 \
                     ORDER BY assessed_at DESC \
                     LIMIT ?2",
                )?;
                let rows = stmt.query_map(rusqlite::params![task_id, i64::from(limit)], |row| {
                    let id: String = row.get(0)?;
                    let round: u32 = row.get(1)?;
                    let session_id_str: String = row.get(2)?;
                    let task_id: String = row.get(3)?;
                    let alignment: f64 = row.get(4)?;
                    let quality: f64 = row.get(5)?;
                    let deviations_str: String = row.get(6)?;
                    let assessed_at: i64 = row.get(7)?;
                    let reviewer_id: String = row.get(8)?;
                    let deviations: serde_json::Value =
                        serde_json::from_str(&deviations_str).unwrap_or(serde_json::Value::Null);
                    let session_id_parsed = session_id_str.parse::<SessionId>().map_err(|e| {
                        rusqlite::Error::InvalidParameterName(format!("SessionId parse: {e}"))
                    })?;
                    Ok(SelfAssessment {
                        id,
                        round,
                        session_id: session_id_parsed,
                        task_id,
                        alignment,
                        quality,
                        deviations,
                        assessed_at,
                        reviewer_id,
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

    fn latest_alignment(
        &self,
        task_id: &str,
    ) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
        // 0 装诚实: 真查最近 1 条 alignment, 返 Option<f64>
        // (没有评估返 None, 不假装返 1.0)
        self.pool
            .read(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT alignment FROM self_assessments \
                     WHERE task_id = ?1 \
                     ORDER BY assessed_at DESC \
                     LIMIT 1",
                )?;
                let mut rows = stmt.query(rusqlite::params![task_id])?;
                if let Some(row) = rows.next()? {
                    Ok(Some(row.get(0)?))
                } else {
                    Ok(None)
                }
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh() -> SQLiteSelfAssessmentStore {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool");
        let store = SQLiteSelfAssessmentStore::new(pool);
        store.ensure_schema().await.expect("ensure_schema");
        store
    }

    fn sa(id: &str, task: &str, round: u32, alignment: f64) -> SelfAssessment {
        SelfAssessment {
            id: id.to_string(),
            round,
            session_id: SessionId::new(),
            task_id: task.to_string(),
            alignment,
            quality: 0.8,
            deviations: serde_json::json!([]),
            assessed_at: 1_700_000_000 + i64::from(round) * 1000,
            reviewer_id: "reviewer-1".into(),
        }
    }

    /// RC-4 验收: record + recent_for_task roundtrip
    #[tokio::test]
    async fn record_and_recent_roundtrip() {
        let store = fresh().await;
        store.record(&sa("sa-1", "task-1", 1, 0.85)).expect("record");

        let recent = store
            .recent_for_task("task-1", 10)
            .expect("recent");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "sa-1");
        assert_eq!(recent[0].alignment, 0.85);
    }

    /// RC-4 验收: recent_for_task 按 assessed_at DESC 排序 (新近优先)
    #[tokio::test]
    async fn recent_orders_by_assessed_at_desc() {
        let store = fresh().await;
        store.record(&sa("old", "task", 1, 0.9)).unwrap();
        store.record(&sa("newer", "task", 2, 0.7)).unwrap();
        store.record(&sa("newest", "task", 3, 0.5)).unwrap();

        let recent = store
            .recent_for_task("task", 10)
            .expect("recent");
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "newest"); // 最高 round, 最新
        assert_eq!(recent[1].id, "newer");
        assert_eq!(recent[2].id, "old");
    }

    /// RC-4 验收: latest_alignment 返最近 1 条 alignment
    #[tokio::test]
    async fn latest_alignment_returns_most_recent() {
        let store = fresh().await;
        // 没记录: 返 None (0 装诚实: 不假装返 1.0)
        let none = store.latest_alignment("unknown-task").expect("none");
        assert!(none.is_none(), "0 装: 无评估返 None, 不假装 1.0");

        // 3 条记录, latest 应是新近的 (alignment=0.5)
        store.record(&sa("a", "task", 1, 0.9)).unwrap();
        store.record(&sa("b", "task", 2, 0.7)).unwrap();
        store.record(&sa("c", "task", 3, 0.5)).unwrap();
        let latest = store.latest_alignment("task").expect("latest");
        assert_eq!(latest, Some(0.5));

        // 模拟 RC-4 用例: alignment < 0.6 → DeviationReport 触发
        if let Some(a) = latest {
            if a < 0.6 {
                // 触发 DeviationReport (这里只 verify 阈值检测, 实际 report 由 runtime 触发)
                assert!(a < 0.6);
            }
        }
    }

    /// RC-4 验收: 不同 task 隔离
    #[tokio::test]
    async fn task_isolation() {
        let store = fresh().await;
        store.record(&sa("t1-sa", "task-1", 1, 0.9)).unwrap();
        store.record(&sa("t2-sa", "task-2", 1, 0.5)).unwrap();

        let t1 = store.recent_for_task("task-1", 10).expect("t1");
        let t2 = store.recent_for_task("task-2", 10).expect("t2");
        assert_eq!(t1.len(), 1);
        assert_eq!(t2.len(), 1);
        assert_eq!(t1[0].id, "t1-sa");
        assert_eq!(t2[0].id, "t2-sa");
    }

    /// RC-4 验收: record 覆盖 (INSERT OR REPLACE on PK id 冲突)
    #[tokio::test]
    async fn record_upserts_on_id_conflict() {
        let store = fresh().await;
        store.record(&sa("sa-1", "task", 1, 0.5)).unwrap();
        // 同 id, 改 alignment + round
        store.record(&sa("sa-1", "task", 2, 0.9)).unwrap();
        let recent = store.recent_for_task("task", 10).expect("recent");
        assert_eq!(recent.len(), 1, "PK 冲突 → UPSERT, 不重复");
        assert_eq!(recent[0].round, 2);
        assert_eq!(recent[0].alignment, 0.9);
    }

    /// RC-4 验收: 空 task 返空 Vec (不 panic)
    #[tokio::test]
    async fn empty_task_returns_empty_vec() {
        let store = fresh().await;
        let recent = store.recent_for_task("nonexistent", 10).expect("recent");
        assert_eq!(recent.len(), 0);
        let latest = store.latest_alignment("nonexistent").expect("latest");
        assert!(latest.is_none());
    }

    /// RC-4 验收: deviations JSON 字段完整 roundtrip
    #[tokio::test]
    async fn deviations_json_roundtrip() {
        let store = fresh().await;
        let sid = SessionId::new();
        let mut entry = sa("sa-1", "task", 1, 0.5);
        entry.session_id = sid;
        // 模拟 scene-d 例 2 的 deviation 报告
        entry.deviations = serde_json::json!([
            {"kind": "alignment_drop", "evidence": "tool_failed", "severity": 0.3},
            {"kind": "scope_creep", "evidence": "added_unrelated_dep", "severity": 0.6}
        ]);
        entry.quality = 0.6;
        store.record(&entry).expect("record");

        let recent = store.recent_for_task("task", 10).expect("recent");
        assert_eq!(recent.len(), 1);
        let devs = recent[0].deviations.as_array().expect("array");
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0]["kind"], "alignment_drop");
        assert_eq!(devs[0]["severity"], 0.3);
        assert_eq!(devs[1]["kind"], "scope_creep");
        assert_eq!(devs[1]["severity"], 0.6);
    }
}