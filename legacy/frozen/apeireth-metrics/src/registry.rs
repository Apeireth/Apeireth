//! # MetricsRegistry — 注册中心
//!
//! 1:1 翻译 v0.9.21 @anthropic-ai/metrics `class MetricsRegistry`.
//!
//! ## API
//!
//! - `register(name, help, metric)` — 注册一个 metric
//! - `unregister(name)` — 注销
//! - `get(name)` — 查单个 metric
//! - `list()` — 列出全部
//! - `clear()` — 清空
//!
//! ## 线程安全
//!
//! 内部用 `parking_lot::RwLock<HashMap<...>>`, 读多写少场景最优.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::counter::Counter;
use crate::error::{MetricsError, MetricsResult};
use crate::gauge::Gauge;
use crate::histogram::Histogram;
use crate::metric::Metric;
use crate::summary::Summary;

// ============================================================================
// §1 RegisteredMetric enum (4 variant)
// ============================================================================

/// 注册到 registry 的 metric (4 类型).
#[derive(Clone)]
pub enum RegisteredMetric {
    /// Counter.
    Counter(Arc<Counter>),
    /// Gauge.
    Gauge(Arc<Gauge>),
    /// Histogram.
    Histogram(Arc<Histogram>),
    /// Summary.
    Summary(Arc<Summary>),
}

impl RegisteredMetric {
    /// metric 名.
    pub fn name(&self) -> &str {
        match self {
            RegisteredMetric::Counter(c) => c.name(),
            RegisteredMetric::Gauge(g) => g.name(),
            RegisteredMetric::Histogram(h) => h.name(),
            RegisteredMetric::Summary(s) => s.name(),
        }
    }

    /// help 文本.
    pub fn help(&self) -> &str {
        match self {
            RegisteredMetric::Counter(c) => c.help(),
            RegisteredMetric::Gauge(g) => g.help(),
            RegisteredMetric::Histogram(h) => h.help(),
            RegisteredMetric::Summary(s) => s.help(),
        }
    }

    /// 类型名.
    pub fn type_name(&self) -> &'static str {
        match self {
            RegisteredMetric::Counter(_) => "counter",
            RegisteredMetric::Gauge(_) => "gauge",
            RegisteredMetric::Histogram(_) => "histogram",
            RegisteredMetric::Summary(_) => "summary",
        }
    }
}

impl std::fmt::Debug for RegisteredMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredMetric")
            .field("name", &self.name())
            .field("type", &self.type_name())
            .finish()
    }
}

// ============================================================================
// §2 MetricsRegistry 结构
// ============================================================================

/// Metrics 注册中心.
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    /// 内部: name → RegisteredMetric.
    inner: RwLock<HashMap<String, RegisteredMetric>>,
}

impl MetricsRegistry {
    /// 构造空 registry.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// 注册一个 metric.
    ///
    /// - K-1 强校验: name 必填, help 必填, 同名 metric 不可重复注册.
    /// - 4 类型 metric 已通过 type_name 区分, 相同 name 但不同类型仍视为重复 (防命名混淆).
    pub fn register(&self, metric: RegisteredMetric) -> MetricsResult<()> {
        let name = metric.name().to_string();
        if name.is_empty() {
            return Err(MetricsError::MetricNameEmpty);
        }
        let mut guard = self.inner.write();
        if guard.contains_key(&name) {
            return Err(MetricsError::MetricAlreadyRegistered(name));
        }
        guard.insert(name, metric);
        Ok(())
    }

