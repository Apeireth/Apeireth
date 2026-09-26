//! `apeireth-gateway::barge_in` — 全双工实时流式打断与插话控制器 (Barge-in / Voice Interruption).
//!
//! ## 核心哲学 (借鉴 Open-LLM-VTuber 与现代语音伴侣架构)
//! 在全双工实时交互（尤其是桌面端语音与 SSE 流式生成）中，用户不应被迫等待模型完全说完整段话：
//! 一旦用户重新开口插话 (VAD 触发) 或发出取消指令，系统必须在毫秒级内广播取消信号，
//! 阻断服务端的模型推理、TTS 生成与流传输，并向客户端推送 `event: interrupt` 帧，
//! 实现拟真真人的即时双向打断与低延迟交互。
//!
//! ## 安全与并发
//! - 纯 Safe Rust 实现 (`#![deny(unsafe_code)]`)，0 未定义行为；
//! - 基于原子布尔量 (`AtomicBool`) 与异步信号灯 (`tokio::sync::Notify`) 实现无锁/极轻并发通知；
//! - 会话隔离，每个 `session_id` 独享生命周期上下文，自动防止资源泄漏。
//!
//! ## 打断流租约面 (资源租约语义推广)
//! 打断通知流是一等订阅面 ([`BargeInController::open_interrupt_stream`]):
//! 首订阅者开流、末订阅者关停, pin 保活; 后订阅者靠快照即读最新打断帧
//! (不回放历史), 打空 (无活跃会话) 成为挂帧的失败帧 (失败即帧),
//! 面关停后无幽灵回调。

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use apeireth_core::resource_lease::{
    ResourceError, ResourceFrame, ResourceHooks, ResourceLease, ResourceRegistry,
};

/// Surface name of the interrupt notice frame stream.
const INTERRUPT_SURFACE: &str = "barge_in_interrupts";

/// 打断原因分类.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptReason {
    /// 用户语音插话 (麦克风 VAD 检测到用户重新发声).
    VoiceBargeIn,
    /// 用户手动取消 (前端点击停止生成或按下 ESC / 热键).
    UserManualCancel,
    /// 新轮次抢占 (同一会话快速收到新的用户请求).
    NewTurnPreempt,
    /// 超时保护 (流式生成超过最大安全阈值强制回收).
    Timeout,
}

impl InterruptReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VoiceBargeIn => "voice_barge_in",
            Self::UserManualCancel => "user_manual_cancel",
            Self::NewTurnPreempt => "new_turn_preempt",
            Self::Timeout => "timeout",
        }
    }
}

/// 单个流式会话的打断句柄.
#[derive(Debug, Clone)]
pub struct StreamHandle {
    pub session_id: String,
    started_at_ms: i64,
    is_interrupted: Arc<AtomicBool>,
    reason: Arc<Mutex<Option<InterruptReason>>>,
    notify: Arc<tokio::sync::Notify>,
}

impl StreamHandle {
    /// 检查当前流是否已被打断.
    pub fn is_interrupted(&self) -> bool {
        self.is_interrupted.load(Ordering::SeqCst)
    }

    /// 获取打断原因 (若未被打断则返回 None).
    pub fn reason(&self) -> Option<InterruptReason> {
        *self.reason.lock().unwrap()
    }

    /// 异步等待打断信号到来 (可配合 tokio::select! 实现毫秒级流取消).
    pub async fn wait_for_interrupt(&self) {
        if self.is_interrupted() {
            return;
        }
        self.notify.notified().await;
    }

    /// 流开启至今经过的毫秒数.
    pub fn elapsed_ms(&self) -> i64 {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        now_ms.saturating_sub(self.started_at_ms).max(0)
    }
}

/// 全双工打断控制器.
#[derive(Debug, Clone)]
pub struct BargeInController {
    sessions: Arc<Mutex<HashMap<String, StreamHandle>>>,
    interrupt_frames: Arc<ResourceRegistry<InterruptNotice>>,
    interrupt_stream: Arc<InterruptStreamDriver>,
}

impl Default for BargeInController {
    fn default() -> Self {
        Self::new()
    }
}

/// 一帧打断事实: 谁、为何、何时被打断 (打断流的帧面值).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InterruptNotice {
    /// 被打断的流会话.
    pub session_id: String,
    /// 打断原因.
    pub reason: InterruptReason,
    /// Epoch milliseconds.
    pub at_ms: i64,
}

