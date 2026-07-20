//! Relation Graph — Episodic + Semantic 双图 (借鉴 AriGraph + Graphiti)
//!
//! 主人 13:47 "按模块按步骤科学造" — Relation Graph 是 schema, 涌现从关系来
//! 主人 12:14 "中央 AI 是多身份" → 关系图谱 schema 是关键
//!
//! 借鉴:
//! - AriGraph (arxiv 2407.04363): Knowledge Graph + Episodic Memory
//! - Graphiti (github.com/getzep/graphiti): temporal validity + entity extraction

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 节点 — 实体 (人 / 概念 / 项目)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub nid: String,
    pub label: String,           // "Person" / "Project" / "Concept"
    pub properties: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// 边 — 关系 (有 temporal validity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub eid: String,
    pub from: String,            // node nid
    pub to: String,              // node nid
    pub relation: String,        // "works_on" / "knows_about" / "is_a"
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>, // None = 仍然有效
    pub evidence: Vec<String>,   // Episode eids
    pub confidence: f64,
}

/// Episodic 边 — 来自具体事件的临时关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicEdge {
    pub episode_id: String,
    pub from: String,
    pub to: String,
    pub observed_relation: String,
    pub ts: DateTime<Utc>,
}

/// Relation Graph — Episodic + Semantic 双层
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub episodic_edges: Vec<EpisodicEdge>,
}

impl RelationGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            episodic_edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, label: impl Into<String>, properties: serde_json::Value) -> String {
        let nid = Uuid::new_v4().simple().to_string()[..16].to_string();
        self.nodes.push(Node {
            nid: nid.clone(),
            label: label.into(),
            properties,
            created_at: Utc::now(),
        });
        nid
    }

    pub fn add_edge(
        &mut self,
        from: String,
        to: String,
        relation: impl Into<String>,
        evidence: Vec<String>,
        confidence: f64,
    ) -> String {
        let eid = Uuid::new_v4().simple().to_string()[..16].to_string();
        self.edges.push(Edge {
            eid: eid.clone(),
            from,
            to,
            relation: relation.into(),
            valid_from: Utc::now(),
            valid_until: None,
            evidence,
            confidence,
        });
        eid
    }

    /// 记录 episodic edge (临时观察)
    pub fn observe(
        &mut self,
        episode_id: impl Into<String>,
        from: String,
        to: String,
        relation: impl Into<String>,
    ) {
        self.episodic_edges.push(EpisodicEdge {
            episode_id: episode_id.into(),
            from,
            to,
            observed_relation: relation.into(),
            ts: Utc::now(),
        });
    }

    /// Promote episodic edge → semantic edge (Graphiti pattern)
    pub fn consolidate_episodic(&mut self, threshold: usize) -> usize {
        let mut promoted = 0;
        // 简化: 同 (from, to, relation) 出现 ≥ threshold 次 → promote
        let groups = self.group_episodic_by_triple();
        for (triple, count) in groups {
            if count >= threshold {
                self.add_edge(triple.0, triple.2, triple.1, vec![], 1.0);
                promoted += 1;
            }
        }
        promoted
    }

    fn group_episodic_by_triple(&self) -> std::collections::HashMap<(String, String, String), usize> {
        let mut map: std::collections::HashMap<(String, String, String), usize> = Default::default();
        for e in &self.episodic_edges {
            *map.entry((e.from.clone(), e.observed_relation.clone(), e.to.clone())).or_insert(0) += 1;
        }
        map
    }
}

impl Default for RelationGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_graph_basic() {
        let mut g = RelationGraph::new();
        let master = g.add_node("Person", serde_json::json!({"name": "Master"}));
        let apeireth = g.add_node("AI", serde_json::json!({"name": "Apeireth"}));
        g.observe("ep1", master.clone(), apeireth.clone(), "creates");
        g.observe("ep2", master.clone(), apeireth.clone(), "creates");
        g.observe("ep3", master.clone(), apeireth.clone(), "creates");
        let promoted = g.consolidate_episodic(3);
        assert_eq!(promoted, 1);
        assert_eq!(g.edges.len(), 1);
    }
}