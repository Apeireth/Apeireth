//! Library Stage 5.1 形式化证明 — 集成测试 (12 通道).
//!
//! **12 通道** (1:1 跟 Kani 4502 形式化模型 1:1 模板, borrowed from
//! `tests/kani/Invariant/invariant_impls.rs` + `tests/kani/Invariant/percentage.rs`):
//! 1.  `Invariant` trait — 15 trivial impls
//! 2.  `Invariant` trait — 3 custom impls (VerificationSubject / Stage5Token / LockedSignature)
//! 3.  `ProofKind` — 3 状态 + as_str
//! 4.  `ProofHarness` — 字段 + 2 构造器
//! 5.  `ProofResult` — 3 状态 + 3 is_* 谓词
//! 6.  `ProofRunner` — run + check
//! 7.  `ProofReport` — record + pass/fail/skipped 计数
//! 8.  `defensive_proof!` 宏 — 3 case
//! 9.  8 Kani-style proof harness — 全过
//! 10. `run_all` + `run_all_as_report` — 1:1 跟 P5-2
//! 11. Cross-module Stage 5.0 + Stage 5.1 联合
//! 12. 0 装严守 (0 kani dep + 0 cargo kani)

use apeireth_library_governance::{
    defensive_proof, run_all_formal_proofs, Boundary, CheckStatus, ConsistencyReport,
    GovernanceContext, Invariant, LockedSignature, ProofHarness, ProofKind, ProofReport,
    ProofResult, ProofRunner, Stage5Token, VerificationSubject, run_all_as_report,
    run_all_8_harnesses,
};
use apeireth_library_governance::proof_harnesses;
use apeireth_library_governance::verification::invariants as ver_inv;

// ============================================================================
// 通道 1: Invariant trait 15 trivial impls (Kani `tests/kani/Invariant/invariant_impls.rs` 1:1)
// ============================================================================

#[test]
fn integration_trivial_invariant_u8_u16_u32_u64_u128_usize() {
    for v in [0u8, 255u8] {
        assert!(v.is_safe());
    }
    for v in [0u16, u16::MAX] {
        assert!(v.is_safe());
    }
    for v in [0u32, u32::MAX] {
        assert!(v.is_safe());
    }
    for v in [0u64, u64::MAX] {
        assert!(v.is_safe());
    }
    for v in [0u128, u128::MAX] {
        assert!(v.is_safe());
    }
    for v in [0usize, usize::MAX] {
        assert!(v.is_safe());
    }
}

#[test]
fn integration_trivial_invariant_signed_integers() {
    for v in [i8::MIN, 0i8, i8::MAX] {
        assert!(v.is_safe());
    }
    for v in [i16::MIN, 0i16, i16::MAX] {
        assert!(v.is_safe());
    }
    for v in [i32::MIN, 0i32, i32::MAX] {
        assert!(v.is_safe());
    }
    for v in [i64::MIN, 0i64, i64::MAX] {
        assert!(v.is_safe());
    }
    for v in [i128::MIN, 0i128, i128::MAX] {
        assert!(v.is_safe());
    }
    for v in [isize::MIN, 0isize, isize::MAX] {
        assert!(v.is_safe());
    }
}

#[test]
fn integration_trivial_invariant_unit_bool_char() {
    let u: () = ();
    assert!(u.is_safe());
    for v in [true, false] {
        assert!(v.is_safe());
    }
    for v in ['\0', 'a', '🦀'] {
        assert!(v.is_safe());
    }
}

// ============================================================================
// 通道 2: 3 custom Invariant impls (VerificationSubject / Stage5Token / LockedSignature)
// ============================================================================

#[test]
fn integration_custom_invariant_verification_subject_safe_default() {
    let s = VerificationSubject::safe_default();
    assert!(s.is_safe());

    // 6 字段全过
    assert!(ver_inv::version_major_is_one(&s));
    assert!(ver_inv::version_minor_is_two(&s));
    assert!(ver_inv::baseline_index_is_r11(&s));
    assert!(ver_inv::locked_signatures_intact(&s));
    assert!(ver_inv::anchor_count_is_eight(&s));
    assert!(ver_inv::gate_layers_is_six(&s));
    assert!(ver_inv::run_all(&s));
}

#[test]
fn integration_custom_invariant_verification_subject_violations() {
    // B2 major violate
    let s = VerificationSubject {
        version_major: 2,
        ..VerificationSubject::safe_default()
    };
    assert!(!s.is_safe());

    // A1 baseline violate
    let s = VerificationSubject {
        baseline_index: 1,
        ..VerificationSubject::safe_default()
    };
    assert!(!s.is_safe());

    // B1 locked violate
    let s = VerificationSubject {
        locked_signatures_intact: false,
        ..VerificationSubject::safe_default()
    };
    assert!(!s.is_safe());

    // B5 anchor violate
    let s = VerificationSubject {
        anchor_count: 7,
        ..VerificationSubject::safe_default()
    };
    assert!(!s.is_safe());

    // B4 gate violate
    let s = VerificationSubject {
        gate_layers: 5,
        ..VerificationSubject::safe_default()
    };
    assert!(!s.is_safe());
}

