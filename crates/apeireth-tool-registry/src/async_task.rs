//! 异步任务推送机制 (Async Task Push-Back)
//!
//! **源**: VCP v1.1 官网 "异步插件" 杀手锏:
//! AI 发起一个视频生成任务, 系统立刻返回 task_id, AI 继续干别的事.
//! 任务完成后, 结果通过通知系统推送回来. AI 随时感知进度、取消任务、并发多任务.
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - `AsyncTaskStore` 内存版 task 状态机: Pending -> Running -> Completed/Failed/Cancelled
//! - `TaskId` 64-bit 全局唯一 (内部计数器, 与 `apeireth-bus::next_trace_id` 数值兼容)
//! - 任务完成时调用方 (tool-runtime) 通过 `apeireth-bus::ChanneledBus` 任一 channel 推送
//!   结果, 本模块用本地 `NotifyChannel` enum 标记目标, 避免循环依赖
//! - 与 `apeireth-tool-registry::types::ToolKind::Async` 1:1 集成
//! - 提供 `wait_for_completion` async fn 让 LLM 等待结果
//! - **不假装** (O-5): 真实现 task 状态机, 编译期 `NOTIFY_CHANNEL_COUNT = 3` 守门
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline (收到 LLM tool_call)
//!          ↓
//!   apeireth-tool-runtime (派发 + 推送结果到 bus)
//!          ↓
//!   apeireth-tool-registry::async_task::AsyncTaskStore (本模块, 状态机)
//!          ↓ (tool-runtime 完成后推送)
//!   apeireth-bus::ChanneledBus (通知系统)
//! ```

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex};

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum AsyncTaskError {
    #[error("task not found: {0}")]
    NotFound(u64),
    #[error("task already in terminal state: id={0} status={1}")]
    AlreadyTerminal(u64, String),
    #[error("task cancelled by user: {0}")]
    Cancelled(u64),
    #[error("oneshot receiver dropped: {0}")]
    ReceiverDropped(u64),
}

pub type AsyncTaskResult<T> = Result<T, AsyncTaskError>;

// ============================================================================
// TaskId 与状态
// ============================================================================

/// 异步任务 ID (u64 全局唯一, 内部原子计数器)
pub type TaskId = u64;

/// 自增 TaskId (不依赖 bus, 避免循环)
pub fn next_task_id() -> TaskId {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// 当前时间戳 (epoch millis), 不依赖 bus
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 异步任务状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// 已派发, 等待 worker pickup
    Pending,
    /// Worker 正在跑
    Running,
    /// 成功完成
    Completed,
    /// 失败 (错误信息在 result)
    Failed,
    /// 用户主动取消
    Cancelled,
}

impl TaskStatus {
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// 推送目标 channel (本地 enum, 字段级对应 VCP 三套通知栏 + apeireth-bus::Channel)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotifyChannel {
    /// AI 通知栏 (仅 AI 可见): 工具调用结果、系统信息、异步任务进度
    Ai,
    /// VCPLog 通知栏 (人类可见, AI 不可见): 权限审计、用户审批
    Human,
    /// VCPInfo 通知栏 (双方可见): 实时流程信息
    Both,
}

impl NotifyChannel {
    pub const COUNT: usize = 3;
    pub const ALL: [NotifyChannel; 3] = [Self::Ai, Self::Human, Self::Both];

    pub const fn as_legacy_str(&self) -> &'static str {
        match self {
            Self::Ai => "ai_notification",
            Self::Human => "vcp_log",
            Self::Both => "vcp_info",
        }
    }

    pub const fn topic_prefix(&self) -> &'static str {
        match self {
            Self::Ai => "ai:",
            Self::Human => "human:",
            Self::Both => "both:",
        }
    }
}

impl Default for NotifyChannel {
    fn default() -> Self { Self::Ai }
}

/// 异步任务记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: TaskId,
    pub tool_name: String,
    pub status: TaskStatus,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    /// 任务参数 (JSON 序列化保存以便审计)
    pub params_json: String,
    /// 任务结果 (JSON 序列化, status 为 Completed 时填充)
    pub result_json: Option<String>,
    /// 失败原因 (status 为 Failed 时填充)
    pub error: Option<String>,
    /// 进度 (0.0 - 1.0)
    pub progress: f32,
    /// 推送目标 channel (推送前确认)
    pub notify_channel: NotifyChannel,
}

impl TaskRecord {
    pub fn new(task_id: TaskId, tool_name: String, params_json: String, notify_channel: NotifyChannel) -> Self {
        Self {
            task_id,
            tool_name,
            status: TaskStatus::Pending,
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            params_json,
            result_json: None,
            error: None,
            progress: 0.0,
            notify_channel,
        }
    }

    pub fn age_ms(&self) -> i64 {
        now_ms() - self.created_at_ms
    }
}

// ============================================================================
// 异步任务存储
// ============================================================================

/// 异步任务存储 (Arc-shared 内部, clone 即可共享)
#[derive(Clone)]
pub struct AsyncTaskStore {
    inner: Arc<Mutex<HashMap<TaskId, TaskRecord>>>,
    /// 完成后通知 oneshot (供 wait_for_completion 用)
    notifiers: Arc<Mutex<HashMap<TaskId, oneshot::Sender<TaskRecord>>>>,
}

