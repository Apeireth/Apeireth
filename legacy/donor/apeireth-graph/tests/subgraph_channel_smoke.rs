//! R126-3: Subgraph + Channel 集成 smoke tests
//!
//! 验证:
//! 1. Channel 4 type 集成: 1 个 graph 同时用 LastValue / Topic / NamedBarrier / BinaryOperatorValue
//! 2. Subgraph 嵌套: 父 → 子 (子含 2 节点), 父视角 2 个 node
//! 3. Subgraph 命名空间避 id 冲突: 2 个 sub 同 base node id, 命名空间化后无冲突
//! 4. Channel + Subgraph 混合: 父用 Channel, 子 subgraph 写 Channel
//! 5. Channel 跨 await 共享: Channel 在 await 前后共享 (Arc<...>)
//! 6. BinaryOperatorValue 跨节点累加: 3 节点各 write 1, read 返合并值
//! 7. NamedBarrier 跨节点协调: 3 节点各 write 1, read 放行
//! 8. Subgraph 嵌套 2 层: 父 → 子1 (子含 2 节点) → 子2 (孙 graph)

use apeireth_graph::{
    BinaryOperator, BinaryOperatorValue, Channel, ChannelRegistry, Graph, LastValue, NamedBarrier,
    Node, NodeId, NodeOutput, Result, State, Subgraph, Topic,
};
use serde_json::json;
use std::sync::Arc;

// ============================================================
// 测试用 Node 适配器
// ============================================================

struct WriteNode {
    id: &'static str,
    key: &'static str,
    value: serde_json::Value,
}

impl Node for WriteNode {
    fn id(&self) -> NodeId {
        self.id.to_string()
    }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        state.insert(self.key, self.value.clone());
        Ok(NodeOutput::new(self.id()))
    }
}

struct AppendTraceNode {
    id: &'static str,
    value: &'static str,
}

impl Node for AppendTraceNode {
    fn id(&self) -> NodeId {
        self.id.to_string()
    }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let mut trace = state
            .remove("trace")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        trace.push(json!(self.value));
        state.insert("trace", json!(trace));
        Ok(NodeOutput::new(self.id()))
    }
}

// ============================================================
// Test 1: Channel 4 type 集成 (1 graph 用 4 type)
// ============================================================

#[tokio::test]
async fn channel_4_types_integrate_in_one_graph() {
    // 验证 4 type Channel 都能用, 1:1 翻译 LangGraph 公开 Channel
    let last_value = Arc::new(LastValue::new("status"));
    let topic = Arc::new(Topic::new("events"));
    let barrier = Arc::new(NamedBarrier::new("sync", 2));
    let binop = Arc::new(BinaryOperatorValue::new("counter", BinaryOperator::Add));

    last_value.write(json!("ready")).unwrap();
    topic.write(json!("event1")).unwrap();
    topic.write(json!("event2")).unwrap();
    barrier.write(json!(null)).unwrap();
    binop.write(json!(10.0)).unwrap();
    binop.write(json!(20.0)).unwrap();

    // 1 个 graph 跑 dummy node (验证 channel 不影响 graph 主流程)
    let mut g = Graph::new();
    g.add_node(WriteNode {
        id: "noop",
        key: "k",
        value: json!("v"),
    });

    let final_state = g.execute(State::new()).await.unwrap();
    assert_eq!(final_state.execution_order, vec!["noop"]);

    // 4 channel 各自读
    assert_eq!(last_value.read().unwrap(), Some(json!("ready")));
    assert_eq!(topic.read().unwrap(), Some(json!("event1")));
    assert_eq!(topic.read().unwrap(), Some(json!("event2")));
    assert!(barrier.read().unwrap().is_none()); // 1 writer, expected 2
    assert_eq!(binop.read().unwrap().unwrap().as_f64().unwrap(), 30.0);
}

// ============================================================
// Test 2: Subgraph 嵌套 (父 → 子 2 节点)
// ============================================================

