//! R129-4 ASI Python 整合 Stage 4 自治 - D2 反思自循环 example
//!
//! 跑: `cargo run -p apeireth-pybridge --example stage4_d2_reflection_self_loop_run`
//!
//! 演示: 反思自循环 (D2 自治维度)
//! 借鉴: langgraph 829 StateGraph + aGLM 108 PODA cycle
//!
//! # 0 装 PASS 严守
//!
//! - ✅ langgraph 829 (R125-13) cloned = 借鉴真实施
//! - ✅ aGLM 108 (R125-7) cloned = 借鉴真实施

use apeireth_pybridge::{
    reflection_self_loop_summary, ReflectionGraph, ReflectionSelfLoop, REFLECTION_GRAPH_NODE_COUNT,
    REFLECTION_MAX_DEPTH, REFLECTION_STATE_COUNT,
};

fn main() {
    println!("=== R129-4 D2: Reflection Self-Loop Demo ===\n");
    println!("{}", reflection_self_loop_summary());
    println!();

    // 1. ReflectionGraph 默认 8 节点
    println!(
        "1. ReflectionGraph 默认 {} 节点, {} states:",
        REFLECTION_GRAPH_NODE_COUNT, REFLECTION_STATE_COUNT
    );
    let g = ReflectionGraph::new_default();
    for id in g.node_ids() {
        println!("   - {}", id);
    }
    println!();

    // 2. 跑 1 cycle (Observe → Analyze → Reflect → Refine 4 阶段闭环)
    println!("2. ReflectionSelfLoop 跑 1 cycle (4 阶段):");
    let mut l = ReflectionSelfLoop::new();
    l.start();
    let r = l.cycle("v1077 measurement result");
    println!("   {}", r);
    println!();

    // 3. 跑 3 cycles
    println!("3. ReflectionSelfLoop 跑 3 cycles:");
    l.run_cycles(3, "v1400 self framework");
    println!("   跑完 3 cycles, history: {}", l.history_len());
    println!();

    // 4. max_depth = 5
    println!("4. max_depth = {} (反思最大深度守门):", REFLECTION_MAX_DEPTH);
    println!();

    // 5. graph 重置
    println!("5. ReflectionGraph reset + move_to:");
    {
        let g = l.graph_mut();
        g.move_to("analyze");
        println!("   move to analyze: current = {}", g.current());
        g.reset();
        println!("   reset: current = {} (起点)", g.current());
    }
    println!();

    println!("=== D2 演示 done, 0 装 PASS 严守 ===");
}
