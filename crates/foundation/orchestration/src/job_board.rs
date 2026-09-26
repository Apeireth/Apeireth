//! 任务身份与所有权 + 完成通知: [`JobBoard`] 是工具执行长任务的过程输出保留处。
//!
//! # 机制 (与 [`crate::job_ring`] 配套的三件事)
//!
//! 1. **任务身份与所有权** ([`JobHandle`]): 每个长任务带 `id` 与 `owner`;
//!    观察按 id ([`JobBoard::read_at`] 等只认任务 id), 写入 / 取消 / 结算认
//!    句柄 (owner 不符一律拒绝)。取消**只影响未启动的段** ([`SegmentState`]
//!    中 `Queued` 的段置 `Skipped`): 已产出的输出保留在环内可读, 已启动的段
//!    不被追溯。
//! 2. **完成通知**: [`JobBoard::wait_for_completion`] 登记等待方, [`JobBoard::on_completion`]
//!    登记事件监听。任务结算 ([`JobBoard::settle`] / [`JobBoard::cancel`]) **先
//!    释放等待方、再发事件**; `awaited` 标记去重通知 —— 同一等待方无论轮询
//!    多少次都只有一个等待槽, 至多唤醒一次。
//! 3. **空闲 owner 的 follow-up 唤醒**: 任务结算时若 owner 已无运行中的段,
//!    经 [`WakeGate`] 发一次 follow-up 唤醒建议; 连续无进展的唤醒受
//!    `max_consecutive_wakes` 上限约束 (封自激链), [`JobBoard::record_progress`]
//!    记录真实进展即清零计数。
//!
//! # 分工边界 (与相邻机制语义互补, 互不替代)
//!
//! - 过程输出的有界留存与双游标读取在 [`crate::job_ring`] (环管"过程输出流");
//! - 单条超长内容的溢出落盘在 [`crate::context_budget::SpillWriter`] (spill 管
//!   "单次超长内容"), 与本环互补不冲突;
//! - 任务硬超时 (见 [`crate::subagent_llm::SUBAGENT_DEFAULT_TIMEOUT_MS`]) 保留为
//!   最终兜底中止: 环 + 完成通知先行落地解决过程可见与结算唤醒, 硬超时只兜底。

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use crate::job_ring::{JobRing, RingSlice, RingStats};

/// 任务标识 (板内自增, 观察按 id 寻址)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u64);

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job-{}", self.0)
    }
}

/// 任务归属方标识 (谁启动、谁持有句柄)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OwnerId(String);

impl OwnerId {
    /// 构造归属方标识。
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// 名称。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OwnerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 任务句柄: 身份 (`id`) + 所有权 (`owner`)。
///
/// 写入 / 取消 / 结算必须出示与登记一致的句柄; 观察只需 id。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobHandle {
    /// 任务标识。
    pub id: JobId,
    /// 归属方。
    pub owner: OwnerId,
}

impl JobHandle {
    /// 组造句柄。
    pub fn new(id: JobId, owner: OwnerId) -> Self {
        Self { id, owner }
    }
}

/// 段序号 (任务内 0 起)。
pub type SegmentIndex = u32;

/// 段状态: 取消只影响未启动的段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentState {
    /// 已排队, 尚未启动。
    Queued,
    /// 已启动 (取消不追溯)。
    Started,
    /// 已完成。
    Finished,
    /// 未启动即被跳过 (取消/结算时置入)。
    Skipped,
}

/// 任务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// 运行中 (未结算)。
    Running,
    /// 已完成结算。
    Completed,
    /// 已取消结算。
    Cancelled,
}

/// 结算结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobOutcome {
    /// 终态。
    pub status: JobStatus,
    /// 从未启动即被跳过的段数。
    pub skipped_segments: SegmentIndex,
}

/// 结算报告: 通知释放情况 + 空闲 owner 的 follow-up 唤醒决定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleReport {
    /// 结算结果。
    pub outcome: JobOutcome,
    /// 本次结算释放的等待方数 (每个等待方至多释放一次)。
    pub waiters_released: usize,
    /// owner 结算后处于空闲时的 follow-up 唤醒决定; owner 仍忙则为 `None`。
    pub owner_wake: Option<WakeDecision>,
}

