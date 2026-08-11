//! # TTL — 过期机制 (lazy + eager)
//!
//! 2 种过期策略 (per task spec §3):
//! 1. **Lazy**: get 时检查是否过期, 过期则返 None + 删除
//! 2. **Eager**: 后台 task 周期扫描, 提前清掉过期 key
//!
//! ## 设计原则
//!
//! - Lazy 是兜底 (eager 漏掉时一定命中), 永远启用
//! - Eager 是优化 (减少 lazy miss), 由调用方决定开/关
//! - K-1 强校验: ttl = Duration::ZERO 拒收 (per CacheError::InvalidTtl)
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::time::{Duration, Instant};

// ============================================================================
// §1 TtlMode 枚举 (lazy / eager / both)
// ============================================================================

/// TTL 模式 (lazy / eager / both).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版的 TTL 模式.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TtlMode {
    /// Lazy: get 时检查 + 删除 (兜底).
    Lazy,

    /// Eager: 后台 task 周期扫描 + 删除.
    Eager,

    /// Both: lazy + eager 双保险 (推荐, 默认).
    Both,
}

impl TtlMode {
    /// 3 模式 1:1 列表.
    pub const ALL: [TtlMode; 3] = [TtlMode::Lazy, TtlMode::Eager, TtlMode::Both];

    /// 是否启用 lazy 检查.
    #[inline]
    pub const fn is_lazy_enabled(&self) -> bool {
        match self {
            TtlMode::Lazy | TtlMode::Both => true,
            TtlMode::Eager => false,
        }
    }

    /// 是否启用 eager 后台扫描.
    #[inline]
    pub const fn is_eager_enabled(&self) -> bool {
        match self {
            TtlMode::Eager | TtlMode::Both => true,
            TtlMode::Lazy => false,
        }
    }
}

impl Default for TtlMode {
    fn default() -> Self {
        TtlMode::Both
    }
}

impl std::fmt::Display for TtlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtlMode::Lazy => f.write_str("lazy"),
            TtlMode::Eager => f.write_str("eager"),
            TtlMode::Both => f.write_str("both"),
        }
    }
}

// ============================================================================
// §2 TtlEntry 包装 (key + value + 过期时间)
// ============================================================================

/// TTL entry (key + value + 插入时间 + ttl).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache CacheEntry.
#[derive(Debug, Clone)]
pub struct TtlEntry<V> {
    /// 值.
    value: V,
    /// 插入时间 (Instant, 单调时钟).
    inserted_at: Instant,
    /// TTL.
    ttl: Duration,
}

impl<V> TtlEntry<V> {
    /// 构造新 entry (用当前 Instant + 给定 ttl).
    pub fn new(value: V, ttl: Duration) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
            ttl,
        }
    }

    /// 构造带 inserted_at (用于序列化 / 跨进程恢复, 测试用).
    pub fn with_inserted_at(value: V, ttl: Duration, inserted_at: Instant) -> Self {
        Self {
            value,
            inserted_at,
            ttl,
        }
    }

    /// 引用 value.
    #[inline]
    pub fn value(&self) -> &V {
        &self.value
    }

    /// 消费 value.
    #[inline]
    pub fn into_value(self) -> V {
        self.value
    }

    /// TTL.
    #[inline]
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// 插入时间.
    #[inline]
    pub fn inserted_at(&self) -> Instant {
        self.inserted_at
    }

    /// 是否过期 (相对 now, 用 monotonic clock).
    #[inline]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    /// 是否过期 (相对给定的 now, 用于 eager 扫描时统一 now).
    #[inline]
    pub fn is_expired_at(&self, now: Instant) -> bool {
        now.duration_since(self.inserted_at) >= self.ttl
    }

    /// 剩余时间 (None = 已过期).
    #[inline]
    pub fn remaining(&self) -> Option<Duration> {
        self.remaining_at(Instant::now())
    }

    /// 剩余时间 (相对给定的 now).
    #[inline]
    pub fn remaining_at(&self, now: Instant) -> Option<Duration> {
        let elapsed = now.duration_since(self.inserted_at);
        if elapsed >= self.ttl {
            None
        } else {
            Some(self.ttl - elapsed)
        }
    }
}

// ============================================================================
// §3 TtlPolicy 策略 (lazy 检查 + eager 扫描间隔)
// ============================================================================

/// TTL 策略 (eager 扫描间隔).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtlPolicy {
    /// 模式.
    pub mode: TtlMode,
    /// eager 扫描间隔 (默认 1s, 越大越省 CPU, 但过期延迟越大).
    pub scan_interval: Duration,
}

