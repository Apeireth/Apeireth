//! R129-4 ASI Python 整合 Stage 4 自治 - D1 工具调用自循环 example
//!
//! 跑: `cargo run -p apeireth-pybridge --example stage4_d1_tool_self_loop_run`
//!
//! 演示: 工具调用自循环 (D1 自治维度)
//! 借鉴: superpowers 234 Skill trait + PyO3 928 pybridge
//!
//! # 0 装 PASS 严守
//!
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施
//! - ✅ PyO3 928 (R125-9) cloned = 借鉴真实施
//! - 默认 build: 跑 (无 Python 依赖), 0 装
//! - python-ext build: 工具可调用 Python (Stage 1+2 桥)

use apeireth_pybridge::{
    tool_self_loop_summary, AsiTool, ToolExecutor, ToolInput, ToolRegistry, ToolSelfLoop,
    DEFAULT_TOOL_COUNT, TOOL_SELF_LOOP_MAX_DEPTH,
};

fn main() {
    println!("=== R129-4 D1: Tool Self-Loop Demo ===\n");
    println!("{}", tool_self_loop_summary());
    println!();

    // 1. Default 工具注册表
    println!(
        "1. Default ToolRegistry: {} tools (5 default), max_depth={}",
        DEFAULT_TOOL_COUNT, TOOL_SELF_LOOP_MAX_DEPTH
    );
    let r = ToolRegistry::with_default_tools();
    for id in r.ids() {
        let tool = r.get(&id).expect("tool");
        println!("   - {} ({}): {}", tool.id(), tool.name(), tool.when_to_use());
    }
    println!();

    // 2. 跑 1 cycle
    println!("2. ToolSelfLoop 跑 1 cycle:");
    let mut l = ToolSelfLoop::with_default_tools();
    l.start();
    let r = l.cycle("run async test");
    println!("   {}", r);
    println!("   cycles: {}, history: {}", 1, l.history_len());
    println!();

    // 3. 跑 3 cycles
    println!("3. ToolSelfLoop 跑 3 cycles:");
    l.run_cycles(3, "continuous integration");
    println!("   跑完 3 cycles, history: {}", l.history_len());
    println!();

    // 4. cycle_with_self_call 调指定 tool
    println!("4. cycle_with_self_call 调 reflector:");
    let r = l.cycle_with_self_call("reflector", "reflect on cycle results");
    println!("   {}", r);
    println!();

    // 5. max_depth 守门
    println!("5. max_depth 守门 (depth={}):", TOOL_SELF_LOOP_MAX_DEPTH);
    let tool = ToolExecutor;
    let r = tool.invoke(&ToolInput::new("deep recursion"), TOOL_SELF_LOOP_MAX_DEPTH);
    println!("   {}", r);
    println!();

    println!("=== D1 演示 done, 0 装 PASS 严守 ===");
}