/// 打断流生命周期记录: 首订阅者开流、末订阅者关停。打断帧由 `interrupt()`
/// 推送 (无独立产出循环), 故钩子记录交付生命周期, 供接线与测试观察。
#[derive(Debug, Default)]
struct InterruptStreamDriver {
    streaming: AtomicBool,
    opens: AtomicUsize,
    closes: AtomicUsize,
}

impl InterruptStreamDriver {
    fn on_open(&self) -> Result<(), String> {
        self.streaming.store(true, Ordering::SeqCst);
        self.opens.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn on_close(&self) {
        self.streaming.store(false, Ordering::SeqCst);
        self.closes.fetch_add(1, Ordering::SeqCst);
    }
}

/// 打断流上的一枚订阅租约: 持有期间打断流开启, 全部释放即关停。
pub struct InterruptStreamSubscription {
    _lease: ResourceLease,
    frames: Arc<ResourceRegistry<InterruptNotice>>,
}

impl std::fmt::Debug for InterruptStreamSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterruptStreamSubscription")
            .field("surface", &self.frames)
            .finish()
    }
}

impl InterruptStreamSubscription {
    /// 快照即读: 最新打断帧 + 挂着的失败帧, 不回放历史。
    pub fn snapshot(&self) -> ResourceFrame<InterruptNotice> {
        self.frames.snapshot()
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl BargeInController {
    pub fn new() -> Self {
        let stream = Arc::new(InterruptStreamDriver::default());
        let open_driver = Arc::clone(&stream);
        let close_driver = Arc::clone(&stream);
        let interrupt_frames = ResourceRegistry::new(
            INTERRUPT_SURFACE,
            ResourceHooks::new(
                Arc::new(move || open_driver.on_open()),
                Arc::new(move || close_driver.on_close()),
            ),
        );
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            interrupt_frames,
            interrupt_stream: stream,
        }
    }

    /// 订阅打断流: 首订阅者开流, 末订阅者 (租约或 pin 全体) 关停。
    pub fn open_interrupt_stream(&self) -> Result<InterruptStreamSubscription, ResourceError> {
        let lease = self.interrupt_frames.acquire()?;
        Ok(InterruptStreamSubscription {
            _lease: lease,
            frames: Arc::clone(&self.interrupt_frames),
        })
    }

    /// 快照即读: 打断流当前帧 (最新打断 + 挂着的失败帧), 不回放历史。
    pub fn interrupt_snapshot(&self) -> ResourceFrame<InterruptNotice> {
        self.interrupt_frames.snapshot()
    }

    /// 打断流是否开启。
    pub fn is_interrupt_stream_open(&self) -> bool {
        self.interrupt_frames.is_open()
    }

    /// 打断流生命周期计数 (开/关钩子触发次数)。
    pub fn interrupt_stream_lifecycle_counts(&self) -> (usize, usize) {
        (
            self.interrupt_stream.opens.load(Ordering::SeqCst),
            self.interrupt_stream.closes.load(Ordering::SeqCst),
        )
    }

    /// 关停打断流: 不再发租约、不再收帧, 事后释放的租约不补调关停钩子
    /// (关停后无幽灵回调)。
    pub fn shutdown_interrupt_stream(&self) {
        self.interrupt_frames.shutdown();
    }

    /// 注册一个活跃流会话并获取监听句柄.
    /// 若存在同名旧会话，会自动触发 `NewTurnPreempt` 抢占旧会话.
    pub fn register_stream(&self, session_id: &str) -> StreamHandle {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let handle = StreamHandle {
            session_id: session_id.to_string(),
            started_at_ms: now_ms,
            is_interrupted: Arc::new(AtomicBool::new(false)),
            reason: Arc::new(Mutex::new(None)),
            notify: Arc::new(tokio::sync::Notify::new()),
        };

        let mut lock = self.sessions.lock().unwrap();
        if let Some(old_handle) = lock.insert(session_id.to_string(), handle.clone()) {
            // 抢占旧会话
            old_handle.is_interrupted.store(true, Ordering::SeqCst);
            *old_handle.reason.lock().unwrap() = Some(InterruptReason::NewTurnPreempt);
            old_handle.notify.notify_waiters();
        }

        handle
    }

    /// 触发指定会话的插话打断.
    /// 返回 true 表示成功命中并打断活跃流；false 表示该会话不存在或已结束.
    ///
    /// 帧面语义 (失败即帧): 命中即把打断事实入帧面 (后订阅者快照即读最新);
    /// 打空不再只是 false —— 它成为挂在最后值旁的显式失败帧, 下一命中自清。
    pub fn interrupt(&self, session_id: &str, reason: InterruptReason) -> bool {
        let hit = {
            let lock = self.sessions.lock().unwrap();
            if let Some(handle) = lock.get(session_id) {
                handle.is_interrupted.store(true, Ordering::SeqCst);
                *handle.reason.lock().unwrap() = Some(reason);
                handle.notify.notify_waiters();
                true
            } else {
                false
            }
        };
        if hit {
            let _ = self.interrupt_frames.publish(InterruptNotice {
                session_id: session_id.to_string(),
                reason,
                at_ms: now_ms(),
            });
        } else {
            let _ = self
                .interrupt_frames
                .publish_failure(format!("no active stream for session {session_id}"));
        }
        hit
    }

    /// 检查指定会话是否已被打断.
    pub fn is_interrupted(&self, session_id: &str) -> bool {
        let lock = self.sessions.lock().unwrap();
        lock.get(session_id).map_or(false, |h| h.is_interrupted())
    }

    /// 清理并注销已完成的会话.
    pub fn cleanup(&self, session_id: &str) {
        let mut lock = self.sessions.lock().unwrap();
        lock.remove(session_id);
    }

    /// 当前处于活跃监听状态的会话总数.
    pub fn active_sessions_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }
}

