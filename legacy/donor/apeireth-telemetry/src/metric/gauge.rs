//! # Gauge — 任意增减 metrics
//!
//! Gauge 是可增可减的瞬时值 (e.g. `memory_bytes`, `in_flight_requests`).
//!
//! ## 1:1 翻译 v0.9.21 @anthropic-ai/metrics
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版   | 1:1 |
//! |------------------|--------------------------------|-----|
//! | `Gauge`          | `class Gauge`                  | ✅  |
//! | `inc()`          | `inc()`                        | ✅  |
//! | `dec()`          | `dec()`                        | ✅  |
//! | `add(n)`         | `add(n)`                       | ✅  |
//! | `sub(n)`         | `sub(n)`                       | ✅  |
//! | `set(v)`         | `set(v)`                       | ✅  |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use super::error::MetricsResult;
use super::label::{validate_labels, Label};
use super::{Metric, MetricValue};

// ============================================================================
// §1 Gauge 结构
// ============================================================================

/// Gauge — 任意增减 metric.
///
/// 内部用 AtomicI64 存储 (把 f64 bits 转 i64, NaN 用哨兵).
#[derive(Debug)]
pub struct Gauge {
    /// metric 名.
    name: String,
    /// Help 文本 (K-1 强校验: 必填).
    help: String,
    /// label 集合 (K-1 强校验: ≤ 10).
    labels: HashMap<String, String>,
    /// 当前值 (atomic, bits: f64::to_bits).
    bits: AtomicI64,
}

impl Gauge {
    /// 构造 Gauge.
    pub fn new(
        name: impl Into<String>,
        help: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> MetricsResult<Self> {
        let name = name.into();
        let help = help.into();
        if name.is_empty() {
            return Err(super::error::MetricsError::MetricNameEmpty);
        }
        if help.is_empty() {
            return Err(super::error::MetricsError::HelpRequired(name));
        }
        validate_labels(&labels)?;
        Ok(Self {
            name,
            help,
            labels,
            bits: AtomicI64::new(0.0_f64.to_bits() as i64),
        })
    }

    /// +1.0.
    pub fn inc(&self) {
        self.add(1.0);
    }

    /// -1.0.
    pub fn dec(&self) {
        self.sub(1.0);
    }

    /// +n.
    pub fn add(&self, n: f64) {
        if n == 0.0 {
            return;
        }
        loop {
            let current_bits = self.bits.load(Ordering::Relaxed);
            let current = f64::from_bits(current_bits as u64);
            let next = current + n;
            let next_bits = next.to_bits() as i64;
            if self
                .bits
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

    /// -n.
    pub fn sub(&self, n: f64) {
        if n == 0.0 {
            return;
        }
        self.add(-n);
    }

    /// 直接设置值.
    pub fn set(&self, v: f64) {
        self.bits.store(v.to_bits() as i64, Ordering::Relaxed);
    }

    /// 当前值.
    pub fn get(&self) -> f64 {
        f64::from_bits(self.bits.load(Ordering::Relaxed) as u64)
    }

    /// label 集合引用 (read-only).
    pub fn label_pairs(&self) -> Vec<Label> {
        self.labels
            .iter()
            .map(|(k, v)| Label::new_unchecked(k, v))
            .collect()
    }
}

impl Metric for Gauge {
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
        MetricValue::Gauge(self.get())
    }

    fn type_name(&self) -> &'static str {
        "gauge"
    }
}

// ============================================================================
// §2 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 构造 + 初始值 0.
    #[test]
    fn gauge_new_initial_zero() {
        let g = Gauge::new("memory_bytes", "Memory used", HashMap::new()).unwrap();
        assert_eq!(g.get(), 0.0);
    }

    /// 守门 #2: inc / dec 任意增减.
    #[test]
    fn gauge_inc_dec() {
        let g = Gauge::new("g", "h", HashMap::new()).unwrap();
        g.inc();
        g.inc();
        g.inc();
        assert_eq!(g.get(), 3.0);
        g.dec();
        assert_eq!(g.get(), 2.0);
    }

    /// 守门 #3: add / sub 任意值.
    #[test]
    fn gauge_add_sub() {
        let g = Gauge::new("g", "h", HashMap::new()).unwrap();
        g.add(1.5);
        g.add(2.5);
        assert_eq!(g.get(), 4.0);
        g.sub(1.0);
        assert_eq!(g.get(), 3.0);
    }

    /// 守门 #4: set 直接赋值.
    #[test]
    fn gauge_set() {
        let g = Gauge::new("g", "h", HashMap::new()).unwrap();
        g.set(42.5);
        assert_eq!(g.get(), 42.5);
        g.set(-100.0);
        assert_eq!(g.get(), -100.0);
    }

    /// 守门 #5: set NaN / Inf.
    #[test]
    fn gauge_set_nan_inf() {
        let g = Gauge::new("g", "h", HashMap::new()).unwrap();
        g.set(f64::NAN);
        assert!(g.get().is_nan());
        g.set(f64::INFINITY);
        assert!(g.get().is_infinite());
    }

    /// 守门 #6: K-1 empty name 拒.
    #[test]
    fn k1_empty_name_rejected() {
        let r = Gauge::new("", "h", HashMap::new());
        assert!(r.is_err());
    }

    /// 守门 #7: K-1 empty help 拒.
    #[test]
    fn k1_empty_help_rejected() {
        let r = Gauge::new("g", "", HashMap::new());
        assert!(r.is_err());
    }

    /// 守门 #8: K-1 invalid label 拒.
    #[test]
    fn k1_invalid_label_rejected() {
        let mut l = HashMap::new();
        l.insert("foo bar".to_string(), "v".to_string());
        let r = Gauge::new("g", "h", l);
        assert!(r.is_err());
    }

    /// 守门 #9: value() 返 Gauge 变体.
    #[test]
    fn gauge_value_variant() {
        let g = Gauge::new("g", "h", HashMap::new()).unwrap();
        g.set(7.5);
        let v = g.value();
        assert!(v.is_gauge());
        assert_eq!(v, MetricValue::Gauge(7.5));
    }

    /// 守门 #10: type_name = "gauge".
    #[test]
    fn gauge_type_name() {
        let g = Gauge::new("g", "h", HashMap::new()).unwrap();
        assert_eq!(g.type_name(), "gauge");
    }

    /// 守门 #11: 1000 并发 add(1.0) → 1000.0.
    #[test]
    fn gauge_concurrent_add() {
        use std::sync::Arc;
        use std::thread;
        let g = Arc::new(Gauge::new("g", "h", HashMap::new()).unwrap());
        let mut handles = vec![];
        for _ in 0..1000 {
            let gg = Arc::clone(&g);
            handles.push(thread::spawn(move || {
                gg.add(1.0);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(g.get(), 1000.0);
    }

    /// 守门 #12: 1000 并发 inc / dec 混合 → 0.
    #[test]
    fn gauge_concurrent_inc_dec() {
        use std::sync::Arc;
        use std::thread;
        let g = Arc::new(Gauge::new("g", "h", HashMap::new()).unwrap());
        let mut handles = vec![];
        for i in 0..1000 {
            let gg = Arc::clone(&g);
            handles.push(thread::spawn(move || {
                if i % 2 == 0 {
                    gg.inc();
                } else {
                    gg.dec();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // 1000 步, 一半 inc 一半 dec = 0
        assert_eq!(g.get(), 0.0);
    }
}
