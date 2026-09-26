//! 耐久输入队列 (Durable Inbox): next-turn / next-step 双队列 + claim 原子批走。
//!
//! # 双队列分流
//!
//! - [`InboxQueue::NextTurn`] — 下一回合才处理的输入;
//! - [`InboxQueue::NextStep`] — 下一步就处理的输入。
//!
//! 两队列持久化、互不混流: [`DurableInbox::claim`] 只从指定队列按 FIFO 批走。
//! **回合中途到达的输入照常入队不丢** —— 入队是耐久写 (持久档原子替换),
//! 中途进来的输入与回合开始前进来的输入同库同序。
//!
//! # claim 原子批走
//!
//! [`DurableInbox::claim`] 一次取走至多 `max` 条, 整批的出队在**一次**原子落盘
//! 里完成: 要么整批出队已落盘, 要么整批还在队里。一次取走的**不重复投递** ——
//! claim 落盘后即便立刻崩溃, 被取走的批不会在重启后重新出现
//! (claim 是 at-most-once 的取走点)。
//!
//! # 与回合生命周期叠加 (不改语义)
//!
//! 本模块只做「输入 = 耐久队列」: 与既有回合生命周期 (轮次编号 / 续行快照)
//! 叠加时, [`InboxItem::turn`] 携带轮次归属, 队列本身不解释回合语义、不改
//! 任何既有生命周期状态。工作量侧的「可折叠事实」见 [`crate::work_consumption`]。
//!
//! # 写路径
//!
//! 只消费已完结的 `apeireth_core::storage_atomic` 两档原子写: 全部落盘走
//! 持久档 [`storage_atomic::write_atomic_durable`] + [`storage_atomic::with_file_lock`]
//! 串行化读改写; claim 的整批出队是一次原子替换。
//!
//! # 诚实声明
//!
//! 「入队」(push) 与「任务执行完成」是两次独立的持久写: 与
//! [`crate::durable_schedule`] 的投递清单配套时, 两次写之间崩溃可以造成同一
//! 发生时点重复投递 (at-least-once), 执行方必须按身份键幂等去重。本模块的
//! claim 保证的是**取走不重复**, 不是「执行恰好一次」。

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use apeireth_core::storage_atomic::{self, DEFAULT_FILE_MODE};

use crate::durable_schedule::{ScheduleError, ScheduleStore};

// ============================================================================
// 错误
// ============================================================================

/// 输入队列域错误的稳定 code 族。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxErrorCode {
    /// 入参不合法。
    InvalidInput,
    /// 持久档读写失败。
    Io,
    /// 持久档内容无法解析。
    Corrupt,
}

impl InboxErrorCode {
    /// 稳定 code 字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "inbox_invalid_input",
            Self::Io => "inbox_io",
            Self::Corrupt => "inbox_corrupt",
        }
    }
}

impl std::fmt::Display for InboxErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 输入队列域的失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxError {
    /// 入参不合法。
    InvalidInput {
        /// 人读原因。
        reason: String,
    },
    /// 持久档读写失败。
    Io {
        /// 失败的操作。
        operation: &'static str,
        /// 人读原因。
        reason: String,
    },
    /// 持久档内容无法解析。
    Corrupt {
        /// 人读原因。
        reason: String,
    },
}

impl InboxError {
    /// 错误归属 code。
    pub const fn code(&self) -> InboxErrorCode {
        match self {
            Self::InvalidInput { .. } => InboxErrorCode::InvalidInput,
            Self::Io { .. } => InboxErrorCode::Io,
            Self::Corrupt { .. } => InboxErrorCode::Corrupt,
        }
    }
}

impl std::fmt::Display for InboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { reason } => write!(f, "invalid inbox input: {reason}"),
            Self::Io { operation, reason } => write!(f, "inbox {operation}: {reason}"),
            Self::Corrupt { reason } => write!(f, "inbox store corrupt: {reason}"),
        }
    }
}

impl std::error::Error for InboxError {}

/// 输入队列域 result 别名。
pub type InboxResult<T> = Result<T, InboxError>;

// ============================================================================
// 队列 / 条目
// ============================================================================

/// 双队列之一: 分流不混流。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InboxQueue {
    /// 下一回合处理的输入。
    NextTurn,
    /// 下一步处理的输入。
    NextStep,
}

/// 一条耐久输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxItem {
    /// 条目 id (入队时为空则由存储按序派号)。
    pub id: String,
    /// 归属队列 (分流键)。
    pub queue: InboxQueue,
    /// 来源标识 (不透明, 存储不解释)。
    pub source: String,
    /// 回合归属 (可空; 只携带, 不解释回合语义)。
    pub turn: Option<u64>,
    /// 入队时刻 (epoch 毫秒)。
    pub enqueued_at_ms: i64,
    /// 不透明载荷。
    pub payload: String,
}