/// 事件通知监听器: 结算释放等待方之后被调用, 每个监听器至多调用一次。
pub type CompletionListener = Arc<dyn Fn(&JobOutcome) + Send + Sync>;

/// follow-up 唤醒决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeDecision {
    /// 放行唤醒 (连续唤醒计数 +1)。
    Proceed,
    /// 超过 `max_consecutive_wakes` 上限, 抑制本次唤醒 (封自激链)。
    Suppressed,
}

/// 空闲 owner 的 follow-up 唤醒闸。
///
/// 连续无进展的唤醒计数超过 `max_consecutive_wakes` 即抑制; [`WakeGate::record_progress`]
/// 记录真实进展后计数清零。
#[derive(Debug)]
pub struct WakeGate {
    max_consecutive_wakes: u32,
    consecutive: HashMap<OwnerId, u32>,
}

impl WakeGate {
    /// 新建唤醒闸: 连续唤醒上限 `max_consecutive_wakes`。
    pub fn new(max_consecutive_wakes: u32) -> Self {
        Self {
            max_consecutive_wakes,
            consecutive: HashMap::new(),
        }
    }

    /// 连续唤醒上限。
    pub fn max_consecutive_wakes(&self) -> u32 {
        self.max_consecutive_wakes
    }

    /// 申请一次 follow-up 唤醒。
    pub fn try_wake(&mut self, owner: &OwnerId) -> WakeDecision {
        let count = self.consecutive.entry(owner.clone()).or_insert(0);
        if *count < self.max_consecutive_wakes {
            *count += 1;
            WakeDecision::Proceed
        } else {
            WakeDecision::Suppressed
        }
    }

    /// 记录真实进展: 连续唤醒计数清零。
    pub fn record_progress(&mut self, owner: &OwnerId) {
        self.consecutive.insert(owner.clone(), 0);
    }

    /// 当前连续无进展唤醒计数。
    pub fn consecutive_wakes(&self, owner: &OwnerId) -> u32 {
        self.consecutive.get(owner).copied().unwrap_or(0)
    }
}

/// 板配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardConfig {
    /// 每任务输出环的总留存量上限 (字节)。
    pub ring_capacity_bytes: usize,
    /// 空闲 owner 的连续 follow-up 唤醒上限。
    pub max_consecutive_wakes: u32,
}

impl Default for BoardConfig {
    fn default() -> Self {
        Self {
            ring_capacity_bytes: 256 * 1024,
            max_consecutive_wakes: 3,
        }
    }
}

/// 板操作错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardError {
    /// 任务 id 不存在 (观察或句柄出示)。
    UnknownJob(JobId),
    /// 句柄的 owner 与登记不符 (所有权校验)。
    WrongOwner {
        /// 被出示的任务 id。
        job: JobId,
    },
    /// 任务已结算 (终态不可再改)。
    AlreadySettled(JobId),
    /// 任务尚未结算 (释放记录前必须先结算)。
    NotSettled(JobId),
    /// 段序号非法或段状态不允许该操作。
    BadSegment {
        /// 被出示的任务 id。
        job: JobId,
        /// 段序号。
        segment: SegmentIndex,
    },
}

impl fmt::Display for BoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownJob(job) => write!(f, "unknown job {job}"),
            Self::WrongOwner { job } => write!(f, "handle owner does not own {job}"),
            Self::AlreadySettled(job) => write!(f, "{job} is already settled"),
            Self::NotSettled(job) => write!(f, "{job} is not settled yet"),
            Self::BadSegment { job, segment } => {
                write!(f, "segment {segment} of {job} rejects this operation")
            }
        }
    }
}

impl std::error::Error for BoardError {}

/// 等待方槽位: `awaited` 标记 + `woken` 去重。
struct WaiterSlot {
    /// 已登记等待 (awaited 标记)。
    awaited: bool,
    /// 已唤醒 —— 去重通知: 置位后不再二次唤醒。
    woken: bool,
    /// 当前登记的 waker (重复轮询只保留最新一份)。
    waker: Option<Waker>,
}

/// 单任务的完成通知状态 (等待方 + 事件监听)。
#[derive(Default)]
struct CompletionState {
    outcome: Option<JobOutcome>,
    waiters: Vec<WaiterSlot>,
    listeners: Vec<CompletionListener>,
    notified: bool,
}

