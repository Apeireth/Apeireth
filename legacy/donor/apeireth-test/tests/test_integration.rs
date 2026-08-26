//! Integration tests for apeireth-test (post-1.0.0)
//!
//! src/lib.rs 已有 13 #[test] + organ_kani_proofs 5 #[test] + property_tests.
//! 这里 (tests/) 加 cross-module 集成测试 + 边界 case.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_test::{
    flaky_cases, retry_count_within_policy, summarize, Budget, CaseResult, RetryPolicy,
    SuiteSummary, TestCase,
};

// =============================================================================
// TestCase
// =============================================================================

#[test]
fn test_case_new_fields() {
    let c = TestCase::new("smoke", "apeireth-api", 3);
    assert_eq!(c.name, "smoke");
    assert_eq!(c.target, "apeireth-api");
    assert_eq!(c.retry_count, 3);
}

#[test]
fn test_case_validate_boundary_zero() {
    assert!(TestCase::new("a", "b", 0).validate(), "0 retries 合法");
}

#[test]
fn test_case_validate_boundary_ten() {
    assert!(
        TestCase::new("a", "b", 10).validate(),
        "10 retries 上限合法"
    );
}

#[test]
fn test_case_validate_over_limit() {
    assert!(!TestCase::new("a", "b", 11).validate(), ">10 非法");
    assert!(!TestCase::new("a", "b", u32::MAX).validate());
}

#[test]
fn test_case_clone_eq() {
    let a = TestCase::new("x", "y", 5);
    let b = a.clone();
    assert_eq!(a, b);
}

// 注: TestCase 派生 Serialize+Deserialize, serde_json 集成测试需要 dev-dep,
// 不强制. 本 crate dev-dep 仅 proptest, 故跳过 serde roundtrip integration test.

// =============================================================================
// RetryPolicy
// =============================================================================

#[test]
fn retry_policy_new_fields() {
    let p = RetryPolicy::new(5, 100);
    assert_eq!(p.max_retries, 5);
    assert_eq!(p.base_delay_ms, 100);
}

#[test]
fn retry_policy_delay_ms_sequence() {
    let p = RetryPolicy::new(10, 50);
    assert_eq!(p.delay_ms(0), 50);
    assert_eq!(p.delay_ms(1), 100);
    assert_eq!(p.delay_ms(2), 200);
    assert_eq!(p.delay_ms(3), 400);
    assert_eq!(p.delay_ms(4), 800);
    assert_eq!(p.delay_ms(5), 1600);
}

#[test]
fn retry_policy_delay_ms_overflow_saturates() {
    let p = RetryPolicy::new(64, 1);
    // attempt 31 = 1 << 31 = 2^31 = 2147483648 (u32 OK)
    assert_eq!(p.delay_ms(31), 1u32 << 31);
    // attempt 32 = 1 << 32 overflow → saturate → u32::MAX
    assert_eq!(p.delay_ms(32), u32::MAX, "32 saturate");
    assert_eq!(p.delay_ms(100), u32::MAX);
}

#[test]
fn retry_policy_should_retry_at_max() {
    let p = RetryPolicy::new(3, 100);
    assert!(p.should_retry(0));
    assert!(p.should_retry(1));
    assert!(p.should_retry(2));
    assert!(!p.should_retry(3), ">=max_retries 不允许");
}

#[test]
fn retry_policy_zero_max_no_retry() {
    let p = RetryPolicy::new(0, 100);
    assert!(!p.should_retry(0));
}

// =============================================================================
// SuiteSummary
// =============================================================================

#[test]
fn suite_summary_default_zero() {
    let s = SuiteSummary::new();
    assert_eq!(s.passed, 0);
    assert_eq!(s.failed, 0);
    assert_eq!(s.skipped, 0);
    assert_eq!(s.total(), 0);
    assert!(s.is_clean(), "空 suite 算 clean");
    assert!((s.pass_rate() - 1.0).abs() < 1e-9);
}

