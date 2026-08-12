//! R150 P1 #10: apeireth-council::session_capture — Council session 自动捕获
//!
//! **借鉴 ID**: `R150-COUNCIL-BORROW-claude-mem-24k-stars-2026-08-13`
//!
//! **claude-mem 模式**: session 生命周期内自动捕获 council deliberation messages,
//! session 结束时归约 + 持久化, 跨 session 可检索.
//!
//! **0 触碰** 既有 `deliberation` / `council_member` / `synthesis` (per R149 P1 #10 完成定义)
//! **0 引外部 dep** (claude-mem 借鉴语义, 0 装 PASS)
//!
//! **设计**:
//! - `CouncilSession { session_id, started_at, ended_at, messages: Vec<SessionMessage> }`
//! - `SessionMessage { seq, advisor_id, role, content, timestamp_ms }`
//! - `SessionCapture { current, history }` — 当前 session + 历史 session list
//! - `start_session()` / `end_session()` / `record_message()` 三个核心 API
//! - 自动捕获: 任何 `record_*` 调用都 append 到 current session (若无则 no-op)
//! - 检索: `search_history(query, k)` 跨 session 关键词匹配
//!
//! **不假装**: 真 Vec 存储, 真 timestamp (epoch ms), 真 history 增长.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Session 消息 (per-message 记录)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMessage {
    /// 序列号 (session 内单调递增)
    pub seq: u64,
    /// Advisor ID (e.g. "safety" / "performance" / "philosophy" / ...)
    pub advisor_id: String,
    /// 角色 ("system" / "user" / "assistant" / "tool" / ...)
    pub role: String,
    /// 消息内容
    pub content: String,
    /// Unix epoch 毫秒
    pub timestamp_ms: i64,
    /// Optional metadata (e.g. persona, confidence)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

/// Council session (从 start 到 end 一段时间内的 message 集合)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouncilSession {
    pub session_id: String,
    /// Unix epoch 毫秒
    pub started_at_ms: i64,
    /// Unix epoch 毫秒 (0 = 未结束)
    pub ended_at_ms: i64,
    pub messages: Vec<SessionMessage>,
}

impl CouncilSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            started_at_ms: current_epoch_ms(),
            ended_at_ms: 0,
            messages: Vec::new(),
        }
    }

    pub fn duration_ms(&self) -> i64 {
        if self.ended_at_ms == 0 {
            current_epoch_ms() - self.started_at_ms
        } else {
            self.ended_at_ms - self.started_at_ms
        }
    }

    pub fn is_ended(&self) -> bool {
        self.ended_at_ms > 0
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 跨所有 advisor 统计消息数
    pub fn messages_by_advisor(&self) -> HashMap<String, usize> {
        let mut m: HashMap<String, usize> = HashMap::new();
        for msg in &self.messages {
            *m.entry(msg.advisor_id.clone()).or_insert(0) += 1;
        }
        m
    }
}

/// Session capture engine — 当前 session + 历史 session
#[derive(Debug, Clone, Default)]
pub struct SessionCapture {
    current: Option<CouncilSession>,
    history: Vec<CouncilSession>,
    /// 全局 sequence counter (跨 session 单调)
    global_seq: u64,
}

impl SessionCapture {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前 session 是否 active
    pub fn has_active_session(&self) -> bool {
        self.current.is_some()
    }

    /// 当前 session ID (若有)
    pub fn current_session_id(&self) -> Option<&str> {
        self.current.as_ref().map(|s| s.session_id.as_str())
    }

    /// 历史 session 数
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// 总 session 数 (current + history)
    pub fn total_sessions(&self) -> usize {
        let cur = if self.current.is_some() { 1 } else { 0 };
        self.history.len() + cur
    }

    /// 启动新 session (若 current 存在则自动 end 并归档到 history)
    pub fn start_session(&mut self, session_id: impl Into<String>) {
        // 若 current 存在, 自动归档
        if let Some(prev) = self.current.take() {
            self.history.push(prev);
        }
        self.current = Some(CouncilSession::new(session_id));
    }

    /// 结束当前 session (返结束的 session; 若无 current 返 None)
    pub fn end_session(&mut self) -> Option<CouncilSession> {
        if let Some(mut cur) = self.current.take() {
            cur.ended_at_ms = current_epoch_ms();
            self.history.push(cur.clone());
            Some(cur)
        } else {
            None
        }
    }

