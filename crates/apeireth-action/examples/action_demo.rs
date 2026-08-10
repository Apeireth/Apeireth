//! apeireth-action example: 端到端演示执行 + 表达 + 沉默三 trait。
//!
//! 运行: `cargo run -p apeireth-action --example action_demo`

use apeireth_action::{
    run_execute, run_express, run_silence, ActionAtom, ActionEngine, ActionExecution, ActionIntent,
    ActionPlan, ActionSilence, ExpressionChannel,
};
use apeireth_core::ActionTarget;

fn main() {
    let engine = ActionEngine::new();

    println!("=== apeireth-action 行动器官 demo ===\n");

    // 1. 执行: 构造 plan, 执行, 回滚
    let plan = ActionPlan::new(
        ActionTarget::NormalAction("read_file".to_string()),
        vec!["open_file".to_string(), "parse_lines".to_string()],
        "demo:read",
    );

    let result = run_execute(&engine, &plan).expect("plan valid");
    println!("[execute] {:?}", result);

    let tx = result.tx_id().expect("applied has tx_id");
    let rollback = engine.rollback_tx(tx);
    println!("[rollback] {:?}", rollback);

    // 2. 表达: 多通道输出
    let intent = ActionIntent::new(ActionTarget::NormalAction("greet".to_string()))
        .with_speaker("assistant")
        .with_audience("session_demo")
        .with_body_hint("你好, 这是 apeireth 行动器官 demo");

    let text_out = run_express(&engine, &intent, ExpressionChannel::Text);
    println!("\n[express:text] {}", text_out.text_payload());

    let struct_out = run_express(&engine, &intent, ExpressionChannel::Structured);
    println!(
        "[express:structured] {}",
        struct_out.to_json().unwrap_or_default()
    );

    // 3. 沉默: 不行动的合法理由
    println!("\n=== silence demo ===");

    // 3a. 正常 → 不沉默
    let normal_intent = ActionIntent::new(ActionTarget::NormalAction("ok".to_string()));
    println!(
        "[silence:normal] reason = {:?} (silent = {})",
        run_silence(&engine, &normal_intent),
        engine.should_silence(&normal_intent)
    );

    // 3b. 假装完美 → 伦理怀疑
    let pretend = ActionIntent::new(ActionTarget::PretendPerfect);
    println!(
        "[silence:pretend_perfect] reason = {:?} (silent = {})",
        run_silence(&engine, &pretend),
        engine.should_silence(&pretend)
    );

    // 3c. ModifyL0HA → 伦理怀疑
    let l0 = ActionIntent::new(ActionTarget::ModifyL0HA);
    println!(
        "[silence:l0_ha] reason = {:?} (silent = {})",
        run_silence(&engine, &l0),
        engine.should_silence(&l0)
    );

    // 4. dispatch_atom: 单步原子
    println!("\n=== dispatch_atom demo ===");
    let atom = ActionAtom::new(
        ActionTarget::NormalAction("single_step".to_string()),
        "payload_42",
    );
    let atom_result = engine.dispatch_atom(atom);
    println!("[dispatch_atom] {:?}", atom_result);

    // 5. tx_log 审计
    println!(
        "\n[audit] tx_count after demo = {} (rollback 清掉 plan 后, dispatch_atom 又加 1, 净剩 1)",
        engine.tx_count()
    );

    println!("\n=== demo 完成 ===");
}
