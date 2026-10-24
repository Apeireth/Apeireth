//! 关系里程碑的 **SQLite 持久实现**（2026-10-24 接电）。
//!
//! # 为什么需要它
//!
//! 里程碑记录的是**关系史**（初见、阶段跃迁…）—— 这种东西重启即散等于没有。
//! 接入前只有 [`crate::milestone::InMemoryMilestoneStore`]（进程内），
//! 而生产者正是「每回合都在跑的羁绊模块」。
//!
//! # 与 `partner_store_sqlite` 同一套路
//!
//! - 表由本模块**首次读写时幂等建**（`CREATE TABLE IF NOT EXISTS`）——
//!   装配层不必记得调 `ensure_schema`（`partner_store_sqlite` 曾因「要手动调」
//!   而在生产上直接报 `no such table`，那是「能编译但用不了」的典型）；
//! - 落库语义就是「一条里程碑一行」，不做任何加工 —— 判定「何时算里程碑」
//!   是 [`crate::milestone::MilestoneRecorder`] 的事。
//!
//! # 查询确定性
//!
//! `query` 固定按 `at_epoch_ms` 升序、同刻按 `id` 升序 —— 同一份数据永远给出
//! 同一个顺序，便于对账与测试（本仓一贯口径）。

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_storage::SqliteConnectionPool;

use crate::milestone::{Milestone, MilestoneKind, MilestonePayload, MilestoneStore};
use crate::MemoryError;

/// 关系里程碑的 SQLite 存储。
#[derive(Clone)]
pub struct SqliteMilestoneStore {
    pool: Arc<SqliteConnectionPool>,
    /// 建表是否已尝试过（幂等，只试一次）。
    schema_ensured: Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for SqliteMilestoneStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteMilestoneStore")
            .finish_non_exhaustive()
    }
}

impl SqliteMilestoneStore {
    /// 共享一个已打开的连接池。
    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self {
            pool,
            schema_ensured: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 借用连接池建库（持有所有权）。
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self::from_arc(Arc::new(pool))
    }

    /// 建表（幂等；`CREATE TABLE IF NOT EXISTS`）。
    pub fn ensure_schema(&self) -> Result<(), MemoryError> {
        self.pool
            .write_sync(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS relationship_milestones (
                        id            TEXT PRIMARY KEY NOT NULL,
                        session_id    TEXT NOT NULL,
                        kind          TEXT NOT NULL,
                        payload_json  TEXT NOT NULL,
                        at_epoch_ms   INTEGER NOT NULL,
                        note          TEXT,
                        created_at_ms INTEGER NOT NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_relationship_milestones_session
                        ON relationship_milestones(session_id, at_epoch_ms);
                    "#,
                )?;
                Ok(())
            })
            .map_err(|error| MemoryError::Other(error.to_string()))
    }

    /// 首次使用前确保表存在（失败则把真实错误抛出去，不吞）。
    fn ensure_schema_once(&self) -> Result<(), MemoryError> {
        if self
            .schema_ensured
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }
        self.ensure_schema()
    }

    /// 某会话的里程碑条数（自述面 / 测试用）。
    pub fn count(&self, session_id: &SessionId) -> Result<i64, MemoryError> {
        self.ensure_schema_once()?;
        let session = session_id.to_string();
        self.pool
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM relationship_milestones WHERE session_id=?1",
                    rusqlite::params![session],
                    |row| row.get(0),
                )?)
            })
            .map_err(|error| MemoryError::Other(error.to_string()))
    }
}

/// 里程碑类型 ↔ 稳定字符串（落库用；**枚举改名会破坏历史数据**，
/// 所以这里显式写死而不是用 `Debug` 派生）。
fn kind_to_str(kind: MilestoneKind) -> &'static str {
    match kind {
        MilestoneKind::FirstMeeting => "first_meeting",
        MilestoneKind::FirstShare => "first_share",
        MilestoneKind::FirstEmotion => "first_emotion",
        MilestoneKind::StageTransition => "stage_transition",
        MilestoneKind::Decision => "decision",
        MilestoneKind::Conflict => "conflict",
        MilestoneKind::Repair => "repair",
        MilestoneKind::Custom => "custom",
    }
}

