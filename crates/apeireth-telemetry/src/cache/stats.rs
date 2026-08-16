//! # CacheStats — 命中率 / 大小 / 延迟 metric
//!
//! 3 类核心 metric:
//! 1. **命中率**: hit / miss / hit_rate (f64)
//! 2. **大小**: size / max_size / evictions
//! 3. **延迟**: get_latency_us / put_latency_us / remove_latency_us (microsecond)
//!
//! ## 设计原则
//!
//! - 线程安全: 用 AtomicU64 / AtomicUsize 计数, 无锁
//! - 不假装: 命中率 = hit / (hit + miss), 不计算 "理论命中率"
//! - K-1 强校验: hit + miss = 0 时 hit_rate = 0.0 (不返 NaN / Inf)
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

// ============================================================================
// §1 CacheStats 结构 (命中率 / 大小 / 延迟 原子计数)
// ============================================================================

/// Cache metric (3 类: 命中率 / 大小 / 延迟).
///
/// 用 atomic 计数, 多线程 get / put / remove 共享同一份 stats snapshot.
#[derive(Debug)]
pub struct CacheStats {
    /// 命中次数.
    pub hit: AtomicU64,
    /// miss 次数.
    pub miss: AtomicU64,
    /// put 次数.
    pub put_count: AtomicU64,
    /// remove 次数.
    pub remove_count: AtomicU64,
    /// 当前 item 数.
    pub size: AtomicUsize,
    /// 最大容量.
    pub max_size: AtomicUsize,
    /// 淘汰次数.
    pub evictions: AtomicU64,
    /// get 累计延迟.
    pub get_latency_us: AtomicU64,
    /// put 累计延迟.
    pub put_latency_us: AtomicU64,
    /// remove 累计延迟.
    pub remove_latency_us: AtomicU64,
}

impl CacheStats {
    /// 构造空 stats (max_size 给定, 不变).
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

