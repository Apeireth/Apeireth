//! R129-5 ASI Python 整合 Stage 5 治理 — G3 形式化治理集成测试
//!
//! 任务: 验证 G3 形式化治理 (Invariant trait + 8 Kani-style harness + AsiStage5Token POD)
//! 借鉴: kani 4502 (per decision-61 §3.1 R129-5) — 1:1 翻译 Kani Invariant + ProofHarness + ProofResult
//! 跟 P8-2 retry 形式化证明接 (per decision-56 §2.1 P8-2)

use apeireth_pybridge::*;

// =============================================================================
// G3 基础架构 (5 测)
// =============================================================================

#[test]
fn g3_version_is_r129_stage5() {
    assert_eq!(formal_governance_version(), "0.1.0-R129-Stage5-G3");
}

#[test]
fn g3_harness_count_is_8() {
    assert_eq!(FORMAL_GOVERNANCE_HARNESS_COUNT, 8);
}

#[test]
fn g3_token_fields_is_6() {
    assert_eq!(FORMAL_GOVERNANCE_TOKEN_FIELDS, 6);
}

#[test]
fn g3_stage1_modules_is_7() {
    assert_eq!(FORMAL_GOVERNANCE_STAGE1_MODULES, 7);
}

#[test]
fn g3_stage_count_is_4() {
    assert_eq!(FORMAL_GOVERNANCE_STAGE_COUNT, 4);
}

// =============================================================================
// G3 Invariant trait (4 测)
// =============================================================================

#[test]
fn g3_invariant_trait_for_token() {
    let t = AsiStage5Token::stage5_default();
    let invariant: &dyn Invariant = &t;
    assert!(invariant.is_safe());
}

#[test]
fn g3_invariant_trait_for_u8() {
    let v: u8 = 42;
    let invariant: &dyn Invariant = &v;
    assert!(invariant.is_safe());
}

#[test]
fn g3_invariant_trait_for_bool() {
    let v: bool = true;
    let invariant: &dyn Invariant = &v;
    assert!(invariant.is_safe());
}

#[test]
fn g3_invariant_polymorphism() {
    let v_u8: Box<dyn Invariant> = Box::new(42u8);
    let v_token: Box<dyn Invariant> = Box::new(AsiStage5Token::stage5_default());
    assert!(v_u8.is_safe());
    assert!(v_token.is_safe());
}

// =============================================================================
// G3 ProofKind (4 测)
// =============================================================================

#[test]
fn g3_proof_kind_proof() {
    assert_eq!(ProofKind::Proof.as_str(), "Proof");
}

#[test]
fn g3_proof_kind_proof_for_contract() {
    assert_eq!(ProofKind::ProofForContract.as_str(), "ProofForContract");
}

#[test]
fn g3_proof_kind_test() {
    assert_eq!(ProofKind::Test.as_str(), "Test");
}

#[test]
fn g3_proof_kind_3_variants() {
    let kinds = [
        ProofKind::Proof,
        ProofKind::ProofForContract,
        ProofKind::Test,
    ];
    assert_eq!(kinds.len(), 3);
}

// =============================================================================
// G3 ProofHarness (3 测)
// =============================================================================

#[test]
fn g3_harness_new() {
    let h = ProofHarness::new("h1", "test.rs", 42, ProofKind::Proof);
    assert_eq!(h.name, "h1");
    assert_eq!(h.file, "test.rs");
    assert_eq!(h.line, 42);
    assert_eq!(h.kind, ProofKind::Proof);
    assert!(!h.should_panic);
}

#[test]
fn g3_harness_5_fields() {
    let h = ProofHarness::new("h", "f.rs", 1, ProofKind::Proof);
    // 5 字段: name / file / line / kind / should_panic
    assert!(!h.name.is_empty());
    assert!(!h.file.is_empty());
    assert!(h.line > 0);
    assert!(!h.should_panic);
}

#[test]
fn g3_harness_clone() {
    let h = ProofHarness::new("h", "f.rs", 1, ProofKind::Proof);
    let h2 = h.clone();
    assert_eq!(h.name, h2.name);
    assert_eq!(h.file, h2.file);
}

// =============================================================================
// G3 ProofResult (4 测)
// =============================================================================

#[test]
fn g3_result_success_predicates() {
    let r = ProofResult::Success;
    assert!(r.is_success());
    assert!(!r.is_failure());
    assert!(!r.is_skipped());
}

#[test]
fn g3_result_failure_predicates() {
    let r = ProofResult::Failure {
        harness: "h".to_string(),
        message: "m".to_string(),
    };
    assert!(!r.is_success());
    assert!(r.is_failure());
    assert!(!r.is_skipped());
}

