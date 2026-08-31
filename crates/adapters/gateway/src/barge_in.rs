//! `apeireth-gateway::barge_in` — 全双工实时流式打断与插话控制器 (Barge-in / Voice Interruption).
//!
//! ## 核心哲学 (基于全双工语音伴侣架构)
//! 在全双工实时交互（尤其是桌面端语音与 SSE 流式生成）中，用户不应被迫等待模型完全说完整段话：
//! 一旦用户重新开口插话 (VAD 触发) 或发出取消指令，系统必须在毫秒级内广播取消信号，
//! 阻断服务端的模型推理、TTS 生成与流传输，并向客户端推送 `event: interrupt` 帧，
//! 实现拟真真人的即时双向打断与低延迟交互。
//!
//! ## 安全与并发
//! - 纯 Safe Rust 实现 (`#![deny(unsafe_code)]`)，0 未定义行为；
//! - 基于原子布尔量 (`AtomicBool`) 与异步信号灯 (`tokio::sync::Notify`) 实现无锁/极轻并发通知；
//! - 会话隔离，每个 `session_id` 独享生命周期上下文，自动防止资源泄漏。

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Default)]
pub struct BargeInController {
    sessions: Arc<Mutex<HashMap<String, StreamHandle>>>,
}

impl BargeInController {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
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
    pub fn interrupt(&self, session_id: &str, reason: InterruptReason) -> bool {
        let lock = self.sessions.lock().unwrap();
        if let Some(handle) = lock.get(session_id) {
            handle.is_interrupted.store(true, Ordering::SeqCst);
            *handle.reason.lock().unwrap() = Some(reason);
            handle.notify.notify_waiters();
            true
        } else {
            false
        }
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
}
