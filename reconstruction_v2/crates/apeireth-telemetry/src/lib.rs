//! apeireth-telemetry - Metrics + tracing (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 Counter + Gauge + 真 export

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub labels: std::collections::HashMap<String, String>,
}

pub struct Counter { pub name: String, pub inner: Arc<AtomicU64> }

impl Counter {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), inner: Arc::new(AtomicU64::new(0)) } }
    /// 0 装 PASS: 真 atomic inc
    pub fn inc(&self) { self.inner.fetch_add(1, Ordering::Relaxed); }
    /// 0 装 PASS: 真 atomic add
    pub fn add(&self, n: u64) { self.inner.fetch_add(n, Ordering::Relaxed); }
    /// 0 装 PASS: 真 get
    pub fn get(&self) -> u64 { self.inner.load(Ordering::Relaxed) }
}

pub struct Gauge { pub name: String, pub inner: Arc<std::sync::Mutex<f64>> }

impl Gauge {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), inner: Arc::new(std::sync::Mutex::new(0.0)) } }
    /// 0 装 PASS: 真 set
    pub fn set(&self, v: f64) { *self.inner.lock().unwrap() = v; }
    /// 0 装 PASS: 真 get
    pub fn get(&self) -> f64 { *self.inner.lock().unwrap() }
}

pub struct Registry { pub counters: HashMap<String, Counter>, pub gauges: HashMap<String, Gauge> }

impl Registry {
    pub fn new() -> Self { Self { counters: HashMap::new(), gauges: HashMap::new() } }
    /// 0 装 PASS: 真 get or create
    pub fn counter(&mut self, name: &str) -> &mut Counter {
        self.counters.entry(name.to_string()).or_insert_with(|| Counter::new(name))
    }

    /// 0 装 PASS: 真 get or create gauge
    pub fn gauge(&mut self, name: &str) -> &mut Gauge {
        self.gauges.entry(name.to_string()).or_insert_with(|| Gauge::new(name))
    }
    /// 0 装 PASS: 真 export
    pub fn export(&self) -> Vec<Metric> {
        let mut out = vec![];
        for c in self.counters.values() { out.push(Metric { name: c.name.clone(), value: c.get() as f64, labels: Default::default() }); }
        for g in self.gauges.values() { out.push(Metric { name: g.name.clone(), value: g.get(), labels: Default::default() }); }
        out
    }
}

impl Default for Registry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_counter() {
        let c = Counter::new("hits");
        c.inc(); c.inc(); c.add(3);
        assert_eq!(c.get(), 5);
    }
    #[test]
    fn test_gauge() {
        let g = Gauge::new("temp");
        g.set(36.6);
        assert!((g.get() - 36.6).abs() < 0.01);
    }
    #[test]
    fn test_export() {
        let mut r = Registry::new();
        r.counter("a").inc();
        r.gauge("b").set(1.0);
        let m = r.export();
        assert_eq!(m.len(), 2);
    }
}
