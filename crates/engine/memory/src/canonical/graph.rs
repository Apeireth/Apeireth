//! Canonical in-memory graph primitives (M1B3).
//!
//! This module owns simple, deterministic relationship/query primitives for
//! memory infrastructure. It is not a knowledge-graph product, a planner, or
//! a causal cognition engine. The donor graph implementations are in-memory
//! only, so this graph makes no persistence promise.

use std::collections::{HashMap, HashSet, VecDeque};

use super::domain::MemoryId;
use super::error::MemoryError;

/// A graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: MemoryId,
    pub label: String,
}

impl Node {
    /// Creates a node.
    pub fn new(id: MemoryId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}

/// A directed, weighted edge with a relation kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: MemoryId,
    pub to: MemoryId,
    pub relation: String,
    pub weight: f64,
}

impl Edge {
    /// Creates an edge with `relation` and `weight`.
    pub fn new(from: MemoryId, to: MemoryId, relation: impl Into<String>, weight: f64) -> Self {
        Self {
            from,
            to,
            relation: relation.into(),
            weight,
        }
    }
}

/// In-memory directed graph with deterministic neighbours and cycle-safe
/// bounded traversal.
#[derive(Debug, Clone, Default)]
pub struct MemoryGraph {
    nodes: HashMap<MemoryId, Node>,
    edges: Vec<Edge>,
}

