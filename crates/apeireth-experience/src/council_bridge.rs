//! R174 stage 6: experience -> council bridge.
//!
//! **Goal**: turn `apeireth-experience` artifacts (WikiEntry + KnowledgeGraph +
//! AssociationNetwork) into the raw values that a council deliberation can
//! consume (history_refs + context block).
//!
//! **Why this lives in experience, not council**: avoid cyclic dependency.
//! The bridge produces plain strings + counts; the council caller assembles
//! them into a `CouncilQuery` (no mutation of locked `CouncilQuery` /
//! `QueryContext` types).
//!
//! **0 drift**:
//! - 0 change to `apeireth-council`.
//! - 0 change to `apeireth-experience` existing modules.
//! - 0 unsafe, 0 IO.

#![deny(unsafe_code)]

use uuid::Uuid;

use crate::association::AssociationNetwork;
use crate::graph::{KnowledgeGraph, KnowledgeNode, RelationKind};
use crate::wiki::WikiEntry;

/// Single-line summary for a wiki entry (history_ref style).
pub fn wiki_to_history_ref(entry: &WikiEntry) -> String {
    format!(
        "wiki:{}:conf={:.3}:tags={}",
        entry.title,
        entry.confidence,
        entry.tags.join(",")
    )
}

/// Multi-line context block for a wiki entry.
pub fn wiki_to_context_block(entry: &WikiEntry) -> String {
    format!(
        "# Wiki: {}
- confidence: {:.3}
- tags: {}
- promotions: {}
- content: {}",
        entry.title,
        entry.confidence,
        if entry.tags.is_empty() {
            "(none)".to_string()
        } else {
            entry.tags.join(", ")
        },
        entry.promotion_count,
        entry.content
    )
}

/// KG summary -> context block (top N nodes by confidence).
pub fn kg_to_context_block(kg: &KnowledgeGraph, max_nodes: usize) -> String {
    let mut nodes: Vec<&KnowledgeNode> = kg.all_nodes();
    nodes.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let taken = nodes.into_iter().take(max_nodes);
    let mut out = String::from(
        "# Knowledge Graph (top by confidence)
",
    );
    for n in taken {
        let kind = match n.kind {
            crate::graph::NodeKind::Extracted => "EX",
            crate::graph::NodeKind::Inferred => "IN",
        };
        out.push_str(&format!(
            "- [{}] {} (conf={:.3})
",
            kind, n.label, n.confidence
        ));
    }
    let mut rel_counts = std::collections::HashMap::<RelationKind, usize>::new();
    for e in kg.all_edges() {
        *rel_counts.entry(e.kind).or_insert(0) += 1;
    }
    if !rel_counts.is_empty() {
        out.push_str(
            "
# Edges by relation
",
        );
        let mut sorted: Vec<(RelationKind, usize)> = rel_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (k, c) in sorted {
            out.push_str(&format!(
                "- {:?}: {}
",
                k, c
            ));
        }
    }
    out
}

/// Association summary -> context block (provided by neighbour ranks).
pub fn association_to_context_block(
    net: &mut AssociationNetwork,
    seed: Uuid,
    depth: usize,
) -> String {
    let neighbours = net.associate(seed, depth);
    if neighbours.is_empty() {
        return "# Association: (no neighbours)
"
        .into();
    }
    let mut out = format!(
        "# Association from node {} (depth {})
",
        seed, depth
    );
    for (id, label, score) in neighbours {
        out.push_str(&format!(
            "- {} ({}) score={:.3}
",
            label, id, score
        ));
    }
    out
}

