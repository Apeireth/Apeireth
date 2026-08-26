//! Example: Build a relation graph from registry, run BFS traversal, find shortest path.
//!
//! Demonstrates the R154 graph / traversal / query modules of apeireth-graph-primitive.

use apeireth_graph_primitive::{
    classify, classify_pair, query::count_by_kind, query::NodeQuery, Relation, RelationGraph,
    RelationKind, RelationRegistry,
};

fn main() {
    let a_str = "a";
    let b_str = "b";
    // Step 1: Build a relation registry (existing flat API)
    let mut registry = RelationRegistry::new();
    registry.register(Relation::new_symbiosis("perception", "cognition").unwrap());
    registry.register(Relation::new_coordination("cognition", "consciousness").unwrap());
    registry.register(Relation::new_embedding("consciousness", "reflection").unwrap());
    registry.register(Relation::new_self_relation("agent_main").unwrap());
    registry.register(
        Relation::new_symbiosis("perception", "memory")
            .unwrap()
            .with_note("shared buffer"),
    );

    println!("Registry has {} relations", registry.len());
    for k in RelationKind::ALL {
        println!("  {:?}: {}", k.semantic_name(), registry.count_by_kind(k));
    }

    // Step 2: Convert to graph (R154 new feature)
    let graph = RelationGraph::from(&registry);
    println!(
        "\nGraph has {} nodes, {} edges",
        graph.node_count(),
        graph.edge_count()
    );

    // Step 3: BFS traversal from "perception"
    println!("\nBFS from `perception` (max depth 3):");
    for (node, depth) in graph_bfs(&graph, "perception", 3) {
        println!("  depth={}: {}", depth, node);
    }

    // Step 4: Shortest path from "memory" to "reflection"
    if let Some(path) = apeireth_graph_primitive::shortest_path(&graph, "memory", "reflection") {
        println!("\nShortest path memory -> reflection:");
        println!("  nodes: {:?}", path.nodes);
        println!("  edges: {:?}", path.edges);
        println!("  hops: {}", path.hop_count());
    } else {
        println!("\nNo path from memory to reflection");
    }

    // Step 5: Predicate query (R154 query module)
    let agent_kind = NodeQuery::new().kind_eq("agent");
    let _ = agent_kind; // demonstrate the query API exists (no nodes have kind set in this example)

    // Step 6: Count edges by kind
    let (sym, coord, emb, self_rel) = count_by_kind(&graph);
    println!(
        "\nEdge counts: symbiosis={} coordination={} embedding={} self_relation={}",
        sym, coord, emb, self_rel
    );

    // Step 7: classify_pair helper
    println!(
        "\nclassify_pair(a_str, a_str) = {:?}",
        classify_pair(a_str, a_str).semantic_name()
    );
    println!(
        "classify_pair(a_str, b_str) = {:?}",
        classify_pair(a_str, b_str).semantic_name()
    );
    println!(
        "classify(decision = AEqualsB) = {:?}",
        classify(classify_pair_to_decision(a_str, a_str)).semantic_name()
    );
}

fn graph_bfs(graph: &RelationGraph, root: &str, max_depth: usize) -> Vec<(String, usize)> {
    use apeireth_graph_primitive::BfsIter;
    BfsIter::new(graph, root)
        .with_max_depth(max_depth)
        .map(|(n, d)| (n, d))
        .collect()
}

fn classify_pair_to_decision(a: &str, b: &str) -> apeireth_graph_primitive::RelationDecision {
    if a == b {
        apeireth_graph_primitive::RelationDecision::AEqualsB
    } else {
        apeireth_graph_primitive::RelationDecision::Default
    }
}
