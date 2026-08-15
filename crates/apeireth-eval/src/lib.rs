
//! apeireth-eval — R23 6 module eval 子模块。
//!
//! R23 P1 #5 实质化: 加 +6 顶层 pub fn — score aggregation + percentile + 跨维度综合.
//! 不假装: 真计算 stddev / percentile / weighted aggregate, 全是 stdlib + 数学 (0 new dep).
//!
//! R32-3: 加 `smoke_task` module — 真接 1 个 task (conventions_scanner + tool_loop 集成).
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.

use serde::{Deserialize, Serialize};

pub mod cross_model_benchmark;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod mcp_bridge;  // R114: EvalScenario → MCP ToolServer 桥接
pub mod real_llm_smoke;
pub mod smoke_task;
// R150 P1 #11: SWE-bench 风格 task runner (借鉴 SWE-bench Verified 1.0)
pub mod swe_bench;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalScore { pub dimension: String, pub value: f64 }
impl EvalScore {
    pub fn new(dimension: impl Into<String>, value: f64) -> Self { Self { dimension: dimension.into(), value } }
    pub fn is_valid(&self) -> bool { self.value.is_finite() && (0.0..=1.0).contains(&self.value) }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — EvalScore aggregation
// ============================================================================

/// 多维度 score 求 arithmetic mean. 仅 valid score 参与计算, invalid 跳过.
///
/// 不假装: 真数 mean. 0 valid 输入返 `None` (不是 0.0, 防止混淆 "0 valid" vs "全是 0 分").
pub fn mean(scores: &[EvalScore]) -> Option<f64> {
    let valid: Vec<f64> = scores.iter().filter(|s| s.is_valid()).map(|s| s.value).collect();
    if valid.is_empty() { return None; }
    Some(valid.iter().sum::<f64>() / valid.len() as f64)
}

/// 多维度 score 求 weighted mean. `weights` 长度必须等于 `scores`,
/// 每个 weight 必须 finite + >= 0. weight 总和 0 时返 `None` (避免除零).
pub fn weighted_mean(scores: &[EvalScore], weights: &[f64]) -> Option<f64> {
    if scores.len() != weights.len() { return None; }
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (s, w) in scores.iter().zip(weights) {
        if !s.is_valid() || !w.is_finite() || *w < 0.0 { return None; }
        total += s.value * w;
        weight_sum += w;
    }
    if weight_sum == 0.0 { return None; }
    Some(total / weight_sum)
}

/// 多维度 sample standard deviation (Bessel correction). 1 个以下 input 返 0.0.
pub fn stddev(scores: &[EvalScore]) -> f64 {
    let valid: Vec<f64> = scores.iter().filter(|s| s.is_valid()).map(|s| s.value).collect();
    if valid.len() < 2 { return 0.0; }
    let m = mean(scores).unwrap_or(0.0);
    let variance = valid.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (valid.len() - 1) as f64;
    variance.sqrt()
}

/// Linear interpolation percentile (0.0..=1.0). 0 个 input 返 `None`.
/// p = 0.5 时是 median; p = 0.95 时是 95 分位.
pub fn percentile(scores: &[EvalScore], p: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&p) { return None; }
    let mut valid: Vec<f64> = scores.iter().filter(|s| s.is_valid()).map(|s| s.value).collect();
    if valid.is_empty() { return None; }
    valid.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if valid.len() == 1 { return Some(valid[0]); }
    let rank = p * (valid.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi { return Some(valid[lo]); }
    let frac = rank - lo as f64;
    Some(valid[lo] * (1.0 - frac) + valid[hi] * frac)
}

/// Validate `p` (0.0..=1.0) finite. 复用 EvalScore::is_valid 思路.
pub fn is_valid_percentile(p: f64) -> bool { p.is_finite() && (0.0..=1.0).contains(&p) }

#[cfg(test)]
mod tests {
    use super::*;
    fn scores() -> Vec<EvalScore> {
        vec![
            EvalScore::new("a", 0.5),
            EvalScore::new("b", 0.8),
            EvalScore::new("c", 1.0),
            EvalScore::new("d", 0.0),
            EvalScore::new("e", 1.5),  // invalid, 跳过
            EvalScore::new("f", f64::NAN),  // invalid, 跳过
        ]
    }

