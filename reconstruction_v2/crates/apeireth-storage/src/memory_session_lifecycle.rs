//! Memory SessionLifecycle - 会话生命周期 (抄 v1 apeireth-memory/session_lifecycle.rs)
use std::collections::HashMap;
pub enum SessionState { Active, Idle, Closed }
pub struct SessionInfo { pub id: String, pub state: SessionState, pub created_ms: i64 }
pub struct SessionLifecycle { pub sessions: HashMap<String, SessionInfo> }
impl SessionLifecycle {
    pub fn new() -> Self { Self { sessions: HashMap::new() } }
    pub fn start(&mut self, id: impl Into<String>) {
        self.sessions.insert(id.into(), SessionInfo { id: "".into(), state: SessionState::Active, created_ms: chrono::Utc::now().timestamp_millis() });
    }
    pub fn close(&mut self, id: &str) { if let Some(s) = self.sessions.get_mut(id) { s.state = SessionState::Closed; } }
    pub fn active(&self) -> Vec<&SessionInfo> { self.sessions.values().filter(|s| matches!(s.state, SessionState::Active)).collect() }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_lifecycle() { let mut l = SessionLifecycle::new(); l.start("s1"); assert_eq!(l.active().len(), 1); l.close("s1"); assert_eq!(l.active().len(), 0); } }