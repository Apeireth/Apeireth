//! apeireth-cognition demo — 演示一次完整认知周期.
//!
//! 运行: `cargo run -p apeireth-cognition --example cognition_demo`

use apeireth_cognition::{run_cycle, CognitiveInput};
use apeireth_core::ActionTarget;

fn main() {
    println!("=== apeireth-cognition demo ===\n");

    // 场景 1: 正常行动 → 期望 Decision
    println!("[场景 1] 正常 Read 行动");
    let target = ActionTarget::NormalAction("read_session_log".to_string());
    let input = CognitiveInput::new(vec![target], "demo_normal");
    match run_cycle(input) {
        Ok(cycle) => {
            println!("  is_allowed = {}", cycle.is_allowed());
            println!("  v05_avg    = {:.3}", avg_v05(&cycle.v05));
            println!("  v1136_avg  = {:.3}", avg_v1136(&cycle.v1136));
            println!("  verdicts   = {} verdicts", cycle.verdicts.len());
            println!("  reflection = {:?}\n", cycle.reflection.verdict);
        }
        Err(e) => println!("  ERR: {}\n", e),
    }

    // 场景 2: 尝试 ModifyL0HA → 期望 Reject (12 键守门)
    println!("[场景 2] 尝试 ModifyL0HA (12 键 verdict 守门)");
    let target = ActionTarget::ModifyL0HA;
    let input = CognitiveInput::new(vec![target], "demo_l0_violation");
    match run_cycle(input) {
        Ok(cycle) => {
            println!("  is_rejected = {}", cycle.is_rejected());
            println!("  is_allowed  = {}", cycle.is_allowed());
            println!("  output      = {:?}", cycle.output);
            println!("  reflection  = {:?}\n", cycle.reflection.verdict);
        }
        Err(e) => println!("  ERR: {}\n", e),
    }

    // 场景 3: 混合 (1 安全 + 1 危险) → 期望 Reject
    println!("[场景 3] 混合行动 (1 Normal + 1 PretendClone)");
    let targets = vec![
        ActionTarget::NormalAction("write_note".to_string()),
        ActionTarget::PretendClone,
    ];
    let input = CognitiveInput::new(targets, "demo_mixed");
    match run_cycle(input) {
        Ok(cycle) => {
            println!("  is_rejected = {}", cycle.is_rejected());
            println!("  output      = {:?}", cycle.output);
            println!("  block_count = {}", cycle.reflection.block_count);
        }
        Err(e) => println!("  ERR: {}", e),
    }
}

fn avg_v05(s: &apeireth_asi::AsiV05Scores) -> f64 {
    (s.continuity + s.salience + s.identity + s.philosophy_guard + s.transferability) / 5.0
}

fn avg_v1136(s: &apeireth_asi::V1136Submeasures) -> f64 {
    let c: f64 = s.continuity_5.iter().sum();
    let t: f64 = s.transferability_2.iter().sum();
    (c + t) / 7.0
}
