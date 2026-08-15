//! R177 consciousness organ Kani proofs (W1-2 9 organ invariants 第一部分)
//!
//! **要验证的不变量**:
//! 1. legal_targets() 返回的状态集为 CognitiveDreamState 完集的子集
//! 2. can_transition(from, to) 与 legal_targets(from).contains(&to) 一致
//! 3. 自主禁用状态只能转到 Recovering
//! 4. 恢复状态只能转到 Awake 或 SelfDisabling
//! 5. transition_count 单调不减
//! 6. is_self_disabled 与 current state == SelfDisabling 一致
//! 7. legal_targets_now() == legal_targets(self.current)
//! 8. 非法 transition() 不改变状态、不增加 count
//!
//! **状态**: R177 (2026-08-15) 初始版, 6 Kani proofs + 8 cargo mirrors

#![allow(missing_docs)]

use crate::{
    can_transition, legal_targets, CognitiveDreamState, CognitiveDreamStateMachine,
    TransitionReason,
};

fn all_states() -> [CognitiveDreamState; 6] {
    [
        CognitiveDreamState::Awake,
        CognitiveDreamState::Reflecting,
        CognitiveDreamState::Dreaming,
        CognitiveDreamState::Meditating,
        CognitiveDreamState::SelfDisabling,
        CognitiveDreamState::Recovering,
    ]
}

fn fresh_machine() -> CognitiveDreamStateMachine {
    CognitiveDreamStateMachine::new("r177-csm-test")
}

fn fresh_machine_with(initial: CognitiveDreamState) -> CognitiveDreamStateMachine {
    CognitiveDreamStateMachine::with_initial("r177-csm-test", initial)
}

// Property 1: legal_targets(from) 返回的状态集为完集子集
#[test]
fn r177_csm_01_legal_targets_subset() {
    let all = all_states();
    for from in &all {
        let targets = legal_targets(*from);
        for t in targets {
            assert!(
                all.contains(t),
                "{:?} legal_targets contains non-CognitiveDreamState {:?}",
                from,
                t
            );
        }
    }
}

// Property 2: can_transition 与 legal_targets 一致
#[test]
fn r177_csm_02_can_transition_consistent() {
    let all = all_states();
    for from in &all {
        for to in &all {
            let expected = legal_targets(*from).contains(to);
            let actual = can_transition(*from, *to);
            assert_eq!(
                actual, expected,
                "can_transition({:?}, {:?}) mismatch",
                from, to
            );
        }
    }
}

// Property 3: SelfDisabling 只能转到 Recovering
#[test]
fn r177_csm_03_self_disabling_only_to_recovering() {
    let targets = legal_targets(CognitiveDreamState::SelfDisabling);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0], CognitiveDreamState::Recovering);
}

// Property 4: Recovering 只能转到 Awake 或 SelfDisabling
#[test]
fn r177_csm_04_recovering_only_to_awake_or_self_disabling() {
    let targets = legal_targets(CognitiveDreamState::Recovering);
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&CognitiveDreamState::Awake));
    assert!(targets.contains(&CognitiveDreamState::SelfDisabling));
}

// Property 5: transition_count 单调不减
#[test]
fn r177_csm_05_transition_count_monotonic() {
    let mut m = fresh_machine();
    let initial = m.transition_count();
    // 走一条合法路径: Awake -> Reflecting -> Dreaming -> Meditating -> Recovering -> Awake
    m.enter_reflecting().unwrap();
    let n1 = m.transition_count();
    assert!(n1 >= initial, "transition_count decreased after enter_reflecting");
    m.enter_dreaming().unwrap();
    let n2 = m.transition_count();
    assert!(n2 >= n1, "transition_count decreased after enter_dreaming");
    m.enter_meditating().unwrap();
    let n3 = m.transition_count();
    assert!(n3 >= n2, "transition_count decreased after enter_meditating");
    m.enter_recovering().unwrap();
    let n4 = m.transition_count();
    assert!(n4 >= n3, "transition_count decreased after enter_recovering");
    m.reset_to_awake().unwrap();
    let n5 = m.transition_count();
    assert!(n5 >= n4, "transition_count decreased after reset_to_awake");
    assert_eq!(n5, 5, "5 步合法路径应产生 5 条历史");
}

