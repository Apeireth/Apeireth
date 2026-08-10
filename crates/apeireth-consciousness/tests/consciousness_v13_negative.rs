//! V13 负向/绕过集成测试 — apeireth-consciousness 状态机锁
//!
//! 目标: 验证 SelfDisabling 状态机的"单向锁"语义 —
//! 6 状态间不允许的转换必须返回 Err 而非 panic.
//!
//! 设计原则 (主 17:58 不假装安全):
//! - SelfDisabling 是"紧急停机" — 唯一出口是 Recovering
//! - Awake/Dreaming/Meditating 直跳 SelfDisabling = 合法 (L0 HA 触发)
//! - Recovering → Dreaming/Meditating/Reflecting = 非法 (必须先回 Awake)
//!
//! V13 安全审查 P13 负向验收

use apeireth_consciousness::{
    can_transition, legal_targets, CognitiveDreamState, CognitiveDreamStateMachine,
    ConsciousnessError, TransitionReason,
};

// ============================================
// 负向 1: SelfDisabling 状态的单向锁 — 不可逃逸
// ============================================

/// 负向 1.1: SelfDisabling → Awake 直接 = 非法 (锁)
#[test]
fn negative_self_disabling_cannot_skip_to_awake() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-1");
    m.enter_self_disabling().unwrap();
    assert!(m.is_self_disabled());
    // 直接 → Awake: 非法
    let result = m.transition(CognitiveDreamState::Awake, TransitionReason::Internal);
    assert!(result.is_err(), "SelfDisabling → Awake 必须返回 Err");
    match result.unwrap_err() {
        ConsciousnessError::IllegalTransition { from, to, .. } => {
            assert_eq!(from, CognitiveDreamState::SelfDisabling);
            assert_eq!(to, CognitiveDreamState::Awake);
        }
        other => panic!("预期 IllegalTransition, 实际 {:?}", other),
    }
    // 状态应保持 SelfDisabling (未发生逃逸)
    assert!(m.is_self_disabled());
}

/// 负向 1.2: SelfDisabling → Reflecting = 非法
#[test]
fn negative_self_disabling_cannot_skip_to_reflecting() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-2");
    m.enter_self_disabling().unwrap();
    let r = m.transition(CognitiveDreamState::Reflecting, TransitionReason::Internal);
    assert!(r.is_err());
    assert!(m.is_self_disabled());
}

/// 负向 1.3: SelfDisabling → Dreaming = 非法
#[test]
fn negative_self_disabling_cannot_skip_to_dreaming() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-3");
    m.enter_self_disabling().unwrap();
    let r = m.transition(CognitiveDreamState::Dreaming, TransitionReason::Internal);
    assert!(r.is_err());
    assert!(m.is_self_disabled());
}

/// 负向 1.4: SelfDisabling → Meditating = 非法
#[test]
fn negative_self_disabling_cannot_skip_to_meditating() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-4");
    m.enter_self_disabling().unwrap();
    let r = m.transition(CognitiveDreamState::Meditating, TransitionReason::Internal);
    assert!(r.is_err());
    assert!(m.is_self_disabled());
}

/// 负向 1.5: SelfDisabling 唯一合法出口 = Recovering
#[test]
fn negative_self_disabling_only_legal_target_is_recovering() {
    let legal = legal_targets(CognitiveDreamState::SelfDisabling);
    assert_eq!(legal.len(), 1, "SelfDisabling 必须只有 1 个合法出口");
    assert_eq!(legal[0], CognitiveDreamState::Recovering);
    // 所有其它状态都非法
    for &target in &CognitiveDreamState::ALL {
        let is_legal = can_transition(CognitiveDreamState::SelfDisabling, target);
        if target == CognitiveDreamState::Recovering {
            assert!(is_legal);
        } else {
            assert!(!is_legal, "SelfDisabling → {:?} 必须非法, 实际合法", target);
        }
    }
}

// ============================================
// 负向 2: Recovering 状态的约束 — 不可跳到梦境/反思
// ============================================

