//! R222 OTel-style metrics for supervisor (counter / gauge / histogram).
//!
//! **动机**: OTel (OpenTelemetry) 兼容 metrics 是 supervisor 可观测性核心.
//! 标准 OTel Metrics API 用 Counter / UpDownCounter (Gauge) / Histogram 三种 instrument.
//! 我们用 std 自实现 0 引 opentelemetry crate, 0 引 prometheus crate (避免 0 引外部 dep).
//!
//! **设计**:
//! - `Counter` — 单调递增, e.g. "supervisor_restart_count"
//! - `Gauge` — 上下波动, e.g. "active_children"
//! - `Histogram` — 分布观测, e.g. "tick_duration_ms"
//! - `MetricsRegistry` — 全局注册表, 跨模块共享
//! - `export_prometheus_text()` — 导出 Prometheus text format (OTel 默认格式之一)
//!
//! **0 触碰**: supervisor.rs / heartbeat.rs / pid_one.rs / strategy.rs 0 改.

#![allow(missing_docs)] // R222 additive
#![allow(clippy::all)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::Instant;

use std::sync::Mutex;

// ============================================================================
// Counter (单调递增 u64)
// ============================================================================

/// OTel-style monotonic counter.
pub struct Counter {
    name: String,
    value: AtomicU64,
    help: String,
}

impl Counter {
    pub fn new(name: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicU64::new(0),
            help: help.into(),
        }
    }
    pub fn inc(&self) { self.inc_by(1); }
    pub fn inc_by(&self, n: u64) { self.value.fetch_add(n, Ordering::Relaxed); }
    pub fn get(&self) -> u64 { self.value.load(Ordering::Relaxed) }
    pub fn name(&self) -> &str { &self.name }
    pub fn help(&self) -> &str { &self.help }
}

// ============================================================================
// Gauge (上下波动 i64)
// ============================================================================

/// OTel-style up/down counter (gauge).
pub struct Gauge {
    name: String,
    value: AtomicI64,
    help: String,
}

impl Gauge {
    pub fn new(name: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicI64::new(0),
            help: help.into(),
        }
    }
    pub fn set(&self, v: i64) { self.value.store(v, Ordering::Relaxed); }
    pub fn inc(&self) { self.value.fetch_add(1, Ordering::Relaxed); }
    pub fn dec(&self) { self.value.fetch_sub(1, Ordering::Relaxed); }
    pub fn get(&self) -> i64 { self.value.load(Ordering::Relaxed) }
    pub fn name(&self) -> &str { &self.name }
    pub fn help(&self) -> &str { &self.help }
}

// ============================================================================
// Histogram (分布观测)
// ============================================================================

/// OTel-style histogram (固定 bucket 计数).
pub struct Histogram {
    name: String,
    help: String,
    /// 累计观察数
    count: AtomicU64,
    /// 累计观察 sum
    sum: Mutex<f64>,
    /// 桶计数 (bucket upper bound → count)
    buckets: Mutex<Vec<(f64, u64)>>,
}

impl Histogram {
    /// 默认 11 桶 (ms): 0.1, 0.5, 1, 5, 10, 50, 100, 500, 1000, 5000, +Inf
    pub fn new_ms(name: impl Into<String>, help: impl Into<String>) -> Self {
        let buckets = vec![
            (0.1, 0u64), (0.5, 0), (1.0, 0), (5.0, 0), (10.0, 0),
            (50.0, 0), (100.0, 0), (500.0, 0), (1000.0, 0), (5000.0, 0),
        ];
        Self {
            name: name.into(),
            help: help.into(),
            count: AtomicU64::new(0),
            sum: Mutex::new(0.0),
            buckets: Mutex::new(buckets),
        }
    }

    /// 观察一个值 (ms).
    pub fn observe(&self, value_ms: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        *self.sum.lock().expect("poisoned") += value_ms;
        let mut bs = self.buckets.lock().expect("poisoned");
        for (upper, count) in bs.iter_mut() {
            if value_ms <= *upper {
                *count += 1;
            }
        }
    }

    /// 计时并自动 observe 耗时 (ms).
    pub fn time<F: FnOnce() -> R, R>(&self, f: F) -> R {
        let start = Instant::now();
        let r = f();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.observe(elapsed_ms);
        r
    }

    pub fn count(&self) -> u64 { self.count.load(Ordering::Relaxed) }
    pub fn sum(&self) -> f64 { *self.sum.lock().expect("poisoned") }
    pub fn mean(&self) -> f64 {
        let c = self.count();
        if c == 0 { 0.0 } else { self.sum() / c as f64 }
    }
    pub fn name(&self) -> &str { &self.name }
    pub fn help(&self) -> &str { &self.help }
}