struct JobRecord {
    owner: OwnerId,
    ring: JobRing,
    segments: Vec<SegmentState>,
    outcome: Option<JobOutcome>,
    completion: Arc<Mutex<CompletionState>>,
}

struct BoardInner {
    config: BoardConfig,
    next_id: u64,
    jobs: HashMap<JobId, JobRecord>,
    wake_gate: WakeGate,
}

/// 完成等待方: [`JobBoard::wait_for_completion`] 返回的 future。
///
/// 同一等待方多次轮询只登记一个等待槽 (awaited 标记去重), 结算至多唤醒一次。
pub struct CompletionWaiter {
    completion: Arc<Mutex<CompletionState>>,
    slot: usize,
}

impl Future for CompletionWaiter {
    type Output = JobOutcome;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<JobOutcome> {
        let this = self.get_mut();
        let mut state = lock(&this.completion);
        if let Some(outcome) = state.outcome {
            return Poll::Ready(outcome);
        }
        if let Some(slot) = state.waiters.get_mut(this.slot) {
            slot.awaited = true;
            slot.waker = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

/// 任务板: 工具执行长任务的过程输出保留处。
///
/// 多任务按 id 各持一环 ([`crate::job_ring::JobRing`]); 本类型内部加锁,
/// 多观察者 + 一消费者的并发读写在锁边界序列化, 读写不相交错。
pub struct JobBoard {
    inner: Arc<Mutex<BoardInner>>,
}

impl JobBoard {
    /// 新建任务板。
    pub fn new(config: BoardConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BoardInner {
                config,
                next_id: 0,
                jobs: HashMap::new(),
                wake_gate: WakeGate::new(config.max_consecutive_wakes),
            })),
        }
    }

    /// 登记一个新任务: `owner` 持有句柄, `queued_segments` 个待启动段。
    pub fn open_job(&self, owner: OwnerId, queued_segments: SegmentIndex) -> JobHandle {
        let mut inner = self.lock();
        let id = JobId(inner.next_id);
        inner.next_id += 1;
        let capacity_bytes = inner.config.ring_capacity_bytes;
        inner.jobs.insert(
            id,
            JobRecord {
                owner: owner.clone(),
                ring: JobRing::new(capacity_bytes),
                segments: vec![SegmentState::Queued; queued_segments as usize],
                outcome: None,
                completion: Arc::new(Mutex::new(CompletionState::default())),
            },
        );
        JobHandle::new(id, owner)
    }

    /// 任务当前状态。
    pub fn status(&self, id: JobId) -> Result<JobStatus, BoardError> {
        let inner = self.lock();
        let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
        Ok(record
            .outcome
            .map_or(JobStatus::Running, |outcome| outcome.status))
    }

    /// 各段状态 (观察用)。
    pub fn segment_states(&self, id: JobId) -> Result<Vec<SegmentState>, BoardError> {
        let inner = self.lock();
        let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
        Ok(record.segments.clone())
    }

    /// 追加一段过程输出 (owner 校验), 返回起始绝对字节偏移。
    ///
    /// 已结算任务不接受新输出 —— 取消只影响未启动的段, 已产出输出保留在环内可读。
    pub fn append(&self, handle: &JobHandle, bytes: &[u8]) -> Result<u64, BoardError> {
        let mut inner = self.lock();
        let record = record_mut(&mut inner, handle)?;
        ensure_running(record, handle.id)?;
        Ok(record.ring.push(bytes))
    }

    /// 启动下一个未启动段 (owner 校验), 返回段序号; 无待启动段返回 `None`。
    pub fn start_next_segment(
        &self,
        handle: &JobHandle,
    ) -> Result<Option<SegmentIndex>, BoardError> {
        let mut inner = self.lock();
        let record = record_mut(&mut inner, handle)?;
        ensure_running(record, handle.id)?;
        for (index, state) in record.segments.iter_mut().enumerate() {
            if *state == SegmentState::Queued {
                *state = SegmentState::Started;
                return Ok(Some(index as SegmentIndex));
            }
        }
        Ok(None)
    }

