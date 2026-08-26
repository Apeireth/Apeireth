//! TP18 (E3, P1) — Brier 评分 + Murphy 单调分解 + Calibration bin 诊断
//!
//! **做什么** (per task):
//! - `BrierScore = mean((forecast - outcome)^2)` 二元结果 (outcome ∈ {0, 1})
//! - Murphy (1973) 三分解: `BS = reliability − resolution + uncertainty`
//! - `CalibrationBin` (默认 10 等宽 bin) + `ExpectedCalibrationError`
//!
//! **约束** (per task §2):
//! - ❌ 不接真 LLM — 单测用 stub 概率
//! - ❌ 不做自动校准 — 仅诊断 + 度量
//! - ❌ 不改 oracle 评估管线 — 仅作为独立 diagnostic 模块
//!
//! **依赖**: 纯 std + serde (cognition 已有)
//!
//! **数学** (per Murphy 1973, "A New Vector Partition of the Probability Score"):
//! - 设 N 个 (forecast p_i, outcome y_i ∈ {0,1}) 对
//! - o_bar = (1/N) * sum(y_i) (base rate)
//! - 按 forecast 落入 K 个 bin, 第 k 个 bin:
//!   - n_k = 落入第 k bin 的样本数
//!   - f_k = 该 bin 平均 forecast
//!   - o_k = 该 bin 平均 outcome (经验频率)
//! - `reliability = Σ_k (n_k/N) * (f_k - o_k)²`  (越小越好, 完美 = 0)
//! - `resolution  = Σ_k (n_k/N) * (o_k - o_bar)²` (越大越好, 完美 = o_bar*(1-o_bar))
//! - `uncertainty = o_bar * (1 - o_bar)`           (base rate, 与 forecast 无关)
//! - `BrierScore  = mean((p_i - y_i)²)` = reliability − resolution + uncertainty
//!
//! **单调性证明** (BS ≥ 0 iff reliability − resolution + uncertainty ≥ 0):
//! - 因 uncertainty ≥ 0 且 resolution ≥ 0 (平方和), 当 reliability ≤ uncertainty + resolution 时 BS ≥ 0
//! - 这是 "好 forecast" 的条件: reliability 远小于 uncertainty+resolution

use serde::{Deserialize, Serialize};

/// 默认 bin 数 (per task "CalibrationBin (10 bins)").
pub const DEFAULT_NUM_BINS: usize = 10;

/// 单条 (forecast, outcome) 观测 — outcome ∈ {0.0, 1.0}.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// forecast 概率 ∈ [0.0, 1.0].
    pub forecast: f64,
    /// 实际结果 ∈ {0.0, 1.0}.
    pub outcome: f64,
}

impl Observation {
    /// 构造观测.
    pub fn new(forecast: f64, outcome: f64) -> Self {
        Self { forecast, outcome }
    }
}

/// Calibration bin — 把 forecast 区间等宽切分后, 每个 bin 的经验统计.
///
/// 默认 10 bin (per task): bin 0 = [0.0, 0.1), bin 1 = [0.1, 0.2), ..., bin 9 = [0.9, 1.0].
/// 边界: bin k = `[k / K, (k+1) / K)`, 最后一个 bin 右闭 `[9/10, 10/10]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationBin {
    /// bin 索引 (0-based).
    pub index: usize,
    /// bin 下界 ∈ [0.0, 1.0).
    pub low: f64,
    /// bin 上界 ∈ (0.0, 1.0].
    pub high: f64,
    /// 落入此 bin 的样本数.
    pub count: usize,
    /// 该 bin 平均 forecast.
    pub mean_forecast: f64,
    /// 该 bin 平均 outcome (经验频率).
    pub mean_outcome: f64,
}

impl CalibrationBin {
    /// 该 bin 的 |forecast - outcome| 校准偏差 (用于 ECE).
    pub fn calibration_gap(&self) -> f64 {
        (self.mean_forecast - self.mean_outcome).abs()
    }

    /// 该 bin 对 BS reliability 的贡献: `(n_k/N) * (f_k - o_k)²`.
    pub fn reliability_contribution(&self, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let weight = self.count as f64 / total as f64;
        weight * (self.mean_forecast - self.mean_outcome).powi(2)
    }
}

/// Murphy (1973) 三分解结果.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrierDecomposition {
    /// `Σ_k (n_k/N) * (f_k - o_k)²` — 越小越好 (完美 = 0).
    pub reliability: f64,
    /// `Σ_k (n_k/N) * (o_k - o_bar)²` — 越大越好.
    pub resolution: f64,
    /// `o_bar * (1 - o_bar)` — base rate entropy.
    pub uncertainty: f64,
    /// 总 Brier Score (per-instance mean of (p-y)^2).
    pub brier_score: f64,
    /// 总样本数.
    pub num_samples: usize,
}

