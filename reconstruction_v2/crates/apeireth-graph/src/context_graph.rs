//! ContextGraph — conversation/session context tracking.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Context error.
#[derive(Debug, Error)]
pub enum ContextError {
    #[error("context phase out of range: {0}")]
    InvalidPhase(u8),
}

/// Number of context phases.
pub const CONTEXT_PHASE_COUNT: usize = 5;

/// Phases of a context lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextPhase {
    /// Initial state, no input yet.
    Init = 0,
    /// User input received.
    Input = 1,
    /// Processing / planning.
    Processing = 2,
    /// Tool execution.
    Tool = 3,
    /// Response ready.
    Response = 4,
}

impl ContextPhase {
    pub fn from_u8(v: u8) -> Result<Self, ContextError> {
        Ok(match v {
            0 => Self::Init,
            1 => Self::Input,
            2 => Self::Processing,
            3 => Self::Tool,
            4 => Self::Response,
            _ => return Err(ContextError::InvalidPhase(v)),
        })
    }
}

/// One node in the context graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextNode {
    pub id: String,
    pub phase: ContextPhase,
    pub data: serde_json::Value,
}

impl ContextNode {
    pub fn new(id: impl Into<String>, phase: ContextPhase) -> Self {
        Self { id: id.into(), phase, data: serde_json::Value::Null }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

/// Snapshot of a context graph state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub session_id: String,
    pub nodes: Vec<ContextNode>,
    pub phase: ContextPhase,
}

/// Context graph — sequence of context nodes for one session.
#[derive(Clone)]
pub struct ContextGraph {
    pub session_id: String,
    pub nodes: Vec<ContextNode>,
}

impl ContextGraph {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self { session_id: session_id.into(), nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: ContextNode) {
        self.nodes.push(node);
    }

    pub fn current_phase(&self) -> ContextPhase {
        self.nodes.last().map(|n| n.phase).unwrap_or(ContextPhase::Init)
    }

    pub fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            session_id: self.session_id.clone(),
            phase: self.current_phase(),
            nodes: self.nodes.clone(),
        }
    }

    pub fn len(&self) -> usize { self.nodes.len() }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }
}

/// Store trait for context graphs.
pub trait ContextStore: Send + Sync {
    fn get(&self, session_id: &str) -> Option<ContextGraph>;
    fn put(&self, graph: ContextGraph);
    fn delete(&self, session_id: &str);
}

/// In-memory context store.
#[derive(Default, Clone)]
pub struct InMemoryContextStore {
    inner: Arc<RwLock<HashMap<String, ContextGraph>>>,
}

impl InMemoryContextStore {
    pub fn new() -> Self { Self::default() }
}

impl ContextStore for InMemoryContextStore {
    fn get(&self, session_id: &str) -> Option<ContextGraph> {
        self.inner.read().unwrap().get(session_id).cloned()
    }
    fn put(&self, graph: ContextGraph) {
        self.inner.write().unwrap().insert(graph.session_id.clone(), graph);
    }
    fn delete(&self, session_id: &str) {
        self.inner.write().unwrap().remove(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_from_u8_round_trip() {
        for v in 0..CONTEXT_PHASE_COUNT as u8 {
            let p = ContextPhase::from_u8(v).unwrap();
            assert_eq!(p as u8, v);
        }
        assert!(ContextPhase::from_u8(99).is_err());
    }

    #[test]
    fn context_graph_lifecycle() {
        let mut g = ContextGraph::new("sess-1");
        g.add_node(ContextNode::new("n1", ContextPhase::Init));
        g.add_node(ContextNode::new("n2", ContextPhase::Input));
        g.add_node(ContextNode::new("n3", ContextPhase::Processing));
        assert_eq!(g.current_phase(), ContextPhase::Processing);
        assert_eq!(g.len(), 3);
        let snap = g.snapshot();
        assert_eq!(snap.session_id, "sess-1");
        assert_eq!(snap.nodes.len(), 3);
    }

    #[test]
    fn store_put_get_delete() {
        let store = InMemoryContextStore::new();
        let mut g = ContextGraph::new("s1");
        g.add_node(ContextNode::new("a", ContextPhase::Init));
        store.put(g);
        let got = store.get("s1").unwrap();
        assert_eq!(got.len(), 1);
        store.delete("s1");
        assert!(store.get("s1").is_none());
    }

    #[test]
    fn node_with_data() {
        let n = ContextNode::new("x", ContextPhase::Response)
            .with_data(serde_json::json!({"answer": 42}));
        assert_eq!(n.data["answer"], 42);
    }
}
