# M1B3 — Canonical Vector / Graph Infrastructure

## Status

Implemented on `reconstruct_v2`. No schema change in this phase; both
structures are in-memory only, matching donor semantics.

## Ownership decision

- Vector owner: `apeireth-memory::canonical::vector`
- Graph owner: `apeireth-memory::canonical::graph`
- Reason: both are memory/query infrastructure. The frozen architecture
  expects memory-owned indexing primitives; no new crate is justified because
  both have only one current consumer and no independent dependency boundary.

## Donor sources

- `origin/master:reconstruction_v2/crates/apeireth-storage/src/vector.rs`
- `origin/master:reconstruction_v2/crates/apeireth-storage/src/graph_primitive.rs`
- `origin/master:reconstruction_v2/crates/apeireth-storage/src/graph_ops.rs`
- `origin/master:reconstruction_v2/crates/apeireth-storage/src/graph.rs`

## Reused

- Cosine similarity metric with explicit zero-vector handling (`0.0`).
- Deterministic top-k query shape.
- Simple directed graph primitives (node, labelled edge, neighbours).
- BFS, DFS-style deterministic traversal (ported as BFS), and unweighted
  shortest path from `graph_ops.rs`.
- Cycle-safe bounded BFS traversal from the donor graph crawl.

## Adapted

- `VectorIndex` now has an explicit fixed dimension and rejects dimension
  mismatch and non-finite values instead of panicking or poisoning ordering.
- Duplicate vector ids are a `Conflict`; update/remove are explicit.
- Graph edges carry both `relation` and finite `weight` (merging donor
  `graph_primitive` label semantics with donor `graph.rs` weight semantics).
- Duplicate edges are rejected instead of silently duplicating.
- `remove_node` removes incident edges; `remove_edge` reports missing edges.
- All iteration order is deterministic (ordered by ids/relations), not
  `HashMap` order.

## Rejected / deferred

- Donor `CausalGraph` MCTS causal simulator: out of scope for M1B3.
- Donor hybrid cosine+BM25 search: deferred. The BM25 half is text retrieval
  rather than vector infrastructure; it is better ported together with
  retrieval semantics after the memory/query contracts are final.
- No persistence: donor vector and graph are in-memory, so no reopen test is
  promised.

## Tests

Vector: insert/update/remove, nearest query, stable tie ordering, dimension
mismatch, non-finite values, duplicate id, top-k bounds, zero-dimension, zero
vector handling. Graph: node/edge insert, neighbour order, remove edge/node,
missing node, duplicate edge, cycle-safe bounded traversal, shortest path.