    /// 记录 1 次 hit (含 latency).
    #[inline]
    pub fn record_hit(&self, latency_us: u64) {
        self.hit.fetch_add(1, Ordering::Relaxed);
        self.get_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// 记录 1 次 miss (含 latency).
    #[inline]
    pub fn record_miss(&self, latency_us: u64) {
        self.miss.fetch_add(1, Ordering::Relaxed);
        self.get_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// 记录 1 次 put (含 latency).
    #[inline]
    pub fn record_put(&self, latency_us: u64) {
        self.put_count.fetch_add(1, Ordering::Relaxed);
        self.put_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// 记录 1 次 remove (含 latency).
    #[inline]
    pub fn record_remove(&self, latency_us: u64) {
        self.remove_count.fetch_add(1, Ordering::Relaxed);
        self.remove_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    /// 记录 1 次 eviction.
    #[inline]
    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// 更新当前 size.
    #[inline]
    pub fn set_size(&self, size: usize) {
        self.size.store(size, Ordering::Relaxed);
    }

    /// 当前 size (原子读).
    #[inline]
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// max_size (原子读).
    #[inline]
    pub fn max_size(&self) -> usize {
        self.max_size.load(Ordering::Relaxed)
    }

    /// 命中率 (0.0..=1.0). 0 hit + miss 时返 0.0 (K-1 强校验, 不返 NaN).
    #[inline]
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

    /// hit 总数.
    #[inline]
    pub fn hit_count(&self) -> u64 {
        self.hit.load(Ordering::Relaxed)
    }

    /// miss 总数.
    #[inline]
    pub fn miss_count(&self) -> u64 {
        self.miss.load(Ordering::Relaxed)
    }

    /// put 总数.
    #[inline]
    pub fn put_total(&self) -> u64 {
        self.put_count.load(Ordering::Relaxed)
    }

    /// remove 总数.
    #[inline]
    pub fn remove_total(&self) -> u64 {
        self.remove_count.load(Ordering::Relaxed)
    }

    /// eviction 总数.
    #[inline]
    pub fn eviction_count(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// 平均 get latency (microsecond). 0 call 时返 0.0.
    #[inline]
    pub fn avg_get_latency_us(&self) -> f64 {
        let total_us = self.get_latency_us.load(Ordering::Relaxed) as f64;
        let calls = (self.hit_count() + self.miss_count()) as f64;
        if calls == 0.0 {
            0.0
        } else {
            total_us / calls
        }
    }

    /// 平均 put latency (microsecond). 0 call 时返 0.0.
    #[inline]
    pub fn avg_put_latency_us(&self) -> f64 {
        let total_us = self.put_latency_us.load(Ordering::Relaxed) as f64;
        let calls = self.put_total() as f64;
        if calls == 0.0 {
            0.0
        } else {
            total_us / calls
        }
    }

    /// 平均 remove latency (microsecond). 0 call 时返 0.0.
    #[inline]
    pub fn avg_remove_latency_us(&self) -> f64 {
        let total_us = self.remove_latency_us.load(Ordering::Relaxed) as f64;
        let calls = self.remove_total() as f64;
        if calls == 0.0 {
            0.0
        } else {
            total_us / calls
        }
    }

    /// 重置全部计数 (clear 时调用). max_size 不被 reset.
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

    /// 序列化成 CacheStatsSnapshot (普通结构, 不可变).
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

// ============================================================================
// §2 CacheStatsSnapshot 不可变快照 (返回给调用方)
// ============================================================================

/// CacheStats 不可变快照.
///
/// 调用方拿到的 stats 是一份 point-in-time 快照, 不随 cache 操作变化.
/// Serialize/Deserialize 用于跨进程传递 (e.g. admin endpoint 暴露).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CacheStatsSnapshot {
    /// 命中次数.
    pub hit: u64,
    /// miss 次数.
    pub miss: u64,
    /// put 次数.
    pub put_count: u64,
    /// remove 次数.
    pub remove_count: u64,
    /// 当前 item 数.
    pub size: usize,
    /// 最大容量.
    pub max_size: usize,
    /// 淘汰次数.
    pub evictions: u64,
    /// 命中率 (0.0..=1.0).
    pub hit_rate: f64,
    /// 平均 get latency (microsecond).
    pub avg_get_latency_us: f64,
    /// 平均 put latency (microsecond).
    pub avg_put_latency_us: f64,
    /// 平均 remove latency (microsecond).
    pub avg_remove_latency_us: f64,
}

impl CacheStatsSnapshot {
    /// 空 snapshot (K-1 强校验, hit + miss = 0 → hit_rate = 0.0).
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

// ============================================================================
// §3 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: hit_rate 在 0 hit + 0 miss 时返 0.0 (K-1 强校验).
    #[test]
    fn k1_hit_rate_zero_when_no_calls() {
        let s = CacheStats::new(100);
        let snap = s.snapshot();
        assert_eq!(snap.hit_rate, 0.0);
        assert_eq!(snap.hit, 0);
        assert_eq!(snap.miss, 0);
    }

    /// 守门 #2: hit_rate 在 5 hit + 5 miss 时 = 0.5.
    #[test]
    fn hit_rate_5_5_is_50_percent() {
        let s = CacheStats::new(100);
        s.record_hit(10);
        s.record_hit(10);
        s.record_hit(10);
        s.record_hit(10);
        s.record_hit(10);
        s.record_miss(20);
        s.record_miss(20);
        s.record_miss(20);
        s.record_miss(20);
        s.record_miss(20);
        let snap = s.snapshot();
        assert!((snap.hit_rate - 0.5).abs() < 1e-9);
    }

    /// 守门 #3: hit_rate 在 10 hit + 0 miss 时 = 1.0.
    #[test]
    fn hit_rate_10_0_is_100_percent() {
        let s = CacheStats::new(100);
        for _ in 0..10 {
            s.record_hit(5);
        }
        let snap = s.snapshot();
        assert!((snap.hit_rate - 1.0).abs() < 1e-9);
    }

    /// 守门 #4: avg_get_latency_us 在 0 call 时返 0.0 (K-1 强校验).
    #[test]
    fn avg_get_latency_zero_when_no_calls() {
        let s = CacheStats::new(100);
        let snap = s.snapshot();
        assert_eq!(snap.avg_get_latency_us, 0.0);
        assert_eq!(snap.avg_put_latency_us, 0.0);
        assert_eq!(snap.avg_remove_latency_us, 0.0);
    }

    /// 守门 #5: avg_get_latency 计算正确 (5 次 hit 每次 10us → 10us).
    #[test]
    fn avg_get_latency_calculates_correctly() {
        let s = CacheStats::new(100);
        for _ in 0..5 {
            s.record_hit(10);
        }
        let snap = s.snapshot();
        assert!((snap.avg_get_latency_us - 10.0).abs() < 1e-9);
    }

    /// 守门 #6: reset 清空全部计数 (max_size 不被 reset).
    #[test]
    fn reset_clears_all_counters() {
        let s = CacheStats::new(100);
        s.record_hit(10);
        s.record_miss(20);
        s.record_put(30);
        s.set_size(80);
        s.record_eviction();
        s.reset();
        let snap = s.snapshot();
        assert_eq!(snap.hit, 0);
        assert_eq!(snap.miss, 0);
        assert_eq!(snap.put_count, 0);
        assert_eq!(snap.size, 0);
        assert_eq!(snap.evictions, 0);
        // max_size 不被 reset
        assert_eq!(snap.max_size, 100);
    }

    /// 守门 #7: snapshot 是不可变 clone.
    #[test]
    fn snapshot_is_immutable() {
        let s = CacheStats::new(100);
        let snap1 = s.snapshot();
        s.record_hit(10);
        let snap2 = s.snapshot();
        assert_eq!(snap1.hit, 0);
        assert_eq!(snap2.hit, 1);
    }

    /// 守门 #8: record_remove 计数 + latency.
    #[test]
    fn record_remove_works() {
        let s = CacheStats::new(100);
        s.record_remove(50);
        s.record_remove(70);
        let snap = s.snapshot();
        assert_eq!(snap.remove_count, 2);
        assert!((snap.avg_remove_latency_us - 60.0).abs() < 1e-9);
    }
}