    /// 记录消息到 current session (若无 current 则 no-op)
    pub fn record_message(
        &mut self,
        advisor_id: impl Into<String>,
        role: impl Into<String>,
        content: impl Into<String>,
        metadata: HashMap<String, String>,
    ) -> Option<u64> {
        let cur = self.current.as_mut()?;
        self.global_seq += 1;
        let seq = self.global_seq;
        cur.messages.push(SessionMessage {
            seq,
            advisor_id: advisor_id.into(),
            role: role.into(),
            content: content.into(),
            timestamp_ms: current_epoch_ms(),
            metadata,
        });
        Some(seq)
    }

    /// 历史 session 列表 (克隆)
    pub fn history(&self) -> Vec<CouncilSession> {
        self.history.clone()
    }

    /// 按 ID 查找 session (current + history)
    pub fn find_session(&self, session_id: &str) -> Option<CouncilSession> {
        if let Some(cur) = &self.current {
            if cur.session_id == session_id {
                return Some(cur.clone());
            }
        }
        self.history.iter().find(|s| s.session_id == session_id).cloned()
    }

    /// 跨历史 session 关键词检索 (top-k 简单匹配)
    pub fn search_history(&self, query: &str, k: usize) -> Vec<(String, usize)> {
        if query.is_empty() || k == 0 {
            return Vec::new();
        }
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(String, usize)> = Vec::new();
        for session in self.history.iter().chain(self.current.iter()) {
            let count = session
                .messages
                .iter()
                .filter(|m| m.content.to_lowercase().contains(&query_lower))
                .count();
            if count > 0 {
                scored.push((session.session_id.clone(), count));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().take(k).collect()
    }

    /// 跨历史 session 按 advisor 统计总消息数
    pub fn advisor_message_counts(&self) -> HashMap<String, usize> {
        let mut m: HashMap<String, usize> = HashMap::new();
        for session in self.history.iter().chain(self.current.iter()) {
            for msg in &session.messages {
                *m.entry(msg.advisor_id.clone()).or_insert(0) += 1;
            }
        }
        m
    }
}

fn current_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// Unit tests (0 网络, 0 真 LLM)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_capture_has_no_active_session() {
        let c = SessionCapture::new();
        assert!(!c.has_active_session());
        assert_eq!(c.current_session_id(), None);
        assert_eq!(c.history_len(), 0);
        assert_eq!(c.total_sessions(), 0);
    }

