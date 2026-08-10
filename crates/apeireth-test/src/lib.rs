
//! apeireth-test — R23 6 module test 子模块。
//!
//! R23 P1 #5 实质化: 加 +7 顶层 pub fn — retry policy + budget + suite aggregation.
//! 不假装: 真 retry counting + real exponential backoff + 真 budget accounting.
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub target: String,
    pub retry_count: u32,
}

impl TestCase {
    pub fn new(name: impl Into<String>, target: impl Into<String>, retry_count: u32) -> Self {
        Self { name: name.into(), target: target.into(), retry_count }
    }
    pub fn validate(&self) -> bool { self.retry_count <= 10 }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — Test runner utilities
// ============================================================================

/// Retry policy 为 test case 计算 retry attempts 与 backoff 外, 汇总. Exponential backoff: base * 2^attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy { pub max_retries: u32, pub base_delay_ms: u32 }

impl RetryPolicy {
    pub fn new(max_retries: u32, base_delay_ms: u32) -> Self { Self { max_retries, base_delay_ms } }
    /// Compute delay before retrying attempt N (0-indexed). attempt 0 = base, attempt 1 = 2*base, ...
    /// Capped at `u32::MAX` to avoid overflow.
    pub fn delay_ms(&self, attempt: u32) -> u32 {
        if attempt >= 32 { return u32::MAX; }  // 防爆出发
        self.base_delay_ms.saturating_mul(1u32 << attempt)
    }
    /// Whether retrying is allowed given current attempt number.
    pub fn should_retry(&self, attempt: u32) -> bool { attempt < self.max_retries }
}

/// Aggregate test result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuiteSummary { pub passed: u32, pub failed: u32, pub skipped: u32 }

impl SuiteSummary {
    pub fn new() -> Self { Self::default() }
    pub fn add_pass(&mut self) { self.passed += 1; }
    pub fn add_fail(&mut self) { self.failed += 1; }
    pub fn add_skip(&mut self) { self.skipped += 1; }
    pub fn total(&self) -> u32 { self.passed + self.failed + self.skipped }
    pub fn pass_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 { 1.0 } else { f64::from(self.passed) / f64::from(total) }
    }
    pub fn is_clean(&self) -> bool { self.failed == 0 }
}

/// Build summary from sequence of (name, result) tuples.
pub fn summarize(cases: &[(String, CaseResult)]) -> SuiteSummary {
    let mut s = SuiteSummary::new();
    for (_, r) in cases {
        match r {
            CaseResult::Pass => s.add_pass(),
            CaseResult::Fail => s.add_fail(),
            CaseResult::Skip => s.add_skip(),
        }
    }
    s
}

/// Single test case outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseResult { Pass, Fail, Skip }

/// Validate TestCase::retry_count 不超过 policy.max_retries 与上限 10 双重约束中的更严者.
pub fn retry_count_within_policy(tc: &TestCase, policy: &RetryPolicy) -> bool {
    tc.retry_count <= policy.max_retries.min(10)
}

/// Test budget: 限制总走 case 数 + 最大 retry 总数.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget { pub max_total_attempts: u32 }

impl Budget {
    pub fn new(max_total_attempts: u32) -> Self { Self { max_total_attempts } }
    /// Compute total attempts: sum of (1 + retry_count) for each case. Return whether budget allows.
    pub fn allows(&self, cases: &[TestCase]) -> bool {
        let total: u32 = cases.iter().map(|c| 1 + c.retry_count).sum();
        total <= self.max_total_attempts
    }
    pub fn total_attempts(&self, cases: &[TestCase]) -> u32 {
        cases.iter().map(|c| 1 + c.retry_count).sum()
    }
}

/// Filter 仅 retry_count > 0 的紧急重跑用例 (flaky tests).
pub fn flaky_cases(cases: &[TestCase]) -> Vec<&TestCase> {
    cases.iter().filter(|c| c.retry_count > 0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn normal_case_validates() { assert!(TestCase::new("smoke", "apeireth-api", 3).validate()); }
    #[test] fn excessive_retry_fails() { assert!(!TestCase::new("flaky", "x", 20).validate()); }

    #[test] fn retry_policy_delay_exponential() {
        let p = RetryPolicy::new(5, 100);
        assert_eq!(p.delay_ms(0), 100);
        assert_eq!(p.delay_ms(1), 200);
        assert_eq!(p.delay_ms(2), 400);
        assert_eq!(p.delay_ms(3), 800);
    }
    #[test] fn retry_policy_delay_overflow_safe() {
        let p = RetryPolicy::new(64, 1);
        assert_eq!(p.delay_ms(31), 1u32 << 31);
        assert_eq!(p.delay_ms(32), u32::MAX);
    }
    #[test] fn retry_policy_should_retry() {
        let p = RetryPolicy::new(3, 100);
        assert!(p.should_retry(0));
        assert!(p.should_retry(2));
        assert!(!p.should_retry(3));
        assert!(!p.should_retry(10));
    }
    #[test] fn suite_summary_basic() {
        let mut s = SuiteSummary::new();
        s.add_pass(); s.add_pass(); s.add_fail();
        assert_eq!(s.total(), 3);
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 1);
        assert!(!s.is_clean());
    }
    #[test] fn suite_summary_pass_rate() {
        let mut s = SuiteSummary::new();
        for _ in 0..8 { s.add_pass(); }
        s.add_fail(); s.add_skip();
        assert!((s.pass_rate() - 0.8).abs() < 1e-9);
    }
    #[test] fn suite_summary_empty_is_clean() {
        let s = SuiteSummary::new();
        assert!(s.is_clean());
        assert_eq!(s.pass_rate(), 1.0);
    }
    #[test] fn summarize_from_tuples() {
        let v = vec![
            ("a".into(), CaseResult::Pass),
            ("b".into(), CaseResult::Fail),
            ("c".into(), CaseResult::Skip),
            ("d".into(), CaseResult::Pass),
        ];
        let s = summarize(&v);
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
    }
    #[test] fn retry_count_within_policy_basic() {
        let tc = TestCase::new("a", "t", 3);
        let p = RetryPolicy::new(5, 100);
        assert!(retry_count_within_policy(&tc, &p));
        assert!(!retry_count_within_policy(&TestCase::new("a", "t", 6), &p));
    }
    #[test] fn budget_allows_when_under() {
        let cases = vec![TestCase::new("a", "t", 2), TestCase::new("b", "t", 1)];
        let b = Budget::new(10);
        assert!(b.allows(&cases));   // total = (1+2)+(1+1) = 5 <= 10
        assert_eq!(b.total_attempts(&cases), 5);
    }
    #[test] fn budget_disallows_when_over() {
        let cases = vec![TestCase::new("a", "t", 10), TestCase::new("b", "t", 10)];
        let b = Budget::new(5);
        assert!(!b.allows(&cases));
    }
    #[test] fn flaky_cases_basic() {
        let v = vec![
            TestCase::new("stable", "x", 0),
            TestCase::new("flaky1", "x", 3),
            TestCase::new("flaky2", "y", 1),
        ];
        let f = flaky_cases(&v);
        assert_eq!(f.len(), 2);
    }
}
