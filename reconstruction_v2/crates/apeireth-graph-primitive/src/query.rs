//! Predicate-based node/edge filtering.

use crate::graph::{GraphEdge, GraphNode, RelationGraph};

/// Property match.
#[derive(Debug, Clone)]
pub struct PropertyMatch {
    pub key: String,
    pub value: serde_json::Value,
}

impl PropertyMatch {
    pub fn new(key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self { key: key.into(), value: value.into() }
    }
}

/// Node query.
#[derive(Debug, Clone, Default)]
pub struct NodeQuery {
    pub labels: Vec<String>,
    pub property_matches: Vec<PropertyMatch>,
}

impl NodeQuery {
    pub fn new() -> Self { Self::default() }
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into()); self
    }
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.property_matches.push(PropertyMatch::new(key, value)); self
    }

    pub fn matches(&self, node: &GraphNode) -> bool {
        for lbl in &self.labels {
            if !node.labels.contains(lbl) { return false; }
        }
        for pm in &self.property_matches {
            if node.properties.get(&pm.key) != Some(&pm.value) { return false; }
        }
        true
    }
}

/// Edge query.
#[derive(Debug, Clone, Default)]
pub struct EdgeQuery {
    pub label: Option<String>,
    pub property_matches: Vec<PropertyMatch>,
}

impl EdgeQuery {
    pub fn new() -> Self { Self::default() }
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into()); self
    }

    pub fn matches(&self, edge: &GraphEdge) -> bool {
        if let Some(lbl) = &self.label {
            if &edge.label != lbl { return false; }
        }
        for pm in &self.property_matches {
            if edge.properties.get(&pm.key) != Some(&pm.value) { return false; }
        }
        true
    }
}

/// Combined query (nodes + edges).
#[derive(Debug, Clone, Default)]
pub struct CombinedQuery {
    pub node_query: NodeQuery,
    pub edge_query: EdgeQuery,
}

impl CombinedQuery {
    pub fn new() -> Self { Self::default() }
    pub fn node_query(mut self, q: NodeQuery) -> Self { self.node_query = q; self }
    pub fn edge_query(mut self, q: EdgeQuery) -> Self { self.edge_query = q; self }
}

/// Count nodes by kind label.
pub fn count_by_kind(g: &RelationGraph, label: &str) -> usize {
    g.all_nodes().filter(|n| n.labels.contains(label)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphNode;

    #[test]
    fn node_query_matches_label() {
        let n = GraphNode::new("x").with_label("Person");
        let q = NodeQuery::new().with_label("Person");
        assert!(q.matches(&n));
    }

    #[test]
    fn node_query_matches_property() {
        let n = GraphNode::new("x").with_property("age", 42);
        let q = NodeQuery::new().with_property("age", 42);
        assert!(q.matches(&n));
    }

    #[test]
    fn node_query_no_match() {
        let n = GraphNode::new("x");
        let q = NodeQuery::new().with_label("Person");
        assert!(!q.matches(&n));
    }

    #[test]
    fn edge_query_matches_label() {
        let e = GraphEdge::new(0, "a", "b", "knows");
        let q = EdgeQuery::new().with_label("knows");
        assert!(q.matches(&e));
    }

    #[test]
    fn combined_query_constructs() {
        let cq = CombinedQuery::new()
            .node_query(NodeQuery::new().with_label("Person"))
            .edge_query(EdgeQuery::new().with_label("knows"));
        assert!(!cq.node_query.labels.is_empty());
        assert!(cq.edge_query.label.is_some());
    }

    #[test]
    fn count_by_kind_works() {
        let mut g = RelationGraph::new();
        g.add_node(GraphNode::new("a").with_label("Person"));
        g.add_node(GraphNode::new("b").with_label("Person"));
        g.add_node(GraphNode::new("c").with_label("Animal"));
        assert_eq!(count_by_kind(&g, "Person"), 2);
        assert_eq!(count_by_kind(&g, "Animal"), 1);
        assert_eq!(count_by_kind(&g, "Robot"), 0);
    }
}
