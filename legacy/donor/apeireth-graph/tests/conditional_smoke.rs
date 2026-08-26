//! R33-5: Conditional edge integration smoke tests
//!
//! 验证:
//! 1. 基础 conditional: 2 node, 1 conditional edge with 2 branches
//! 2. Default fallback: condition 不在 path_map
//! 3. END 终止: condition 返 "__end__" → 终止
//! 4. Conditional chain: A → B → C (3 nodes 链式 conditional)
//! 5. Cycle detection: A → condition → A (会触发 cycle error)
//! 6. Conditional + DAG 混合: DAG 节点 + conditional 节点混跑
//! 7. Tool loop 借鉴 (R32-2 tool_loop 复用 LangGraph pattern): 模拟 max_turns 控制

use apeireth_graph::{Graph, GraphError, Node, NodeId, NodeOutput, Result, State, END_LABEL};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

struct AppendNode {
    id: String,
    value: String,
}

impl AppendNode {
    fn new(id: &str, value: &str) -> Self {
        Self {
            id: id.to_string(),
            value: value.to_string(),
        }
    }
}

impl Node for AppendNode {
    fn id(&self) -> NodeId {
        self.id.clone()
    }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let mut trace = state
            .remove("trace")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        trace.push(json!(self.value));
        state.insert("trace", json!(trace));
        Ok(NodeOutput::new(&self.id))
    }
}

fn state_get_turn(state: &State) -> u64 {
    state.get("turn").and_then(|v| v.as_u64()).unwrap_or(0)
}

#[tokio::test]
async fn conditional_two_branches_routes_to_target() {
    let mut g = Graph::new();
    g.add_node(AppendNode::new("a", "A"));
    g.add_node(AppendNode::new("b", "B"));
    g.add_node(AppendNode::new("c", "C"));
    let mut pm = BTreeMap::new();
    pm.insert("yes".to_string(), "b".to_string());
    pm.insert("no".to_string(), "c".to_string());
    g.add_conditional_edge(
        "a",
        pm,
        None,
        Arc::new(|s| {
            if s.get("decision").and_then(|v| v.as_str()) == Some("yes") {
                "yes".to_string()
            } else {
                "no".to_string()
            }
        }),
    );

    let s_yes = State::with("decision", "yes");
    let f = g.execute(s_yes).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "b"]);
    assert_eq!(f.get("trace").unwrap().as_array().unwrap().len(), 2);

    let s_no = State::with("decision", "no");
    let f = g.execute(s_no).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "c"]);
}

#[tokio::test]
async fn conditional_default_fallback_when_label_missing() {
    let mut g = Graph::new();
    g.add_node(AppendNode::new("a", "A"));
    g.add_node(AppendNode::new("d", "D"));
    let mut pm = BTreeMap::new();
    pm.insert("known".to_string(), "d".to_string());
    g.add_conditional_edge(
        "a",
        pm,
        Some("d".to_string()),
        Arc::new(|_| "unknown".to_string()),
    );
    let f = g.execute(State::new()).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "d"]);
}

#[tokio::test]
async fn conditional_end_label_terminates_execution() {
    let mut g = Graph::new();
    g.add_node(AppendNode::new("a", "A"));
    g.add_node(AppendNode::new("never", "NEVER"));
    let mut pm = BTreeMap::new();
    pm.insert("end".to_string(), "never".to_string());
    g.add_conditional_edge("a", pm, None, Arc::new(|_| END_LABEL.to_string()));
    let f = g.execute(State::new()).await.unwrap();
    assert_eq!(f.execution_order, vec!["a"]);
}

#[tokio::test]
async fn conditional_chain_a_b_c() {
    let mut g = Graph::new();
    for id in ["a", "b", "c"] {
        g.add_node(AppendNode::new(id, id));
    }
    let mut pm = BTreeMap::new();
    pm.insert("next".to_string(), "b".to_string());
    g.add_conditional_edge("a", pm, None, Arc::new(|_| "next".to_string()));
    let mut pm2 = BTreeMap::new();
    pm2.insert("end".to_string(), "c".to_string());
    g.add_conditional_edge("b", pm2, None, Arc::new(|_| "end".to_string()));
    let f = g.execute(State::new()).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "b", "c"]);
}

#[tokio::test]
async fn conditional_cycle_detected() {
    let mut g = Graph::new();
    g.add_node(AppendNode::new("a", "A"));
    g.add_node(AppendNode::new("b", "B"));
    let mut pm = BTreeMap::new();
    pm.insert("loop".to_string(), "a".to_string());
    g.add_conditional_edge("a", pm, None, Arc::new(|_| "loop".to_string()));
    let result = g.execute(State::new()).await;
    assert!(matches!(result, Err(GraphError::Cycle { .. })));
}

#[tokio::test]
async fn conditional_mixed_with_dag() {
    let mut g = Graph::new();
    for id in ["a", "b", "c", "d"] {
        g.add_node(AppendNode::new(id, id));
    }
    g.add_edge("a", "b");
    let mut pm = BTreeMap::new();
    pm.insert("c_path".to_string(), "c".to_string());
    pm.insert("d_path".to_string(), "d".to_string());
    g.add_conditional_edge(
        "b",
        pm,
        None,
        Arc::new(|s| {
            if s.get("route").and_then(|v| v.as_str()) == Some("d") {
                "d_path".to_string()
            } else {
                "c_path".to_string()
            }
        }),
    );

    let s_c = State::with("route", "c");
    let f = g.execute(s_c).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "b", "c"]);

    let s_d = State::with("route", "d");
    let f = g.execute(s_d).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "b", "d"]);
}

