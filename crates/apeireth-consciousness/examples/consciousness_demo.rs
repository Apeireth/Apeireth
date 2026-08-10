//! apeireth-consciousness demo — 演示 6 状态机的完整生命期.
//!
//! 运行: `cargo run -p apeireth-consciousness --example consciousness_demo`

use apeireth_consciousness::{
    can_transition, legal_targets, CognitiveDreamState, CognitiveDreamStateMachine,
    TransitionReason,
};

fn main() {
    println!("=== apeireth-consciousness demo ===\n");

    // 场景 1: 打印全部 6 状态语义.
    println!("[场景 1] 6 状态总览");
    for s in CognitiveDreamState::ALL {
        println!(
            "  {:>14} = {} ({})",
            s.semantic_name(),
            s.describe(),
            s as u8
        );
    }
    println!();

    // 场景 2: 打印全部合法转换矩阵.
    println!("[场景 2] 合法转换矩阵");
    for &from in &CognitiveDreamState::ALL {
        let targets = legal_targets(from);
        let target_names: Vec<&str> = targets.iter().map(|t| t.semantic_name()).collect();
        println!(
            "  {:>14} -> [{}]",
            from.semantic_name(),
            target_names.join(", ")
        );
    }
    println!();

    // 场景 3: 一次正常的主备 → 反思 → 梦境 → 冥想 → 恢复 → Awake 完整周期.
    println!(
        "[场景 3] 正常周期: Awake -> Reflecting -> Dreaming -> Meditating -> Recovering -> Awake"
    );
    let mut m = CognitiveDreamStateMachine::new("cid-demo-normal");
    println!("  init:           {:?}", m.current);
    m.enter_reflecting().unwrap();
    println!("  +reflecting:    {:?}", m.current);
    m.enter_dreaming().unwrap();
    println!("  +dreaming:      {:?}", m.current);
    m.enter_meditating().unwrap();
    println!("  +meditating:    {:?}", m.current);
    m.enter_recovering().unwrap();
    println!("  +recovering:    {:?}", m.current);
    m.reset_to_awake().unwrap();
    println!("  +reset_to_awake:{:?}", m.current);
    println!("  transitions:    {}", m.transition_count());
    println!();

    // 场景 4: L0 HA 紧急停 + 恢复.
    println!("[场景 4] L0 HA 紧急停 -> 唯一出口 Recovering -> Awake");
    let mut m = CognitiveDreamStateMachine::new("cid-demo-l0ha");
    println!("  init:           {:?}", m.current);
    m.enter_self_disabling().unwrap();
    println!(
        "  +self_disable:  {:?} (is_self_disabled={})",
        m.current,
        m.is_self_disabled()
    );
    // 试图跳过恢复 → 失败
    match m.reset_to_awake() {
        Ok(_) => println!("  ERR: 跳过恢复竟然成功!"),
        Err(e) => println!("  +try_skip:      ERR {e}"),
    }
    m.enter_recovering().unwrap();
    println!("  +recovering:    {:?}", m.current);
    m.reset_to_awake().unwrap();
    println!("  +reset_to_awake:{:?}", m.current);
    println!();

    // 场景 5: 校验 Awake -> Dreaming 非法.
    println!("[场景 5] 校验 Awake -> Dreaming 非法 (必须先 Reflecting)");
    println!(
        "  can_transition(Awake, Dreaming) = {} (期望 false)",
        can_transition(CognitiveDreamState::Awake, CognitiveDreamState::Dreaming)
    );
    println!("\n=== demo 完成 ===");
    // 静默使用以避免 dead_code 警告 (TransitionReason 已 export).
    let _ = TransitionReason::Internal;
}
