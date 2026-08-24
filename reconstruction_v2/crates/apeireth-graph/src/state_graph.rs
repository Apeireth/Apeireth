//! StateGraph — LangGraph-style state graph builder.

use std::collections::BTreeMap;
use std::sync::Arc;
use crate::conditional::{ConditionalDecision, ConditionalEdge};
use crate::state::State;
use crate::{FinalState, Graph, GraphError, Node, NodeId, NodeOutput, Result};

/// StateGraph edge — typed edge in a state graph.
#[derive(Debug, Clone)]
pub struct StateGraphEdge {
    pub from: NodeId,
    pub to: NodeId,
}

/// Conditional edge in state graph (typed wrapper).
#[derive(Clone)]
pub struct StateGraphConditionalEdge {
    pub inner: ConditionalEdge,
}

impl std::fmt::Debug for StateGraphConditionalEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateGraphConditionalEdge")
            .field("from", &self.inner.from)
            .field("path_map", &self.inner.path_map)
            .finish()
    }
}

/// StateGraph — typed state graph with builder pattern.
pub struct StateGraph {
    pub nodes: BTreeMap<NodeId, Arc<dyn Node>>,
    pub edges: Vec<StateGraphEdge>,
    pub conditional_edges: Vec<StateGraphConditionalEdge>,
}

/// Builder for StateGraph.
#[derive(Clone)]
pub struct StateGraphBuilder {
    nodes: BTreeMap<NodeId, Arc<dyn Node>>,
    edges: Vec<StateGraphEdge>,
    conditional_edges: Vec<StateGraphConditionalEdge>,
}

impl StateGraphBuilder {
    pub fn new() -> Self {
        Self { nodes: BTreeMap::new(), edges: Vec::new(), conditional_edges: Vec::new() }
    }

    pub fn add_node(mut self, node: impl Node + 'static) -> Self {
        self.nodes.insert(node.id(), Arc::new(node));
        self
    }

    pub fn add_edge(mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) -> Self {
        self.edges.push(StateGraphEdge { from: from.into(), to: to.into() });
        self
    }

    pub fn add_conditional_edge(
        mut self,
        from: impl Into<NodeId>,
        path_map: BTreeMap<String, NodeId>,
        default: Option<NodeId>,
        condition: Arc<dyn Fn(&State) -> String + Send + Sync>,
    ) -> Self {
        self.conditional_edges.push(StateGraphConditionalEdge {
            inner: ConditionalEdge { from: from.into(), path_map, default, condition },
        });
        self
    }

    pub fn build(self) -> StateGraph {
        StateGraph {
            nodes: self.nodes,
            edges: self.edges,
            conditional_edges: self.conditional_edges,
        }
    }
}

impl Default for StateGraphBuilder {
    fn default() -> Self { Self::new() }
}

/// Executes a StateGraph.
pub struct StateGraphExecutor {
    graph: Option<StateGraph>,
}

impl StateGraphExecutor {
    pub fn new(graph: StateGraph) -> Self { Self { graph: Some(graph) } }

    pub fn empty() -> Self { Self { graph: None } }

    pub async fn execute(&self, init: State) -> Result<FinalState> {
        let g = self.graph.as_ref().expect("StateGraph required");
        self.execute_from(g, init).await
    }

    pub async fn execute_from(&self, graph: &StateGraph, init: State) -> Result<FinalState> {
        let mut state = init;
        let mut outputs = BTreeMap::new();
        let mut order = Vec::new();
        let mut keys: Vec<NodeId> = graph.nodes.keys().cloned().collect();
        keys.sort();
        for nid in keys {
            let node = graph.nodes.get(&nid).expect("node present");
            let out = node.run(&mut state).map_err(|e| match e {
                GraphError::NodeExecution { .. } => e,
                other => GraphError::NodeExecution { node_id: nid.clone(), message: other.to_string() },
            })?;
            order.push(nid.clone());
            outputs.insert(nid, out);
        }
        for ce in &graph.conditional_edges {
            match ce.inner.decide(&state) {
                ConditionalDecision::GoTo(target) => {
                    if order.contains(&target) { continue; }
                    if let Some(node) = graph.nodes.get(&target) {
                        let out = node.run(&mut state)?;
                        order.push(target.clone());
                        outputs.insert(target, out);
                    }
                }
                _ => {}
            }
        }
        Ok(FinalState { state, outputs, execution_order: order })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
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

    #[tokio::test]
    async fn state_graph_builder_basic() {
        let g = StateGraphBuilder::new()
            .add_node(AppendNode { id: "a" })
            .add_node(AppendNode { id: "b" })
            .add_edge("a", "b")
            .build();
        let ex = StateGraphExecutor::new(g);
        let final_state = ex.execute(State::new()).await.unwrap();
        assert_eq!(final_state.execution_order.len(), 2);
    }

    #[test]
    fn builder_default() {
        let _ = StateGraphBuilder::default();
    }
}
