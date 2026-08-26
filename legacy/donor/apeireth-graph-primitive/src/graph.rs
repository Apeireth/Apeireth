//! Graph data structure with nodes, edges, and adjacency-list indexes.
//!
//! ## Borrowed Reference (per O-5)
//!
//! Inspired by **SurrealDB** (graph storage + RELATE statement + traversal
//! via `->` arrow operator) and **Memgraph** (in-memory property graph). We
//! mirror the **node + edge + property** model with adjacency-list indexes,
//! but keep the API minimal and 0 external dep.
//!
//! ## Design
//!
//! - `RelationGraph` — owns `HashMap<NodeId, GraphNode>` + `HashMap<EdgeId, GraphEdge>` + adjacency indexes
//! - `GraphNode` / `GraphEdge` — property bag via `serde_json::Value` (any JSON)
//! - `From<&RelationRegistry> for RelationGraph` — bridges the existing flat `RelationRegistry` view
//!
//! ## Not implemented (留 R155+)
//!
//! - Persistent storage (currently in-memory only)
//! - Concurrent writes (single-threaded; wraps `Arc<RwLock<...>>` at the call site if needed)
//! - GQL full-text parser (only the simple `GraphQuery` predicate API in `query.rs`)

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Relation, RelationKind, RelationRegistry};

// ============================================================================
// §1 Error type
// ============================================================================

/// Graph errors (5 variants).
#[derive(Debug, Error)]
pub enum GraphError {
    /// Node ID not found.
    #[error("node not found: {0}")]
    NodeNotFound(String),
    /// Edge not found.
    #[error("edge not found: {0}")]
    EdgeNotFound(String),
    /// Duplicate node insertion.
    #[error("duplicate node: {0}")]
    DuplicateNode(String),
    /// Duplicate edge insertion.
    #[error("duplicate edge: {0}")]
    DuplicateEdge(String),
    /// Invalid edge endpoint (node does not exist).
    #[error("edge references missing node: {0}")]
    InvalidEdgeEndpoint(String),
}

pub type GraphResult<T> = Result<T, GraphError>;

// ============================================================================
// §2 Node / Edge types
// ============================================================================

/// Node identifier. Any hashable type (e.g. continuity_id string).
pub type NodeId = String;

/// Edge identifier. Default = UUID, but can be customized.
pub type EdgeId = String;

/// Graph node (party in the relation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique ID within the graph.
    pub id: NodeId,
    /// Optional kind/category for filtering (e.g. "agent", "tool", "concept").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Property bag (any JSON).
    #[serde(default)]
    pub properties: serde_json::Value,
}

impl GraphNode {
    /// Create a node with id only.
    pub fn new(id: impl Into<NodeId>) -> Self {
        Self {
            id: id.into(),
            kind: None,
            properties: serde_json::Value::Null,
        }
    }

    /// Create a node with id + kind.
    pub fn with_kind(id: impl Into<NodeId>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Some(kind.into()),
            properties: serde_json::Value::Null,
        }
    }

    /// Attach property bag.
    pub fn with_properties(mut self, props: serde_json::Value) -> Self {
        self.properties = props;
        self
    }
}

/// Graph edge (typed relation between two nodes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique edge ID (UUID by default).
    pub id: EdgeId,
    /// Source node.
    pub from: NodeId,
    /// Target node.
    pub to: NodeId,
    /// Relation kind (Symbiosis / Coordination / Embedding / SelfRelation).
    pub kind: RelationKind,
    /// Property bag (any JSON).
    #[serde(default)]
    pub properties: serde_json::Value,
}

impl GraphEdge {
    /// Create a new edge (auto-assigned UUID id).
    pub fn new(from: impl Into<NodeId>, kind: RelationKind, to: impl Into<NodeId>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.into(),
            to: to.into(),
            kind,
            properties: serde_json::Value::Null,
        }
    }

    /// Attach property bag.
    pub fn with_properties(mut self, props: serde_json::Value) -> Self {
        self.properties = props;
        self
    }

    /// Is this a self-loop edge (from == to)?
    pub fn is_self_loop(&self) -> bool {
        self.from == self.to
    }
}

// ============================================================================
// §3 RelationGraph — main storage
// ============================================================================

