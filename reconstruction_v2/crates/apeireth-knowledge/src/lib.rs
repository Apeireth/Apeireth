//! apeireth-knowledge - Knowledge graph (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 Node + Edge + 真 query

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KNode { pub id: String, pub label: String, pub kind: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KEdge { pub from: String, pub to: String, pub rel: String }

pub struct KnowledgeGraph { pub nodes: HashMap<String, KNode>, pub edges: Vec<KEdge> }

impl KnowledgeGraph {
    pub fn new() -> Self { Self { nodes: HashMap::new(), edges: vec![] } }
    pub fn add_node(&mut self, n: KNode) { self.nodes.insert(n.id.clone(), n); }
    pub fn add_edge(&mut self, e: KEdge) { self.edges.push(e); }
    pub fn neighbors(&self, id: &str) -> Vec<&KNode> {
        self.edges.iter().filter(|e| e.from == id).filter_map(|e| self.nodes.get(&e.to)).collect()
    }
    pub fn count_nodes(&self) -> usize { self.nodes.len() }
}

impl Default for KnowledgeGraph { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_neighbor() {
        let mut g = KnowledgeGraph::new();
        g.add_node(KNode { id: "a".into(), label: "A".into(), kind: "x".into() });
        g.add_node(KNode { id: "b".into(), label: "B".into(), kind: "x".into() });
        g.add_edge(KEdge { from: "a".into(), to: "b".into(), rel: "r".into() });
        let n = g.neighbors("a");
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].id, "b");
    }
    #[test]
    fn test_empty() {
        let g = KnowledgeGraph::new();
        assert_eq!(g.count_nodes(), 0);
    }
}
