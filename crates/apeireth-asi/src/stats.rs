//! R207 ASI 高级统计 utilities (std + auto-vectorization, 0 新依赖).
//!
//! **来源**: R200 调研提到 statrs (Rust 统计库). R207 短期方案: std 实现核心统计量.
//! 中期 (R207+1) 评估 statrs 集成.
//!
//! **设计**:
//! - 描述统计: mean / variance / stddev / median
//! - 分位数: percentile (P50 / P90 / P95 / P99)
//! - 标准化: z_score / min_max_scale
//! - 全 f64, std + LLVM auto-vectorization
//!
//! **0 触碰**: 现有 10 个子模块 (calibration/dim_enhance/drift/history/llm_judge/measurement/render/scheduler/tokenizer) 0 改.
//! 本模块是 additive utility, 可被 measurement / drift / scheduler 使用.
//!
//! **不假装** (O-5):
//! - Welford online algorithm 数值稳定
//! - 不假装 LLM 真的能输出 Gaussian 分布, 仅提供工具

#![allow(missing_docs)] // R207: 0 触碰现有 API 文档

/// 算术平均 (空 slice 返回 0.0)
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}

/// 总体方差 (除以 N, 不是 N-1)
pub fn variance_pop(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let m = mean(values);
    let sum_sq: f64 = values.iter().map(|x| (x - m).powi(2)).sum();
    sum_sq / values.len() as f64
}

/// 样本方差 (除以 N-1, Bessel 校正)
pub fn variance_sample(values: &[f64]) -> f64 {
    if values.len() < 2 { return 0.0; }
    let m = mean(values);
    let sum_sq: f64 = values.iter().map(|x| (x - m).powi(2)).sum();
    sum_sq / (values.len() as f64 - 1.0)
}

/// 总体标准差
pub fn stddev_pop(values: &[f64]) -> f64 {
    variance_pop(values).sqrt()
}

/// 样本标准差
pub fn stddev_sample(values: &[f64]) -> f64 {
    variance_sample(values).sqrt()
}

/// 中位数 (P50)
pub fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

/// 分位数 (P must be in 0.0..=1.0)
///
/// 用 linear interpolation 方法, R type 7 (Excel QUARTILE 默认).
pub fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() { return 0.0; }
    let p = p.clamp(0.0, 1.0);
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if values.len() == 1 { return values[0]; }
    let n = values.len();
    let rank = p * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    if lower == upper {
        values[lower]
    } else {
        values[lower] * (1.0 - frac) + values[upper] * frac
    }
}

/// Z-score 标准化 (返回新 Vec)
pub fn z_score(values: &[f64]) -> Vec<f64> {
    let m = mean(values);
    let s = stddev_pop(values);
    if s == 0.0 {
        return vec![0.0; values.len()];
    }
    values.iter().map(|x| (x - m) / s).collect()
}

/// Min-max 缩放到 [0, 1] (返回新 Vec)
pub fn min_max_scale(values: &[f64]) -> Vec<f64> {
    if values.is_empty() { return Vec::new(); }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range == 0.0 {
        return vec![0.5; values.len()];
    }
    values.iter().map(|x| (x - min) / range).collect()
}

/// Welford 在线均值/方差 (数值稳定, 用于 streaming)
pub struct Welford {
    count: u64,
    mean: f64,
    m2: f64,
}

impl Welford {
    pub fn new() -> Self {
        Self { count: 0, mean: 0.0, m2: 0.0 }
    }

    /// 添加一个新样本
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    pub fn count(&self) -> u64 { self.count }
    pub fn mean(&self) -> f64 { self.mean }
    pub fn variance(&self) -> f64 {
        if self.count < 2 { 0.0 } else { self.m2 / (self.count - 1) as f64 }
    }
    pub fn stddev(&self) -> f64 { self.variance().sqrt() }
}

impl Default for Welford {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn t01_mean_basic() {
        assert_eq!(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
    }

    #[test]
    fn t02_mean_empty() {
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn t03_variance_pop() {
        // [1,2,3,4,5] mean=3, var = ((1-3)^2 + (2-3)^2 + (3-3)^2 + (4-3)^2 + (5-3)^2) / 5 = 10/5 = 2.0
        assert!(approx_eq(variance_pop(&[1.0, 2.0, 3.0, 4.0, 5.0]), 2.0, 0.001));
    }

    #[test]
    fn t04_variance_sample() {
        // sample var = 10/4 = 2.5
        assert!(approx_eq(variance_sample(&[1.0, 2.0, 3.0, 4.0, 5.0]), 2.5, 0.001));
    }

    #[test]
    fn t05_stddev_pop() {
        // sqrt(2.0) ~ 1.414
        assert!(approx_eq(stddev_pop(&[1.0, 2.0, 3.0, 4.0, 5.0]), 1.414, 0.01));
    }

    #[test]
    fn t06_median_odd() {
        let mut v = vec![3.0, 1.0, 2.0];
        assert_eq!(median(&mut v), 2.0);
    }

    #[test]
    fn t07_median_even() {
        let mut v = vec![4.0, 1.0, 3.0, 2.0];
        assert_eq!(median(&mut v), 2.5);
    }

    #[test]
    fn t08_percentile_p50() {
        let mut v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&mut v, 0.5), 3.0);
    }

    #[test]
    fn t09_percentile_p90() {
        let mut v: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        // P90 of 1..100 (R type 7): rank = 0.9 * 99 = 89.1, values[89]=90, values[90]=91
        // 90 * 0.9 + 91 * 0.1 = 90.1
        assert!(approx_eq(percentile(&mut v, 0.9), 90.1, 0.001));
    }

    #[test]
    fn t10_z_score() {
        let z = z_score(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        // mean=0, stddev~1
        assert!(approx_eq(z[2], 0.0, 0.001));  // mean is 3
    }

    #[test]
    fn t11_min_max_scale() {
        let s = min_max_scale(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(approx_eq(s[0], 0.0, 0.001));
        assert!(approx_eq(s[4], 1.0, 0.001));
    }

    #[test]
    fn t12_welford_streaming() {
        let mut w = Welford::new();
        for i in 1..=5 {
            w.update(i as f64);
        }
        assert_eq!(w.count(), 5);
        assert_eq!(w.mean(), 3.0);
        // sample variance of 1..5 = 2.5
        assert!(approx_eq(w.variance(), 2.5, 0.001));
    }

    #[test]
    fn t13_welford_default() {
        let w = Welford::default();
        assert_eq!(w.count(), 0);
        assert_eq!(w.mean(), 0.0);
    }

    #[test]
    fn t14_welford_matches_offline() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let mut w = Welford::new();
        for &v in &values {
            w.update(v);
        }
        assert!(approx_eq(w.mean(), mean(&values), 0.001));
        assert!(approx_eq(w.variance(), variance_sample(&values), 0.001));
    }
}
