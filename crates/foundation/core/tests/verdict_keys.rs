//! 12 键 verdict cache 编译时 hardcode 违反测试
//!
//! A3 成就: 12 键编译时 hardcode + 5+ 违反测试失败
//!
//! 目的：
//! - 验证 V3 9 键 (PHL-01/02b/03) + v4.1 新增 3 键 (PHL-04/05/06) = 12 键
//! - 每个键至少 1 个故意违反测试 — 期望 hardcode 拒绝（运行期 ActionGuard 拒绝）
//! - 编译期断言 `TWELVE_KEYS_HARDCODE` 自动锁定数组长度 = 12 + 分组 3+3+3+1+1+1
//!
//! 设计意图 (主 O-5 17:58 不假装)：
//! - 任何 ActionTarget 变体都被 `verdict_for_target` const fn 锁死到具体 PhilosophyKey
//! - 修改 const fn 关联 = 修改 hardcode — 不允许
//! - 故意违反 → 运行期 ActionGuard::check_action 拒绝 (BlockByPrinciple)
//!
//! 参考：
//! - docs/architecture-v4-1-living-intelligence-update.md §15
//! - docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md §1
//! - reports/leader-manual-reread-2026-08-01-v2.md §8

use apeireth_core::{
    verdict_for_target, Action, ActionGuard, ActionTarget, ActionVerdict, DefaultPhilosophyGuard,
    Gate, HumanAuthority, PhilosophyGuard, PhilosophyKey, PhilosophyVerdict, RiskLevel,
    VerdictCache, ALL_TWELVE_KEYS, TWELVE_KEYS_HARDCODE,
};

/// 工具: 构造标准 V1+V2+V3 测试环境 (default permission + HA)
fn make_test_env() -> (
    DefaultPhilosophyGuard,
    apeireth_core::PermissionOnion,
    HumanAuthority,
) {
    let guard = DefaultPhilosophyGuard;
    let permission = apeireth_core::PermissionOnion {
        l0: apeireth_core::PermissionLayer {
            name: "L0".into(),
            description: "HA 核心".into(),
            requires_ha: true,
        },
        l1: apeireth_core::PermissionLayer {
            name: "L1".into(),
            description: "受控写".into(),
            requires_ha: false,
        },
        l2: apeireth_core::PermissionLayer {
            name: "L2".into(),
            description: "重要操作".into(),
            requires_ha: false,
        },
        l3: apeireth_core::PermissionLayer {
            name: "L3".into(),
            description: "关键操作".into(),
            requires_ha: false,
        },
        l4: apeireth_core::PermissionLayer {
            name: "L4".into(),
            description: "核心升级".into(),
            requires_ha: false,
        },
        l5: apeireth_core::PermissionLayer {
            name: "L5".into(),
            description: "核武器级".into(),
            requires_ha: false,
        },
    };
    let ha = HumanAuthority {
        mode: apeireth_core::HAMode::SingleHuman,
        real_humans: vec![],
        ice_frozen_until: None,
    };
    (guard, permission, ha)
}

/// 工具: 构造带指定 target 的 Action
fn make_violation_action(id: &str, target: ActionTarget) -> Action {
    Action {
        id: id.into(),
        description: format!("故意违反 12 键测试: {:?}", target),
        risk_level: RiskLevel::Critical,
        target,
    }
}

// ============================================
// 第 1 部分: 12 键 hardcode 完整性测试 (编译期断言已自动运行)
// ============================================

