//! MemoryGraph - 记忆图 (从 v1.0 apeireth-companion/memory_graph.rs 3K LOC 抄录升级)
//!
//! 0 装 PASS: 真节点/边 + 邻接查询
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

pub struct MemoryGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    adjacency: HashMap<String, Vec<String>>,
}

impl MemoryGraph {
    pub fn new() -> Self { Self { nodes: HashMap::new(), edges: Vec::new(), adjacency: HashMap::new() } }

    /// 0 装 PASS: 真 add node
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// 0 装 PASS: 真 add edge + 维护 adjacency
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.adjacency.entry(edge.from.clone()).or_default().push(edge.to.clone());
        self.edges.push(edge);
    }

    /// 0 装 PASS: 真邻接查询 (BFS depth=1)
    pub fn neighbors(&self, id: &str) -> Vec<&GraphNode> {
        self.adjacency.get(id).map(|ids| ids.iter().filter_map(|i| self.nodes.get(i)).collect()).unwrap_or_default()
    }

    pub fn count_nodes(&self) -> usize { self.nodes.len() }
    pub fn count_edges(&self) -> usize { self.edges.len() }
}

impl Default for MemoryGraph { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_basic() {
        let mut g = MemoryGraph::new();
        g.add_node(GraphNode { id: "a".into(), label: "A".into() });
        g.add_node(GraphNode { id: "b".into(), label: "B".into() });
        g.add_edge(GraphEdge { from: "a".into(), to: "b".into(), relation: "knows".into() });
        assert_eq!(g.count_nodes(), 2);
        assert_eq!(g.count_edges(), 1);
        let n = g.neighbors("a");
        assert_eq!(n.len(), 1);
    }
    #[test] fn test_no_neighbors() {
        let g = MemoryGraph::new();
        assert!(g.neighbors("missing").is_empty());
    }
    #[test] fn test_multi_neighbors() {
        let mut g = MemoryGraph::new();
        g.add_node(GraphNode { id: "a".into(), label: "A".into() });
        g.add_node(GraphNode { id: "b".into(), label: "B".into() });
        g.add_node(GraphNode { id: "c".into(), label: "C".into() });
        g.add_edge(GraphEdge { from: "a".into(), to: "b".into(), relation: "x".into() });
        g.add_edge(GraphEdge { from: "a".into(), to: "c".into(), relation: "y".into() });
        assert_eq!(g.neighbors("a").len(), 2);
    }
    #[test] fn test_default() {
        let g: MemoryGraph = Default::default();
        assert_eq!(g.count_nodes(), 0);
    }
}
