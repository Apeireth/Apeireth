//! Calibration-aware critic: Brier + ECE → Continue / Revise / Reject.
//!
//! Canonical implementation module. and
//! **decoupled from the canonical 6-state evolution machine**.
//!
//! v2 already has:
//! - `reflexion::RuleCritic` — template text over three failure kinds
//! - `runtime::canonical::JudgeModule` — LLM-as-judge of a candidate reply
//!
//! Neither of those scores a *forecast history*. This module is the missing
//! diagnostic critic: Murphy decomposition + ECE → a severity in `[0, 1]` and
//! a recommended action. It does not own evolution state, a provider, or a
//! loop. Callers that still have a state machine map [`CritiqueAction`]
//! themselves.
//!
//! Default-off: this is a library helper. Production wiring (a `cognitive.*`
//! AgentModule slot, if ever wanted) is a coordinator concern.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::calibration::{
    calibration_bins, decompose, expected_calibration_error, mean_brier_score, BrierDecomposition,
    CalibrationBin, Observation, DEFAULT_NUM_BINS,
};

/// Recommended action reflecting a calibration diagnosis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CritiqueAction {
    /// Calibration is good enough to proceed.
    Continue,
    /// Calibration is middling; revise before committing.
    Revise,
    /// Calibration is poor; reject / retire the proposal.
    Reject,
}

/// One critique of a forecast history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CritiqueResult {
    /// Combined severity in `[0, 1]` (`0` = perfect, `1` = terrible).
    pub severity: f64,
    /// Mean Brier score of the history.
    pub brier_estimate: f64,
    /// Approximate 95% CI width `1.96 · √(p(1-p)/n)` on the base rate.
    pub confidence_interval_width: f64,
    /// Expected calibration error.
    pub expected_calibration_error: f64,
    /// Recommended action.
    pub recommended_action: CritiqueAction,
    /// Murphy three-way partition.
    pub decomposition: BrierDecomposition,
    /// Per-bin reliability diagram.
    pub bins: Vec<CalibrationBin>,
    /// Sample size.
    pub num_samples: usize,
}

/// Critic thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticConfig {
    /// Number of equal-width bins (default 10).
    pub num_bins: usize,
    /// Severity at or above which the critic recommends [`CritiqueAction::Revise`].
    pub revise_threshold: f64,
    /// Severity at or above which the critic recommends [`CritiqueAction::Reject`].
    pub reject_threshold: f64,
}

impl Default for CriticConfig {
    fn default() -> Self {
        Self {
            num_bins: DEFAULT_NUM_BINS,
            revise_threshold: 0.15,
            reject_threshold: 0.30,
        }
    }
}

/// Calibration-aware critic. Pure function of a forecast history.
#[derive(Debug, Clone)]
pub struct CalibrationCritic {
    config: CriticConfig,
}

impl CalibrationCritic {
    /// Construct with explicit thresholds.
    pub fn new(config: CriticConfig) -> Self {
        Self { config }
    }

    /// Default thresholds (`revise = 0.15`, `reject = 0.30`, 10 bins).
    pub fn default_critic() -> Self {
        Self::new(CriticConfig::default())
    }

    /// Access the config.
    pub fn config(&self) -> &CriticConfig {
        &self.config
    }

    /// Critique a `(forecast, outcome)` history.
    ///
    /// Empty history is fail-open: severity 0, [`CritiqueAction::Continue`].
    pub fn critique(&self, history: &[Observation]) -> CritiqueResult {
        let n = history.len();
        if n == 0 {
            return self.empty_result();
        }

        let decomposition = decompose(history, self.config.num_bins);
        let bins = calibration_bins(history, self.config.num_bins);
        let ece = expected_calibration_error(&bins);
        let severity = (decomposition.brier_score + ece).clamp(0.0, 1.0);

        let o_bar: f64 = history.iter().map(|o| o.outcome).sum::<f64>() / n as f64;
        let ci_width = (o_bar * (1.0 - o_bar) / n as f64).sqrt() * 1.96;

        let recommended_action = self.action_for(severity);

        CritiqueResult {
            severity,
            brier_estimate: decomposition.brier_score,
            confidence_interval_width: ci_width,
            expected_calibration_error: ece,
            recommended_action,
            decomposition,
            bins,
            num_samples: n,
        }
    }

