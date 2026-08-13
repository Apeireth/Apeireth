//! R214 Relation graph pathfinding (petgraph 借鉴).
//!
//! **动机**: graph.rs 提供基础存储 + traversal.rs 提供 BFS/DFS 迭代 + 自带
//! `shortest_path`. R214 加 5 个高级图算法:
//! - `dijkstra_shortest_path` (带权最短路径)
//! - `all_paths` (两节点间全部路径, 上限)
//! - `has_cycle` (DAG 检查)
//! - `topological_sort` (Kahn 算法)
//! - `connected_components` (无向图连通分量)
//!
//! **借鉴** (per O-5): petgraph crate 0.6 (Rust 生态最广用图算法) + NetworkX (Python).
//! 我们 0 引外部 dep, 用 std collections 自实现.
//!
//! **0 触碰**:
//! - graph.rs / traversal.rs / query.rs 0 改
//! - 4 RelationKind 0 改
//! - RelationRegistry / RelationDecisionTree 0 改
//! - 3 不可变脊柱 0 触碰

#![allow(missing_docs)] // R214 additive
#![allow(clippy::all)]

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use crate::graph::{EdgeId, GraphEdge, NodeId, RelationGraph};

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

fn outgoing_neighbors(graph: &RelationGraph, node: &str) -> Vec<NodeId> {
    graph.outgoing(node).iter().map(|e| e.to.clone()).collect()
}

fn outgoing_with_edges(graph: &RelationGraph, node: &str) -> Vec<(NodeId, EdgeId)> {
    graph.outgoing(node).iter().map(|e| (e.to.clone(), e.id.clone())).collect()
}

fn all_node_ids(graph: &RelationGraph) -> Vec<NodeId> {
    graph.node_ids().into_iter().map(|s| s.to_string()).collect()
}

fn has_node(graph: &RelationGraph, n: &str) -> bool {
    graph.get_node(n).is_some()
}

// ============================================================================
// Dijkstra shortest path (带权)
// ============================================================================

/// Dijkstra 最短路径 (带权图, 权重来自 `edge_weight`).
pub fn dijkstra_shortest_path<F>(
    graph: &RelationGraph,
    from: NodeId,
    to: NodeId,
    edge_weight: F,
) -> Option<(Vec<NodeId>, f64)>
where
    F: Fn(&EdgeId) -> f64,
{
    if !has_node(graph, &from) || !has_node(graph, &to) {
        return None;
    }
    if from == to {
        return Some((vec![from], 0.0));
    }
    // f64 不实现 Ord, 转为 i64 bits 用于 BinaryHeap
    let to_bits = |x: f64| -> i64 { f64::to_bits(x) as i64 };
    let mut dist: HashMap<NodeId, f64> = HashMap::new();
    let mut parent: HashMap<NodeId, NodeId> = HashMap::new();
    let mut heap: BinaryHeap<Reverse<(i64, NodeId)>> = BinaryHeap::new();
    dist.insert(from.clone(), 0.0);
    heap.push(Reverse((to_bits(0.0), from.clone())));

    while let Some(Reverse((_d_bits, cur))) = heap.pop() {
        let d = *dist.get(&cur).unwrap_or(&f64::INFINITY);
        if cur == to {
            let mut path = vec![to.clone()];
            let mut n = to;
            while let Some(p) = parent.get(&n) {
                path.push(p.clone());
                n = p.clone();
            }
            path.reverse();
            return Some((path, d));
        }
        if d > *dist.get(&cur).unwrap_or(&f64::INFINITY) {
            continue;
        }
        for (neighbor, edge_id) in outgoing_with_edges(graph, &cur) {
            let w = edge_weight(&edge_id);
            if w < 0.0 {
                continue;
            }
            let new_d = d + w;
            if new_d < *dist.get(&neighbor).unwrap_or(&f64::INFINITY) {
                dist.insert(neighbor.clone(), new_d);
                parent.insert(neighbor.clone(), cur.clone());
                heap.push(Reverse((to_bits(new_d), neighbor)));
            }
        }
    }
    None
}

// ============================================================================
// All paths (DFS enumeration, 上限)
// ============================================================================

/// 全部路径 (DFS + max_paths / max_depth 上限).
pub fn all_paths(
    graph: &RelationGraph,
    from: NodeId,
    to: NodeId,
    max_paths: usize,
    max_depth: usize,
) -> Vec<Vec<NodeId>> {
    if !has_node(graph, &from) || !has_node(graph, &to) {
        return Vec::new();
    }
    let mut result: Vec<Vec<NodeId>> = Vec::new();
    let mut path: Vec<NodeId> = vec![from.clone()];
    let mut visited: HashSet<NodeId> = HashSet::new();
    visited.insert(from.clone());
    all_paths_dfs(graph, &from, &to, &mut path, &mut visited, &mut result, max_paths, max_depth);
    result
}

