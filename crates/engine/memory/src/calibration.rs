//! Murphy (1973) Brier 三分解 + 等宽 CalibrationBin + Expected Calibration Error.
//!
//! Canonical implementation module.
//!
//! v2 already has per-intent Brier (`intent_brier`) and an inline Wilson/Beta
//! posterior inside W1 World Model. This module is the missing **diagnostic**
//! layer: reliability / resolution / uncertainty, 10-bin reliability diagrams,
//! and ECE. It does not own a main loop, a provider, or a store.
//!
//! Naming: the single-observation squared error is [`brier_squared`]. The mean
//! over a slice is [`mean_brier_score`]. These names deliberately do not collide
//! with [`crate::intent_brier::brier_score`].

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Default number of equal-width probability bins.
pub const DEFAULT_NUM_BINS: usize = 10;

/// One `(forecast, outcome)` pair.
///
/// Production callers typically use a binary outcome in `{0.0, 1.0}`. Fractional
/// outcomes are still accepted (clamped to `[0, 1]`) so the Brier identity
/// `(p − y)² = 0` when `p = y` holds, matching the canonical diagnostic tests.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Forecast probability in `[0.0, 1.0]`.
    pub forecast: f64,
    /// Realized outcome in `[0.0, 1.0]` (binary in the usual case).
    pub outcome: f64,
}

impl Observation {
    /// Construct an observation. Non-finite values become `0.0`; finite values
    /// are clamped to the unit interval. Outcomes are **not** snapped to `{0, 1}`.
    pub fn new(forecast: f64, outcome: f64) -> Self {
        Self {
            forecast: if forecast.is_finite() {
                forecast.clamp(0.0, 1.0)
            } else {
                0.0
            },
            outcome: if outcome.is_finite() {
                outcome.clamp(0.0, 1.0)
            } else {
                0.0
            },
        }
    }

    /// Convenience constructor from a boolean hit.
    pub fn from_hit(forecast: f64, hit: bool) -> Self {
        Self::new(forecast, if hit { 1.0 } else { 0.0 })
    }
}

/// One equal-width calibration bin.
///
/// Default 10 bins: bin `k` covers `[k/K, (k+1)/K)`; the last bin is closed
/// on the right so `forecast = 1.0` lands in bin `K-1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationBin {
    /// 0-based bin index.
    pub index: usize,
    /// Inclusive lower bound in `[0.0, 1.0)`.
    pub low: f64,
    /// Exclusive upper bound except for the last bin, which is inclusive.
    pub high: f64,
    /// Number of observations that landed in this bin.
    pub count: usize,
    /// Mean forecast inside the bin (0.0 when empty).
    pub mean_forecast: f64,
    /// Empirical frequency inside the bin (0.0 when empty).
    pub mean_outcome: f64,
}

impl CalibrationBin {
    /// Absolute calibration gap `|mean_forecast - mean_outcome|`.
    pub fn calibration_gap(&self) -> f64 {
        (self.mean_forecast - self.mean_outcome).abs()
    }

    /// Contribution to Murphy reliability: `(n_k / N) * (f_k - o_k)²`.
    pub fn reliability_contribution(&self, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let weight = self.count as f64 / total as f64;
        weight * (self.mean_forecast - self.mean_outcome).powi(2)
    }
}

/// Murphy (1973) three-way partition of the Brier score.
///
/// `BS = reliability − resolution + uncertainty` in the infinite-sample /
/// constant-within-bin limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrierDecomposition {
    /// `Σ_k (n_k/N) * (f_k - o_k)²` — smaller is better (perfect = 0).
    pub reliability: f64,
    /// `Σ_k (n_k/N) * (o_k - ō)²` — larger is better.
    pub resolution: f64,
    /// `ō * (1 - ō)` — base-rate entropy, independent of the forecast.
    pub uncertainty: f64,
    /// Mean per-observation `(p - y)²`.
    pub brier_score: f64,
    /// Sample size.
    pub num_samples: usize,
}

impl BrierDecomposition {
    /// Finite-sample monotonicity check: `|BS − (rel − res + unc)| < 5/√N`.
    pub fn is_monotonic(&self) -> bool {
        if self.num_samples == 0 {
            return true;
        }
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        let tolerance = 5.0 / (self.num_samples as f64).sqrt();
        (self.brier_score - reconstructed).abs() < tolerance
    }

