//! 可恢复摄取队列 (V4-b 记忆保鲜回路三件套第②件): 待摄取清单落库, 重启续摄。
//!
//! # 机制
//!
//! 放行的摄取任务落进 [`ingest_queue`] 表 (待摄取清单): **进程重启续摄** ——
//! 清单在库里, 重启后 [`SqliteMemoryStore::pending_ingests`] 照常可见, 泵接着
//! 摄; **失败留队列 + 尝试计数** —— 摄取失败的任务不静默丢、不伪造完成, 行
//! 留在 `pending` 并累加 `attempts`、记 `last_error`, 等下一次泵重试。
//!
//! # 串行泵 (一次一条)
//!
//! [`SqliteMemoryStore::pump_next_ingest`] 一次处理**最早**的一条待摄取 (串行,
//! 不放大下游并发); 折入与队列记账、散列记账在**同一事务**里原子落库 ——
//! 不会出现「段折了但账没记 / 账记了但队列没收场」的裂缝。
//!
//! # 幂等 (同源同散列单行)
//!
//! `ingest_queue` 对 `(source, content_hash)` 有唯一约束: 重复投递不产生第二
//! 行 ([`EnqueueOutcome::AlreadyQueued`]); 同源同散列已在散列账上 (摄取过) 的
//! 投递根本不会入队, 走散列门的 [`EnqueueOutcome::DuplicateSkipped`]。

use rusqlite::OptionalExtension;

use crate::freshness::fold::fold_conn;
use crate::freshness::hash_gate::{
    check_conn, mark_ingested_conn, mark_skipped_conn, GateDecision,
};
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 摄取任务状态 (两态; 失败留在 `pending`, 不是终态)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestState {
    /// 待摄取 (含摄取失败待重试的)。
    Pending,
    /// 已摄取 (折入成功并已记散列账)。
    Done,
}

impl IngestState {
    /// 库内文本值。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Done => "done",
        }
    }

    fn from_row(value: &str) -> MemoryResult<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            other => Err(MemoryError::Invalid(format!(
                "unknown ingest task state: {other}"
            ))),
        }
    }
}

/// 一条摄取任务 (待摄取清单的一行)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestTask {
    /// 任务 id (确定性派号: `ingest-<源散列>-<内容散列>`)。
    pub id: String,
    /// 来源 (同源同散列去重的第一维)。
    pub source: String,
    /// 折入主题 (增量折入的合并键)。
    pub topic: String,
    /// 摄取内容 (逐字保留)。
    pub content: String,
    /// 内容散列 (FNV-1a 64)。
    pub content_hash: String,
    /// 当前状态。
    pub state: IngestState,
    /// 尝试计数 (每次泵 +1, 成败都计; 失败留队列)。
    pub attempts: u32,
    /// 最近一次失败原因 (`None` = 未失败过)。
    pub last_error: Option<String>,
    /// 入队时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 最近推进时刻 (epoch 毫秒)。
    pub updated_at_ms: i64,
}

/// 入队结果 (幂等: 同源同散列单行 / 已摄取过不入队)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueOutcome {
    /// 新任务已入队。
    Enqueued {
        /// 任务 id。
        task_id: String,
    },
    /// 同源同散列已摄取过 = 重复摄入, 散列门跳过 (不入队, 不重复记忆)。
    DuplicateSkipped {
        /// 内容散列。
        content_hash: String,
        /// 累计去重计数 (含本次)。
        skip_count: u64,
    },
    /// 同源同散列已在队 (含摄取失败待重试的) = 不产生第二行。
    AlreadyQueued {
        /// 既有任务 id。
        task_id: String,
    },
}

/// 泵一条的结果 (串行泵, 一次一条)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PumpOutcome {
    /// 已折入 (同主题追加新段, 散列账已记摄入)。
    Folded {
        /// 任务 id。
        task_id: String,
        /// 折入主题。
        topic: String,
        /// 新段序号。
        seq: i64,
    },
    /// 摄取失败: **留队列** + 尝试计数 +1 + 记原因 (等下一次泵)。
    Failed {
        /// 任务 id。
        task_id: String,
        /// 失败后的尝试计数。
        attempts: u32,
        /// 失败原因。
        error: String,
    },
    /// 无待摄取 (队列清空)。
    Idle,
}

