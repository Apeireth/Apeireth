//! Generalized online calibration: EMA baseline, linear scale+offset fit, 2σ drift.
//!
//! **Stripped of the 24-dim / 9-submeasure ASI scaffolding**.
//!
//! The baseline types were locked to `V05_DIM_COUNT` / `V1136_SUBMEASURE_COUNT`.
//! v2 has no ASI crate, so this module operates on a caller-owned `f64` series:
//!
//! - [`AdaptiveBaseline`] — EMA mean / variance + z-score
//! - [`LinearCalibration`] — `y = clamp(scale · x + offset, 0, 1)` from
//!   (observed, expected) feedback plus residual offset against the baseline
//! - [`DriftDetector`] — consecutive `|z| > threshold` streak → alarm
//!   feedback, optional dry-run
//!
//! No provider, no 24-dim registry, no persistent turn invoker.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Affine coefficient: `y = clamp(scale · x + offset, 0, 1)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coeff {
    /// Scale (default 1.0 = identity).
    pub scale: f64,
    /// Offset (default 0.0).
    pub offset: f64,
}

impl Default for Coeff {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: 0.0,
        }
    }
}

impl Coeff {
    /// Apply and clamp to the unit interval. Non-finite `x` maps to 0.0.
    pub fn apply(&self, x: f64) -> f64 {
        let x = if x.is_finite() { x } else { 0.0 };
        (self.scale * x + self.offset).clamp(0.0, 1.0)
    }
}

/// One named series' calibration coefficients plus audit metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationCoefficients {
    /// Per-series coefficients, parallel to the baseline's series names.
    pub coeffs: Vec<Coeff>,
    /// Sample count used for this fit.
    pub sample_count: usize,
    /// Unix-epoch seconds of the fit (caller-supplied).
    pub calibrated_at: i64,
}

impl CalibrationCoefficients {
    /// Identity coefficients for `n` series.
    pub fn identity(n: usize, sample_count: usize, calibrated_at: i64) -> Self {
        Self {
            coeffs: vec![Coeff::default(); n],
            sample_count,
            calibrated_at,
        }
    }

    /// Apply coefficients to a parallel observation vector. Length mismatch
    /// truncates to the shorter of the two.
    pub fn apply(&self, values: &[f64]) -> Vec<f64> {
        values
            .iter()
            .zip(self.coeffs.iter())
            .map(|(x, c)| c.apply(*x))
            .collect()
    }
}

/// Named (observed, expected) feedback for one series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserFeedback {
    /// Series name (must match a name registered on the baseline).
    pub name: String,
    /// Observed value.
    pub observed: f64,
    /// Expected / labelled value.
    pub expected: f64,
    /// Unix-epoch seconds.
    pub timestamp: i64,
}

impl UserFeedback {
    /// Construct a feedback record.
    pub fn new(name: impl Into<String>, observed: f64, expected: f64, timestamp: i64) -> Self {
        Self {
            name: name.into(),
            observed,
            expected,
            timestamp,
        }
    }

    /// `expected − observed`.
    pub fn error(&self) -> f64 {
        self.expected - self.observed
    }
}

/// EMA rolling mean / variance for a named series of `f64` values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveBaseline {
    /// EMA smoothing coefficient in `(0, 1]`. Larger → faster tracking.
    pub alpha: f64,
    /// Series names (stable order; index is the identity of a series).
    names: Vec<String>,
    mean: Vec<f64>,
    var: Vec<f64>,
    seen: usize,
    initialized: bool,
}

impl AdaptiveBaseline {
    /// Create a baseline for `names`. `alpha` is clamped to `[0.001, 1.0]`.
    pub fn new(names: impl IntoIterator<Item = impl Into<String>>, alpha: f64) -> Self {
        let names: Vec<String> = names.into_iter().map(Into::into).collect();
        let n = names.len();
        Self {
            alpha: alpha.clamp(0.001, 1.0),
            names,
            mean: vec![0.5; n],
            var: vec![0.0; n],
            seen: 0,
            initialized: false,
        }
    }

    /// Convenience: a single unnamed series (`"value"`).
    pub fn scalar(alpha: f64) -> Self {
        Self::new(["value"], alpha)
    }

    /// Number of series.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// True when no series were registered.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Series names in registration order.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Samples observed so far.
    pub fn seen(&self) -> usize {
        self.seen
    }