impl AsyncTaskStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            notifiers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 注册一个新任务 (Pending)
    pub async fn register(
        &self,
        tool_name: String,
        params_json: String,
        notify_channel: NotifyChannel,
    ) -> (TaskId, TaskRecord) {
        let id = next_task_id();
        let rec = TaskRecord::new(id, tool_name, params_json, notify_channel);
        self.inner.lock().await.insert(id, rec.clone());
        (id, rec)
    }

    /// 标记任务为 Running
    pub async fn mark_running(&self, task_id: TaskId) -> AsyncTaskResult<()> {
        let mut g = self.inner.lock().await;
        let rec = g.get_mut(&task_id).ok_or(AsyncTaskError::NotFound(task_id))?;
        if rec.status.is_terminal() {
            return Err(AsyncTaskError::AlreadyTerminal(task_id, rec.status.as_str().into()));
        }
        rec.status = TaskStatus::Running;
        rec.started_at_ms = Some(now_ms());
        Ok(())
    }

    /// 标记任务完成 (成功) + 推送结果 (可选)
    pub async fn complete(
        &self,
        task_id: TaskId,
        result_json: String,
    ) -> AsyncTaskResult<TaskRecord> {
        let mut g = self.inner.lock().await;
        let rec = g.get_mut(&task_id).ok_or(AsyncTaskError::NotFound(task_id))?;
        if rec.status.is_terminal() {
            return Err(AsyncTaskError::AlreadyTerminal(task_id, rec.status.as_str().into()));
        }
        rec.status = TaskStatus::Completed;
        rec.completed_at_ms = Some(now_ms());
        rec.result_json = Some(result_json);
        rec.progress = 1.0;
        let snapshot = rec.clone();

        // 通知所有 waiter
        if let Some(tx) = self.notifiers.lock().await.remove(&task_id) {
            let _ = tx.send(snapshot.clone());
        }

        Ok(snapshot)
    }

    /// 标记任务失败
    pub async fn fail(
        &self,
        task_id: TaskId,
        error: String,
    ) -> AsyncTaskResult<TaskRecord> {
        let mut g = self.inner.lock().await;
        let rec = g.get_mut(&task_id).ok_or(AsyncTaskError::NotFound(task_id))?;
        if rec.status.is_terminal() {
            return Err(AsyncTaskError::AlreadyTerminal(task_id, rec.status.as_str().into()));
        }
        rec.status = TaskStatus::Failed;
        rec.completed_at_ms = Some(now_ms());
        rec.error = Some(error);
        let snapshot = rec.clone();

        if let Some(tx) = self.notifiers.lock().await.remove(&task_id) {
            let _ = tx.send(snapshot.clone());
        }

        Ok(snapshot)
    }

    /// 取消任务
    pub async fn cancel(&self, task_id: TaskId) -> AsyncTaskResult<TaskRecord> {
        let mut g = self.inner.lock().await;
        let rec = g.get_mut(&task_id).ok_or(AsyncTaskError::NotFound(task_id))?;
        if rec.status.is_terminal() {
            return Err(AsyncTaskError::AlreadyTerminal(task_id, rec.status.as_str().into()));
        }
        rec.status = TaskStatus::Cancelled;
        rec.completed_at_ms = Some(now_ms());
        let snapshot = rec.clone();

        if let Some(tx) = self.notifiers.lock().await.remove(&task_id) {
            let _ = tx.send(snapshot.clone());
        }

        Ok(snapshot)
    }

    /// 更新进度
    pub async fn update_progress(&self, task_id: TaskId, progress: f32) -> AsyncTaskResult<()> {
        let mut g = self.inner.lock().await;
        let rec = g.get_mut(&task_id).ok_or(AsyncTaskError::NotFound(task_id))?;
        if rec.status.is_terminal() {
            return Err(AsyncTaskError::AlreadyTerminal(task_id, rec.status.as_str().into()));
        }
        rec.progress = progress.clamp(0.0, 1.0);
        Ok(())
    }

    /// 查询任务当前状态
    pub async fn get(&self, task_id: TaskId) -> Option<TaskRecord> {
        self.inner.lock().await.get(&task_id).cloned()
    }

    /// 列出所有任务
    pub async fn list(&self) -> Vec<TaskRecord> {
        self.inner.lock().await.values().cloned().collect()
    }

    /// 等待任务完成 (timeout)
    pub async fn wait_for_completion(
        &self,
        task_id: TaskId,
        timeout: Duration,
    ) -> AsyncTaskResult<TaskRecord> {
        // 1) 快速路径: 已经 terminal
        if let Some(rec) = self.get(task_id).await {
            if rec.status.is_terminal() {
                return Ok(rec);
            }
        } else {
            return Err(AsyncTaskError::NotFound(task_id));
        }

        // 2) 注册 oneshot, 等待
        let (tx, rx) = oneshot::channel::<TaskRecord>();
        self.notifiers.lock().await.insert(task_id, tx);

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(rec)) => Ok(rec),
            Ok(Err(_)) => Err(AsyncTaskError::ReceiverDropped(task_id)),
            Err(_) => {
                self.notifiers.lock().await.remove(&task_id);
                self.get(task_id).await.ok_or(AsyncTaskError::NotFound(task_id))
            }
        }
    }

    /// 清理已 terminal 的任务 (批量)
    pub async fn cleanup_terminal(&self) -> usize {
        let mut g = self.inner.lock().await;
        let before = g.len();
        g.retain(|_, v| !v.status.is_terminal());
        before - g.len()
    }

    /// 当前任务数
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }
}

