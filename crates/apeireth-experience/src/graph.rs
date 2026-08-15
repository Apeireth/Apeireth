//! Knowledge Graph — 节点 + 边 (per stage2 §3 + Stage1 §13 候选).
//!
//! 借鉴 safishamsi/graphify EXTRACTED/INFERRED 双层知识图谱.
//!
//! 节点/边分类 (per stage2):
//! - EXTRACTED: LLM 直接抽取的事实
//! - INFERRED:  推理/关联产生的事实
//!
//! 边类型 (4 类 per apeireth-graph-primitive 4 关系对应):
//! - Symbiosis    互利
//! - Coordination 协调
//! - Embedding    包含
//! - SelfRelation 自身

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    /// 互利共生
    Symbiosis,
    /// 协调
    Coordination,
    /// 包含
    Embedding,
    /// 自身
    SelfRelation,
}

impl RelationKind {
    pub const ALL: [RelationKind; 4] = [
        Self::Symbiosis,
        Self::Coordination,
        Self::Embedding,
        Self::SelfRelation,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Symbiosis => "symbiosis",
            Self::Coordination => "coordination",
            Self::Embedding => "embedding",
            Self::SelfRelation => "self_relation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    /// LLM 抽取的事实
    Extracted,
    /// 推理产生的事实
    Inferred,
}

impl NodeKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Extracted => "extracted",
            Self::Inferred => "inferred",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: Uuid,
    pub label: String,
    pub kind: NodeKind,
    pub confidence: f64,
    pub source: String,
    pub created_at: i64,
}

impl KnowledgeNode {
    pub fn new(label: impl Into<String>, kind: NodeKind, confidence: f64, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: label.into(),
            kind,
            confidence: confidence.clamp(0.0, 1.0),
            source: source.into(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub id: Uuid,
    pub from: Uuid,
    pub to: Uuid,
    pub kind: RelationKind,
    pub weight: f64,
    pub created_at: i64,
}

impl KnowledgeEdge {
    pub fn new(from: Uuid, to: Uuid, kind: RelationKind, weight: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            kind,
            weight: weight.clamp(0.0, 1.0),
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Default)]
pub struct KnowledgeGraph {
    nodes: std::collections::HashMap<Uuid, KnowledgeNode>,
    edges: Vec<KnowledgeEdge>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: KnowledgeNode) -> Uuid {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> Result<(), String> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(format!("from node {} not found", edge.from));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(format!("to node {} not found", edge.to));
        }
        self.edges.push(edge);
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn node(&self, id: &Uuid) -> Option<&KnowledgeNode> {
        self.nodes.get(id)
    }

    pub fn edges_from(&self, id: Uuid) -> Vec<&KnowledgeEdge> {
        self.edges.iter().filter(|e| e.from == id).collect()
    }

    pub fn edges_to(&self, id: Uuid) -> Vec<&KnowledgeEdge> {
        self.edges.iter().filter(|e| e.to == id).collect()
    }

    pub fn edges_of_kind(&self, kind: RelationKind) -> Vec<&KnowledgeEdge> {
        self.edges.iter().filter(|e| e.kind == kind).collect()
    }

    pub fn nodes_of_kind(&self, kind: NodeKind) -> Vec<&KnowledgeNode> {
        self.nodes.values().filter(|n| n.kind == kind).collect()
    }

    /// BFS 遍历, 从 start 出发, max_depth 限定
    pub fn bfs_reachable(&self, start: Uuid, max_depth: usize) -> std::collections::HashSet<Uuid> {
        let mut visited = std::collections::HashSet::new();
        if !self.nodes.contains_key(&start) {
            return visited;
        }
        let mut frontier = vec![(start, 0usize)];
        visited.insert(start);
        while let Some((node, depth)) = frontier.pop() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.edges_from(node) {
                if !visited.contains(&edge.to) {
                    visited.insert(edge.to);
                    frontier.push((edge.to, depth + 1));
                }
            }
        }
        visited
    }

    pub fn all_nodes(&self) -> Vec<&KnowledgeNode> {
        self.nodes.values().collect()
    }

    pub fn all_edges(&self) -> &[KnowledgeEdge] {
        &self.edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_node_increments_count() {
        let mut g = KnowledgeGraph::new();
        let n = KnowledgeNode::new("a", NodeKind::Extracted, 0.9, "ep-1");
        let id = g.add_node(n);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.node(&id).unwrap().label, "a");
    }