impl SqliteMemoryStore {
    /// 入队一条摄取任务 (先过散列增量门)。
    ///
    /// - 同源同散列已摄取过 → [`EnqueueOutcome::DuplicateSkipped`] (去重计数 +1);
    /// - 同源同散列已在队 → [`EnqueueOutcome::AlreadyQueued`] (单行, 不重复摄);
    /// - 否则 → [`EnqueueOutcome::Enqueued`]。
    pub fn enqueue_ingest(
        &self,
        source: &str,
        topic: &str,
        content: &str,
        now_ms: i64,
    ) -> MemoryResult<EnqueueOutcome> {
        crate::freshness::validate_topic(topic)?;
        let id = ingest_task_id(source, content);
        let hash = crate::freshness::content_hash(content);
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        match check_conn(&tx, source, content)? {
            GateDecision::SkipDuplicate { .. } => {
                let skip_count = mark_skipped_conn(&tx, source, content, now_ms)?;
                tx.commit()?;
                return Ok(EnqueueOutcome::DuplicateSkipped {
                    content_hash: hash,
                    skip_count,
                });
            }
            GateDecision::Ingest { .. } => {}
        }
        let inserted = tx.execute(
            "INSERT INTO ingest_queue \
                 (id, source, topic, content, content_hash, state, attempts, last_error, \
                  created_ms, updated_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, NULL, ?6, ?6) \
             ON CONFLICT(source, content_hash) DO NOTHING",
            rusqlite::params![id, source, topic, content, hash, now_ms],
        )?;
        if inserted == 0 {
            let existing: String = tx.query_row(
                "SELECT id FROM ingest_queue WHERE source = ?1 AND content_hash = ?2",
                rusqlite::params![source, hash],
                |row| row.get(0),
            )?;
            tx.commit()?;
            return Ok(EnqueueOutcome::AlreadyQueued { task_id: existing });
        }
        tx.commit()?;
        Ok(EnqueueOutcome::Enqueued { task_id: id })
    }

    /// 待摄取清单 (按入队时刻升序, 同刻按 id; 失败待重试的也在内)。
    pub fn pending_ingests(&self) -> MemoryResult<Vec<IngestTask>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, source, topic, content, content_hash, state, attempts, last_error, \
                    created_ms, updated_ms \
             FROM ingest_queue WHERE state = 'pending' \
             ORDER BY created_ms ASC, id ASC",
        )?;
        let rows = stmt
            .query_map([], row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 读一条摄取任务 (不存在 = `None`)。
    pub fn ingest_task(&self, id: &str) -> MemoryResult<Option<IngestTask>> {
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT id, source, topic, content, content_hash, state, attempts, last_error, \
                        created_ms, updated_ms \
                 FROM ingest_queue WHERE id = ?1",
                rusqlite::params![id],
                row_to_task,
            )
            .optional()?;
        Ok(row)
    }

    /// 泵一条待摄取 (串行: 一次处理最早一条; 无待摄取回 [`PumpOutcome::Idle`])。
    pub fn pump_next_ingest(&self, now_ms: i64) -> MemoryResult<PumpOutcome> {
        self.pump_next_ingest_with(now_ms, |_| Ok(()))
    }

    /// 泵一条待摄取, 带可注入的**纯计算**前置步 (摄取加工/质检)。
    ///
    /// 前置步失败 = 摄取失败: 任务**留队列** (`pending`), `attempts` +1, 记
    /// `last_error`。前置步不碰库 (纯计算, 无锁序问题); 折入与队列/散列记账在
    /// 同一事务里原子落库。
    pub fn pump_next_ingest_with<F>(&self, now_ms: i64, prep: F) -> MemoryResult<PumpOutcome>
    where
        F: FnOnce(&IngestTask) -> Result<(), String>,
    {
        let mut conn = self.conn()?;
        let task = next_pending(&conn)?;
        let Some(task) = task else {
            return Ok(PumpOutcome::Idle);
        };
        if let Err(error) = prep(&task) {
            let attempts = record_failure(&conn, &task.id, now_ms, &error)?;
            return Ok(PumpOutcome::Failed {
                task_id: task.id,
                attempts,
                error,
            });
        }
        let tx = conn.transaction()?;
        match fold_conn(&tx, &task.topic, &task.content, now_ms) {
            Ok(report) => {
                mark_ingested_conn(&tx, &task.source, &task.content, now_ms)?;
                tx.execute(
                    "UPDATE ingest_queue \
                     SET state = 'done', attempts = attempts + 1, updated_ms = ?2 \
                     WHERE id = ?1",
                    rusqlite::params![task.id, now_ms],
                )?;
                tx.commit()?;
                Ok(PumpOutcome::Folded {
                    task_id: task.id,
                    topic: report.topic,
                    seq: report.seq,
                })
            }
            Err(error) => {
                drop(tx);
                let attempts = record_failure(&conn, &task.id, now_ms, &error.to_string())?;
                Ok(PumpOutcome::Failed {
                    task_id: task.id,
                    attempts,
                    error: error.to_string(),
                })
            }
        }
    }
}