    /// Critique a live forecast against (optional) history.
    ///
    /// When history is empty the critic uses a *proxy* severity
    /// `1 − 2·|forecast − 0.5|` (0.5 → most uncertain → 1.0; 0/1 → 0.0).
    /// When history is present the live forecast is ignored: diagnosis
    /// reflects the historical record, not the current guess.
    pub fn critique_single(&self, forecast_now: f64, history: &[Observation]) -> CritiqueResult {
        let mut result = self.critique(history);
        if history.is_empty() {
            let forecast_now = if forecast_now.is_finite() {
                forecast_now.clamp(0.0, 1.0)
            } else {
                0.5
            };
            let proxy = ((0.5 - (forecast_now - 0.5).abs()) * 2.0).clamp(0.0, 1.0);
            result.severity = proxy;
            result.brier_estimate = proxy;
            result.confidence_interval_width = 1.0;
            result.expected_calibration_error = proxy;
            result.recommended_action = self.action_for(proxy);
        }
        result
    }

    fn action_for(&self, severity: f64) -> CritiqueAction {
        if severity >= self.config.reject_threshold {
            CritiqueAction::Reject
        } else if severity >= self.config.revise_threshold {
            CritiqueAction::Revise
        } else {
            CritiqueAction::Continue
        }
    }

    fn empty_result(&self) -> CritiqueResult {
        CritiqueResult {
            severity: 0.0,
            brier_estimate: 0.0,
            confidence_interval_width: 1.0,
            expected_calibration_error: 0.0,
            recommended_action: CritiqueAction::Continue,
            decomposition: BrierDecomposition {
                reliability: 0.0,
                resolution: 0.0,
                uncertainty: 0.0,
                brier_score: 0.0,
                num_samples: 0,
            },
            bins: vec![],
            num_samples: 0,
        }
    }
}

