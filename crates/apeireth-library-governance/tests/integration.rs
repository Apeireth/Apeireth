//! Library Stage 5 治理 — 集成测试 (8 通道).
//!
//! **8 通道**:
//! 1. strategy 模块: clap derive 模式 5 策略 + 3 行动 + 决策树
//! 2. verification 模块: Kani POD 模型 6 invariant + 6 harness + 8 boundary
//! 3. consistency 模块: Kani proofs 模板 5 check + 5 API lock + 编译期 hardcode
//! 4. invariants 模块: 6 Stage 5 不变量 + sanity_check
//! 5. lib 入口: run_all + verify + GovernanceEngine + evaluate
//! 6. 跨模块集成: strategy + verification + consistency 联合
//! 7. API 锁定: 5 编译期 hardcode 严守
//! 8. 0 越界 8 硬墙: 8 通道全 Pass

use apeireth_library_governance::consistency::{
    api_lock, checks, tokens_locked, ANCHOR_COUNT, BASELINE_VALUE_1_X1000, BASELINE_VALUE_2_X1000,
    BASELINE_VALUE_3_X1000, GATE_LAYERS, LOCKED_CRATE_COUNT, WORKSPACE_VERSION_MAJOR,
    WORKSPACE_VERSION_MINOR,
};
use apeireth_library_governance::proof_harnesses;
use apeireth_library_governance::strategy::{required_tokens_present, REQUIRED_TOKEN_COUNT};
use apeireth_library_governance::verification::invariants as ver_inv;
use apeireth_library_governance::{
    evaluate, run_all, run_all_8_harnesses, run_all_formal_proofs, verify, Boundary, CheckStatus,
    ConsistencyReport, DecisionTree, GovernanceAction, GovernanceContext, GovernanceDecision,
    GovernanceEngine, Invariant, LockedSignature, PolicyKind, ProofHarness, ProofKind, ProofReport,
    ProofResult, ProofRunner, Stage5Token, VerificationSubject,
};

#[test]
fn integration_strategy_dispatch_5_known_allow() {
    // clap derive 模式: 5 已知策略全 Allow
    let known = [
        GovernanceContext::version(),
        GovernanceContext::baseline(),
        GovernanceContext::locked(0),
        GovernanceContext::locked(12),
        GovernanceContext::locked(23),
        GovernanceContext::anchor(),
        GovernanceContext::gate(),
    ];
    for ctx in &known {
        let action = DecisionTree::from_context(ctx).action();
        assert_eq!(action, GovernanceAction::Allow, "ctx.policy={}", ctx.policy);
    }
}

#[test]
fn integration_verification_6_invariants_pass() {
    // Kani POD 模型: 6 invariant 全过
    let s = VerificationSubject::safe_default();
    assert!(ver_inv::run_all(&s));
}

#[test]
fn integration_consistency_5_checks_pass() {
    // Kani proofs 模板: 5 check 全过
    let r = ConsistencyReport::check();
    assert!(r.is_ok());
    assert_eq!(r.pass_count(), 5);
}

#[test]
fn integration_6_stage_5_invariants_pass() {
    // Stage 5 invariants 模块
    assert!(apeireth_library_governance::invariants::run_all());
    assert!(apeireth_library_governance::invariants::sanity_check());
}

#[test]
fn integration_lib_entry_evaluate_verify_run_all() {
    // lib 入口: evaluate + verify + run_all
    // safe_default policy=0 (Version) value=0 → 决策树 action=Audit, requires_audit=true (0 装严守 正确行为)
    let decision = evaluate(&GovernanceContext::safe_default());
    assert_eq!(decision.action, GovernanceAction::Audit);
    assert!(decision.requires_audit);

    let engine = GovernanceEngine::new();
    assert!(engine.verify());

    assert!(run_all());
    verify();
}

#[test]
fn integration_cross_module_strategy_verification_consistency() {
    // 跨模块: strategy.decision + verification.invariant + consistency.check
    let ctx = GovernanceContext::version();
    let decision = DecisionTree::from_context(&ctx).dispatch();
    assert_eq!(decision.policy, PolicyKind::Version);
    assert_eq!(decision.action, GovernanceAction::Allow);

    let s = VerificationSubject::safe_default();
    assert!(ver_inv::version_major_is_one(&s));
    assert!(ver_inv::version_minor_is_two(&s));

    let r = ConsistencyReport::check();
    assert!(matches!(r.version_locked, CheckStatus::Pass));
}