/// Graph storage with adjacency-list indexes.
#[derive(Debug, Default, Clone)]
pub struct RelationGraph {
    /// Node index by ID.
    nodes: HashMap<NodeId, GraphNode>,
    /// Edge index by ID.
    edges: HashMap<EdgeId, GraphEdge>,
    /// Outgoing edges per node (from_node -> edge ids).
    out_edges: HashMap<NodeId, HashSet<EdgeId>>,
    /// Incoming edges per node (to_node -> edge ids).
    in_edges: HashMap<NodeId, HashSet<EdgeId>>,
    /// Edges grouped by kind (kind -> edge ids).
    edges_by_kind: HashMap<RelationKind, HashSet<EdgeId>>,
}

impl RelationGraph {
    /// Create empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a node. Returns error if id already present.
    pub fn insert_node(&mut self, node: GraphNode) -> GraphResult<()> {
        if self.nodes.contains_key(&node.id) {
            return Err(GraphError::DuplicateNode(node.id));
        }
        self.out_edges.insert(node.id.clone(), HashSet::new());
        self.in_edges.insert(node.id.clone(), HashSet::new());
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Insert an edge. Both endpoints must already exist as nodes.
    pub fn insert_edge(&mut self, edge: GraphEdge) -> GraphResult<()> {
        if self.edges.contains_key(&edge.id) {
            return Err(GraphError::DuplicateEdge(edge.id));
        }
        if !self.nodes.contains_key(&edge.from) {
            return Err(GraphError::InvalidEdgeEndpoint(edge.from));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(GraphError::InvalidEdgeEndpoint(edge.to));
        }
        let edge_id = edge.id.clone();
        let from = edge.from.clone();
        let to = edge.to.clone();
        let kind = edge.kind;
        self.edges.insert(edge_id.clone(), edge);
        self.out_edges
            .entry(from)
            .or_default()
            .insert(edge_id.clone());
        self.in_edges.entry(to).or_default().insert(edge_id.clone());
        self.edges_by_kind.entry(kind).or_default().insert(edge_id);
        Ok(())
    }

    /// Remove a node and all incident edges.
    pub fn remove_node(&mut self, id: &str) -> GraphResult<GraphNode> {
        let node = self
            .nodes
            .remove(id)
            .ok_or_else(|| GraphError::NodeNotFound(id.to_string()))?;

        // Remove incident edges (collect first to avoid borrow issues)
        let mut incident: Vec<EdgeId> = Vec::new();
        if let Some(s) = self.out_edges.get(id) {
            incident.extend(s.iter().cloned());
        }
        if let Some(s) = self.in_edges.get(id) {
            incident.extend(s.iter().cloned());
        }
        incident.sort();
        incident.dedup();

        for edge_id in incident {
            self.remove_edge(&edge_id)?;
        }

        self.out_edges.remove(id);
        self.in_edges.remove(id);
        Ok(node)
    }

    /// Remove an edge.
    pub fn remove_edge(&mut self, id: &str) -> GraphResult<GraphEdge> {
        let edge = self
            .edges
            .remove(id)
            .ok_or_else(|| GraphError::EdgeNotFound(id.to_string()))?;
        if let Some(set) = self.out_edges.get_mut(&edge.from) {
            set.remove(id);
        }
        if let Some(set) = self.in_edges.get_mut(&edge.to) {
            set.remove(id);
        }
        if let Some(set) = self.edges_by_kind.get_mut(&edge.kind) {
            set.remove(id);
        }
        Ok(edge)
    }

    /// Get a node by id.
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// Get an edge by id.
    pub fn get_edge(&self, id: &str) -> Option<&GraphEdge> {
        self.edges.get(id)
    }

    /// Total nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// All node IDs.
    pub fn node_ids(&self) -> Vec<&NodeId> {
        self.nodes.keys().collect()
    }

    /// All edges.
    pub fn all_edges(&self) -> Vec<&GraphEdge> {
        self.edges.values().collect()
    }

    /// Outgoing edges from a node.
    pub fn outgoing(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.out_edges
            .get(node_id)
            .map(|ids| ids.iter().filter_map(|eid| self.edges.get(eid)).collect())
            .unwrap_or_default()
    }

    /// Incoming edges to a node.
    pub fn incoming(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.in_edges
            .get(node_id)
            .map(|ids| ids.iter().filter_map(|eid| self.edges.get(eid)).collect())
            .unwrap_or_default()
    }

    /// Edges of a given kind.
    pub fn edges_of_kind(&self, kind: RelationKind) -> Vec<&GraphEdge> {
        self.edges_by_kind
            .get(&kind)
            .map(|ids| ids.iter().filter_map(|eid| self.edges.get(eid)).collect())
            .unwrap_or_default()
    }

    /// Find shortest path between two nodes (BFS, undirected).
    /// Returns `None` if no path exists.
    pub fn shortest_path(&self, from: &str, to: &str) -> Option<Vec<NodeId>> {
        if !self.nodes.contains_key(from) || !self.nodes.contains_key(to) {
            return None;
        }
        if from == to {
            return Some(vec![from.to_string()]);
        }
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<(NodeId, Vec<NodeId>)> = VecDeque::new();
        queue.push_back((from.to_string(), vec![from.to_string()]));
        visited.insert(from.to_string());

        while let Some((current, path)) = queue.pop_front() {
            // Traverse outgoing + incoming (undirected)
            let mut neighbors: HashSet<NodeId> = HashSet::new();
            for edge in self.outgoing(&current) {
                neighbors.insert(edge.to.clone());
            }
            for edge in self.incoming(&current) {
                neighbors.insert(edge.from.clone());
            }

            for next in neighbors {
                if next == to {
                    let mut full_path = path.clone();
                    full_path.push(next);
                    return Some(full_path);
                }
                if !visited.contains(&next) {
                    visited.insert(next.clone());
                    let mut new_path = path.clone();
                    new_path.push(next.clone());
                    queue.push_back((next, new_path));
                }
            }
        }
        None
    }
}

// ============================================================================
// §4 Conversion from RelationRegistry
// ============================================================================

impl From<&RelationRegistry> for RelationGraph {
    fn from(registry: &RelationRegistry) -> Self {
        let mut graph = Self::new();
        // First pass: insert all nodes (one per unique party)
        let mut party_set: HashSet<String> = HashSet::new();
        for r in registry.all() {
            for p in r.involved_parties() {
                party_set.insert(p.to_string());
            }
        }
        for party in party_set {
            let _ = graph.insert_node(GraphNode::new(party));
        }
        // Second pass: insert all edges
        for r in registry.all() {
            let edge = GraphEdge::new(r.party_a.clone(), r.kind, r.party_b.clone());
            // Silently ignore duplicate edges (registry may have multiple of same kind)
            let _ = graph.insert_edge(edge);
        }
        graph
    }
}

// ============================================================================
// §5 Re-export for lib.rs
// ============================================================================

// Note: the public API of these types is exposed via `lib.rs` re-exports.

// ============================================================================
// §6 Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelationKind;

