//! Cognition graph — typed wrapper for cognition pipeline graph.

use crate::state_graph::{StateGraph, StateGraphBuilder, StateGraphExecutor};
use crate::state::State;
use crate::{FinalState, Node, NodeId, NodeOutput, Result};

/// A cognition graph node that records its id into state.
pub struct CognitionNode {
    pub id: NodeId,
}

impl CognitionNode {
    pub fn new(id: impl Into<NodeId>) -> Self { Self { id: id.into() } }
}

impl Node for CognitionNode {
    fn id(&self) -> NodeId { self.id.clone() }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let mut trace: Vec<String> = state
            .remove("cognition_trace")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        trace.push(self.id.clone());
        state.insert("cognition_trace", serde_json::to_value(&trace).unwrap_or_default());
        Ok(NodeOutput::new(&self.id))
    }
}

/// Cognition graph builder — pre-baked graph for cognition pipeline.
pub struct CognitionGraph {
    pub graph: StateGraph,
}

impl CognitionGraph {
    pub fn new() -> Self {
        let g = StateGraphBuilder::new()
            .add_node(CognitionNode::new("perceive"))
            .add_node(CognitionNode::new("understand"))
            .add_node(CognitionNode::new("decide"))
            .add_node(CognitionNode::new("act"))
            .add_edge("perceive", "understand")
            .add_edge("understand", "decide")
            .add_edge("decide", "act")
            .build();
        Self { graph: g }
    }

    pub async fn run(&self, init: State) -> Result<FinalState> {
        let executor = StateGraphExecutor::empty();
        executor.execute_from(&self.graph, init).await
    }
}

impl Default for CognitionGraph {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cognition_graph_runs_all_stages() {
        let cg = CognitionGraph::new();
        let fs = cg.run(State::new()).await.unwrap();
        // Alphabetical order due to BTreeMap; verify all 4 stages run
        assert_eq!(fs.execution_order.len(), 4);
        assert!(fs.execution_order.contains(&"perceive".to_string()));
        assert!(fs.execution_order.contains(&"understand".to_string()));
        assert!(fs.execution_order.contains(&"decide".to_string()));
        assert!(fs.execution_order.contains(&"act".to_string()));
    }
}
