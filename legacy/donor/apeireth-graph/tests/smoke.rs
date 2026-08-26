//! apeireth-graph integration smoke tests.

use apeireth_graph::{Graph, GraphError, Node, NodeId, NodeOutput, Result, State};
use serde_json::json;

struct SetNode {
    id: &'static str,
    key: &'static str,
    value: &'static str,
    fail: bool,
}

impl SetNode {
    fn new(id: &'static str, key: &'static str, value: &'static str) -> Self {
        Self {
            id,
            key,
            value,
            fail: false,
        }
    }

    fn failing(id: &'static str) -> Self {
        Self {
            id,
            key: "unused",
            value: "unused",
            fail: true,
        }
    }
}

impl Node for SetNode {
    fn id(&self) -> NodeId {
        self.id.to_owned()
    }

    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        if self.fail {
            return Err(GraphError::node("boom"));
        }
        state.insert(self.key, json!(self.value));
        Ok(NodeOutput::new(self.value))
    }
}

fn linear_graph() -> Graph {
    let mut graph = Graph::new();
    graph.try_add_node(SetNode::new("a", "k1", "v1")).unwrap();
    graph.try_add_node(SetNode::new("b", "k2", "v2")).unwrap();
    graph.try_add_node(SetNode::new("c", "k3", "v3")).unwrap();
    graph.try_add_edge("a", "b").unwrap();
    graph.try_add_edge("b", "c").unwrap();
    graph
}

#[tokio::test]
async fn smoke_three_node_linear_pipeline() {
    let final_state = linear_graph().execute(State::new()).await.unwrap();
    assert_eq!(final_state.execution_order, ["a", "b", "c"]);
    assert_eq!(final_state.get("k1"), Some(&json!("v1")));
    assert_eq!(final_state.get("k2"), Some(&json!("v2")));
    assert_eq!(final_state.get("k3"), Some(&json!("v3")));
}

#[tokio::test]
async fn smoke_checkpoint_writes_and_reads() {
    let graph = linear_graph();
    let final_state = graph.execute(State::new()).await.unwrap();
    let checkpoint = graph.checkpoint(&final_state.state).await.unwrap();
    let path = std::env::temp_dir().join(format!("{}-smoke.json", checkpoint.id));
    checkpoint.write_to(&path).await.unwrap();
    let restored = apeireth_graph::Checkpoint::read_from(&path).await.unwrap();
    tokio::fs::remove_file(path).await.unwrap();
    assert_eq!(restored.state, final_state.state);
}

#[tokio::test]
async fn smoke_node_failure_returns_node_id() {
    let mut graph = Graph::new();
    graph.add_node(SetNode::new("a", "k", "v"));
    graph.add_node(SetNode::failing("b"));
    graph.add_edge("a", "b");
    assert!(matches!(
        graph.execute(State::new()).await,
        Err(GraphError::NodeExecution { node_id, .. }) if node_id == "b"
    ));
}

#[tokio::test]
async fn smoke_cycle_is_rejected() {
    let mut graph = linear_graph();
    graph.add_edge("c", "a");
    assert!(matches!(
        graph.execute(State::new()).await,
        Err(GraphError::Cycle { .. })
    ));
}

#[test]
fn smoke_duplicate_node_is_rejected_by_checked_api() {
    let mut graph = Graph::new();
    graph.try_add_node(SetNode::new("dup", "a", "1")).unwrap();
    assert!(matches!(
        graph.try_add_node(SetNode::new("dup", "b", "2")),
        Err(GraphError::DuplicateNode(id)) if id == "dup"
    ));
}

#[tokio::test]
async fn smoke_single_node_graph_runs() {
    let mut graph = Graph::new();
    graph.add_node(SetNode::new("only", "x", "y"));
    let final_state = graph.execute(State::new()).await.unwrap();
    assert_eq!(final_state.execution_order, ["only"]);
}

#[tokio::test]
async fn smoke_diamond_dependencies_are_respected() {
    let mut graph = Graph::new();
    for (id, key) in [("a", "ka"), ("b", "kb"), ("c", "kc"), ("d", "kd")] {
        graph.add_node(SetNode::new(id, key, "value"));
    }
    graph.add_edge("a", "b");
    graph.add_edge("a", "c");
    graph.add_edge("b", "d");
    graph.add_edge("c", "d");
    let order = graph.execute(State::new()).await.unwrap().execution_order;
    let position = |id: &str| order.iter().position(|node| node == id).unwrap();
    assert!(position("a") < position("b"));
    assert!(position("a") < position("c"));
    assert!(position("b") < position("d"));
    assert!(position("c") < position("d"));
}

#[tokio::test]
async fn smoke_initial_state_is_preserved() {
    let mut graph = Graph::new();
    graph.add_node(SetNode::new("node", "new", "value"));
    let mut state = State::new();
    state.insert("seed", json!("from-caller"));
    let final_state = graph.execute(state).await.unwrap();
    assert_eq!(final_state.get("seed"), Some(&json!("from-caller")));
    assert_eq!(final_state.get("new"), Some(&json!("value")));
}
