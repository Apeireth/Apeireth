//! Graph traversal — BFS / DFS iterators, depth-limited walks, neighbor queries.
//!
//! ## Borrowed Reference (per O-5)
//!
//! Inspired by **SurrealDB** traversal via `->` arrow operator and Neo4j
//! BFS/DFS semantics. We mirror the **directional edge traversal** + **depth
//! limit** + **visit-set memoization**, but as Rust iterators (not lazy DB).
//!
//! ## Design
//!
//! - `TraversalDirection` — Outgoing / Incoming / Both (undirected)
//! - `bfs(root)` / `dfs(root)` — level-order / pre-order iterators
//! - `walk_depth_limited(root, max_depth)` — bounded exploration
//! - `neighbors(node, direction)` — single-step neighbor list

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::{HashSet, VecDeque};

use crate::graph::{EdgeId, GraphEdge, NodeId, RelationGraph};

// ============================================================================
// §1 TraversalDirection
// ============================================================================

/// Direction of edge traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraversalDirection {
    /// Follow outgoing edges only (follow -> arrows).
    Outgoing,
    /// Follow incoming edges only (reverse <- arrows).
    Incoming,
    /// Follow both (undirected traversal).
    Both,
}

// ============================================================================
// §2 BFS iterator
// ============================================================================

/// BFS (breadth-first search) iterator over nodes reachable from `root`.
pub struct BfsIter<'g> {
    graph: &'g RelationGraph,
    queue: VecDeque<(NodeId, usize)>,
    visited: HashSet<NodeId>,
    direction: TraversalDirection,
    max_depth: Option<usize>,
}

impl<'g> BfsIter<'g> {
    /// Create a BFS iterator over all reachable nodes.
    pub fn new(graph: &'g RelationGraph, root: impl Into<NodeId>) -> Self {
        let mut visited = HashSet::new();
        let root_id = root.into();
        visited.insert(root_id.clone());
        let mut queue = VecDeque::new();
        queue.push_back((root_id, 0));
        Self {
            graph,
            queue,
            visited,
            direction: TraversalDirection::Both,
            max_depth: None,
        }
    }

    /// Restrict to a specific direction.
    pub fn with_direction(mut self, dir: TraversalDirection) -> Self {
        self.direction = dir;
        self
    }

    /// Restrict traversal depth.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
        self
    }

    fn next_neighbors(&self, node_id: &str) -> Vec<NodeId> {
        let mut neighbors: Vec<NodeId> = match self.direction {
            TraversalDirection::Outgoing => self
                .graph
                .outgoing(node_id)
                .iter()
                .map(|e| e.to.clone())
                .collect(),
            TraversalDirection::Incoming => self
                .graph
                .incoming(node_id)
                .iter()
                .map(|e| e.from.clone())
                .collect(),
            TraversalDirection::Both => {
                let mut n: Vec<NodeId> = self
                    .graph
                    .outgoing(node_id)
                    .iter()
                    .map(|e| e.to.clone())
                    .collect();
                n.extend(self.graph.incoming(node_id).iter().map(|e| e.from.clone()));
                n
            }
        };
        // Dedup (a neighbor might be both out and in target)
        neighbors.sort();
        neighbors.dedup();
        neighbors
    }
}

impl<'g> Iterator for BfsIter<'g> {
    type Item = (NodeId, usize); // (node_id, depth)

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((node_id, depth)) = self.queue.pop_front() {
            if let Some(max_d) = self.max_depth {
                if depth >= max_d {
                    return Some((node_id, depth));
                }
            }
            for next_node in self.next_neighbors(&node_id) {
                if !self.visited.contains(&next_node) {
                    self.visited.insert(next_node.clone());
                    self.queue.push_back((next_node, depth + 1));
                }
            }
            Some((node_id, depth))
        } else {
            None
        }
    }
}

// ============================================================================
// §3 DFS iterator
// ============================================================================

/// DFS (depth-first search) iterator over nodes reachable from `root`.
pub struct DfsIter<'g> {
    graph: &'g RelationGraph,
    stack: Vec<(NodeId, usize)>,
    visited: HashSet<NodeId>,
    direction: TraversalDirection,
    max_depth: Option<usize>,
}

