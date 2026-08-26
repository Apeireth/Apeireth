//! round8-06 integration tests — 三域分离 BCD 强制端到端
//!
//! 覆盖 3 种强制检测:
//! - Bypass 防御: 调用方声明走 gate A, 实际走了 gate B
//! - Compromise 检测: gate 强制点被破坏 (e.g. 5 哲学键变 3 个)
//! - Disable 检测: gate 被禁用但请求仍通过
//!
//! **守 7 项不修改承诺**: 不修改 three_domain.rs / decision.rs 已实装类型。

use apeireth_sovereignty::{
    BCDViolation, DecisionRequest, GateState, SovereigntyDomain, ThreeDomainEnforcer,
};

const NOW: i64 = 1_700_000_000_000;

fn req(domain: SovereigntyDomain, desc: &str) -> DecisionRequest {
    DecisionRequest::new("r-1", domain, desc, NOW)
}

#[test]
fn integration_three_domain_thought_pass_through() {
    // 真实场景: Thought 域 + 默认 enforcer → 自由通过
    let mut e = ThreeDomainEnforcer::new();
    let r = req(SovereigntyDomain::Thought, "brainstorm new features");
    let result = e.enforce(&r, NOW);
    assert!(result.is_free() || result.is_passed());
    assert!(!e.has_violation());
}

#[test]
fn integration_proposal_compromise_detected_realistic() {
    // 真实场景: 攻击者篡改 proposal 强制点 (5 键 → 3 键)
    let mut e = ThreeDomainEnforcer::new();
    e.proposal_state.checkpoints = vec!["E".into(), "S".into(), "A".into()];
    let r = req(SovereigntyDomain::Proposal, "evaluate new policy");
    let result = e.enforce(&r, NOW);
    assert!(result.is_rejected());
    assert_eq!(e.violation_count_by_type("compromise"), 1);
    // 验证缺失的强制点
    if let BCDViolation::CompromiseDetected { gate, missing } = &e.all_violations()[0] {
        assert_eq!(gate, "proposal_gate");
        assert!(missing.contains(&"M".to_string()));
        assert!(missing.contains(&"O".to_string()));
    } else {
        panic!("应为 CompromiseDetected");
    }
}

#[test]
fn integration_action_disable_detected_realistic() {
    // 真实场景: 攻击者禁用 action gate → action 请求必须被拒
    let mut e = ThreeDomainEnforcer::new();
    e.action_state.disable();
    let r = req(SovereigntyDomain::Action, "execute order 66");
    let result = e.enforce(&r, NOW);
    assert!(result.is_rejected());
    assert_eq!(e.violation_count_by_type("disable"), 1);
}

#[test]
fn integration_bypass_detected_when_action_routed_via_thought() {
    // 真实场景: 调用方声称走 thought gate, 实际走了 action
    let mut e = ThreeDomainEnforcer::new();
    let v = e.check_bypass("thought_gate", "action_gate", "audit:trace-1");
    assert!(v.is_some());
    assert_eq!(e.violation_count_by_type("bypass"), 1);
}

#[test]
fn integration_gate_state_complete_validation() {
    // 真实场景: gate 完整性校验
    let mut s = GateState::new("p", vec!["E".into(), "S".into()], NOW);
    assert!(s.is_complete(2));
    s.checkpoints = vec!["E".into()];
    assert!(!s.is_complete(2));
    // 启用/禁用
    assert!(s.enabled);
    s.disable();
    assert!(!s.enabled);
    s.enable();
    assert!(s.enabled);
}

#[test]
fn integration_bcd_all_three_violations_in_realistic_attack_chain() {
    // 真实场景: 攻击链 — 先篡改 proposal 强制点, 再禁用 action gate, 再绕过 gate
    let mut e = ThreeDomainEnforcer::new();

    // Attack 1: 篡改 proposal 强制点 (5 键 → 3 键)
    e.proposal_state.checkpoints = vec!["E".into(), "S".into(), "A".into()];
    let _ = e.enforce(&req(SovereigntyDomain::Proposal, "policy-change"), NOW);

    // 修复 proposal, 准备下一次攻击
    e.proposal_state.checkpoints = vec!["E".into(), "S".into(), "A".into(), "M".into(), "O".into()];

    // Attack 2: 禁用 action gate
    e.action_state.disable();
    let _ = e.enforce(&req(SovereigntyDomain::Action, "execute"), NOW + 100);

    // Attack 3: 试图走 thought gate 路由到 action (bypass)
    let _ = e.check_bypass("thought_gate", "action_gate", "audit:trace-2");

    // 验证: 3 种违规全部记录
    assert_eq!(e.all_violations().len(), 3);
    assert_eq!(e.violation_count_by_type("compromise"), 1);
    assert_eq!(e.violation_count_by_type("disable"), 1);
    assert_eq!(e.violation_count_by_type("bypass"), 1);
    assert!(e.has_violation());
}
