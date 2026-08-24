//! SessionManager - 单 host 维度的会话生命周期管理
//!
//! 0 装 PASS: 从 UnifiedRuntimeHost (host.rs:52, 62) 抽取，保持原 HashMap<session_id, SessionState>
//! 行为完全一致；新增 helper API 让调用方无需直接 lock Mutex。
//!
//! 0-breaking: 字段类型 + 方法签名保兼容；调用处只需 `self.sessions.lock()` → `self.session_manager.with_mut()`.

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_protocol::normalized::NormalizedMessage;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

/// 单个会话的完整状态。
///
/// 0 装 PASS: 该结构定义与抽取前完全一致；只追加 derive Clone + Debug (原 host.rs 是手工 pub)。
#[derive(Debug, Clone)]
pub struct SessionState {
    pub id: String,
    pub messages: Vec<NormalizedMessage>,
    pub created_at: DateTime<Utc>,
    pub last_interaction: DateTime<Utc>,
}

impl SessionState {
    /// 0 装 PASS: 与 host.rs::handle_chat_turn 中 or_insert_with 块的行为 1:1 一致
    pub fn new(id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            messages: Vec::new(),
            created_at: now,
            last_interaction: now,
        }
    }

    /// 追加 message + 更新 last_interaction (与原 host.rs:301 行为一致)
    pub fn append_message(&mut self, msg: NormalizedMessage) {
        self.messages.push(msg);
        self.last_interaction = Utc::now();
    }

    /// 返回最近 N 条消息（保留最近上下文）；N=0 返回全部。
    /// 0 装 PASS: 与 host.rs:296 行为一致（messages.extend_from_slice(&session.messages[start..])）
    pub fn recent_messages(&self, n: usize) -> &[NormalizedMessage] {
        if n == 0 || self.messages.len() <= n {
            return &self.messages;
        }
        let start = self.messages.len() - n;
        &self.messages[start..]
    }
}

/// 会话注册表 + 生命周期管理。
///
/// 0 装 PASS: 内部用 tokio::sync::Mutex 与原 host.rs Arc<Mutex<HashMap>> 一致；
/// 暴露 `get_or_create` / `get` / `with_mut` 让调用方无需关心 lock。
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { sessions: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// 获取或创建（与 host.rs:294 or_insert_with 一致）
    pub async fn get_or_create(&self, session_id: &str) -> SessionState {
        let mut sessions = self.sessions.lock().await;
        sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionState::new(session_id.to_string()))
            .clone()
    }

    /// 仅获取，不存在返 None
    pub async fn get(&self, session_id: &str) -> Option<SessionState> {
        self.sessions.lock().await.get(session_id).cloned()
    }

    /// 可变访问（追加 message 等）；闭包返回 Option 是为了"未找到则不创建"
    pub async fn with_mut<F, R>(&self, session_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut SessionState) -> R,
    {
        let mut sessions = self.sessions.lock().await;
        sessions.get_mut(session_id).map(f)
    }

    /// 当前会话总数（observability 用）
    pub async fn len(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// 0 装 PASS: 同步迭代 (gateway panel 用);短暂持锁 + 克隆避免跨 await 持锁。
    /// 返回 Vec<(String, SessionState)> 一次性快照, 调用方无需自己 lock。
    pub async fn snapshot(&self) -> Vec<(String, SessionState)> {
        self.sessions
            .lock()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Default for SessionManager {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_create_get() {
        let mgr = SessionManager::new();
        let s = mgr.get_or_create("sess-1").await;
        assert_eq!(s.id, "sess-1");
        assert_eq!(s.messages.len(), 0);
        assert_eq!(mgr.len().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_recent_messages() {
        let mgr = SessionManager::new();
        let _ = mgr.get_or_create("sess-2").await;
        for i in 0..5 {
            mgr.with_mut("sess-2", |s| {
                s.append_message(NormalizedMessage::user(format!("msg-{}", i)));
            }).await;
        }
        let s = mgr.get("sess-2").await.unwrap();
        let recent = s.recent_messages(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].extract_text(), "msg-2");
        assert_eq!(recent[2].extract_text(), "msg-4");
    }
}
