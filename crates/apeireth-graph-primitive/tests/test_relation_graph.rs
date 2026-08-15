//! Integration test for apeireth_relation graph / traversal / query modules.
//!
//! Covers:
//! - Graph construction (nodes + edges + indexes)
//! - BFS / DFS traversal (max depth, direction)
//! - Shortest path (BFS-based, with edge IDs)
//! - Predicate-based query (NodeQuery / EdgeQuery / CombinedQuery)
//! - Conversion from RelationRegistry

use apeireth_graph_primitive::{
    query::{count_by_kind, CombinedQuery, EdgeQuery, NodeQuery},
    BfsIter, DfsIter, GraphEdge, GraphNode, Relation, RelationGraph, RelationKind,
    RelationRegistry, TraversalDirection, shortest_path,
};

fn build_complex_graph() -> RelationGraph {
    let mut g = RelationGraph::new();
    // 6 nodes: 3 agents + 2 tools + 1 concept
    g.insert_node(GraphNode::with_kind("alice", "agent")
        .with_properties(serde_json::json!({"role": "assistant"}))).unwrap();
    g.insert_node(GraphNode::with_kind("bob", "agent")
        .with_properties(serde_json::json!({"role": "user"}))).unwrap();
    g.insert_node(GraphNode::with_kind("carol", "agent")
        .with_properties(serde_json::json!({"role": "supervisor"}))).unwrap();
    g.insert_node(GraphNode::with_kind("fs", "tool")).unwrap();
    g.insert_node(GraphNode::with_kind("web", "tool")).unwrap();
    g.insert_node(GraphNode::with_kind("memory", "concept")).unwrap();
    // 7 edges
    g.insert_edge(GraphEdge::new("alice", RelationKind::Symbiosis, "bob")).unwrap();
    g.insert_edge(GraphEdge::new("bob", RelationKind::Coordination, "carol")).unwrap();
    g.insert_edge(GraphEdge::new("alice", RelationKind::Coordination, "carol")).unwrap();
    g.insert_edge(GraphEdge::new("alice", RelationKind::Embedding, "fs")).unwrap();
    g.insert_edge(GraphEdge::new("alice", RelationKind::Embedding, "web")).unwrap();
    g.insert_edge(GraphEdge::new("carol", RelationKind::Embedding, "memory")).unwrap();
    g.insert_edge(GraphEdge::new("alice", RelationKind::SelfRelation, "alice")).unwrap();
    g
}

#[test]
fn integration_graph_node_edge_counts() {
    let g = build_complex_graph();
    assert_eq!(g.node_count(), 6);
    assert_eq!(g.edge_count(), 7);
}

#[test]
fn integration_bfs_full_traversal() {
    let g = build_complex_graph();
    let nodes: Vec<String> = BfsIter::new(&g, "alice").map(|(n, _)| n).collect();
    assert_eq!(nodes.len(), 6, "all nodes reachable from alice");
}

#[test]
fn integration_bfs_outgoing_only() {
    let g = build_complex_graph();
    // Use direction filter
    let nodes: Vec<String> = BfsIter::new(&g, "alice")
        .with_direction(TraversalDirection::Outgoing)
        .map(|(n, _)| n)
        .collect();
    // alice -> bob (symbiosis), carol (coord), fs (embed), web (embed), alice (self)
    // bob and carol have no outgoing edges -> DFS terminates quickly
    assert!(nodes.contains(&"bob".to_string()));
    assert!(nodes.contains(&"fs".to_string()));
}

#[test]
fn integration_dfs_max_depth_2() {
    let g = build_complex_graph();
    let nodes: Vec<String> = DfsIter::new(&g, "alice")
        .with_max_depth(2)
        .map(|(n, _)| n)
        .collect();
    // depth 0: alice; depth 1: direct neighbors; depth 2: their neighbors
    assert!(nodes.len() >= 3);
}

#[test]
fn integration_shortest_path_with_edges() {
    let g = build_complex_graph();
    let path = shortest_path(&g, "bob", "memory").expect("path must exist");
    assert_eq!(path.nodes[0], "bob");
    assert_eq!(path.nodes[path.nodes.len() - 1], "memory");
    assert_eq!(path.hop_count(), path.nodes.len() - 1);
    assert_eq!(path.edges.len(), path.hop_count());
}