    #[test]
    fn add_edge_validates_nodes() {
        let mut g = KnowledgeGraph::new();
        let n1 = KnowledgeNode::new("a", NodeKind::Extracted, 0.9, "ep-1");
        let id1 = g.add_node(n1);
        let n2 = KnowledgeNode::new("b", NodeKind::Extracted, 0.9, "ep-1");
        let id2 = g.add_node(n2);
        let edge = KnowledgeEdge::new(Uuid::new_v4(), id2, RelationKind::Coordination, 0.5);
        assert!(g.add_edge(edge).is_err());
        let valid = KnowledgeEdge::new(id1, id2, RelationKind::Coordination, 0.5);
        assert!(g.add_edge(valid).is_ok());
    }

    #[test]
    fn edges_from_and_to() {
        let mut g = KnowledgeGraph::new();
        let n1 = KnowledgeNode::new("a", NodeKind::Extracted, 0.9, "ep-1");
        let n2 = KnowledgeNode::new("b", NodeKind::Extracted, 0.9, "ep-1");
        let n3 = KnowledgeNode::new("c", NodeKind::Extracted, 0.9, "ep-1");
        let id1 = g.add_node(n1);
        let id2 = g.add_node(n2);
        let id3 = g.add_node(n3);
        g.add_edge(KnowledgeEdge::new(id1, id2, RelationKind::Coordination, 0.5)).unwrap();
        g.add_edge(KnowledgeEdge::new(id2, id3, RelationKind::Coordination, 0.5)).unwrap();
        assert_eq!(g.edges_from(id1).len(), 1);
        assert_eq!(g.edges_to(id3).len(), 1);
    }

    #[test]
    fn bfs_reachable_with_depth_limit() {
        let mut g = KnowledgeGraph::new();
        let n1 = KnowledgeNode::new("a", NodeKind::Extracted, 0.9, "ep-1");
        let n2 = KnowledgeNode::new("b", NodeKind::Extracted, 0.9, "ep-1");
        let n3 = KnowledgeNode::new("c", NodeKind::Extracted, 0.9, "ep-1");
        let id1 = g.add_node(n1);
        let id2 = g.add_node(n2);
        let id3 = g.add_node(n3);
        g.add_edge(KnowledgeEdge::new(id1, id2, RelationKind::Coordination, 0.5)).unwrap();
        g.add_edge(KnowledgeEdge::new(id2, id3, RelationKind::Coordination, 0.5)).unwrap();
        let reachable = g.bfs_reachable(id1, 1);
        assert_eq!(reachable.len(), 2);
        assert!(reachable.contains(&id1));
        assert!(reachable.contains(&id2));
        assert!(!reachable.contains(&id3));
        let reachable2 = g.bfs_reachable(id1, 2);
        assert_eq!(reachable2.len(), 3);
    }

    #[test]
    fn filter_by_relation_kind() {
        let mut g = KnowledgeGraph::new();
        let n1 = KnowledgeNode::new("a", NodeKind::Extracted, 0.9, "ep-1");
        let n2 = KnowledgeNode::new("b", NodeKind::Extracted, 0.9, "ep-1");
        let id1 = g.add_node(n1);
        let id2 = g.add_node(n2);
        g.add_edge(KnowledgeEdge::new(id1, id2, RelationKind::Symbiosis, 0.5)).unwrap();
        g.add_edge(KnowledgeEdge::new(id1, id2, RelationKind::Coordination, 0.5)).unwrap();
        assert_eq!(g.edges_of_kind(RelationKind::Symbiosis).len(), 1);
        assert_eq!(g.edges_of_kind(RelationKind::Embedding).len(), 0);
    }

    #[test]
    fn nodes_of_kind_filter() {
        let mut g = KnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", NodeKind::Extracted, 0.9, "ep-1"));
        g.add_node(KnowledgeNode::new("b", NodeKind::Inferred, 0.5, "ep-1"));
        g.add_node(KnowledgeNode::new("c", NodeKind::Extracted, 0.9, "ep-1"));
        assert_eq!(g.nodes_of_kind(NodeKind::Extracted).len(), 2);
        assert_eq!(g.nodes_of_kind(NodeKind::Inferred).len(), 1);
    }

    #[test]
    fn relation_kind_as_str_covers_all() {
        for k in RelationKind::ALL {
            assert!(!k.as_str().is_empty());
        }
    }
}
