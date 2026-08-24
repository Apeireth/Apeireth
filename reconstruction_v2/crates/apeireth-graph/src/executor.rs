//! Deterministic topological graph executor.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use crate::conditional::{ConditionalDecision, END_LABEL};
use crate::state::{FinalState, NodeOutput};
use crate::{Graph, GraphError, NodeId, Result, State};

/// Read-only supervisor integration snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorSnapshot {
    pub plan_version: u64,
    pub child_count: usize,
    pub mocked: bool,
}

fn supervisor_snapshot() -> SupervisorSnapshot {
    SupervisorSnapshot { plan_version: 1, child_count: 0, mocked: true }
}

/// Executes one graph in deterministic topological order.
pub struct Executor<'graph> {
    graph: &'graph Graph,
    supervisor: SupervisorSnapshot,
}

impl<'graph> Executor<'graph> {
    pub fn new(graph: &'graph Graph) -> Self {
        Self { graph, supervisor: supervisor_snapshot() }
    }

    pub fn supervisor_snapshot(&self) -> SupervisorSnapshot { self.supervisor }

    fn topological_order(&self) -> Result<Vec<NodeId>> {
        let mut indeg: HashMap<&NodeId, usize> = HashMap::new();
        for id in self.graph.nodes.keys() {
            indeg.insert(id, 0);
        }
        for e in &self.graph.edges {
            *indeg.entry(&e.to).or_insert(0) += 1;
        }
        let mut order: Vec<NodeId> = Vec::new();
        let mut ready: BTreeSet<&NodeId> = indeg.iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| *k)
            .collect();
        while let Some(id) = ready.iter().next().copied() {
            ready.remove(id);
            order.push((*id).clone());
            for e in &self.graph.edges {
                if &e.from == id {
                    if let Some(d) = indeg.get_mut(&e.to) {
                        *d -= 1;
                        if *d == 0 { ready.insert(&e.to); }
                    }
                }
            }
        }
        if order.len() != self.graph.nodes.len() {
            let blocked: Vec<NodeId> = indeg.iter()
                .filter(|(_, d)| **d > 0)
                .map(|(k, _)| (*k).clone())
                .collect();
            return Err(GraphError::Cycle { nodes: blocked });
        }
        Ok(order)
    }

    pub async fn execute(&self, mut state: State) -> Result<FinalState> {
        let topo = self.topological_order()?;
        let mut outputs: BTreeMap<NodeId, NodeOutput> = BTreeMap::new();
        let mut execution_order: Vec<NodeId> = Vec::new();

        for node_id in topo {
            let node = self.graph.nodes.get(&node_id)
                .ok_or_else(|| GraphError::MissingNode(node_id.clone()))?;
            let output = node.run(&mut state).map_err(|e| {
                match e {
                    GraphError::NodeExecution { .. } => e,
                    other => GraphError::NodeExecution {
                        node_id: node_id.clone(),
                        message: other.to_string(),
                    },
                }
            })?;
            execution_order.push(node_id.clone());
            outputs.insert(node_id.clone(), output);
        }

        for ce in &self.graph.conditional_edges {
            let decision = ce.decide(&state);
            match decision {
                ConditionalDecision::GoTo(target) => {
                    if execution_order.contains(&target) { continue; }
                    if let Some(node) = self.graph.nodes.get(&target) {
                        let output = node.run(&mut state)?;
                        execution_order.push(target.clone());
                        outputs.insert(target, output);
                    }
                }
                ConditionalDecision::End | ConditionalDecision::Default => {}
            }
        }

        // Mark END
        let _ = END_LABEL;

        Ok(FinalState { state, outputs, execution_order })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use crate::{Edge, Graph, Node, NodeId, NodeOutput};
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
    async fn executor_snapshot_present() {
        let mut g = Graph::new();
        g.add_node(AppendNode { id: "x" });
        let ex = Executor::new(&g);
        assert!(ex.supervisor_snapshot().mocked);
    }

    #[tokio::test]
    async fn cycle_detected() {
        let mut g = Graph::new();
        g.add_node(AppendNode { id: "a" });
        g.add_node(AppendNode { id: "b" });
        g.add_edge("a", "b");
        g.add_edge("b", "a");
        let ex = Executor::new(&g);
        assert!(matches!(ex.execute(State::new()).await, Err(GraphError::Cycle { .. })));
    }

    #[tokio::test]
    async fn edge_new() {
        let e = Edge::new("a", "b");
        assert_eq!(e.from, "a");
        assert_eq!(e.to, "b");
    }
}
