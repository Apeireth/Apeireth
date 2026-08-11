//! # Histogram — 分桶统计 metrics
//!
//! Histogram 在一组边界 (buckets) 上累计样本数 (e.g. `request_duration_seconds`
//! buckets = `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]`).
//!
//! ## 1:1 翻译 v0.9.21 @anthropic-ai/metrics
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版   | 1:1 |
//! |------------------|--------------------------------|-----|
//! | `Histogram`      | `class Histogram`              | ✅  |
//! | `observe(v)`     | `observe(v)`                   | ✅  |
//! | `buckets`        | `buckets`                      | ✅  |
//!
//! ## 默认 buckets (per Prometheus convention)
//!
//! `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]` (11 个边界).
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::error::{MetricsError, MetricsResult};
use crate::label::{validate_labels, Label};
use crate::metric::{Metric, MetricValue};

// ============================================================================
// §1 默认 buckets (11 个边界, per Prometheus convention)
// ============================================================================

/// 默认 bucket 边界 (11 个, per Prometheus convention).
///
/// `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]`
pub const DEFAULT_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

// ============================================================================
// §2 Histogram 结构
// ============================================================================

/// Histogram — 分桶 metric.
#[derive(Debug)]
pub struct Histogram {
    /// metric 名.
    name: String,
    /// Help 文本 (K-1 强校验: 必填).
    help: String,
    /// label 集合 (K-1 强校验: ≤ 10).
    labels: HashMap<String, String>,
    /// bucket 边界 (升序, 编译期 hardcode 或用户传).
    bucket_bounds: Vec<f64>,
    /// 每个 bucket 内的样本数 (atomic, len = bucket_bounds.len()).
    bucket_counts: Vec<AtomicU64>,
    /// 总样本数.
    count: AtomicU64,
    /// 样本总和 (用 f64 bits 存, 跟 Gauge 同样策略).
    sum_bits: AtomicU64,
    /// 守护: bucket 边界不可重复.
    _guard: Mutex<()>,
}

