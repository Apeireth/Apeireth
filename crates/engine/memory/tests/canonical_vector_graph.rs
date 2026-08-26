//! M1B3 public vector/graph infrastructure smoke tests.
//!
//! The modules have exhaustive unit tests; these tests verify that the public
//! canonical re-exports are usable by future consumers without touching the
//! legacy crate roots.

use apeireth_memory::canonical::{
    cosine_similarity, Edge, MemoryGraph, MemoryId, Node, VectorIndex,
};

fn id(s: &str) -> MemoryId {
    MemoryId::new(s).unwrap()
}

#[test]
fn public_vector_index_supports_insert_update_remove_and_query() {
    let mut index = VectorIndex::new(3).unwrap();
    index.insert(id("a"), vec![1.0, 0.0, 0.0]).unwrap();
    index.insert(id("b"), vec![0.0, 1.0, 0.0]).unwrap();

    let hits = index.query(&[0.9, 0.1, 0.0], 2).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id.as_str(), "a");
    assert!(hits[0].score > 0.9);

    index.update(&id("a"), vec![0.0, 1.0, 0.0]).unwrap();
    let hits = index.query(&[0.0, 0.9, 0.0], 1).unwrap();
    assert_eq!(hits.len(), 1);

    index.remove(&id("a")).unwrap();
    assert_eq!(index.len(), 1);

    assert!((cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]) - 0.0).abs() < 1e-6);
}

#[test]
fn public_graph_supports_edges_neighbors_and_cycle_safe_traversal() {
    let mut graph = MemoryGraph::new();
    graph.add_node(Node::new(id("a"), "A")).unwrap();
    graph.add_node(Node::new(id("b"), "B")).unwrap();
    graph.add_node(Node::new(id("c"), "C")).unwrap();

    graph
        .add_edge(Edge::new(id("a"), id("b"), "next", 1.0))
        .unwrap();
    graph
        .add_edge(Edge::new(id("b"), id("c"), "next", 1.0))
        .unwrap();
    graph
        .add_edge(Edge::new(id("c"), id("a"), "next", 1.0))
        .unwrap();

    let neighbors = graph.neighbors(&id("a"));
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].id.as_str(), "b");

    let order = graph.traverse(&id("a"), 10);
    assert_eq!(order.len(), 3);

    assert_eq!(
        graph.shortest_path(&id("a"), &id("c")),
        Some(vec![id("a"), id("b"), id("c")])
    );
}

#[test]
fn public_vector_and_graph_are_in_memory_only() {
    let index = VectorIndex::new(1).unwrap();
    assert_eq!(index.len(), 0);

    let graph = MemoryGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.edge_count(), 0);
}
