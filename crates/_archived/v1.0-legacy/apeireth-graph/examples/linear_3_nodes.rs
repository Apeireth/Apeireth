//! Runs a three-node linear graph and persists its final state.
//!
//! `cargo run -p apeireth-graph --example linear_3_nodes`

use apeireth_graph::{Graph, Node, NodeId, NodeOutput, Result, State};
use serde_json::json;

struct StepNode {
    id: &'static str,
}

impl Node for StepNode {
    fn id(&self) -> NodeId {
        self.id.to_owned()
    }

    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let mut visited = state
            .remove("visited")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        visited.push(json!(self.id));
        state.insert("visited", json!(visited));
        println!("executed node {}", self.id);
        Ok(NodeOutput::new(self.id))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut graph = Graph::new();
    for id in ["fetch", "transform", "store"] {
        graph.try_add_node(StepNode { id })?;
    }
    graph.try_add_edge("fetch", "transform")?;
    graph.try_add_edge("transform", "store")?;

    let final_state = graph.execute(State::new()).await?;
    assert_eq!(
        final_state.get("visited"),
        Some(&json!(["fetch", "transform", "store"]))
    );

    let checkpoint = graph.checkpoint(&final_state.state).await?;
    let path = std::env::temp_dir().join(format!("{}.json", checkpoint.id));
    checkpoint.write_to(&path).await?;
    println!("checkpoint written: {}", path.display());

    // The example proves the file is complete and readable, then avoids leaving
    // build-time artifacts in the repository.
    let restored = apeireth_graph::Checkpoint::read_from(&path).await?;
    assert_eq!(restored.state, final_state.state);
    tokio::fs::remove_file(path).await?;

    println!("3-node linear graph completed successfully");
    Ok(())
}
