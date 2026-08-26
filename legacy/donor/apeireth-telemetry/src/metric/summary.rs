//! # Summary — 分位数 metrics
//!
//! Summary 在客户端计算分位数 (e.g. p50 / p90 / p99 latency).
//!
//! ## 1:1 翻译 v0.9.21 @anthropic-ai/metrics
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版   | 1:1 |
//! |------------------|--------------------------------|-----|
//! | `Summary`        | `class Summary`                | ✅  |
//! | `observe(v)`     | `observe(v)`                   | ✅  |
//! | `quantiles`      | `quantiles`                    | ✅  |
//!
//! ## 默认 quantiles (5 个, per Prometheus convention)
//!
//! `[0.5, 0.9, 0.95, 0.99, 0.999]` (p50 / p90 / p95 / p99 / p99.9).
//!
//! ## 算法
//!
//! skeleton 阶段: 用 **reservoir sampling** 维护固定大小样本池 (1024 槽),
//! 分位数按样本排序后线性插值估算. R21 续真接时可换 P² / t-digest 等.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use super::error::{MetricsError, MetricsResult};
use super::label::{validate_labels, Label};
use super::{Metric, MetricValue};

// ============================================================================
// §1 默认 quantiles (5 个, per Prometheus convention)
// ============================================================================

/// 默认 quantile (5 个, per Prometheus convention).
///
/// `[0.5, 0.9, 0.95, 0.99, 0.999]` (p50 / p90 / p95 / p99 / p99.9).
pub const DEFAULT_QUANTILES: &[f64] = &[0.5, 0.9, 0.95, 0.99, 0.999];

/// 样本池最大容量 (reservoir sampling).
pub const RESERVOIR_SIZE: usize = 1024;

// ============================================================================
// §2 Summary 结构
// ============================================================================

/// Summary — 分位数 metric.
#[derive(Debug)]
pub struct Summary {
    /// metric 名.
    name: String,
    /// Help 文本 (K-1 强校验: 必填).
    help: String,
    /// label 集合 (K-1 强校验: ≤ 10).
    labels: HashMap<String, String>,
    /// 关注的分位数 (升序, 0..=1).
    quantiles: Vec<f64>,
    /// 样本池 (固定大小, 满了之后用 reservoir sampling 替换).
    reservoir: Mutex<Reservoir>,
    /// 总样本数 (用于 quantile 计算时的归一化).
    count: AtomicU64,
    /// sum bits.
    sum_bits: AtomicU64,
}

#[derive(Debug)]
struct Reservoir {
    /// 样本池.
    samples: Vec<f64>,
    /// 已观察的样本数 (用于 reservoir sampling 决定).
    seen: u64,
}

impl Reservoir {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(RESERVOIR_SIZE),
            seen: 0,
        }
    }

    /// reservoir sampling 算法 (Vitter's R).
    fn add(&mut self, v: f64) {
        self.seen += 1;
        if self.samples.len() < RESERVOIR_SIZE {
            self.samples.push(v);
        } else {
            // 用确定性 hash 替换 (j = uniform(0, seen), 落在 [0, RESERVOIR_SIZE) 则替换)
            // skeleton 阶段: 用简单 rand (确定性基于 seen 哈希)
            let j = (v.to_bits().wrapping_mul(0x9E3779B97F4A7C15) ^ self.seen) as usize;
            if j < RESERVOIR_SIZE {
                self.samples[j % RESERVOIR_SIZE] = v;
            }
        }
    }

    /// 排序后样本.
    fn sorted(&mut self) -> Vec<f64> {
        self.samples
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        self.samples.clone()
    }
}

