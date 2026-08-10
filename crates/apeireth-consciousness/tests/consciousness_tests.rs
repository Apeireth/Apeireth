//! 集成测试: 跨模块/外部 API 调用.
use apeireth_consciousness::{
    can_transition, legal_targets, CognitiveDreamState, CognitiveDreamStateMachine,
};

#[test]
fn integration_full_lifecycle_awake_to_awake() {
    // 集成: 完整生命期 Awake -> ... -> Awake, 6 步, 校验 history.
    let mut m = CognitiveDreamStateMachine::new("cid-integration-1");
    assert_eq!(m.continuity_id, "cid-integration-1");
    assert_eq!(m.current, CognitiveDreamState::Awake);
    assert_eq!(m.transition_count(), 0);

    m.enter_reflecting().unwrap();
    m.enter_dreaming().unwrap();
    m.enter_meditating().unwrap();
    m.enter_recovering().unwrap();
    m.reset_to_awake().unwrap();

    assert_eq!(m.transition_count(), 5);
    assert_eq!(m.current, CognitiveDreamState::Awake);
    assert!(!m.is_self_disabled());
}

#[test]
fn integration_self_disable_lock_to_recovering() {
    // 集成: 验证 SelfDisabling 是单向锁 — 只能经 Recovering 退出.
    let mut m =
        CognitiveDreamStateMachine::with_initial("cid-integration-2", CognitiveDreamState::Awake);
    m.enter_self_disabling().unwrap();
    assert!(m.is_self_disabled());

    // Awake 直跳: 非法
    assert!(!can_transition(m.current, CognitiveDreamState::Awake));
    // Reflecting 直跳: 非法
    assert!(!can_transition(m.current, CognitiveDreamState::Reflecting));
    // Recovering 直跳: 合法
    assert!(can_transition(m.current, CognitiveDreamState::Recovering));

    m.enter_recovering().unwrap();
    // Recovering -> Awake: 合法
    assert!(can_transition(m.current, CognitiveDreamState::Awake));
    m.reset_to_awake().unwrap();
    assert_eq!(m.current, CognitiveDreamState::Awake);
}

#[test]
fn integration_legal_targets_matrix_invariant() {
    // 集成: 验证 6 状态共 16 条合法转换 (审计完整性).
    let mut total = 0;
    for &from in &CognitiveDreamState::ALL {
        total += legal_targets(from).len();
    }
    // Awake(2) + Reflecting(4) + Dreaming(3) + Meditating(3) + SelfDisabling(1) + Recovering(2) = 15
    assert_eq!(total, 15, "合法转换总数应为 15, 实际 {}", total);
}