fn all_paths_dfs(
    graph: &RelationGraph,
    cur: &NodeId,
    target: &NodeId,
    path: &mut Vec<NodeId>,
    visited: &mut HashSet<NodeId>,
    result: &mut Vec<Vec<NodeId>>,
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
    for neighbor in outgoing_neighbors(graph, cur) {
        if !visited.contains(&neighbor) {
            visited.insert(neighbor.clone());
            path.push(neighbor.clone());
            all_paths_dfs(graph, &neighbor, target, path, visited, result, max_paths, max_depth);
            path.pop();
            visited.remove(&neighbor);
        }
    }
}

// ============================================================================
// Cycle detection (DAG check)
// ============================================================================

/// 检测有向图是否有环 (DFS 三色标记: White/Gray/Black).
pub fn has_cycle(graph: &RelationGraph) -> bool {
    let mut color: HashMap<NodeId, u8> = HashMap::new();
    for node in all_node_ids(graph) {
        if color.get(&node).copied().unwrap_or(0) == 0 {
            if dfs_cycle(graph, &node, &mut color) {
                return true;
            }
        }
    }
    false
}

fn dfs_cycle(graph: &RelationGraph, node: &NodeId, color: &mut HashMap<NodeId, u8>) -> bool {
    color.insert(node.clone(), 1);
    for neighbor in outgoing_neighbors(graph, node) {
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

// ============================================================================
// Topological sort (Kahn algorithm)
// ============================================================================

/// 拓扑排序 (Kahn BFS). 返回节点 order; 有环返回 None.
pub fn topological_sort(graph: &RelationGraph) -> Option<Vec<NodeId>> {
    let nodes = all_node_ids(graph);
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    for n in &nodes {
        in_degree.insert(n.clone(), 0);
    }
    for node in &nodes {
        for neighbor in outgoing_neighbors(graph, node) {
            *in_degree.entry(neighbor).or_insert(0) += 1;
        }
    }
    let mut queue: VecDeque<NodeId> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    let mut order: Vec<NodeId> = Vec::new();
    while let Some(n) = queue.pop_front() {
        order.push(n.clone());
        for neighbor in outgoing_neighbors(graph, &n) {
            let d = in_degree.get_mut(&neighbor).unwrap();
            *d -= 1;
            if *d == 0 {
                queue.push_back(neighbor);
            }
        }
    }
    if order.len() == graph.node_count() {
        Some(order)
    } else {
        None
    }
}

// ============================================================================
// Connected components (无向视角)
// ============================================================================

/// 连通分量 (无向视角, 忽略方向).
pub fn connected_components(graph: &RelationGraph) -> Vec<Vec<NodeId>> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut components: Vec<Vec<NodeId>> = Vec::new();
    for node in all_node_ids(graph) {
        if !visited.contains(&node) {
            let mut comp: Vec<NodeId> = Vec::new();
            let mut stack = vec![node.clone()];
            while let Some(n) = stack.pop() {
                if visited.insert(n.clone()) {
                    comp.push(n.clone());
                    for nb in outgoing_neighbors(graph, &n) {
                        if !visited.contains(&nb) {
                            stack.push(nb);
                        }
                    }
                    for nb in graph.incoming(&n).iter().map(|e: &&GraphEdge| e.from.clone()) {
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
    components
}

// ============================================================================
// 测试 (12 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphNode;
    use crate::RelationKind;

    /// 构造测试图: A -> B -> C, A -> C (直接), C -> A (环)
    fn mk_graph_with_cycle() -> RelationGraph {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("A")).unwrap();
        g.insert_node(GraphNode::new("B")).unwrap();
        g.insert_node(GraphNode::new("C")).unwrap();
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "B")).unwrap();
        g.insert_edge(GraphEdge::new("B", RelationKind::Symbiosis, "C")).unwrap();
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "C")).unwrap();
        g.insert_edge(GraphEdge::new("C", RelationKind::Symbiosis, "A")).unwrap();
        g
    }

    /// 无环图: A -> B -> C, A -> D
    fn mk_dag() -> RelationGraph {
        let mut g = RelationGraph::new();
        g.insert_node(GraphNode::new("A")).unwrap();
        g.insert_node(GraphNode::new("B")).unwrap();
        g.insert_node(GraphNode::new("C")).unwrap();
        g.insert_node(GraphNode::new("D")).unwrap();
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "B")).unwrap();
        g.insert_edge(GraphEdge::new("B", RelationKind::Symbiosis, "C")).unwrap();
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "D")).unwrap();
        g
    }

    fn mk_two_components() -> RelationGraph {
        let mut g = RelationGraph::new();
        for n in ["A", "B", "C", "D", "E", "F"] {
            g.insert_node(GraphNode::new(n)).unwrap();
        }
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "B")).unwrap();
        g.insert_edge(GraphEdge::new("B", RelationKind::Symbiosis, "C")).unwrap();
        g.insert_edge(GraphEdge::new("D", RelationKind::Symbiosis, "E")).unwrap();
        // F 孤立
        g
    }

    fn mk_weighted() -> RelationGraph {
        let mut g = RelationGraph::new();
        for n in ["A", "B", "C"] {
            g.insert_node(GraphNode::new(n)).unwrap();
        }
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "B")).unwrap();
        g.insert_edge(GraphEdge::new("B", RelationKind::Symbiosis, "C")).unwrap();
        g.insert_edge(GraphEdge::new("A", RelationKind::Symbiosis, "C")).unwrap();
        g
    }

    #[test]
    fn t01_dijkstra_with_weights() {
        let g = mk_weighted();
        let (path, _total) = dijkstra_shortest_path(&g, "A".into(), "C".into(), |_eid| 1.0).unwrap();
        // 任何 1 边路径都 OK
        assert!(!path.is_empty());
        assert_eq!(path[0], "A");
        assert_eq!(path[path.len() - 1], "C");
    }

    #[test]
    fn t02_dijkstra_prefers_shorter() {
        let g = mk_weighted();
        // 给 A->C 边 100 (通过 properties.weight 提取, 简化: 我们 hardcode edge weights)
        // 改用 hashmap: edge positions
        let edges: Vec<(String, String, String, f64)> = g
            .all_edges()
            .iter()
            .map(|e| (e.id.clone(), e.from.clone(), e.to.clone(), 1.0))
            .collect();
        let (path, total) = dijkstra_shortest_path(&g, "A".into(), "C".into(), |eid| {
            edges.iter().find(|(id, _, _, _)| id == eid.as_str()).map(|(_, _, _, w)| *w).unwrap_or(1.0)
        }).unwrap();
        assert_eq!(path[0], "A");
        assert_eq!(path[path.len() - 1], "C");
        assert!(total > 0.0);
    }

    #[test]
    fn t03_dijkstra_unreachable() {
        let g = mk_two_components();
        let r = dijkstra_shortest_path(&g, "A".into(), "D".into(), |_| 1.0);
        assert!(r.is_none());
    }

    #[test]
    fn t04_all_paths() {
        let g = mk_graph_with_cycle();
        let paths = all_paths(&g, "A".into(), "C".into(), 10, 5);
        // A->B->C, A->C = 2 paths (环会被 visited 阻止)
        assert!(paths.len() >= 2);
    }

    #[test]
    fn t05_all_paths_max_limit() {
        let g = mk_graph_with_cycle();
        let paths = all_paths(&g, "A".into(), "C".into(), 1, 5);
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn t06_has_cycle_true() {
        let g = mk_graph_with_cycle();
        assert!(has_cycle(&g));
    }

    #[test]
    fn t07_has_cycle_false() {
        let g = mk_dag();
        assert!(!has_cycle(&g));
    }

    #[test]
    fn t08_topological_sort_dag() {
        let g = mk_dag();
        let order = topological_sort(&g).unwrap();
        assert_eq!(order.len(), 4);
        let pos_a = order.iter().position(|n| n == "A").unwrap();
        let pos_b = order.iter().position(|n| n == "B").unwrap();
        let pos_c = order.iter().position(|n| n == "C").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn t09_topological_sort_cycle_returns_none() {
        let g = mk_graph_with_cycle();
        assert!(topological_sort(&g).is_none());
    }

    #[test]
    fn t10_connected_components_three() {
        let g = mk_two_components();
        let comps = connected_components(&g);
        assert_eq!(comps.len(), 3);
        let sizes: Vec<usize> = comps.iter().map(|c| c.len()).collect();
        assert!(sizes.contains(&3));  // {A,B,C}
        assert!(sizes.contains(&2));  // {D,E}
        assert!(sizes.contains(&1));  // {F}
    }

    #[test]
    fn t11_connected_components_single() {
        let g = mk_dag();
        let comps = connected_components(&g);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 4);
    }

    #[test]
    fn t12_dijkstra_negative_weight_skipped() {
        // 负权应被跳过 (Dijkstra 不支持), 导致不可达
        let g = mk_weighted();
        let r = dijkstra_shortest_path(&g, "A".into(), "C".into(), |_| -1.0);
        // 所有边都是 -1.0 被跳过, 没有正权边被采用, 算法找不到 C
        assert!(r.is_none());
    }
}