#[tokio::test]
async fn subgraph_nested_in_parent_graph() {
    let mut inner = Graph::new();
    inner.add_node(AppendTraceNode {
        id: "a",
        value: "inner_a",
    });
    inner.add_node(AppendTraceNode {
        id: "b",
        value: "inner_b",
    });
    inner.add_edge("a", "b");

    let sub = Subgraph::new("auth", inner);

    let mut parent = Graph::new();
    parent.add_node(AppendTraceNode {
        id: "main",
        value: "parent_main",
    });
    parent.add_node(sub.as_node());
    parent.add_edge("main", "subgraph.auth");

    let final_state = parent.execute(State::new()).await.unwrap();
    // 父视角 2 个 node
    assert_eq!(final_state.execution_order.len(), 2);
    assert_eq!(final_state.execution_order[0], "main");
    assert_eq!(final_state.execution_order[1], "subgraph.auth");
    // trace: 父 main → 子 a (trace 含 "inner_a") → 子 b (trace 含 "inner_b")
    let trace = final_state.get("trace").unwrap().as_array().unwrap();
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0], json!("parent_main"));
    assert_eq!(trace[1], json!("inner_a"));
    assert_eq!(trace[2], json!("inner_b"));
}

// ============================================================
// Test 3: 2 个 Subgraph 命名空间避 id 冲突
// ============================================================

#[tokio::test]
async fn two_subgraphs_namespace_avoid_id_collision() {
    // 2 个 subgraph 内部都用 "check" id, 命名空间化后父视角不冲突
    let mut inner1 = Graph::new();
    inner1.add_node(WriteNode {
        id: "check",
        key: "check1_result",
        value: json!("from_sub1"),
    });
    let mut inner2 = Graph::new();
    inner2.add_node(WriteNode {
        id: "check",
        key: "check2_result",
        value: json!("from_sub2"),
    });

    let sub1 = Subgraph::new("sub1", inner1);
    let sub2 = Subgraph::new("sub2", inner2);

    let mut parent = Graph::new();
    parent.add_node(sub1.as_node());
    parent.add_node(sub2.as_node());
    parent.add_edge("subgraph.sub1", "subgraph.sub2");

    let final_state = parent.execute(State::new()).await.unwrap();
    // 父视角 2 个 node (subgraph.sub1 + subgraph.sub2)
    assert_eq!(final_state.execution_order.len(), 2);
    // 2 个子都跑了, state 含 2 个 key
    assert_eq!(final_state.get("check1_result"), Some(&json!("from_sub1")));
    assert_eq!(final_state.get("check2_result"), Some(&json!("from_sub2")));
}

// ============================================================
// Test 4: Channel + Subgraph 混合
// ============================================================

#[tokio::test]
async fn channel_and_subgraph_combined() {
    // 父 graph 持有 1 个 channel, 跑 1 个 subgraph, subgraph 写 channel
    let last_value: Arc<LastValue> = Arc::new(LastValue::new("subgraph_status"));
    let last_value_for_node: Arc<LastValue> = Arc::clone(&last_value);

    // 自定义 node 写 channel
    struct ChannelWriteNode {
        id: &'static str,
        channel: Arc<LastValue>,
    }
    impl Node for ChannelWriteNode {
        fn id(&self) -> NodeId {
            self.id.to_string()
        }
        fn run(&self, _state: &mut State) -> Result<NodeOutput> {
            self.channel.write(json!("written_by_node")).unwrap();
            Ok(NodeOutput::new(self.id()))
        }
    }

    let mut inner = Graph::new();
    inner.add_node(ChannelWriteNode {
        id: "writer",
        channel: Arc::clone(&last_value_for_node),
    });

    let sub = Subgraph::new("inner", inner);

    let mut parent = Graph::new();
    parent.add_node(AppendTraceNode {
        id: "main",
        value: "main_run",
    });
    parent.add_node(sub.as_node());
    parent.add_edge("main", "subgraph.inner");

    let final_state = parent.execute(State::new()).await.unwrap();
    // 父跑完, channel 必含子节点的 write
    assert_eq!(last_value.read().unwrap(), Some(json!("written_by_node")));
    // 父 state 含 trace
    let trace = final_state.get("trace").unwrap().as_array().unwrap();
    assert!(trace.contains(&json!("main_run")));
}

// ============================================================
// Test 5: Channel 跨 await 共享 (Arc + Mutex)
// ============================================================

#[tokio::test]
async fn channel_shared_across_await() {
    let channel = Arc::new(LastValue::new("shared"));

    let channel_for_task = Arc::clone(&channel);
    let task = tokio::spawn(async move {
        // 在 spawned task 中写 channel
        channel_for_task.write(json!(42)).unwrap();
    });
    task.await.unwrap();

    // 主测试 task 读
    let v = channel.read().unwrap();
    assert_eq!(v, Some(json!(42)));
}

// ============================================================
// Test 6: BinaryOperatorValue 跨节点累加
// ============================================================

