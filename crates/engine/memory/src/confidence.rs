//! Beta-Binomial confidence with a Wilson 95% interval.
//!
//! Recovered from `legacy/donor/apeireth-companion/src/confidence.rs`.
//!
//! v2 already inlines a one-shot Beta(1,1) posterior + Wilson interval inside
//! W1 World Model's `CalibratedResolver`. This module is the reusable, stateful
//! helper: successive `observe(success)` updates, posterior mean, Wilson
//! interval, and an observation-count strength ladder.
//!
//! Pure math. No LLM self-report. Uniform prior `(α₀, β₀) = (1, 1)` by default.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Beta-Binomial confidence estimator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BetaBinomial {
    /// Prior α (successes). Default 1.0 (uniform).
    pub alpha0: f64,
    /// Prior β (failures). Default 1.0 (uniform).
    pub beta0: f64,
    /// Observed successes.
    pub successes: u64,
    /// Observed trials.
    pub observations: u64,
}

impl Default for BetaBinomial {
    fn default() -> Self {
        Self {
            alpha0: 1.0,
            beta0: 1.0,
            successes: 0,
            observations: 0,
        }
    }
}

/// Strength ladder keyed on observation count, not on the posterior mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strength {
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

impl Strength {
    /// Stable uppercase label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Weak => "WEAK",
            Self::Moderate => "MODERATE",
            Self::Strong => "STRONG",
            Self::VeryStrong => "VERY_STRONG",
        }
    }
}

impl BetaBinomial {
    /// Construct with a custom prior. Non-positive / non-finite values are
    /// floored at `0.001`.
    pub fn new(alpha0: f64, beta0: f64) -> Self {
        Self {
            alpha0: finite_min(alpha0, 0.001),
            beta0: finite_min(beta0, 0.001),
            successes: 0,
            observations: 0,
        }
    }

    /// Record one Bernoulli trial.
    pub fn observe(&mut self, success: bool) {
        self.observations = self.observations.saturating_add(1);
        if success {
            self.successes = self.successes.saturating_add(1);
        }
    }

    /// Record `k` successes out of `n` trials in one shot.
    pub fn observe_counts(&mut self, successes: u64, observations: u64) {
        let successes = successes.min(observations);
        self.successes = self.successes.saturating_add(successes);
        self.observations = self.observations.saturating_add(observations);
    }

    /// Posterior mean `E[θ] = (α₀ + k) / (α₀ + β₀ + n)`.
    pub fn mean(&self) -> f64 {
        let a = self.alpha0 + self.successes as f64;
        let b = self.beta0 + (self.observations - self.successes) as f64;
        let denom = a + b;
        if denom <= 0.0 || !denom.is_finite() {
            0.5
        } else {
            a / denom
        }
    }

    /// Wilson score 95% interval `(lo, hi)`, using the posterior mean as `p̂`.
    ///
    /// Empty observations return `(0.0, 1.0)`.
    pub fn interval95(&self) -> (f64, f64) {
        let n = self.observations as f64;
        if n == 0.0 {
            return (0.0, 1.0);
        }
        let p = self.mean();
        let z = 1.96;
        let z2 = z * z;
        let denom = 1.0 + z2 / n;
        let center = (p + z2 / (2.0 * n)) / denom;
        let half = z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt() / denom;
        ((center - half).max(0.0), (center + half).min(1.0))
    }

    /// Strength from observation count.
    pub fn strength(&self) -> Strength {
        match self.observations {
            0..=4 => Strength::Weak,
            5..=49 => Strength::Moderate,
            50..=999 => Strength::Strong,
            _ => Strength::VeryStrong,
        }
    }

    /// One-line report, hydra-CCA style:
    /// `conf=91% [89%-93%] obs=25000 strength=STRONG`.
    pub fn report(&self) -> String {
        let (lo, hi) = self.interval95();
        format!(
            "conf={:.0}% [{:.0}%-{:.0}%] obs={} strength={}",
            self.mean() * 100.0,
            lo * 100.0,
            hi * 100.0,
            self.observations,
            self.strength().label(),
        )
    }
}

fn finite_min(x: f64, floor: f64) -> f64 {
    if x.is_finite() {
        x.max(floor)
    } else {
        floor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_is_uninformative() {
        let b = BetaBinomial::default();
        assert!((b.mean() - 0.5).abs() < 1e-9);
        assert_eq!(b.strength(), Strength::Weak);
        assert_eq!(b.interval95(), (0.0, 1.0));
    }

    #[test]
    fn success_drives_mean_up() {
        let mut b = BetaBinomial::default();
        for _ in 0..9 {
            b.observe(true);
        }
        b.observe(false);
        assert!(
            (b.mean() - 10.0 / 12.0).abs() < 0.01,
            "9/10 success → ≈0.833: {}",
            b.mean()
        );
        assert_eq!(b.strength(), Strength::Moderate);

        let mut many = BetaBinomial::default();
        for _ in 0..99 {
            many.observe(true);
        }
        many.observe(false);
        assert!(
            (many.mean() - 0.98).abs() < 0.01,
            "99/100 → ≈0.98: {}",
            many.mean()
        );
    }

    #[test]
    fn interval_narrows_with_data() {
        let mut few = BetaBinomial::default();
        for _ in 0..5 {
            few.observe(true);
        }
        let (lo1, hi1) = few.interval95();
        let mut many = BetaBinomial::default();
        for _ in 0..500 {
            many.observe(true);
        }
        let (lo2, hi2) = many.interval95();
        assert!(
            hi2 - lo2 < hi1 - lo1,
            "more observations → narrower interval"
        );
        assert_eq!(many.strength(), Strength::Strong);
    }

    #[test]
    fn very_strong_at_thousand() {
        let mut b = BetaBinomial::default();
        b.observe_counts(900, 1000);
        assert_eq!(b.strength(), Strength::VeryStrong);
        assert!((b.mean() - 901.0 / 1002.0).abs() < 1e-9);
    }

    #[test]
    fn report_format() {
        let mut b = BetaBinomial::default();
        for _ in 0..100 {
            b.observe(true);
        }
        let r = b.report();
        assert!(r.starts_with("conf="), "format: {r}");
        assert!(r.contains("obs=100"));
        assert!(r.contains("strength=STRONG"));
    }

    #[test]
    fn new_floors_non_positive_prior() {
        let b = BetaBinomial::new(0.0, f64::NAN);
        assert!(b.alpha0 >= 0.001);
        assert!(b.beta0 >= 0.001);
    }

    #[test]
    fn observe_counts_clamps_successes() {
        let mut b = BetaBinomial::default();
        b.observe_counts(12, 10);
        assert_eq!(b.successes, 10);
        assert_eq!(b.observations, 10);
    }
}