/// 格式化为标准 SSE 打断帧数据 (供 Gateway SSE 发送给客户端).
pub fn format_sse_interrupt_event(
    session_id: &str,
    reason: InterruptReason,
    char_offset: usize,
) -> String {
    let payload = serde_json::json!({
        "session_id": session_id,
        "interrupted": true,
        "reason": reason.as_str(),
        "char_offset": char_offset,
    });
    format!("event: interrupt\ndata: {}\n\n", payload)
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_register_and_manual_cancel() {
        let controller = BargeInController::new();
        let handle = controller.register_stream("session_123");

        assert_eq!(handle.session_id, "session_123");
        assert!(!handle.is_interrupted());
        assert_eq!(controller.active_sessions_count(), 1);

        // 触发手动取消
        let hit = controller.interrupt("session_123", InterruptReason::UserManualCancel);
        assert!(hit);
        assert!(handle.is_interrupted());
        assert_eq!(handle.reason(), Some(InterruptReason::UserManualCancel));

        // 清理
        controller.cleanup("session_123");
        assert_eq!(controller.active_sessions_count(), 0);
    }

    #[tokio::test]
    async fn test_voice_barge_in_async_notification() {
        let controller = BargeInController::new();
        let handle = controller.register_stream("voice_sess_1");

        let handle_clone = handle.clone();
        let waiter_task = tokio::spawn(async move {
            handle_clone.wait_for_interrupt().await;
            true
        });

        // 模拟语音插话
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        controller.interrupt("voice_sess_1", InterruptReason::VoiceBargeIn);

        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(200), waiter_task).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), true);
        assert_eq!(handle.reason(), Some(InterruptReason::VoiceBargeIn));
    }

    #[tokio::test]
    async fn test_new_turn_preempts_previous_stream() {
        let controller = BargeInController::new();
        let handle_turn_1 = controller.register_stream("user_session");
        assert!(!handle_turn_1.is_interrupted());

        // 用户在上一轮尚未生成完时直接发送新问题
        let handle_turn_2 = controller.register_stream("user_session");

        // 验证旧会话被自动抢占打断
        assert!(handle_turn_1.is_interrupted());
        assert_eq!(
            handle_turn_1.reason(),
            Some(InterruptReason::NewTurnPreempt)
        );

        // 验证新会话正常运行中
        assert!(!handle_turn_2.is_interrupted());
        assert_eq!(controller.active_sessions_count(), 1);
    }

    #[test]
    fn test_format_sse_interrupt_event() {
        let sse = format_sse_interrupt_event("sess_abc", InterruptReason::VoiceBargeIn, 42);
        assert!(sse.starts_with("event: interrupt\ndata: "));
        assert!(sse.contains("\"session_id\":\"sess_abc\""));
        assert!(sse.contains("\"reason\":\"voice_barge_in\""));
        assert!(sse.contains("\"char_offset\":42"));
        assert!(sse.ends_with("\n\n"));
    }

    // 打断流租约面 (首开末关 / 快照即读 / 失败即帧 / 无幽灵)

    #[test]
    fn interrupt_stream_opens_with_the_first_subscriber_and_closes_with_the_last() {
        let controller = BargeInController::new();
        assert!(
            !controller.is_interrupt_stream_open(),
            "无订阅者时打断流不开"
        );

        let first = controller.open_interrupt_stream().expect("subscription");
        assert!(controller.is_interrupt_stream_open(), "首订阅者开流");
        assert_eq!(controller.interrupt_stream_lifecycle_counts(), (1, 0));

        let second = controller.open_interrupt_stream().expect("subscription");
        assert_eq!(
            controller.interrupt_stream_lifecycle_counts(),
            (1, 0),
            "共享不重复开流"
        );

        drop(first);
        assert!(controller.is_interrupt_stream_open(), "仍有订阅者, 流不关");
        assert_eq!(controller.interrupt_stream_lifecycle_counts(), (1, 0));

        drop(second);
        assert!(!controller.is_interrupt_stream_open(), "末订阅者关停");
        assert_eq!(
            controller.interrupt_stream_lifecycle_counts(),
            (1, 1),
            "关停恰好一次"
        );
    }

    #[test]
    fn late_subscriber_reads_the_latest_interrupt_from_the_snapshot_without_replay() {
        let controller = BargeInController::new();
        let _first = controller.open_interrupt_stream().expect("subscription");
        controller.register_stream("sess_a");
        controller.register_stream("sess_b");

        assert!(controller.interrupt("sess_a", InterruptReason::UserManualCancel));
        assert!(controller.interrupt("sess_b", InterruptReason::VoiceBargeIn));

        // 后订阅者: 快照即读最新打断帧, 不回放 sess_a 的历史帧。
        let late = controller.open_interrupt_stream().expect("subscription");
        let frame = late.snapshot();
        let notice = frame.value.as_ref().expect("latest interrupt");
        assert_eq!(notice.session_id, "sess_b");
        assert_eq!(notice.reason, InterruptReason::VoiceBargeIn);
        assert!(!frame.has_failure());
    }

    #[test]
    fn interrupt_miss_becomes_a_failure_frame_and_the_next_hit_clears_it() {
        let controller = BargeInController::new();
        let _sub = controller.open_interrupt_stream().expect("subscription");
        controller.register_stream("sess_live");

        // 打空 = 显式失败帧 (不静默), 且不清最后事实。
        assert!(!controller.interrupt("sess_ghost", InterruptReason::Timeout));
        let frame = controller.interrupt_snapshot();
        assert_eq!(
            frame.failure.as_deref(),
            Some("no active stream for session sess_ghost")
        );

        // 下一命中自清失败帧, 打断事实照常入面。
        assert!(controller.interrupt("sess_live", InterruptReason::NewTurnPreempt));
        let frame = controller.interrupt_snapshot();
        assert!(!frame.has_failure(), "下一成功帧自清失败帧");
        assert_eq!(frame.value.expect("notice").session_id, "sess_live");
    }

    #[test]
    fn shut_down_interrupt_stream_fires_no_ghost_callbacks() {
        let controller = BargeInController::new();
        let sub = controller.open_interrupt_stream().expect("subscription");
        controller.register_stream("sess_x");
        assert!(controller.interrupt("sess_x", InterruptReason::VoiceBargeIn));
        let frozen = controller.interrupt_snapshot();

        controller.shutdown_interrupt_stream();
        assert!(!controller.is_interrupt_stream_open(), "关停即关流");
        let counts = controller.interrupt_stream_lifecycle_counts();

        // 幽灵检查: 事后释放的订阅不得再触发关停钩子, 帧面不得再更新。
        drop(sub);
        assert_eq!(
            controller.interrupt_stream_lifecycle_counts(),
            counts,
            "关停后无幽灵回调"
        );
        assert!(
            controller.open_interrupt_stream().is_err(),
            "关停后不再发放租约"
        );
        assert!(controller.interrupt("sess_x", InterruptReason::Timeout));
        assert_eq!(controller.interrupt_snapshot(), frozen, "关停后无幽灵帧");
    }
}