/// 输入队列持久档 (内存镜像)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct InboxState {
    /// 已应用的最大写派号 (跨重启单调)。
    write_seq: u64,
    /// next-turn 队列 (FIFO)。
    next_turn: VecDeque<InboxItem>,
    /// next-step 队列 (FIFO)。
    next_step: VecDeque<InboxItem>,
}

/// 耐久输入队列: 双队列 + push 耐久入队 + claim 原子批走。
pub struct DurableInbox {
    path: PathBuf,
    state: InboxState,
}

impl DurableInbox {
    /// 打开 (或初始化) 指定路径的输入档: 文件不存在即空库。
    pub fn open(path: impl Into<PathBuf>) -> InboxResult<Self> {
        let path = path.into();
        let state = load_state(&path)?;
        Ok(Self { path, state })
    }

    /// 持久档路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 已应用的最大写派号。
    pub fn write_seq(&self) -> u64 {
        self.state.write_seq
    }

    /// 指定队列的待处理条数。
    pub fn pending(&self, queue: InboxQueue) -> usize {
        self.queue(queue).len()
    }

    /// 指定队列的待处理快照 (FIFO 序)。
    pub fn items(&self, queue: InboxQueue) -> impl Iterator<Item = &InboxItem> {
        match queue {
            InboxQueue::NextTurn => self.state.next_turn.iter(),
            InboxQueue::NextStep => self.state.next_step.iter(),
        }
    }

    /// 耐久入队 (回合中途到达的输入同样走这里, 不丢)。
    ///
    /// `item.id` 为空时由存储按写派号派号; 入队即原子落盘。
    pub fn push(&mut self, mut item: InboxItem) -> InboxResult<InboxItem> {
        if item.payload.is_empty() {
            return Err(InboxError::InvalidInput {
                reason: "inbox payload must not be empty".to_string(),
            });
        }
        let mut next = self.state.clone();
        next.write_seq = next.write_seq.saturating_add(1);
        if item.id.is_empty() {
            item.id = format!("inbox-{}", next.write_seq);
        }
        match item.queue {
            InboxQueue::NextTurn => next.next_turn.push_back(item.clone()),
            InboxQueue::NextStep => next.next_step.push_back(item.clone()),
        }
        self.persist(&next)?;
        self.state = next;
        Ok(item)
    }

    /// claim 原子批走: 从指定队列 FIFO 取走至多 `max` 条, 整批出队一次原子落盘。
    ///
    /// 一次取走的不重复投递: 落盘后即便崩溃, 被取走的批不会重新出现。
    pub fn claim(&mut self, queue: InboxQueue, max: usize) -> InboxResult<Vec<InboxItem>> {
        let mut next = self.state.clone();
        let target = match queue {
            InboxQueue::NextTurn => &mut next.next_turn,
            InboxQueue::NextStep => &mut next.next_step,
        };
        let taken: Vec<InboxItem> = target.drain(..max.min(target.len())).collect();
        if taken.is_empty() {
            return Ok(taken);
        }
        next.write_seq = next.write_seq.saturating_add(1);
        self.persist(&next)?;
        self.state = next;
        Ok(taken)
    }

    /// 原子落盘 (持久档 + 文件锁): 只消费 `storage_atomic` 的两档原子写。
    fn persist(&self, state: &InboxState) -> InboxResult<()> {
        let bytes = serde_json::to_vec(state).map_err(|e| InboxError::Corrupt {
            reason: format!("serialize inbox state: {e}"),
        })?;
        let lock_path = storage_atomic::lock_path_for(&self.path);
        let written = storage_atomic::with_file_lock(&lock_path, || {
            storage_atomic::write_atomic_durable(&self.path, &bytes, DEFAULT_FILE_MODE)
        })
        .map_err(|e| InboxError::Io {
            operation: "lock inbox store",
            reason: e.to_string(),
        })?;
        written.map_err(|e| InboxError::Io {
            operation: "write inbox store",
            reason: e.to_string(),
        })
    }

    fn queue(&self, queue: InboxQueue) -> &VecDeque<InboxItem> {
        match queue {
            InboxQueue::NextTurn => &self.state.next_turn,
            InboxQueue::NextStep => &self.state.next_step,
        }
    }
}

