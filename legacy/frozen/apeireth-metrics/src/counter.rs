//! # Counter — 单调递增 metrics
//!
//! Counter 是只增不减的累计值 (e.g. `requests_total`, `errors_total`).
//! 唯一允许的非加操作是 `reset()` (测试场景), 业务上应当持续递增.
//!
//! ## 1:1 翻译 v0.9.21 @anthropic-ai/metrics
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版   | 1:1 |
//! |------------------|--------------------------------|-----|
//! | `Counter`        | `class Counter`                | ✅  |
//! | `inc()`          | `inc()`                        | ✅  |
//! | `inc_by(n)`      | `inc(n)`                       | ✅  |
//! | `reset()`        | `reset()`                      | ✅  |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use crate::error::MetricsResult;
use crate::label::{validate_labels, Label};
use crate::metric::{Metric, MetricValue};

// ============================================================================
// §1 Counter 结构
// ============================================================================

/// Counter — 单调递增 metric.
#[derive(Debug)]
pub struct Counter {
    /// metric 名 (不带 namespace/subsystem 前缀).
    name: String,
    /// Help 文本 (K-1 强校验: 必填, 不能空字符串).
    help: String,
    /// label 集合 (K-1 强校验: ≤ 10).
    labels: HashMap<String, String>,
    /// 当前累计值 (atomic, 无锁并发).
    value: AtomicU64,
    /// 增量守护: 拒绝 inc_by(0) (K-1 强校验) — 实际上 inc_by(0) 是合法 no-op, 故仅在 docstring 中警告.
    _guard: RwLock<()>,
}

impl Counter {
    /// 构造 Counter.
    ///
    /// K-1 强校验: name 必填, help 必填, labels 合法.
    pub fn new(
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> MetricsResult<Self> {
        let name = name.into();
        let help = help.into();
        if name.is_empty() {
            return Err(crate::error::MetricsError::MetricNameEmpty);
        }
        if help.is_empty() {
            return Err(crate::error::MetricsError::HelpRequired(name));
        }
        validate_labels(&labels)?;
        Ok(Self {
            name,
            help,
            labels,
            value: AtomicU64::new(0),
            _guard: RwLock::new(()),
        })
    }

    /// +1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// +n (n = 0 是合法 no-op).
    pub fn inc_by(&self, n: u64) {
        if n == 0 {
            return;
        }
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// 当前值.
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// 重置为 0 (K-1 强校验: 仅测试场景, 业务逻辑慎用).
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    /// label 集合引用 (read-only).
    pub fn label_pairs(&self) -> Vec<Label> {
        self.labels
            .iter()
            .map(|(k, v)| Label::new_unchecked(k, v))
            .collect()
    }
}

impl Metric for Counter {
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
        MetricValue::Counter(self.get())
    }

    fn type_name(&self) -> &'static str {
        "counter"
    }
}

// ============================================================================
// §2 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("method".to_string(), "GET".to_string());
        m
    }

    /// 守门 #1: 构造 + 初始值 0.
    #[test]
    fn counter_new_initial_zero() {
        let c = Counter::new("requests_total", "Total requests", HashMap::new()).unwrap();
        assert_eq!(c.get(), 0);
        assert_eq!(c.name(), "requests_total");
        assert_eq!(c.help(), "Total requests");
    }

    /// 守门 #2: inc +1.
    #[test]
    fn counter_inc() {
        let c = Counter::new("c", "h", HashMap::new()).unwrap();
        c.inc();
        c.inc();
        c.inc();
        assert_eq!(c.get(), 3);
    }

    /// 守门 #3: inc_by n.
    #[test]
    fn counter_inc_by() {
        let c = Counter::new("c", "h", HashMap::new()).unwrap();
        c.inc_by(5);
        c.inc_by(10);
        assert_eq!(c.get(), 15);
    }

    /// 守门 #4: inc_by(0) 是合法 no-op.
    #[test]
    fn counter_inc_by_zero_noop() {
        let c = Counter::new("c", "h", HashMap::new()).unwrap();
        c.inc_by(0);
        assert_eq!(c.get(), 0);
    }

    /// 守门 #5: reset 只能重置为 0.
    #[test]
    fn counter_reset_zero_only() {
        let c = Counter::new("c", "h", HashMap::new()).unwrap();
        c.inc_by(42);
        assert_eq!(c.get(), 42);
        c.reset();
        assert_eq!(c.get(), 0);
    }

    /// 守门 #6: K-1 empty name 拒.
    #[test]
    fn k1_empty_name_rejected() {
        let r = Counter::new("", "h", HashMap::new());
        assert!(r.is_err());
    }

    /// 守门 #7: K-1 empty help 拒.
    #[test]
    fn k1_empty_help_rejected() {
        let r = Counter::new("c", "", HashMap::new());
        assert!(r.is_err());
    }

    /// 守门 #8: K-1 invalid label 拒.
    #[test]
    fn k1_invalid_label_rejected() {
        let mut l = HashMap::new();
        l.insert("123-bad".to_string(), "v".to_string());
        let r = Counter::new("c", "h", l);
        assert!(r.is_err());
    }

    /// 守门 #9: value() 返 Counter 变体.
    #[test]
    fn counter_value_variant() {
        let c = Counter::new("c", "h", HashMap::new()).unwrap();
        c.inc_by(7);
        let v = c.value();
        assert!(v.is_counter());
        assert_eq!(v, MetricValue::Counter(7));
    }

    /// 守门 #10: type_name = "counter".
    #[test]
    fn counter_type_name() {
        let c = Counter::new("c", "h", HashMap::new()).unwrap();
        assert_eq!(c.type_name(), "counter");
    }

    /// 守门 #11: 带 label 构造.
    #[test]
    fn counter_with_labels() {
        let c = Counter::new("requests_total", "Total", labels()).unwrap();
        assert_eq!(c.labels().get("method").unwrap(), "GET");
    }

    /// 守门 #12: 1000 并发 inc_by(1) → 1000 (atomic 守门).
    #[test]
    fn counter_concurrent_inc() {
        use std::sync::Arc;
        use std::thread;
        let c = Arc::new(Counter::new("c", "h", HashMap::new()).unwrap());
        let mut handles = vec![];
        for _ in 0..1000 {
            let cc = Arc::clone(&c);
            handles.push(thread::spawn(move || {
                cc.inc();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(c.get(), 1000);
    }

    /// 守门 #13: 1000 并发 inc_by(1) 跨 8 线程 → 1000.
    #[test]
    fn counter_concurrent_inc_by_8threads() {
        use std::sync::Arc;
        use std::thread;
        let c = Arc::new(Counter::new("c", "h", HashMap::new()).unwrap());
        let mut handles = vec![];
        for _ in 0..8 {
            let cc = Arc::clone(&c);
            handles.push(thread::spawn(move || {
                for _ in 0..125 {
                    cc.inc_by(1);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(c.get(), 1000);
    }
}
