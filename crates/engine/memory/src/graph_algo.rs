//! Graph traversal helpers over [`MemoryGraph`] (salvage of canonical
//! `apeireth-graph-primitive` traversal + pathfinding).
//!
//! Canonical [`MemoryGraph`] already owns BFS (`traverse`) and unweighted
//! shortest path. This module recovers the missing directed algorithms:
//!
//! - direction-aware BFS/DFS (`walk`)
//! - weighted Dijkstra (uses [`Edge::weight`], skips negative / non-finite)
//! - bounded `all_paths`
//! - directed cycle detection (3-color DFS)
//! - Kahn topological sort
//! - undirected connected components
//! - relation / endpoint edge filters
//!
//! This is **not** a second graph store and **not** a LangGraph runtime.
//! Callers pass the existing [`MemoryGraph`]. Neighbor order is always
//! destination-id ascending so replay does not depend on HashMap layout.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};

use crate::canonical::domain::MemoryId;
use crate::canonical::graph::{Edge, MemoryGraph};

/// Direction of a walk over a directed [`MemoryGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraversalDirection {
    /// Follow outgoing edges only.
    Outgoing,
    /// Follow incoming edges only.
    Incoming,
    /// Treat the graph as undirected for this walk.
    Both,
}

/// Breadth-first or depth-first walk order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WalkOrder {
    Bfs,
    Dfs,
}

/// One step of a walk: node plus depth from the start (start is depth 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkStep {
    pub id: MemoryId,
    pub depth: usize,
}

/// Neighbor ids of `id` in `direction`, sorted and de-duplicated.
pub fn neighbors_directed(
    graph: &MemoryGraph,
    id: &MemoryId,
    direction: TraversalDirection,
) -> Vec<MemoryId> {
    let mut ids: Vec<MemoryId> = match direction {
        TraversalDirection::Outgoing => graph
            .edges_from(id)
            .into_iter()
            .map(|edge| edge.to.clone())
            .collect(),
        TraversalDirection::Incoming => graph
            .edges_into(id)
            .into_iter()
            .map(|edge| edge.from.clone())
            .collect(),
        TraversalDirection::Both => {
            let mut both: Vec<MemoryId> = graph
                .edges_from(id)
                .into_iter()
                .map(|edge| edge.to.clone())
                .collect();
            both.extend(
                graph
                    .edges_into(id)
                    .into_iter()
                    .map(|edge| edge.from.clone()),
            );
            both
        }
    };
    ids.sort();
    ids.dedup();
    ids
}

/// Bounded, cycle-safe walk from `start`.
///
/// Missing `start` returns an empty vector. `max_depth = 0` returns only
/// `start` when it exists. Duplicate nodes are never emitted.
pub fn walk(
    graph: &MemoryGraph,
    start: &MemoryId,
    max_depth: usize,
    direction: TraversalDirection,
    order: WalkOrder,
) -> Vec<WalkStep> {
    if graph.node(start).is_none() {
        return Vec::new();
    }

    let mut visited = HashSet::new();
    visited.insert(start.clone());
    let mut out = Vec::new();

    match order {
        WalkOrder::Bfs => {
            let mut queue = VecDeque::new();
            queue.push_back((start.clone(), 0_usize));
            while let Some((node, depth)) = queue.pop_front() {
                out.push(WalkStep {
                    id: node.clone(),
                    depth,
                });
                if depth >= max_depth {
                    continue;
                }
                for next in neighbors_directed(graph, &node, direction) {
                    if visited.insert(next.clone()) {
                        queue.push_back((next, depth + 1));
                    }
                }
            }
        }
        WalkOrder::Dfs => {
            let mut stack = vec![(start.clone(), 0_usize)];
            while let Some((node, depth)) = stack.pop() {
                out.push(WalkStep {
                    id: node.clone(),
                    depth,
                });
                if depth >= max_depth {
                    continue;
                }
                let mut nxt = neighbors_directed(graph, &node, direction);
                // Push in reverse so the lexicographically smallest neighbor is
                // visited first (pre-order DFS with sorted adjacency).
                nxt.reverse();
                for next in nxt {
                    if visited.insert(next.clone()) {
                        stack.push((next, depth + 1));
                    }
                }
            }
        }
    }

    out
}