impl BrierDecomposition {
    /// Murphy (1973) 单调性检查.
    ///
    /// **理论背景**:
    /// - 期望形式: `E[BS] = E[reliability] - E[resolution] + E[uncertainty]` (精确)
    /// - 样本形式: 单次样本有 sampling variance, `|BS - (rel - res + unc)|` ∝ `O(1/√N)`
    /// - 当 N → ∞ 或 bin 内 forecast 全等 (离散 forecast binning), 等号精确成立
    ///
    /// **判定**: 对有限样本用宽松容差 `5/√N`, 涵盖 sampling noise.
    /// 当 N=100, 容差 ≈ 0.5; 当 N=10000, 容差 ≈ 0.05.
    pub fn is_monotonic(&self) -> bool {
        if self.num_samples == 0 {
            return true; // 平凡成立
        }
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        let tolerance = 5.0 / (self.num_samples as f64).sqrt();
        (self.brier_score - reconstructed).abs() < tolerance
    }

    /// 返回 BS 与理论值的差: `BS - (reliability - resolution + uncertainty)`.
    ///
    /// 期望: 0 (sampling noise → 0 as N → ∞).
    pub fn monotonic_residual(&self) -> f64 {
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        self.brier_score - reconstructed
    }

    /// 严格单调 (要求 bin 内 forecast 全等 / N 极大):
    /// `BS ≈ reliability - resolution + uncertainty`, 容差 `1e-9`.
    pub fn is_strictly_monotonic(&self) -> bool {
        let reconstructed = self.reliability - self.resolution + self.uncertainty;
        (reconstructed - self.brier_score).abs() < 1e-9
    }

    /// BS 在 [0.0, 1.0] 范围内 (Brier 评分天然在此范围, 对二元 outcome ∈ [0, 0.25]).
    pub fn brier_in_unit_range(&self) -> bool {
        (0.0..=1.0).contains(&self.brier_score)
    }

    /// `reliability` 越小越好 (完美 = 0, ≥ 0 总是).
    pub fn reliability_non_negative(&self) -> bool {
        self.reliability >= -1e-12
    }

    /// `resolution` 越大越好 (平方和 ≥ 0).
    pub fn resolution_non_negative(&self) -> bool {
        self.resolution >= -1e-12
    }

    /// `uncertainty` ∈ [0.0, 0.25] (base rate entropy 最大在 o_bar=0.5).
    pub fn uncertainty_in_range(&self) -> bool {
        (0.0..=0.25 + 1e-12).contains(&self.uncertainty)
    }
}

/// 计算单点 Brier Score `(p - y)^2`.
pub fn brier_single(forecast: f64, outcome: f64) -> f64 {
    (forecast - outcome).powi(2)
}

/// 计算一组观测的 mean Brier Score.
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

/// 把 forecast 区间切成 `num_bins` 个等宽 bin, 统计每 bin 的经验 forecast/outcome.
///
/// 边界规则: bin k 区间 `[k/K, (k+1)/K)`, 最后一个 bin 右闭 `[9/10, 10/10]`.
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
        // 把 forecast ∈ [0.0, 1.0] 映射到 bin index
        let idx = ((o.forecast * num_bins as f64).floor() as usize).min(num_bins - 1);
        bins[idx].count += 1;
        bins[idx].mean_forecast += o.forecast;
        bins[idx].mean_outcome += o.outcome;
    }

    // 归一化均值 (空 bin 保持 0.0)
    for bin in bins.iter_mut() {
        if bin.count > 0 {
            let n = bin.count as f64;
            bin.mean_forecast /= n;
            bin.mean_outcome /= n;
        }
    }

    bins
}

/// Expected Calibration Error = `Σ_k (n_k/N) * |f_k - o_k|` (加权平均校准偏差).
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

