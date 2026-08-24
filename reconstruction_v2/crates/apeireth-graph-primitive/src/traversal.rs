//! BFS/DFS iterators + shortest path.

use std::collections::{HashMap, VecDeque};
use crate::graph::NodeId;

pub use crate::graph::{EdgeId, GraphEdge, GraphNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathResult {
    Found(Vec<NodeId>),
    NotFound,
}

/// BFS iterator.
pub struct BfsIter<'a> {
    pub(crate) graph: &'a crate::graph::RelationGraph,
    pub(crate) queue: VecDeque<(NodeId, Vec<NodeId>)>,
    pub(crate) direction: TraversalDirection,
}

impl<'a> Iterator for BfsIter<'a> {
    type Item = (NodeId, Vec<NodeId>);
    fn next(&mut self) -> Option<Self::Item> {
        let (current, path) = self.queue.pop_front()?;
        let neighbors: Vec<NodeId> = match self.direction {
            TraversalDirection::Outgoing => self.graph.out_neighbors(&current).into_iter().cloned().collect(),
            TraversalDirection::Incoming => self.graph.in_neighbors(&current).into_iter().cloned().collect(),
            TraversalDirection::Both => {
                let mut v = self.graph.out_neighbors(&current).into_iter().cloned().collect::<Vec<_>>();
                v.extend(self.graph.in_neighbors(&current).into_iter().cloned());
                v
            }
        };
        for n in neighbors {
            if !path.contains(&n) {
                let mut new_path = path.clone();
                new_path.push(n.clone());
                self.queue.push_back((n, new_path));
            }
        }
        Some((current, path))
    }
}

/// DFS iterator (iterative).
pub struct DfsIter<'a> {
    pub(crate) graph: &'a crate::graph::RelationGraph,
    pub(crate) stack: Vec<(NodeId, Vec<NodeId>)>,
    pub(crate) direction: TraversalDirection,
}

impl<'a> Iterator for DfsIter<'a> {
    type Item = (NodeId, Vec<NodeId>);
    fn next(&mut self) -> Option<Self::Item> {
        let (current, path) = self.stack.pop()?;
        let neighbors: Vec<NodeId> = match self.direction {
            TraversalDirection::Outgoing => self.graph.out_neighbors(&current).into_iter().cloned().collect(),
            TraversalDirection::Incoming => self.graph.in_neighbors(&current).into_iter().cloned().collect(),
            TraversalDirection::Both => {
                let mut v = self.graph.out_neighbors(&current).into_iter().cloned().collect::<Vec<_>>();
                v.extend(self.graph.in_neighbors(&current).into_iter().cloned());
                v
            }
        };
        for n in neighbors.into_iter().rev() {
            if !path.contains(&n) {
                let mut new_path = path.clone();
                new_path.push(n.clone());
                self.stack.push((n, new_path));
            }
        }
        Some((current, path))
    }
}

impl crate::graph::RelationGraph {
    pub fn bfs(&self, start: &str) -> BfsIter<'_> {
        let mut q = VecDeque::new();
        q.push_back((start.to_string(), vec![start.to_string()]));
        BfsIter { graph: self, queue: q, direction: TraversalDirection::Outgoing }
    }
    pub fn dfs(&self, start: &str) -> DfsIter<'_> {
        let mut s = Vec::new();
        s.push((start.to_string(), vec![start.to_string()]));
        DfsIter { graph: self, stack: s, direction: TraversalDirection::Outgoing }
    }
}

/// Shortest path via BFS.
pub fn shortest_path(g: &crate::graph::RelationGraph, from: &str, to: &str) -> PathResult {
    if from == to { return PathResult::Found(vec![from.to_string()]); }
    let mut visited: HashMap<NodeId, NodeId> = HashMap::new();
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    queue.push_back(from.to_string());
    visited.insert(from.to_string(), from.to_string());
    while let Some(node) = queue.pop_front() {
        for neighbor in g.out_neighbors(&node) {
            let n = neighbor.clone();
            if !visited.contains_key(&n) {
                visited.insert(n.clone(), node.clone());
                if n == to {
                    // reconstruct
                    let mut path = vec![n.clone()];
                    let mut cur = node;
                    while cur != from {
                        path.push(cur.clone());
                        cur = visited.get(&cur).unwrap().clone();
                    }
                    path.push(from.to_string());
                    path.reverse();
                    return PathResult::Found(path);
                }
                queue.push_back(n);
            }
        }
    }
    PathResult::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, RelationGraph};

    fn make_graph() -> RelationGraph {
        let mut g = RelationGraph::new();
        for id in ["a", "b", "c", "d"] {
            g.add_node(GraphNode::new(id));
        }
        g.add_edge(GraphEdge::new(0, "a", "b", "e"));
        g.add_edge(GraphEdge::new(1, "b", "c", "e"));
        g.add_edge(GraphEdge::new(2, "c", "d", "e"));
        g
    }

    #[test]
    fn bfs_visits_all() {
        let g = make_graph();
        let visited: Vec<_> = g.bfs("a").map(|(n, _)| n).collect();
        assert_eq!(visited.len(), 4);
    }

    #[test]
    fn dfs_visits_all() {
        let g = make_graph();
        let visited: Vec<_> = g.dfs("a").map(|(n, _)| n).collect();
        assert_eq!(visited.len(), 4);
    }

    #[test]
    fn shortest_path_finds() {
        let g = make_graph();
        match shortest_path(&g, "a", "d") {
            PathResult::Found(path) => {
                assert_eq!(path, vec!["a", "b", "c", "d"]);
            }
            PathResult::NotFound => panic!("expected path"),
        }
    }

    #[test]
    fn shortest_path_not_found() {
        let g = make_graph();
        let mut g2 = g;
        g2.add_node(GraphNode::new("isolated"));
        assert!(matches!(shortest_path(&g2, "a", "isolated"), PathResult::NotFound));
    }

    #[test]
    fn shortest_path_same() {
        let g = make_graph();
        assert!(matches!(shortest_path(&g, "a", "a"), PathResult::Found(_)));
    }
}
