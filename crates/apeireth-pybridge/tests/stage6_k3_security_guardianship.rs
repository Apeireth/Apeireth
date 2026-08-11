//! R129-6 K3 安全守护 集成测试
//!
//! per decision-61 §3.1 R129-6 — Stage 6 守护 K3 维度 (6+1 重门, B4 严守 0 改)

use apeireth_pybridge::{
    stage6_record_security, stage6_security_baseline_intact, stage6_security_healthy,
    stage6_security_summary, CrossLanguageCheck, SecurityEvent, SecurityEventKind, SecurityGate,
    SecurityGuard, SecuritySeverity, SecurityVerdict, V7BaselineCheck,
};

#[test]
fn k3_6_plus_1_gates_intact() {
    // B4 严守: 6 重 v7 + 1 跨语言 = 7 重
    assert_eq!(SecurityGate::N_GATES, 7);
    assert!(V7BaselineCheck::v7_baseline_intact());
    assert!(CrossLanguageCheck::g7_baseline_intact());
    // baseline_intact 公共 API 严守
    assert!(stage6_security_baseline_intact());
}

#[test]
fn k3_v7_baseline_6_gates() {
    for g in [
        SecurityGate::G1Identity,
        SecurityGate::G2Goal,
        SecurityGate::G3Capability,
        SecurityGate::G4Compliance,
        SecurityGate::G5Resource,
        SecurityGate::G6Audit,
    ] {
        assert!(g.is_v7_baseline());
    }
    assert!(!SecurityGate::G7CrossLanguage.is_v7_baseline());
}

#[test]
fn k3_record_and_summary() {
    stage6_record_security(SecurityEvent::new(
        SecurityGate::G1Identity,
        SecurityEventKind::Pass,
        SecuritySeverity::Low,
        "test",
        "global",
    ));
    let s = stage6_security_summary();
    assert!(s.contains("K3 SecurityGuard"));
}

#[test]
fn k3_security_event_block() {
    let ev = SecurityEvent::new(
        SecurityGate::G1Identity,
        SecurityEventKind::Block,
        SecuritySeverity::High,
        "pybridge",
        "denied",
    );
    assert!(ev.blocked);
}

#[test]
fn k3_security_event_chain() {
    let ev = SecurityEvent::new(
        SecurityGate::G7CrossLanguage,
        SecurityEventKind::Warn,
        SecuritySeverity::Medium,
        "bridge",
        "gil contention",
    )
    .with_context("test_ctx")
    .with_timestamp(42);
    assert!(!ev.blocked);
    assert_eq!(ev.context.as_deref(), Some("test_ctx"));
}

#[test]
fn k3_security_verdict_4() {
    assert!(SecurityVerdict::Allow.is_pass());
    assert!(!SecurityVerdict::Block.is_pass());
    assert_eq!(format!("{}", SecurityVerdict::Allow), "Allow");
    assert_eq!(format!("{}", SecurityVerdict::Block), "Block");
}

#[test]
fn k3_security_guard_7_gate_isolation() {
    let mut g = SecurityGuard::default();
    for gate in [
        SecurityGate::G1Identity,
        SecurityGate::G2Goal,
        SecurityGate::G3Capability,
        SecurityGate::G4Compliance,
        SecurityGate::G5Resource,
        SecurityGate::G6Audit,
        SecurityGate::G7CrossLanguage,
    ] {
        g.record(SecurityEvent::new(gate, SecurityEventKind::Pass, SecuritySeverity::Low, "x", "x"));
    }
    for i in 0..7 {
        assert_eq!(g.gate_counts[i], 1);
    }
    assert_eq!(g.total_events, 7);
}

#[test]
fn k3_security_guard_healthy() {
    let g = SecurityGuard::default();
    assert!(g.is_healthy());
    assert!(g.all_gates_intact());
}

#[test]
fn k3_security_guard_verdict_per_gate() {
    let mut g = SecurityGuard::default();
    assert_eq!(g.verdict(SecurityGate::G1Identity), SecurityVerdict::Allow);
    g.record(SecurityEvent::new(
        SecurityGate::G1Identity,
        SecurityEventKind::Block,
        SecuritySeverity::High,
        "x",
        "x",
    ));
    assert_eq!(g.verdict(SecurityGate::G1Identity), SecurityVerdict::Block);
}

#[test]
fn k3_security_event_kind_display() {
    assert_eq!(format!("{}", SecurityEventKind::Pass), "Pass");
    assert_eq!(format!("{}", SecurityEventKind::Block), "Block");
}

#[test]
fn k3_healthy_default() {
    let _ = stage6_security_healthy();
}

#[test]
fn k3_summary_contains_intact() {
    let s = stage6_security_summary();
    assert!(s.contains("K3 SecurityGuard"));
    assert!(s.contains("v7_intact=true"));
    assert!(s.contains("g7_intact=true"));
}

#[test]
fn k3_gate_names() {
    assert_eq!(SecurityGate::G1Identity.name(), "G1_Identity");
    assert_eq!(SecurityGate::G7CrossLanguage.name(), "G7_CrossLanguage");
}