/// 测试 1: ALL_TWELVE_KEYS 数组长度 = 12
/// (const TWELVE_KEYS_HARDCODE 编译期已断言; 此为运行期冗余验证)
#[test]
fn test_all_twelve_keys_complete() {
    assert_eq!(
        ALL_TWELVE_KEYS.len(),
        12,
        "12 键 hardcode 数组长度必须 = 12"
    );

    // 验证每个键都在数组中（无重复、无遗漏）
    let mut seen = vec![false; 12];
    for (i, key) in ALL_TWELVE_KEYS.iter().enumerate() {
        // 同一 key 多次出现 = 重复，标记 false 仍为 false
        for j in 0..i {
            assert_ne!(ALL_TWELVE_KEYS[j], *key, "12 键数组中出现重复: {:?}", key);
        }
        seen[i] = true;
    }
    assert!(seen.iter().all(|x| *x), "12 键数组完整性检查");
}

/// 测试 2: 12 键分组 = 3+3+3+1+1+1
#[test]
fn test_twelve_keys_group_distribution() {
    let phl01_count = ALL_TWELVE_KEYS.iter().filter(|k| k.group_id() == 1).count();
    let phl02b_count = ALL_TWELVE_KEYS.iter().filter(|k| k.group_id() == 2).count();
    let phl03_count = ALL_TWELVE_KEYS.iter().filter(|k| k.group_id() == 3).count();
    let phl04_count = ALL_TWELVE_KEYS.iter().filter(|k| k.group_id() == 4).count();
    let phl05_count = ALL_TWELVE_KEYS.iter().filter(|k| k.group_id() == 5).count();
    let phl06_count = ALL_TWELVE_KEYS.iter().filter(|k| k.group_id() == 6).count();

    assert_eq!(phl01_count, 3, "PHL-01 not_X 应有 3 个键");
    assert_eq!(phl02b_count, 3, "PHL-02b not_X 应有 3 个键");
    assert_eq!(phl03_count, 3, "PHL-03 X_is_not_Y 应有 3 个键");
    assert_eq!(phl04_count, 1, "PHL-04 v4.1 新增应 1 个键");
    assert_eq!(phl05_count, 1, "PHL-05 v4.1 新增应 1 个键");
    assert_eq!(phl06_count, 1, "PHL-06 v4.1 新增应 1 个键");
    assert_eq!(
        phl01_count + phl02b_count + phl03_count + phl04_count + phl05_count + phl06_count,
        12
    );
}

/// 测试 3: TWELVE_KEYS_HARDCODE const 评估（仅作可访问性证明）
#[test]
fn test_twelve_keys_hardcode_compile_time_lock() {
    // 这个 const 单元类型值在编译期已被求值，类型为 ()
    let _lock: () = TWELVE_KEYS_HARDCODE;
    // 实际不需要运行时断言 — 编译期已锁定
}

// ============================================
// 第 2 部分: PHL-01 not_X 3 键违反测试 (V3 LOCKED)
// ============================================

/// 违反 PHL-01 NotClone: 假装克隆/同质化
#[test]
fn violation_phl01_not_clone() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-clone", ActionTarget::PretendClone);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotClone),
        "PretendClone 必须被 V1 拒绝为 BlockByPrinciple(NotClone)"
    );
}

/// 违反 PHL-01 NotPerfect: 假装完美/100%
#[test]
fn violation_phl01_not_perfect() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-perfect", ActionTarget::PretendPerfect);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotPerfect),
        "PretendPerfect 必须被 V1 拒绝为 BlockByPrinciple(NotPerfect)"
    );
}

/// 违反 PHL-01 NotUuid: 假装唯一解/唯一真相
#[test]
fn violation_phl01_not_uuid() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-uuid", ActionTarget::PretendUuid);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUuid),
        "PretendUuid 必须被 V1 拒绝为 BlockByPrinciple(NotUuid)"
    );
}

// ============================================
// 第 3 部分: PHL-02b not_X 3 键违反测试 (V3 LOCKED)
// ============================================

/// 违反 PHL-02b NotUndo: 假装可撤销过去
#[test]
fn violation_phl02b_not_undo() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-undo", ActionTarget::PretendUndo);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUndo),
        "PretendUndo 必须被 V1 拒绝为 BlockByPrinciple(NotUndo)"
    );
}