/// 稳定字符串 ↔ 里程碑类型（不认识的返回 `None`，由调用方当作数据损坏）。
fn kind_from_str(raw: &str) -> Option<MilestoneKind> {
    Some(match raw {
        "first_meeting" => MilestoneKind::FirstMeeting,
        "first_share" => MilestoneKind::FirstShare,
        "first_emotion" => MilestoneKind::FirstEmotion,
        "stage_transition" => MilestoneKind::StageTransition,
        "decision" => MilestoneKind::Decision,
        "conflict" => MilestoneKind::Conflict,
        "repair" => MilestoneKind::Repair,
        "custom" => MilestoneKind::Custom,
        _ => return None,
    })
}

impl MilestoneStore for SqliteMilestoneStore {
    fn record(&self, milestone: &Milestone) -> Result<(), MemoryError> {
        self.ensure_schema_once()?;
        let payload_json = serde_json::to_string(&milestone.payload)
            .map_err(|error| MemoryError::Other(format!("里程碑载荷序列化失败: {error}")))?;
        let id = milestone.id.clone();
        let session = milestone.session_id.to_string();
        let kind = kind_to_str(milestone.kind).to_string();
        let at = milestone.at_epoch_ms;
        let note = milestone.note.clone();
        self.pool
            .write_sync(move |conn| {
                conn.execute(
                    "INSERT INTO relationship_milestones \
                     (id, session_id, kind, payload_json, at_epoch_ms, note, created_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5)",
                    rusqlite::params![id, session, kind, payload_json, at, note],
                )?;
                Ok(())
            })
            .map_err(|error| MemoryError::Other(error.to_string()))
    }

    fn query(
        &self,
        session_id: &SessionId,
        kind: Option<MilestoneKind>,
    ) -> Result<Vec<Milestone>, MemoryError> {
        self.ensure_schema_once()?;
        let session = session_id.to_string();
        let kind_filter = kind.map(|value| kind_to_str(value).to_string());
        let rows: Vec<(String, String, String, String, i64, Option<String>)> = self
            .pool
            .read(move |conn| {
                // 两个分支而不是动态拼 SQL：避免字符串拼接带来的注入面与可读性损失。
                let build = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(
                    String,
                    String,
                    String,
                    String,
                    i64,
                    Option<String>,
                )> {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                };
                let mut out = Vec::new();
                match kind_filter {
                    Some(kind) => {
                        let mut stmt = conn.prepare(
                            "SELECT id, session_id, kind, payload_json, at_epoch_ms, note \
                             FROM relationship_milestones \
                             WHERE session_id=?1 AND kind=?2 \
                             ORDER BY at_epoch_ms ASC, id ASC",
                        )?;
                        let mut query = stmt.query(rusqlite::params![session, kind])?;
                        while let Some(row) = query.next()? {
                            out.push(build(row)?);
                        }
                    }
                    None => {
                        let mut stmt = conn.prepare(
                            "SELECT id, session_id, kind, payload_json, at_epoch_ms, note \
                             FROM relationship_milestones \
                             WHERE session_id=?1 \
                             ORDER BY at_epoch_ms ASC, id ASC",
                        )?;
                        let mut query = stmt.query(rusqlite::params![session])?;
                        while let Some(row) = query.next()? {
                            out.push(build(row)?);
                        }
                    }
                }
                Ok(out)
            })
            .map_err(|error| MemoryError::Other(error.to_string()))?;

        let mut milestones = Vec::with_capacity(rows.len());
        for (id, session_raw, kind_raw, payload_json, at_epoch_ms, note) in rows {
            let Some(kind) = kind_from_str(&kind_raw) else {
                // 不认识的类型 = 数据损坏（例如未来版本写入的新类型）。
                // 如实报错而不是静默丢弃 —— 静默丢会让「关系史少了一条」不可见。
                return Err(MemoryError::Other(format!(
                    "里程碑类型无法识别: {kind_raw}（id={id}）"
                )));
            };
            let payload: MilestonePayload =
                serde_json::from_str(&payload_json).map_err(|error| {
                    MemoryError::Other(format!("里程碑载荷无法解析（id={id}）: {error}"))
                })?;
            milestones.push(Milestone {
                id,
                session_id: session_raw
                    .parse()
                    .map_err(|error| MemoryError::Other(format!("会话 id 无法解析: {error}")))?,
                kind,
                payload,
                at_epoch_ms,
                note,
            });
        }
        Ok(milestones)
    }

