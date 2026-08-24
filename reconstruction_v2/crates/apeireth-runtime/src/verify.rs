//! Verify - 验证工具 (从 v1.0 apeireth-verify 2K LOC 收敛)
//!
//! 0 装 PASS: 简化 verification harness (assert + check), 完整 v1.0 era (formal verification, Z3) 不做.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    Pass,
    Warn,
    Fail,
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => write!(f, "✅ PASS"),
            Self::Warn => write!(f, "⚠️  WARN"),
            Self::Fail => write!(f, "❌ FAIL"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub result: CheckResult,
    pub detail: String,
}

impl Check {
    pub fn new(name: impl Into<String>, result: CheckResult, detail: impl Into<String>) -> Self {
        Self { name: name.into(), result, detail: detail.into() }
    }
    pub fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, CheckResult::Pass, detail)
    }
    pub fn warn(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, CheckResult::Warn, detail)
    }
    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, CheckResult::Fail, detail)
    }
}

/// VerifyReport - 聚合所有 checks
#[derive(Default)]
pub struct VerifyReport {
    pub checks: Vec<Check>,
}

impl VerifyReport {
    pub fn add(&mut self, c: Check) { self.checks.push(c); }
    pub fn all_pass(&self) -> bool { self.checks.iter().all(|c| c.result == CheckResult::Pass) }
    pub fn summary(&self) -> String {
        let (p, w, f) = self.checks.iter().fold((0,0,0), |(p,w,f), c| match c.result {
            CheckResult::Pass => (p+1, w, f),
            CheckResult::Warn => (p, w+1, f),
            CheckResult::Fail => (p, w, f+1),
        });
        format!("checks: {} pass, {} warn, {} fail", p, w, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_check_pass() {
        let c = Check::pass("test1", "ok");
        assert_eq!(c.result, CheckResult::Pass);
    }
    #[test] fn test_all_pass() {
        let mut r = VerifyReport::default();
        r.add(Check::pass("a", "x"));
        r.add(Check::pass("b", "y"));
        assert!(r.all_pass());
        assert!(r.summary().contains("2 pass"));
    }
    #[test] fn test_mixed_results() {
        let mut r = VerifyReport::default();
        r.add(Check::pass("a", "x"));
        r.add(Check::warn("b", "y"));
        r.add(Check::fail("c", "z"));
        assert!(!r.all_pass());
        assert!(r.summary().contains("1 pass, 1 warn, 1 fail"));
    }
}