// ============================================================================
// Registry
// ============================================================================

/// Metric 类型 enum (export 用).
pub enum MetricEntry {
    Counter(Arc<Counter>),
    Gauge(Arc<Gauge>),
    Histogram(Arc<Histogram>),
}

/// 全局 metrics registry.
#[derive(Default, Clone)]
pub struct MetricsRegistry {
    counters: Arc<Mutex<HashMap<String, Arc<Counter>>>>,
    gauges: Arc<Mutex<HashMap<String, Arc<Gauge>>>>,
    histograms: Arc<Mutex<HashMap<String, Arc<Histogram>>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register_counter(&self, c: Counter) -> Arc<Counter> {
        let arc = Arc::new(c);
        self.counters.lock().expect("poisoned").insert(arc.name().to_string(), arc.clone());
        arc
    }

    pub fn register_gauge(&self, g: Gauge) -> Arc<Gauge> {
        let arc = Arc::new(g);
        self.gauges.lock().expect("poisoned").insert(arc.name().to_string(), arc.clone());
        arc
    }

    pub fn register_histogram(&self, h: Histogram) -> Arc<Histogram> {
        let arc = Arc::new(h);
        self.histograms.lock().expect("poisoned").insert(arc.name().to_string(), arc.clone());
        arc
    }

    pub fn counter(&self, name: &str) -> Option<Arc<Counter>> {
        self.counters.lock().expect("poisoned").get(name).cloned()
    }

    pub fn gauge(&self, name: &str) -> Option<Arc<Gauge>> {
        self.gauges.lock().expect("poisoned").get(name).cloned()
    }

    pub fn histogram(&self, name: &str) -> Option<Arc<Histogram>> {
        self.histograms.lock().expect("poisoned").get(name).cloned()
    }

    /// 导出 Prometheus text format (OTel 兼容).
    pub fn export_prometheus_text(&self) -> String {
        let mut out = String::new();
        // Counters
        for c in self.counters.lock().expect("poisoned").values() {
            out.push_str(&format!("# HELP {} {}\n", c.name(), c.help()));
            out.push_str(&format!("# TYPE {} counter\n", c.name()));
            out.push_str(&format!("{} {}\n", c.name(), c.get()));
        }
        // Gauges
        for g in self.gauges.lock().expect("poisoned").values() {
            out.push_str(&format!("# HELP {} {}\n", g.name(), g.help()));
            out.push_str(&format!("# TYPE {} gauge\n", g.name()));
            out.push_str(&format!("{} {}\n", g.name(), g.get()));
        }
        // Histograms
        for h in self.histograms.lock().expect("poisoned").values() {
            out.push_str(&format!("# HELP {} {}\n", h.name(), h.help()));
            out.push_str(&format!("# TYPE {} histogram\n", h.name()));
            out.push_str(&format!("{}_count {}\n", h.name(), h.count()));
            out.push_str(&format!("{}_sum {}\n", h.name(), h.sum()));
            let bs = h.buckets.lock().expect("poisoned");
            for (upper, count) in bs.iter() {
                out.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", h.name(), upper, count));
            }
        }
        out
    }
}

// ============================================================================
// Supervisor 默认 metrics (3 个 counter + 2 个 gauge + 1 个 histogram)
// ============================================================================

/// Supervisor 默认注册 metrics.
pub fn supervisor_default_metrics() -> (MetricsRegistry, SupervisorMetrics) {
    let reg = MetricsRegistry::new();
    let restart_count = reg.register_counter(Counter::new(
        "supervisor_restart_count",
        "supervisor 触发的 child 重启次数",
    ));
    let heartbeat_count = reg.register_counter(Counter::new(
        "supervisor_heartbeat_count",
        "heartbeat 调度器 tick 总数",
    ));
    let panic_count = reg.register_counter(Counter::new(
        "supervisor_panic_count",
        "child panic 次数",
    ));
    let active_children = reg.register_gauge(Gauge::new(
        "supervisor_active_children",
        "当前 active child 数",
    ));
    let total_children = reg.register_gauge(Gauge::new(
        "supervisor_total_children",
        "注册的 child 总数 (含已退出)",
    ));
    let tick_duration = reg.register_histogram(Histogram::new_ms(
        "supervisor_tick_duration_ms",
        "heartbeat tick 耗时分布",
    ));
    let m = SupervisorMetrics {
        restart_count,
        heartbeat_count,
        panic_count,
        active_children,
        total_children,
        tick_duration,
    };
    (reg, m)
}

