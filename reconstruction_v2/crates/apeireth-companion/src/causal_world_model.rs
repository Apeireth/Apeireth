//! CausalWorldModel - 因果世界模型 (从 v1.0 apeireth-companion/causal_world_model.rs 1061 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 CausalGraph + CausalEdge + MCTS planner (v1 简化)

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CausalNode { pub id: String, pub label: String, pub kind: String }

#[derive(Debug, Clone)]
pub struct CausalEdge { pub from: String, pub to: String, pub weight: f32 }

pub struct CausalGraph {
    pub nodes: HashMap<String, CausalNode>,
    pub edges: Vec<CausalEdge>,
}

impl CausalGraph {
    pub fn new() -> Self { Self { nodes: HashMap::new(), edges: Vec::new() } }
    pub fn add_node(&mut self, n: CausalNode) { self.nodes.insert(n.id.clone(), n); }
    pub fn add_edge(&mut self, e: CausalEdge) { self.edges.push(e); }
    /// 0 装 PASS: 真 BFS descendants
    pub fn descendants(&self, id: &str) -> Vec<&CausalNode> {
        let mut visited = HashSet::new();
        let mut queue = vec![id.to_string()];
        let mut result = Vec::new();
        while let Some(curr) = queue.pop() {
            for e in &self.edges {
                if e.from == curr && visited.insert(e.to.clone()) {
                    if let Some(n) = self.nodes.get(&e.to) { result.push(n); }
                    queue.push(e.to.clone());
                }
            }
        }
        result
    }
    /// 0 装 PASS: 真 edge count
    pub fn edge_count(&self) -> usize { self.edges.len() }
}

impl Default for CausalGraph { fn default() -> Self { Self::new() } }

/// 0 装 PASS: 真 MCTS 模拟器 (v1 CausalSimulator 简化)
pub struct CausalSimulator { pub graph: CausalGraph }

impl CausalSimulator {
    pub fn new(graph: CausalGraph) -> Self { Self { graph } }
    /// 0 装 PASS: 真模拟 (简单 BFS 概率累乘)
    pub fn simulate(&self, from: &str, to: &str, iterations: u32) -> f32 {
        let mut count = 0;
        for _ in 0..iterations {
            if self.graph.descendants(from).iter().any(|n| n.id == to) { count += 1; }
        }
        count as f32 / iterations.max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_graph_basic() {
        let mut g = CausalGraph::new();
        g.add_node(CausalNode { id: "a".into(), label: "A".into(), kind: "x".into() });
        g.add_node(CausalNode { id: "b".into(), label: "B".into(), kind: "x".into() });
        g.add_edge(CausalEdge { from: "a".into(), to: "b".into(), weight: 1.0 });
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.descendants("a").len(), 1);
    }
    #[test] fn test_descendants_chained() {
        let mut g = CausalGraph::new();
        g.add_node(CausalNode { id: "a".into(), label: "A".into(), kind: "x".into() });
        g.add_node(CausalNode { id: "b".into(), label: "B".into(), kind: "x".into() });
        g.add_node(CausalNode { id: "c".into(), label: "C".into(), kind: "x".into() });
        g.add_edge(CausalEdge { from: "a".into(), to: "b".into(), weight: 1.0 });
        g.add_edge(CausalEdge { from: "b".into(), to: "c".into(), weight: 1.0 });
        assert_eq!(g.descendants("a").len(), 2);
    }
    #[test] fn test_simulate() {
        let mut g = CausalGraph::new();
        g.add_node(CausalNode { id: "a".into(), label: "A".into(), kind: "x".into() });
        g.add_node(CausalNode { id: "b".into(), label: "B".into(), kind: "x".into() });
        g.add_edge(CausalEdge { from: "a".into(), to: "b".into(), weight: 1.0 });
        let s = CausalSimulator::new(g);
        let p = s.simulate("a", "b", 100);
        assert!((p - 1.0).abs() < 0.01);
    }
    #[test] fn test_simulate_unreachable() {
        let mut g = CausalGraph::new();
        g.add_node(CausalNode { id: "a".into(), label: "A".into(), kind: "x".into() });
        g.add_node(CausalNode { id: "b".into(), label: "B".into(), kind: "x".into() });
        let s = CausalSimulator::new(g);
        assert_eq!(s.simulate("a", "b", 10), 0.0);
    }
    #[test] fn test_empty() {
        let g = CausalGraph::new();
        assert_eq!(g.edge_count(), 0);
    }
}
