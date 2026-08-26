//! R150 P1 #12: apeireth-test::property_tests — property-based testing 基础设施
//!
//! **借鉴**: alt-proptest/proptest (1.7K stars, Rust 生态 property-based testing 标杆)
//! **0 触碰** 既有 14 个 unit test (`#[test]` 风格), 仅加 proptest `proptest!` 块
//! **价值**: 任意输入自动生成, 比手写 case 覆盖更广, 找出 corner case
//!
//! **4 个 property 验证** (per R150 P1 #12 完成定义):
//! 1. `SuiteSummary::total` ≡ `passed + failed + skipped` 永真 (任意输入)
//! 2. `SuiteSummary::pass_rate` ∈ [0.0, 1.0] 永真 (任意输入, 含边界)
//! 3. `RetryPolicy::delay_ms` 单调递增 (任意 attempt 0..31)
//! 4. `RetryPolicy::delay_ms` saturating (attempt >= 32 返 u32::MAX)
//! 5. `Budget::allows` ⇔ `total_attempts ≤ max_total_attempts` 永真 (任意 cases)
//! 6. `TestCase::validate` 单调性 — retry_count 越大越不 valid

use proptest::prelude::*;

use crate::{Budget, CaseResult, RetryPolicy, SuiteSummary, TestCase};

// ============================================================
// Strategy 辅助 — 让 proptest 生成更合理的输入
// ============================================================

/// 任意 retry_count in [0..15] (覆盖正常 + 边界)
fn arb_retry_count() -> impl Strategy<Value = u32> {
    0u32..15
}

/// 任意 base_delay_ms in [1..10000] (避开 0 让 saturating mul 行为稳定)
fn arb_base_delay() -> impl Strategy<Value = u32> {
    1u32..10000
}

/// 任意 attempt in [0..35] (覆盖正常 + saturating 边界)
fn arb_attempt() -> impl Strategy<Value = u32> {
    0u32..35
}

/// 任意 max_retries in [0..20]
fn arb_max_retries() -> impl Strategy<Value = u32> {
    0u32..20
}

/// 任意 (passed, failed, skipped) 三元组 in [0..50]
fn arb_summary_counts() -> impl Strategy<Value = (u32, u32, u32)> {
    (0u32..50, 0u32..50, 0u32..50)
}

/// 任意 TestCase 列表 (1..20 个)
fn arb_test_cases() -> impl Strategy<Value = Vec<TestCase>> {
    prop::collection::vec(
        (".*", arb_retry_count()).prop_map(|(target, retry_count)| {
            TestCase::new(format!("case_{}", retry_count), target, retry_count)
        }),
        1..20,
    )
}

// ============================================================
// Property #1: SuiteSummary::total ≡ passed + failed + skipped
// ============================================================

proptest! {
    #[test]
    fn prop_summary_total_equals_sum(
        (passed, failed, skipped) in arb_summary_counts(),
    ) {
        let mut s = SuiteSummary::new();
        for _ in 0..passed { s.add_pass(); }
        for _ in 0..failed { s.add_fail(); }
        for _ in 0..skipped { s.add_skip(); }
        prop_assert_eq!(s.total(), passed + failed + skipped);
        prop_assert_eq!(s.passed, passed);
        prop_assert_eq!(s.failed, failed);
        prop_assert_eq!(s.skipped, skipped);
    }
}

// ============================================================
// Property #2: SuiteSummary::pass_rate ∈ [0.0, 1.0]
// ============================================================

proptest! {
    #[test]
    fn prop_summary_pass_rate_in_unit_interval(
        (passed, failed, skipped) in arb_summary_counts(),
    ) {
        let mut s = SuiteSummary::new();
        for _ in 0..passed { s.add_pass(); }
        for _ in 0..failed { s.add_fail(); }
        for _ in 0..skipped { s.add_skip(); }
        let rate = s.pass_rate();
        prop_assert!(rate >= 0.0, "pass_rate {} < 0.0", rate);
        prop_assert!(rate <= 1.0, "pass_rate {} > 1.0", rate);
        // empty case: total=0 → pass_rate=1.0 (per `if total == 0 { 1.0 }`)
        if passed + failed + skipped == 0 {
            prop_assert_eq!(rate, 1.0);
        }
    }
}

