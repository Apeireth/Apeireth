//! GraphOps - 图操作 (BFS/DFS 遍历, 关联查询) (从 v1.0 apeireth-graph 6K LOC 收敛)
//!
//! 0 装 PASS: 简化图算法 (BFS, DFS, shortest_path), 完整 v1.0 era (PageRank, community detection) 不做.

use std::collections::{HashSet, VecDeque};
use super::graph_primitive::Graph;

/// BFS - 返回从 start 出发 bfs 顺序
pub fn bfs(graph: &Graph, start: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut order = Vec::new();
    if graph.node(start).is_none() { return order; }
    queue.push_back(start.to_string());
    visited.insert(start.to_string());
    while let Some(n) = queue.pop_front() {
        order.push(n.clone());
        for nb in graph.neighbors(&n) {
            if visited.insert(nb.id.clone()) {
                queue.push_back(nb.id.clone());
            }
        }
    }
    order
}

/// DFS - 深度优先遍历
pub fn dfs(graph: &Graph, start: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    if graph.node(start).is_none() { return order; }
    dfs_visit(graph, start, &mut visited, &mut order);
    order
}

fn dfs_visit(graph: &Graph, node: &str, visited: &mut HashSet<String>, order: &mut Vec<String>) {
    visited.insert(node.to_string());
    order.push(node.to_string());
    for nb in graph.neighbors(node) {
        if !visited.contains(&nb.id) {
            dfs_visit(graph, &nb.id, visited, order);
        }
    }
}

/// 0 装 PASS: BFS 找最短路径 (无权图), 找不到返 None
pub fn shortest_path(graph: &Graph, from: &str, to: &str) -> Option<Vec<String>> {
    if from == to { return Some(vec![from.to_string()]); }
    if graph.node(from).is_none() || graph.node(to).is_none() { return None; }
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parents: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    queue.push_back(from.to_string());
    visited.insert(from.to_string());
    while let Some(n) = queue.pop_front() {
        if n == to {
            // 回溯
            let mut path = vec![n.clone()];
            while let Some(p) = parents.get(&path[path.len()-1]) {
                path.push(p.clone());
            }
            path.reverse();
            return Some(path);
        }
        for nb in graph.neighbors(&n) {
            if visited.insert(nb.id.clone()) {
                parents.insert(nb.id.clone(), n.clone());
                queue.push_back(nb.id.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::graph_primitive::{Node, Edge};
    fn mk() -> Graph {
        let mut g = Graph::new();
        g.add_node(Node { id: "a".into(), label: "A".into() });
        g.add_node(Node { id: "b".into(), label: "B".into() });
        g.add_node(Node { id: "c".into(), label: "C".into() });
        g.add_edge(Edge { from: "a".into(), to: "b".into(), label: "ab".into() });
        g.add_edge(Edge { from: "b".into(), to: "c".into(), label: "bc".into() });
        g
    }
    #[test] fn test_bfs() {
        let g = mk();
        assert_eq!(bfs(&g, "a"), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
    #[test] fn test_dfs() {
        let g = mk();
        let order = dfs(&g, "a");
        assert!(order.contains(&"a".to_string()));
        assert!(order.contains(&"c".to_string()));
    }
    #[test] fn test_shortest_path() {
        let g = mk();
        let path = shortest_path(&g, "a", "c").unwrap();
        assert_eq!(path, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
    #[test] fn test_shortest_path_same() {
        let g = mk();
        assert_eq!(shortest_path(&g, "a", "a").unwrap(), vec!["a".to_string()]);
    }
}