    #[test]
    fn start_session_creates_active() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        assert!(c.has_active_session());
        assert_eq!(c.current_session_id(), Some("s1"));
    }

    #[test]
    fn start_session_auto_archives_previous() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.record_message("safety", "assistant", "msg1", HashMap::new());
        c.start_session("s2"); // 自动归档 s1
        assert_eq!(c.current_session_id(), Some("s2"));
        assert_eq!(c.history_len(), 1);
        let archived = &c.history[0];
        assert_eq!(archived.session_id, "s1");
        assert_eq!(archived.message_count(), 1);
    }

    #[test]
    fn end_session_moves_current_to_history() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.record_message("safety", "assistant", "hi", HashMap::new());
        let ended = c.end_session().unwrap();
        assert_eq!(ended.session_id, "s1");
        assert!(ended.is_ended());
        assert!(!c.has_active_session());
        assert_eq!(c.history_len(), 1);
    }

    #[test]
    fn end_without_active_returns_none() {
        let mut c = SessionCapture::new();
        assert!(c.end_session().is_none());
    }

    #[test]
    fn record_message_without_active_is_noop() {
        let mut c = SessionCapture::new();
        let seq = c.record_message("safety", "assistant", "lost", HashMap::new());
        assert_eq!(seq, None);
    }

    #[test]
    fn record_message_assigns_monotonic_seq() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        let s1 = c.record_message("a", "assistant", "m1", HashMap::new()).unwrap();
        let s2 = c.record_message("b", "assistant", "m2", HashMap::new()).unwrap();
        let s3 = c.record_message("c", "assistant", "m3", HashMap::new()).unwrap();
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        assert_eq!(s3, 3);
        // 跨 session 也单调
        c.start_session("s2");
        let s4 = c.record_message("d", "assistant", "m4", HashMap::new()).unwrap();
        assert_eq!(s4, 4);
    }

    #[test]
    fn record_message_stores_advisor_role_content() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        let mut meta = HashMap::new();
        meta.insert("persona".into(), "sage".into());
        c.record_message("philosophy", "assistant", "deep thought", meta);
        let cur = c.find_session("s1").unwrap();
        assert_eq!(cur.messages.len(), 1);
        assert_eq!(cur.messages[0].advisor_id, "philosophy");
        assert_eq!(cur.messages[0].role, "assistant");
        assert_eq!(cur.messages[0].content, "deep thought");
        assert_eq!(cur.messages[0].metadata.get("persona").unwrap(), "sage");
    }

    #[test]
    fn find_session_searches_current_and_history() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.start_session("s2"); // s1 archived
        assert!(c.find_session("s1").is_some());
        assert!(c.find_session("s2").is_some());
        assert!(c.find_session("nope").is_none());
    }

    #[test]
    fn search_history_keyword_matching() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.record_message("a", "assistant", "Rust ownership rules", HashMap::new());
        c.record_message("b", "assistant", "borrowing checker", HashMap::new());
        c.start_session("s2"); // s1 archived
        c.record_message("c", "assistant", "Python decorators", HashMap::new());
        c.end_session();

        let hits = c.search_history("rust", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "s1");
        assert_eq!(hits[0].1, 1);
    }

    #[test]
    fn search_history_top_k_limit() {
        let mut c = SessionCapture::new();
        for i in 0..5 {
            c.start_session(format!("s{}", i));
            c.record_message("a", "assistant", "rust pattern", HashMap::new());
            c.end_session();
        }
        let hits = c.search_history("rust", 3);
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_history_case_insensitive() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.record_message("a", "assistant", "RUST OWNERSHIP", HashMap::new());
        c.end_session();
        let hits = c.search_history("rust", 10);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_history_empty_query_no_match() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.record_message("a", "assistant", "anything", HashMap::new());
        c.end_session();
        assert!(c.search_history("", 10).is_empty());
    }

    #[test]
    fn advisor_message_counts_aggregates() {
        let mut c = SessionCapture::new();
        c.start_session("s1");
        c.record_message("safety", "assistant", "a", HashMap::new());
        c.record_message("safety", "assistant", "b", HashMap::new());
        c.record_message("philosophy", "assistant", "c", HashMap::new());
        c.start_session("s2"); // s1 archived
        c.record_message("safety", "assistant", "d", HashMap::new());
        let counts = c.advisor_message_counts();
        assert_eq!(counts.get("safety"), Some(&3));
        assert_eq!(counts.get("philosophy"), Some(&1));
    }

    #[test]
    fn session_duration_when_active() {
        let s = CouncilSession::new("s1");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let d = s.duration_ms();
        assert!(d >= 5, "duration_ms should be >= 5ms after sleep");
    }

    #[test]
    fn session_messages_by_advisor() {
        let mut s = CouncilSession::new("s1");
        s.messages.push(SessionMessage {
            seq: 1, advisor_id: "safety".into(), role: "assistant".into(),
            content: "a".into(), timestamp_ms: 0, metadata: HashMap::new(),
        });
        s.messages.push(SessionMessage {
            seq: 2, advisor_id: "safety".into(), role: "assistant".into(),
            content: "b".into(), timestamp_ms: 0, metadata: HashMap::new(),
        });
        s.messages.push(SessionMessage {
            seq: 3, advisor_id: "philosophy".into(), role: "assistant".into(),
            content: "c".into(), timestamp_ms: 0, metadata: HashMap::new(),
        });
        let counts = s.messages_by_advisor();
        assert_eq!(counts.get("safety"), Some(&2));
        assert_eq!(counts.get("philosophy"), Some(&1));
    }

    #[test]
    fn r150_session_capture_deliverables() {
        // R150 P1 #10 完成定义:
        // - SessionCapture + start/end/record/search/find + history
        // - 16 unit tests + 0 外部 LLM/IO
        let mut c = SessionCapture::new();
        assert_eq!(c.total_sessions(), 0);
        c.start_session("s1");
        assert_eq!(c.total_sessions(), 1);
        c.record_message("safety", "assistant", "test", HashMap::new());
        assert_eq!(c.history_len(), 0);
        c.end_session();
        assert_eq!(c.history_len(), 1);
        assert_eq!(c.total_sessions(), 1);
    }
}
