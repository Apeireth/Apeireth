//! Apeireth graph orchestration skeleton.
//!
//! Graphs execute synchronous nodes in deterministic topological order and
//! expose versioned JSON checkpoints for recovery.

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use thiserror::Error;

pub mod checkpoint;
pub mod conditional;
pub mod executor;
pub mod mcp_resource;  // R89: CognitionGraph → MCP ResourceServer (graph state 暴露为 MCP resources)
pub mod state;

pub use checkpoint::{Checkpoint, CheckpointStore};
pub use conditional::{ConditionalDecision, ConditionalEdge, ConditionalError, END_LABEL};
pub use executor::{Executor, SupervisorSnapshot};
pub use state::{FinalState, NodeOutput, State};

/// Stable identifier for a graph node.
pub type NodeId = String;

/// Crate result type.
pub type Result<T> = std::result::Result<T, GraphError>;

/// Graph construction, execution, or persistence error.
#[derive(Debug, Error)]
pub enum GraphError {
    /// An edge or executor lookup references an unknown node.
    #[error("graph references missing node `{0}`")]
    MissingNode(NodeId),
    /// Inserting a duplicate node through the checked API is rejected.
    #[error("node `{0}` already exists")]
    DuplicateNode(NodeId),
    /// The graph contains a directed cycle.
    #[error("graph contains a cycle involving nodes: {nodes:?}")]
    Cycle {
        /// Nodes still blocked after topological sorting.
        nodes: Vec<NodeId>,
    },
    /// A node returned an error.
    #[error("node `{node_id}` failed: {message}")]
    NodeExecution {
        /// Failing node.
        node_id: NodeId,
        /// Original error text.
        message: String,
    },
    /// System time was earlier than the Unix epoch.
    #[error("cannot create checkpoint timestamp: {0}")]
    Clock(String),
    /// The checkpoint schema is newer or otherwise unsupported.
    #[error("unsupported checkpoint version {0}")]
    UnsupportedCheckpointVersion(u32),
    /// A checkpoint ID was unsafe for directory-backed lookup.
    #[error("invalid checkpoint id `{0}`")]
    InvalidCheckpointId(String),
    /// Checkpoint filesystem failure.
    #[error("checkpoint I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Checkpoint serialization failure.
    #[error("checkpoint JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A node-specific validation or application failure.
    #[error("node error: {0}")]
    Node(String),
}

impl GraphError {
    /// Creates a node-specific error without exposing an error dependency to callers.
    pub fn node(message: impl Into<String>) -> Self {
        Self::Node(message.into())
    }
}

/// A directed connection between two graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    /// Source node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
}

impl Edge {
    /// Creates a directed edge.
    pub fn new(from: impl Into<NodeId>, to: impl Into<NodeId>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// One executable unit in a graph.
pub trait Node: Send + Sync {
    /// Returns this node's stable unique ID.
    fn id(&self) -> NodeId;

    /// Runs the node and may update shared state.
    fn run(&self, state: &mut State) -> Result<NodeOutput>;
}

/// A deterministic directed graph of executable nodes.
pub struct Graph {
    pub(crate) nodes: BTreeMap<NodeId, Box<dyn Node>>,
    pub(crate) edges: Vec<Edge>,
    /// R33-5: conditional edges (per LangGraph dd_conditional_edges 借鉴)
    pub(crate) conditional_edges: Vec<conditional::ConditionalEdge>,
}

impl Graph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node, replacing a node with the same ID.
    ///
    /// This preserves the specification's infallible API. Use
    /// [`Graph::try_add_node`] when replacement would be unsafe.
    pub fn add_node(&mut self, node: impl Node + 'static) {
        self.nodes.insert(node.id(), Box::new(node));
    }

    /// Adds a node and rejects duplicate IDs.
    pub fn try_add_node(&mut self, node: impl Node + 'static) -> Result<()> {
        let node_id = node.id();
        if self.nodes.contains_key(&node_id) {
            return Err(GraphError::DuplicateNode(node_id));
        }
        self.nodes.insert(node_id, Box::new(node));
        Ok(())
    }

    /// Adds a directed edge. Endpoints are validated before execution.
    pub fn add_edge(&mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) {
        self.edges.push(Edge::new(from, to));
    }

    /// Adds a directed edge after validating both endpoints.
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

    /// Returns the number of registered nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns graph edges in insertion order.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// R33-5: 添加 conditional edge (per LangGraph dd_conditional_edges 借鉴)
    pub fn add_conditional_edge(
        &mut self,
        from: impl Into<NodeId>,
        path_map: std::collections::BTreeMap<String, NodeId>,
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

    /// R33-5: 返回 conditional edges 列表 (for reporting / tests).
    pub fn conditional_edges(&self) -> &[conditional::ConditionalEdge] {
        &self.conditional_edges
    }

    /// Runs the graph in deterministic topological order.
    pub async fn execute(&self, init_state: State) -> Result<FinalState> {
        Executor::new(self).execute(init_state).await
    }

    /// Captures current state and graph node IDs in a versioned checkpoint.
    pub async fn checkpoint(&self, state: &State) -> Result<Checkpoint> {
        Checkpoint::new(self.nodes.keys().cloned().collect(), state.clone())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            conditional_edges: Vec::new(),
        }
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
    use serde_json::json;

    use super::*;

    struct AppendNode {
        id: &'static str,
    }

    impl Node for AppendNode {
        fn id(&self) -> NodeId {
            self.id.to_owned()
        }

        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            let mut trace = state
                .remove("trace")
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default();
            trace.push(json!(self.id));
            state.insert("trace", json!(trace));
            Ok(NodeOutput::new(self.id))
        }
    }

    fn linear_graph() -> Graph {
        let mut graph = Graph::new();
        for id in ["one", "two", "three"] {
            graph.add_node(AppendNode { id });
        }
        graph.add_edge("one", "two");
        graph.add_edge("two", "three");
        graph
    }

    #[tokio::test]
    async fn executes_linear_graph_in_order() {
        let final_state = linear_graph().execute(State::new()).await.unwrap();
        assert_eq!(final_state.execution_order, ["one", "two", "three"]);
        assert_eq!(
            final_state.get("trace"),
            Some(&json!(["one", "two", "three"]))
        );
    }

    #[tokio::test]
    async fn rejects_cycles_without_running_nodes() {
        let mut graph = linear_graph();
        graph.add_edge("three", "one");
        assert!(matches!(
            graph.execute(State::new()).await,
            Err(GraphError::Cycle { .. })
        ));
    }

    #[tokio::test]
    async fn checkpoint_round_trip_preserves_state() {
        let graph = linear_graph();
        let final_state = graph.execute(State::new()).await.unwrap();
        let checkpoint = graph.checkpoint(&final_state.state).await.unwrap();
        let path = std::env::temp_dir().join(format!("{}.json", checkpoint.id));
        checkpoint.write_to(&path).await.unwrap();
        let restored = Checkpoint::read_from(&path).await.unwrap();
        tokio::fs::remove_file(path).await.unwrap();
        assert_eq!(restored.state, final_state.state);
    }
}

pub mod cognition_graph;

