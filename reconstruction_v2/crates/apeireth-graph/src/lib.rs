//! Apeireth v2 graph orchestration — LangGraph-style deterministic graphs.
//!
//! v1 API surface preserved: `Graph`, `Node`, `Edge`, `State`, `FinalState`,
//! `NodeOutput`, `Executor`, `Checkpoint`, `CheckpointStore`, `ConditionalEdge`,
//! `ConditionalDecision`, `END_LABEL`, `Subgraph`, `Channel*`, `StateGraph*`,
//! `ContextGraph`, `ContextNode`, `ContextStore`, `ThreadHistory`.

#![deny(unsafe_code)]

pub mod channel;
pub mod checkpoint;
pub mod cognition_graph;
pub mod conditional;
pub mod context_graph;
pub mod executor;
pub mod mcp_resource;
pub mod state;
pub mod state_graph;
pub mod subgraph;
pub mod thread_history;

pub use channel::{
    BinaryOperator, BinaryOperatorValue, Channel, ChannelError, ChannelRegistry, ChannelType,
    LastValue, NamedBarrier, Topic,
};
pub use checkpoint::{Checkpoint, CheckpointStore};
pub use conditional::{
    ConditionalDecision, ConditionalEdge, ConditionalError, END_LABEL,
};
pub use context_graph::{
    ContextError, ContextGraph, ContextNode, ContextPhase, ContextSnapshot, ContextStore,
    InMemoryContextStore, CONTEXT_PHASE_COUNT,
};
pub use executor::{Executor, SupervisorSnapshot};
pub use state::{FinalState, NodeOutput, State};
pub use state_graph::{
    StateGraph, StateGraphBuilder, StateGraphConditionalEdge, StateGraphEdge, StateGraphExecutor,
};
pub use subgraph::Subgraph;
pub use thread_history::{ThreadCheckpointStore, ThreadHistory};

use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Stable identifier for a graph node.
pub type NodeId = String;

/// Crate result type.
pub type Result<T> = std::result::Result<T, GraphError>;

/// Graph construction, execution, or persistence error.
#[derive(Debug, Error)]
pub enum GraphError {
    #[error("graph references missing node `{0}`")]
    MissingNode(NodeId),
    #[error("node `{0}` already exists")]
    DuplicateNode(NodeId),
    #[error("graph contains a cycle involving nodes: {nodes:?}")]
    Cycle { nodes: Vec<NodeId> },
    #[error("node `{node_id}` failed: {message}")]
    NodeExecution { node_id: NodeId, message: String },
    #[error("cannot create checkpoint timestamp: {0}")]
    Clock(String),
    #[error("unsupported checkpoint version {0}")]
    UnsupportedCheckpointVersion(u32),
    #[error("invalid checkpoint id `{0}`")]
    InvalidCheckpointId(String),
    #[error("checkpoint I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("checkpoint JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("node error: {0}")]
    Node(String),
}

impl GraphError {
    pub fn node(message: impl Into<String>) -> Self {
        Self::Node(message.into())
    }
}

/// A directed connection between two graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

impl Edge {
    pub fn new(from: impl Into<NodeId>, to: impl Into<NodeId>) -> Self {
        Self { from: from.into(), to: to.into() }
    }
}

/// One executable unit in a graph.
pub trait Node: Send + Sync {
    fn id(&self) -> NodeId;
    fn run(&self, state: &mut State) -> Result<NodeOutput>;
}

/// A deterministic directed graph of executable nodes.
pub struct Graph {
    pub(crate) nodes: BTreeMap<NodeId, Box<dyn Node>>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) conditional_edges: Vec<conditional::ConditionalEdge>,
}