// ============================================================
// Property #3: RetryPolicy::delay_ms 单调递增 (attempt < 32)
// ============================================================

proptest! {
    #[test]
    fn prop_retry_delay_monotonic(
        base in arb_base_delay(),
        attempt1 in 0u32..31,
        attempt2 in 0u32..31,
    ) {
        let p = RetryPolicy::new(64, base);
        let d1 = p.delay_ms(attempt1);
        let d2 = p.delay_ms(attempt2);
        if attempt1 <= attempt2 {
            prop_assert!(d1 <= d2, "delay_ms({}) = {} > delay_ms({}) = {}", attempt1, d1, attempt2, d2);
        } else {
            prop_assert!(d2 <= d1, "delay_ms({}) = {} > delay_ms({}) = {}", attempt2, d2, attempt1, d1);
        }
    }
}

// ============================================================
// Property #4: RetryPolicy::delay_ms saturating at attempt >= 32
// ============================================================

proptest! {
    #[test]
    fn prop_retry_delay_saturates(
        base in arb_base_delay(),
        attempt in 32u32..100,
    ) {
        let p = RetryPolicy::new(128, base);
        prop_assert_eq!(p.delay_ms(attempt), u32::MAX);
    }
}

// ============================================================
// Property #5: Budget::allows ⇔ total_attempts ≤ max_total_attempts
// ============================================================

proptest! {
    #[test]
    fn prop_budget_allows_iff_total_within(
        max in 0u32..100,
        cases in arb_test_cases(),
    ) {
        let b = Budget::new(max);
        let total = b.total_attempts(&cases);
        prop_assert_eq!(b.allows(&cases), total <= max);
    }
}

// ============================================================
// Property #6: TestCase::validate 单调性
// ============================================================

proptest! {
    #[test]
    fn prop_test_case_validate_monotonic(
        target in ".*",
        retry in 0u32..30,
    ) {
        let tc = TestCase::new("c", &target, retry);
        // retry_count <= 10 → valid; retry_count > 10 → invalid
        prop_assert_eq!(tc.validate(), retry <= 10);
    }
}

// ============================================================
// Property #7: RetryPolicy::should_retry ⇔ attempt < max_retries
// ============================================================

proptest! {
    #[test]
    fn prop_should_retry_iff_attempt_within(
        max in arb_max_retries(),
        attempt in 0u32..50,
    ) {
        let p = RetryPolicy::new(max, 100);
        prop_assert_eq!(p.should_retry(attempt), attempt < max);
    }
}

// ============================================================
// Property #8: CaseResult::summarize round-trip
// ============================================================

proptest! {
    #[test]
    fn prop_summarize_consistency(
        cases in prop::collection::vec(
            (0u8..3).prop_map(|i| match i {
                0 => CaseResult::Pass,
                1 => CaseResult::Fail,
                _ => CaseResult::Skip,
            }),
            0..30,
        ),
    ) {
        let tuples: Vec<(String, CaseResult)> = cases
            .iter()
            .enumerate()
            .map(|(i, r)| (format!("case_{}", i), *r))
            .collect();
        let s = crate::summarize(&tuples);
        let p = tuples.iter().filter(|(_, r)| *r == CaseResult::Pass).count() as u32;
        let f = tuples.iter().filter(|(_, r)| *r == CaseResult::Fail).count() as u32;
        let sk = tuples.iter().filter(|(_, r)| *r == CaseResult::Skip).count() as u32;
        prop_assert_eq!(s.passed, p);
        prop_assert_eq!(s.failed, f);
        prop_assert_eq!(s.skipped, sk);
    }
}

// ============================================================
// Property #9: flaky_cases ⇔ retry_count > 0
// ============================================================

proptest! {
    #[test]
    fn prop_flaky_cases_iff_retry_positive(
        cases in arb_test_cases(),
    ) {
        let flaky = crate::flaky_cases(&cases);
        let expected = cases.iter().filter(|c| c.retry_count > 0).count();
        prop_assert_eq!(flaky.len(), expected);
        // 额外保证: 所有返项的 retry_count > 0
        for tc in &flaky {
            prop_assert!(tc.retry_count > 0);
        }
    }
}