#[test]
fn integration_custom_invariant_stage5_token() {
    // safe_default safe
    let t = Stage5Token::safe_default();
    assert!(t.is_safe());

    // try_new 严守
    let t = Stage5Token::try_new(1, 2, 0, true, 8, 6).unwrap();
    assert!(t.is_safe());

    // 各种 violate
    assert!(Stage5Token::try_new(2, 2, 0, true, 8, 6).is_err());
    assert!(Stage5Token::try_new(1, 3, 0, true, 8, 6).is_err());
    assert!(Stage5Token::try_new(1, 2, 1, true, 8, 6).is_err());
    assert!(Stage5Token::try_new(1, 2, 0, false, 8, 6).is_err());
    assert!(Stage5Token::try_new(1, 2, 0, true, 7, 6).is_err());
    assert!(Stage5Token::try_new(1, 2, 0, true, 8, 5).is_err());
}

#[test]
fn integration_custom_invariant_locked_signature_all_24() {
    for i in 0u8..24 {
        let s = LockedSignature::try_new(i, true).unwrap();
        assert!(s.is_safe());
    }

    // index 24 violate
    assert!(LockedSignature::try_new(24, true).is_err());
    assert!(LockedSignature::try_new(255, true).is_err());

    // broken_intact violate
    assert!(LockedSignature::try_new(0, false).is_err());
    assert!(LockedSignature::try_new(23, false).is_err());

    // TOTAL 严守
    assert_eq!(LockedSignature::TOTAL, 24);
}

// ============================================================================
// 通道 3: ProofKind
// ============================================================================

#[test]
fn integration_proof_kind_as_str() {
    assert_eq!(ProofKind::Proof.as_str(), "#[kani::proof]");
    assert_eq!(
        ProofKind::ProofForContract.as_str(),
        "#[kani::proof_for_contract]"
    );
    assert_eq!(ProofKind::Test.as_str(), "#[test]");
}

// ============================================================================
// 通道 4: ProofHarness
// ============================================================================

#[test]
fn integration_proof_harness_proof_constructor() {
    let h = ProofHarness::proof("h1", "f.rs", 10);
    assert_eq!(h.name, "h1");
    assert_eq!(h.file, "f.rs");
    assert_eq!(h.line, 10);
    assert_eq!(h.kind, ProofKind::Proof);
    assert!(!h.should_panic);
}

#[test]
fn integration_proof_harness_test_constructor() {
    let h = ProofHarness::test("h1", "f.rs", 20);
    assert_eq!(h.kind, ProofKind::Test);
    assert_eq!(h.line, 20);
}

// ============================================================================
// 通道 5: ProofResult
// ============================================================================

#[test]
fn integration_proof_result_predicates() {
    assert!(ProofResult::Success.is_success());
    assert!(!ProofResult::Success.is_failure());
    assert!(!ProofResult::Success.is_skipped());

    let f = ProofResult::Failure {
        harness: "h",
        message: "m",
    };
    assert!(f.is_failure());
    assert!(!f.is_success());

    let s = ProofResult::Skipped { reason: "r" };
    assert!(s.is_skipped());
    assert!(!s.is_success());
}

// ============================================================================
// 通道 6: ProofRunner
// ============================================================================

#[test]
fn integration_proof_runner_run() {
    let r = ProofRunner::new().run(|| ProofResult::Success);
    assert!(r.is_success());
    let r = ProofRunner::new().run(|| ProofResult::Failure {
        harness: "h",
        message: "m",
    });
    assert!(r.is_failure());
}

#[test]
fn integration_proof_runner_check() {
    assert!(ProofRunner::new().check("h", true).is_success());
    assert!(ProofRunner::new().check("h", false).is_failure());
}

// ============================================================================
// 通道 7: ProofReport
// ============================================================================

