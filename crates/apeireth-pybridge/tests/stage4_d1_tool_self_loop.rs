//! R129-4 ASI Python 整合 Stage 4 自治 - D1 工具调用自循环集成测试
//!
//! **任务**: ASI Python 整合 Stage 4 自治 (per decision-61 §3.1 R129-4)
//! **借鉴**: superpowers 234 Skill trait + SkillRegistry 模式 (R125-14 ✅ done)
//!           + PyO3 928 Python ↔ Rust bridge (R125-9 ✅ done)
//! **目标**: D1 工具调用自循环 (tool self-loop, max_depth 守门) 集成测试
//!
//! # 0 装 PASS 严守
//!
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施
//! - ✅ PyO3 928 (R125-9) cloned = 借鉴真实施
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 严守
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 commit (写到 reports 不动 git)
//! - C2 0 装 PASS 严守
//! - 0 主动 push

use apeireth_pybridge::{
    tool_self_loop_summary, AsiTool, ToolComposer, ToolExecutor, ToolInput, ToolLoopStage,
    ToolPlanner, ToolReflector, ToolRegistry, ToolResult, ToolSelfLoop, ToolValidator,
    DEFAULT_TOOL_COUNT, TOOL_SELF_LOOP_MAX_DEPTH,
};

// 1. D1 5 default tools 5 ID 唯一
#[test]
fn d1_01_default_tool_ids_unique() {
    let r = ToolRegistry::with_default_tools();
    let ids = r.ids();
    assert_eq!(ids.len(), DEFAULT_TOOL_COUNT);
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        assert!(seen.insert(id.clone()), "tool id {id} 重复");
    }
}

// 2. D1 5 default tools 各自 tdd_required = true
#[test]
fn d1_02_default_tools_tdd_required() {
    let r = ToolRegistry::with_default_tools();
    for id in r.ids() {
        let tool = r.get(&id).expect("tool");
        assert!(tool.tdd_required(), "{id} tdd_required 必 = true");
    }
}

// 3. D1 5 default tools 各自 when_to_use 不空
#[test]
fn d1_03_default_tools_when_to_use_not_empty() {
    let r = ToolRegistry::with_default_tools();
    for id in r.ids() {
        let tool = r.get(&id).expect("tool");
        assert!(!tool.when_to_use().is_empty(), "{id} when_to_use 不能空");
    }
}

// 4. D1 ToolSelfLoop cycle 跑得通
#[test]
fn d1_04_tool_self_loop_cycle_runs() {
    let mut l = ToolSelfLoop::with_default_tools();
    l.start();
    let r = l.cycle("test");
    assert!(r.result.success);
    assert_eq!(l.history_len(), 1);
}

// 5. D1 ToolSelfLoop run_cycles(3) 跑 3 cycles
#[test]
fn d1_05_tool_self_loop_run_3_cycles() {
    let mut l = ToolSelfLoop::with_default_tools();
    l.start();
    let reports = l.run_cycles(3, "p");
    assert_eq!(reports.len(), 3);
}

// 6. D1 max_depth 守门
#[test]
fn d1_06_max_depth_guard() {
    assert_eq!(TOOL_SELF_LOOP_MAX_DEPTH, 3);
    let tool = ToolExecutor;
    let r = tool.invoke(&ToolInput::new("hi"), TOOL_SELF_LOOP_MAX_DEPTH);
    assert!(!r.success);
    assert!(r.error.as_ref().unwrap().contains("max depth"));
}

// 7. D1 4 阶段 enum
#[test]
fn d1_07_4_loop_stages() {
    assert_eq!(ToolLoopStage::ALL.len(), 4);
    assert!(ToolLoopStage::Act.is_terminal());
    assert!(!ToolLoopStage::Observe.is_terminal());
}

// 8. D1 tool result 字段
#[test]
fn d1_08_tool_result_fields() {
    let r = ToolResult {
        tool_id: "executor".to_string(),
        success: true,
        output: "o".to_string(),
        error: None,
        depth: 1,
        sub_calls: 2,
    };
    assert_eq!(r.tool_id, "executor");
    assert!(r.success);
    assert_eq!(r.depth, 1);
}

// 9. D1 cycle_with_self_call 调指定 tool
#[test]
fn d1_09_cycle_with_self_call() {
    let mut l = ToolSelfLoop::with_default_tools();
    l.start();
    let r = l.cycle_with_self_call("reflector", "r");
    assert!(r.result.success);
    assert!(r.tool_id == "reflector");
}

// 10. D1 tool 5 default 都是 Send + Sync
#[test]
fn d1_10_default_tools_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ToolExecutor>();
    assert_send_sync::<ToolReflector>();
    assert_send_sync::<ToolPlanner>();
    assert_send_sync::<ToolValidator>();
    assert_send_sync::<ToolComposer>();
}

// 11. D1 summary 引用 superpowers + PyO3
#[test]
fn d1_11_summary_cites_borrow_ids() {
    let s = tool_self_loop_summary();
    assert!(s.contains("R129-4 D1"));
    assert!(s.contains("superpowers-234"));
    assert!(s.contains("PyO3-928"));
    assert!(s.contains("✅"));
    assert!(s.contains("0 装 PASS 严守"));
}

// 12. D1 ToolInput with context
#[test]
fn d1_12_tool_input_with_context() {
    let i = ToolInput::new("p").with("k", "v");
    assert_eq!(i.prompt, "p");
    assert_eq!(i.context.get("k"), Some(&"v".to_string()));
}

// 13. D1 cycle_with_self_call 工具不存在
#[test]
fn d1_13_cycle_with_unknown_tool() {
    let mut l = ToolSelfLoop::with_default_tools();
    l.start();
    let r = l.cycle_with_self_call("nope", "x");
    assert!(!r.result.success);
    assert!(r.result.error.as_ref().unwrap().contains("not registered"));
}

// 14. D1 ToolRegistry register + get
#[test]
fn d1_14_registry_register_get() {
    let mut r = ToolRegistry::new();
    r.register(Box::new(ToolExecutor));
    assert_eq!(r.len(), 1);
    assert!(r.get("executor").is_some());
    assert!(r.get("nope").is_none());
}

// 15. D1 run_cycles(0) = 1
#[test]
fn d1_15_run_0_cycles_means_1() {
    let mut l = ToolSelfLoop::with_default_tools();
    l.start();
    let reports = l.run_cycles(0, "p");
    assert_eq!(reports.len(), 1);
}