#[test]
fn suite_summary_counters_increment() {
    let mut s = SuiteSummary::new();
    for _ in 0..3 {
        s.add_pass();
    }
    s.add_fail();
    s.add_fail();
    s.add_skip();
    assert_eq!(s.passed, 3);
    assert_eq!(s.failed, 2);
    assert_eq!(s.skipped, 1);
    assert_eq!(s.total(), 6);
}

#[test]
fn suite_summary_pass_rate_pure_pass() {
    let mut s = SuiteSummary::new();
    for _ in 0..5 {
        s.add_pass();
    }
    assert!((s.pass_rate() - 1.0).abs() < 1e-9);
}

#[test]
fn suite_summary_pass_rate_pure_fail() {
    let mut s = SuiteSummary::new();
    s.add_fail();
    assert!((s.pass_rate() - 0.0).abs() < 1e-9);
}

#[test]
fn suite_summary_pass_rate_mixed() {
    let mut s = SuiteSummary::new();
    for _ in 0..7 {
        s.add_pass();
    }
    for _ in 0..3 {
        s.add_fail();
    }
    assert!((s.pass_rate() - 0.7).abs() < 1e-9);
}

#[test]
fn suite_summary_is_clean_only_when_no_fail() {
    let mut s = SuiteSummary::new();
    s.add_pass();
    s.add_skip();
    assert!(s.is_clean(), "0 failed → clean");
    s.add_fail();
    assert!(!s.is_clean());
}

#[test]
fn suite_summary_clone_eq() {
    let mut a = SuiteSummary::new();
    a.add_pass();
    let b = a.clone();
    assert_eq!(a, b);
}

// =============================================================================
// CaseResult + summarize
// =============================================================================

#[test]
fn case_result_variants() {
    let _p = CaseResult::Pass;
    let _f = CaseResult::Fail;
    let _s = CaseResult::Skip;
    assert_ne!(CaseResult::Pass, CaseResult::Fail);
    assert_ne!(CaseResult::Pass, CaseResult::Skip);
    assert_ne!(CaseResult::Fail, CaseResult::Skip);
}

#[test]
fn case_result_debug() {
    assert_eq!(format!("{:?}", CaseResult::Pass), "Pass");
    assert_eq!(format!("{:?}", CaseResult::Fail), "Fail");
    assert_eq!(format!("{:?}", CaseResult::Skip), "Skip");
}

#[test]
fn summarize_empty() {
    let s = summarize(&[]);
    assert!(s.is_clean());
    assert_eq!(s.total(), 0);
}

#[test]
fn summarize_all_pass() {
    let v = vec![
        ("a".into(), CaseResult::Pass),
        ("b".into(), CaseResult::Pass),
    ];
    let s = summarize(&v);
    assert_eq!(s.passed, 2);
    assert!(s.is_clean());
}

#[test]
fn summarize_mixed_outcomes() {
    let v = vec![
        ("a".into(), CaseResult::Pass),
        ("b".into(), CaseResult::Fail),
        ("c".into(), CaseResult::Skip),
        ("d".into(), CaseResult::Pass),
        ("e".into(), CaseResult::Fail),
    ];
    let s = summarize(&v);
    assert_eq!(s.passed, 2);
    assert_eq!(s.failed, 2);
    assert_eq!(s.skipped, 1);
    assert_eq!(s.total(), 5);
}

// =============================================================================
// retry_count_within_policy
// =============================================================================

#[test]
fn retry_within_policy_basic() {
    let tc = TestCase::new("a", "t", 3);
    let p = RetryPolicy::new(5, 100);
    assert!(retry_count_within_policy(&tc, &p));
}

#[test]
fn retry_within_policy_above_max() {
    let tc = TestCase::new("a", "t", 6);
    let p = RetryPolicy::new(5, 100);
    assert!(!retry_count_within_policy(&tc, &p));
}

#[test]
fn retry_within_policy_capped_at_10() {
    let p = RetryPolicy::new(20, 100);
    // min(20,10) = 10 cap
    assert!(
        retry_count_within_policy(&TestCase::new("a", "t", 8), &p),
        "8 <= 10 OK"
    );
    assert!(
        !retry_count_within_policy(&TestCase::new("a", "t", 12), &p),
        "12 > 10 拒"
    );
    assert!(
        retry_count_within_policy(&TestCase::new("a", "t", 10), &p),
        "10 上限 OK"
    );
}