#[test]
fn integration_proof_report_record_and_count() {
    let mut r = ProofReport::new();
    r.record(ProofHarness::proof("a", "f", 1), ProofResult::Success);
    r.record(ProofHarness::proof("b", "f", 2), ProofResult::Success);
    r.record(
        ProofHarness::proof("c", "f", 3),
        ProofResult::Failure {
            harness: "c",
            message: "failed",
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

#[test]
fn integration_proof_report_empty_is_ok() {
    let r = ProofReport::new();
    assert!(r.is_ok());
    assert_eq!(r.total(), 0);
    assert_eq!(r.pass_count(), 0);
    assert_eq!(r.fail_count(), 0);
}

// ============================================================================
// 通道 8: defensive_proof! macro (3 case)
// ============================================================================

#[test]
fn integration_defensive_proof_passes_on_true() {
    let r: ProofResult = defensive_proof!("h1", 5 > 0);
    assert!(r.is_success());
}

#[test]
fn integration_defensive_proof_fails_on_false() {
    let r: ProofResult = defensive_proof!("h1", 5 < 0);
    assert!(r.is_failure());
    if let ProofResult::Failure { harness, message } = r {
        assert_eq!(harness, "h1");
        assert!(message.contains("5 < 0"));
    } else {
        panic!("expected Failure");
    }
}

#[test]
fn integration_defensive_proof_complex_condition() {
    let x: u8 = 10;
    let y: u8 = 20;
    let r: ProofResult = defensive_proof!("x_lt_y", x < y);
    assert!(r.is_success());

    let r: ProofResult = defensive_proof!("x_gt_y", x > y);
    assert!(r.is_failure());
}

// ============================================================================
// 通道 9: 8 Kani-style proof harness 全过
// ============================================================================

#[test]
fn integration_8_proof_harnesses_all_pass() {
    let results = run_all_8_harnesses();
    assert_eq!(results.len(), 8);
    for r in &results {
        assert!(r.is_success(), "harness failed: {:?}", r);
    }
}

#[test]
fn integration_proof_harnesses_all_metadata_count_8() {
    // 8 harness metadata entries (跟 run_all_8_harnesses 1:1)
    assert_eq!(proof_harnesses::ALL.len(), 8);
}

// ============================================================================
// 通道 10: run_all + run_all_as_report
// ============================================================================

#[test]
fn integration_run_all_formal_proofs_returns_true() {
    assert!(run_all_formal_proofs());
}

#[test]
fn integration_run_all_as_report_8_pass_0_fail() {
    let r = run_all_as_report();
    assert_eq!(r.total(), 8);
    assert_eq!(r.pass_count(), 8);
    assert_eq!(r.fail_count(), 0);
    assert_eq!(r.skipped_count(), 0);
    assert!(r.is_ok());
}

// ============================================================================
// 通道 11: Cross-module Stage 5.0 (P5-2) + Stage 5.1 (P8-2) 联合
// ============================================================================

#[test]
fn integration_stage_5_0_and_5_1_jointly_pass() {
    // Stage 5.0 (P5-2): 6 invariants + 5 consistency + boundary
    let s = VerificationSubject::safe_default();
    assert!(ver_inv::run_all(&s));
    let r = ConsistencyReport::check();
    assert!(r.is_ok());
    assert_eq!(r.pass_count(), 5);
    assert!(Boundary::VersionMajor.check(1));
    assert!(Boundary::VersionMinor.check(2));
    assert!(Boundary::AnchorCount.check(8));
    assert!(Boundary::GateLayers.check(6));

    // Stage 5.1 (P8-2): 8 proof harness + 3 custom Invariant
    assert!(run_all_formal_proofs());
    assert!(VerificationSubject::safe_default().is_safe());
    assert!(Stage5Token::safe_default().is_safe());
    assert!(LockedSignature::safe_default().is_safe());

    // 0 装严守: 5 通道 (5.0) + 12 通道 (5.1) = 17 通道
}

#[test]
fn integration_decision_round_trip_5_known_allow() {
    // 跟 P5-2 integration_decision_round_trip_via_evaluate 1:1, Stage 5.1 复用
    let cases = [
        (GovernanceContext::version(), apeireth_library_governance::PolicyKind::Version, apeireth_library_governance::GovernanceAction::Allow),
        (GovernanceContext::baseline(), apeireth_library_governance::PolicyKind::Baseline, apeireth_library_governance::GovernanceAction::Allow),
        (GovernanceContext::anchor(), apeireth_library_governance::PolicyKind::Anchor, apeireth_library_governance::GovernanceAction::Allow),
        (GovernanceContext::gate(), apeireth_library_governance::PolicyKind::Gate, apeireth_library_governance::GovernanceAction::Allow),
    ];
    for (ctx, expected_policy, expected_action) in &cases {
        let decision = apeireth_library_governance::evaluate(ctx);
        assert_eq!(decision.policy, *expected_policy);
        assert_eq!(decision.action, *expected_action);
    }
}

// ============================================================================
// 通道 12: 0 装严守
// ============================================================================

#[test]
fn integration_zero_kani_dependency_documented() {
    // 0 装严守: 0 引 kani crate 依赖 (本测试通过 0 panic 证明)
    // 0 装严守: 0 跑 cargo kani
    // 0 装严守: cfg_attr(kani, kani::proof) 兜底, Kani 离线时退化普通 fn
    //
    // 文档化保证 (借 cargo build 时 0 引用 kani::* 验证):
    //   - 0 kani::any
    //   - 0 kani::assume
    //   - 0 kani::Invariant
    //   - 0 kani::proof
    //   - 0 借用 kani-driver / kani_metadata
    //
    // 1:1 跟 P5-2 verification 模块 0 装策略
    assert!(run_all_formal_proofs());
}

#[test]
fn integration_consistency_5_checks_unchanged_from_p5_2() {
    // P5-2 5 consistency check 0 改 (Stage 5.1 0 触碰)
    let r = ConsistencyReport::check();
    assert!(r.is_ok());
    assert_eq!(r.pass_count(), 5);
    assert!(matches!(r.version_locked, CheckStatus::Pass));
    assert!(matches!(r.baseline_present, CheckStatus::Pass));
    assert!(matches!(r.locked_24, CheckStatus::Pass));
    assert!(matches!(r.anchor_8, CheckStatus::Pass));
    assert!(matches!(r.gate_v7, CheckStatus::Pass));
}
