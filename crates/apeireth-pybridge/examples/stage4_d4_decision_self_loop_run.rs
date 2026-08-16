//! R129-4 ASI Python 整合 Stage 4 自治 - D4 决策自循环 example
//!
//! 跑: `cargo run -p apeireth-pybridge --example stage4_d4_decision_self_loop_run`
//!
//! 演示: 决策自循环 (D4 自治维度)
//! 借鉴: aGLM 108 PODA 4 阶段 + superpowers 234 Skill priority 5 层级
//!
//! # 0 装 PASS 严守
//!
//! - ✅ aGLM 108 (R125-7) cloned = 借鉴真实施
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施

use std::collections::HashMap;

use apeireth_pybridge::{
    decision_self_loop_summary, DecisionPolicy, DecisionSelfLoop, DecisionTrigger,
    DECISION_MAX_REVISIT, DECISION_POLICY_COUNT, DECISION_STAGE_COUNT, DECISION_STATE_COUNT,
    DECISION_TRIGGER_COUNT,
};

fn main() {
    println!("=== R129-4 D4: Decision Self-Loop Demo ===\n");
    println!("{}", decision_self_loop_summary());
    println!();

    // 1. 5 维: 5 policy + 4 stage + 5 trigger + 5 state + max_revisit
    println!(
        "1. 5 维: policies={}, stages={}, triggers={}, states={}, max_revisit={}",
        DECISION_POLICY_COUNT,
        DECISION_STAGE_COUNT,
        DECISION_TRIGGER_COUNT,
        DECISION_STATE_COUNT,
        DECISION_MAX_REVISIT
    );
    println!();

    // 2. 5 决策策略 weight 0-4
    println!("2. 5 决策策略 (weight 0-4):");
    for p in DecisionPolicy::ALL {
        println!(
            "   - {} (weight={}): {}",
            p.name(),
            p.weight(),
            p.description()
        );
    }
    println!();

    // 3. 5 触发器 + suggested_policy
    println!("3. 5 触发器 + suggested_policy:");
    for t in DecisionTrigger::ALL {
        println!("   - {} → {}", t.name(), t.suggested_policy().name());
    }
    println!();

    // 4. 跑 1 cycle (decide → act)
    println!("4. DecisionSelfLoop 跑 1 cycle (decide → act):");
    let mut l = DecisionSelfLoop::new();
    l.start();
    let r = l.cycle("use v1458 anchor 0.9105", "preserve baseline 0.8682");
    println!("   policy: {}", l.policy().name());
    println!("   decision: {}", r.decision);
    println!("   reason: {}", r.reason);
    println!();

    // 5. revisit_decision 重做
    println!("5. revisit_decision 重做:");
    let r1 = l.revisit_decision("use v1458 anchor 0.9105", "preserve baseline 0.8682");
    let r2 = l.revisit_decision("use v1458 anchor 0.9105", "preserve baseline 0.8682");
    let r3 = l.revisit_decision("use v1458 anchor 0.9105", "preserve baseline 0.8682");
    let r4 = l.revisit_decision("use v1458 anchor 0.9105", "preserve baseline 0.8682");
    println!("   revisit 1: {}", if r1.is_some() { "✅" } else { "❌" });
    println!("   revisit 2: {}", if r2.is_some() { "✅" } else { "❌" });
    println!(
        "   revisit 3: {} (max_revisit={})",
        if r3.is_some() { "✅" } else { "❌" },
        DECISION_MAX_REVISIT
    );
    println!(
        "   revisit 4: {} (max_revisit 守门, 必 None)",
        if r4.is_some() { "✅" } else { "❌" }
    );
    println!();

    // 6. detect_and_tune 调 policy
    println!("6. detect_and_tune (基于 metrics 调 policy):");
    let mut m = HashMap::new();
    m.insert("hard_walls_pass".to_string(), "false".to_string());
    let t = l.detect_and_tune(&m);
    println!(
        "   metrics: hard_walls_pass=false → trigger={}, policy={}",
        t.name(),
        l.policy().name()
    );
    println!();

    println!("=== D4 演示 done, 0 装 PASS 严守 ===");
}
