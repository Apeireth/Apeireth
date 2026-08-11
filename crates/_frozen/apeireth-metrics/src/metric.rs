//! # Metric — 核心 trait + MetricValue 枚举
//!
//! 1:1 翻译 v0.9.21 @anthropic-ai/metrics:
//! - `Metric` trait — 4 metric 类型通用接口
//! - `MetricValue` enum — Counter(u64) / Gauge(f64) / Histogram / Summary
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;

// ============================================================================
// §1 MetricValue 枚举 (4 variant)
// ============================================================================

/// Metric 当前值.
///
/// - `Counter(u64)` — 单调递增整数
/// - `Gauge(f64)` — 任意增减浮点
/// - `Histogram { buckets, sum, count }` — 分桶 + sum + count
/// - `Summary { quantiles, sum, count }` — 分位数 + sum + count
#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    /// Counter 当前累计值.
    Counter(u64),
    /// Gauge 当前值.
    Gauge(f64),
    /// Histogram 完整快照.
    Histogram {
        /// bucket 边界 → 该 bucket 内的样本数 (非累加, encoder 自己累加).
        buckets: Vec<(f64, u64)>,
        /// 所有样本的总和.
        sum: f64,
        /// 样本总数.
        count: u64,
    },
    /// Summary 完整快照.
    Summary {
        /// 分位数 → 估计值 (e.g. 0.5 → 0.123, 0.99 → 0.987).
        quantiles: Vec<(f64, f64)>,
        /// 所有样本的总和.
        sum: f64,
        /// 样本总数.
        count: u64,
    },
}

impl MetricValue {
    /// 类型名 (per Prometheus exposition format `# TYPE`).
    pub fn type_name(&self) -> &'static str {
        match self {
            MetricValue::Counter(_) => "counter",
            MetricValue::Gauge(_) => "gauge",
            MetricValue::Histogram { .. } => "histogram",
            MetricValue::Summary { .. } => "summary",
        }
    }

    /// 是否 Counter.
    pub fn is_counter(&self) -> bool {
        matches!(self, MetricValue::Counter(_))
    }

    /// 是否 Gauge.
    pub fn is_gauge(&self) -> bool {
        matches!(self, MetricValue::Gauge(_))
    }

    /// 是否 Histogram.
    pub fn is_histogram(&self) -> bool {
        matches!(self, MetricValue::Histogram { .. })
    }

    /// 是否 Summary.
    pub fn is_summary(&self) -> bool {
        matches!(self, MetricValue::Summary { .. })
    }
}

// ============================================================================
// §2 Metric trait
// ============================================================================

/// Metric 通用 trait (4 类型共享).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/metrics class Metric 商业版.
pub trait Metric: Send + Sync {
    /// metric 名 (per task spec §5, e.g. `requests_total`).
    fn name(&self) -> &str;

    /// Help 文本 (per task spec §5 K-1 强校验: 必填).
    fn help(&self) -> &str;

    /// label 集合 (per task spec §5, ≤ 10 个 K-1 守门).
    fn labels(&self) -> &HashMap<String, String>;

    /// 当前值.
    fn value(&self) -> MetricValue;

    /// 类型名 (跟 value().type_name() 一致, 但 trait 限定返回 'static str).
    fn type_name(&self) -> &'static str;

    /// metric 完整名 = "{namespace}_{subsystem}_{name}" (per task spec §5).
    fn full_name(&self, namespace: &str, subsystem: &str) -> String {
        format!("{namespace}_{subsystem}_{}", self.name())
    }
}

// ============================================================================
// §3 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: MetricValue type_name 4 段.
    #[test]
    fn metric_value_type_name() {
        assert_eq!(MetricValue::Counter(0).type_name(), "counter");
        assert_eq!(MetricValue::Gauge(0.0).type_name(), "gauge");
        assert_eq!(
            MetricValue::Histogram {
                buckets: vec![],
                sum: 0.0,
                count: 0
            }
            .type_name(),
            "histogram"
        );
        assert_eq!(
            MetricValue::Summary {
                quantiles: vec![],
                sum: 0.0,
                count: 0
            }
            .type_name(),
            "summary"
        );
    }

    /// 守门 #2: 4 is_* 守门.
    #[test]
    fn metric_value_is_kinds() {
        assert!(MetricValue::Counter(0).is_counter());
        assert!(!MetricValue::Counter(0).is_gauge());

        assert!(MetricValue::Gauge(0.0).is_gauge());
        assert!(!MetricValue::Gauge(0.0).is_counter());

        let h = MetricValue::Histogram {
            buckets: vec![],
            sum: 0.0,
            count: 0,
        };
        assert!(h.is_histogram());
        assert!(!h.is_counter());

        let s = MetricValue::Summary {
            quantiles: vec![],
            sum: 0.0,
            count: 0,
        };
        assert!(s.is_summary());
        assert!(!s.is_histogram());
    }

    /// 守门 #3: Metric trait 默认 full_name 拼接.
    #[test]
    fn metric_trait_full_name() {
        struct StubMetric;
        impl Metric for StubMetric {
            fn name(&self) -> &str {
                "requests_total"
            }
            fn help(&self) -> &str {
                "Total requests"
            }
            fn labels(&self) -> &HashMap<String, String> {
                static LABELS: std::sync::OnceLock<HashMap<String, String>> =
                    std::sync::OnceLock::new();
                LABELS.get_or_init(HashMap::new)
            }
            fn value(&self) -> MetricValue {
                MetricValue::Counter(0)
            }
            fn type_name(&self) -> &'static str {
                "counter"
            }
        }
        let m = StubMetric;
        assert_eq!(m.full_name("apeireth", "agent"), "apeireth_agent_requests_total");
        // 空 namespace + subsystem: format!("{}_{}_{}", "", "", "requests_total") = "__requests_total"
        assert_eq!(m.full_name("", ""), "__requests_total");
    }
}
