//! R129-6 Stage 6 K3 安全守护 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage6_k3_security_run`
//!
//! 演示 K3 安全守护: 6 重 v7 (B4 严守) + 1 跨语言 G7 + SecurityEvent 记录 + 7 门裁决

use apeireth_pybridge::{
    stage6_record_security, stage6_security_baseline_intact, stage6_security_healthy,
    stage6_security_summary, SecurityEvent, SecurityEventKind, SecurityGate, SecuritySeverity,
    SecurityVerdict,
};

fn main() {
    println!("=== R129-6 Stage 6 K3 安全守护 (anyone-can-run) ===\n");

    // 1. 6+1 重门 baseline 严守 verify (B4 6 重 v7 严守 + G7 跨语言 K3 创新)
    println!(
        "  B4 6-fold v7 baseline intact: {}",
        stage6_security_baseline_intact()
    );
    assert_eq!(SecurityGate::N_GATES, 7);
    assert!(stage6_security_baseline_intact());

    // 2. 演示 6 重 v7 + 1 跨语言 各记录 1 个 Pass 事件
    let gates = [
        (SecurityGate::G1Identity, "G1_Identity"),
        (SecurityGate::G2Goal, "G2_Goal"),
        (SecurityGate::G3Capability, "G3_Capability"),
        (SecurityGate::G4Compliance, "G4_Compliance"),
        (SecurityGate::G5Resource, "G5_Resource"),
        (SecurityGate::G6Audit, "G6_Audit"),
        (SecurityGate::G7CrossLanguage, "G7_CrossLanguage"),
    ];
    for (gate, name) in gates.iter() {
        stage6_record_security(SecurityEvent::new(
            *gate,
            SecurityEventKind::Pass,
            SecuritySeverity::Low,
            "demo",
            "r129-6 verify",
        ));
        println!("  {name}: Pass");
    }

    // 3. 演示 1 个 Block 事件
    stage6_record_security(SecurityEvent::new(
        SecurityGate::G1Identity,
        SecurityEventKind::Block,
        SecuritySeverity::High,
        "demo",
        "module denied",
    ));
    println!("  G1_Identity: Block (sample)");

    // 4. 4 裁决演示
    println!("\n  SecurityVerdict: Allow/Warn/Block/Audit (4)");
    for v in [
        SecurityVerdict::Allow,
        SecurityVerdict::Warn,
        SecurityVerdict::Block,
        SecurityVerdict::Audit,
    ] {
        println!("    {v} (is_pass={})", v.is_pass());
    }

    // 5. 摘要
    println!("\n  summary: {}", stage6_security_summary());
    println!("  healthy: {}", stage6_security_healthy());
    println!("  baseline_intact: {}", stage6_security_baseline_intact());

    println!("\n=== K3 done (B4 6-fold v7 严守 0 改) ===");
}