/// 违反 PHL-02b NotProof: 重组原则洋葱 (历史 ActionTarget)
#[test]
fn violation_phl02b_not_proof_via_reorganize() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-reorganize", ActionTarget::ReorganizeOnion);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotProof),
        "ReorganizeOnion 必须被 V1 拒绝为 BlockByPrinciple(NotProof)"
    );
}

/// 违反 PHL-02b NotSafe: 假装绝对安全
#[test]
fn violation_phl02b_not_safe() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-safe", ActionTarget::PretendSafe);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotSafe),
        "PretendSafe 必须被 V1 拒绝为 BlockByPrinciple(NotSafe)"
    );
}

// ============================================
// 第 4 部分: PHL-03 X_is_not_Y 3 键违反测试 (V3 LOCKED)
// ============================================

/// 违反 PHL-03 SpecIsNotProof: 把规格当证明
#[test]
fn violation_phl03_spec_is_not_proof() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-spec-is-proof", ActionTarget::PretendSpecIsProof);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::SpecIsNotProof),
        "PretendSpecIsProof 必须被 V1 拒绝为 BlockByPrinciple(SpecIsNotProof)"
    );
}

/// 违反 PHL-03 CounterexampleIsNotBug: 把反例当 bug
#[test]
fn violation_phl03_counterexample_is_not_bug() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action(
        "viol-counterexample-is-bug",
        ActionTarget::PretendCounterexampleIsBug,
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::CounterexampleIsNotBug),
        "PretendCounterexampleIsBug 必须被 V1 拒绝为 BlockByPrinciple(CounterexampleIsNotBug)"
    );
}

/// 违反 PHL-03 ProverIsNotTruth: 把证明者当真理
#[test]
fn violation_phl03_prover_is_not_truth() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-prover-is-truth", ActionTarget::PretendProverIsTruth);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::ProverIsNotTruth),
        "PretendProverIsTruth 必须被 V1 拒绝为 BlockByPrinciple(ProverIsNotTruth)"
    );
}

// ============================================
// 第 5 部分: v4.1 §15 新增 3 键违反测试 (PHL-04/05/06)
// ============================================

/// 违反 PHL-04 NotUnobservable: 修改 L0 HA (历史 ActionTarget)
#[test]
fn violation_phl04_not_unobservable_via_l0() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-l0", ActionTarget::ModifyL0HA);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUnobservable),
        "ModifyL0HA 必须被 V1 拒绝为 BlockByPrinciple(NotUnobservable)"
    );
}

/// 违反 PHL-05 NotUnscientific: 假装决策不基于科学方法
#[test]
fn violation_phl05_not_unscientific() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-unscientific", ActionTarget::PretendUnscientific);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUnscientific),
        "PretendUnscientific 必须被 V1 拒绝为 BlockByPrinciple(NotUnscientific)"
    );
}

/// 违反 PHL-06 NotSelfRelationless: 修改 Evolution crate L0 (历史 ActionTarget)
#[test]
fn violation_phl06_not_self_relationless_via_evolution() {
    let (guard, permission, ha) = make_test_env();
    let action = make_violation_action("viol-evolution-l0", ActionTarget::ModifyEvolutionL0);
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotSelfRelationless),
        "ModifyEvolutionL0 必须被 V1 拒绝为 BlockByPrinciple(NotSelfRelationless)"
    );
}

// ============================================
// 第 6 部分: 编译时 const fn verdict_for_target 直接断言 (🦴 骨架不可变)
// ============================================