/// Weighted shortest path using Dijkstra on outgoing edges.
///
/// Returns `(path, total_weight)`. Negative or non-finite weights are skipped
/// (canonical Dijkstra does not support them). A self-path is `([from], 0.0)` when
/// the node exists. Missing endpoints or an unreachable target return `None`.
pub fn dijkstra_shortest_path(
    graph: &MemoryGraph,
    from: &MemoryId,
    to: &MemoryId,
) -> Option<(Vec<MemoryId>, f64)> {
    if graph.node(from).is_none() || graph.node(to).is_none() {
        return None;
    }
    if from == to {
        return Some((vec![from.clone()], 0.0));
    }

    let mut dist: HashMap<MemoryId, f64> = HashMap::new();
    let mut parent: HashMap<MemoryId, MemoryId> = HashMap::new();
    let mut heap: BinaryHeap<Reverse<(u64, MemoryId)>> = BinaryHeap::new();

    dist.insert(from.clone(), 0.0);
    heap.push(Reverse((finite_nonneg_bits(0.0)?, from.clone())));

    while let Some(Reverse((_, cur))) = heap.pop() {
        let d = *dist.get(&cur).unwrap_or(&f64::INFINITY);
        if cur == *to {
            let mut path = vec![to.clone()];
            let mut cursor = to.clone();
            while let Some(p) = parent.get(&cursor) {
                path.push(p.clone());
                cursor = p.clone();
            }
            path.reverse();
            return Some((path, d));
        }
        if d.is_infinite() {
            continue;
        }
        for edge in graph.edges_from(&cur) {
            if !edge.weight.is_finite() || edge.weight < 0.0 {
                continue;
            }
            let new_d = d + edge.weight;
            if !new_d.is_finite() {
                continue;
            }
            if new_d < *dist.get(&edge.to).unwrap_or(&f64::INFINITY) {
                dist.insert(edge.to.clone(), new_d);
                parent.insert(edge.to.clone(), cur.clone());
                if let Some(bits) = finite_nonneg_bits(new_d) {
                    heap.push(Reverse((bits, edge.to.clone())));
                }
            }
        }
    }
    None
}

fn finite_nonneg_bits(x: f64) -> Option<u64> {
    if x.is_finite() && x >= 0.0 {
        Some(x.to_bits())
    } else {
        None
    }
}

/// Enumerate simple directed paths from `from` to `to`, capped by `max_paths`
/// and `max_depth` (path length in nodes). The trivial self-path is omitted
/// (canonical behaviour). Neighbors are visited in id-ascending order.
pub fn all_paths(
    graph: &MemoryGraph,
    from: &MemoryId,
    to: &MemoryId,
    max_paths: usize,
    max_depth: usize,
) -> Vec<Vec<MemoryId>> {
    if graph.node(from).is_none() || graph.node(to).is_none() || max_paths == 0 || max_depth == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut path = vec![from.clone()];
    let mut visited = HashSet::new();
    visited.insert(from.clone());
    all_paths_dfs(
        graph,
        from,
        to,
        &mut path,
        &mut visited,
        &mut result,
        max_paths,
        max_depth,
    );
    result
}

fn all_paths_dfs(
    graph: &MemoryGraph,
    cur: &MemoryId,
    target: &MemoryId,
    path: &mut Vec<MemoryId>,
    visited: &mut HashSet<MemoryId>,
    result: &mut Vec<Vec<MemoryId>>,
    max_paths: usize,
    max_depth: usize,
) {
    if result.len() >= max_paths || path.len() > max_depth {
        return;
    }
    if cur == target && path.len() > 1 {
        result.push(path.clone());
        return;
    }
    for neighbor in neighbors_directed(graph, cur, TraversalDirection::Outgoing) {
        if visited.insert(neighbor.clone()) {
            path.push(neighbor.clone());
            all_paths_dfs(
                graph, &neighbor, target, path, visited, result, max_paths, max_depth,
            );
            path.pop();
            visited.remove(&neighbor);
        }
    }
}

