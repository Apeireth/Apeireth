//! Graph query — predicate-based filtering of nodes/edges.
//!
//! ## Borrowed Reference (per O-5)
//!
//! Inspired by **SurrealDB** SELECT syntax (`SELECT * FROM table WHERE ...`)
//! and **Cypher** MATCH patterns. We provide a minimal predicate evaluator —
//! no full SQL parser. Use the builder API:
//!
//! ```ignore
//! use apeireth_relation::query::NodeQuery;
//!
//! let results = NodeQuery::new()
//!     .kind_eq(\"agent\")
//!     .property_string(\"role\", \"assistant\")
//!     .execute(&graph);
//! ```
//!
//! ## Why not full GQL?
//!
//! Full GQL is a multi-year ISO/IEC standard (JTC1). A real implementation
//! requires lexer + parser + AST + optimizer + executor — at minimum ~3-5K
//! LOC. For our scope we provide:
//! - `NodeQuery` — filter nodes by kind + property equality
//! - `EdgeQuery` — filter edges by kind + endpoint + property equality
//! - `CombinedQuery` — node filter + 1-hop edge expansion (graph traversal in spirit)
//!
//! 0 引外部 dep. Predicate evaluation is O(n) over matched set.

#![allow(missing_docs)]
#![allow(clippy::all)]

use serde_json::Value;

use crate::graph::{GraphEdge, GraphNode, RelationGraph};
use crate::RelationKind;

// ============================================================================
// §1 Property predicate
// ============================================================================

/// Property equality predicate: matches when `properties[key] == value`.
/// JSON equality (deep). Missing property = no match.
#[derive(Debug, Clone)]
pub struct PropertyMatch {
    /// Property key.
    pub key: String,
    /// Expected JSON value.
    pub value: Value,
}

impl PropertyMatch {
    /// Create a property match.
    pub fn new(key: impl Into<String>, value: Value) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }

    /// Evaluate against a property bag.
    pub fn matches(&self, properties: &Value) -> bool {
        properties
            .get(&self.key)
            .map(|v| v == &self.value)
            .unwrap_or(false)
    }
}

// ============================================================================
// §2 NodeQuery
// ============================================================================

/// Node filter: kind + property matches (AND).
#[derive(Debug, Default, Clone)]
pub struct NodeQuery {
    /// Optional kind filter.
    pub kind: Option<String>,
    /// Optional property matches (all must match).
    pub properties: Vec<PropertyMatch>,
}

impl NodeQuery {
    /// Empty query (matches all nodes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by node kind.
    pub fn kind_eq(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Add a property equality predicate.
    pub fn property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.push(PropertyMatch::new(key, value));
        self
    }

    /// Add a property string predicate (convenience).
    pub fn property_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties
            .push(PropertyMatch::new(key, Value::String(value.into())));
        self
    }

    /// Check whether a single node matches.
    pub fn matches(&self, node: &GraphNode) -> bool {
        if let Some(ref k) = self.kind {
            if node.kind.as_deref() != Some(k.as_str()) {
                return false;
            }
        }
        for p in &self.properties {
            if !p.matches(&node.properties) {
                return false;
            }
        }
        true
    }

    /// Execute the query, returning matching nodes.
    pub fn execute<'g>(&self, graph: &'g RelationGraph) -> Vec<&'g GraphNode> {
        graph
            .node_ids()
            .into_iter()
            .filter_map(|id| graph.get_node(id))
            .filter(|n| self.matches(n))
            .collect()
    }
}

// ============================================================================
// §3 EdgeQuery
// ============================================================================

/// Edge filter: kind + endpoint + property matches (AND).
#[derive(Debug, Default, Clone)]
pub struct EdgeQuery {
    /// Optional edge kind filter.
    pub kind: Option<RelationKind>,
    /// Optional source node filter.
    pub from: Option<String>,
    /// Optional target node filter.
    pub to: Option<String>,
    /// Optional property matches.
    pub properties: Vec<PropertyMatch>,
}

impl EdgeQuery {
    /// Empty query (matches all edges).
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by relation kind.
    pub fn kind(mut self, kind: RelationKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Filter by source node.
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Filter by target node.
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = Some(to.into());
        self
    }

    /// Add a property equality predicate.
    pub fn property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.push(PropertyMatch::new(key, value));
        self
    }

    /// Check whether a single edge matches.
    pub fn matches(&self, edge: &GraphEdge) -> bool {
        if let Some(k) = self.kind {
            if edge.kind != k {
                return false;
            }
        }
        if let Some(ref f) = self.from {
            if edge.from != *f {
                return false;
            }
        }
        if let Some(ref t) = self.to {
            if edge.to != *t {
                return false;
            }
        }
        for p in &self.properties {
            if !p.matches(&edge.properties) {
                return false;
            }
        }
        true
    }

    /// Execute the query, returning matching edges.
    pub fn execute<'g>(&self, graph: &'g RelationGraph) -> Vec<&'g GraphEdge> {
        graph
            .all_edges()
            .into_iter()
            .filter(|e| self.matches(e))
            .collect()
    }
}

// ============================================================================
// §4 CombinedQuery — node + edge filter
// ============================================================================

/// Combined query: filter nodes + filter edges (independently). Both result
/// sets are returned. Edges are also restricted to incident-on-filtered-nodes
/// if `restrict_to_nodes` is true.
#[derive(Debug, Default, Clone)]
pub struct CombinedQuery {
    /// Node query.
    pub nodes: NodeQuery,
    /// Edge query.
    pub edges: EdgeQuery,
    /// If true, edges are restricted to those incident on filtered nodes.
    pub restrict_to_nodes: bool,
}

