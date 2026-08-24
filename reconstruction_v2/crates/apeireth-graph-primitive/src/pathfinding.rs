//! Advanced graph algorithms: shortest path, cycle detection, topological sort, connected components.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::NodeId;
use crate::traversal::{shortest_path, PathResult};

/// Find shortest path between two nodes.
pub fn find_path(g: &crate::graph::RelationGraph, from: &str, to: &str) -> Option<Vec<NodeId>> {
    match shortest_path(g, from, to) {
        PathResult::Found(p) => Some(p),
        PathResult::NotFound => None,
    }
}

/// Detect if the graph contains a cycle (using DFS).
pub fn cycle_detect(g: &crate::graph::RelationGraph) -> bool {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack: HashSet<NodeId> = HashSet::new();

    fn dfs(
        g: &crate::graph::RelationGraph,
        node: &str,
        visited: &mut HashSet<NodeId>,
        stack: &mut HashSet<NodeId>,
    ) -> bool {
        if stack.contains(node) { return true; }
        if visited.contains(node) { return false; }
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        for n in g.out_neighbors(node) {
            if dfs(g, n, visited, stack) { return true; }
        }
        stack.remove(node);
        false
    }

    for node_id in g.all_nodes().map(|n| n.id.clone()) {
        if !visited.contains(&node_id) {
            if dfs(g, &node_id, &mut visited, &mut stack) { return true; }
        }
    }
    false
}

/// Topological sort of the graph. Returns None if there's a cycle.
pub fn topological_sort(g: &crate::graph::RelationGraph) -> Option<Vec<NodeId>> {
    let mut indeg: HashMap<&NodeId, usize> = HashMap::new();
    for id in g.all_nodes().map(|n| &n.id) {
        indeg.insert(id, 0);
    }
    for edge in g.all_edges() {
        *indeg.entry(&edge.to).or_insert(0) += 1;
    }
    let mut queue: VecDeque<&NodeId> = indeg.iter().filter(|(_, d)| **d == 0).map(|(k, _)| *k).collect();
    let mut result = Vec::new();
    while let Some(node) = queue.pop_front() {
        result.push(node.clone());
        for neighbor in g.out_neighbors(node) {
            if let Some(d) = indeg.get_mut(neighbor) {
                *d -= 1;
                if *d == 0 { queue.push_back(neighbor); }
            }
        }
    }
    if result.len() == g.node_count() { Some(result) } else { None }
}

/// Find connected components in the undirected version of the graph.
pub fn connected_components(g: &crate::graph::RelationGraph) -> Vec<Vec<NodeId>> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut components = Vec::new();

    for node_id in g.all_nodes().map(|n| n.id.clone()) {
        if !visited.contains(&node_id) {
            let mut comp = Vec::new();
            let mut stack = vec![node_id.clone()];
            while let Some(n) = stack.pop() {
                if visited.contains(&n) { continue; }
                visited.insert(n.clone());
                comp.push(n.clone());
                for nb in g.out_neighbors(&n) { stack.push(nb.clone()); }
                for nb in g.in_neighbors(&n) { stack.push(nb.clone()); }
            }
            comp.sort();
            components.push(comp);
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, RelationGraph};

    fn linear() -> RelationGraph {
        let mut g = RelationGraph::new();
        for id in ["a", "b", "c"] {
            g.add_node(GraphNode::new(id));
        }
        g.add_edge(GraphEdge::new(0, "a", "b", "e"));
        g.add_edge(GraphEdge::new(1, "b", "c", "e"));
        g
    }

    fn cyclic() -> RelationGraph {
        let mut g = linear();
        g.add_edge(GraphEdge::new(2, "c", "a", "e"));
        g
    }

    #[test]
    fn find_path_works() {
        let g = linear();
        let p = find_path(&g, "a", "c").unwrap();
        assert_eq!(p, vec!["a", "b", "c"]);
    }

    #[test]
    fn cycle_detect_finds_cycle() {
        let g = cyclic();
        assert!(cycle_detect(&g));
    }

    #[test]
    fn cycle_detect_no_cycle() {
        let g = linear();
        assert!(!cycle_detect(&g));
    }

    #[test]
    fn topological_sort_linear() {
        let g = linear();
        let sorted = topological_sort(&g).unwrap();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn topological_sort_returns_none_for_cycle() {
        let g = cyclic();
        assert!(topological_sort(&g).is_none());
    }

    #[test]
    fn connected_components_two() {
        let mut g = linear();
        g.add_node(GraphNode::new("isolated"));
        let comps = connected_components(&g);
        assert_eq!(comps.len(), 2);
    }
}
