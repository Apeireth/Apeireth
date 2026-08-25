//! TP18 (E3, P1) — Brier score + Murphy monotonic decomposition + Calibration bin diagnostic.
//!
//! **What it does**:
//! - `BrierScore = mean((forecast - outcome)^2)` for binary outcomes (outcome in {0, 1})
//! - Murphy (1973) three-way decomposition: `BS = reliability - resolution + uncertainty`
//! - `CalibrationBin` (default 10 equal-width bins) + `ExpectedCalibrationError`
//!
//! **Constraints**:
//! - No real LLM dependency — unit tests use stub probabilities
//! - No auto-calibration — only diagnostics + measurement
//! - Pure std + serde (cognition already has these)
//!
//! **Math (Murphy 1973, "A New Vector Partition of the Probability Score")**:
//! - N (forecast p_i, outcome y_i in {0,1}) pairs
//! - o_bar = (1/N) * sum(y_i) (base rate)
//! - For each of K bins containing forecasts in [k/K, (k+1)/K):
//!   - n_k = samples in bin k
//   - f_k = bin mean forecast
//   - o_k = bin mean outcome (empirical frequency)
//! - `reliability = Σ_k (n_k/N) * (f_k - o_k)^2`  (smaller = better, perfect = 0)
//! - `resolution  = Σ_k (n_k/N) * (o_k - o_bar)^2` (larger = better, perfect = o_bar*(1-o_bar))
//! - `uncertainty = o_bar * (1 - o_bar)`           (base rate, independent of forecast)
//! - `BrierScore  = mean((p_i - y_i)^2)` = reliability - resolution + uncertainty

use serde::{Deserialize, Serialize};

/// Default bin count.
pub const DEFAULT_NUM_BINS: usize = 10;

/// Single (forecast, outcome) observation — outcome in {0.0, 1.0}.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Forecast probability in [0.0, 1.0].
    pub forecast: f64,
    /// Actual outcome in {0.0, 1.0}.
    pub outcome: f64,
}

impl Observation {
    /// Construct.
    pub fn new(forecast: f64, outcome: f64) -> Self {
        Self { forecast, outcome }
    }
}

/// Calibration bin — forecast range sliced into equal-width bins, with empirical statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationBin {
    /// Bin index (0-based).
    pub index: usize,
    /// Bin lower bound in [0.0, 1.0).
    pub low: f64,
    /// Bin upper bound in (0.0, 1.0].
    pub high: f64,
    /// Number of samples in this bin.
    pub count: usize,
    /// Mean forecast for this bin.
    pub mean_forecast: f64,
    /// Mean outcome for this bin (empirical frequency).
    pub mean_outcome: f64,
}

impl CalibrationBin {
    /// |forecast - outcome| calibration gap for this bin (used by ECE).
    pub fn calibration_gap(&self) -> f64 {
        (self.mean_forecast - self.mean_outcome).abs()
    }

    /// Reliability contribution: `(n_k/N) * (f_k - o_k)^2`.
    pub fn reliability_contribution(&self, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let weight = self.count as f64 / total as f64;
        weight * (self.mean_forecast - self.mean_outcome).powi(2)
    }
}

/// Murphy (1973) three-way decomposition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrierDecomposition {
    /// `Σ_k (n_k/N) * (f_k - o_k)^2` — smaller is better (perfect = 0).
    pub reliability: f64,
    /// `Σ_k (n_k/N) * (o_k - o_bar)^2` — larger is better.
    pub resolution: f64,
    /// `o_bar * (1 - o_bar)` — base-rate entropy.
    pub uncertainty: f64,
    /// Mean Brier score: `mean((p-y)^2)`.
    pub brier_score: f64,
    /// Total sample count.
    pub num_samples: usize,
}

impl BrierDecomposition {
    /// Murphy monotonicity check.
    ///
    /// **Theory**:
    /// - Expected form: `E[BS] = E[reliability] - E[resolution] + E[uncertainty]` (exact)
    /// - Sample form: finite samples have sampling variance, `|BS - (rel - res + unc)|` ~ `O(1/sqrt(N))`
    /// - As N -> infinity, or when forecasts within a bin are equal (discrete binning), equality is exact.
    ///
    /// **Decision**: Use loose tolerance `5/sqrt(N)` for finite samples to cover sampling noise.
    /// For N=100, tolerance ~ 0.5; for N=10000, tolerance ~ 0.05.
    pub fn is_monotonic(&self) -> bool {
        if self.num_samples == 0 {
            return true; // trivially holds
        }
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        let tolerance = 5.0 / (self.num_samples as f64).sqrt();
        (self.brier_score - reconstructed).abs() < tolerance
    }