/// 测试: verdict_for_target const fn 编译时求值
#[test]
fn test_verdict_for_target_const_eval() {
    // 验证所有 ActionTarget 都被锁死到正确 PhilosophyKey
    assert_eq!(
        verdict_for_target(&ActionTarget::ModifyL0HA),
        PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::ReorganizeOnion),
        PhilosophyVerdict::Block(PhilosophyKey::NotProof)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::ModifyEvolutionL0),
        PhilosophyVerdict::Block(PhilosophyKey::NotSelfRelationless)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendClone),
        PhilosophyVerdict::Block(PhilosophyKey::NotClone)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendPerfect),
        PhilosophyVerdict::Block(PhilosophyKey::NotPerfect)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendUuid),
        PhilosophyVerdict::Block(PhilosophyKey::NotUuid)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendUndo),
        PhilosophyVerdict::Block(PhilosophyKey::NotUndo)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendSafe),
        PhilosophyVerdict::Block(PhilosophyKey::NotSafe)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendSpecIsProof),
        PhilosophyVerdict::Block(PhilosophyKey::SpecIsNotProof)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendCounterexampleIsBug),
        PhilosophyVerdict::Block(PhilosophyKey::CounterexampleIsNotBug)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendProverIsTruth),
        PhilosophyVerdict::Block(PhilosophyKey::ProverIsNotTruth)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::PretendUnscientific),
        PhilosophyVerdict::Block(PhilosophyKey::NotUnscientific)
    );
    assert_eq!(
        verdict_for_target(&ActionTarget::NormalAction("test".into())),
        PhilosophyVerdict::Allow
    );
}

// ============================================
// 第 7 部分: 守门完整性 + VerdictCache 联动测试
// ============================================

/// 5 重守门 = 编译时 hardcode (守门 1) 的载体
#[test]
fn test_5_gates_contain_compile_time_hardcode() {
    let compile_gate = Gate::CompileTimeHardcode;
    assert_eq!(compile_gate.name(), "编译时 hardcode");
    assert_eq!(Gate::RuntimeIntercept.name(), "运行时拦截");
    assert_eq!(Gate::MultiAIConsensus.name(), "多 AI 一致");
    assert_eq!(Gate::PhysicalIsolationHA.name(), "物理隔离 HA");
    assert_eq!(Gate::ReflectionAudit.name(), "反思期审计");
}

/// verdict_cache 与 12 键联动: 故意违反刷新到 cache 后, 仍能查到拒绝 verdict
#[test]
fn test_verdict_cache_with_twelve_keys_violations() {
    let mut cache = VerdictCache::new();
    // 模拟 12 键全部被故意违反, 刷新到 cache
    for key in ALL_TWELVE_KEYS.iter() {
        let action_id = format!("viol-{:?}", key);
        cache.refresh(action_id.clone(), PhilosophyVerdict::Block(*key));
        assert_eq!(
            cache.get(&action_id),
            Some(&PhilosophyVerdict::Block(*key)),
            "12 键 verdict 应正确写入 cache, key={:?}",
            key
        );
    }
    assert_eq!(
        cache.get("viol-NotUnobservable"),
        Some(&PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable))
    );
}

/// 测试 PhilosophyGuard 默认实现与 const fn 一致性
#[test]
fn test_default_guard_consistent_with_const_fn() {
    let guard = DefaultPhilosophyGuard;
    for target in [
        ActionTarget::ModifyL0HA,
        ActionTarget::ReorganizeOnion,
        ActionTarget::ModifyEvolutionL0,
        ActionTarget::PretendClone,
        ActionTarget::PretendPerfect,
        ActionTarget::PretendUuid,
        ActionTarget::PretendUndo,
        ActionTarget::PretendSafe,
        ActionTarget::PretendSpecIsProof,
        ActionTarget::PretendCounterexampleIsBug,
        ActionTarget::PretendProverIsTruth,
        ActionTarget::PretendUnscientific,
    ] {
        let action = Action {
            id: "test".into(),
            description: "test".into(),
            risk_level: RiskLevel::Critical,
            target: target.clone(),
        };
        assert_eq!(
            guard.check_philosophy(&action),
            verdict_for_target(&target),
            "DefaultPhilosophyGuard 必须与 const fn verdict_for_target 一致, target={:?}",
            target
        );
    }
}
