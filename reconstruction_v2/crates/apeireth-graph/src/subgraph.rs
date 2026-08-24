//! Subgraph — namespace-prefixed sub-Graph embedding (LangGraph-style).

use std::fmt;
use std::sync::Arc;
use crate::{FinalState, Graph, GraphError, Node, NodeId, NodeOutput, Result, State};

/// A sub-Graph with namespace prefix.
pub struct Subgraph {
    pub namespace: String,
    pub graph: Graph,
}

impl Subgraph {
    pub fn new(namespace: impl Into<String>, graph: Graph) -> Self {
        Self { namespace: namespace.into(), graph }
    }

    /// Wrap this Subgraph as a single Node in a parent graph.
    pub fn as_node(self) -> SubgraphNode {
        SubgraphNode { subgraph: Arc::new(self) }
    }

    fn prefixed(&self, id: &str) -> String {
        format!("{}__{}", self.namespace, id)
    }

    pub async fn run_subgraph(&self, mut state: State) -> Result<FinalState> {
        // Run inner graph with prefixed node IDs into state keys
        let inner_state = State::new();
        let mut final_inner = self.graph.execute(inner_state).await?;
        // Copy inner results to outer state under prefix
        for k in final_inner.state.keys() {
            if let Some(v) = final_inner.state.get(k).cloned() {
                state.insert(self.prefixed(k), v);
            }
        }
        final_inner.state = state;
        Ok(final_inner)
    }
}

impl fmt::Debug for Subgraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subgraph")
            .field("namespace", &self.namespace)
            .field("node_count", &self.graph.node_count())
            .finish()
    }
}

/// A Node that runs a Subgraph.
pub struct SubgraphNode {
    subgraph: Arc<Subgraph>,
}

impl Node for SubgraphNode {
    fn id(&self) -> NodeId { self.subgraph.namespace.clone() }

    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let ns = self.subgraph.namespace.clone();
        let inner = self.subgraph.clone();
        let state_clone = state.clone();
        // Sync entry: spawn a new runtime to run async subgraph
        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(move || {
                handle.block_on(async move { inner.run_subgraph(state_clone).await })
            })
        } else {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| GraphError::Node(format!("runtime init failed: {e}")))?;
            rt.block_on(async move { inner.run_subgraph(state_clone).await })
        };
        match result {
            Ok(final_state) => {
                *state = final_state.state.clone();
                Ok(NodeOutput::new(ns))
            }
            Err(e) => Err(GraphError::Node(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use serde_json::json;

    struct IdNode { id: &'static str, v: i64 }
    impl Node for IdNode {
        fn id(&self) -> NodeId { self.id.to_owned() }
        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            state.insert(self.id, json!(self.v));
            Ok(NodeOutput::new(self.id))
        }
    }

    #[tokio::test]
    async fn subgraph_run_with_prefix() {
        let mut inner = Graph::new();
        inner.add_node(IdNode { id: "a", v: 1 });
        inner.add_node(IdNode { id: "b", v: 2 });
        let sub = Subgraph::new("ns", inner);
        let final_state = sub.run_subgraph(State::new()).await.unwrap();
        assert_eq!(final_state.state.get("ns__a"), Some(&json!(1)));
        assert_eq!(final_state.state.get("ns__b"), Some(&json!(2)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subgraph_as_node_runs() {
        let mut inner = Graph::new();
        inner.add_node(IdNode { id: "only", v: 42 });
        let sub = Subgraph::new("sub", inner).as_node();
        let mut parent = Graph::new();
        parent.add_node(sub);
        let final_state = parent.execute(State::new()).await.unwrap();
        assert_eq!(final_state.state.get("sub__only"), Some(&json!(42)));
    }

    #[test]
    fn subgraph_debug() {
        let sub = Subgraph::new("x", Graph::new());
        let s = format!("{:?}", sub);
        assert!(s.contains("Subgraph"));
        assert!(s.contains("x"));
    }
}