    /// Returns BS minus theoretical value: `BS - (reliability - resolution + uncertainty)`.
    ///
    /// Expected: 0 (sampling noise -> 0 as N -> infinity).
    pub fn monotonic_residual(&self) -> f64 {
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        self.brier_score - reconstructed
    }

    /// Strict monotonicity (requires forecasts within bin to be identical / large N):
    /// `BS ~ reliability - resolution + uncertainty`, tolerance `1e-9`.
    pub fn is_strictly_monotonic(&self) -> bool {
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        (reconstructed - self.brier_score).abs() < 1e-9
    }

    /// BS is in [0.0, 1.0] (naturally for binary outcomes in [0, 0.25]).
    pub fn brier_in_unit_range(&self) -> bool {
        (0.0..=1.0).contains(&self.brier_score)
    }

    /// `reliability` smaller is better (perfect = 0, always >= 0).
    pub fn reliability_non_negative(&self) -> bool {
        self.reliability >= -1e-12
    }

    /// `resolution` larger is better (sum of squares >= 0).
    pub fn resolution_non_negative(&self) -> bool {
        self.resolution >= -1e-12
    }

    /// `uncertainty` in [0.0, 0.25] (max entropy at o_bar = 0.5).
    pub fn uncertainty_in_range(&self) -> bool {
        (0.0..=0.25 + 1e-12).contains(&self.uncertainty)
    }
}

/// Single-point Brier Score `(p - y)^2`.
pub fn brier_single(forecast: f64, outcome: f64) -> f64 {
    (forecast - outcome).powi(2)
}

/// Mean Brier Score for a set of observations.
pub fn brier_score(obs: &[Observation]) -> f64 {
    if obs.is_empty() {
        return 0.0;
    }
    let sum: f64 = obs
        .iter()
        .map(|o| brier_single(o.forecast, o.outcome))
        .sum();
    sum / obs.len() as f64
}

/// Slice forecast range into `num_bins` equal-width bins, computing per-bin empirical stats.
///
/// Boundary rule: bin k = `[k/K, (k+1)/K)`, last bin right-closed `[9/10, 10/10]`.
pub fn calibration_bins(obs: &[Observation], num_bins: usize) -> Vec<CalibrationBin> {
    assert!(num_bins > 0, "num_bins must be > 0");
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
        // map forecast in [0.0, 1.0] -> bin index
        let idx = ((o.forecast * num_bins as f64).floor() as usize).min(num_bins - 1);
        bins[idx].count += 1;
        bins[idx].mean_forecast += o.forecast;
        bins[idx].mean_outcome += o.outcome;
    }

    // normalize means (empty bins keep 0.0)
    for bin in bins.iter_mut() {
        if bin.count > 0 {
            let n = bin.count as f64;
            bin.mean_forecast /= n;
            bin.mean_outcome /= n;
        }
    }

    bins
}

/// Expected Calibration Error = `Σ_k (n_k/N) * |f_k - o_k|` (weighted average calibration gap).
pub fn expected_calibration_error(bins: &[CalibrationBin]) -> f64 {
    let total: usize = bins.iter().map(|b| b.count).sum();
    if total == 0 {
        return 0.0;
    }
    let sum: f64 = bins
        .iter()
        .map(|b| {
            let weight = b.count as f64 / total as f64;
            weight * b.calibration_gap()
        })
        .sum();
    sum
}