/// Directed cycle detection via 3-color DFS. Isolated nodes are not cycles.
pub fn has_cycle(graph: &MemoryGraph) -> bool {
    let mut color: HashMap<MemoryId, u8> = HashMap::new();
    for node in graph.node_ids() {
        if color.get(&node).copied().unwrap_or(0) == 0 && dfs_cycle(graph, &node, &mut color) {
            return true;
        }
    }
    false
}

fn dfs_cycle(graph: &MemoryGraph, node: &MemoryId, color: &mut HashMap<MemoryId, u8>) -> bool {
    color.insert(node.clone(), 1);
    for neighbor in neighbors_directed(graph, node, TraversalDirection::Outgoing) {
        match color.get(&neighbor).copied().unwrap_or(0) {
            1 => return true,
            0 => {
                if dfs_cycle(graph, &neighbor, color) {
                    return true;
                }
            }
            _ => {}
        }
    }
    color.insert(node.clone(), 2);
    false
}

/// Kahn topological sort. Zero in-degree nodes are dequeued in id-ascending
/// order so the result is replay-stable. Returns `None` when a cycle exists.
pub fn topological_sort(graph: &MemoryGraph) -> Option<Vec<MemoryId>> {
    let nodes = graph.node_ids();
    let mut in_degree: HashMap<MemoryId, usize> = nodes.iter().cloned().map(|n| (n, 0)).collect();
    for node in &nodes {
        for neighbor in neighbors_directed(graph, node, TraversalDirection::Outgoing) {
            *in_degree.entry(neighbor).or_insert(0) += 1;
        }
    }

    let mut ready: BTreeSet<MemoryId> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    let mut order = Vec::new();
    while let Some(n) = ready.pop_first() {
        order.push(n.clone());
        for neighbor in neighbors_directed(graph, &n, TraversalDirection::Outgoing) {
            if let Some(d) = in_degree.get_mut(&neighbor) {
                *d = d.saturating_sub(1);
                if *d == 0 {
                    ready.insert(neighbor);
                }
            }
        }
    }

    if order.len() == graph.len() {
        Some(order)
    } else {
        None
    }
}

/// Undirected connected components (incoming + outgoing). Members of each
/// component are sorted; components are sorted by their first id.
pub fn connected_components(graph: &MemoryGraph) -> Vec<Vec<MemoryId>> {
    let mut visited: HashSet<MemoryId> = HashSet::new();
    let mut components: Vec<Vec<MemoryId>> = Vec::new();
    for node in graph.node_ids() {
        if !visited.contains(&node) {
            let mut comp = Vec::new();
            let mut stack = vec![node.clone()];
            while let Some(n) = stack.pop() {
                if visited.insert(n.clone()) {
                    comp.push(n.clone());
                    for nb in neighbors_directed(graph, &n, TraversalDirection::Both) {
                        if !visited.contains(&nb) {
                            stack.push(nb);
                        }
                    }
                }
            }
            comp.sort();
            components.push(comp);
        }
    }
    components.sort_by(|a, b| a.first().cmp(&b.first()));
    components
}

/// Filter edges by optional relation and endpoints. All provided predicates
/// are AND-ed. Results are ordered by `(from, relation, to)`.
pub fn edges_matching<'a>(
    graph: &'a MemoryGraph,
    relation: Option<&str>,
    from: Option<&MemoryId>,
    to: Option<&MemoryId>,
) -> Vec<&'a Edge> {
    let mut edges: Vec<&Edge> = graph
        .all_edges()
        .iter()
        .filter(|edge| relation.is_none_or(|r| edge.relation == r))
        .filter(|edge| from.is_none_or(|id| &edge.from == id))
        .filter(|edge| to.is_none_or(|id| &edge.to == id))
        .collect();
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.relation.cmp(&b.relation))
            .then_with(|| a.to.cmp(&b.to))
    });
    edges
}

