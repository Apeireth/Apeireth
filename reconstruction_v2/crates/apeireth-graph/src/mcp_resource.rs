//! MCP Resource server exposing graph state.

use serde::{Deserialize, Serialize};
use crate::context_graph::ContextGraph;
use crate::state_graph::StateGraph;

/// A graph resource exposed via MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResource {
    pub uri: String,
    pub name: String,
    pub kind: String,
    pub description: String,
}

/// MCP resource server exposing graph state.
pub struct GraphResourceServer {
    pub resources: Vec<GraphResource>,
}

impl GraphResourceServer {
    pub fn new() -> Self { Self { resources: Vec::new() } }

    pub fn register(&mut self, resource: GraphResource) { self.resources.push(resource); }

    pub fn from_state_graph(graph: &StateGraph) -> Self {
        let mut s = Self::new();
        s.register(GraphResource {
            uri: format!("graph://{}", "state"),
            name: "state_graph".into(),
            kind: "graph".into(),
            description: format!("StateGraph with {} nodes", graph.nodes.len()),
        });
        s
    }

    pub fn from_context_graph(graph: &ContextGraph) -> Self {
        let mut s = Self::new();
        s.register(GraphResource {
            uri: format!("context://{}", graph.session_id),
            name: "context_graph".into(),
            kind: "context".into(),
            description: format!("ContextGraph for session {} ({} nodes)", graph.session_id, graph.len()),
        });
        s
    }

    pub fn list(&self) -> &[GraphResource] { &self.resources }
}

impl Default for GraphResourceServer {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_graph::{ContextGraph, ContextPhase, ContextNode};

    #[test]
    fn from_state_graph_lists() {
        let sg = StateGraph {
            nodes: std::collections::BTreeMap::new(),
            edges: vec![],
            conditional_edges: vec![],
        };
        let s = GraphResourceServer::from_state_graph(&sg);
        assert_eq!(s.list().len(), 1);
    }

    #[test]
    fn from_context_graph() {
        let mut cg = ContextGraph::new("s");
        cg.add_node(ContextNode::new("a", ContextPhase::Init));
        let s = GraphResourceServer::from_context_graph(&cg);
        assert!(s.list()[0].uri.contains("s"));
    }

    #[test]
    fn server_register_default() {
        let mut s = GraphResourceServer::default();
        s.register(GraphResource {
            uri: "u".into(), name: "n".into(), kind: "k".into(), description: "d".into(),
        });
        assert_eq!(s.list().len(), 1);
    }
}