impl<'g> DfsIter<'g> {
    /// Create a DFS iterator over all reachable nodes.
    pub fn new(graph: &'g RelationGraph, root: impl Into<NodeId>) -> Self {
        let mut visited = HashSet::new();
        let root_id = root.into();
        visited.insert(root_id.clone());
        let mut stack = Vec::new();
        stack.push((root_id, 0));
        Self {
            graph,
            stack,
            visited,
            direction: TraversalDirection::Both,
            max_depth: None,
        }
    }

    /// Restrict to a specific direction.
    pub fn with_direction(mut self, dir: TraversalDirection) -> Self {
        self.direction = dir;
        self
    }

    /// Restrict traversal depth.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
        self
    }

    fn next_neighbors(&self, node_id: &str) -> Vec<NodeId> {
        let mut neighbors: Vec<NodeId> = match self.direction {
            TraversalDirection::Outgoing => self
                .graph
                .outgoing(node_id)
                .iter()
                .map(|e| e.to.clone())
                .collect(),
            TraversalDirection::Incoming => self
                .graph
                .incoming(node_id)
                .iter()
                .map(|e| e.from.clone())
                .collect(),
            TraversalDirection::Both => {
                let mut n: Vec<NodeId> = self
                    .graph
                    .outgoing(node_id)
                    .iter()
                    .map(|e| e.to.clone())
                    .collect();
                n.extend(self.graph.incoming(node_id).iter().map(|e| e.from.clone()));
                n
            }
        };
        neighbors.sort();
        neighbors.dedup();
        neighbors
    }
}

impl<'g> Iterator for DfsIter<'g> {
    type Item = (NodeId, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((node_id, depth)) = self.stack.pop() {
            if let Some(max_d) = self.max_depth {
                if depth < max_d {
                    for next_node in self.next_neighbors(&node_id) {
                        if !self.visited.contains(&next_node) {
                            self.visited.insert(next_node.clone());
                            self.stack.push((next_node, depth + 1));
                        }
                    }
                }
            } else {
                for next_node in self.next_neighbors(&node_id) {
                    if !self.visited.contains(&next_node) {
                        self.visited.insert(next_node.clone());
                        self.stack.push((next_node, depth + 1));
                    }
                }
            }
            Some((node_id, depth))
        } else {
            None
        }
    }
}

// ============================================================================
// §4 Shortest path via BFS
// ============================================================================

/// Result of a shortest-path query: ordered list of node IDs (inclusive of
/// `from` and `to`), plus the list of edges traversed.
#[derive(Debug, Clone, PartialEq)]
pub struct PathResult {
    /// Node IDs from start to end (inclusive).
    pub nodes: Vec<NodeId>,
    /// Edge IDs in traversal order (len = nodes.len() - 1).
    pub edges: Vec<EdgeId>,
}

impl PathResult {
    /// Number of hops.
    pub fn hop_count(&self) -> usize {
        self.nodes.len().saturating_sub(1)
    }

    /// Is this the trivial path (start == end)?
    pub fn is_trivial(&self) -> bool {
        self.nodes.len() == 1
    }
}

/// Find shortest path between two nodes (BFS, undirected by default).
/// Returns `None` if no path exists.
pub fn shortest_path(graph: &RelationGraph, from: &str, to: &str) -> Option<PathResult> {
    if !graph.get_node(from).is_some() || !graph.get_node(to).is_some() {
        return None;
    }
    if from == to {
        return Some(PathResult {
            nodes: vec![from.to_string()],
            edges: vec![],
        });
    }

    let mut visited: HashSet<NodeId> = HashSet::new();
    // BFS with parent tracking
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    let mut parent: std::collections::HashMap<NodeId, (NodeId, EdgeId)> =
        std::collections::HashMap::new();

    visited.insert(from.to_string());
    queue.push_back(from.to_string());

    while let Some(current) = queue.pop_front() {
        // Gather all edges (out + in for undirected)
        let edges: Vec<&GraphEdge> = graph
            .outgoing(&current)
            .into_iter()
            .chain(graph.incoming(&current))
            .collect();

        for edge in edges {
            let next = if edge.from == current {
                edge.to.clone()
            } else {
                edge.from.clone()
            };
            if !visited.contains(&next) {
                visited.insert(next.clone());
                parent.insert(next.clone(), (current.clone(), edge.id.clone()));
                if next == to {
                    // Reconstruct path by walking back through parent pointers.
                    // parent["y"] = (parent_node, edge_from_parent_to_y).
                    // We build the path in reverse, then reverse it.
                    let mut nodes_rev: Vec<NodeId> = vec![to.to_string()];
                    let mut edges_rev: Vec<EdgeId> = Vec::new();
                    let mut cursor = to.to_string();
                    while let Some((p_node, p_edge)) = parent.get(&cursor) {
                        nodes_rev.push(p_node.clone());
                        edges_rev.push(p_edge.clone());
                        cursor = p_node.clone();
                    }
                    nodes_rev.reverse();
                    edges_rev.reverse();
                    return Some(PathResult {
                        nodes: nodes_rev,
                        edges: edges_rev,
                    });
                }
                queue.push_back(next);
            }
        }
    }
    None
}

