//! Local graph runtime — minimal Node/Graph/State primitives (v2 适配).
//!
//! **v2 适配**:
//! v1 依赖 `apeireth_graph` crate (提供 `Node` / `Graph` / `State` / `Edge` 等).
//! v2 没有 apeireth-graph crate. 在本模块本地定义等价的最小 runtime,
//! 公共 API 与 v1 等价 (函数签名 / 方法名 1:1), 让 graph_orchestration.rs 0 改代码即可编译.
//!
//! 设计: 同步 Node trait + 异步 `Graph::execute`. 节点拓扑排序保证 cycle-free.
//! 仅 HashMap<String, serde_json::Value> 做 state 容器 (跟 v1 一致).

#![allow(missing_docs)]

use std::collections::HashMap;
use thiserror::Error;

/// 节点 ID (per mode + instance)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// State 容器 (per v1: HashMap<String, serde_json::Value>)
#[derive(Debug, Clone, Default)]
pub struct State(pub HashMap<String, serde_json::Value>);

impl State {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn insert(&mut self, k: String, v: serde_json::Value) {
        self.0.insert(k, v);
    }
    pub fn get(&self, k: &str) -> Option<&serde_json::Value> {
        self.0.get(k)
    }
    pub fn contains(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }
}

/// Node output (run 完返)
#[derive(Debug, Clone)]
pub struct NodeOutput {
    pub node_id: NodeId,
}

impl NodeOutput {
    pub fn new(node_id: impl Into<NodeId>) -> Self {
        Self { node_id: node_id.into() }
    }
}

/// Graph error
#[derive(Debug, Error)]
pub enum GraphError {
    #[error("cycle detected in graph")]
    Cycle,
    #[error("node run failed: {0}")]
    NodeRun(String),
}

pub type Result<T> = std::result::Result<T, GraphError>;

/// Node trait — 同步执行 (per v1 设计, 0 引入 async LLM HTTP)
pub trait Node: Send + Sync {
    fn id(&self) -> NodeId;
    fn run(&self, state: &mut State) -> Result<NodeOutput>;
}

/// Edge
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

impl Edge {
    pub fn new(from: impl Into<NodeId>, to: impl Into<NodeId>) -> Self {
        Self { from: from.into(), to: to.into() }
    }
}

/// Graph — 节点 + 边容器, 拓扑排序执行
pub struct Graph {
    nodes: Vec<Box<dyn Node>>,
    edges: Vec<Edge>,
}

impl std::fmt::Debug for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graph")
            .field("node_count", &self.nodes.len())
            .field("edge_count", &self.edges.len())
            .finish()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new() }
    }

    pub fn add_node(&mut self, n: impl Node + 'static) {
        self.nodes.push(Box::new(n));
    }

    pub fn add_edge(&mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) {
        self.edges.push(Edge::new(from, to));
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn nodes(&self) -> &[Box<dyn Node>] {
        &self.nodes
    }

    /// 拓扑排序 (Kahn's algorithm), 检测 cycle
    fn topo_order(&self) -> Result<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for n in &self.nodes {
            in_degree.entry(n.id()).or_insert(0);
            adj.entry(n.id()).or_insert_with(Vec::new);
        }
        for e in &self.edges {
            *in_degree.entry(e.to.clone()).or_insert(0) += 1;
            adj.entry(e.from.clone()).or_insert_with(Vec::new).push(e.to.clone());
        }
        let mut queue: Vec<NodeId> = in_degree.iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        queue.sort_by(|a, b| a.0.cmp(&b.0)); // stable order
        let mut order = Vec::new();
        while let Some(n) = queue.pop() {
            order.push(n.clone());
            if let Some(succs) = adj.get(&n) {
                for s in succs {
                    if let Some(d) = in_degree.get_mut(s) {
                        *d -= 1;
                        if *d == 0 { queue.push(s.clone()); }
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(GraphError::Cycle);
        }
        Ok(order)
    }

    /// 异步执行 (per v1 Graph::execute().await) — 内部调 Node::run (sync)
    pub async fn execute(&self, mut state: State) -> Result<State> {
        let order = self.topo_order()?;
        // 节点 ID → 节点 映射
        let node_map: HashMap<NodeId, &Box<dyn Node>> = self.nodes.iter()
            .map(|n| (n.id(), n))
            .collect();
        for nid in order {
            if let Some(n) = node_map.get(&nid) {
                let output = n.run(&mut state).map_err(|e| GraphError::NodeRun(e.to_string()))?;
                state.insert(format!("node.ran.{}", output.node_id.as_str()), serde_json::json!(true));
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct IncNode { id: NodeId, key: String }
    impl Node for IncNode {
        fn id(&self) -> NodeId { self.id.clone() }
        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            let cur = state.get(&self.key).and_then(|v| v.as_i64()).unwrap_or(0);
            state.insert(self.key.clone(), serde_json::json!(cur + 1));
            Ok(NodeOutput::new(self.id.clone()))
        }
    }

    #[test]
    fn empty_graph_executes() {
        let g = Graph::new();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let r = rt.block_on(g.execute(State::new()));
        assert!(r.is_ok());
    }

    #[test]
    fn linear_chain_executes_topologically() {
        let mut g = Graph::new();
        g.add_node(IncNode { id: NodeId::new("a"), key: "x".into() });
        g.add_node(IncNode { id: NodeId::new("b"), key: "x".into() });
        g.add_edge("a", "b");
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let final_state = rt.block_on(g.execute(State::new())).unwrap();
        // a 跑完 x=1, b 跑完 x=2
        assert_eq!(final_state.get("x").unwrap().as_i64(), Some(2));
    }

    #[test]
    fn cycle_returns_error() {
        let mut g = Graph::new();
        g.add_node(IncNode { id: NodeId::new("a"), key: "x".into() });
        g.add_node(IncNode { id: NodeId::new("b"), key: "y".into() });
        g.add_edge("a", "b");
        g.add_edge("b", "a"); // cycle
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let r = rt.block_on(g.execute(State::new()));
        assert!(r.is_err());
    }
}