    fn sample_graph() -> RelationGraph {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::with_kind("alice", "agent"))
            .unwrap();
        g.insert_node(GraphNode::with_kind("bob", "agent")).unwrap();
        g.insert_node(GraphNode::with_kind("carol", "agent"))
            .unwrap();
        g.insert_node(GraphNode::with_kind("tools", "concept"))
            .unwrap();
        g.insert_edge(GraphEdge::new("alice", RelationKind::Symbiosis, "bob"))
            .unwrap();
        g.insert_edge(GraphEdge::new("alice", RelationKind::Coordination, "carol"))
            .unwrap();
        g.insert_edge(GraphEdge::new("alice", RelationKind::Embedding, "tools"))
            .unwrap();
        g
    }

    #[test]
    fn test_insert_node_ok() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("x")).unwrap();
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn test_insert_duplicate_node_rejected() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("x")).unwrap();
        let err = g.insert_node(GraphNode::new("x")).unwrap_err();
        assert!(matches!(err, GraphError::DuplicateNode(_)));
    }

    #[test]
    fn test_insert_edge_requires_existing_nodes() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("a")).unwrap();
        let err = g
            .insert_edge(GraphEdge::new("a", RelationKind::Symbiosis, "ghost"))
            .unwrap_err();
        assert!(matches!(err, GraphError::InvalidEdgeEndpoint(_)));
    }

    #[test]
    fn test_outgoing_and_incoming() {
        let g = sample_graph();
        assert_eq!(g.outgoing("alice").len(), 3);
        assert_eq!(g.outgoing("bob").len(), 0);
        assert_eq!(g.incoming("bob").len(), 1);
    }

    #[test]
    fn test_edges_of_kind() {
        let g = sample_graph();
        assert_eq!(g.edges_of_kind(RelationKind::Symbiosis).len(), 1);
        assert_eq!(g.edges_of_kind(RelationKind::Coordination).len(), 1);
        assert_eq!(g.edges_of_kind(RelationKind::Embedding).len(), 1);
        assert_eq!(g.edges_of_kind(RelationKind::SelfRelation).len(), 0);
    }

    #[test]
    fn test_remove_node_cascades_edges() {
        let mut g = sample_graph();
        let removed = g.remove_node("alice").unwrap();
        assert_eq!(removed.id, "alice");
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 0, "all incident edges must be removed");
    }

    #[test]
    fn test_remove_nonexistent_node_rejected() {
        let mut g = RelationGraph::new();
        let err = g.remove_node("ghost").unwrap_err();
        assert!(matches!(err, GraphError::NodeNotFound(_)));
    }

    #[test]
    fn test_remove_edge_decrements_indexes() {
        let mut g = sample_graph();
        let edges = g.outgoing("alice");
        let edge_id = edges[0].id.clone();
        g.remove_edge(&edge_id).unwrap();
        assert_eq!(g.edge_count(), 2);
        assert_eq!(g.outgoing("alice").len(), 2);
    }

    #[test]
    fn test_self_loop_edge() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("agent")).unwrap();
        let edge = GraphEdge::new("agent", RelationKind::SelfRelation, "agent");
        assert!(edge.is_self_loop());
        g.insert_edge(edge).unwrap();
    }

    #[test]
    fn test_shortest_path_direct() {
        let g = sample_graph();
        let path = g.shortest_path("alice", "bob").unwrap();
        assert_eq!(path, vec!["alice".to_string(), "bob".to_string()]);
    }

    #[test]
    fn test_shortest_path_transitive() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("a")).unwrap();
        g.insert_node(GraphNode::new("b")).unwrap();
        g.insert_node(GraphNode::new("c")).unwrap();
        g.insert_edge(GraphEdge::new("a", RelationKind::Coordination, "b"))
            .unwrap();
        g.insert_edge(GraphEdge::new("b", RelationKind::Coordination, "c"))
            .unwrap();
        let path = g.shortest_path("a", "c").unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], "a");
        assert_eq!(path[2], "c");
    }

    #[test]
    fn test_shortest_path_self() {
        let g = sample_graph();
        let path = g.shortest_path("alice", "alice").unwrap();
        assert_eq!(path, vec!["alice".to_string()]);
    }

    #[test]
    fn test_shortest_path_none_for_disconnected() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("x")).unwrap();
        g.insert_node(GraphNode::new("y")).unwrap();
        let path = g.shortest_path("x", "y");
        assert!(path.is_none());
    }

    #[test]
    fn test_shortest_path_none_for_missing_node() {
        let g = sample_graph();
        assert!(g.shortest_path("alice", "ghost").is_none());
        assert!(g.shortest_path("ghost", "alice").is_none());
    }

    #[test]
    fn test_from_relation_registry() {
        let mut reg = RelationRegistry::new();
        reg.register(Relation::new_symbiosis("a", "b").unwrap());
        reg.register(Relation::new_coordination("b", "c").unwrap());
        reg.register(Relation::new_self_relation("a").unwrap());
        let g = RelationGraph::from(&reg);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn test_node_with_properties() {
        let node = GraphNode::with_kind("alice", "agent")
            .with_properties(serde_json::json!({"role": "assistant", "level": 5}));
        assert_eq!(node.kind.as_deref(), Some("agent"));
        assert_eq!(
            node.properties.get("role").and_then(|v| v.as_str()),
            Some("assistant")
        );
        assert_eq!(
            node.properties.get("level").and_then(|v| v.as_i64()),
            Some(5)
        );
    }

    #[test]
    fn test_edge_with_properties() {
        let edge = GraphEdge::new("alice", RelationKind::Coordination, "bob")
            .with_properties(serde_json::json!({"since": "2026-01-01"}));
        assert_eq!(
            edge.properties.get("since").and_then(|v| v.as_str()),
            Some("2026-01-01")
        );
    }
}