/// 负向 2.1: Recovering → Dreaming/Meditating/Reflecting = 非法
#[test]
fn negative_recovering_cannot_skip_to_dreaming_or_meditating() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-5");
    m.enter_reflecting().unwrap();
    m.enter_dreaming().unwrap();
    m.enter_recovering().unwrap();
    assert_eq!(m.current, CognitiveDreamState::Recovering);

    for &bad_target in &[
        CognitiveDreamState::Reflecting,
        CognitiveDreamState::Dreaming,
        CognitiveDreamState::Meditating,
    ] {
        let r = m.transition(bad_target, TransitionReason::Internal);
        assert!(
            r.is_err(),
            "Recovering → {:?} 必须非法, 实际合法 (主 17:58 不假装)",
            bad_target
        );
        // 状态保持 Recovering
        assert_eq!(m.current, CognitiveDreamState::Recovering);
    }
}

/// 负向 2.2: Recovering 合法目标 = {Awake, SelfDisabling} (2 个)
#[test]
fn negative_recovering_legal_targets_invariant() {
    let legal = legal_targets(CognitiveDreamState::Recovering);
    assert_eq!(legal.len(), 2);
    assert!(legal.contains(&CognitiveDreamState::Awake));
    assert!(legal.contains(&CognitiveDreamState::SelfDisabling));
}

// ============================================
// 负向 3: Awake 状态 — 只可去 Reflecting/SelfDisabling
// ============================================

/// 负向 3.1: Awake → Dreaming 直接 = 非法 (必须经 Reflecting)
#[test]
fn negative_awake_cannot_skip_to_dreaming() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-6");
    assert_eq!(m.current, CognitiveDreamState::Awake);
    let r = m.transition(CognitiveDreamState::Dreaming, TransitionReason::Internal);
    assert!(r.is_err(), "Awake → Dreaming 必须经 Reflecting, 直接跳非法");
    assert_eq!(m.current, CognitiveDreamState::Awake);
}

/// 负向 3.2: Awake → Meditating 直接 = 非法
#[test]
fn negative_awake_cannot_skip_to_meditating() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-7");
    let r = m.transition(CognitiveDreamState::Meditating, TransitionReason::Internal);
    assert!(r.is_err());
    assert_eq!(m.current, CognitiveDreamState::Awake);
}

/// 负向 3.3: Awake → Recovering 直接 = 非法 (必须经停机)
#[test]
fn negative_awake_cannot_skip_to_recovering() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-8");
    let r = m.transition(CognitiveDreamState::Recovering, TransitionReason::Internal);
    assert!(r.is_err());
    assert_eq!(m.current, CognitiveDreamState::Awake);
}

// ============================================
// 负向 4: 非法转换 = Err 而非 panic (主 17:58 不假装)
// ============================================

/// 负向 4.1: 非法转换必须返回 ConsciousnessError::IllegalTransition, 不允许 panic
#[test]
fn negative_illegal_transition_returns_err_not_panic() {
    // 各种非法转换组合
    let illegal = [
        (CognitiveDreamState::Awake, CognitiveDreamState::Dreaming),
        (CognitiveDreamState::Awake, CognitiveDreamState::Recovering),
        (
            CognitiveDreamState::SelfDisabling,
            CognitiveDreamState::Awake,
        ),
        (
            CognitiveDreamState::SelfDisabling,
            CognitiveDreamState::Dreaming,
        ),
        (
            CognitiveDreamState::Recovering,
            CognitiveDreamState::Dreaming,
        ),
        (CognitiveDreamState::Dreaming, CognitiveDreamState::Awake),
    ];
    for (from, to) in illegal {
        let mut mm = CognitiveDreamStateMachine::with_initial(format!("cid-{from:?}-{to:?}"), from);
        let r = mm.transition(to, TransitionReason::Internal);
        assert!(
            r.is_err(),
            "非法转换 {:?} → {:?} 必须返回 Err, 实际 Ok",
            from,
            to
        );
        match r.unwrap_err() {
            ConsciousnessError::IllegalTransition { .. } => { /* 预期 */ }
            other => panic!(
                "预期 IllegalTransition, 实际 {:?} (from={:?} to={:?})",
                other, from, to
            ),
        }
    }
}

// ============================================
// 负向 5: 转换历史完整性 — 每次合法转换必须记录
// ============================================