/// Supervisor 6 个核心 metric handles.
#[derive(Clone)]
pub struct SupervisorMetrics {
    pub restart_count: Arc<Counter>,
    pub heartbeat_count: Arc<Counter>,
    pub panic_count: Arc<Counter>,
    pub active_children: Arc<Gauge>,
    pub total_children: Arc<Gauge>,
    pub tick_duration: Arc<Histogram>,
}

// ============================================================================
// 测试 (10 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_counter_inc() {
        let c = Counter::new("test", "test help");
        c.inc();
        c.inc_by(5);
        assert_eq!(c.get(), 6);
    }

    #[test]
    fn t02_gauge_set_inc_dec() {
        let g = Gauge::new("g", "h");
        g.set(10);
        assert_eq!(g.get(), 10);
        g.inc();
        assert_eq!(g.get(), 11);
        g.dec();
        assert_eq!(g.get(), 10);
    }

    #[test]
    fn t03_histogram_observe() {
        let h = Histogram::new_ms("h", "h");
        h.observe(1.0);
        h.observe(10.0);
        h.observe(100.0);
        assert_eq!(h.count(), 3);
        assert!((h.sum() - 111.0).abs() < 1e-9);
        assert!((h.mean() - 37.0).abs() < 1e-9);
    }

    #[test]
    fn t04_histogram_time() {
        let h = Histogram::new_ms("h", "h");
        h.time(|| {
            std::thread::sleep(std::time::Duration::from_millis(2));
        });
        assert_eq!(h.count(), 1);
        assert!(h.sum() > 1.0);  // 至少 1ms
    }

    #[test]
    fn t05_registry_register_get() {
        let reg = MetricsRegistry::new();
        let c = reg.register_counter(Counter::new("c1", "h"));
        c.inc();
        let got = reg.counter("c1").unwrap();
        assert_eq!(got.get(), 1);
        assert!(reg.counter("c2").is_none());
    }

    #[test]
    fn t06_prometheus_export_basic() {
        let reg = MetricsRegistry::new();
        let c = reg.register_counter(Counter::new("test_counter", "test help"));
        c.inc_by(3);
        let text = reg.export_prometheus_text();
        assert!(text.contains("# HELP test_counter"));
        assert!(text.contains("# TYPE test_counter counter"));
        assert!(text.contains("test_counter 3"));
    }

    #[test]
    fn t07_supervisor_default_metrics() {
        let (reg, m) = supervisor_default_metrics();
        m.restart_count.inc();
        m.active_children.set(5);
        m.tick_duration.observe(2.5);
        let text = reg.export_prometheus_text();
        assert!(text.contains("supervisor_restart_count"));
        assert!(text.contains("supervisor_active_children"));
        assert!(text.contains("supervisor_tick_duration_ms"));
    }

    #[test]
    fn t08_histogram_buckets() {
        let h = Histogram::new_ms("h", "h");
        h.observe(0.05);  // <= 0.1
        h.observe(2.0);   // <= 5
        h.observe(200.0); // <= 500
        // 全部 3 观察应累加到 buckets
        let bs = h.buckets.lock().expect("poisoned");
        // 第一个桶 <= 0.1: 1 个
        assert_eq!(bs[0].1, 1);
        // 第三个桶 <= 1: 1 个 (0.05)
        assert_eq!(bs[2].1, 1);
        // 第四个桶 <= 5: 2 个 (0.05, 2.0)
        assert_eq!(bs[3].1, 2);
        // 第七个桶 <= 100: 2 个
        assert_eq!(bs[6].1, 2);
        // 第八个桶 <= 500: 3 个
        assert_eq!(bs[7].1, 3);
    }

    #[test]
    fn t09_multiple_counters_export() {
        let reg = MetricsRegistry::new();
        reg.register_counter(Counter::new("a", "h"));
        reg.register_counter(Counter::new("b", "h"));
        reg.register_gauge(Gauge::new("c", "h"));
        let text = reg.export_prometheus_text();
        assert!(text.contains("a "));
        assert!(text.contains("b "));
        assert!(text.contains("c "));
    }

    #[test]
    fn t10_supervisor_metrics_clone_shares() {
        let (_reg, m) = supervisor_default_metrics();
        let m2 = m.clone();
        m.restart_count.inc();
        // m2 共享同一 Arc, 计数应同步
        assert_eq!(m2.restart_count.get(), 1);
    }
}