impl MemoryGraph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` when the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Adds a node. Duplicate ids are rejected with [`MemoryError::Conflict`].
    pub fn add_node(&mut self, node: Node) -> Result<(), MemoryError> {
        if self.nodes.contains_key(&node.id) {
            return Err(MemoryError::Conflict(format!(
                "graph node already exists: {}",
                node.id
            )));
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Removes a node and all edges incident to it.
    ///
    /// Missing ids fail with [`MemoryError::NotFound`].
    pub fn remove_node(&mut self, id: &MemoryId) -> Result<(), MemoryError> {
        if self.nodes.remove(id).is_none() {
            return Err(MemoryError::NotFound(format!("graph node not found: {id}")));
        }
        self.edges.retain(|edge| &edge.from != id && &edge.to != id);
        Ok(())
    }

    /// Returns the node with `id`, if present.
    pub fn node(&self, id: &MemoryId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Adds a directed edge.
    ///
    /// Both endpoint nodes must exist, `relation` must not be empty, `weight`
    /// must be finite, and an exact duplicate `(from, to, relation)` is
    /// rejected with [`MemoryError::Conflict`].
    pub fn add_edge(&mut self, edge: Edge) -> Result<(), MemoryError> {
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return Err(MemoryError::NotFound(
                "edge endpoint node does not exist".into(),
            ));
        }
        if edge.relation.is_empty() {
            return Err(MemoryError::InvalidData(
                "edge relation must not be empty".into(),
            ));
        }
        if !edge.weight.is_finite() {
            return Err(MemoryError::InvalidData(
                "edge weight must be finite".into(),
            ));
        }
        if self.edges.iter().any(|existing| {
            existing.from == edge.from
                && existing.to == edge.to
                && existing.relation == edge.relation
        }) {
            return Err(MemoryError::Conflict(format!(
                "edge already exists: {} -[{}]-> {}",
                edge.from, edge.relation, edge.to
            )));
        }

        self.edges.push(edge);
        Ok(())
    }

    /// Removes the exact directed edge `(from, to, relation)`, if present.
    ///
    /// Missing edges fail with [`MemoryError::NotFound`].
    pub fn remove_edge(
        &mut self,
        from: &MemoryId,
        to: &MemoryId,
        relation: &str,
    ) -> Result<(), MemoryError> {
        let before = self.edges.len();
        self.edges
            .retain(|edge| !(&edge.from == from && &edge.to == to && edge.relation == relation));
        if self.edges.len() == before {
            return Err(MemoryError::NotFound(format!(
                "edge not found: {from} -[{relation}]-> {to}"
            )));
        }
        Ok(())
    }

    /// Outgoing edges from `id`, deterministically ordered by relation and
    /// then destination id.
    pub fn edges_from(&self, id: &MemoryId) -> Vec<&Edge> {
        let mut edges: Vec<&Edge> = self.edges.iter().filter(|edge| &edge.from == id).collect();
        edges.sort_by(|a, b| a.relation.cmp(&b.relation).then_with(|| a.to.cmp(&b.to)));
        edges
    }

    /// Outgoing neighbours of `id`, deterministically ordered by destination
    /// id.
    pub fn neighbors(&self, id: &MemoryId) -> Vec<&Node> {
        let mut edges: Vec<&Edge> = self.edges.iter().filter(|edge| &edge.from == id).collect();
        edges.sort_by(|a, b| a.to.cmp(&b.to));
        edges
            .into_iter()
            .filter_map(|edge| self.nodes.get(&edge.to))
            .collect()
    }

    /// BFS traversal from `start`, bounded by `max_depth` and safe on cycles.
    ///
    /// Returns node ids in deterministic BFS order. `max_depth = 0` returns
    /// only `start` when it exists. Missing `start` returns an empty vector.
    pub fn traverse(&self, start: &MemoryId, max_depth: usize) -> Vec<MemoryId> {
        if !self.nodes.contains_key(start) {
            return Vec::new();
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut order = Vec::new();

        visited.insert(start.clone());
        queue.push_back((start.clone(), 0_usize));

        while let Some((node, depth)) = queue.pop_front() {
            order.push(node.clone());
            if depth >= max_depth {
                continue;
            }
            for neighbor in self.neighbors(&node) {
                if visited.insert(neighbor.id.clone()) {
                    queue.push_back((neighbor.id.clone(), depth + 1));
                }
            }
        }

        order
    }

    /// Unweighted shortest path from `from` to `to` by BFS.
    ///
    /// Returns `None` when either node is missing or no path exists. A
    /// self-path returns `Some(vec![from])` when the node exists.
    pub fn shortest_path(&self, from: &MemoryId, to: &MemoryId) -> Option<Vec<MemoryId>> {
        if from == to {
            return self.nodes.contains_key(from).then(|| vec![from.clone()]);
        }
        if !self.nodes.contains_key(from) || !self.nodes.contains_key(to) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parents: HashMap<MemoryId, MemoryId> = HashMap::new();

        visited.insert(from.clone());
        queue.push_back(from.clone());

        while let Some(node) = queue.pop_front() {
            if node == *to {
                let mut path = vec![node.clone()];
                while let Some(parent) = parents.get(&path[path.len() - 1]) {
                    path.push(parent.clone());
                }
                path.reverse();
                return Some(path);
            }

            for neighbor in self.neighbors(&node) {
                if visited.insert(neighbor.id.clone()) {
                    parents.insert(neighbor.id.clone(), node.clone());
                    queue.push_back(neighbor.id.clone());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> MemoryId {
        MemoryId::new(s).unwrap()
    }

    fn node(id: &str) -> Node {
        Node::new(MemoryId::new(id).unwrap(), id)
    }

    #[test]
    fn insert_nodes_edges_and_query_neighbors() {
        let mut graph = MemoryGraph::new();
        graph.add_node(node("a")).unwrap();
        graph.add_node(node("b")).unwrap();
        graph.add_node(node("c")).unwrap();
        graph
            .add_edge(Edge::new(id("a"), id("b"), "knows", 1.0))
            .unwrap();
        graph
            .add_edge(Edge::new(id("a"), id("c"), "knows", 1.0))
            .unwrap();

        assert_eq!(graph.len(), 3);
        assert_eq!(graph.edge_count(), 2);

        let neighbors = graph.neighbors(&id("a"));
        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors[0].id, id("b"));
        assert_eq!(neighbors[1].id, id("c"));
    }

    #[test]
    fn remove_edge_and_remove_node_semantics() {
        let mut graph = MemoryGraph::new();
        graph.add_node(node("a")).unwrap();
        graph.add_node(node("b")).unwrap();
        graph
            .add_edge(Edge::new(id("a"), id("b"), "knows", 1.0))
            .unwrap();

        graph.remove_edge(&id("a"), &id("b"), "knows").unwrap();
        assert_eq!(graph.edge_count(), 0);
        assert!(matches!(
            graph.remove_edge(&id("a"), &id("b"), "knows"),
            Err(MemoryError::NotFound(_))
        ));

        graph
            .add_edge(Edge::new(id("a"), id("b"), "knows", 1.0))
            .unwrap();
        graph.remove_node(&id("a")).unwrap();
        assert!(graph.node(&id("a")).is_none());
        assert_eq!(graph.edge_count(), 0);
        assert!(matches!(
            graph.remove_node(&id("a")),
            Err(MemoryError::NotFound(_))
        ));
    }

    #[test]
    fn missing_node_and_duplicate_edge_behavior() {
        let mut graph = MemoryGraph::new();
        graph.add_node(node("a")).unwrap();
        graph.add_node(node("b")).unwrap();

        assert!(matches!(
            graph.add_edge(Edge::new(id("a"), id("missing"), "knows", 1.0)),
            Err(MemoryError::NotFound(_))
        ));

        graph
            .add_edge(Edge::new(id("a"), id("b"), "knows", 1.0))
            .unwrap();
        assert!(matches!(
            graph.add_edge(Edge::new(id("a"), id("b"), "knows", 1.0)),
            Err(MemoryError::Conflict(_))
        ));
        assert!(matches!(
            graph.add_edge(Edge::new(id("a"), id("b"), "", 1.0)),
            Err(MemoryError::InvalidData(_))
        ));
        assert!(matches!(
            graph.add_edge(Edge::new(id("a"), id("b"), "knows", f64::NAN)),
            Err(MemoryError::InvalidData(_))
        ));
    }

    #[test]
    fn traversal_is_cycle_safe_and_depth_bounded() {
        let mut graph = MemoryGraph::new();
        graph.add_node(node("a")).unwrap();
        graph.add_node(node("b")).unwrap();
        graph.add_node(node("c")).unwrap();
        graph
            .add_edge(Edge::new(id("a"), id("b"), "next", 1.0))
            .unwrap();
        graph
            .add_edge(Edge::new(id("b"), id("c"), "next", 1.0))
            .unwrap();
        graph
            .add_edge(Edge::new(id("c"), id("a"), "next", 1.0))
            .unwrap();

        let order = graph.traverse(&id("a"), 0);
        assert_eq!(order, vec![id("a")]);

        let order = graph.traverse(&id("a"), 1);
        assert_eq!(order, vec![id("a"), id("b")]);

        // A cycle must not produce duplicates or an infinite loop.
        let order = graph.traverse(&id("a"), 10);
        assert_eq!(order.len(), 3);

        assert!(graph.traverse(&id("missing"), 10).is_empty());
    }

    #[test]
    fn shortest_path_is_deterministic_and_handles_missing_nodes() {
        let mut graph = MemoryGraph::new();
        graph.add_node(node("a")).unwrap();
        graph.add_node(node("b")).unwrap();
        graph.add_node(node("c")).unwrap();
        graph
            .add_edge(Edge::new(id("a"), id("b"), "next", 1.0))
            .unwrap();
        graph
            .add_edge(Edge::new(id("b"), id("c"), "next", 1.0))
            .unwrap();

        assert_eq!(
            graph.shortest_path(&id("a"), &id("c")),
            Some(vec![id("a"), id("b"), id("c")])
        );
        assert_eq!(graph.shortest_path(&id("a"), &id("a")), Some(vec![id("a")]));
        assert_eq!(graph.shortest_path(&id("a"), &id("missing")), None);
        assert_eq!(graph.shortest_path(&id("missing"), &id("a")), None);
    }
}