/// Nodes whose `label` equals `label`, ordered by id.
pub fn nodes_with_label(graph: &MemoryGraph, label: &str) -> Vec<MemoryId> {
    let mut ids: Vec<MemoryId> = graph
        .node_ids()
        .into_iter()
        .filter(|id| graph.node(id).is_some_and(|n| n.label == label))
        .collect();
    ids.sort();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::graph::{Edge, Node};

    fn id(s: &str) -> MemoryId {
        MemoryId::new(s).unwrap()
    }

    fn node(name: &str) -> Node {
        Node::new(id(name), name)
    }

    fn chain() -> MemoryGraph {
        let mut g = MemoryGraph::new();
        for n in ["a", "b", "c", "d", "e"] {
            g.add_node(node(n)).unwrap();
        }
        for (f, t) in [("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")] {
            g.add_edge(Edge::new(id(f), id(t), "next", 1.0)).unwrap();
        }
        g
    }

    fn dag() -> MemoryGraph {
        let mut g = MemoryGraph::new();
        for n in ["A", "B", "C", "D"] {
            g.add_node(node(n)).unwrap();
        }
        g.add_edge(Edge::new(id("A"), id("B"), "next", 1.0))
            .unwrap();
        g.add_edge(Edge::new(id("B"), id("C"), "next", 1.0))
            .unwrap();
        g.add_edge(Edge::new(id("A"), id("D"), "next", 1.0))
            .unwrap();
        g
    }

    fn cyclic() -> MemoryGraph {
        let mut g = MemoryGraph::new();
        for n in ["A", "B", "C"] {
            g.add_node(node(n)).unwrap();
        }
        g.add_edge(Edge::new(id("A"), id("B"), "next", 1.0))
            .unwrap();
        g.add_edge(Edge::new(id("B"), id("C"), "next", 1.0))
            .unwrap();
        g.add_edge(Edge::new(id("A"), id("C"), "next", 1.0))
            .unwrap();
        g.add_edge(Edge::new(id("C"), id("A"), "next", 1.0))
            .unwrap();
        g
    }

    #[test]
    fn bfs_outgoing_matches_canonical_traverse() {
        let g = chain();
        let steps = walk(
            &g,
            &id("a"),
            10,
            TraversalDirection::Outgoing,
            WalkOrder::Bfs,
        );
        let ids: Vec<_> = steps.into_iter().map(|s| s.id).collect();
        assert_eq!(ids, g.traverse(&id("a"), 10));
    }

    #[test]
    fn bfs_max_depth_and_incoming_direction() {
        let g = chain();
        let steps = walk(
            &g,
            &id("a"),
            2,
            TraversalDirection::Outgoing,
            WalkOrder::Bfs,
        );
        let ids: Vec<_> = steps.iter().map(|s| s.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);

        let incoming = walk(
            &g,
            &id("c"),
            1,
            TraversalDirection::Incoming,
            WalkOrder::Bfs,
        );
        let ids: Vec<_> = incoming.iter().map(|s| s.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["c", "b"]);
    }

    #[test]
    fn dfs_visits_chain_in_preorder() {
        let g = chain();
        let steps = walk(
            &g,
            &id("a"),
            10,
            TraversalDirection::Outgoing,
            WalkOrder::Dfs,
        );
        let ids: Vec<_> = steps
            .into_iter()
            .map(|s| s.id.as_str().to_string())
            .collect();
        assert_eq!(ids, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn dijkstra_prefers_cheaper_direct_edge() {
        let mut g = MemoryGraph::new();
        for n in ["A", "B", "C"] {
            g.add_node(node(n)).unwrap();
        }
        g.add_edge(Edge::new(id("A"), id("B"), "via", 1.0)).unwrap();
        g.add_edge(Edge::new(id("B"), id("C"), "via", 1.0)).unwrap();
        g.add_edge(Edge::new(id("A"), id("C"), "direct", 10.0))
            .unwrap();

        let (path, total) = dijkstra_shortest_path(&g, &id("A"), &id("C")).unwrap();
        assert_eq!(
            path.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );
        assert!((total - 2.0).abs() < 1e-12);

        // Negative weights are skipped, leaving only the expensive direct edge.
        let mut g2 = MemoryGraph::new();
        for n in ["A", "B", "C"] {
            g2.add_node(node(n)).unwrap();
        }
        g2.add_edge(Edge::new(id("A"), id("B"), "neg", -1.0))
            .unwrap();
        g2.add_edge(Edge::new(id("B"), id("C"), "neg", -1.0))
            .unwrap();
        g2.add_edge(Edge::new(id("A"), id("C"), "ok", 5.0)).unwrap();
        let (path, total) = dijkstra_shortest_path(&g2, &id("A"), &id("C")).unwrap();
        assert_eq!(path.last().unwrap(), &id("C"));
        assert!((total - 5.0).abs() < 1e-12);
    }

    #[test]
    fn dijkstra_unreachable_and_self() {
        let g = dag();
        assert!(dijkstra_shortest_path(&g, &id("C"), &id("A")).is_none());
        let (path, total) = dijkstra_shortest_path(&g, &id("A"), &id("A")).unwrap();
        assert_eq!(path, vec![id("A")]);
        assert_eq!(total, 0.0);
    }

    #[test]
    fn all_paths_enumerates_and_respects_cap() {
        let g = cyclic();
        let paths = all_paths(&g, &id("A"), &id("C"), 10, 5);
        assert!(paths.len() >= 2);
        let capped = all_paths(&g, &id("A"), &id("C"), 1, 5);
        assert_eq!(capped.len(), 1);
        assert!(all_paths(&g, &id("A"), &id("A"), 10, 5).is_empty());
    }

    #[test]
    fn cycle_and_topo_sort() {
        assert!(has_cycle(&cyclic()));
        assert!(!has_cycle(&dag()));
        let order = topological_sort(&dag()).unwrap();
        let pos = |n: &str| order.iter().position(|id| id.as_str() == n).unwrap();
        assert!(pos("A") < pos("B"));
        assert!(pos("B") < pos("C"));
        assert!(pos("A") < pos("D"));
        assert!(topological_sort(&cyclic()).is_none());
    }

    #[test]
    fn connected_components_are_sorted() {
        let mut g = MemoryGraph::new();
        for n in ["A", "B", "C", "D", "E", "F"] {
            g.add_node(node(n)).unwrap();
        }
        g.add_edge(Edge::new(id("A"), id("B"), "x", 1.0)).unwrap();
        g.add_edge(Edge::new(id("B"), id("C"), "x", 1.0)).unwrap();
        g.add_edge(Edge::new(id("D"), id("E"), "x", 1.0)).unwrap();
        let comps = connected_components(&g);
        assert_eq!(comps.len(), 3);
        assert_eq!(
            comps[0].iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );
        assert_eq!(
            comps[1].iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            vec!["D", "E"]
        );
        assert_eq!(
            comps[2].iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            vec!["F"]
        );
    }

    #[test]
    fn edge_and_label_filters() {
        let mut g = MemoryGraph::new();
        g.add_node(Node::new(id("alice"), "agent")).unwrap();
        g.add_node(Node::new(id("bob"), "agent")).unwrap();
        g.add_node(Node::new(id("carol"), "tool")).unwrap();
        g.add_edge(Edge::new(id("alice"), id("bob"), "symbiosis", 1.0))
            .unwrap();
        g.add_edge(Edge::new(id("alice"), id("carol"), "coordination", 1.0))
            .unwrap();

        let agents = nodes_with_label(&g, "agent");
        assert_eq!(agents, vec![id("alice"), id("bob")]);
        let coord = edges_matching(&g, Some("coordination"), Some(&id("alice")), None);
        assert_eq!(coord.len(), 1);
        assert_eq!(coord[0].to, id("carol"));
    }

    #[test]
    fn missing_start_is_empty_and_walk_is_cycle_safe() {
        let g = cyclic();
        assert!(walk(
            &g,
            &id("missing"),
            10,
            TraversalDirection::Outgoing,
            WalkOrder::Bfs
        )
        .is_empty());
        let steps = walk(
            &g,
            &id("A"),
            20,
            TraversalDirection::Outgoing,
            WalkOrder::Bfs,
        );
        assert_eq!(steps.len(), 3);
    }
}
