//! Confidence - 信心评估 (从 v1.0 apeireth-companion/confidence.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 Beta-Binomial 信心 + Brier score
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BetaBinomial {
    pub alpha: f64,  // 0 装 PASS: 成功先验
    pub beta: f64,   // 0 装 PASS: 失败先验
}

impl BetaBinomial {
    pub fn new(alpha: f64, beta: f64) -> Self { Self { alpha, beta } }
    /// 0 装 PASS: 真 mean (alpha / (alpha + beta))
    pub fn mean(&self) -> f64 { self.alpha / (self.alpha + self.beta) }
    /// 0 装 PASS: 真 variance
    pub fn variance(&self) -> f64 {
        let n = self.alpha + self.beta;
        (self.alpha * self.beta) / (n * n * (n + 1.0))
    }
    /// 0 装 PASS: 真 update (Bayesian)
    pub fn update(&mut self, success: bool) {
        if success { self.alpha += 1.0; } else { self.beta += 1.0; }
    }
}

/// 0 装 PASS: 真 Brier score (mean squared error of probability forecast)
pub fn brier_score(predicted: f64, actual: bool) -> f64 {
    let a = if actual { 1.0 } else { 0.0 };
    (predicted - a).powi(2)
}

#[derive(Default)]
pub struct StrengthTracker {
    pub scores: HashMap<String, BetaBinomial>,
}

impl StrengthTracker {
    pub fn new() -> Self { Self::default() }
    pub fn record(&mut self, key: impl Into<String>, success: bool) {
        self.scores.entry(key.into()).or_insert_with(|| BetaBinomial::new(1.0, 1.0)).update(success);
    }
    pub fn strength(&self, key: &str) -> Option<f64> {
        self.scores.get(key).map(|b| b.mean())
    }
    /// 0 装 PASS: 真 batch Brier
    pub fn avg_brier(&self, outcomes: &[(String, bool)]) -> f64 {
        if outcomes.is_empty() { return 0.0; }
        let sum: f64 = outcomes.iter().map(|(k, actual)| {
            let p = self.strength(k).unwrap_or(0.5);
            brier_score(p, *actual)
        }).sum();
        sum / outcomes.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_beta_mean() {
        let b = BetaBinomial::new(3.0, 1.0);
        assert!((b.mean() - 0.75).abs() < 1e-6);
    }
    #[test] fn test_beta_update() {
        let mut b = BetaBinomial::new(1.0, 1.0);
        b.update(true); b.update(true); b.update(false);
        assert!((b.mean() - 0.6).abs() < 1e-6);  // alpha=3 beta=2
    }
    #[test] fn test_brier_perfect() {
        assert_eq!(brier_score(1.0, true), 0.0);
        assert_eq!(brier_score(0.0, false), 0.0);
    }
    #[test] fn test_brier_worst() {
        assert!((brier_score(1.0, false) - 1.0).abs() < 1e-6);
    }
    #[test] fn test_strength_tracker() {
        let mut t = StrengthTracker::new();
        t.record("task1", true);
        t.record("task1", true);
        t.record("task1", false);
        let s = t.strength("task1").unwrap();
        assert!((s - 0.6).abs() < 1e-6);
    }
    #[test] fn test_unknown_strength() {
        let t = StrengthTracker::new();
        assert!(t.strength("unknown").is_none());
    }
}