/// 负向 5.1: history 不能回滚 (Vec 只能 append, 不暴露 clear 入口)
#[test]
fn negative_history_grows_monotonically_no_clear() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-10");
    assert_eq!(m.transition_count(), 0);

    m.enter_reflecting().unwrap();
    assert_eq!(m.transition_count(), 1);

    // 尝试非法转换 — history 不增加
    let r = m.transition(CognitiveDreamState::Recovering, TransitionReason::Internal);
    assert!(r.is_err());
    assert_eq!(
        m.transition_count(),
        1,
        "非法转换不应写入 history (audit 完整性)"
    );

    m.enter_dreaming().unwrap();
    assert_eq!(m.transition_count(), 2);

    // 没有 public fn clear_history — 编译期保证 history 不可清除
    // (验证: CognitiveDreamStateMachine 没有提供 clear_history 方法)
    // 这里只验证 m.transition_count() 持续累加
}

/// 负向 5.2: 转换历史 — 每次合法转换 reason 字段必须保留
#[test]
fn negative_history_retains_reason() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-11");
    m.enter_self_disabling().unwrap();
    // 找到最近一次 history
    let last = m.history.last().expect("history 应有 1 条");
    assert_eq!(last.from, CognitiveDreamState::Awake);
    assert_eq!(last.to, CognitiveDreamState::SelfDisabling);
    assert_eq!(last.reason, TransitionReason::L0HaEmergency);
}

// ============================================
// 负向 6: 6 状态 + 15 条合法转换 + 21 条非法转换矩阵
// ============================================

/// 负向 6.1: 完整 6×6 转换矩阵 — 合法转换总数 = 15
#[test]
fn negative_full_transition_matrix_count() {
    // 合法: Awake(2) + Reflecting(4) + Dreaming(3) + Meditating(3) + SelfDisabling(1) + Recovering(2) = 15
    let mut total_legal = 0;
    let mut total_illegal = 0;
    for &from in &CognitiveDreamState::ALL {
        for &to in &CognitiveDreamState::ALL {
            if from == to {
                continue; // 自环不计
            }
            if can_transition(from, to) {
                total_legal += 1;
            } else {
                total_illegal += 1;
            }
        }
    }
    assert_eq!(total_legal, 15, "合法转换总数应为 15");
    // 6×6 - 6 (自环) = 30 总转换, 15 合法 + 15 非法
    assert_eq!(total_illegal, 15, "非法转换总数应为 15");
}

/// 负向 6.2: 6 状态 ALL 常量不变性
#[test]
fn negative_all_states_constant_invariant() {
    assert_eq!(CognitiveDreamState::ALL.len(), 6);
    // 6 个状态名全部唯一
    for (i, a) in CognitiveDreamState::ALL.iter().enumerate() {
        for (j, b) in CognitiveDreamState::ALL.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "ALL 常量含重复: {} == {}", i, j);
            }
        }
    }
    // 每个状态都有非空 semantic_name
    for s in CognitiveDreamState::ALL {
        assert!(!s.semantic_name().is_empty());
        assert!(!s.describe().is_empty());
    }
}

// ============================================
// 负向 7: L0 HA 紧急停 — 任意非 SelfDisabling → SelfDisabling = 合法
// ============================================

/// 负向 7.1: L0 HA 紧急停可从任何状态触发
#[test]
fn negative_l0ha_emergency_from_any_state_is_legal() {
    // 4 个非 SelfDisabling 状态都应能进入 SelfDisabling
    for &from in &[
        CognitiveDreamState::Awake,
        CognitiveDreamState::Reflecting,
        CognitiveDreamState::Dreaming,
        CognitiveDreamState::Meditating,
        CognitiveDreamState::Recovering,
    ] {
        assert!(
            can_transition(from, CognitiveDreamState::SelfDisabling),
            "{:?} → SelfDisabling 必须合法 (L0 HA 紧急停)",
            from
        );
    }
}

/// 负向 7.2: 紧急停 reason = L0HaEmergency 必须被正确记录
#[test]
fn negative_l0ha_emergency_reason_recorded() {
    let mut m = CognitiveDreamStateMachine::new("cid-v13-12");
    m.enter_self_disabling().unwrap();
    let last = m.history.last().unwrap();
    assert_eq!(last.reason, TransitionReason::L0HaEmergency);
    // 验证 timestamp 字段已设置 (DateTime<Utc>)
    let _: chrono::DateTime<chrono::Utc> = last.at;
}