    /// `BS − (reliability − resolution + uncertainty)`.
    pub fn monotonic_residual(&self) -> f64 {
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        self.brier_score - reconstructed
    }

    /// Strict equality (bin-aligned / large-N): residual `< 1e-9`.
    pub fn is_strictly_monotonic(&self) -> bool {
        self.monotonic_residual().abs() < 1e-9
    }

    /// Brier score lies in `[0, 1]`.
    pub fn brier_in_unit_range(&self) -> bool {
        (0.0..=1.0).contains(&self.brier_score)
    }

    /// Reliability is non-negative (up to float noise).
    pub fn reliability_non_negative(&self) -> bool {
        self.reliability >= -1e-12
    }

    /// Resolution is non-negative (up to float noise).
    pub fn resolution_non_negative(&self) -> bool {
        self.resolution >= -1e-12
    }

    /// Uncertainty lies in `[0, 0.25]` (max at base rate 0.5).
    pub fn uncertainty_in_range(&self) -> bool {
        (0.0..=0.25 + 1e-12).contains(&self.uncertainty)
    }
}

/// Single-observation Brier: `(forecast − outcome)²`.
pub fn brier_squared(forecast: f64, outcome: f64) -> f64 {
    (forecast - outcome).powi(2)
}

/// Mean Brier score over a slice. Empty input returns `0.0`.
pub fn mean_brier_score(obs: &[Observation]) -> f64 {
    if obs.is_empty() {
        return 0.0;
    }
    let sum: f64 = obs
        .iter()
        .map(|o| brier_squared(o.forecast, o.outcome))
        .sum();
    sum / obs.len() as f64
}

/// Partition forecasts into `num_bins` equal-width bins.
///
/// Mapping: `idx = floor(p * K).min(K - 1)` so `p = 1.0` lands in the last bin.
pub fn calibration_bins(obs: &[Observation], num_bins: usize) -> Vec<CalibrationBin> {
    let num_bins = num_bins.max(1);
    let mut bins: Vec<CalibrationBin> = (0..num_bins)
        .map(|i| CalibrationBin {
            index: i,
            low: i as f64 / num_bins as f64,
            high: (i + 1) as f64 / num_bins as f64,
            count: 0,
            mean_forecast: 0.0,
            mean_outcome: 0.0,
        })
        .collect();

    for o in obs {
        let idx = ((o.forecast * num_bins as f64).floor() as usize).min(num_bins - 1);
        bins[idx].count += 1;
        bins[idx].mean_forecast += o.forecast;
        bins[idx].mean_outcome += o.outcome;
    }

    for bin in &mut bins {
        if bin.count > 0 {
            let n = bin.count as f64;
            bin.mean_forecast /= n;
            bin.mean_outcome /= n;
        }
    }

    bins
}

/// Expected Calibration Error: `Σ_k (n_k/N) * |f_k − o_k|`.
pub fn expected_calibration_error(bins: &[CalibrationBin]) -> f64 {
    let total: usize = bins.iter().map(|b| b.count).sum();
    if total == 0 {
        return 0.0;
    }
    bins.iter()
        .map(|b| {
            let weight = b.count as f64 / total as f64;
            weight * b.calibration_gap()
        })
        .sum()
}

/// Murphy three-way decomposition of a set of observations.
pub fn decompose(obs: &[Observation], num_bins: usize) -> BrierDecomposition {
    let n = obs.len();
    if n == 0 {
        return BrierDecomposition {
            reliability: 0.0,
            resolution: 0.0,
            uncertainty: 0.0,
            brier_score: 0.0,
            num_samples: 0,
        };
    }

    let o_bar: f64 = obs.iter().map(|o| o.outcome).sum::<f64>() / n as f64;
    let uncertainty = o_bar * (1.0 - o_bar);
    let bins = calibration_bins(obs, num_bins);

    let reliability: f64 = bins.iter().map(|b| b.reliability_contribution(n)).sum();
    let resolution: f64 = bins
        .iter()
        .map(|b| {
            let weight = b.count as f64 / n as f64;
            weight * (b.mean_outcome - o_bar).powi(2)
        })
        .sum();

    BrierDecomposition {
        reliability,
        resolution,
        uncertainty,
        brier_score: mean_brier_score(obs),
        num_samples: n,
    }
}

