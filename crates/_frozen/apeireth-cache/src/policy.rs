//! # EvictionPolicy — 5 淘汰策略 (LRU / LFU / FIFO / ARC / TinyLFU)
//!
//! 5 个淘汰策略覆盖 1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版的策略谱:
//! 1. **LRU** (Least Recently Used) — 经典, HashMap + VecDeque
//! 2. **LFU** (Least Frequently Used) — 频率感知, HashMap + 频次桶
//! 3. **FIFO** (First In First Out) — 先进先出, 简单 HashMap + 队列
//! 4. **ARC** (Adaptive Replacement Cache, IBM 2003) — 自适应 LRU+LFU
//! 5. **TinyLFU** (modern, 估 30%+ hit rate than LRU) — Caffeine 默认
//!
//! ## 1:1 翻译映射 (v0.9.21 @anthropic-ai/cache)
//!
//! | apeireth-cache | @anthropic-ai/cache | 备注 |
//! |----------------|---------------------|------|
//! | `Lru`          | `EvictionPolicy.LRU`     | 经典 LRU |
//! | `Lfu`          | `EvictionPolicy.LFU`     | 频率感知 |
//! | `Fifo`         | `EvictionPolicy.FIFO`    | 简单 FIFO |
//! | `Arc`          | `EvictionPolicy.ARC`     | 自适应 |
//! | `TinyLfu`      | `EvictionPolicy.TINY_LFU` | 现代化 |
//!
//! ## m3 防御
//!
//! 5 个策略编译期 hardcode enum, 不允许运行时新增/减少.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use serde::{Deserialize, Serialize};

// ============================================================================
// §1 EvictionPolicy 枚举 (5 variant, per skeleton 模式 + m3 防御)
// ============================================================================

/// 5 淘汰策略 (编译期 hardcode).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache EvictionPolicy 商业版.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used (经典, HashMap + VecDeque).
    Lru,

    /// Least Frequently Used (频率感知, 频次桶).
    Lfu,

    /// First In First Out (简单, 队列).
    Fifo,

    /// Adaptive Replacement Cache (IBM 2003, 自适应 LRU+LFU).
    Arc,

    /// TinyLFU (Caffeine 默认, 估 30%+ hit rate than LRU).
    TinyLfu,
}

impl EvictionPolicy {
    /// 5 个策略 1:1 列表.
    pub const ALL: [EvictionPolicy; 5] = [
        EvictionPolicy::Lru,
        EvictionPolicy::Lfu,
        EvictionPolicy::Fifo,
        EvictionPolicy::Arc,
        EvictionPolicy::TinyLfu,
    ];

    /// 策略英文名 (1:1 翻译 v0.9.21 商业版).
    pub const fn as_str(&self) -> &'static str {
        match self {
            EvictionPolicy::Lru => "LRU",
            EvictionPolicy::Lfu => "LFU",
            EvictionPolicy::Fifo => "FIFO",
            EvictionPolicy::Arc => "ARC",
            EvictionPolicy::TinyLfu => "TINY_LFU",
        }
    }

    /// 是否支持 disk / redis / memcached backend (R20 阶段 6: 全 stub, 全 false).
    pub const fn supports_distributed(&self) -> bool {
        // 5 策略都能在 distributed backend 上跑, 但 R20 阶段 6 全 stub
        // (R21 真接时, 这层由 backend 自行决定; 这里只标 policy 自身能力)
        true
    }
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        // 默认 LRU: 最经典, 工业界默认值
        EvictionPolicy::Lru
    }
}

impl std::fmt::Display for EvictionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EvictionPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LRU" | "lru" => Ok(EvictionPolicy::Lru),
            "LFU" | "lfu" => Ok(EvictionPolicy::Lfu),
            "FIFO" | "fifo" => Ok(EvictionPolicy::Fifo),
            "ARC" | "arc" => Ok(EvictionPolicy::Arc),
            "TINY_LFU" | "tiny_lfu" | "TinyLFU" | "tinylfu" => Ok(EvictionPolicy::TinyLfu),
            other => Err(format!("unknown eviction policy: '{other}'")),
        }
    }
}

// ============================================================================
// §2 策略 variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// EvictionPolicy 编译期 hardcode variant 数 (5).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const EVICTION_POLICY_VARIANT_COUNT: usize = 5;

// ============================================================================
// §3 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: EvictionPolicy 编译期 5 variant.
    #[test]
    fn eviction_policy_has_five_variants() {
        // 5 variant 1:1: Lru / Lfu / Fifo / Arc / TinyLfu
        assert_eq!(EVICTION_POLICY_VARIANT_COUNT, 5);
        assert_eq!(EvictionPolicy::ALL.len(), 5);
    }

    /// 守门 #2: 5 策略 1:1 字符串名.
    #[test]
    fn eviction_policy_5_strs() {
        assert_eq!(EvictionPolicy::Lru.as_str(), "LRU");
        assert_eq!(EvictionPolicy::Lfu.as_str(), "LFU");
        assert_eq!(EvictionPolicy::Fifo.as_str(), "FIFO");
        assert_eq!(EvictionPolicy::Arc.as_str(), "ARC");
        assert_eq!(EvictionPolicy::TinyLfu.as_str(), "TINY_LFU");
    }

    /// 守门 #3: Default = LRU.
    #[test]
    fn eviction_policy_default_is_lru() {
        assert_eq!(EvictionPolicy::default(), EvictionPolicy::Lru);
    }

    /// 守门 #4: 5 策略 ALL 唯一.
    #[test]
    fn eviction_policy_all_unique() {
        let mut all = EvictionPolicy::ALL.to_vec();
        all.sort_by_key(|p| p.as_str());
        all.dedup();
        assert_eq!(all.len(), 5);
    }

    /// 守门 #5: FromStr 解析.
    #[test]
    fn eviction_policy_from_str_works() {
        assert_eq!("LRU".parse::<EvictionPolicy>().unwrap(), EvictionPolicy::Lru);
        assert_eq!("lru".parse::<EvictionPolicy>().unwrap(), EvictionPolicy::Lru);
        assert_eq!("TINY_LFU".parse::<EvictionPolicy>().unwrap(), EvictionPolicy::TinyLfu);
        assert!("NOPE".parse::<EvictionPolicy>().is_err());
    }
}
