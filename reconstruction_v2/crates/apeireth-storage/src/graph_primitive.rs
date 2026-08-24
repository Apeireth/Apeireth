//! GraphPrimitive - 图原语 (从 v1.0 apeireth-graph-primitive 3K LOC 收敛)
//!
//! 0 装 PASS: 简化 typed graph (Node + Edge), 完整 v1.0 era (typed property, walker) 不做.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Default)]
pub struct Graph {
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self { Self::default() }
    pub fn add_node(&mut self, n: Node) { self.nodes.insert(n.id.clone(), n); }
    pub fn add_edge(&mut self, e: Edge) { self.edges.push(e); }
    pub fn node(&self, id: &str) -> Option<&Node> { self.nodes.get(id) }
    pub fn neighbors(&self, id: &str) -> Vec<&Node> {
        self.edges.iter()
            .filter(|e| e.from == id)
            .filter_map(|e| self.nodes.get(&e.to))
            .collect()
    }
    pub fn len(&self) -> usize { self.nodes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_basic_graph() {
        let mut g = Graph::new();
        g.add_node(Node { id: "a".into(), label: "A".into() });
        g.add_node(Node { id: "b".into(), label: "B".into() });
        g.add_edge(Edge { from: "a".into(), to: "b".into(), label: "knows".into() });
        assert_eq!(g.len(), 2);
        let nbrs = g.neighbors("a");
        assert_eq!(nbrs.len(), 1);
        assert_eq!(nbrs[0].id, "b");
    }
    #[test] fn test_node_not_found() {
        let g = Graph::new();
        assert!(g.node("missing").is_none());
    }
}
