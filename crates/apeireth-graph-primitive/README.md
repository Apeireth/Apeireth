# apeireth-graph-primitive

> Apeireth graph primitive subsystem (A12) — 4 relation kinds + property graph + traversal + predicate query.

## Two-layer architecture (R154 unified)

| Layer | Module | Purpose |
|-------|--------|---------|
| Relation modeling (A12) | `src/lib.rs` | `RelationKind` (4 kinds: Symbiosis / Coordination / Embedding / SelfRelation), `Relation` struct, `RelationRegistry` (flat list), relation decision tree |
| Property graph (R154) | `src/graph.rs` | `RelationGraph` with nodes + edges + adjacency-list indexes (out/in/kind), `GraphNode` / `GraphEdge` with `serde_json::Value` properties, shortest-path BFS |
| Traversal (R154) | `src/traversal.rs` | `BfsIter` / `DfsIter` (level-order / pre-order), depth-limited, direction filter (Outgoing/Incoming/Both), `shortest_path` |
| Query (R154) | `src/query.rs` | `NodeQuery` (kind + property filter, builder), `EdgeQuery` (kind + endpoint + property filter), `CombinedQuery` (nodes + edges with restrict), `count_by_kind` |

## Borrowed upstream references (per O-5)

- **SurrealDB** — RELATE statement, `->` arrow traversal, graph storage
- **Neo4j / Memgraph** — BFS/DFS semantics, depth-limited traversal
- **Cypher** — MATCH pattern inspiration for predicate filters

## Quick start

### Relation modeling (existing flat API)

```rust
use apeireth_graph_primitive::{Relation, RelationKind, RelationRegistry};

let mut registry = RelationRegistry::new();
registry.register(Relation::new_symbiosis("perception", "cognition")?);
registry.register(Relation::new_embedding("consciousness", "reflection")?);
registry.register(Relation::new_self_relation("agent_main")?);
println!("{} relations", registry.len());
```

### Property graph (R154 new)

```rust
use apeireth_graph_primitive::{GraphEdge, GraphNode, RelationGraph, RelationKind};

let mut graph = RelationGraph::new();
graph.insert_node(GraphNode::with_kind("alice", "agent"))?;
graph.insert_node(GraphNode::with_kind("bob", "agent"))?;
graph.insert_edge(GraphEdge::new("alice", RelationKind::Symbiosis, "bob"))?;

// Shortest path (BFS)
let path = graph.shortest_path("alice", "bob");
assert_eq!(path, Some(vec!["alice".to_string(), "bob".to_string()]));
```

### Traversal (R154 new)

```rust
use apeireth_graph_primitive::{BfsIter, DfsIter, TraversalDirection, shortest_path};

let nodes: Vec<(String, usize)> = BfsIter::new(&graph, "alice")
    .with_max_depth(3)
    .with_direction(TraversalDirection::Outgoing)
    .collect();

let path = shortest_path(&graph, "alice", "bob").unwrap();
println!("hops: {}, nodes: {:?}", path.hop_count(), path.nodes);
```

### Predicate query (R154 new)

```rust
use apeireth_graph_primitive::query::{NodeQuery, EdgeQuery, count_by_kind};

let agents = NodeQuery::new()
    .kind_eq("agent")
    .property_string("role", "assistant")
    .execute(&graph);

let embeddings = EdgeQuery::new()
    .kind(RelationKind::Embedding)
    .execute(&graph);

let (sym, coord, emb, self_rel) = count_by_kind(&graph);
```

### Convert from registry

```rust
let graph = RelationGraph::from(&registry);
// All parties become nodes; all relations become edges (with UUIDs)
```

## Tests (R154 cumulative)

| Type | Count |
|------|-------|
| Lib unit (lib.rs + graph + traversal + query) | 48 |
| Integration (existing relation_demo + new test_relation_graph) | 16 |
| Examples (existing + new graph_demo) | 2 |
| **Total** | **64 + 0 failed** |

Run with `cargo test -p apeireth-graph-primitive`.

## Where to start

- Cargo.toml: see [dependencies](Cargo.toml) for upstream crates.
- src/lib.rs: existing flat relation API + relation decision tree.
- src/graph.rs: property graph with adjacency indexes (R154).
- src/traversal.rs: BFS / DFS iterators + shortest path (R154).
- src/query.rs: predicate-based query API (R154).

## Examples

- `cargo run -p apeireth-graph-primitive --example relation_demo` — existing flat demo
- `cargo run -p apeireth-graph-primitive --example graph_demo` — R154 graph + traversal + query

## See also

- [Apeireth conventions](../../docs/conventions/README.md)
- [R154 report](../../docs/r154/r154-relation-graph-query.md)
- [Apeireth roadmap](../../docs/pages-source/roadmap.md)

---

_Last-modified: 2026-08-13 (R154). Tracked in git log._
