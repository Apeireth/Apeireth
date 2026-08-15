//! apeireth-experience: 经验沉淀层 (R173/Stage2 §3).
//!
//! **职责**: 3 类经验沉淀 — LLM Wiki + Knowledge Graph + VCP 联想网络.
//!
//! 借鉴 (per stage2):
//! - claude-mem 3-layer progressive disclosure
//! - safishamsi/graphify EXTRACTED/INFERRED 双层
//! - vcptoolbox compound_eye 联想网络
//! - MemPalace 物理化记忆
//!
//! **不漂移**:
//! - 0 改 apeireth-memory 任何已实装类型
//! - 0 副作用: 联想传播是 mutating 但原子
//!
//! **当前状态**: R173 阶段 6 后端补全 — 3 模块 + 入口.

#![deny(unsafe_code)]

pub mod association;
pub mod graph;
pub mod wiki;
// R177: experience invariants
mod organ_kani_proofs;
pub mod council_bridge;  // R174: experience -> council (wiki + KG + association) bridge

// Re-exports 公共 API
pub use association::{AssociationEdge, AssociationNetwork, AssociationNode};
pub use graph::{KnowledgeEdge, KnowledgeGraph, KnowledgeNode, NodeKind, RelationKind};
pub use wiki::WikiEntry;
pub use council_bridge::{
    association_to_context_block, bundle_to_history_refs, kg_to_context_block,
    wiki_to_context_block, wiki_to_history_ref,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_kg_association_integration() {
        // 1. 创建 Wiki entry
        let mut wiki = WikiEntry::new("Rust ownership", "Lending rules", 0.9)
            .with_tag("rust")
            .with_source("ep-1");
        assert_eq!(wiki.title, "Rust ownership");
        // 2. 升迁 1 次
        wiki.promote();
        assert_eq!(wiki.promotion_count, 1);
        // 3. 关联 KG
        let mut kg = KnowledgeGraph::new();
        let n1 = kg.add_node(KnowledgeNode::new("Rust", NodeKind::Extracted, 0.9, "ep-1"));
        let n2 = kg.add_node(KnowledgeNode::new("Borrow checker", NodeKind::Inferred, 0.7, "ep-1"));
        kg.add_edge(crate::KnowledgeEdge::new(n1, n2, RelationKind::Coordination, 0.8)).unwrap();
        assert_eq!(kg.node_count(), 2);
        assert_eq!(kg.edge_count(), 1);
        // 4. 联想
        let mut assoc = AssociationNetwork::new();
        let a = assoc.add_node(AssociationNode::new("Rust", 0.0));
        let b = assoc.add_node(AssociationNode::new("Borrow checker", 0.0));
        assoc.connect(a, b, 1.0);
        let result = assoc.associate(a, 2);
        assert_eq!(result.len(), 2);
        assert!(result[0].2 >= result[1].2);
    }
}