/// Convenience: decompose with [`DEFAULT_NUM_BINS`].
pub fn decompose_default(obs: &[Observation]) -> BrierDecomposition {
    decompose(obs, DEFAULT_NUM_BINS)
}

/// Convenience: ECE over default 10 bins.
pub fn ece_default(obs: &[Observation]) -> f64 {
    expected_calibration_error(&calibration_bins(obs, DEFAULT_NUM_BINS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brier_squared_perfect_zero() {
        assert_eq!(brier_squared(0.0, 0.0), 0.0);
        assert_eq!(brier_squared(1.0, 1.0), 0.0);
        assert_eq!(brier_squared(0.5, 0.5), 0.0);
    }

    #[test]
    fn brier_squared_worst_one() {
        assert!((brier_squared(0.0, 1.0) - 1.0).abs() < 1e-12);
        assert!((brier_squared(1.0, 0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mean_brier_score_mean_basic() {
        let obs = vec![
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
            Observation::new(0.5, 0.5),
        ];
        assert_eq!(mean_brier_score(&obs), 0.0);
    }

    #[test]
    fn mean_brier_score_mean_worst() {
        let obs = vec![Observation::new(0.0, 1.0), Observation::new(1.0, 0.0)];
        assert!((mean_brier_score(&obs) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mean_brier_score_empty_returns_zero() {
        assert_eq!(mean_brier_score(&[]), 0.0);
    }

    #[test]
    fn observation_clamps_without_snapping() {
        let o = Observation::new(1.5, 0.7);
        assert_eq!(o.forecast, 1.0);
        assert_eq!(o.outcome, 0.7);
        let o2 = Observation::new(f64::NAN, 0.2);
        assert_eq!(o2.forecast, 0.0);
        assert_eq!(o2.outcome, 0.2);
        let o3 = Observation::new(0.5, f64::NAN);
        assert_eq!(o3.outcome, 0.0);
    }

    #[test]
    fn monotonic_decomposition_perfect_forecaster() {
        let perfect = vec![
            Observation::new(0.05, 0.0),
            Observation::new(0.15, 0.0),
            Observation::new(0.95, 1.0),
            Observation::new(0.85, 1.0),
            Observation::new(0.25, 0.0),
        ];
        let decomp = decompose(&perfect, DEFAULT_NUM_BINS);
        assert!(
            decomp.is_monotonic(),
            "BS={}, reconstructed={}",
            decomp.brier_score,
            decomp.reliability - decomp.resolution + decomp.uncertainty
        );
        assert!(
            decomp.is_strictly_monotonic(),
            "residual {}",
            decomp.monotonic_residual()
        );
        assert!(decomp.reliability < 0.05);
        assert!(decomp.brier_score < 0.05);
    }

    #[test]
    fn monotonic_decomposition_random_forecaster() {
        let obs: Vec<Observation> = (0..100)
            .map(|i| Observation::new(0.5, f64::from(i % 2)))
            .collect();
        let decomp = decompose(&obs, DEFAULT_NUM_BINS);
        assert!(decomp.is_monotonic());
        assert!(decomp.brier_in_unit_range());
        assert!(decomp.reliability_non_negative());
        assert!(decomp.resolution_non_negative());
        assert!(decomp.uncertainty_in_range());
        assert!((decomp.brier_score - 0.25).abs() < 0.05);
    }

    #[test]
    fn monotonic_decomposition_skilled_forecaster() {
        let mut obs = Vec::new();
        for _ in 0..50 {
            obs.push(Observation::new(0.9, 1.0));
            obs.push(Observation::new(0.1, 0.0));
        }
        let decomp = decompose(&obs, DEFAULT_NUM_BINS);
        assert!(decomp.is_monotonic());
        assert!(decomp.brier_score < 0.1);
        assert!(decomp.resolution > decomp.reliability);
    }

    #[test]
    fn decompose_empty_returns_zero() {
        let decomp = decompose(&[], DEFAULT_NUM_BINS);
        assert_eq!(decomp.brier_score, 0.0);
        assert_eq!(decomp.reliability, 0.0);
        assert_eq!(decomp.resolution, 0.0);
        assert_eq!(decomp.uncertainty, 0.0);
        assert_eq!(decomp.num_samples, 0);
        assert!(decomp.is_monotonic());
    }

    #[test]
    fn bins_partition_correctly_10() {
        let obs: Vec<Observation> = (0..10)
            .map(|i| Observation::new(f64::from(i) / 10.0 + 0.05, f64::from(i % 2)))
            .collect();
        let bins = calibration_bins(&obs, 10);
        assert_eq!(bins.len(), 10);
        for bin in &bins {
            assert_eq!(bin.count, 1, "bin {} should have 1 obs", bin.index);
            assert_eq!(bin.low, bin.index as f64 / 10.0);
            assert_eq!(bin.high, (bin.index + 1) as f64 / 10.0);
        }
    }

    #[test]
    fn bins_handle_forecast_one_in_last_bin() {
        let obs = vec![Observation::new(1.0, 1.0)];
        let bins = calibration_bins(&obs, 10);
        assert_eq!(bins[9].count, 1);
    }

    #[test]
    fn bins_handle_forecast_zero_in_first_bin() {
        let obs = vec![Observation::new(0.0, 0.0)];
        let bins = calibration_bins(&obs, 10);
        assert_eq!(bins[0].count, 1);
    }

    #[test]
    fn bins_compute_mean_forecast_and_outcome() {
        let obs = vec![
            Observation::new(0.05, 0.0),
            Observation::new(0.15, 1.0),
            Observation::new(0.15, 1.0),
        ];
        let bins = calibration_bins(&obs, 10);
        assert_eq!(bins[1].count, 2);
        assert!((bins[1].mean_forecast - 0.15).abs() < 1e-9);
        assert!((bins[1].mean_outcome - 1.0).abs() < 1e-9);
    }

    #[test]
    fn bin_calibration_gap_perfect_forecaster_is_zero() {
        let obs = vec![Observation::new(0.0, 0.0), Observation::new(1.0, 1.0)];
        let bins = calibration_bins(&obs, 10);
        for bin in bins.iter().filter(|b| b.count > 0) {
            assert!(bin.calibration_gap() < 0.01);
        }
    }

    #[test]
    fn ece_perfect_forecaster_is_zero() {
        let obs = vec![
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
        ];
        assert!(ece_default(&obs) < 0.01);
    }

    #[test]
    fn ece_miscalibrated_is_high() {
        let obs = vec![
            Observation::new(0.9, 0.0),
            Observation::new(0.8, 0.0),
            Observation::new(0.7, 0.0),
        ];
        let ece = ece_default(&obs);
        assert!(ece > 0.5, "ECE for miscalibrated should be high, got {ece}");
    }

    #[test]
    fn ece_empty_returns_zero() {
        assert_eq!(ece_default(&[]), 0.0);
    }

    #[test]
    fn monotonic_invariant_holds_across_random_seeds() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        for seed in 0..20u64 {
            let mut h = DefaultHasher::new();
            seed.hash(&mut h);
            let mut state = h.finish();
            let mut obs = Vec::new();
            for _ in 0..50 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let p = (state as f64 / u64::MAX as f64).abs();
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let y = f64::from(u32::from((state as f64 / u64::MAX as f64) > 0.5));
                obs.push(Observation::new(p, y));
            }
            let decomp = decompose(&obs, DEFAULT_NUM_BINS);
            assert!(
                decomp.is_monotonic(),
                "monotonic violated at seed {seed}: BS={}, reconstructed={}",
                decomp.brier_score,
                decomp.reliability - decomp.resolution + decomp.uncertainty
            );
        }
    }

    #[test]
    fn brier_score_serialization_round_trip() {
        let obs = vec![Observation::new(0.3, 0.0), Observation::new(0.7, 1.0)];
        let json = serde_json::to_string(&obs).unwrap();
        let back: Vec<Observation> = serde_json::from_str(&json).unwrap();
        assert_eq!(obs, back);
    }

    #[test]
    fn intent_record_converts_via_from_hit() {
        let o = Observation::from_hit(0.9, true);
        assert_eq!(o.forecast, 0.9);
        assert_eq!(o.outcome, 1.0);
        assert!((brier_squared(o.forecast, o.outcome) - 0.01).abs() < 1e-9);
    }
}