    /// `EXISTS` 直查（不把整段历史读出来 —— 幂等闸每个回合都要问一次）。
    fn has_milestone(
        &self,
        session_id: &SessionId,
        kind: MilestoneKind,
    ) -> Result<bool, MemoryError> {
        self.ensure_schema_once()?;
        let session = session_id.to_string();
        let kind = kind_to_str(kind).to_string();
        self.pool
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM relationship_milestones \
                     WHERE session_id=?1 AND kind=?2)",
                    rusqlite::params![session, kind],
                    |row| row.get::<_, i64>(0),
                )? > 0)
            })
            .map_err(|error| MemoryError::Other(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> SqliteMilestoneStore {
        let pool = SqliteConnectionPool::in_memory().await.expect("pool");
        SqliteMilestoneStore::new(pool)
    }

    #[tokio::test]
    async fn round_trips_every_kind_and_payload_shape() {
        let store = store().await;
        let session = SessionId::new();
        let cases = [
            (
                MilestoneKind::FirstMeeting,
                MilestonePayload::Text("初见".into()),
            ),
            (
                MilestoneKind::FirstShare,
                MilestonePayload::Text("第一次分享".into()),
            ),
            (MilestoneKind::FirstEmotion, MilestonePayload::Number(0.72)),
            (
                MilestoneKind::StageTransition,
                MilestonePayload::Stage {
                    from: "Familiar".into(),
                    to: "Trusted".into(),
                },
            ),
            (
                MilestoneKind::Decision,
                MilestonePayload::Decision("一起决定搬去武汉".into()),
            ),
            (
                MilestoneKind::Conflict,
                MilestonePayload::Text("争执".into()),
            ),
            (MilestoneKind::Repair, MilestonePayload::Text("和好".into())),
        ];
        for (index, (kind, payload)) in cases.into_iter().enumerate() {
            // 逆序时间戳写入（后面的反而更早），验证查询按时间排序
            let at = 1_000 + (cases_len() - index as i64);
            let milestone = Milestone::new(session, kind, payload, at);
            store.record(&milestone).expect("record");
        }
        let all = store.query(&session, None).expect("query");
        assert_eq!(all.len(), 7, "{all:?}");
        assert!(
            all.windows(2)
                .all(|pair| pair[0].at_epoch_ms <= pair[1].at_epoch_ms),
            "必须按时间升序: {all:?}"
        );
        // 按类型过滤
        let transitions = store
            .query(&session, Some(MilestoneKind::StageTransition))
            .expect("query filtered");
        assert_eq!(transitions.len(), 1);
        match &transitions[0].payload {
            MilestonePayload::Stage { from, to } => {
                assert_eq!((from.as_str(), to.as_str()), ("Familiar", "Trusted"));
            }
            other => panic!("载荷类型错了: {other:?}"),
        }
    }

    fn cases_len() -> i64 {
        7
    }

    #[tokio::test]
    async fn surviving_a_new_store_instance_means_it_is_really_persisted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("milestones.sqlite");
        let session = SessionId::new();
        {
            let pool = SqliteConnectionPool::open(&path).await.expect("pool");
            let store = SqliteMilestoneStore::new(pool);
            store
                .record(&Milestone::new(
                    session,
                    MilestoneKind::FirstMeeting,
                    MilestonePayload::Text("初见".into()),
                    1_000,
                ))
                .expect("record");
        }
        // 新实例（模拟重启）
        let pool = SqliteConnectionPool::open(&path).await.expect("pool");
        let reopened = SqliteMilestoneStore::new(pool);
        let list = reopened.query(&session, None).expect("query");
        assert_eq!(list.len(), 1, "里程碑必须跨重启存在: {list:?}");
        assert!(reopened
            .has_milestone(&session, MilestoneKind::FirstMeeting)
            .expect("has"));
    }

    #[tokio::test]
    async fn empty_session_reads_as_empty_not_error() {
        let store = store().await;
        let list = store.query(&SessionId::new(), None).expect("query");
        assert!(list.is_empty(), "没有里程碑应返回空而不是报错");
        assert!(!store
            .has_milestone(&SessionId::new(), MilestoneKind::FirstMeeting)
            .expect("has"));
    }
}