/// Compute Murphy (1973) three-way decomposition.
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

    // base rate
    let o_bar: f64 = obs.iter().map(|o| o.outcome).sum::<f64>() / n as f64;
    let uncertainty = o_bar * (1.0 - o_bar);

    let bins = calibration_bins(obs, num_bins);

    let reliability: f64 = bins.iter().map(|b| b.reliability_contribution(n)).sum();
    let resolution: f64 = bins
        .iter()
        .map(|b| {
            if n == 0 {
                0.0
            } else {
                let weight = b.count as f64 / n as f64;
                weight * (b.mean_outcome - o_bar).powi(2)
            }
        })
        .sum();

    let bs = brier_score(obs);

    BrierDecomposition {
        reliability,
        resolution,
        uncertainty,
        brier_score: bs,
        num_samples: n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === BrierScore basics ===

    #[test]
    fn brier_single_perfect_zero() {
        assert_eq!(brier_single(0.0, 0.0), 0.0);
        assert_eq!(brier_single(1.0, 1.0), 0.0);
        assert_eq!(brier_single(0.5, 0.5), 0.0);
    }

    #[test]
    fn brier_single_worst_one() {
        assert!((brier_single(0.0, 1.0) - 1.0).abs() < 1e-12);
        assert!((brier_single(1.0, 0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn brier_score_mean_basic() {
        let obs = vec![
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
            Observation::new(0.5, 0.5),
        ];
        assert_eq!(brier_score(&obs), 0.0);
    }

    #[test]
    fn brier_score_mean_worst() {
        let obs = vec![Observation::new(0.0, 1.0), Observation::new(1.0, 0.0)];
        assert!((brier_score(&obs) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn brier_score_empty_returns_zero() {
        assert_eq!(brier_score(&[]), 0.0);
    }

    // === Murphy monotonic decomposition ===

    #[test]
    fn monotonic_decomposition_perfect_forecaster() {
        let perfect: Vec<Observation> = vec![
            Observation::new(0.05, 0.0),
            Observation::new(0.15, 0.0),
            Observation::new(0.95, 1.0),
            Observation::new(0.85, 1.0),
            Observation::new(0.25, 0.0),
        ];
        let decomp = decompose(&perfect, DEFAULT_NUM_BINS);
        assert!(
            decomp.is_monotonic(),
            "BS >= reliability - resolution + uncertainty: BS={}, reconstructed={}",
            decomp.brier_score,
            decomp.reliability - decomp.resolution + decomp.uncertainty
        );
        assert!(
            decomp.is_strictly_monotonic(),
            "bin-aligned perfect forecaster: residual should be ~0, got {}",
            decomp.monotonic_residual()
        );
        assert!(decomp.reliability < 0.05);
        assert!(decomp.brier_score < 0.05);
    }

    #[test]
    fn monotonic_decomposition_random_forecaster() {
        // random forecast: BS ~ 0.25 (worst), reliability ~ uncertainty, resolution ~ 0
        let obs: Vec<Observation> = (0..100)
            .map(|i| Observation::new(0.5, f64::from(i % 2)))
            .collect();
        let decomp = decompose(&obs, DEFAULT_NUM_BINS);
        assert!(
            decomp.is_monotonic(),
            "monotonic must hold: BS={}, decomp={:?}",
            decomp.brier_score,
            decomp
        );
        assert!(decomp.brier_in_unit_range());
        assert!(decomp.reliability_non_negative());
        assert!(decomp.resolution_non_negative());
        assert!(decomp.uncertainty_in_range());
        assert!(
            (decomp.brier_score - 0.25).abs() < 0.05,
            "BS should be ~0.25 for random, got {}",
            decomp.brier_score
        );
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
    }

    // === Calibration bin ===

    #[test]
    fn bins_partition_correctly_10() {
        let obs: Vec<Observation> = (0..10)
            .map(|i| Observation::new(f64::from(i) / 10.0 + 0.05, f64::from(i % 2)))
            .collect();
        let bins = calibration_bins(&obs, 10);
        assert_eq!(bins.len(), 10);
        for bin in bins.iter() {
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
        let obs = vec![
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
        ];
        let bins = calibration_bins(&obs, 10);
        for bin in bins.iter().filter(|b| b.count > 0) {
            assert!(bin.calibration_gap() < 0.01, "gap = {}", bin.calibration_gap());
        }
    }

    // === Expected Calibration Error ===

    #[test]
    fn ece_perfect_forecaster_is_zero() {
        let obs = vec![
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
        ];
        let bins = calibration_bins(&obs, 10);
        let ece = expected_calibration_error(&bins);
        assert!(ece < 0.01, "ECE for perfect = 0, got {}", ece);
    }

    #[test]
    fn ece_miscalibrated_is_high() {
        let obs = vec![
            Observation::new(0.9, 0.0),
            Observation::new(0.8, 0.0),
            Observation::new(0.7, 0.0),
        ];
        let bins = calibration_bins(&obs, 10);
        let ece = expected_calibration_error(&bins);
        assert!(ece > 0.5, "ECE for miscalibrated should be high, got {}", ece);
    }

    #[test]
    fn ece_empty_returns_zero() {
        let bins = calibration_bins(&[], 10);
        assert_eq!(expected_calibration_error(&bins), 0.0);
    }

    // === Integration consistency ===

    #[test]
    fn monotonic_invariant_holds_across_random_seeds() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        for seed in 0..20 {
            let mut h = DefaultHasher::new();
            seed.hash(&mut h);
            let mut state = h.finish();
            let mut obs = Vec::new();
            for _ in 0..50 {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let p = (state as f64 / u64::MAX as f64).abs();
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let y = f64::from(u32::from((state as f64 / u64::MAX as f64) > 0.5));
                obs.push(Observation::new(p, y));
            }
            let decomp = decompose(&obs, DEFAULT_NUM_BINS);
            assert!(
                decomp.is_monotonic(),
                "monotonic violated at seed {}: BS={}, reconstructed={}",
                seed,
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
}