/// 确定性任务 id (同源同散列永远同一行)。
fn ingest_task_id(source: &str, content: &str) -> String {
    format!(
        "ingest-{:016x}-{}",
        crate::freshness::fnv1a64(source),
        crate::freshness::content_hash(content)
    )
}

fn next_pending(conn: &rusqlite::Connection) -> MemoryResult<Option<IngestTask>> {
    let row = conn
        .query_row(
            "SELECT id, source, topic, content, content_hash, state, attempts, last_error, \
                    created_ms, updated_ms \
             FROM ingest_queue WHERE state = 'pending' \
             ORDER BY created_ms ASC, id ASC LIMIT 1",
            [],
            row_to_task,
        )
        .optional()?;
    Ok(row)
}

/// 失败记账: 留队列 + 尝试计数 +1 + 记原因 (不静默丢, 不伪造完成)。
fn record_failure(
    conn: &rusqlite::Connection,
    id: &str,
    now_ms: i64,
    error: &str,
) -> MemoryResult<u32> {
    conn.execute(
        "UPDATE ingest_queue \
         SET attempts = attempts + 1, last_error = ?2, updated_ms = ?3 \
         WHERE id = ?1",
        rusqlite::params![id, error, now_ms],
    )?;
    let attempts: i64 = conn.query_row(
        "SELECT attempts FROM ingest_queue WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get(0),
    )?;
    Ok(attempts.max(0) as u32)
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<IngestTask> {
    let state: String = row.get(5)?;
    Ok(IngestTask {
        id: row.get(0)?,
        source: row.get(1)?,
        topic: row.get(2)?,
        content: row.get(3)?,
        content_hash: row.get(4)?,
        state: IngestState::from_row(&state).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        attempts: row.get(6)?,
        last_error: row.get(7)?,
        created_at_ms: row.get(8)?,
        updated_at_ms: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("in-memory store")
    }

    /// 入队幂等: 同源同散列重复投递不产生第二行。
    #[test]
    fn enqueue_is_idempotent_per_source_and_hash() {
        let store = store();
        let first = store
            .enqueue_ingest("src-a", "主题甲", "内容甲", 1_000)
            .expect("enqueue 1");
        let second = store
            .enqueue_ingest("src-a", "主题甲", "内容甲", 2_000)
            .expect("enqueue 2");
        match (first, second) {
            (
                EnqueueOutcome::Enqueued { task_id },
                EnqueueOutcome::AlreadyQueued { task_id: again },
            ) => {
                assert_eq!(task_id, again);
            }
            other => panic!("同源同散列必须单行: {other:?}"),
        }
        assert_eq!(store.pending_ingests().expect("pending").len(), 1);
    }

    /// 重复摄入不入队 (散列门跳过 + 去重计数真值)。
    #[test]
    fn already_ingested_content_is_skipped_and_counted() {
        let store = store();
        let outcome = store
            .enqueue_ingest("src-a", "主题甲", "内容甲", 1_000)
            .expect("enqueue");
        let task_id = match outcome {
            EnqueueOutcome::Enqueued { task_id } => task_id,
            other => panic!("必须新入队: {other:?}"),
        };
        assert!(matches!(
            store.pump_next_ingest(1_500).expect("pump"),
            PumpOutcome::Folded { .. }
        ));
        assert!(store.ingest_task(&task_id).expect("task").is_some());

        let repeat = store
            .enqueue_ingest("src-a", "主题甲", "内容甲", 2_000)
            .expect("repeat");
        match repeat {
            EnqueueOutcome::DuplicateSkipped {
                content_hash,
                skip_count,
            } => {
                assert_eq!(content_hash, crate::freshness::content_hash("内容甲"));
                assert_eq!(skip_count, 1, "去重计数真值");
            }
            other => panic!("摄取过的同源同散列必须跳过: {other:?}"),
        }
        assert_eq!(store.pending_ingests().expect("pending").len(), 0);
    }

    /// 变内容重摄: 同源内容有变 → 新行入队, 两条都可摄。
    #[test]
    fn changed_content_enqueues_a_new_task() {
        let store = store();
        store
            .enqueue_ingest("src-a", "主题甲", "内容甲", 1_000)
            .expect("enqueue 1");
        let second = store
            .enqueue_ingest("src-a", "主题甲", "内容乙", 2_000)
            .expect("enqueue 2");
        assert!(matches!(second, EnqueueOutcome::Enqueued { .. }));
        assert_eq!(store.pending_ingests().expect("pending").len(), 2);
    }

    /// 失败留队列 + 尝试计数: 摄取失败不丢行、不伪造完成。
    #[test]
    fn failed_ingest_stays_queued_with_attempt_count() {
        let store = store();
        let outcome = store
            .enqueue_ingest("src-a", "主题甲", "内容甲", 1_000)
            .expect("enqueue");
        let task_id = match outcome {
            EnqueueOutcome::Enqueued { task_id } => task_id,
            other => panic!("必须新入队: {other:?}"),
        };

        let failed = store
            .pump_next_ingest_with(2_000, |_| Err("质检不过".to_string()))
            .expect("pump");
        match failed {
            PumpOutcome::Failed {
                task_id: failed_id,
                attempts,
                error,
            } => {
                assert_eq!(failed_id, task_id);
                assert_eq!(attempts, 1, "尝试计数");
                assert_eq!(error, "质检不过");
            }
            other => panic!("失败必须如实上报: {other:?}"),
        }
        let queued = store.pending_ingests().expect("pending");
        assert_eq!(queued.len(), 1, "失败必须留队列");
        assert_eq!(queued[0].state, IngestState::Pending);
        assert_eq!(queued[0].attempts, 1);
        assert_eq!(queued[0].last_error.as_deref(), Some("质检不过"));

        // 下一次泵重试成功: 收场为 done, 尝试计数累计为 2。
        let recovered = store.pump_next_ingest(3_000).expect("pump");
        assert!(matches!(recovered, PumpOutcome::Folded { .. }));
        let done = store.ingest_task(&task_id).expect("task").expect("present");
        assert_eq!(done.state, IngestState::Done);
        assert_eq!(done.attempts, 2, "成败都计尝试");
    }

    /// 队列空 = Idle (不编造工作)。
    #[test]
    fn empty_queue_pumps_idle() {
        let store = store();
        assert_eq!(
            store.pump_next_ingest(1_000).expect("pump"),
            PumpOutcome::Idle
        );
    }
}