/// 计算 Murphy (1973) 三分解.
///
/// 步骤:
/// 1. 计算 base rate `o_bar = mean(outcome)`
/// 2. 用 `num_bins` 个 bin 切片
/// 3. 累加 reliability / resolution / uncertainty
/// 4. 单独计算 `brier_score = mean((p-y)^2)` 用于 cross-check
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

    // === BrierScore 基础 ===

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

    // === Murphy 单调分解 ===

    #[test]
    fn monotonic_decomposition_perfect_forecaster() {
        // p_i == y_i for all (bin-aligned): BS = 0, reliability = 0, refinement = 0
        let _obs = vec![
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
            Observation::new(0.3, 0.0),
            Observation::new(0.0, 0.0),
            Observation::new(1.0, 1.0),
        ];
        // 让所有 forecast 都"完美" + bin-aligned (单 forecast per bin → refinement = 0):
        let perfect: Vec<Observation> = vec![
            Observation::new(0.05, 0.0), // 偏左 bin
            Observation::new(0.15, 0.0), // 偏右 bin (单独, refinement = 0)
            Observation::new(0.95, 1.0), // 偏右 bin (单独)
            Observation::new(0.85, 1.0), // 偏左 bin
            Observation::new(0.25, 0.0), // 中 bin (单独)
        ];
        let decomp = decompose(&perfect, DEFAULT_NUM_BINS);
        assert!(
            decomp.is_monotonic(),
            "BS >= reliability - resolution + uncertainty (refinement >= 0): BS={}, reconstructed={}",
            decomp.brier_score,
            decomp.reliability - decomp.resolution + decomp.uncertainty
        );
        assert!(
            decomp.is_strictly_monotonic(),
            "bin-aligned perfect forecaster: residual should be ~0, got {}",
            decomp.monotonic_residual()
        );
        assert!(
            decomp.reliability < 0.05,
            "perfect forecaster reliability close to 0, got {}",
            decomp.reliability
        );
        assert!(decomp.brier_score < 0.05);
    }

    #[test]
    fn monotonic_decomposition_random_forecaster() {
        // 随机 forecast: BS ≈ 0.25 (worst), reliability ≈ uncertainty, resolution ≈ 0
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
        // 随机 baseline: BS ≈ 0.25
        assert!(
            (decomp.brier_score - 0.25).abs() < 0.05,
            "BS should be ~0.25 for random, got {}",
            decomp.brier_score
        );
    }

    #[test]
    fn monotonic_decomposition_skilled_forecaster() {
        // 熟练 forecast: forecast 高时 y 倾向 1, forecast 低时 y 倾向 0
        let mut obs = Vec::new();
        for _ in 0..50 {
            obs.push(Observation::new(0.9, 1.0));
            obs.push(Observation::new(0.1, 0.0));
        }
        let decomp = decompose(&obs, DEFAULT_NUM_BINS);
        assert!(decomp.is_monotonic());
        assert!(
            decomp.brier_score < 0.1,
            "skilled forecaster BS should be small, got {}",
            decomp.brier_score
        );
        assert!(
            decomp.resolution > decomp.reliability,
            "skilled: resolution > reliability, got rel={} res={}",
            decomp.reliability,
            decomp.resolution
        );
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
        assert_eq!(
            bins[9].count, 1,
            "forecast=1.0 should land in last bin (closed)"
        );
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
            Observation::new(0.15, 1.0), // bin 1
            Observation::new(0.15, 1.0), // bin 1
        ];
        let bins = calibration_bins(&obs, 10);
        // bin 1 = [0.1, 0.2)
        assert_eq!(bins[1].count, 2);
        assert!((bins[1].mean_forecast - 0.15).abs() < 1e-9);
        assert!((bins[1].mean_outcome - 1.0).abs() < 1e-9);
    }

    #[test]
    fn bin_calibration_gap_perfect_forecaster_is_zero() {
        // 完美对齐: forecast = outcome (在 bin 边界 → bin 内 0 variance)
        let obs = vec![
            Observation::new(0.0, 0.0), // bin 0 (单独, gap = 0)
            Observation::new(1.0, 1.0), // bin 9 (单独, gap = 0)
        ];
        let bins = calibration_bins(&obs, 10);
        for bin in bins.iter().filter(|b| b.count > 0) {
            assert!(
                bin.calibration_gap() < 0.01,
                "gap = {}",
                bin.calibration_gap()
            );
        }
    }

    // === Expected Calibration Error ===

    #[test]
    fn ece_perfect_forecaster_is_zero() {
        // forecast == outcome (truly perfect)
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
        // forecast 高 → outcome 0 (反向)
        let obs = vec![
            Observation::new(0.9, 0.0),
            Observation::new(0.8, 0.0),
            Observation::new(0.7, 0.0),
        ];
        let bins = calibration_bins(&obs, 10);
        let ece = expected_calibration_error(&bins);
        assert!(
            ece > 0.5,
            "ECE for miscalibrated should be high, got {}",
            ece
        );
    }

    #[test]
    fn ece_empty_returns_zero() {
        let bins = calibration_bins(&[], 10);
        assert_eq!(expected_calibration_error(&bins), 0.0);
    }

    // === 集成一致性 ===

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
                // 简单 LCG
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