#[test]
fn integration_api_lock_5_compile_time_hardcodes() {
    // 编译期 hardcode 5 API lock 全 hold
    assert!(api_lock::all_locks_hold());
    assert!(api_lock::version_lock_holds());
    assert!(api_lock::baseline_lock_holds());
    assert!(api_lock::locked_count_lock_holds());
    assert!(api_lock::anchor_lock_holds());
    assert!(api_lock::gate_lock_holds());
}

#[test]
fn integration_zero_violation_8_hard_walls() {
    // 8 硬墙 0 越界: B2 / A1 / B1 / B5 / B4 + boundary × 3
    // B2: workspace.version 1.2.0
    assert_eq!(WORKSPACE_VERSION_MAJOR, 1);
    assert_eq!(WORKSPACE_VERSION_MINOR, 2);
    assert!(Boundary::VersionMajor.check(WORKSPACE_VERSION_MAJOR));
    assert!(Boundary::VersionMinor.check(WORKSPACE_VERSION_MINOR));

    // A1: R11 baseline 3 值
    assert_eq!(BASELINE_VALUE_1_X1000, 868);
    assert_eq!(BASELINE_VALUE_2_X1000, 853);
    assert_eq!(BASELINE_VALUE_3_X1000, 906);
    assert!(Boundary::BaselineIndex.check(0));

    // B1: 24 LOCKED crate
    assert_eq!(LOCKED_CRATE_COUNT, 24);
    for i in 0u8..=23 {
        assert!(Boundary::LockedIndex.check(i));
    }

    // B5: 8 哲学锚
    assert_eq!(ANCHOR_COUNT, 8);
    assert!(Boundary::AnchorCount.check(ANCHOR_COUNT));

    // B4: 6 重守门 v7
    assert_eq!(GATE_LAYERS, 6);
    assert!(Boundary::GateLayers.check(GATE_LAYERS));

    // B3: 30 维
    assert!(Boundary::DimCount.check(30));

    // A3: 13 键
    assert!(Boundary::KeyCount.check(13));

    // Tokens
    assert!(tokens_locked());
    assert!(required_tokens_present());
    assert_eq!(REQUIRED_TOKEN_COUNT, 2);
}

#[test]
fn integration_decision_round_trip_via_evaluate() {
    // 完整 round-trip: context → evaluate → decision → action
    let cases = [
        (
            GovernanceContext::version(),
            PolicyKind::Version,
            GovernanceAction::Allow,
        ),
        (
            GovernanceContext::baseline(),
            PolicyKind::Baseline,
            GovernanceAction::Allow,
        ),
        (
            GovernanceContext::anchor(),
            PolicyKind::Anchor,
            GovernanceAction::Allow,
        ),
        (
            GovernanceContext::gate(),
            PolicyKind::Gate,
            GovernanceAction::Allow,
        ),
    ];
    for (ctx, expected_policy, expected_action) in &cases {
        let decision: GovernanceDecision = evaluate(ctx);
        assert_eq!(
            decision.policy, *expected_policy,
            "ctx.policy={}",
            ctx.policy
        );
        assert_eq!(decision.action, *expected_action);
    }
}

// =============================================================================
// Stage 5.1 形式化证明 cross-module 集成测试 (R127 P8-2, 深化 P5-2 治理)
// =============================================================================

#[test]
fn integration_formal_proof_run_all_8_harnesses_pass() {
    // Stage 5.1: 8 Kani-style proof harness 全过 (B2 / A1 / B1 / B5 / B4 + Stage5Token + LockedSignature)
    let results = run_all_8_harnesses();
    assert_eq!(results.len(), 8);
    for r in &results {
        assert!(r.is_success(), "harness failed: {:?}", r);
    }
    assert!(run_all_formal_proofs());
}

#[test]
fn integration_formal_proof_invariant_trait_for_verification_subject() {
    // Stage 5.1: Invariant trait impl for VerificationSubject (P5-2 verification POD 1:1 集成)
    let s = VerificationSubject::safe_default();
    assert!(s.is_safe(), "safe_default 整合 #4 commit 严守必须 is_safe");
}

#[test]
fn integration_formal_proof_invariant_trait_for_stage5_token() {
    // Stage 5.1: Invariant trait impl for Stage5Token (Kani MyDate 1:1)
    let t = Stage5Token::safe_default();
    assert!(t.is_safe());
    assert_eq!(t.version_major, WORKSPACE_VERSION_MAJOR);
    assert_eq!(t.version_minor, WORKSPACE_VERSION_MINOR);
    assert_eq!(t.anchor_count, ANCHOR_COUNT);
    assert_eq!(t.gate_layers, GATE_LAYERS);
}