impl Histogram {
    /// 构造 Histogram (使用默认 buckets).
    pub fn new(
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> MetricsResult<Self> {
        Self::with_buckets(name, help, labels, DEFAULT_BUCKETS.to_vec())
    }

    /// 构造 Histogram (自定义 buckets).
    ///
    /// K-1 强校验: buckets 必填 (至少 1 个), 升序, 无重复.
    pub fn with_buckets(
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
        buckets: Vec<f64>,
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
        validate_buckets(&buckets)?;

        let bucket_counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();

        Ok(Self {
            name,
            help,
            labels,
            bucket_bounds: buckets,
            bucket_counts,
            count: AtomicU64::new(0),
            sum_bits: AtomicU64::new(0.0_f64.to_bits()),
            _guard: Mutex::new(()),
        })
    }

    /// 观察一个值 v (落入 bucket 或 +Inf).
    pub fn observe(&self, v: f64) {
        if v.is_nan() {
            // NaN 不计入任何 bucket (跟 Prometheus 规范一致)
            return;
        }
        for (i, bound) in self.bucket_bounds.iter().enumerate() {
            if v <= *bound {
                self.bucket_counts[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
        // +Inf bucket: 总 count
        self.count.fetch_add(1, Ordering::Relaxed);

        // 更新 sum (CAS loop)
        loop {
            let current_bits = self.sum_bits.load(Ordering::Relaxed);
            let current = f64::from_bits(current_bits);
            let next = current + v;
            let next_bits = next.to_bits();
            if self
                .sum_bits
                .compare_exchange_weak(current_bits, next_bits, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }
    }

    /// 观察 n 个同样的值 v.
    pub fn observe_n(&self, v: f64, n: u64) {
        for _ in 0..n {
            self.observe(v);
        }
    }

    /// 当前总样本数.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// 当前 sum.
    pub fn sum(&self) -> f64 {
        f64::from_bits(self.sum_bits.load(Ordering::Relaxed))
    }

    /// bucket 边界.
    pub fn bucket_bounds(&self) -> &[f64] {
        &self.bucket_bounds
    }

    /// 每个 bucket 的非累加样本数 (per bucket, 跟 Prometheus 累加前的 raw 计数).
    pub fn bucket_counts(&self) -> Vec<u64> {
        self.bucket_counts
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .collect()
    }

    /// 计算分位数近似 (线性插值).
    pub fn quantile(&self, q: f64) -> f64 {
        if !(0.0..=1.0).contains(&q) {
            return f64::NAN;
        }
        let total = self.count();
        if total == 0 {
            return 0.0;
        }
        // 累加 bucket, 找到 q * total 对应的 bucket
        let target = (q * total as f64).ceil() as u64;
        let mut cumulative: u64 = 0;
        for (i, count) in self.bucket_counts.iter().enumerate() {
            cumulative = cumulative.saturating_add(count.load(Ordering::Relaxed));
            if cumulative >= target {
                return self.bucket_bounds[i];
            }
        }
        // 兜底: 比所有 bucket 都大, 返 sum / count
        if total > 0 {
            self.sum() / total as f64
        } else {
            0.0
        }
    }

    /// label 集合引用.
    pub fn label_pairs(&self) -> Vec<Label> {
        self.labels
            .iter()
            .map(|(k, v)| Label::new_unchecked(k, v))
            .collect()
    }
}

impl Metric for Histogram {
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
        let buckets: Vec<(f64, u64)> = self
            .bucket_bounds
            .iter()
            .zip(self.bucket_counts.iter())
            .map(|(b, c)| (*b, c.load(Ordering::Relaxed)))
            .collect();
        MetricValue::Histogram {
            buckets,
            sum: self.sum(),
            count: self.count(),
        }
    }

    fn type_name(&self) -> &'static str {
        "histogram"
    }
}

// ============================================================================
// §3 辅助函数
// ============================================================================

/// 校验 buckets (K-1 强校验: 至少 1 个 + 升序 + 无重复).
pub fn validate_buckets(buckets: &[f64]) -> MetricsResult<()> {
    if buckets.is_empty() {
        return Err(MetricsError::EncodeError(
            "buckets must not be empty".to_string(),
        ));
    }
    for window in buckets.windows(2) {
        if window[0] >= window[1] {
            return Err(MetricsError::EncodeError(format!(
                "buckets must be strictly ascending: {} >= {}",
                window[0], window[1]
            )));
        }
    }
    for b in buckets {
        if !b.is_finite() || *b <= 0.0 {
            return Err(MetricsError::EncodeError(format!(
                "bucket bound must be > 0 and finite: {b}"
            )));
        }
    }
    Ok(())
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 默认 buckets 11 个.
    #[test]
    fn default_buckets_count() {
        assert_eq!(DEFAULT_BUCKETS.len(), 11);
    }

    /// 守门 #2: 默认 buckets 升序.
    #[test]
    fn default_buckets_sorted() {
        for w in DEFAULT_BUCKETS.windows(2) {
            assert!(w[0] < w[1], "buckets not sorted: {} >= {}", w[0], w[1]);
        }
    }

    /// 守门 #3: 构造 + 初始 count = 0.
    #[test]
    fn histogram_new_initial_zero() {
        let h = Histogram::new("latency", "Latency", HashMap::new()).unwrap();
        assert_eq!(h.count(), 0);
        assert_eq!(h.sum(), 0.0);
    }

    /// 守门 #4: observe 落入正确 bucket.
    #[test]
    fn histogram_observe_bucket() {
        let h = Histogram::new("latency", "Latency", HashMap::new()).unwrap();
        h.observe(0.003); // ≤ 0.005
        h.observe(0.05); // ≤ 0.05
        h.observe(0.5); // ≤ 0.5
        h.observe(15.0); // > 10, 不入任何 bucket 但 +Inf count
        let counts = h.bucket_counts();
        // 0.005 bucket: 1 (只 0.003)
        assert_eq!(counts[0], 1);
        // 0.05 bucket: 1 (0.05)
        assert_eq!(counts[3], 1);
        // 0.5 bucket: 1 (0.5)
        assert_eq!(counts[6], 1);
        // 总数 = 4
        assert_eq!(h.count(), 4);
        // sum = 0.003 + 0.05 + 0.5 + 15 = 15.553
        assert!((h.sum() - 15.553).abs() < 1e-9);
    }

    /// 守门 #5: quantile 近似.
    #[test]
    fn histogram_quantile() {
        let h = Histogram::new("latency", "Latency", HashMap::new()).unwrap();
        // 10 个值 = 0.001, 0.002, ..., 0.01 (全 ≤ 0.01)
        for i in 1..=10 {
            h.observe(i as f64 * 0.001);
        }
        // p50: target = ceil(0.5*10) = 5, 累加 bucket 找到 0.005 bound
        let p50 = h.quantile(0.5);
        assert!((p50 - 0.005).abs() < 1e-9, "p50 = {p50}");
        // p90: target = ceil(0.9*10) = 9, 累加 bucket 找到 0.01 bound
        // (注意: 第 9 个样本 0.009 落在 0.01 bucket 内, 累加第 9 个时返 0.01)
        let p90 = h.quantile(0.9);
        assert!((p90 - 0.01).abs() < 1e-9, "p90 = {p90}");
    }

    /// 守门 #6: K-1 empty buckets 拒.
    #[test]
    fn k1_empty_buckets_rejected() {
        let r = Histogram::with_buckets("h", "h", HashMap::new(), vec![]);
        assert!(r.is_err());
    }

    /// 守门 #7: K-1 降序 buckets 拒.
    #[test]
    fn k1_descending_buckets_rejected() {
        let r = Histogram::with_buckets("h", "h", HashMap::new(), vec![1.0, 0.5, 0.1]);
        assert!(r.is_err());
    }

    /// 守门 #8: K-1 含 0 边界拒.
    #[test]
    fn k1_zero_bucket_rejected() {
        let r = Histogram::with_buckets("h", "h", HashMap::new(), vec![0.0, 1.0, 2.0]);
        assert!(r.is_err());
    }

    /// 守门 #9: K-1 NaN observe 不入 bucket 不增 count.
    #[test]
    fn k1_nan_observation_ignored() {
        let h = Histogram::new("h", "h", HashMap::new()).unwrap();
        h.observe(f64::NAN);
        assert_eq!(h.count(), 0);
    }

    /// 守门 #10: value() 返 Histogram 变体.
    #[test]
    fn histogram_value_variant() {
        let h = Histogram::new("h", "h", HashMap::new()).unwrap();
        h.observe(0.1);
        h.observe(0.5);
        let v = h.value();
        assert!(v.is_histogram());
        if let MetricValue::Histogram { count, sum, .. } = v {
            assert_eq!(count, 2);
            assert!((sum - 0.6).abs() < 1e-9);
        } else {
            panic!("expected Histogram variant");
        }
    }

    /// 守门 #11: type_name = "histogram".
    #[test]
    fn histogram_type_name() {
        let h = Histogram::new("h", "h", HashMap::new()).unwrap();
        assert_eq!(h.type_name(), "histogram");
    }

    /// 守门 #12: 1000 并发 observe → 1000.
    #[test]
    fn histogram_concurrent_observe() {
        use std::sync::Arc;
        use std::thread;
        let h = Arc::new(Histogram::new("h", "h", HashMap::new()).unwrap());
        let mut handles = vec![];
        for _ in 0..1000 {
            let hh = Arc::clone(&h);
            handles.push(thread::spawn(move || {
                hh.observe(0.05);
            }));
        }
        for hd in handles {
            hd.join().unwrap();
        }
        assert_eq!(h.count(), 1000);
    }

    /// 守门 #13: observe_n 累加.
    #[test]
    fn histogram_observe_n() {
        let h = Histogram::new("h", "h", HashMap::new()).unwrap();
        h.observe_n(0.05, 100);
        assert_eq!(h.count(), 100);
    }

    /// 守门 #14: 0 count 时 quantile = 0.
    #[test]
    fn histogram_quantile_empty() {
        let h = Histogram::new("h", "h", HashMap::new()).unwrap();
        assert_eq!(h.quantile(0.5), 0.0);
    }

    /// 守门 #15: K-1 out-of-range q 返 NaN.
    #[test]
    fn histogram_quantile_oob() {
        let h = Histogram::new("h", "h", HashMap::new()).unwrap();
        h.observe(0.1);
        assert!(h.quantile(1.5).is_nan());
        assert!(h.quantile(-0.1).is_nan());
    }
}