    /// 完成一个已启动段 (owner 校验)。
    pub fn finish_segment(
        &self,
        handle: &JobHandle,
        segment: SegmentIndex,
    ) -> Result<(), BoardError> {
        let mut inner = self.lock();
        let record = record_mut(&mut inner, handle)?;
        ensure_running(record, handle.id)?;
        let state = record
            .segments
            .get_mut(segment as usize)
            .ok_or(BoardError::BadSegment {
                job: handle.id,
                segment,
            })?;
        if *state != SegmentState::Started {
            return Err(BoardError::BadSegment {
                job: handle.id,
                segment,
            });
        }
        *state = SegmentState::Finished;
        Ok(())
    }

    /// 完成结算 (owner 校验): 先释放等待方, 再发事件。
    pub fn settle(&self, handle: &JobHandle) -> Result<SettleReport, BoardError> {
        self.terminate(handle, JobStatus::Completed)
    }

    /// 取消结算 (owner 校验): **只影响未启动的段** (置 `Skipped`), 已启动段
    /// 不被追溯, 已产出的输出保留在环内可读。同样先释放等待方再发事件。
    pub fn cancel(&self, handle: &JobHandle) -> Result<SettleReport, BoardError> {
        self.terminate(handle, JobStatus::Cancelled)
    }

    /// 观察读取 (按 id): 从绝对字节偏移读到当前产出末尾。
    ///
    /// **绝不推进消费游标**, 观察者互不干扰。
    pub fn read_at(&self, id: JobId, offset: u64) -> Result<RingSlice, BoardError> {
        let inner = self.lock();
        let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
        Ok(record.ring.read_at(offset))
    }

    /// 消费读取 (按 id): 从消费游标读至多 `max_bytes` 并推进消费游标。
    pub fn consume(&self, id: JobId, max_bytes: usize) -> Result<RingSlice, BoardError> {
        let mut inner = self.lock();
        let record = inner.jobs.get_mut(&id).ok_or(BoardError::UnknownJob(id))?;
        Ok(record.ring.consume(max_bytes))
    }

    /// 环的留存与游标快照 (按 id)。
    pub fn ring_stats(&self, id: JobId) -> Result<RingStats, BoardError> {
        let inner = self.lock();
        let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
        Ok(record.ring.stats())
    }

    /// 登记完成等待方 (按 id)。结算前返回 `Pending`, 结算时被释放一次。
    pub fn wait_for_completion(&self, id: JobId) -> Result<CompletionWaiter, BoardError> {
        let inner = self.lock();
        let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
        let mut completion = lock(&record.completion);
        let slot = completion.waiters.len();
        completion.waiters.push(WaiterSlot {
            awaited: true,
            woken: false,
            waker: None,
        });
        Ok(CompletionWaiter {
            completion: Arc::clone(&record.completion),
            slot,
        })
    }

    /// 登记事件监听 (按 id)。结算时在等待方释放之后恰好调用一次; 登记时
    /// 已结算则立即调用一次。
    pub fn on_completion(&self, id: JobId, listener: CompletionListener) -> Result<(), BoardError> {
        let settled = {
            let inner = self.lock();
            let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
            let mut completion = lock(&record.completion);
            match completion.outcome {
                Some(outcome) => Some(outcome),
                None => {
                    completion.listeners.push(Arc::clone(&listener));
                    None
                }
            }
        };
        if let Some(outcome) = settled {
            listener(&outcome);
        }
        Ok(())
    }

    /// 记录归属方的真实进展: 连续 follow-up 唤醒计数清零。
    pub fn record_progress(&self, owner: &OwnerId) {
        self.lock().wake_gate.record_progress(owner);
    }

    /// 归属方当前连续无进展唤醒计数。
    pub fn consecutive_wakes(&self, owner: &OwnerId) -> u32 {
        self.lock().wake_gate.consecutive_wakes(owner)
    }

    /// 释放一个已结算的任务记录 (过程输出一并释放; 未结算任务拒绝)。
    pub fn remove_job(&self, id: JobId) -> Result<(), BoardError> {
        let mut inner = self.lock();
        let record = inner.jobs.get(&id).ok_or(BoardError::UnknownJob(id))?;
        if record.outcome.is_none() {
            return Err(BoardError::NotSettled(id));
        }
        inner.jobs.remove(&id);
        Ok(())
    }