impl Graph {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self, node: impl Node + 'static) {
        self.nodes.insert(node.id(), Box::new(node));
    }

    pub fn try_add_node(&mut self, node: impl Node + 'static) -> Result<()> {
        let node_id = node.id();
        if self.nodes.contains_key(&node_id) {
            return Err(GraphError::DuplicateNode(node_id));
        }
        self.nodes.insert(node_id, Box::new(node));
        Ok(())
    }

    pub fn add_edge(&mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) {
        self.edges.push(Edge::new(from, to));
    }

    pub fn try_add_edge(&mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) -> Result<()> {
        let edge = Edge::new(from, to);
        if !self.nodes.contains_key(&edge.from) {
            return Err(GraphError::MissingNode(edge.from));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(GraphError::MissingNode(edge.to));
        }
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
        Ok(())
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edges(&self) -> &[Edge] { &self.edges }

    pub fn add_conditional_edge(
        &mut self,
        from: impl Into<NodeId>,
        path_map: BTreeMap<String, NodeId>,
        default: Option<NodeId>,
        condition: std::sync::Arc<dyn Fn(&State) -> String + Send + Sync>,
    ) {
        self.conditional_edges.push(conditional::ConditionalEdge {
            from: from.into(),
            path_map,
            default,
            condition,
        });
    }

    pub fn conditional_edges(&self) -> &[conditional::ConditionalEdge] {
        &self.conditional_edges
    }

    pub async fn execute(&self, init_state: State) -> Result<FinalState> {
        Executor::new(self).execute(init_state).await
    }

    pub async fn checkpoint(&self, state: &State) -> Result<Checkpoint> {
        Checkpoint::new(self.nodes.keys().cloned().collect(), state.clone())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self { nodes: BTreeMap::new(), edges: Vec::new(), conditional_edges: Vec::new() }
    }
}

impl fmt::Debug for Graph {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Graph")
            .field("nodes", &self.nodes.keys().collect::<Vec<_>>())
            .field("edges", &self.edges)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct AppendNode { id: &'static str }
    impl Node for AppendNode {
        fn id(&self) -> NodeId { self.id.to_owned() }
        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            let mut trace: Vec<serde_json::Value> = state
                .remove("trace")
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            trace.push(json!(self.id));
            state.insert("trace", json!(trace));
            Ok(NodeOutput::new(self.id))
        }
    }

    fn linear() -> Graph {
        let mut g = Graph::new();
        for id in ["one", "two", "three"] {
            g.add_node(AppendNode { id });
        }
        g.add_edge("one", "two");
        g.add_edge("two", "three");
        g
    }

    #[tokio::test]
    async fn executes_linear_in_order() {
        let final_state = linear().execute(State::new()).await.unwrap();
        assert_eq!(final_state.execution_order, ["one", "two", "three"]);
        assert_eq!(final_state.get("trace"), Some(&json!(["one", "two", "three"])));
    }

    #[tokio::test]
    async fn rejects_cycles() {
        let mut g = linear();
        g.add_edge("three", "one");
        assert!(matches!(g.execute(State::new()).await, Err(GraphError::Cycle { .. })));
    }

    #[tokio::test]
    async fn duplicate_node_rejected() {
        let mut g = Graph::new();
        g.add_node(AppendNode { id: "x" });
        let err = g.try_add_node(AppendNode { id: "x" });
        assert!(matches!(err, Err(GraphError::DuplicateNode(_))));
    }

    #[tokio::test]
    async fn conditional_edge_routes() {
        let mut g = Graph::new();
        g.add_node(AppendNode { id: "src" });
        g.add_node(AppendNode { id: "left" });
        g.add_node(AppendNode { id: "right" });
        g.add_edge("src", "left");
        let mut path_map = BTreeMap::new();
        path_map.insert("left".into(), "left".to_string());
        path_map.insert("right".into(), "right".to_string());
        let condition: std::sync::Arc<dyn Fn(&State) -> String + Send + Sync> =
            std::sync::Arc::new(|_s| "right".to_string());
        g.add_conditional_edge("src", path_map, None, condition);
        let final_state = g.execute(State::new()).await.unwrap();
        assert!(final_state.execution_order.contains(&"right".to_string()));
    }
}