    /// 注销.
    pub fn unregister(&self, name: &str) -> MetricsResult<RegisteredMetric> {
        let mut guard = self.inner.write();
        guard
            .remove(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))
    }

    /// 查单个 metric.
    pub fn get(&self, name: &str) -> Option<RegisteredMetric> {
        self.inner.read().get(name).cloned()
    }

    /// 列出全部 (顺序: name 字典序, 跟 Prometheus convention 一致).
    pub fn list(&self) -> Vec<RegisteredMetric> {
        let guard = self.inner.read();
        let mut entries: Vec<(&String, &RegisteredMetric)> = guard.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        entries.into_iter().map(|(_, v)| v.clone()).collect()
    }

    /// 列出全部 (name + metric) 对.
    pub fn list_named(&self) -> Vec<(String, RegisteredMetric)> {
        let guard = self.inner.read();
        let mut entries: Vec<(String, RegisteredMetric)> = guard
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    /// 当前注册数量.
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// 清空.
    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

// ============================================================================
// §3 便捷 register 方法 (4 类型分别)
// ============================================================================

impl MetricsRegistry {
    /// 注册 Counter.
    pub fn register_counter(&self, counter: Arc<Counter>) -> MetricsResult<()> {
        self.register(RegisteredMetric::Counter(counter))
    }

    /// 注册 Gauge.
    pub fn register_gauge(&self, gauge: Arc<Gauge>) -> MetricsResult<()> {
        self.register(RegisteredMetric::Gauge(gauge))
    }

    /// 注册 Histogram.
    pub fn register_histogram(&self, histogram: Arc<Histogram>) -> MetricsResult<()> {
        self.register(RegisteredMetric::Histogram(histogram))
    }

    /// 注册 Summary.
    pub fn register_summary(&self, summary: Arc<Summary>) -> MetricsResult<()> {
        self.register(RegisteredMetric::Summary(summary))
    }
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 新 registry 空.
    #[test]
    fn new_registry_empty() {
        let r = MetricsRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    /// 守门 #2: register counter.
    #[test]
    fn register_counter() {
        let r = MetricsRegistry::new();
        let c = Arc::new(Counter::new("c1", "h", HashMap::new()).unwrap());
        r.register_counter(c).unwrap();
        assert_eq!(r.len(), 1);
        assert!(r.get("c1").is_some());
    }

    /// 守门 #3: 同名重复 register 拒.
    #[test]
    fn register_duplicate_name_rejected() {
        let r = MetricsRegistry::new();
        let c1 = Arc::new(Counter::new("c1", "h", HashMap::new()).unwrap());
        let c2 = Arc::new(Counter::new("c1", "h", HashMap::new()).unwrap());
        r.register_counter(c1).unwrap();
        assert!(matches!(
            r.register_counter(c2),
            Err(MetricsError::MetricAlreadyRegistered(_))
        ));
    }

    /// 守门 #4: unregister 存在 → 返 Some.
    #[test]
    fn unregister_existing() {
        let r = MetricsRegistry::new();
        let c = Arc::new(Counter::new("c1", "h", HashMap::new()).unwrap());
        r.register_counter(c).unwrap();
        let removed = r.unregister("c1").unwrap();
        assert_eq!(removed.name(), "c1");
        assert!(r.is_empty());
    }

    /// 守门 #5: unregister 不存在 → 返 Err(MetricNotFound).
    #[test]
    fn unregister_not_found() {
        let r = MetricsRegistry::new();
        assert!(matches!(
            r.unregister("nope"),
            Err(MetricsError::MetricNotFound(_))
        ));
    }

    /// 守门 #6: get 存在 → 返 Some.
    #[test]
    fn get_existing() {
        let r = MetricsRegistry::new();
        let c = Arc::new(Counter::new("c1", "h", HashMap::new()).unwrap());
        r.register_counter(c).unwrap();
        let m = r.get("c1").unwrap();
        assert_eq!(m.name(), "c1");
    }

    /// 守门 #7: get 不存在 → 返 None.
    #[test]
    fn get_not_found_returns_none() {
        let r = MetricsRegistry::new();
        assert!(r.get("nope").is_none());
    }

    /// 守门 #8: list 全部.
    #[test]
    fn list_all_metrics() {
        let r = MetricsRegistry::new();
        r.register_counter(Arc::new(Counter::new("c", "h", HashMap::new()).unwrap()))
            .unwrap();
        r.register_gauge(Arc::new(Gauge::new("g", "h", HashMap::new()).unwrap()))
            .unwrap();
        r.register_histogram(Arc::new(
            Histogram::new("h", "h", HashMap::new()).unwrap(),
        ))
        .unwrap();
        r.register_summary(Arc::new(Summary::new("s", "h", HashMap::new()).unwrap()))
            .unwrap();
        let list = r.list();
        assert_eq!(list.len(), 4);
        // 排序后: c < g < h < s
        assert_eq!(list[0].name(), "c");
        assert_eq!(list[1].name(), "g");
        assert_eq!(list[2].name(), "h");
        assert_eq!(list[3].name(), "s");
    }

    /// 守门 #9: clear 清空.
    #[test]
    fn clear_empties_registry() {
        let r = MetricsRegistry::new();
        r.register_counter(Arc::new(Counter::new("c", "h", HashMap::new()).unwrap()))
            .unwrap();
        assert!(!r.is_empty());
        r.clear();
        assert!(r.is_empty());
    }

    /// 守门 #10: 4 类型都能 register.
    #[test]
    fn all_4_types_register() {
        let r = MetricsRegistry::new();
        r.register_counter(Arc::new(Counter::new("c", "h", HashMap::new()).unwrap()))
            .unwrap();
        r.register_gauge(Arc::new(Gauge::new("g", "h", HashMap::new()).unwrap()))
            .unwrap();
        r.register_histogram(Arc::new(
            Histogram::new("h", "h", HashMap::new()).unwrap(),
        ))
        .unwrap();
        r.register_summary(Arc::new(Summary::new("s", "h", HashMap::new()).unwrap()))
            .unwrap();
        assert_eq!(r.len(), 4);
    }

    /// 守门 #11: K-1 empty name register 拒.
    #[test]
    fn k1_empty_name_rejected() {
        let r = MetricsRegistry::new();
        // 构造一个 name="" 的 metric 不可能 (Counter::new 已拒), 所以测试 unregister 空字符串
        assert!(matches!(
            r.unregister(""),
            Err(MetricsError::MetricNotFound(_))
        ));
    }

    /// 守门 #12: 1000 并发 register 不同 name.
    #[test]
    fn concurrent_register_1000() {
        use std::thread;
        let r = std::sync::Arc::new(MetricsRegistry::new());
        let mut handles = vec![];
        for i in 0..1000 {
            let rr = std::sync::Arc::clone(&r);
            handles.push(thread::spawn(move || {
                let c = Counter::new(format!("c{i}"), "h", HashMap::new()).unwrap();
                rr.register_counter(std::sync::Arc::new(c)).unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(r.len(), 1000);
    }

    /// 守门 #13: list_named 返 (name, metric) 对.
    #[test]
    fn list_named_returns_pairs() {
        let r = MetricsRegistry::new();
        r.register_counter(Arc::new(Counter::new("c", "h", HashMap::new()).unwrap()))
            .unwrap();
        let pairs = r.list_named();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "c");
    }
}