#[tokio::test]
async fn binary_operator_value_sums_across_nodes() {
    let counter: Arc<BinaryOperatorValue> =
        Arc::new(BinaryOperatorValue::new("counter", BinaryOperator::Add));
    let counter_for_node: Arc<BinaryOperatorValue> = Arc::clone(&counter);

    struct IncrementNode {
        id: &'static str,
        delta: f64,
        channel: Arc<BinaryOperatorValue>,
    }
    impl Node for IncrementNode {
        fn id(&self) -> NodeId {
            self.id.to_string()
        }
        fn run(&self, _state: &mut State) -> Result<NodeOutput> {
            self.channel.write(json!(self.delta)).unwrap();
            Ok(NodeOutput::new(self.id()))
        }
    }

    let mut g = Graph::new();
    g.add_node(IncrementNode {
        id: "inc1",
        delta: 10.0,
        channel: Arc::clone(&counter_for_node),
    });
    g.add_node(IncrementNode {
        id: "inc2",
        delta: 20.0,
        channel: Arc::clone(&counter_for_node),
    });
    g.add_node(IncrementNode {
        id: "inc3",
        delta: 30.0,
        channel: Arc::clone(&counter_for_node),
    });
    g.add_edge("inc1", "inc2");
    g.add_edge("inc2", "inc3");

    let _ = g.execute(State::new()).await.unwrap();
    // 3 节点各 write 1 次, read 返 60.0
    let v = counter.read().unwrap().unwrap();
    assert_eq!(v.as_f64().unwrap(), 60.0);
}

// ============================================================
// Test 7: NamedBarrier 跨节点协调
// ============================================================

#[tokio::test]
async fn named_barrier_releases_after_n_writers() {
    let barrier: Arc<NamedBarrier> = Arc::new(NamedBarrier::new("sync", 3));
    let barrier_for_node: Arc<NamedBarrier> = Arc::clone(&barrier);

    struct SignalNode {
        id: &'static str,
        channel: Arc<NamedBarrier>,
    }
    impl Node for SignalNode {
        fn id(&self) -> NodeId {
            self.id.to_string()
        }
        fn run(&self, _state: &mut State) -> Result<NodeOutput> {
            self.channel.write(json!(null)).unwrap();
            Ok(NodeOutput::new(self.id()))
        }
    }

    let mut g = Graph::new();
    g.add_node(SignalNode {
        id: "sig1",
        channel: Arc::clone(&barrier_for_node),
    });
    g.add_node(SignalNode {
        id: "sig2",
        channel: Arc::clone(&barrier_for_node),
    });
    g.add_node(SignalNode {
        id: "sig3",
        channel: Arc::clone(&barrier_for_node),
    });
    g.add_edge("sig1", "sig2");
    g.add_edge("sig2", "sig3");

    let _ = g.execute(State::new()).await.unwrap();
    // 3 节点 write 后, read 放行
    let v = barrier.read().unwrap().unwrap();
    assert_eq!(v["arrived"], 3);
    assert_eq!(v["expected"], 3);
    assert_eq!(v["barrier"], "sync");
}

// ============================================================
// Test 8: ChannelRegistry 4 type 全部 register + get
// ============================================================

#[tokio::test]
async fn channel_registry_all_4_types_round_trip() {
    let mut registry = ChannelRegistry::new();
    let last_value: Arc<dyn Channel> = Arc::new(LastValue::new("a"));
    let topic: Arc<dyn Channel> = Arc::new(Topic::new("b"));
    let barrier: Arc<dyn Channel> = Arc::new(NamedBarrier::new("c", 1));
    let binop: Arc<dyn Channel> = Arc::new(BinaryOperatorValue::new("d", BinaryOperator::Add));

    registry.register(Arc::clone(&last_value));
    registry.register(Arc::clone(&topic));
    registry.register(Arc::clone(&barrier));
    registry.register(Arc::clone(&binop));

    assert_eq!(registry.len(), 4);

    // 写 → 读 round-trip
    registry.get("a").unwrap().write(json!("hello")).unwrap();
    registry.get("b").unwrap().write(json!(42)).unwrap();
    registry.get("c").unwrap().write(json!(null)).unwrap();
    registry.get("d").unwrap().write(json!(100.0)).unwrap();

    assert_eq!(
        registry.get("a").unwrap().read().unwrap(),
        Some(json!("hello"))
    );
    assert_eq!(registry.get("b").unwrap().read().unwrap(), Some(json!(42)));
    assert!(registry.get("c").unwrap().read().unwrap().is_some()); // barrier 已放行
    assert_eq!(
        registry
            .get("d")
            .unwrap()
            .read()
            .unwrap()
            .unwrap()
            .as_f64()
            .unwrap(),
        100.0
    );
}