#[test]
fn integration_formal_proof_invariant_trait_for_locked_signature() {
    // Stage 5.1: Invariant trait impl for LockedSignature (B1 1:1)
    for i in 0u8..24 {
        let sig = LockedSignature::try_new(i, true).unwrap();
        assert!(sig.is_safe(), "LOCKED[{}] 必须 is_safe", i);
    }
    assert_eq!(LockedSignature::TOTAL, LOCKED_CRATE_COUNT);
}

#[test]
fn integration_formal_proof_report_has_8_entries_all_pass() {
    // Stage 5.1: ProofReport 聚合 8 harness 结果, is_ok + pass_count = 8
    let report = apeireth_library_governance::run_all_as_report();
    assert_eq!(report.total(), 8);
    assert_eq!(report.pass_count(), 8);
    assert_eq!(report.fail_count(), 0);
    assert!(report.is_ok());
}

#[test]
fn integration_formal_proof_defensive_proof_macro() {
    // Stage 5.1: defensive_proof! 宏 runtime 强制断言 (Kani kani::assume 1:1)
    let x: u8 = 5;
    let r_ok = apeireth_library_governance::defensive_proof!("x_positive", x > 0);
    assert!(r_ok.is_success());
    let r_fail = apeireth_library_governance::defensive_proof!("x_positive", x > 100);
    assert!(r_fail.is_failure());
}

#[test]
fn integration_formal_proof_trivial_invariant_15_primitive_types() {
    // Stage 5.1: trivial_invariant! 宏 15 原生类型 impl (Kani 1:1)
    use apeireth_library_governance::formal_proof::Invariant as _;
    let v: u8 = 42;
    assert!(v.is_safe());
    let v: u16 = u16::MAX;
    assert!(v.is_safe());
    let v: u32 = u32::MAX;
    assert!(v.is_safe());
    let v: u64 = u64::MAX;
    assert!(v.is_safe());
    let v: u128 = u128::MAX;
    assert!(v.is_safe());
    let v: usize = usize::MAX;
    assert!(v.is_safe());
    let v: i8 = i8::MIN;
    assert!(v.is_safe());
    let v: i16 = i16::MIN;
    assert!(v.is_safe());
    let v: i32 = i32::MIN;
    assert!(v.is_safe());
    let v: i64 = i64::MIN;
    assert!(v.is_safe());
    let v: i128 = i128::MIN;
    assert!(v.is_safe());
    let v: isize = isize::MIN;
    assert!(v.is_safe());
    let v: () = ();
    assert!(v.is_safe());
    let v: bool = true;
    assert!(v.is_safe());
    let v: char = '🦀';
    assert!(v.is_safe());
}

#[test]
fn integration_formal_proof_proof_kind_serialization_3_variants() {
    // Stage 5.1: ProofKind 3 变体 Kani 序列化字符串 (#[kani::proof] / proof_for_contract / test)
    assert_eq!(ProofKind::Proof.as_str(), "#[kani::proof]");
    assert_eq!(
        ProofKind::ProofForContract.as_str(),
        "#[kani::proof_for_contract]"
    );
    assert_eq!(ProofKind::Test.as_str(), "#[test]");
}

#[test]
fn integration_formal_proof_proof_harness_metadata_count_8() {
    // Stage 5.1: ProofHarness::ALL 数组 = 8 (跟 run_all_8_harnesses 1:1)
    assert_eq!(proof_harnesses::ALL.len(), 8);
    for h in proof_harnesses::ALL.iter() {
        assert_eq!(h.kind, ProofKind::Proof);
        assert!(!h.should_panic);
        assert!(!h.name.is_empty());
    }
}

#[test]
fn integration_formal_proof_proof_runner_check_bool() {
    // Stage 5.1: ProofRunner::check(bool) → ProofResult 转换
    let r_pass = ProofRunner::new().check("h", true);
    assert!(r_pass.is_success());
    let r_fail = ProofRunner::new().check("h", false);
    assert!(r_fail.is_failure());
}

#[test]
fn integration_formal_proof_report_pass_fail_skipped_count() {
    // Stage 5.1: ProofReport 4 entries (2 pass + 1 fail + 1 skipped) 计数
    let mut r = ProofReport::new();
    r.record(ProofHarness::proof("a", "f", 1), ProofResult::Success);
    r.record(ProofHarness::proof("b", "f", 2), ProofResult::Success);
    r.record(
        ProofHarness::proof("c", "f", 3),
        ProofResult::Failure {
            harness: "c",
            message: "m",
        },
    );
    r.record(
        ProofHarness::test("d", "f", 4),
        ProofResult::Skipped { reason: "r" },
    );
    assert_eq!(r.total(), 4);
    assert_eq!(r.pass_count(), 2);
    assert_eq!(r.fail_count(), 1);
    assert_eq!(r.skipped_count(), 1);
    assert!(!r.is_ok());
}