impl TtlPolicy {
    /// 默认策略 (both, 1s 扫描).
    pub const fn default_policy() -> Self {
        Self {
            mode: TtlMode::Both,
            scan_interval: Duration::from_secs(1),
        }
    }

    /// 仅 lazy.
    pub const fn lazy_only() -> Self {
        Self {
            mode: TtlMode::Lazy,
            scan_interval: Duration::from_secs(0),
        }
    }

    /// 仅 eager (用给定扫描间隔).
    pub const fn eager_only(scan_interval: Duration) -> Self {
        Self {
            mode: TtlMode::Eager,
            scan_interval,
        }
    }

    /// lazy + eager (给定扫描间隔).
    pub const fn both(scan_interval: Duration) -> Self {
        Self {
            mode: TtlMode::Both,
            scan_interval,
        }
    }
}

impl Default for TtlPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: TtlMode 3 模式.
    #[test]
    fn ttl_mode_3_all() {
        assert_eq!(TtlMode::ALL.len(), 3);
    }

    /// 守门 #2: TtlMode::Lazy 仅 lazy, 不 eager.
    #[test]
    fn ttl_mode_lazy_only() {
        assert!(TtlMode::Lazy.is_lazy_enabled());
        assert!(!TtlMode::Lazy.is_eager_enabled());
    }

    /// 守门 #3: TtlMode::Eager 仅 eager, 不 lazy.
    #[test]
    fn ttl_mode_eager_only() {
        assert!(!TtlMode::Eager.is_lazy_enabled());
        assert!(TtlMode::Eager.is_eager_enabled());
    }

    /// 守门 #4: TtlMode::Both 双开.
    #[test]
    fn ttl_mode_both_enabled() {
        assert!(TtlMode::Both.is_lazy_enabled());
        assert!(TtlMode::Both.is_eager_enabled());
    }

    /// 守门 #5: TtlEntry 未过期.
    #[test]
    fn ttl_entry_not_expired() {
        let entry = TtlEntry::new("hello".to_string(), Duration::from_secs(60));
        assert!(!entry.is_expired());
        assert!(entry.remaining().is_some());
    }

    /// 守门 #6: TtlEntry 已过期 (构造时用过去的时间).
    #[test]
    fn ttl_entry_expired_with_past_timestamp() {
        let past = Instant::now() - Duration::from_secs(120);
        let entry = TtlEntry::with_inserted_at("hello".to_string(), Duration::from_secs(60), past);
        assert!(entry.is_expired());
        assert!(entry.remaining().is_none());
    }

    /// 守门 #7: TtlEntry is_expired_at 用给定 now.
    #[test]
    fn ttl_entry_is_expired_at() {
        let now = Instant::now();
        let entry = TtlEntry::with_inserted_at(
            42_i32,
            Duration::from_millis(100),
            now - Duration::from_millis(200),
        );
        // 200ms ago + 100ms ttl → 已过期
        assert!(entry.is_expired_at(now));
    }

    /// 守门 #8: TtlEntry remaining 计算正确.
    #[test]
    fn ttl_entry_remaining() {
        let now = Instant::now();
        let entry = TtlEntry::with_inserted_at(
            "x".to_string(),
            Duration::from_secs(10),
            now - Duration::from_secs(3),
        );
        let rem = entry.remaining_at(now).unwrap();
        // 容差 ±1s (Instant 精度)
        assert!(rem.as_secs() >= 6 && rem.as_secs() <= 7);
    }

    /// 守门 #9: TtlPolicy::default_policy 用 both + 1s.
    #[test]
    fn ttl_policy_default_is_both_1s() {
        let p = TtlPolicy::default_policy();
        assert_eq!(p.mode, TtlMode::Both);
        assert_eq!(p.scan_interval, Duration::from_secs(1));
    }

    /// 守门 #10: TtlPolicy::lazy_only 不 eager.
    #[test]
    fn ttl_policy_lazy_only() {
        let p = TtlPolicy::lazy_only();
        assert_eq!(p.mode, TtlMode::Lazy);
    }

    /// 守门 #11: TtlPolicy::eager_only(s) 用给定 interval.
    #[test]
    fn ttl_policy_eager_only_custom() {
        let p = TtlPolicy::eager_only(Duration::from_millis(500));
        assert_eq!(p.mode, TtlMode::Eager);
        assert_eq!(p.scan_interval, Duration::from_millis(500));
    }

    /// 守门 #12: TtlEntry value 消费.
    #[test]
    fn ttl_entry_into_value() {
        let entry = TtlEntry::new(String::from("v"), Duration::from_secs(1));
        let v = entry.into_value();
        assert_eq!(v, "v");
    }
}
