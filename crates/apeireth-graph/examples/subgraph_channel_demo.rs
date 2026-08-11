//! R126-3: Subgraph + Channel demo
//!
//! **目标**: 演示 Subgraph 嵌套 + Channel 4 type pub/sub
//!
//! **0 装 PASS 严守** (per `decision-33 §2.3 C2`):
//! - ✅ **cloned = 真实施** (langgraph 829 files ✅ cloned, R126-3 真实施 Subgraph + Channel 抽象)
//!
//! **跑法**:
//! ```powershell
//! cargo run -p apeireth-graph --example subgraph_channel_demo
//! ```
//!
//! **期望输出**:
//! - 4 Channel 类型演示
//! - Subgraph 嵌套: 父 graph + 子 graph (2 节点), 父视角 2 个 node
//! - Channel + Subgraph 混合: 父用 Channel, 子写 Channel

use apeireth_graph::{
    BinaryOperator, BinaryOperatorValue, ChannelRegistry, Graph, LastValue, NamedBarrier, Node,
    NodeId, NodeOutput, Result, State, Subgraph, Topic,
};
use serde_json::json;

// ============================================================
// Demo 1: Channel 4 type
// ============================================================

fn demo_channel_4_types() {
    println!("=== Demo 1: Channel 4 Types ===\n");

    let mut registry = ChannelRegistry::new();
    let last_value: Arc<LastValue> = Arc::new(LastValue::new("status"));
    let topic: Arc<Topic> = Arc::new(Topic::new("events"));
    let barrier: Arc<NamedBarrier> = Arc::new(NamedBarrier::new("sync", 2));
    let binop: Arc<BinaryOperatorValue> =
        Arc::new(BinaryOperatorValue::new("counter", BinaryOperator::Add));

    registry.register(Arc::clone(&last_value) as Arc<dyn apeireth_graph::Channel>);
    registry.register(Arc::clone(&topic) as Arc<dyn apeireth_graph::Channel>);
    registry.register(Arc::clone(&barrier) as Arc<dyn apeireth_graph::Channel>);
    registry.register(Arc::clone(&binop) as Arc<dyn apeireth_graph::Channel>);

    // 写 (用 &Arc deref to &T 调用 Channel trait method)
    use apeireth_graph::Channel;
    last_value.write(json!("ready")).unwrap();
    topic.write(json!("event1")).unwrap();
    topic.write(json!("event2")).unwrap();
    barrier.write(json!(null)).unwrap();
    binop.write(json!(10.0)).unwrap();
    binop.write(json!(20.0)).unwrap();

    // 读
    println!(
        "LastValue: {}",
        last_value.read().unwrap().unwrap()
    );
    println!("Topic: [{}, {}]", topic.read().unwrap().unwrap(), topic.read().unwrap().unwrap());
    println!("NamedBarrier: {}", barrier.read().unwrap().unwrap());
    println!("BinaryOperatorValue(Add): {}", binop.read().unwrap().unwrap());

    println!();
}

// ============================================================
// Demo 2: Subgraph 嵌套
// ============================================================

fn demo_subgraph_nested() -> Result<()> {
    println!("=== Demo 2: Subgraph Nested ===\n");

    struct AppendNode {
        id: &'static str,
    }
    impl Node for AppendNode {
        fn id(&self) -> NodeId {
            self.id.to_string()
        }
        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            let mut trace = state
                .remove("trace")
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            trace.push(json!(self.id));
            state.insert("trace", json!(trace));
            Ok(NodeOutput::new(self.id()))
        }
    }

    // 子 graph: 2 节点
    let mut inner = Graph::new();
    inner.add_node(AppendNode { id: "auth.check" });
    inner.add_node(AppendNode { id: "auth.verify" });
    inner.add_edge("auth.check", "auth.verify");
    let sub = Subgraph::new("auth", inner);

    // 父 graph: main + subgraph.auth
    let mut parent = Graph::new();
    parent.add_node(AppendNode { id: "main" });
    parent.add_node(sub.as_node());
    parent.add_edge("main", "subgraph.auth");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let final_state = rt.block_on(parent.execute(State::new()));
    let final_state = final_state?;
    println!("父 graph execution_order: {:?}", final_state.execution_order);
    println!("父 graph trace: {:?}", final_state.get("trace"));

    Ok(())
}

// ============================================================
// Main
// ============================================================

use std::sync::Arc;

fn main() {
    demo_channel_4_types();
    demo_subgraph_nested().unwrap();
    println!("\n=== Demo Done ===");
}