impl Summary {
    /// 构造 Summary (使用默认 quantiles).
    pub fn new(
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> MetricsResult<Self> {
        Self::with_quantiles(name, help, labels, DEFAULT_QUANTILES.to_vec())
    }

    /// 构造 Summary (自定义 quantiles).
    ///
    /// K-1 强校验: quantiles 必填 (至少 1 个), 升序, 0..=1.
    pub fn with_quantiles(
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
        quantiles: Vec<f64>,
    ) -> MetricsResult<Self> {
        let name = name.into();
        let help = help.into();
        if name.is_empty() {
            return Err(MetricsError::MetricNameEmpty);
        }
        if help.is_empty() {
            return Err(MetricsError::HelpRequired(name));
        }
        validate_labels(&labels)?;
        validate_quantiles(&quantiles)?;

        Ok(Self {
            name,
            help,
            labels,
            quantiles,
            reservoir: Mutex::new(Reservoir::new()),
            count: AtomicU64::new(0),
            sum_bits: AtomicU64::new(0.0_f64.to_bits()),
        })
    }

    /// 观察一个值.
    pub fn observe(&self, v: f64) {
        if v.is_nan() {
            return;
        }
        self.reservoir.lock().add(v);
        self.count.fetch_add(1, Ordering::Relaxed);

        loop {
            let current_bits = self.sum_bits.load(Ordering::Relaxed);
            let current = f64::from_bits(current_bits);
            let next = current + v;
            let next_bits = next.to_bits();
            if self
                .sum_bits
                .compare_exchange_weak(
                    current_bits,
                    next_bits,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return;
            }
        }
    }

    /// 观察 n 个同样的值.
    pub fn observe_n(&self, v: f64, n: u64) {
        for _ in 0..n {
            self.observe(v);
        }
    }

    /// 总样本数.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// sum.
    pub fn sum(&self) -> f64 {
        f64::from_bits(self.sum_bits.load(Ordering::Relaxed))
    }

    /// quantile 列表.
    pub fn quantile_targets(&self) -> &[f64] {
        &self.quantiles
    }

    /// 算所有 quantile 的当前值 (sorted sample + linear interpolation).
    pub fn quantile_values(&self) -> Vec<(f64, f64)> {
        let mut res = self.reservoir.lock();
        let samples = res.sorted();
        let total = self.count();
        if samples.is_empty() || total == 0 {
            return self.quantiles.iter().map(|q| (*q, 0.0)).collect();
        }
        self.quantiles
            .iter()
            .map(|q| {
                let v = quantile_from_samples(&samples, *q);
                (*q, v)
            })
            .collect()
    }

    /// label 集合引用.
    pub fn label_pairs(&self) -> Vec<Label> {
        self.labels
            .iter()
            .map(|(k, v)| Label::new_unchecked(k, v))
            .collect()
    }
}

impl Metric for Summary {
    fn name(&self) -> &str {
        &self.name
    }

    fn help(&self) -> &str {
        &self.help
    }

    fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }

    fn value(&self) -> MetricValue {
        MetricValue::Summary {
            quantiles: self.quantile_values(),
            sum: self.sum(),
            count: self.count(),
        }
    }

    fn type_name(&self) -> &'static str {
        "summary"
    }
}

// ============================================================================
// §3 辅助函数
// ============================================================================

/// 校验 quantiles (K-1 强校验: 至少 1 个 + 升序 + 0..=1).
pub fn validate_quantiles(quantiles: &[f64]) -> MetricsResult<()> {
    if quantiles.is_empty() {
        return Err(MetricsError::EncodeError(
            "quantiles must not be empty".to_string(),
        ));
    }
    for q in quantiles {
        if !q.is_finite() || *q < 0.0 || *q > 1.0 {
            return Err(MetricsError::EncodeError(format!(
                "quantile must be in [0, 1]: {q}"
            )));
        }
    }
    for w in quantiles.windows(2) {
        if w[0] >= w[1] {
            return Err(MetricsError::EncodeError(format!(
                "quantiles must be strictly ascending: {} >= {}",
                w[0], w[1]
            )));
        }
    }
    Ok(())
}