impl CombinedQuery {
    /// Create a combined query.
    pub fn new(nodes: NodeQuery, edges: EdgeQuery) -> Self {
        Self {
            nodes,
            edges,
            restrict_to_nodes: false,
        }
    }

    /// Set restrict_to_nodes flag.
    pub fn restrict_to_nodes(mut self, flag: bool) -> Self {
        self.restrict_to_nodes = flag;
        self
    }

    /// Execute the query.
    pub fn execute<'g>(
        &self,
        graph: &'g RelationGraph,
    ) -> (Vec<&'g GraphNode>, Vec<&'g GraphEdge>) {
        let nodes = self.nodes.execute(graph);
        let mut edges = self.edges.execute(graph);

        if self.restrict_to_nodes {
            let node_ids: std::collections::HashSet<String> =
                nodes.iter().map(|n| n.id.clone()).collect();
            edges.retain(|e| {
                node_ids.contains(&e.from) || node_ids.contains(&e.to)
            });
        }

        (nodes, edges)
    }
}

// ============================================================================
// §5 Convenience: count_by_kind
// ============================================================================

/// Count edges by kind (returns 4-tuple of counts in fixed order).
pub fn count_by_kind(graph: &RelationGraph) -> (usize, usize, usize, usize) {
    (
        graph.edges_of_kind(RelationKind::Symbiosis).len(),
        graph.edges_of_kind(RelationKind::Coordination).len(),
        graph.edges_of_kind(RelationKind::Embedding).len(),
        graph.edges_of_kind(RelationKind::SelfRelation).len(),
    )
}

// ============================================================================
// §6 Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, RelationGraph};
    use crate::RelationKind;

    fn populated_graph() -> RelationGraph {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::with_kind("alice", "agent")
            .with_properties(serde_json::json!({"role": "assistant", "level": 5})))
            .unwrap();
        g.insert_node(GraphNode::with_kind("bob", "agent")
            .with_properties(serde_json::json!({"role": "user", "level": 3})))
            .unwrap();
        g.insert_node(GraphNode::with_kind("carol", "tool")
            .with_properties(serde_json::json!({"category": "compute"})))
            .unwrap();
        g.insert_edge(GraphEdge::new("alice", RelationKind::Symbiosis, "bob")).unwrap();
        g.insert_edge(GraphEdge::new("alice", RelationKind::Coordination, "carol")).unwrap();
        g
    }

    #[test]
    fn test_property_match_basic() {
        let pm = PropertyMatch::new("role", serde_json::json!("assistant"));
        let props = serde_json::json!({"role": "assistant", "level": 5});
        assert!(pm.matches(&props));
    }

    #[test]
    fn test_property_match_mismatch() {
        let pm = PropertyMatch::new("role", serde_json::json!("assistant"));
        let props = serde_json::json!({"role": "user"});
        assert!(!pm.matches(&props));
    }

    #[test]
    fn test_property_match_missing_key() {
        let pm = PropertyMatch::new("missing", serde_json::json!("x"));
        let props = serde_json::json!({"other": "y"});
        assert!(!pm.matches(&props));
    }

    #[test]
    fn test_node_query_kind_eq() {
        let g = populated_graph();
        let results = NodeQuery::new().kind_eq("agent").execute(&g);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_node_query_property_string() {
        let g = populated_graph();
        let results = NodeQuery::new()
            .kind_eq("agent")
            .property_string("role", "assistant")
            .execute(&g);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "alice");
    }

    #[test]
    fn test_node_query_property_int() {
        let g = populated_graph();
        let results = NodeQuery::new()
            .kind_eq("agent")
            .property("level", serde_json::json!(3))
            .execute(&g);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "bob");
    }

    #[test]
    fn test_node_query_no_match() {
        let g = populated_graph();
        let results = NodeQuery::new().kind_eq("nonexistent").execute(&g);
        assert!(results.is_empty());
    }

    #[test]
    fn test_node_query_empty_matches_all() {
        let g = populated_graph();
        let results = NodeQuery::new().execute(&g);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_edge_query_kind() {
        let g = populated_graph();
        let results = EdgeQuery::new().kind(RelationKind::Symbiosis).execute(&g);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, RelationKind::Symbiosis);
    }

    #[test]
    fn test_edge_query_endpoint() {
        let g = populated_graph();
        let results = EdgeQuery::new()
            .from("alice")
            .to("carol")
            .execute(&g);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_edge_query_combined() {
        let g = populated_graph();
        let results = EdgeQuery::new()
            .kind(RelationKind::Coordination)
            .from("alice")
            .execute(&g);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_combined_query_with_restrict() {
        let g = populated_graph();
        let nodes_q = NodeQuery::new().kind_eq("agent");
        let edges_q = EdgeQuery::new(); // all
        let (nodes, edges) = CombinedQuery::new(nodes_q, edges_q)
            .restrict_to_nodes(true)
            .execute(&g);
        assert_eq!(nodes.len(), 2);
        // Edges incident on agent nodes (OR semantics): alice-bob (Symbiosis)
        // and alice-carol (Coordination, since alice is an agent).
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_combined_query_without_restrict() {
        let g = populated_graph();
        let nodes_q = NodeQuery::new().kind_eq("agent");
        let edges_q = EdgeQuery::new();
        let (nodes, edges) = CombinedQuery::new(nodes_q, edges_q).execute(&g);
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges.len(), 2); // all edges, not restricted
    }

    #[test]
    fn test_count_by_kind() {
        let g = populated_graph();
        let (sym, coord, emb, self_rel) = count_by_kind(&g);
        assert_eq!(sym, 1);
        assert_eq!(coord, 1);
        assert_eq!(emb, 0);
        assert_eq!(self_rel, 0);
    }
}
