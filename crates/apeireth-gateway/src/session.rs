//! Session lifecycle.
//!
//! Borrowed from OpenClaw: a single gateway process owns ONE long-lived root
//! `Session`; each Node adapter may spawn child sessions for parallel work,
//! but the root session never dies until the process exits. This is the
//! OpenClaw "single long-lived" mode.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;
use crate::node::{NodeId, NodeKind};

/// Stable session identifier.
pub type SessionId = Uuid;

/// Lifecycle state for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Active,
    Suspended,
    Closed,
}

impl SessionState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Closed => "closed",
        }
    }

    pub const fn is_alive(&self) -> bool {
        matches!(self, Self::Active | Self::Suspended)
    }
}

/// One session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub parent: Option<SessionId>,
    pub node_id: NodeId,
    pub node_kind: NodeKind,
    pub state: SessionState,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
    pub skill_count: usize,
    pub memory_keys: usize,
}

impl Session {
    pub fn new(node_id: NodeId, node_kind: NodeKind, now: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent: None,
            node_id,
            node_kind,
            state: SessionState::Active,
            opened_at: now,
            closed_at: None,
            skill_count: 0,
            memory_keys: 0,
        }
    }

    pub fn child_of(mut self, parent: SessionId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn suspend(&mut self) {
        if self.state == SessionState::Active {
            self.state = SessionState::Suspended;
        }
    }

    pub fn resume(&mut self) {
        if self.state == SessionState::Suspended {
            self.state = SessionState::Active;
        }
    }

    pub fn close(&mut self, now: i64) {
        self.state = SessionState::Closed;
        if self.closed_at.is_none() {
            self.closed_at = Some(now);
        }
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    pub fn lifetime_ms(&self, now: i64) -> i64 {
        let end = self.closed_at.unwrap_or(now);
        end.saturating_sub(self.opened_at)
    }
}

/// Concurrency-safe registry of sessions.
#[derive(Debug, Default, Clone)]
pub struct SessionRegistry {
    inner: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&self, node_id: NodeId, node_kind: NodeKind, now: i64) -> Session {
        let s = Session::new(node_id, node_kind, now);
        self.inner.write().insert(s.id, s.clone());
        s
    }

    pub fn close(&self, id: SessionId, now: i64) -> bool {
        let mut w = self.inner.write();
        if let Some(s) = w.get_mut(&id) {
            s.close(now);
            true
        } else {
            false
        }
    }

    pub fn suspend(&self, id: SessionId) -> bool {
        let mut w = self.inner.write();
        if let Some(s) = w.get_mut(&id) {
            s.suspend();
            true
        } else {
            false
        }
    }

    pub fn resume(&self, id: SessionId) -> bool {
        let mut w = self.inner.write();
        if let Some(s) = w.get_mut(&id) {
            s.resume();
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: SessionId) -> Option<Session> {
        self.inner.read().get(&id).cloned()
    }

    pub fn list_by_node(&self, node_id: NodeId) -> Vec<Session> {
        self.inner.read().values().filter(|s| s.node_id == node_id).cloned().collect()
    }

    pub fn live_count(&self) -> usize {
        self.inner.read().values().filter(|s| s.state.is_alive()).count()
    }

    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    pub fn root(&self) -> Option<Session> {
        self.inner.read().values().find(|s| s.is_root() && s.state.is_alive()).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_as_str() {
        assert_eq!(SessionState::Active.as_str(), "active");
        assert_eq!(SessionState::Suspended.as_str(), "suspended");
        assert_eq!(SessionState::Closed.as_str(), "closed");
    }

    #[test]
    fn session_state_is_alive() {
        assert!(SessionState::Active.is_alive());
        assert!(SessionState::Suspended.is_alive());
        assert!(!SessionState::Closed.is_alive());
    }

    #[test]
    fn session_lifecycle_transitions() {
        let node_id = Uuid::new_v4();
        let mut s = Session::new(node_id, NodeKind::Tui, 100);
        assert_eq!(s.state, SessionState::Active);
        assert!(s.is_root());
        s.suspend();
        assert_eq!(s.state, SessionState::Suspended);
        s.resume();
        assert_eq!(s.state, SessionState::Active);
        s.close(200);
        assert_eq!(s.state, SessionState::Closed);
        assert_eq!(s.closed_at, Some(200));
        assert_eq!(s.lifetime_ms(300), 100);
    }

    #[test]
    fn session_close_idempotent() {
        let node_id = Uuid::new_v4();
        let mut s = Session::new(node_id, NodeKind::Http, 100);
        s.close(200);
        s.close(300); // should not change closed_at
        assert_eq!(s.closed_at, Some(200));
    }

    #[test]
    fn session_close_from_suspended_kept_dead() {
        let node_id = Uuid::new_v4();
        let mut s = Session::new(node_id, NodeKind::Cli, 100);
        s.suspend();
        s.close(300);
        assert_eq!(s.state, SessionState::Closed);
        s.resume(); // should be no-op
        assert_eq!(s.state, SessionState::Closed);
    }

    #[test]
    fn session_child_of_records_parent() {
        let node_id = Uuid::new_v4();
        let parent = Uuid::new_v4();
        let s = Session::new(node_id, NodeKind::Desktop, 0).child_of(parent);
        assert_eq!(s.parent, Some(parent));
        assert!(!s.is_root());
    }

    #[test]
    fn registry_open_close_list() {
        let r = SessionRegistry::new();
        let node_id = Uuid::new_v4();
        let s = r.open(node_id, NodeKind::Tui, 100);
        assert_eq!(r.len(), 1);
        assert_eq!(r.live_count(), 1);
        assert!(r.close(s.id, 200));
        assert_eq!(r.live_count(), 0);
    }

    #[test]
    fn registry_suspend_resume() {
        let r = SessionRegistry::new();
        let s = r.open(Uuid::new_v4(), NodeKind::Http, 0);
        assert!(r.suspend(s.id));
        assert_eq!(r.get(s.id).unwrap().state, SessionState::Suspended);
        assert!(r.resume(s.id));
        assert_eq!(r.get(s.id).unwrap().state, SessionState::Active);
    }

    #[test]
    fn registry_list_by_node() {
        let r = SessionRegistry::new();
        let n1 = Uuid::new_v4();
        let n2 = Uuid::new_v4();
        r.open(n1, NodeKind::Tui, 0);
        r.open(n1, NodeKind::Tui, 0);
        r.open(n2, NodeKind::Http, 0);
        assert_eq!(r.list_by_node(n1).len(), 2);
        assert_eq!(r.list_by_node(n2).len(), 1);
    }

    #[test]
    fn registry_root_picks_live_root() {
        let r = SessionRegistry::new();
        let s = r.open(Uuid::new_v4(), NodeKind::Tui, 0);
        let root = r.root().unwrap();
        assert_eq!(root.id, s.id);
        assert!(r.close(s.id, 100));
        assert!(r.root().is_none());
    }
}
