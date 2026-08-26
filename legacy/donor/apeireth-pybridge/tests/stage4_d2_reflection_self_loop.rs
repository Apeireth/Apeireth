//! R129-4 ASI Python 整合 Stage 4 自治 - D2 反思自循环集成测试
//!
//! **任务**: ASI Python 整合 Stage 4 自治 (per decision-61 §3.1 R129-4)
//! **借鉴**: langgraph 829 StateGraph 状态机 (R125-13 ✅ done)
//!           + aGLM 108 PODA 4 阶段 (R125-7 ✅ done)
//! **目标**: D2 反思自循环 (reflection self-loop, 8 节点) 集成测试

use apeireth_pybridge::{
    reflection_self_loop_summary, ReflectionAction, ReflectionGraph, ReflectionLoopStage,
    ReflectionNode, ReflectionSelfLoop, ReflectionState, REFLECTION_ACTION_COUNT,
    REFLECTION_GRAPH_NODE_COUNT, REFLECTION_MAX_DEPTH, REFLECTION_STATE_COUNT,
};

// 1. D2 ReflectionState 6 状态
#[test]
fn d2_01_reflection_state_6() {
    assert_eq!(ReflectionState::ALL.len(), REFLECTION_STATE_COUNT);
    assert_eq!(REFLECTION_STATE_COUNT, 6);
}

// 2. D2 ReflectionAction 5 动作
#[test]
fn d2_02_reflection_action_5() {
    assert_eq!(ReflectionAction::ALL.len(), REFLECTION_ACTION_COUNT);
    assert_eq!(REFLECTION_ACTION_COUNT, 5);
}

// 3. D2 ReflectionGraph 默认 8 节点
#[test]
fn d2_03_graph_default_8_nodes() {
    let g = ReflectionGraph::new_default();
    assert_eq!(g.node_count(), REFLECTION_GRAPH_NODE_COUNT);
    assert_eq!(REFLECTION_GRAPH_NODE_COUNT, 8);
}

// 4. D2 graph 8 节点 ID 唯一
#[test]
fn d2_04_graph_8_node_ids_unique() {
    let g = ReflectionGraph::new_default();
    let ids = g.node_ids();
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        assert!(seen.insert(id.clone()), "node id {id} 重复");
    }
    assert_eq!(ids.len(), 8);
}

// 5. D2 graph 5 内部节点名 (observe + analyze + reflect + refine + finalize = 5 主, 3 内部)
#[test]
fn d2_05_graph_5_main_3_internal() {
    let g = ReflectionGraph::new_default();
    let ids = g.node_ids();
    let main_nodes = ["observe", "analyze", "reflect", "refine", "finalize"];
    let internal_nodes = ["internal_audit", "internal_ceiling", "internal_harness"];
    for n in main_nodes {
        assert!(ids.contains(&n.to_string()), "缺主节点 {n}");
    }
    for n in internal_nodes {
        assert!(ids.contains(&n.to_string()), "缺内部节点 {n}");
    }
}

// 6. D2 graph reset
#[test]
fn d2_06_graph_reset() {
    let mut g = ReflectionGraph::new_default();
    g.move_to("analyze");
    g.reset();
    assert_eq!(g.current(), g.start_id());
}

// 7. D2 graph move_to 邻居
#[test]
fn d2_07_graph_move_to_neighbor() {
    let mut g = ReflectionGraph::new_default();
    assert!(g.move_to("analyze"));
    assert_eq!(g.current(), "analyze");
    assert!(g.move_to("reflect"));
    assert_eq!(g.current(), "reflect");
    assert!(g.move_to("refine"));
    assert_eq!(g.current(), "refine");
    assert!(g.move_to("finalize"));
    assert_eq!(g.current(), "finalize");
    assert!(!g.move_to("nonexistent"));
}

// 8. D2 graph 5 节点状态跨 5 state
#[test]
fn d2_08_graph_5_states() {
    let mut g = ReflectionGraph::new_default();
    assert_eq!(g.current_state(), Some(ReflectionState::Pending));
    g.move_to("analyze");
    assert_eq!(g.current_state(), Some(ReflectionState::Analyzing));
    g.move_to("reflect");
    assert_eq!(g.current_state(), Some(ReflectionState::Reflecting));
    g.move_to("refine");
    assert_eq!(g.current_state(), Some(ReflectionState::Refined));
    g.move_to("finalize");
    assert_eq!(g.current_state(), Some(ReflectionState::Finalized));
}

// 9. D2 ReflectionSelfLoop cycle 跑得通
#[test]
fn d2_09_self_loop_cycle() {
    let mut l = ReflectionSelfLoop::new();
    l.start();
    let r = l.cycle("test");
    assert!(r.success);
    assert_eq!(r.cycle, 1);
    // 改用 history_len verify
    assert_eq!(l.history_len(), 1);
}

// 10. D2 run_cycles(3) 跑 3 cycles
#[test]
fn d2_10_run_3_cycles() {
    let mut l = ReflectionSelfLoop::new();
    l.start();
    let results = l.run_cycles(3, "p");
    assert_eq!(results.len(), 3);
}

// 11. D2 4 阶段 ALL
#[test]
fn d2_11_4_loop_stages() {
    assert_eq!(ReflectionLoopStage::ALL.len(), 4);
    assert!(ReflectionLoopStage::Refine.is_terminal());
}

// 12. D2 max_depth = 5
#[test]
fn d2_12_max_depth() {
    assert_eq!(REFLECTION_MAX_DEPTH, 5);
}

// 13. D2 summary 引用 langgraph + aGLM
#[test]
fn d2_13_summary_cites_borrow_ids() {
    let s = reflection_self_loop_summary();
    assert!(s.contains("R129-4 D2"));
    assert!(s.contains("langgraph-829"));
    assert!(s.contains("aGLM-108"));
    assert!(s.contains("✅"));
    assert!(s.contains("0 装 PASS 严守"));
}

// 14. D2 graph_mut 可变
#[test]
fn d2_14_graph_mut_works() {
    let mut l = ReflectionSelfLoop::new();
    {
        let g = l.graph_mut();
        g.move_to("reflect");
    }
    assert_eq!(l.graph().current(), "reflect");
}

// 15. D2 ReflectionNode add_next
#[test]
fn d2_15_node_add_next() {
    let mut n = ReflectionNode::new("a", ReflectionState::Pending, "test");
    n.add_next("b");
    n.add_next("c");
    assert_eq!(n.next.len(), 2);
}