/// 读档: 缺失即空库; 内容无法解析 fail-closed 报 [`InboxError::Corrupt`]。
fn load_state(path: &Path) -> InboxResult<InboxState> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(InboxState::default());
        }
        Err(err) => {
            return Err(InboxError::Io {
                operation: "read inbox store",
                reason: err.to_string(),
            });
        }
    };
    serde_json::from_slice(&bytes).map_err(|e| InboxError::Corrupt {
        reason: format!("parse inbox state: {e}"),
    })
}

// ============================================================================
// 接线示范: 调度投递 → 耐久输入队列
// ============================================================================

/// 接线失败 (两个持久库各有各的错误族)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// 调度库侧失败。
    Schedule(ScheduleError),
    /// 输入队列侧失败。
    Inbox(InboxError),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schedule(err) => write!(f, "wire schedule side: {err}"),
            Self::Inbox(err) => write!(f, "wire inbox side: {err}"),
        }
    }
}

impl std::error::Error for WireError {}

/// 接线示范: 调度到期投递 → 耐久输入队列 (生产侧)。
///
/// 取 [`ScheduleStore::collect_due`] 的到期投递 (补发纪律: 每调度至多一条),
/// 逐条推入 [`InboxQueue::NextStep`] 队列, 载荷为投递记录的 JSON
/// (执行方可 [`serde_json::from_str`] 还原 [`crate::durable_schedule::ScheduleDelivery`],
/// 拿 (schedule_id, occurrence_ms) 做幂等键)。任务执行完成后回
/// [`ScheduleStore::acknowledge_execution`] 记账 —— 入队与执行记账是两次
/// 持久写, 中间崩溃可重复投递 (见 [`crate::durable_schedule`] 模块文档)。
///
/// 返回实际入队的条目 (FIFO 序)。
pub fn wire_due_deliveries_into_inbox(
    store: &mut ScheduleStore,
    inbox: &mut DurableInbox,
    turn: Option<u64>,
    now_ms: i64,
) -> Result<Vec<InboxItem>, WireError> {
    let deliveries = store.collect_due(now_ms).map_err(WireError::Schedule)?;
    let mut pushed = Vec::with_capacity(deliveries.len());
    for delivery in &deliveries {
        let payload = serde_json::to_string(delivery).map_err(|e| {
            WireError::Schedule(ScheduleError::Corrupt {
                reason: format!("serialize delivery: {e}"),
            })
        })?;
        let item = InboxItem {
            id: String::new(),
            queue: InboxQueue::NextStep,
            source: format!("schedule:{}", delivery.schedule_id),
            turn,
            enqueued_at_ms: now_ms,
            payload,
        };
        pushed.push(inbox.push(item).map_err(WireError::Inbox)?);
    }
    Ok(pushed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durable_schedule::{ScheduleRecord, ScheduleStatus, ScheduleTrigger};

    const T0: i64 = 1_700_000_000_000;
    const HOUR_MS: i64 = 3_600_000;

    fn item(queue: InboxQueue, payload: &str) -> InboxItem {
        InboxItem {
            id: String::new(),
            queue,
            source: "test".to_string(),
            turn: None,
            enqueued_at_ms: T0,
            payload: payload.to_string(),
        }
    }

    fn inbox_at(dir: &tempfile::TempDir) -> DurableInbox {
        DurableInbox::open(dir.path().join("inbox.json")).expect("open inbox")
    }

    #[test]
    fn next_turn_and_next_step_queues_route_independently() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut inbox = inbox_at(&dir);
        inbox.push(item(InboxQueue::NextTurn, "t1")).expect("push");
        inbox.push(item(InboxQueue::NextStep, "s1")).expect("push");
        inbox.push(item(InboxQueue::NextTurn, "t2")).expect("push");
        inbox.push(item(InboxQueue::NextStep, "s2")).expect("push");

        // 分流不混流: 各自 FIFO。
        assert_eq!(inbox.pending(InboxQueue::NextTurn), 2);
        assert_eq!(inbox.pending(InboxQueue::NextStep), 2);
        let steps = inbox.claim(InboxQueue::NextStep, 10).expect("claim");
        assert_eq!(
            steps.iter().map(|i| i.payload.as_str()).collect::<Vec<_>>(),
            vec!["s1", "s2"]
        );
        assert_eq!(inbox.pending(InboxQueue::NextTurn), 2, "另一队列不受影响");
        let turns = inbox.claim(InboxQueue::NextTurn, 10).expect("claim");
        assert_eq!(
            turns.iter().map(|i| i.payload.as_str()).collect::<Vec<_>>(),
            vec!["t1", "t2"]
        );
    }

    #[test]
    fn claim_takes_a_batch_atomically_without_redelivery() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut inbox = inbox_at(&dir);
        for n in 0..3 {
            inbox
                .push(item(InboxQueue::NextStep, &format!("p{n}")))
                .expect("push");
        }
        // 整批一次取走 (至多 max 条), 已取走的不重复投递。
        let first = inbox.claim(InboxQueue::NextStep, 2).expect("claim");
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].payload, "p0");
        assert_eq!(first[1].payload, "p1");
        let second = inbox.claim(InboxQueue::NextStep, 2).expect("claim");
        assert_eq!(second.len(), 1, "只剩一条");
        assert_eq!(second[0].payload, "p2");
        assert!(inbox
            .claim(InboxQueue::NextStep, 2)
            .expect("claim")
            .is_empty());
        // 派号 id 单调, 便于执行方按 id 定序。
        assert_eq!(first[0].id, "inbox-1");
        assert_eq!(second[0].id, "inbox-3");
    }

    #[test]
    fn crash_after_claim_does_not_redeliver_the_taken_batch() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let mut inbox = inbox_at(&dir);
            for n in 0..3 {
                inbox
                    .push(item(InboxQueue::NextStep, &format!("p{n}")))
                    .expect("push");
            }
            let taken = inbox.claim(InboxQueue::NextStep, 2).expect("claim");
            assert_eq!(taken.len(), 2);
            // claim 落盘后立刻崩溃: 取走的批已出队。
        }
        let mut reopened = inbox_at(&dir);
        // 崩溃恢复不重复投递: 被取走的批不回来, 只剩未取走的那条。
        let rest = reopened.claim(InboxQueue::NextStep, 10).expect("claim");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].payload, "p2");
    }

    #[test]
    fn mid_turn_inputs_are_queued_and_not_lost() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut inbox = inbox_at(&dir);
        // 回合开始前一条。
        inbox
            .push(item(InboxQueue::NextTurn, "before"))
            .expect("push");
        // 回合中途到达的输入: 照常入队, 不丢。
        inbox
            .push(item(InboxQueue::NextStep, "mid-1"))
            .expect("push");
        inbox
            .push(item(InboxQueue::NextStep, "mid-2"))
            .expect("push");
        drop(inbox); // 中途进程崩溃。

        let mut reopened = inbox_at(&dir);
        assert_eq!(reopened.pending(InboxQueue::NextTurn), 1);
        assert_eq!(
            reopened.pending(InboxQueue::NextStep),
            2,
            "中途输入耐久入队"
        );
        let mid = reopened.claim(InboxQueue::NextStep, 10).expect("claim");
        assert_eq!(
            mid.iter().map(|i| i.payload.as_str()).collect::<Vec<_>>(),
            vec!["mid-1", "mid-2"]
        );
    }

    #[test]
    fn wire_due_deliveries_into_inbox_drives_the_producer_side() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store =
            crate::durable_schedule::ScheduleStore::open(dir.path().join("schedule.json"))
                .expect("open schedule store");
        store
            .create(ScheduleRecord {
                id: "tick".to_string(),
                revision: 0,
                trigger: ScheduleTrigger::Every {
                    every_ms: HOUR_MS,
                    anchor_ms: T0,
                },
                status: ScheduleStatus::Active,
                payload: "run-me".to_string(),
                created_at_ms: T0,
                updated_at_ms: T0,
                last_fired_at_ms: None,
                fire_count: 0,
            })
            .expect("create");
        let mut inbox = inbox_at(&dir);

        // 接线: 到期投递进 next-step 队列; 载荷可还原成投递记录。
        let pushed =
            wire_due_deliveries_into_inbox(&mut store, &mut inbox, Some(7), T0).expect("wire");
        assert_eq!(pushed.len(), 1);
        assert_eq!(pushed[0].queue, InboxQueue::NextStep);
        assert_eq!(pushed[0].turn, Some(7));
        let claimed = inbox.claim(InboxQueue::NextStep, 1).expect("claim");
        let delivery: crate::durable_schedule::ScheduleDelivery =
            serde_json::from_str(&claimed[0].payload).expect("delivery payload round-trips");
        assert_eq!(delivery.schedule_id, "tick");
        assert_eq!(delivery.occurrence_ms, T0);

        // 执行完成回调度库记账 (第二次持久写), 待执行清单清空。
        store
            .acknowledge_execution(delivery.schedule_id, delivery.occurrence_ms, T0 + 1)
            .expect("ack");
        assert!(store.unacknowledged_deliveries().is_empty());
    }

    #[test]
    fn empty_payload_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut inbox = inbox_at(&dir);
        let err = inbox
            .push(item(InboxQueue::NextStep, ""))
            .expect_err("empty payload");
        assert_eq!(err.code(), InboxErrorCode::InvalidInput);
    }
}