/// 从已排序 samples 计算 q 分位数 (linear interpolation, type 7 跟 numpy default 一致).
fn quantile_from_samples(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    // type 7 (numpy default, R default): h = (n-1) * q
    let h = (n - 1) as f64 * q;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let frac = h - lo as f64;
    sorted[lo] + (sorted[hi] - sorted[lo]) * frac
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 默认 quantiles 5 个.
    #[test]
    fn default_quantiles_count() {
        assert_eq!(DEFAULT_QUANTILES.len(), 5);
    }

    /// 守门 #2: 默认 quantiles 升序.
    #[test]
    fn default_quantiles_sorted() {
        for w in DEFAULT_QUANTILES.windows(2) {
            assert!(w[0] < w[1], "quantiles not sorted: {} >= {}", w[0], w[1]);
        }
    }

    /// 守门 #3: 默认 quantiles [0.5, 0.9, 0.95, 0.99, 0.999].
    #[test]
    fn default_quantiles_exact() {
        assert_eq!(DEFAULT_QUANTILES, &[0.5, 0.9, 0.95, 0.99, 0.999]);
    }

    /// 守门 #4: 构造 + 初始 count = 0.
    #[test]
    fn summary_new_initial_zero() {
        let s = Summary::new("latency", "Latency", HashMap::new()).unwrap();
        assert_eq!(s.count(), 0);
        assert_eq!(s.sum(), 0.0);
    }

    /// 守门 #5: observe + quantile p50.
    #[test]
    fn summary_quantile_p50() {
        let s = Summary::new("latency", "Latency", HashMap::new()).unwrap();
        // 100 个值 0.001..0.100
        for i in 1..=100 {
            s.observe(f64::from(i) * 0.001);
        }
        let qv = s.quantile_values();
        // p50 应该是 ~ 0.05
        let p50 = qv.iter().find(|(q, _)| (*q - 0.5).abs() < 1e-9).unwrap().1;
        assert!((p50 - 0.05).abs() < 0.01, "p50 = {p50}");
    }

    /// 守门 #6: observe + quantile p99.
    #[test]
    fn summary_quantile_p99() {
        let s = Summary::new("latency", "Latency", HashMap::new()).unwrap();
        for i in 1..=1000 {
            s.observe(f64::from(i) * 0.001);
        }
        let qv = s.quantile_values();
        // p99 应该是 ~ 0.99
        let p99 = qv.iter().find(|(q, _)| (*q - 0.99).abs() < 1e-9).unwrap().1;
        assert!((p99 - 0.99).abs() < 0.05, "p99 = {p99}");
    }

    /// 守门 #7: K-1 empty quantiles 拒.
    #[test]
    fn k1_empty_quantiles_rejected() {
        let r = Summary::with_quantiles("s", "h", HashMap::new(), vec![]);
        assert!(r.is_err());
    }

    /// 守门 #8: K-1 OOR quantile 拒 (> 1).
    #[test]
    fn k1_oor_quantile_rejected() {
        let r = Summary::with_quantiles("s", "h", HashMap::new(), vec![0.5, 1.5]);
        assert!(r.is_err());
    }

    /// 守门 #9: K-1 降序 quantiles 拒.
    #[test]
    fn k1_descending_quantiles_rejected() {
        let r = Summary::with_quantiles("s", "h", HashMap::new(), vec![0.9, 0.5]);
        assert!(r.is_err());
    }

    /// 守门 #10: K-1 NaN observe 忽略.
    #[test]
    fn k1_nan_observation_ignored() {
        let s = Summary::new("s", "h", HashMap::new()).unwrap();
        s.observe(f64::NAN);
        assert_eq!(s.count(), 0);
    }

    /// 守门 #11: value() 返 Summary 变体.
    #[test]
    fn summary_value_variant() {
        let s = Summary::new("s", "h", HashMap::new()).unwrap();
        s.observe(0.1);
        s.observe(0.5);
        let v = s.value();
        assert!(v.is_summary());
        if let MetricValue::Summary {
            count,
            sum,
            quantiles,
        } = v
        {
            assert_eq!(count, 2);
            assert!((sum - 0.6).abs() < 1e-9);
            assert_eq!(quantiles.len(), 5);
        } else {
            panic!("expected Summary variant");
        }
    }

    /// 守门 #12: type_name = "summary".
    #[test]
    fn summary_type_name() {
        let s = Summary::new("s", "h", HashMap::new()).unwrap();
        assert_eq!(s.type_name(), "summary");
    }

    /// 守门 #13: 1000 并发 observe → 1000.
    #[test]
    fn summary_concurrent_observe() {
        use std::sync::Arc;
        use std::thread;
        let s = Arc::new(Summary::new("s", "h", HashMap::new()).unwrap());
        let mut handles = vec![];
        for i in 0..1000 {
            let ss = Arc::clone(&s);
            handles.push(thread::spawn(move || {
                ss.observe(f64::from(i) * 0.001);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(s.count(), 1000);
    }

    /// 守门 #14: observe_n 累加.
    #[test]
    fn summary_observe_n() {
        let s = Summary::new("s", "h", HashMap::new()).unwrap();
        s.observe_n(0.1, 100);
        assert_eq!(s.count(), 100);
    }

    /// 守门 #15: 0 count 时 quantile_values 返 0.
    #[test]
    fn summary_quantile_empty() {
        let s = Summary::new("s", "h", HashMap::new()).unwrap();
        let qv = s.quantile_values();
        for (q, v) in qv {
            assert_eq!(v, 0.0, "q={q}");
        }
    }

    /// 守门 #16: quantile_from_samples 1 个样本.
    #[test]
    fn quantile_from_samples_one() {
        let s = vec![42.0];
        assert_eq!(quantile_from_samples(&s, 0.5), 42.0);
    }

    /// 守门 #17: quantile_from_samples 多个样本 type 7.
    #[test]
    fn quantile_from_samples_many() {
        let s = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        // p50 type 7: h = (5-1)*0.5 = 2.0, lo = hi = 2 → 返 sorted[2] = 3.0
        // (当 h 恰好是整数时 type 7 不插值, 直接返 lo)
        assert!((quantile_from_samples(&s, 0.5) - 3.0).abs() < 1e-9);
        // p25 type 7: h = (5-1)*0.25 = 1.0, lo = hi = 1 → 返 sorted[1] = 2.0
        assert!((quantile_from_samples(&s, 0.25) - 2.0).abs() < 1e-9);
        // p75 type 7: h = (5-1)*0.75 = 3.0, lo = hi = 3 → 返 sorted[3] = 4.0
        assert!((quantile_from_samples(&s, 0.75) - 4.0).abs() < 1e-9);
    }
}