    /// 终态转移: 记结算结果 → 先释放等待方 → 再发事件。
    ///
    /// 空闲 owner 的 follow-up 唤醒决定 (受 [`WakeGate`] 的
    /// `max_consecutive_wakes` 上限) 在结算记账时一并算出, 由报告返回。
    fn terminate(&self, handle: &JobHandle, status: JobStatus) -> Result<SettleReport, BoardError> {
        let (report, wakers, listeners) = {
            let mut inner = self.lock();
            terminate_record(&mut inner, handle, status)?
        };
        // 任务结算先释放等待方 (awaited 标记去重, 每个等待方至多一次)……
        for waker in wakers {
            waker.wake();
        }
        // ……再发事件 (每个监听器恰好一次)。
        for listener in &listeners {
            listener(&report.outcome);
        }
        Ok(report)
    }

    fn lock(&self) -> MutexGuard<'_, BoardInner> {
        lock(&self.inner)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn record_mut<'a>(
    inner: &'a mut BoardInner,
    handle: &JobHandle,
) -> Result<&'a mut JobRecord, BoardError> {
    let record = inner
        .jobs
        .get_mut(&handle.id)
        .ok_or(BoardError::UnknownJob(handle.id))?;
    if record.owner != handle.owner {
        return Err(BoardError::WrongOwner { job: handle.id });
    }
    Ok(record)
}

fn ensure_running(record: &JobRecord, id: JobId) -> Result<(), BoardError> {
    if record.outcome.is_some() {
        return Err(BoardError::AlreadySettled(id));
    }
    Ok(())
}

fn owner_is_idle(inner: &BoardInner, owner: &OwnerId) -> bool {
    !inner.jobs.values().any(|record| {
        record.outcome.is_none()
            && record.owner == *owner
            && record
                .segments
                .iter()
                .any(|state| *state == SegmentState::Started)
    })
}

/// 终态转移的核心记账: 未启动段置 Skipped、结算完成通知状态、收集待释放方,
/// 并为结算后空闲的 owner 算出 follow-up 唤醒决定 (受 max_consecutive_wakes 上限)。
fn terminate_record(
    inner: &mut BoardInner,
    handle: &JobHandle,
    status: JobStatus,
) -> Result<(SettleReport, Vec<Waker>, Vec<CompletionListener>), BoardError> {
    let owner = handle.owner.clone();
    let (outcome, waiters_released, wakers, listeners) = {
        let record = record_mut(inner, handle)?;
        ensure_running(record, handle.id)?;
        let mut skipped_segments = 0;
        for state in record.segments.iter_mut() {
            if *state == SegmentState::Queued {
                *state = SegmentState::Skipped;
                skipped_segments += 1;
            }
        }
        let outcome = JobOutcome {
            status,
            skipped_segments,
        };
        record.outcome = Some(outcome);
        let mut completion = lock(&record.completion);
        completion.outcome = Some(outcome);
        let mut wakers = Vec::new();
        let mut waiters_released = 0;
        for slot in completion.waiters.iter_mut() {
            if slot.awaited && !slot.woken {
                slot.woken = true;
                waiters_released += 1;
                if let Some(waker) = slot.waker.take() {
                    wakers.push(waker);
                }
            }
        }
        let listeners = if completion.notified {
            Vec::new()
        } else {
            completion.notified = true;
            std::mem::take(&mut completion.listeners)
        };
        (outcome, waiters_released, wakers, listeners)
    };
    let owner_wake = if owner_is_idle(inner, &owner) {
        Some(inner.wake_gate.try_wake(&owner))
    } else {
        None
    };
    let report = SettleReport {
        outcome,
        waiters_released,
        owner_wake,
    };
    Ok((report, wakers, listeners))
}

#[cfg(test)]
mod board_smoke_tests {
    use super::*;

    #[test]
    fn open_job_mints_distinct_ids_under_one_owner() {
        let board = JobBoard::new(BoardConfig::default());
        let first = board.open_job(OwnerId::new("worker"), 1);
        let second = board.open_job(OwnerId::new("worker"), 1);
        assert_ne!(first.id, second.id);
        assert_eq!(first.owner, second.owner);
    }
}
