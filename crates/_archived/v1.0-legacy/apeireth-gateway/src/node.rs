//! Multi-Node adapter identity.
//!
//! Borrowed from OpenClaw: a single gateway process can be reached over many
//! channels (TUI, HTTP, Desktop, Mobile, CLI). Each Node is identified by a
//! stable `NodeId` and exposes its `NodeKind` so the daemon can apply the
//! correct transport + access policy.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// 5 canonical Node kinds (OpenClaw taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    /// Terminal UI (ratatui).
    Tui,
    /// HTTP / OpenAI-compatible API adapter.
    Http,
    /// Desktop companion (Tauri/Live2D).
    Desktop,
    /// Mobile companion (iOS/Android).
    Mobile,
    /// CLI one-shot.
    Cli,
}

impl NodeKind {
    pub const ALL: [NodeKind; 5] = [
        Self::Tui,
        Self::Http,
        Self::Desktop,
        Self::Mobile,
        Self::Cli,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Tui => "tui",
            Self::Http => "http",
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Cli => "cli",
        }
    }

    pub const fn is_interactive(&self) -> bool {
        matches!(self, Self::Tui | Self::Desktop | Self::Mobile)
    }

    pub const fn is_headless(&self) -> bool {
        matches!(self, Self::Http | Self::Cli)
    }
}

/// Stable identifier for a Node registration. UUID v4 for now; the ceiling is
/// device-bound hardware fingerprint.
pub type NodeId = Uuid;

/// Per-Node registration record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: NodeId,
    pub kind: NodeKind,
    pub label: String,
    pub owner: String,
    pub registered_at: i64,
    pub last_seen_at: i64,
}

impl NodeRecord {
    pub fn new(
        kind: NodeKind,
        label: impl Into<String>,
        owner: impl Into<String>,
        now: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            label: label.into(),
            owner: owner.into(),
            registered_at: now,
            last_seen_at: now,
        }
    }

    pub fn touch(&mut self, now: i64) {
        self.last_seen_at = now;
    }
}

/// Concurrency-safe registry of connected Nodes.
#[derive(Debug, Default, Clone)]
pub struct NodeRegistry {
    inner: Arc<RwLock<HashMap<NodeId, NodeRecord>>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, rec: NodeRecord) -> NodeId {
        let id = rec.id;
        self.inner.write().insert(id, rec);
        id
    }

    pub fn unregister(&self, id: NodeId) -> Option<NodeRecord> {
        self.inner.write().remove(&id)
    }

    pub fn get(&self, id: NodeId) -> Option<NodeRecord> {
        self.inner.read().get(&id).cloned()
    }

    pub fn touch(&self, id: NodeId, now: i64) -> bool {
        let mut w = self.inner.write();
        if let Some(r) = w.get_mut(&id) {
            r.touch(now);
            true
        } else {
            false
        }
    }

    pub fn by_kind(&self, kind: NodeKind) -> Vec<NodeRecord> {
        self.inner
            .read()
            .values()
            .filter(|r| r.kind == kind)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    pub fn ids(&self) -> Vec<NodeId> {
        self.inner.read().keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_kind_as_str_covers_all() {
        for k in NodeKind::ALL {
            assert!(!k.as_str().is_empty());
        }
    }

    #[test]
    fn node_kind_classification() {
        assert!(NodeKind::Tui.is_interactive());
        assert!(NodeKind::Desktop.is_interactive());
        assert!(NodeKind::Mobile.is_interactive());
        assert!(NodeKind::Http.is_headless());
        assert!(NodeKind::Cli.is_headless());
    }

    #[test]
    fn registry_register_and_get() {
        let r = NodeRegistry::new();
        let id = r.register(NodeRecord::new(NodeKind::Tui, "test-tui", "user-a", 100));
        assert_eq!(r.len(), 1);
        let rec = r.get(id).unwrap();
        assert_eq!(rec.kind, NodeKind::Tui);
        assert_eq!(rec.label, "test-tui");
        assert_eq!(rec.owner, "user-a");
    }

    #[test]
    fn registry_unregister() {
        let r = NodeRegistry::new();
        let id = r.register(NodeRecord::new(NodeKind::Http, "h", "u", 0));
        assert!(r.unregister(id).is_some());
        assert!(r.is_empty());
    }

    #[test]
    fn registry_touch_updates_last_seen() {
        let r = NodeRegistry::new();
        let id = r.register(NodeRecord::new(NodeKind::Cli, "c", "u", 100));
        assert!(r.touch(id, 200));
        assert_eq!(r.get(id).unwrap().last_seen_at, 200);
        assert!(!r.touch(Uuid::new_v4(), 300));
    }

    #[test]
    fn registry_by_kind_filters() {
        let r = NodeRegistry::new();
        r.register(NodeRecord::new(NodeKind::Tui, "t1", "u", 0));
        r.register(NodeRecord::new(NodeKind::Tui, "t2", "u", 0));
        r.register(NodeRecord::new(NodeKind::Http, "h", "u", 0));
        assert_eq!(r.by_kind(NodeKind::Tui).len(), 2);
        assert_eq!(r.by_kind(NodeKind::Http).len(), 1);
        assert_eq!(r.by_kind(NodeKind::Mobile).len(), 0);
    }

    #[test]
    fn registry_ids_returns_all() {
        let r = NodeRegistry::new();
        let a = r.register(NodeRecord::new(NodeKind::Tui, "a", "u", 0));
        let b = r.register(NodeRecord::new(NodeKind::Http, "b", "u", 0));
        let mut ids = r.ids();
        ids.sort();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[test]
    fn node_record_touch_bumps_ts() {
        let mut rec = NodeRecord::new(NodeKind::Desktop, "d", "u", 100);
        rec.touch(500);
        assert_eq!(rec.last_seen_at, 500);
    }
}