    /// Look up a series index by name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// Feed one observation vector. Extra values are ignored; missing values
    /// are treated as the current mean (no-op for that series).
    pub fn observe(&mut self, values: &[f64]) {
        let n = self.names.len();
        if n == 0 {
            return;
        }
        if !self.initialized {
            for i in 0..n {
                self.mean[i] = values
                    .get(i)
                    .copied()
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.5);
            }
            self.initialized = true;
            self.seen = 1;
            return;
        }
        let a = self.alpha;
        for i in 0..n {
            let x = values
                .get(i)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(self.mean[i]);
            let prev = self.mean[i];
            let new = a * x + (1.0 - a) * prev;
            let delta = x - new;
            let prev_delta = prev - new;
            let instant_var = delta * delta;
            self.var[i] = (1.0 - a) * (self.var[i] + prev_delta * prev_delta * a) + a * instant_var;
            self.mean[i] = new;
        }
        self.seen += 1;
    }

    /// Feed a batch.
    pub fn observe_batch(&mut self, traces: &[Vec<f64>]) {
        for t in traces {
            self.observe(t);
        }
    }

    /// Current mean of series `i`.
    pub fn mean(&self, i: usize) -> f64 {
        self.mean.get(i).copied().unwrap_or(0.5)
    }

    /// Standard deviation of series `i` (floored at `1e-6`).
    pub fn std(&self, i: usize) -> f64 {
        self.var.get(i).copied().unwrap_or(0.0).max(1e-12).sqrt()
    }

    /// Z-score of `value` against series `i`.
    pub fn z(&self, i: usize, value: f64) -> f64 {
        (value - self.mean(i)) / self.std(i)
    }
}

/// Closed-form linear calibrator.
///
/// 1. Per-feedback: `target_scale = expected / observed`, blended toward 1
///    with `feedback_gain`.
/// 2. Residual against the baseline mean over a trailing window → offset.
/// 3. EMA-smooth coefficients toward identity so a single feedback cannot
///    jump the mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearCalibration {
    /// Trailing history window (default 50).
    pub window: usize,
    /// Feedback scale step in `(0, 1]` (default 0.3).
    pub feedback_gain: f64,
    /// Residual → offset gain (default 0.5).
    pub residual_gain: f64,
    /// Coefficient EMA toward identity (default 0.2).
    pub coeff_ema: f64,
}

impl Default for LinearCalibration {
    fn default() -> Self {
        Self {
            window: 50,
            feedback_gain: 0.3,
            residual_gain: 0.5,
            coeff_ema: 0.2,
        }
    }
}

impl LinearCalibration {
    /// Construct with a custom window.
    pub fn with_window(window: usize) -> Self {
        Self {
            window: window.max(1),
            ..Self::default()
        }
    }

    /// Stable calibrator name.
    pub fn name(&self) -> &'static str {
        "linear_ema_v1"
    }

    /// Fit coefficients for `baseline.len()` series.
    pub fn compute(
        &self,
        history: &[Vec<f64>],
        feedback: &[UserFeedback],
        baseline: &AdaptiveBaseline,
        now: i64,
    ) -> CalibrationCoefficients {
        let n = baseline.len();
        let mut coefs = CalibrationCoefficients::identity(n, history.len(), now);

        for fb in feedback {
            if let Some(i) = baseline.index_of(&fb.name) {
                let observed = fb.observed.max(1e-6);
                let target_scale = fb.expected / observed;
                let smoothed = 1.0 + self.feedback_gain * (target_scale - 1.0);
                coefs.coeffs[i].scale = smoothed.max(0.1);
            }
        }

        let window_traces: Vec<&Vec<f64>> = history.iter().rev().take(self.window).collect();
        if !window_traces.is_empty() {
            for i in 0..n {
                let residual: f64 = window_traces
                    .iter()
                    .map(|t| {
                        let x = t.get(i).copied().unwrap_or(baseline.mean(i));
                        baseline.mean(i) - x
                    })
                    .sum::<f64>()
                    / window_traces.len() as f64;
                coefs.coeffs[i].offset = self.residual_gain * residual;
            }
        }

        let ema = self.coeff_ema;
        for c in &mut coefs.coeffs {
            c.scale = ema * c.scale + (1.0 - ema) * 1.0;
            c.offset *= ema;
        }

        coefs
    }
}

/// One drift alarm for a named series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriftAlarm {
    /// Series name.
    pub name: String,
    /// Current value.
    pub current: f64,
    /// Baseline mean.
    pub mean: f64,
    /// Baseline standard deviation.
    pub std: f64,
    /// `(current − mean) / std`.
    pub z_score: f64,
    /// Consecutive-exceedances streak that triggered the alarm.
    pub streak: usize,
}

/// Per-series 2σ streak detector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetector {
    /// `|z|` threshold (default 2.0).
    pub z_threshold: f64,
    /// Consecutive exceedances required to alarm (default 3).
    pub window_threshold: usize,
    streak: Vec<usize>,
}

impl DriftDetector {
    /// Construct for `n` series.
    pub fn new(n: usize, z_threshold: f64, window_threshold: usize) -> Self {
        Self {
            z_threshold,
            window_threshold: window_threshold.max(1),
            streak: vec![0; n],
        }
    }

    /// Default 2σ / 3-streak detector for `n` series.
    pub fn with_len(n: usize) -> Self {
        Self::new(n, 2.0, 3)
    }