// ============================================================================
// §6 Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, RelationGraph};
    use crate::RelationKind;

    fn chain_graph() -> RelationGraph {
        let mut g = RelationGraph::new();
        for n in &["a", "b", "c", "d", "e"] {
            g.insert_node(GraphNode::new(*n)).unwrap();
        }
        g.insert_edge(GraphEdge::new("a", RelationKind::Coordination, "b"))
            .unwrap();
        g.insert_edge(GraphEdge::new("b", RelationKind::Coordination, "c"))
            .unwrap();
        g.insert_edge(GraphEdge::new("c", RelationKind::Coordination, "d"))
            .unwrap();
        g.insert_edge(GraphEdge::new("d", RelationKind::Coordination, "e"))
            .unwrap();
        g
    }

    #[test]
    fn test_bfs_root_only() {
        let g = chain_graph();
        let nodes: Vec<_> = BfsIter::new(&g, "a").map(|(n, _)| n).collect();
        assert_eq!(nodes[0], "a");
    }

    #[test]
    fn test_bfs_visits_all_chain() {
        let g = chain_graph();
        let nodes: Vec<_> = BfsIter::new(&g, "a").map(|(n, _)| n).collect();
        assert_eq!(nodes.len(), 5);
        // BFS order: a, b, c, d, e
        assert_eq!(nodes, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn test_bfs_max_depth_2() {
        let g = chain_graph();
        let nodes: Vec<_> = BfsIter::new(&g, "a")
            .with_max_depth(2)
            .map(|(n, _)| n)
            .collect();
        // depth 0: a; depth 1: b; depth 2: c
        assert_eq!(nodes, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_dfs_visits_all_chain() {
        let g = chain_graph();
        let nodes: Vec<_> = DfsIter::new(&g, "a").map(|(n, _)| n).collect();
        assert_eq!(nodes.len(), 5);
        // DFS pre-order: a, b, c, d, e (chain has only one path)
        assert_eq!(nodes, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn test_dfs_max_depth() {
        let g = chain_graph();
        let nodes: Vec<_> = DfsIter::new(&g, "a")
            .with_max_depth(3)
            .map(|(n, _)| n)
            .collect();
        // DFS with depth limit 3: a, b, c, d
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0], "a");
    }

    #[test]
    fn test_directional_outgoing() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("a")).unwrap();
        g.insert_node(GraphNode::new("b")).unwrap();
        g.insert_node(GraphNode::new("c")).unwrap();
        g.insert_edge(GraphEdge::new("a", RelationKind::Coordination, "b"))
            .unwrap();
        g.insert_edge(GraphEdge::new("c", RelationKind::Coordination, "a"))
            .unwrap();
        // Outgoing from a only: [b]
        let nodes: Vec<_> = BfsIter::new(&g, "a")
            .with_direction(TraversalDirection::Outgoing)
            .map(|(n, _)| n)
            .collect();
        // Should NOT visit c via incoming edge
        assert_eq!(nodes, vec!["a", "b"]);
    }

    #[test]
    fn test_shortest_path_via_traversal_module() {
        let g = chain_graph();
        let path = shortest_path(&g, "a", "e").unwrap();
        assert_eq!(path.nodes, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(path.edges.len(), 4);
        assert_eq!(path.hop_count(), 4);
    }

    #[test]
    fn test_shortest_path_self_is_trivial() {
        let g = chain_graph();
        let path = shortest_path(&g, "a", "a").unwrap();
        assert!(path.is_trivial());
        assert_eq!(path.hop_count(), 0);
    }

    #[test]
    fn test_shortest_path_returns_none_for_disconnected() {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("x")).unwrap();
        g.insert_node(GraphNode::new("y")).unwrap();
        assert!(shortest_path(&g, "x", "y").is_none());
    }
}