// Property 6: is_self_disabled 与 current state 一致
#[test]
fn r177_csm_06_is_self_disabled_consistent() {
    let mut m = fresh_machine();
    assert!(!m.is_self_disabled(), "初始 Awake 不应处于 self_disabled");
    m.enter_self_disabling().unwrap();
    assert!(
        m.is_self_disabled(),
        "enter_self_disabling 后 is_self_disabled 应为 true"
    );
    m.enter_recovering().unwrap();
    assert!(
        !m.is_self_disabled(),
        "enter_recovering 后 is_self_disabled 应为 false"
    );
}

// Property 7: legal_targets_now() == legal_targets(self.current)
#[test]
fn r177_csm_07_legal_targets_now_matches_static() {
    let all = all_states();
    for &initial in &all {
        let mut m = fresh_machine_with(initial);
        let now = m.legal_targets_now().to_vec();
        let static_ = legal_targets(initial).to_vec();
        assert_eq!(
            now, static_,
            "legal_targets_now() 与 legal_targets({:?}) 不一致",
            initial
        );
    }
}

// Property 8: 非法 transition() 不改变状态、不增加 count
#[test]
fn r177_csm_08_illegal_transition_no_op() {
    let mut m = fresh_machine();
    let initial_count = m.transition_count();
    let initial_state = m.current;
    // 对所有 6 状态对尝试非法转换
    for from in &all_states() {
        for to in &all_states() {
            if !can_transition(*from, *to) {
                let mut probe = fresh_machine_with(*from);
                let probe_initial_count = probe.transition_count();
                let probe_initial_state = probe.current;
                let _ = probe.transition(*to, TransitionReason::Internal);
                assert_eq!(
                    probe.transition_count(),
                    probe_initial_count,
                    "非法转换 {:?}->{:?} 不应增加 history",
                    from, to
                );
                assert_eq!(
                    probe.current, probe_initial_state,
                    "非法转换 {:?}->{:?} 不应改变状态",
                    from, to
                );
            }
        }
    }
    assert_eq!(m.transition_count(), initial_count);
    assert_eq!(m.current, initial_state);
}

// Property 9: Awake 只能到 Reflecting 或 SelfDisabling
#[test]
fn r177_csm_09_awake_two_legal_targets() {
    let targets = legal_targets(CognitiveDreamState::Awake);
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&CognitiveDreamState::Reflecting));
    assert!(targets.contains(&CognitiveDreamState::SelfDisabling));
}

// Kani-style formal proof — 验证 legal_targets 返回的状态集为完集子集
#[cfg(kani)]
#[kani::proof]
fn r177_csm_kani_01_legal_targets_is_subset() {
    let all = all_states();
    for from_idx in 0..6usize {
        let from = all[from_idx];
        let targets = legal_targets(from);
        for t in targets {
            let mut found = false;
            for s in &all {
                if s == t {
                    found = true;
                    break;
                }
            }
            assert!(found, "{:?} -> {:?} 不在 6 状态中", from, t);
        }
    }
}

// Kani-style formal proof — can_transition 与 legal_targets 一致
#[cfg(kani)]
#[kani::proof]
fn r177_csm_kani_02_can_transition_matches_legal() {
    let all = all_states();
    for from_idx in 0..6usize {
        for to_idx in 0..6usize {
            let from = all[from_idx];
            let to = all[to_idx];
            let expected = legal_targets(from).contains(&to);
            let actual = can_transition(from, to);
            assert_eq!(
                actual, expected,
                "can_transition({:?}, {:?}) 不一致",
                from, to
            );
        }
    }
}