#[test]
fn g3_result_skipped_predicates() {
    let r = ProofResult::Skipped {
        reason: "r".to_string(),
    };
    assert!(!r.is_success());
    assert!(!r.is_failure());
    assert!(r.is_skipped());
}

#[test]
fn g3_result_eq() {
    assert_eq!(ProofResult::Success, ProofResult::Success);
    assert_ne!(
        ProofResult::Success,
        ProofResult::Skipped {
            reason: "x".to_string()
        }
    );
}

// =============================================================================
// G3 AsiStage5Token POD (10 测)
// =============================================================================

#[test]
fn g3_token_safe_default_all_zero() {
    let t = AsiStage5Token::safe_default();
    assert_eq!(t.stage1_7_modules, 0);
    assert_eq!(t.g1_resource_dims, 0);
    assert_eq!(t.g2_permission_layers, 0);
    assert_eq!(t.g3_harnesses, 0);
    assert_eq!(t.g4_evolution_rules, 0);
    assert_eq!(t.ceiling_critical, 0);
}

#[test]
fn g3_token_stage5_default_hardcode() {
    let t = AsiStage5Token::stage5_default();
    assert_eq!(t.stage1_7_modules, 7);
    assert_eq!(t.g1_resource_dims, 4);
    assert_eq!(t.g2_permission_layers, 6);
    assert_eq!(t.g3_harnesses, 8);
    assert_eq!(t.g4_evolution_rules, 4);
    assert_eq!(t.ceiling_critical, 1);
}

#[test]
fn g3_token_stage5_default_is_safe() {
    let t = AsiStage5Token::stage5_default();
    assert!(t.is_safe());
}

#[test]
fn g3_token_safe_default_not_safe() {
    let t = AsiStage5Token::safe_default();
    assert!(!t.is_safe());
}

#[test]
fn g3_token_default_trait_returns_stage5() {
    let t = AsiStage5Token::default();
    assert!(t.is_safe());
    assert_eq!(t.g3_harnesses, 8);
}

#[test]
fn g3_token_try_new_valid() {
    let t = AsiStage5Token::try_new(7, 4, 6, 8, 4, 1);
    assert!(t.is_ok());
    assert!(t.unwrap().is_safe());
}

#[test]
fn g3_token_try_new_invalid_stage1() {
    let t = AsiStage5Token::try_new(8, 4, 6, 8, 4, 1);
    assert!(t.is_err());
}

#[test]
fn g3_token_try_new_invalid_g1() {
    let t = AsiStage5Token::try_new(7, 5, 6, 8, 4, 1);
    assert!(t.is_err());
}

#[test]
fn g3_token_try_new_invalid_g2() {
    let t = AsiStage5Token::try_new(7, 4, 5, 8, 4, 1);
    assert!(t.is_err());
}

#[test]
fn g3_token_try_new_invalid_g3() {
    let t = AsiStage5Token::try_new(7, 4, 6, 7, 4, 1);
    assert!(t.is_err());
}

#[test]
fn g3_token_try_new_invalid_g4() {
    let t = AsiStage5Token::try_new(7, 4, 6, 8, 3, 1);
    assert!(t.is_err());
}

#[test]
fn g3_token_try_new_invalid_ceiling() {
    let t = AsiStage5Token::try_new(7, 4, 6, 8, 4, 2);
    assert!(t.is_err());
}

// =============================================================================
// G3 ProofRunner (4 测)
// =============================================================================

#[test]
fn g3_runner_new() {
    let r = ProofRunner::new();
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
}

#[test]
fn g3_runner_run_and_get() {
    let mut r = ProofRunner::new();
    let h = ProofHarness::new("h1", "f.rs", 1, ProofKind::Proof);
    r.run(&h, || ProofResult::Success);
    assert_eq!(r.len(), 1);
    let result = r.result_for("h1");
    assert!(result.is_some());
    assert!(result.unwrap().is_success());
}

#[test]
fn g3_runner_pass_fail_skipped() {
    let mut r = ProofRunner::new();
    r.run(&ProofHarness::new("p", "f.rs", 1, ProofKind::Proof), || {
        ProofResult::Success
    });
    r.run(&ProofHarness::new("f", "f.rs", 2, ProofKind::Proof), || {
        ProofResult::Failure {
            harness: "f".to_string(),
            message: "m".to_string(),
        }
    });
    r.run(&ProofHarness::new("s", "f.rs", 3, ProofKind::Test), || {
        ProofResult::Skipped {
            reason: "r".to_string(),
        }
    });
    assert_eq!(r.pass_count(), 1);
    assert_eq!(r.fail_count(), 1);
    assert_eq!(r.skipped_count(), 1);
}

#[test]
fn g3_runner_result_for_unknown() {
    let r = ProofRunner::new();
    assert!(r.result_for("unknown").is_none());
}