// =============================================================================
// Budget
// =============================================================================

#[test]
fn budget_new_field() {
    let b = Budget::new(10);
    assert_eq!(b.max_total_attempts, 10);
}

#[test]
fn budget_allows_when_total_le_max() {
    let cases = vec![TestCase::new("a", "t", 2), TestCase::new("b", "t", 1)];
    let b = Budget::new(10);
    assert!(b.allows(&cases), "total = (1+2)+(1+1) = 5");
}

#[test]
fn budget_total_attempts_includes_initial() {
    let cases = vec![TestCase::new("a", "t", 0)];
    let b = Budget::new(1);
    assert!(b.allows(&cases), "1 case with 0 retries = 1 attempt");
    assert_eq!(b.total_attempts(&cases), 1);
}

#[test]
fn budget_disallows_over_max() {
    let cases = vec![TestCase::new("a", "t", 10), TestCase::new("b", "t", 10)];
    let b = Budget::new(5);
    assert!(!b.allows(&cases), "total = 22 > 5");
    assert_eq!(b.total_attempts(&cases), 22);
}

#[test]
fn budget_zero_max_blocks_all() {
    let cases = vec![TestCase::new("a", "t", 0)];
    let b = Budget::new(0);
    assert!(!b.allows(&cases));
}

// =============================================================================
// flaky_cases
// =============================================================================

#[test]
fn flaky_cases_empty_input() {
    let f = flaky_cases(&[]);
    assert!(f.is_empty());
}

#[test]
fn flaky_cases_no_flaky() {
    let v = vec![TestCase::new("a", "t", 0), TestCase::new("b", "t", 0)];
    let f = flaky_cases(&v);
    assert!(f.is_empty());
}

#[test]
fn flaky_cases_all_flaky() {
    let v = vec![TestCase::new("a", "t", 1), TestCase::new("b", "t", 5)];
    let f = flaky_cases(&v);
    assert_eq!(f.len(), 2);
}

#[test]
fn flaky_cases_mixed() {
    let v = vec![
        TestCase::new("stable1", "t", 0),
        TestCase::new("flaky1", "t", 3),
        TestCase::new("stable2", "t", 0),
        TestCase::new("flaky2", "y", 1),
        TestCase::new("flaky3", "z", 7),
    ];
    let f = flaky_cases(&v);
    assert_eq!(f.len(), 3);
    let names: Vec<&str> = f.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"flaky1"));
    assert!(names.contains(&"flaky2"));
    assert!(names.contains(&"flaky3"));
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_full_pipeline() {
    // 模拟真测试运行: TestCase[] → Budget allows → filter flaky → summarize
    let cases = vec![
        TestCase::new("smoke", "apeireth-api", 0),
        TestCase::new("unit", "apeireth-core", 1),
        TestCase::new("integration", "apeireth-pipeline-g5", 2),
        TestCase::new("stable", "apeireth-config", 0),
    ];
    let budget = Budget::new(20);
    assert!(budget.allows(&cases), "total = 1+2+3+1 = 7 <= 20");

    let flaky = flaky_cases(&cases);
    assert_eq!(flaky.len(), 2);

    // simulate all pass
    let results: Vec<(String, CaseResult)> = cases
        .iter()
        .map(|c| (c.name.clone(), CaseResult::Pass))
        .collect();
    let summary = summarize(&results);
    assert!(summary.is_clean());
    assert_eq!(summary.passed, 4);
}

#[test]
fn integration_retry_decision_uses_policy() {
    let policy = RetryPolicy::new(3, 100);
    let cases = vec![
        TestCase::new("c1", "t", 0),
        TestCase::new("c2", "t", 3),
        TestCase::new("c3", "t", 4), // out of policy
    ];
    let within: Vec<&TestCase> = cases
        .iter()
        .filter(|c| retry_count_within_policy(c, &policy))
        .collect();
    assert_eq!(within.len(), 2, "c3 (4 retries) out of policy");
}
