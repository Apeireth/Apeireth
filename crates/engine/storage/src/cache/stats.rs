//! Lock-free cache counters.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Atomic cache metrics.
#[derive(Debug)]
pub struct CacheStats {
    hit: AtomicU64,
    miss: AtomicU64,
    put_count: AtomicU64,
    remove_count: AtomicU64,
    size: AtomicUsize,
    max_size: AtomicUsize,
    evictions: AtomicU64,
    get_latency_us: AtomicU64,
    put_latency_us: AtomicU64,
    remove_latency_us: AtomicU64,
}

impl CacheStats {
    /// Empty stats with a fixed max_size.
    pub fn new(max_size: usize) -> Self {
        Self {
            hit: AtomicU64::new(0),
            miss: AtomicU64::new(0),
            put_count: AtomicU64::new(0),
            remove_count: AtomicU64::new(0),
            size: AtomicUsize::new(0),
            max_size: AtomicUsize::new(max_size),
            evictions: AtomicU64::new(0),
            get_latency_us: AtomicU64::new(0),
            put_latency_us: AtomicU64::new(0),
            remove_latency_us: AtomicU64::new(0),
        }
    }

    /// Record a hit.
    pub fn record_hit(&self, latency_us: u64) {
        self.hit.fetch_add(1, Ordering::Relaxed);
        self.get_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a miss.
    pub fn record_miss(&self, latency_us: u64) {
        self.miss.fetch_add(1, Ordering::Relaxed);
        self.get_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a put.
    pub fn record_put(&self, latency_us: u64) {
        self.put_count.fetch_add(1, Ordering::Relaxed);
        self.put_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a remove.
    pub fn record_remove(&self, latency_us: u64) {
        self.remove_count.fetch_add(1, Ordering::Relaxed);
        self.remove_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record an eviction.
    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Store live size.
    pub fn set_size(&self, size: usize) {
        self.size.store(size, Ordering::Relaxed);
    }

    /// Live size.
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Max size.
    pub fn max_size(&self) -> usize {
        self.max_size.load(Ordering::Relaxed)
    }

    /// Hit rate in 0.0..=1.0. Unused caches report 0.0, never NaN.
    pub fn hit_rate(&self) -> f64 {
        let h = self.hit.load(Ordering::Relaxed) as f64;
        let m = self.miss.load(Ordering::Relaxed) as f64;
        let total = h + m;
        if total == 0.0 {
            0.0
        } else {
            h / total
        }
    }

    /// Hits.
    pub fn hit_count(&self) -> u64 {
        self.hit.load(Ordering::Relaxed)
    }

    /// Misses.
    pub fn miss_count(&self) -> u64 {
        self.miss.load(Ordering::Relaxed)
    }

    /// Puts.
    pub fn put_total(&self) -> u64 {
        self.put_count.load(Ordering::Relaxed)
    }

    /// Removes.
    pub fn remove_total(&self) -> u64 {
        self.remove_count.load(Ordering::Relaxed)
    }

    /// Evictions.
    pub fn eviction_count(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Average get latency in microseconds.
    pub fn avg_get_latency_us(&self) -> f64 {
        let total_us = self.get_latency_us.load(Ordering::Relaxed) as f64;
        let calls = (self.hit_count() + self.miss_count()) as f64;
        if calls == 0.0 {
            0.0
        } else {
            total_us / calls
        }
    }

    /// Average put latency in microseconds.
    pub fn avg_put_latency_us(&self) -> f64 {
        let total_us = self.put_latency_us.load(Ordering::Relaxed) as f64;
        let calls = self.put_total() as f64;
        if calls == 0.0 {
            0.0
        } else {
            total_us / calls
        }
    }

    /// Average remove latency in microseconds.
    pub fn avg_remove_latency_us(&self) -> f64 {
        let total_us = self.remove_latency_us.load(Ordering::Relaxed) as f64;
        let calls = self.remove_total() as f64;
        if calls == 0.0 {
            0.0
        } else {
            total_us / calls
        }
    }

    /// Zero counters. `max_size` is kept.
    pub fn reset(&self) {
        self.hit.store(0, Ordering::Relaxed);
        self.miss.store(0, Ordering::Relaxed);
        self.put_count.store(0, Ordering::Relaxed);
        self.remove_count.store(0, Ordering::Relaxed);
        self.size.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.get_latency_us.store(0, Ordering::Relaxed);
        self.put_latency_us.store(0, Ordering::Relaxed);
        self.remove_latency_us.store(0, Ordering::Relaxed);
    }

    /// Immutable snapshot.
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hit: self.hit_count(),
            miss: self.miss_count(),
            put_count: self.put_total(),
            remove_count: self.remove_total(),
            size: self.size(),
            max_size: self.max_size(),
            evictions: self.eviction_count(),
            hit_rate: self.hit_rate(),
            avg_get_latency_us: self.avg_get_latency_us(),
            avg_put_latency_us: self.avg_put_latency_us(),
            avg_remove_latency_us: self.avg_remove_latency_us(),
        }
    }
}

/// Point-in-time stats.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheStatsSnapshot {
    /// Hits.
    pub hit: u64,
    /// Misses.
    pub miss: u64,
    /// Puts.
    pub put_count: u64,
    /// Removes.
    pub remove_count: u64,
    /// Live size.
    pub size: usize,
    /// Max size.
    pub max_size: usize,
    /// Evictions.
    pub evictions: u64,
    /// Hit rate.
    pub hit_rate: f64,
    /// Average get latency (µs).
    pub avg_get_latency_us: f64,
    /// Average put latency (µs).
    pub avg_put_latency_us: f64,
    /// Average remove latency (µs).
    pub avg_remove_latency_us: f64,
}

impl CacheStatsSnapshot {
    /// Empty snapshot.
    pub fn empty(max_size: usize) -> Self {
        Self {
            hit: 0,
            miss: 0,
            put_count: 0,
            remove_count: 0,
            size: 0,
            max_size,
            evictions: 0,
            hit_rate: 0.0,
            avg_get_latency_us: 0.0,
            avg_put_latency_us: 0.0,
            avg_remove_latency_us: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_rate_zero_when_unused() {
        let s = CacheStats::new(10);
        assert_eq!(s.hit_rate(), 0.0);
        assert_eq!(s.avg_get_latency_us(), 0.0);
    }

    #[test]
    fn hit_rate_is_hits_over_total() {
        let s = CacheStats::new(10);
        s.record_hit(10);
        s.record_hit(10);
        s.record_miss(10);
        assert!((s.hit_rate() - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn reset_keeps_max_size() {
        let s = CacheStats::new(42);
        s.record_hit(1);
        s.set_size(3);
        s.reset();
        assert_eq!(s.hit_count(), 0);
        assert_eq!(s.size(), 0);
        assert_eq!(s.max_size(), 42);
    }
}