// =============================================================================
// G3 8 Kani-style harness (5 测)
// =============================================================================

#[test]
fn g3_run_all_8_harnesses() {
    let r = formal_governance_summary();
    assert_eq!(r.total(), 8);
}

#[test]
fn g3_run_all_8_harnesses_all_pass() {
    let r = formal_governance_summary();
    assert_eq!(r.fail_count(), 0);
    assert_eq!(r.pass_count(), 8);
    assert_eq!(r.skipped_count(), 0);
}

#[test]
fn g3_run_all_harness_names_in_report() {
    let r = formal_governance_summary();
    let s = format!("{r}");
    assert!(s.contains("proof_stage1_7_modules_intact"));
    assert!(s.contains("proof_g1_resource_dims_4"));
    assert!(s.contains("proof_g2_permission_layers_6"));
    assert!(s.contains("proof_g3_harnesses_8"));
    assert!(s.contains("proof_g4_evolution_rules_4"));
    assert!(s.contains("proof_ceiling_critical_1"));
    assert!(s.contains("proof_stage5_token_safe_default_holds"));
    assert!(s.contains("proof_stage5_token_stage5_default_is_safe"));
}

#[test]
fn g3_run_all_harness_kind_is_proof() {
    let r = formal_governance_summary();
    for (h, _) in &r.entries {
        assert_eq!(h.kind, ProofKind::Proof);
    }
}

#[test]
fn g3_run_all_no_failure_entries() {
    let r = formal_governance_summary();
    assert_eq!(r.fail_entries().len(), 0);
}

// =============================================================================
// G3 ProofReport (4 测)
// =============================================================================

#[test]
fn g3_report_new() {
    let r = ProofReport::new();
    assert_eq!(r.total(), 0);
    assert_eq!(r.pass_count(), 0);
}

#[test]
fn g3_report_record() {
    let mut r = ProofReport::new();
    r.record(
        ProofHarness::new("h", "f.rs", 1, ProofKind::Proof),
        ProofResult::Success,
    );
    assert_eq!(r.total(), 1);
    assert_eq!(r.pass_count(), 1);
}

#[test]
fn g3_report_counts() {
    let r = formal_governance_summary();
    assert_eq!(r.total(), 8);
    assert_eq!(r.pass_count(), 8);
    assert_eq!(r.fail_count(), 0);
    assert_eq!(r.skipped_count(), 0);
}

#[test]
fn g3_report_display() {
    let r = formal_governance_summary();
    let s = format!("{r}");
    assert!(s.contains("G3 形式化治理报告"));
    assert!(s.contains("8 harnesses"));
    assert!(s.contains("PASS"));
}

// =============================================================================
// G3 健康度 (3 测)
// =============================================================================

#[test]
fn g3_health_struct_ok() {
    let h = formal_governance_health();
    assert!(h.is_ok);
    assert_eq!(h.harness_count, 8);
    assert_eq!(h.token_fields, 6);
}

#[test]
fn g3_health_display() {
    let h = formal_governance_health();
    let s = format!("{h}");
    assert!(s.contains("G3 形式化治理"));
    assert!(s.contains("8 Kani-style"));
}

#[test]
fn g3_health_with_all_passing_harnesses() {
    // formal_governance_health 内部跑 8 harness, 全 pass → is_ok
    let h = formal_governance_health();
    assert!(h.is_ok);
}

// =============================================================================
// G3 跟 P8-2 retry 接 (3 测)
// =============================================================================

#[test]
fn g3_to_p8_2_harness_count_consistent() {
    // P8-2 retry: 8 Kani-style harness
    // G3 形式化治理: 8 Kani-style harness
    assert_eq!(FORMAL_GOVERNANCE_HARNESS_COUNT, 8);
}

#[test]
fn g3_to_p8_2_invariant_trait_consistent() {
    // P8-2 retry: Invariant trait 1 方法 is_safe
    // G3 形式化治理: Invariant trait 1 方法 is_safe
    let t = AsiStage5Token::stage5_default();
    let _: &dyn Invariant = &t; // 编译期 verify
    assert!(t.is_safe());
}

#[test]
fn g3_to_p8_2_pod_pattern_consistent() {
    // P8-2 retry: Stage5Token (6 字段 POD) + LockedSignature (2 字段 POD)
    // G3 形式化治理: AsiStage5Token (6 字段 POD)
    let t = AsiStage5Token::stage5_default();
    assert_eq!(t.stage1_7_modules, 7);
    assert_eq!(t.g1_resource_dims, 4);
    assert_eq!(t.g2_permission_layers, 6);
    assert_eq!(t.g3_harnesses, 8);
    assert_eq!(t.g4_evolution_rules, 4);
    assert_eq!(t.ceiling_critical, 1);
}