#[test]
fn integration_shortest_path_no_path() {
    let g = build_complex_graph();
    // Make a disconnected island
    let mut g2 = g.clone();
    g2.insert_node(GraphNode::new("isolated")).unwrap();
    assert!(shortest_path(&g2, "alice", "isolated").is_none());
}

#[test]
fn integration_node_query_by_kind() {
    let g = build_complex_graph();
    let agents: Vec<&GraphNode> = NodeQuery::new().kind_eq("agent").execute(&g);
    assert_eq!(agents.len(), 3);

    let tools: Vec<&GraphNode> = NodeQuery::new().kind_eq("tool").execute(&g);
    assert_eq!(tools.len(), 2);
}

#[test]
fn integration_node_query_by_property() {
    let g = build_complex_graph();
    let supervisors: Vec<&GraphNode> = NodeQuery::new()
        .kind_eq("agent")
        .property_string("role", "supervisor")
        .execute(&g);
    assert_eq!(supervisors.len(), 1);
    assert_eq!(supervisors[0].id, "carol");
}

#[test]
fn integration_edge_query_by_kind() {
    let g = build_complex_graph();
    let embeddings: Vec<&GraphEdge> = EdgeQuery::new().kind(RelationKind::Embedding).execute(&g);
    assert_eq!(embeddings.len(), 3);
}

#[test]
fn integration_edge_query_by_endpoint() {
    let g = build_complex_graph();
    let edges_from_alice: Vec<&GraphEdge> = EdgeQuery::new().from("alice").execute(&g);
    // alice has: bob (symbiosis), carol (coord), fs (embed), web (embed), alice (self)
    assert_eq!(edges_from_alice.len(), 5);
}

#[test]
fn integration_combined_query_restrict() {
    let g = build_complex_graph();
    let agent_nodes = NodeQuery::new().kind_eq("agent");
    let agent_edges = EdgeQuery::new();
    let (nodes, edges) = CombinedQuery::new(agent_nodes, agent_edges)
        .restrict_to_nodes(true)
        .execute(&g);
    assert_eq!(nodes.len(), 3);
    // Edges incident on agent nodes: alice-bob (sym), bob-carol (coord), alice-carol (coord), alice-alice (self)
    // = 4 edges
    assert!(edges.len() >= 4);
}

#[test]
fn integration_count_by_kind_4_tuple() {
    let g = build_complex_graph();
    let (sym, coord, emb, self_rel) = count_by_kind(&g);
    assert_eq!(sym, 1); // alice-bob
    assert_eq!(coord, 2); // bob-carol, alice-carol
    assert_eq!(emb, 3); // alice-fs, alice-web, carol-memory
    assert_eq!(self_rel, 1); // alice-alice
}

#[test]
fn integration_from_registry_preserves_data() {
    let mut reg = RelationRegistry::new();
    reg.register(Relation::new_symbiosis("a", "b").unwrap());
    reg.register(Relation::new_coordination("b", "c").unwrap());
    reg.register(Relation::new_embedding("c", "d").unwrap());
    reg.register(Relation::new_self_relation("a").unwrap());

    let g = RelationGraph::from(&reg);
    assert_eq!(g.node_count(), 4); // a, b, c, d
    assert_eq!(g.edge_count(), 4);
}

#[test]
fn integration_from_registry_self_relation_dedups_node() {
    let mut reg = RelationRegistry::new();
    reg.register(Relation::new_self_relation("solo").unwrap());
    let g = RelationGraph::from(&reg);
    assert_eq!(g.node_count(), 1);
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn integration_remove_node_cascade_preserves_invariants() {
    let mut g = build_complex_graph();
    g.remove_node("alice").unwrap();
    assert_eq!(g.node_count(), 5);
    // 5 alice-edges should be gone (bob, carol, fs, web, alice-self)
    assert_eq!(g.edge_count(), 7 - 5); // = 2
}

#[test]
fn integration_full_lifecycle_query() {
    let g = build_complex_graph();
    // 1. total embedding count
    let counts = count_by_kind(&g);
    assert_eq!(counts.2, 3); // alice-fs, alice-web, carol-memory
    // 2. embedding edges FROM alice only
    let emb_from_alice: Vec<&GraphEdge> = EdgeQuery::new()
        .from("alice")
        .kind(RelationKind::Embedding)
        .execute(&g);
    assert_eq!(emb_from_alice.len(), 2); // alice-fs, alice-web
}