#[tokio::test]
async fn conditional_with_state_evolution_max_1_iteration() {
    // 真实 tool loop: step 节点 + step 自己的 conditional 控循环
    // 借 apeireth-pipeline::tool_loop 1:1 (max_turns=1 跑满)
    struct LoopStepNode {
        id: String,
    }
    impl Node for LoopStepNode {
        fn id(&self) -> NodeId {
            self.id.clone()
        }
        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            let turn = state.get("turn").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
            state.insert("turn", json!(turn));
            Ok(NodeOutput::new(&self.id))
        }
    }

    let mut g = Graph::new();
    g.add_node(AppendNode::new("init", "INIT"));
    g.add_node(LoopStepNode {
        id: "step".to_string(),
    });
    let mut pm1 = BTreeMap::new();
    pm1.insert("start".to_string(), "step".to_string());
    g.add_conditional_edge("init", pm1, None, Arc::new(|_| "start".to_string()));
    let mut pm2 = BTreeMap::new();
    pm2.insert("end".to_string(), "init".to_string());
    g.add_conditional_edge(
        "step",
        pm2,
        None,
        Arc::new(|s| {
            let turn = state_get_turn(s);
            if turn >= 1 {
                END_LABEL.to_string()
            } else {
                "end".to_string()
            }
        }),
    );

    let f = g.execute(State::new()).await.unwrap();
    // init -> step (turn=1) -> condition 返 "end" (不在 pm, default=None) -> 终止
    assert_eq!(f.execution_order, vec!["init", "step"]);
    assert_eq!(f.get("turn").unwrap().as_u64().unwrap(), 1);
}

#[tokio::test]
async fn conditional_tool_loop_max_2_iterations_clamps() {
    // 工具循环 max_turns 截断: init (entry) -> step -> step (self-loop until turn>=2).
    // step 自己条件控循环, 跑 2 轮后返 END_LABEL, 走 LangGraph 1:1 语义.
    struct LoopStepNode {
        id: String,
    }
    impl Node for LoopStepNode {
        fn id(&self) -> NodeId {
            self.id.clone()
        }
        fn run(&self, state: &mut State) -> Result<NodeOutput> {
            let turn = state.get("turn").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
            state.insert("turn", json!(turn));
            Ok(NodeOutput::new(&self.id))
        }
    }

    let mut g = Graph::new();
    g.add_node(AppendNode::new("init", "INIT"));
    g.add_node(LoopStepNode {
        id: "step".to_string(),
    });
    let mut pm1 = BTreeMap::new();
    pm1.insert("start".to_string(), "step".to_string());
    g.add_conditional_edge("init", pm1, None, Arc::new(|_| "start".to_string()));
    // step 的 self-loop: turn<2 走 "loop" -> step (再跑一轮), 否则 END_LABEL
    let mut pm2 = BTreeMap::new();
    pm2.insert("loop".to_string(), "step".to_string());
    g.add_conditional_edge(
        "step",
        pm2,
        None,
        Arc::new(|s| {
            let turn = state_get_turn(s);
            if turn >= 2 {
                END_LABEL.to_string()
            } else {
                "loop".to_string()
            }
        }),
    );

    let f = g.execute(State::new()).await.unwrap();
    // init -> step (turn=1) -> step (turn=2) -> END_LABEL 终止
    assert_eq!(f.execution_order, vec!["init", "step", "step"]);
    assert_eq!(f.get("turn").unwrap().as_u64().unwrap(), 2);
}

#[tokio::test]
async fn conditional_no_label_terminates_after_dag() {
    let mut g = Graph::new();
    g.add_node(AppendNode::new("a", "A"));
    g.add_node(AppendNode::new("b", "B"));
    g.add_edge("a", "b");
    let f = g.execute(State::new()).await.unwrap();
    assert_eq!(f.execution_order, vec!["a", "b"]);
}

#[tokio::test]
async fn conditional_uses_arc_to_capture_external_counter() {
    // 借鉴 LangGraph: condition 闭包可捕获外部 state
    let counter = Arc::new(Mutex::new(0u32));
    let counter_for_cond = Arc::clone(&counter);
    let mut g = Graph::new();
    g.add_node(AppendNode::new("src", "SRC"));
    g.add_node(AppendNode::new("never", "NEVER"));
    let mut pm = BTreeMap::new();
    pm.insert("inc".to_string(), "never".to_string());
    g.add_conditional_edge(
        "src",
        pm,
        None,
        Arc::new(move |_| {
            let mut c = counter_for_cond.lock().unwrap();
            *c += 1;
            END_LABEL.to_string()
        }),
    );
    let f = g.execute(State::new()).await.unwrap();
    // 1 次 conditional 调用, counter=1, END_LABEL → 终止
    assert_eq!(f.execution_order, vec!["src"]);
    assert_eq!(*counter.lock().unwrap(), 1);
}