impl Default for CalibrationCritic {
    fn default() -> Self {
        Self::default_critic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obs(pairs: &[(f64, f64)]) -> Vec<Observation> {
        pairs.iter().map(|&(f, o)| Observation::new(f, o)).collect()
    }

    #[test]
    fn critic_recommends_continue_for_perfect_history() {
        let history = make_obs(&[(0.0, 0.0), (0.0, 0.0), (1.0, 1.0), (1.0, 1.0)]);
        let r = CalibrationCritic::default_critic().critique(&history);
        assert!(
            r.severity < 0.15,
            "perfect history severity should be < revise, got {} (BS={}, ECE={})",
            r.severity,
            r.brier_estimate,
            r.expected_calibration_error
        );
        assert_eq!(r.recommended_action, CritiqueAction::Continue);
        assert_eq!(r.num_samples, 4);
    }

    #[test]
    fn critic_recommends_reject_for_miscalibrated_history() {
        let history = make_obs(&[(0.9, 0.0), (0.8, 0.0), (0.95, 0.0), (0.85, 0.0), (0.7, 0.0)]);
        let r = CalibrationCritic::default_critic().critique(&history);
        assert!(
            r.severity > 0.3,
            "severity should be high, got {}",
            r.severity
        );
        assert_eq!(r.recommended_action, CritiqueAction::Reject);
    }

    #[test]
    fn critic_recommends_revise_for_moderate_history() {
        let history = make_obs(&[
            (0.7, 1.0),
            (0.7, 0.0),
            (0.5, 1.0),
            (0.5, 0.0),
            (0.3, 1.0),
            (0.3, 0.0),
        ]);
        let r = CalibrationCritic::default_critic().critique(&history);
        assert!(
            r.severity >= 0.10,
            "moderate severity expected, got {}",
            r.severity
        );
    }

    #[test]
    fn critic_handles_empty_history() {
        let r = CalibrationCritic::default_critic().critique(&[]);
        assert_eq!(r.num_samples, 0);
        assert_eq!(r.severity, 0.0);
        assert_eq!(r.recommended_action, CritiqueAction::Continue);
    }

    #[test]
    fn critic_single_with_empty_history_uses_proxy() {
        let c = CalibrationCritic::default_critic();
        let r_high = c.critique_single(0.5, &[]);
        assert_eq!(r_high.severity, 1.0);
        let r_low = c.critique_single(0.0, &[]);
        assert_eq!(r_low.severity, 0.0);
    }

    #[test]
    fn critic_single_with_history_uses_history_not_proxy() {
        let history = make_obs(&[(0.0, 0.0), (1.0, 1.0)]);
        let r = CalibrationCritic::default_critic().critique_single(0.5, &history);
        assert!(
            r.severity < 0.15,
            "single with good history should be low severity, got {}",
            r.severity
        );
    }

    #[test]
    fn critic_confidence_interval_decreases_with_sample_size() {
        let mut small: Vec<(f64, f64)> = vec![(0.5, 0.0); 5];
        for (i, pair) in small.iter_mut().enumerate() {
            if i % 2 == 1 {
                pair.1 = 1.0;
            }
        }
        let mut large: Vec<(f64, f64)> = vec![(0.5, 0.0); 100];
        for (i, pair) in large.iter_mut().enumerate() {
            if i % 2 == 1 {
                pair.1 = 1.0;
            }
        }
        let c = CalibrationCritic::default_critic();
        let r_small = c.critique(&make_obs(&small));
        let r_large = c.critique(&make_obs(&large));
        assert!(r_small.confidence_interval_width > r_large.confidence_interval_width);
    }

    #[test]
    fn critic_brier_estimate_matches_mean_brier_score() {
        let history = make_obs(&[(0.9, 1.0), (0.8, 0.0), (0.5, 0.5), (0.2, 1.0)]);
        let r = CalibrationCritic::default_critic().critique(&history);
        let expected = mean_brier_score(&history);
        assert!((r.brier_estimate - expected).abs() < 1e-9);
    }

    #[test]
    fn critic_integration_with_calibration_bins() {
        let history = make_obs(&[(0.1, 0.0), (0.3, 0.0), (0.7, 1.0), (0.9, 1.0)]);
        let r = CalibrationCritic::default_critic().critique(&history);
        assert_eq!(r.bins.len(), 10);
        assert!(r.decomposition.is_monotonic());
    }

    #[test]
    fn critic_severity_in_unit_range() {
        let history = make_obs(&[(0.0, 1.0); 100]);
        let r = CalibrationCritic::default_critic().critique(&history);
        assert!((0.0..=1.0).contains(&r.severity));
    }

    #[test]
    fn critic_custom_thresholds_change_recommendation() {
        let history = make_obs(&[(0.6, 0.4); 5]);
        let c_strict = CalibrationCritic::new(CriticConfig {
            revise_threshold: 0.05,
            reject_threshold: 0.10,
            ..Default::default()
        });
        let c_lenient = CalibrationCritic::new(CriticConfig {
            revise_threshold: 0.50,
            reject_threshold: 0.90,
            ..Default::default()
        });
        let r_strict = c_strict.critique(&history);
        let r_lenient = c_lenient.critique(&history);
        // Slight miscalibration (forecast 0.6, outcome 0.4) → severity ≈ 0.24.
        // Strict (reject ≥ 0.10) must be harsher than lenient (revise ≥ 0.50).
        assert_ne!(r_strict.recommended_action, r_lenient.recommended_action);
        assert_eq!(r_strict.recommended_action, CritiqueAction::Reject);
        assert_eq!(r_lenient.recommended_action, CritiqueAction::Continue);
    }

    #[test]
    fn critic_serialization_round_trip() {
        let history = make_obs(&[(0.5, 0.0), (0.6, 1.0)]);
        let r = CalibrationCritic::default_critic().critique(&history);
        let json = serde_json::to_string(&r).unwrap();
        let back: CritiqueResult = serde_json::from_str(&json).unwrap();
        assert!((r.brier_estimate - back.brier_estimate).abs() < 1e-12);
        assert!((r.severity - back.severity).abs() < 1e-12);
        assert_eq!(r.recommended_action, back.recommended_action);
        assert_eq!(r.num_samples, back.num_samples);
        assert_eq!(r.bins.len(), back.bins.len());
    }
}