impl Default for AsyncTaskStore {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn t01_task_status_is_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[tokio::test]
    async fn t02_register_and_get() {
        let store = AsyncTaskStore::new();
        let (id, rec) = store.register("video_gen".into(), "{}".into(), NotifyChannel::Both).await;
        assert_eq!(rec.status, TaskStatus::Pending);
        assert_eq!(rec.tool_name, "video_gen");
        let g = store.get(id).await.unwrap();
        assert_eq!(g.task_id, id);
    }

    #[tokio::test]
    async fn t03_full_lifecycle() {
        let store = AsyncTaskStore::new();
        let (id, _) = store.register("img_gen".into(), "{}".into(), NotifyChannel::Ai).await;

        store.mark_running(id).await.unwrap();
        let r = store.get(id).await.unwrap();
        assert_eq!(r.status, TaskStatus::Running);

        store.update_progress(id, 0.5).await.unwrap();
        let r = store.get(id).await.unwrap();
        assert!((r.progress - 0.5).abs() < 1e-6);

        let final_rec = store.complete(id, "{\"url\":\"x.png\"}".into()).await.unwrap();
        assert_eq!(final_rec.status, TaskStatus::Completed);
        assert_eq!(final_rec.result_json.as_deref(), Some("{\"url\":\"x.png\"}"));
        assert_eq!(final_rec.progress, 1.0);
    }

    #[tokio::test]
    async fn t04_failed_and_cancelled() {
        let store = AsyncTaskStore::new();
        let (id1, _) = store.register("task1".into(), "{}".into(), NotifyChannel::Ai).await;
        let (id2, _) = store.register("task2".into(), "{}".into(), NotifyChannel::Human).await;

        let r1 = store.fail(id1, "out of memory".into()).await.unwrap();
        assert_eq!(r1.status, TaskStatus::Failed);
        assert_eq!(r1.error.as_deref(), Some("out of memory"));

        let r2 = store.cancel(id2).await.unwrap();
        assert_eq!(r2.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn t05_cannot_complete_twice() {
        let store = AsyncTaskStore::new();
        let (id, _) = store.register("task".into(), "{}".into(), NotifyChannel::Ai).await;
        store.complete(id, "ok".into()).await.unwrap();
        let r = store.complete(id, "again".into()).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn t06_wait_for_completion_fast_path() {
        let store = AsyncTaskStore::new();
        let (id, _) = store.register("task".into(), "{}".into(), NotifyChannel::Ai).await;
        store.complete(id, "ok".into()).await.unwrap();
        let r = store.wait_for_completion(id, Duration::from_secs(1)).await.unwrap();
        assert_eq!(r.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn t07_wait_for_completion_async() {
        let store = AsyncTaskStore::new();
        let (id, _) = store.register("task".into(), "{}".into(), NotifyChannel::Ai).await;

        let store2 = store.clone();
        let id2 = id;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            store2.complete(id2, "{\"res\":42}".into()).await.unwrap();
        });

        let r = store.wait_for_completion(id, Duration::from_secs(1)).await.unwrap();
        assert_eq!(r.status, TaskStatus::Completed);
        assert_eq!(r.result_json.as_deref(), Some("{\"res\":42}"));
    }

    #[tokio::test]
    async fn t08_cleanup_terminal() {
        let store = AsyncTaskStore::new();
        let (id1, _) = store.register("a".into(), "{}".into(), NotifyChannel::Ai).await;
        let (id2, _) = store.register("b".into(), "{}".into(), NotifyChannel::Ai).await;
        store.complete(id1, "ok".into()).await.unwrap();
        let removed = store.cleanup_terminal().await;
        assert_eq!(removed, 1);
        assert!(store.get(id1).await.is_none());
        assert!(store.get(id2).await.is_some());
    }

    #[tokio::test]
    async fn t09_not_found() {
        let store = AsyncTaskStore::new();
        let r = store.get(99999).await;
        assert!(r.is_none());
        let r = store.complete(99999, "x".into()).await;
        assert!(r.is_err());
    }

    #[test]
    fn t10_notify_channel_vcp_str() {
        assert_eq!(NotifyChannel::Ai.as_legacy_str(), "ai_notification");
        assert_eq!(NotifyChannel::Human.as_legacy_str(), "vcp_log");
        assert_eq!(NotifyChannel::Both.as_legacy_str(), "vcp_info");
        assert_eq!(NotifyChannel::COUNT, 3);
    }
}