// -------- Stage 5.1 × P5-2 跨模块联动 (form_proof ↔ strategy/verification/consistency) --------

#[test]
fn integration_formal_proof_cross_module_8_hard_walls_via_8_harnesses() {
    // Stage 5.1: 8 Kani-style harness 1:1 跟 8 硬墙对应 (B2 / A1 / B1 / B5 / B4 + 3 NEW)
    // 1. B2 workspace.version major (1)
    assert_eq!(proof_harnesses::ALL[0].name, "proof_version_major_is_one");
    // 2. B2 workspace.version minor (2)
    assert_eq!(proof_harnesses::ALL[1].name, "proof_version_minor_is_two");
    // 3. A1 R11 baseline index (0)
    assert_eq!(proof_harnesses::ALL[2].name, "proof_baseline_index_is_r11");
    // 4. B1 24 LOCKED signatures intact
    assert_eq!(
        proof_harnesses::ALL[3].name,
        "proof_locked_signatures_intact"
    );
    // 5. B5 8 哲学锚
    assert_eq!(proof_harnesses::ALL[4].name, "proof_anchor_count_is_eight");
    // 6. B4 6 重守门 v7
    assert_eq!(proof_harnesses::ALL[5].name, "proof_gate_layers_is_six");
    // 7. Stage 5.1 NEW: Stage5Token::is_safe (Kani MyDate 1:1)
    assert_eq!(
        proof_harnesses::ALL[6].name,
        "proof_stage5_token_safe_default_holds"
    );
    // 8. Stage 5.1 NEW: LockedSignature::is_safe (B1 1:1)
    assert_eq!(
        proof_harnesses::ALL[7].name,
        "proof_locked_signature_safe_default_holds"
    );
}

#[test]
fn integration_formal_proof_strategy_dispatch_x_invariant_safe_default() {
    // Stage 5.1 × P5-2 strategy 联动: 5 已知策略全 Allow 后, 对应 VerificationSubject safe_default 全 invariant is_safe
    let known_strategies = [
        GovernanceContext::version(),
        GovernanceContext::baseline(),
        GovernanceContext::locked(0),
        GovernanceContext::locked(12),
        GovernanceContext::locked(23),
        GovernanceContext::anchor(),
        GovernanceContext::gate(),
    ];
    for ctx in &known_strategies {
        // strategy: 5 已知策略全 Allow
        let action = DecisionTree::from_context(ctx).action();
        assert_eq!(action, GovernanceAction::Allow, "ctx.policy={}", ctx.policy);
        // verification: 6 invariant 全过
        assert!(ver_inv::run_all(&VerificationSubject::safe_default()));
        // formal_proof: 8 harness 全过
        assert!(run_all_formal_proofs());
    }
}

#[test]
fn integration_formal_proof_consistency_x_invariant_x_proof_all_8_aligned() {
    // Stage 5.1 × P5-2 联动: 3 大件验证 (consistency + invariants + formal_proof) 全 PASS
    // consistency: 5 check
    let cr = ConsistencyReport::check();
    assert!(cr.is_ok());
    assert_eq!(cr.pass_count(), 5);
    // invariants: 6 Stage 5 不变量
    assert!(apeireth_library_governance::invariants::run_all());
    // formal_proof: 8 Kani-style harness
    assert!(run_all_formal_proofs());
    // lib 入口
    assert!(run_all());
    verify();
}

#[test]
fn integration_formal_proof_token_construction_matches_compile_time_hardcodes() {
    // Stage 5.1: Stage5Token::try_new 用 编译期 hardcode (1, 2, 0, true, 8, 6) 严守值
    let t = Stage5Token::try_new(
        WORKSPACE_VERSION_MAJOR,
        WORKSPACE_VERSION_MINOR,
        0,            // baseline_index = 0 = R11
        true,         // B1 严守
        ANCHOR_COUNT, // 8 哲学锚
        GATE_LAYERS,  // 6 重 v7
    )
    .unwrap();
    assert!(t.is_safe());
    // 一致性: t 跟 VerificationSubject safe_default 1:1 对齐
    let s = VerificationSubject::safe_default();
    assert_eq!(t.version_major, s.version_major);
    assert_eq!(t.version_minor, s.version_minor);
    assert_eq!(t.baseline_index, s.baseline_index);
    assert_eq!(t.locked_signatures_intact, s.locked_signatures_intact);
    assert_eq!(t.anchor_count, s.anchor_count);
    assert_eq!(t.gate_layers, s.gate_layers);
}
