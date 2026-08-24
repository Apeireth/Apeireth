//! Property graph storage with adjacency indexes.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Node ID (string-backed).
pub type NodeId = String;
/// Edge ID.
pub type EdgeId = u64;

/// Property graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub labels: BTreeSet<String>,
    pub properties: HashMap<String, serde_json::Value>,
}

impl GraphNode {
    pub fn new(id: impl Into<NodeId>) -> Self {
        Self { id: id.into(), labels: BTreeSet::new(), properties: HashMap::new() }
    }
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.insert(label.into()); self
    }
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.properties.insert(key.into(), value.into()); self
    }
}

/// Property graph edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl GraphEdge {
    pub fn new(id: EdgeId, from: impl Into<NodeId>, to: impl Into<NodeId>, label: impl Into<String>) -> Self {
        Self { id, from: from.into(), to: to.into(), label: label.into(), properties: HashMap::new() }
    }
}

/// Property graph.
pub struct RelationGraph {
    pub(crate) nodes: BTreeMap<NodeId, GraphNode>,
    pub(crate) edges: BTreeMap<EdgeId, GraphEdge>,
    pub(crate) out_edges: HashMap<NodeId, Vec<EdgeId>>,
    pub(crate) in_edges: HashMap<NodeId, Vec<EdgeId>>,
    next_edge_id: EdgeId,
}

impl Default for RelationGraph {
    fn default() -> Self {
        Self { nodes: BTreeMap::new(), edges: BTreeMap::new(), out_edges: HashMap::new(), in_edges: HashMap::new(), next_edge_id: 0 }
    }
}

impl RelationGraph {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self, node: GraphNode) {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        self.out_edges.entry(id.clone()).or_insert_with(Vec::new);
        self.in_edges.entry(id).or_insert_with(Vec::new);
    }

    pub fn add_edge(&mut self, mut edge: GraphEdge) -> EdgeId {
        let id = self.next_edge_id;
        self.next_edge_id += 1;
        edge.id = id;
        self.out_edges.entry(edge.from.clone()).or_insert_with(Vec::new).push(id);
        self.in_edges.entry(edge.to.clone()).or_insert_with(Vec::new).push(id);
        self.edges.insert(id, edge);
        id
    }

    pub fn get_node(&self, id: &str) -> Option<&GraphNode> { self.nodes.get(id) }
    pub fn get_edge(&self, id: EdgeId) -> Option<&GraphEdge> { self.edges.get(&id) }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }

    pub fn out_neighbors(&self, id: &str) -> Vec<&NodeId> {
        self.out_edges.get(id)
            .map(|v| v.iter().filter_map(|eid| self.edges.get(eid).map(|e| &e.to)).collect())
            .unwrap_or_default()
    }

    pub fn in_neighbors(&self, id: &str) -> Vec<&NodeId> {
        self.in_edges.get(id)
            .map(|v| v.iter().filter_map(|eid| self.edges.get(eid).map(|e| &e.from)).collect())
            .unwrap_or_default()
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &GraphNode> { self.nodes.values() }
    pub fn all_edges(&self) -> impl Iterator<Item = &GraphEdge> { self.edges.values() }

    /// Internal RW lock for concurrent access.
    pub fn lock(&self) -> &RwLock<()> { static NULL: std::sync::OnceLock<RwLock<()>> = std::sync::OnceLock::new(); NULL.get_or_init(|| RwLock::new(())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_node_edge_basic() {
        let mut g = RelationGraph::new();
        g.add_node(GraphNode::new("a"));
        g.add_node(GraphNode::new("b"));
        let eid = g.add_edge(GraphEdge::new(0, "a", "b", "knows"));
        assert_eq!(eid, 0);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn out_in_neighbors() {
        let mut g = RelationGraph::new();
        g.add_node(GraphNode::new("a"));
        g.add_node(GraphNode::new("b"));
        g.add_node(GraphNode::new("c"));
        g.add_edge(GraphEdge::new(0, "a", "b", "e1"));
        g.add_edge(GraphEdge::new(1, "b", "c", "e2"));
        assert_eq!(g.out_neighbors("a").len(), 1);
        assert_eq!(g.in_neighbors("b").len(), 1);
        assert_eq!(g.out_neighbors("b").len(), 1);
        assert_eq!(g.in_neighbors("c").len(), 1);
    }

    #[test]
    fn node_with_label_property() {
        let n = GraphNode::new("x")
            .with_label("Person")
            .with_property("age", 42);
        assert!(n.labels.contains("Person"));
        assert_eq!(n.properties["age"], 42);
    }

    #[test]
    fn get_node_returns_some() {
        let mut g = RelationGraph::new();
        g.add_node(GraphNode::new("id"));
        assert!(g.get_node("id").is_some());
        assert!(g.get_node("missing").is_none());
    }
}