    /// Observe one vector against `baseline`. Returns every series whose
    /// streak has reached `window_threshold`. Extra / missing values are
    /// ignored / treated as in-band (reset the streak).
    pub fn observe(&mut self, values: &[f64], baseline: &AdaptiveBaseline) -> Vec<DriftAlarm> {
        let n = baseline.len().min(self.streak.len());
        let mut alarms = Vec::new();
        for i in 0..n {
            let Some(v) = values.get(i).copied().filter(|x| x.is_finite()) else {
                self.streak[i] = 0;
                continue;
            };
            let z = baseline.z(i, v);
            if z.abs() > self.z_threshold {
                self.streak[i] += 1;
            } else {
                self.streak[i] = 0;
            }
            if self.streak[i] >= self.window_threshold {
                alarms.push(DriftAlarm {
                    name: baseline
                        .names()
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("series-{i}")),
                    current: v,
                    mean: baseline.mean(i),
                    std: baseline.std(i),
                    z_score: z,
                    streak: self.streak[i],
                });
            }
        }
        alarms
    }

    /// Current streaks (read-only).
    pub fn streaks(&self) -> &[usize] {
        &self.streak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> [&'static str; 2] {
        ["thread_continuity", "fact_recall"]
    }

    fn vec2(v: f64) -> Vec<f64> {
        vec![v, v]
    }

    #[test]
    fn coeff_default_is_identity() {
        let c = Coeff::default();
        assert_eq!(c.scale, 1.0);
        assert_eq!(c.offset, 0.0);
        assert!((c.apply(0.5) - 0.5).abs() < 1e-12);
        assert!((c.apply(1.5) - 1.0).abs() < 1e-12);
        assert!((c.apply(-0.5) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn user_feedback_error_is_expected_minus_observed() {
        let fb = UserFeedback::new("thread_continuity", 0.5, 0.8, 0);
        assert!((fb.error() - 0.3).abs() < 1e-12);
    }

    #[test]
    fn adaptive_baseline_seeds_on_first_observation() {
        let mut b = AdaptiveBaseline::new(names(), 0.1);
        b.observe(&vec2(0.7));
        assert_eq!(b.seen(), 1);
        assert!((b.mean(0) - 0.7).abs() < 1e-12);
        assert!((b.mean(1) - 0.7).abs() < 1e-12);
    }

    #[test]
    fn adaptive_baseline_tracks_regime_change() {
        let mut b = AdaptiveBaseline::new(names(), 0.1);
        for _ in 0..50 {
            b.observe(&vec2(0.7));
        }
        for _ in 0..50 {
            b.observe(&vec2(0.3));
        }
        let m = b.mean(0);
        assert!(m < 0.7 && m > 0.3, "expected tracking, got {m}");
    }

    #[test]
    fn linear_calibration_with_feedback_moves_scale() {
        let cal = LinearCalibration::default();
        let baseline = AdaptiveBaseline::new(names(), 0.1);
        let history = vec![vec2(0.5); 10];
        let fb = vec![UserFeedback::new("thread_continuity", 0.5, 0.9, 0)];
        let coefs = cal.compute(&history, &fb, &baseline, 1000);
        assert!(
            coefs.coeffs[0].scale > 1.0,
            "scale should be > 1, got {}",
            coefs.coeffs[0].scale
        );
        assert!((coefs.coeffs[1].scale - 1.0).abs() < 0.3);
    }

    #[test]
    fn linear_calibration_no_feedback_returns_near_identity() {
        let cal = LinearCalibration::default();
        let mut baseline = AdaptiveBaseline::new(names(), 0.1);
        let history = vec![vec2(0.5); 10];
        baseline.observe_batch(&history);
        let coefs = cal.compute(&history, &[], &baseline, 1000);
        for c in &coefs.coeffs {
            assert!(
                (c.scale - 1.0).abs() < 0.3,
                "scale too far from 1: {}",
                c.scale
            );
        }
    }

    #[test]
    fn apply_coefficients_clamps() {
        let mut coefs = CalibrationCoefficients::identity(2, 0, 0);
        coefs.coeffs[0] = Coeff {
            scale: 10.0,
            offset: 0.0,
        };
        let adj = coefs.apply(&[0.5, 0.5]);
        assert!((adj[0] - 1.0).abs() < 1e-12);
        assert!((adj[1] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn dim_z_score_uses_rolling_baseline() {
        let mut b = AdaptiveBaseline::new(names(), 0.2);
        for _ in 0..20 {
            b.observe(&vec2(0.5));
        }
        let z = b.z(0, 0.5);
        assert!(
            z.abs() < 1.0,
            "z should be small for a stable signal, got {z}"
        );
    }

    #[test]
    fn drift_detector_alarms_on_streak() {
        let mut b = AdaptiveBaseline::new(names(), 0.1);
        for _ in 0..30 {
            b.observe(&vec2(0.5));
        }
        let mut d = DriftDetector::new(2, 2.0, 3);
        let mut last = Vec::new();
        for _ in 0..5 {
            last = d.observe(&[1.0, 0.5], &b);
        }
        assert!(
            last.iter().any(|a| a.name == "thread_continuity"),
            "expected alarm on series 0, got {last:?}"
        );
        assert!(!last.iter().any(|a| a.name == "fact_recall"));
    }
}