/// One-shot bundle of history_refs the council caller can attach to a query.
pub fn bundle_to_history_refs(
    wiki: &[&WikiEntry],
    kg: &KnowledgeGraph,
    seed_node: Uuid,
    depth: usize,
) -> Vec<String> {
    let mut refs: Vec<String> = wiki.iter().map(|w| wiki_to_history_ref(w)).collect();
    refs.push(format!(
        "kg:nodes={},edges={}",
        kg.node_count(),
        kg.edge_count()
    ));
    refs.push(format!("association:seed={},depth={}", seed_node, depth));
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::association::AssociationNode;
    use crate::graph::{KnowledgeEdge, NodeKind};

    fn wiki(title: &str, content: &str, confidence: f64) -> WikiEntry {
        WikiEntry::new(title, content, confidence).with_tag("test")
    }

    #[test]
    fn wiki_to_history_ref_format() {
        let w = wiki("Rust", "ownership", 0.9);
        let r = wiki_to_history_ref(&w);
        assert!(r.contains("wiki:Rust"));
        assert!(r.contains("conf=0.900"));
        assert!(r.contains("tags=test"));
    }

    #[test]
    fn wiki_to_context_block_includes_fields() {
        let w = wiki("A", "B", 0.5);
        let s = wiki_to_context_block(&w);
        assert!(s.contains("A"));
        assert!(s.contains("0.500"));
    }

    #[test]
    fn kg_to_context_block_ranks_top_nodes() {
        let mut kg = KnowledgeGraph::new();
        let a = kg.add_node(KnowledgeNode::new(
            "alpha",
            NodeKind::Extracted,
            0.9,
            "ep-1",
        ));
        let b = kg.add_node(KnowledgeNode::new("beta", NodeKind::Inferred, 0.7, "ep-1"));
        let _c = kg.add_node(KnowledgeNode::new(
            "gamma",
            NodeKind::Extracted,
            0.5,
            "ep-1",
        ));
        kg.add_edge(KnowledgeEdge::new(a, b, RelationKind::Coordination, 0.8))
            .unwrap();
        let s = kg_to_context_block(&kg, 2);
        assert!(s.contains("alpha"));
        assert!(s.contains("beta"));
        let alpha_pos = s.find("alpha").unwrap();
        let beta_pos = s.find("beta").unwrap();
        assert!(alpha_pos < beta_pos);
        assert!(s.contains("Coordination"));
    }

    #[test]
    fn kg_to_context_block_empty_graph() {
        let kg = KnowledgeGraph::new();
        let s = kg_to_context_block(&kg, 5);
        assert!(s.contains("Knowledge Graph"));
    }

    #[test]
    fn association_to_context_block_returns_neighbours() {
        let mut net = AssociationNetwork::new();
        let a = net.add_node(AssociationNode::new("alpha", 0.0));
        let b = net.add_node(AssociationNode::new("beta", 0.0));
        let c = net.add_node(AssociationNode::new("gamma", 0.0));
        net.connect(a, b, 1.0);
        net.connect(a, c, 0.5);
        let s = association_to_context_block(&mut net, a, 1);
        assert!(s.contains("alpha"));
        assert!(s.contains("beta"));
        assert!(s.contains("gamma"));
    }

    #[test]
    fn association_to_context_block_handles_island() {
        let mut net = AssociationNetwork::new();
        let a = net.add_node(AssociationNode::new("isolated", 0.0));
        let unknown = Uuid::new_v4();
        let s = association_to_context_block(&mut net, unknown, 2);
        assert!(s.contains("no neighbours"));
        // The known seed at least appears in the summary.
        let s2 = association_to_context_block(&mut net, a, 0);
        assert!(s2.contains("isolated"));
    }

    #[test]
    fn bundle_to_history_refs_combines_all() {
        let w1 = wiki("a", "x", 0.5);
        let w2 = wiki("b", "y", 0.7);
        let kg = KnowledgeGraph::new();
        let dummy_id = Uuid::new_v4();
        let refs = bundle_to_history_refs(&[&w1, &w2], &kg, dummy_id, 2);
        assert!(refs.iter().any(|r| r.starts_with("wiki:")));
        assert!(refs.iter().any(|r| r.starts_with("kg:")));
        assert!(refs.iter().any(|r| r.starts_with("association:")));
    }
}
