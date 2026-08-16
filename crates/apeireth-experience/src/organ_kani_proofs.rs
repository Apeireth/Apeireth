//! R177 experience organ Kani proofs (W6)

#![allow(missing_docs)]

use crate::wiki::WikiEntry;

#[test]
fn r177_exp_01_wiki_entry_new() {
    let e = WikiEntry::new("title", "content", 0.8);
    assert_eq!(e.title, "title");
    assert_eq!(e.confidence, 0.8);
    assert_eq!(e.promotion_count, 0);
}

#[test]
fn r177_exp_02_wiki_confidence_clamped() {
    let e1 = WikiEntry::new("t", "c", 2.0);
    assert!(e1.confidence <= 1.0);
    let e2 = WikiEntry::new("t", "c", -0.5);
    assert!(e2.confidence >= 0.0);
}

#[test]
fn r177_exp_03_wiki_with_tag() {
    let e = WikiEntry::new("t", "c", 0.5)
        .with_tag("rust")
        .with_source("ep-1");
    assert_eq!(e.tags.len(), 1);
    assert_eq!(e.tags[0], "rust");
    assert_eq!(e.source_episode_ids.len(), 1);
}

#[test]
fn r177_exp_04_wiki_promote() {
    let mut e = WikiEntry::new("t", "c", 0.5);
    e.promote();
    assert_eq!(e.promotion_count, 1);
    e.promote();
    assert_eq!(e.promotion_count, 2);
}

#[test]
fn r177_exp_05_knowledge_graph_new() {
    use crate::graph::{KnowledgeGraph, KnowledgeNode, NodeKind};
    let mut kg = KnowledgeGraph::new();
    let id = kg.add_node(KnowledgeNode::new("Rust", NodeKind::Extracted, 0.9, "ep-1"));
    assert!(!id.is_nil());
    assert_eq!(kg.node_count(), 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_exp_kani_01_confidence_clamp() {
    let e1 = WikiEntry::new("t", "c", 2.0);
    let e2 = WikiEntry::new("t", "c", -0.5);
    assert!(e1.confidence <= 1.0);
    assert!(e2.confidence >= 0.0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_exp_kani_02_promotion_increment() {
    let mut e = WikiEntry::new("t", "c", 0.5);
    e.promote();
    assert_eq!(e.promotion_count, 1);
}
