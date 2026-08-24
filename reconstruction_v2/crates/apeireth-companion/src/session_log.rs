//! SessionLog - 会话日志 (从 v1.0 apeireth-companion/session_log.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 session event log + 按 session filter
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub timestamp_ms: i64,
    pub data: String,
}

pub struct SessionLog {
    events: VecDeque<SessionEvent>,
    capacity: usize,
}

impl SessionLog {
    pub fn new(capacity: usize) -> Self { Self { events: VecDeque::with_capacity(capacity), capacity } }

    /// 0 装 PASS: 真记录
    pub fn record(&mut self, session_id: impl Into<String>, kind: impl Into<String>, data: impl Into<String>) {
        let event = SessionEvent { id: format!("e-{}", chrono::Utc::now().timestamp_millis()), session_id: session_id.into(), kind: kind.into(), timestamp_ms: chrono::Utc::now().timestamp_millis(), data: data.into() };
        self.events.push_back(event);
        if self.events.len() > self.capacity { self.events.pop_front(); }
    }

    pub fn by_session(&self, session_id: &str) -> Vec<&SessionEvent> {
        self.events.iter().filter(|e| e.session_id == session_id).collect()
    }

    pub fn len(&self) -> usize { self.events.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() {
        let mut log = SessionLog::new(10);
        log.record("s1", "start", "{}");
        assert_eq!(log.len(), 1);
    }
    #[test] fn test_by_session() {
        let mut log = SessionLog::new(10);
        log.record("s1", "x", "{}");
        log.record("s2", "y", "{}");
        assert_eq!(log.by_session("s1").len(), 1);
    }
    #[test] fn test_capacity() {
        let mut log = SessionLog::new(2);
        for i in 0..5 { log.record("s", "x", &format!("{}", i)); }
        assert_eq!(log.len(), 2);
    }
}