    #[test] fn valid_score_passes() { assert!(EvalScore::new("q", 0.85).is_valid()); }
    #[test] fn out_of_range_score_fails() { assert!(!EvalScore::new("q", 1.5).is_valid()); }
    #[test] fn nan_score_fails() { assert!(!EvalScore::new("q", f64::NAN).is_valid()); }

    #[test] fn mean_skips_invalid() {
        let m = mean(&scores()).unwrap();
        assert!((m - 0.575).abs() < 1e-9);  // (0.5+0.8+1.0+0.0)/4
    }
    #[test] fn mean_returns_none_for_all_invalid() {
        let all_bad = vec![EvalScore::new("x", 2.0)];
        assert!(mean(&all_bad).is_none());
    }
    #[test] fn weighted_mean_simple() {
        let s = vec![EvalScore::new("a", 0.5), EvalScore::new("b", 1.0)];
        assert_eq!(weighted_mean(&s, &[1.0, 1.0]), Some(0.75));
        assert_eq!(weighted_mean(&s, &[0.0, 0.0]), None);
    }
    #[test] fn weighted_mean_len_mismatch() {
        let s = vec![EvalScore::new("a", 0.5)];
        assert!(weighted_mean(&s, &[1.0, 2.0]).is_none());
    }
    #[test] fn stddev_basic() {
        // scores must be in [0, 1] to pass is_valid filter
        let s = vec![EvalScore::new("a", 0.1), EvalScore::new("b", 0.3)];
        // sample stddev of [0.1, 0.3]: mean=0.2, variance=((0.1-0.2)²+(0.3-0.2)²)/1=0.02
        // stddev = sqrt(0.02) ≈ 0.14142
        assert!((stddev(&s) - 0.02f64.sqrt()).abs() < 1e-9);
    }
    #[test] fn stddev_too_few() {
        assert_eq!(stddev(&[]), 0.0);
        assert_eq!(stddev(&[EvalScore::new("a", 0.5)]), 0.0);
    }
    #[test] fn percentile_median() {
        let s = vec![EvalScore::new("a", 0.1), EvalScore::new("b", 0.5), EvalScore::new("c", 0.9)];
        // sorted [0.1, 0.5, 0.9], p=0.5 → rank=1.0 → lo=hi=1 → valid[1]=0.5
        assert!((percentile(&s, 0.5).unwrap() - 0.5).abs() < 1e-9);
    }
    #[test] fn percentile_p95() {
        // valid range [0, 1] - use 20 valid scores 0.05..1.00 in 0.05 步长
        let vs: Vec<EvalScore> = (1..=20).map(|i| EvalScore::new("x", f64::from(i) / 20.0)).collect();
        let p95 = percentile(&vs, 0.95).unwrap();
        // p95 应接近 0.95 (头尾之间插值)
        assert!(p95 >= 0.90 && p95 <= 1.0, "p95 = {p95}");
    }
    #[test] fn percentile_invalid_p() {
        let s = vec![EvalScore::new("a", 0.5)];
        assert!(percentile(&s, 1.5).is_none());
        assert!(percentile(&s, -0.1).is_none());
    }
    #[test] fn percentile_empty() {
        assert!(percentile(&[], 0.5).is_none());
    }
    #[test] fn is_valid_percentile_basic() {
        assert!(is_valid_percentile(0.0));
        assert!(is_valid_percentile(0.5));
        assert!(is_valid_percentile(1.0));
        assert!(!is_valid_percentile(1.5));
        assert!(!is_valid_percentile(-0.1));
        assert!(!is_valid_percentile(f64::NAN));
    }
}
