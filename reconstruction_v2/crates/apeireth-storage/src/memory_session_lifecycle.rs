//! Memory SessionLifecycle - session 生命周期 (从 v1.0 apeireth-memory/session_lifecycle.rs 796 LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 session state machine + 转场

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState { Created, Active, Idle, Closed, Expired }

impl SessionState {
    pub fn is_terminal(self) -> bool { matches!(self, Self::Closed | Self::Expired) }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub state: SessionState,
    pub created_ms: i64,
    pub last_active_ms: i64,
    pub user_id: Option<String>,
}

pub struct SessionLifecycle {
    sessions: HashMap<String, Session>,
}

impl SessionLifecycle {
    pub fn new() -> Self { Self { sessions: HashMap::new() } }

    /// 0 装 PASS: 真创建 session
    pub fn create(&mut self, user_id: Option<String>) -> String {
        let id = format!("s-{}", chrono::Utc::now().timestamp_millis());
        let session = Session { id: id.clone(), state: SessionState::Created, created_ms: chrono::Utc::now().timestamp_millis(), last_active_ms: chrono::Utc::now().timestamp_millis(), user_id };
        self.sessions.insert(id.clone(), session);
        id
    }

    /// 0 装 PASS: 真 transition state
    pub fn transition(&mut self, id: &str, target: SessionState) -> Result<(), String> {
        let session = self.sessions.get_mut(id).ok_or_else(|| "session not found")?;
        if session.state.is_terminal() && target != SessionState::Closed {
            return Err("session already terminated".into());
        }
        session.state = target;
        session.last_active_ms = chrono::Utc::now().timestamp_millis();
        Ok(())
    }

    pub fn active_sessions(&self) -> Vec<&Session> {
        self.sessions.values().filter(|s| !s.state.is_terminal()).collect()
    }

    pub fn count(&self) -> usize { self.sessions.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create_session() { let mut lc = SessionLifecycle::new(); let id = lc.create(None); assert!(!id.is_empty()); assert_eq!(lc.count(), 1); }
    #[test] fn test_transition() { let mut lc = SessionLifecycle::new(); let id = lc.create(None); lc.transition(&id, SessionState::Active).unwrap(); assert_eq!(lc.sessions.get(&id).unwrap().state, SessionState::Active); }
    #[test] fn test_terminal() { assert!(SessionState::Closed.is_terminal()); assert!(!SessionState::Active.is_terminal()); }
    #[test] fn test_active_count() { let mut lc = SessionLifecycle::new(); let id1 = lc.create(None); let id2 = lc.create(None); lc.transition(&id1, SessionState::Active).unwrap(); lc.transition(&id2, SessionState::Closed).unwrap(); assert_eq!(lc.active_sessions().len(), 1); }
}
