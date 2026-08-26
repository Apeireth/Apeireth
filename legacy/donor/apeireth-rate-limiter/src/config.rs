//! `apeireth-rate-limiter` 配置类型。
//!
//! 4 段配置: `BucketConfig` / `StrategyConfig` / `StorageConfig` / `ObservabilityConfig`,
//! 顶层聚合为 `RateLimiterConfig`。
//!
//! **来源**: v0.9.21 `@anthropic-ai/rate-limiter` 商业版 `RateLimiterOptions` 1:1 翻译
//! + 4 段拆分（bucket / strategy / storage / observability）, 每段含 4-5 字段。
//!
//! **设计取舍**:
//! - 所有字段 `pub` — 编译期 hardcode 也能 serde 反序列化（调试 / 配置中心用）
//! - 用 `Duration` 表达时间 — 不暴露裸 `u64` ms / ns, 强制单位
//! - 枚举字段 `Copy` — 构造简单, 无需 clone

use serde::{Deserialize, Serialize};
use std::time::Duration;

// =====================================================================
// 顶层
// =====================================================================

/// Rate limiter 完整配置（4 段聚合）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateLimiterConfig {
    /// 桶 / 限流参数（rate / burst / initial / max_wait / refill_interval）。
    pub bucket: BucketConfig,
    /// 策略选择 + 策略专属参数（4 种算法共用）。
    pub strategy: StrategyConfig,
    /// 存储后端选择 + 连接信息。
    pub storage: StorageConfig,
    /// 可观测性参数（stats / metrics）。
    pub observability: ObservabilityConfig,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            bucket: BucketConfig::default(),
            strategy: StrategyConfig::default(),
            storage: StorageConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

// =====================================================================
// 段 1: bucket
// =====================================================================

/// 桶配置 — 5 字段: rate / burst / initial / max_wait / refill_interval。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BucketConfig {
    /// 每秒补充令牌数（必须 > 0, K-1 强校验）。
    pub rate_per_second: f64,
    /// 桶容量 / 突发上限（必须 > 0, K-1 强校验）。
    pub burst: u32,
    /// 启动时初始令牌数（默认 = `burst`）。
    pub initial_tokens: Option<u32>,
    /// `acquire` 阻塞最长等待（None = 不阻塞, 立即返回 try_acquire 语义）。
    pub max_wait: Option<Duration>,
    /// 补充周期（默认 100ms, 1ms-1s 之间）。
    pub refill_interval: Duration,
}

impl Default for BucketConfig {
    fn default() -> Self {
        Self {
            rate_per_second: 10.0,
            burst: 20,
            initial_tokens: None,
            max_wait: Some(Duration::from_secs(5)),
            refill_interval: Duration::from_millis(100),
        }
    }
}

impl BucketConfig {
    /// K-1 强校验: rate > 0 && burst > 0 && refill_interval > 0。
    pub fn validate(&self) -> Result<(), crate::error::RateLimiterError> {
        use crate::error::RateLimiterError;
        if self.rate_per_second <= 0.0 {
            return Err(RateLimiterError::ZeroRate);
        }
        if self.burst == 0 {
            return Err(RateLimiterError::ZeroBurst);
        }
        if self.refill_interval.is_zero() {
            return Err(RateLimiterError::InvalidParameter(
                "refill_interval must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

// =====================================================================
// 段 2: strategy
// =====================================================================

/// 策略配置 — 策略选择 + 4 算法各自的参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyConfig {
    /// 4 算法选择。
    pub kind: StrategyKind,
    /// 窗口大小（fixed / sliding 用, token / leaky 不用）。
    pub window_size: Option<Duration>,
    /// 滑窗 slide 步长（sliding 用, 其他 None）。
    pub slide_interval: Option<Duration>,
    /// 窗口内最大请求数（fixed / sliding 用）。
    pub max_requests: Option<u32>,
    /// 滑窗精度（log 精确 / counter 折中）。
    pub precision: Option<SlidingWindowPrecision>,
    /// 漏桶溢出策略（leaky 用, 其他 None）。
    pub overflow_policy: Option<LeakyBucketOverflow>,
    /// 固定窗口重置策略（fixed 用, 其他 None）。
    pub reset_strategy: Option<FixedWindowReset>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            kind: StrategyKind::TokenBucket,
            window_size: None,
            slide_interval: None,
            max_requests: None,
            precision: None,
            overflow_policy: None,
            reset_strategy: None,
        }
    }
}

/// 4 种限流算法。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StrategyKind {
    /// 令牌桶（rate + burst, 允许突发）。
    TokenBucket,
    /// 漏桶（rate 平滑, 溢出策略控制）。
    LeakyBucket,
    /// 固定窗口（max_requests / window, 边界突刺问题）。
    FixedWindow,
    /// 滑动窗口（log 精确 / counter 折中, 解决边界突刺）。
    SlidingWindow,
}

/// 漏桶溢出策略。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LeakyBucketOverflow {
    /// 溢出直接拒绝（默认）。
    Drop,
    /// 溢出阻塞等待（类似 acquire 语义）。
    Block,
}

/// 固定窗口重置策略。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FixedWindowReset {
    /// 窗口结束自动重置（默认）。
    OnWindowEnd,
    /// 仅手动 reset 时重置。
    OnDemand,
    /// 自动 + 手动都生效。
    Both,
}

/// 滑窗精度模式。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SlidingWindowPrecision {
    /// 精确模式：记录每次请求时间戳（O(requests) 内存）。
    Log,
    /// 折中模式：把窗口切成 N 个 sub-bucket（O(N) 内存）。
    Counter,
}

// =====================================================================
// 段 3: storage
// =====================================================================

/// 存储配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageConfig {
    /// 5 种存储后端。
    pub kind: StorageKind,
    /// 连接字符串（Redis/Memcached/Distributed 用, 其他 None）。
    pub connection_string: Option<String>,
    /// 文件路径（File 用, 其他 None）。
    pub file_path: Option<String>,
    /// 分片数（in-memory 用, 减少锁竞争, 默认 16）。
    pub shard_count: Option<usize>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            kind: StorageKind::InMemory,
            connection_string: None,
            file_path: None,
            shard_count: Some(16),
        }
    }
}

/// 5 种存储后端。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StorageKind {
    /// 内存（parking_lot + HashMap, 完整实现, 默认）。
    InMemory,
    /// Redis（network, stub）。
    Redis,
    /// Memcached（network, stub）。
    Memcached,
    /// 文件（持久化, stub）。
    File,
    /// 分布式（consensus, stub）。
    Distributed,
}

// =====================================================================
// 段 4: observability
// =====================================================================

/// 可观测性配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservabilityConfig {
    /// 是否记录命中 / 未命中 / 等待时长。
    pub track_stats: bool,
    /// stats HashMap 最大 key 数（防泄漏, None = 无限）。
    pub max_stats_keys: Option<usize>,
    /// tracing span 名（None = 不输出）。
    pub tracing_span: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            track_stats: true,
            max_stats_keys: Some(10_000),
            tracing_span: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_token_bucket_in_memory() {
        let cfg = RateLimiterConfig::default();
        assert_eq!(cfg.strategy.kind, StrategyKind::TokenBucket);
        assert_eq!(cfg.storage.kind, StorageKind::InMemory);
        assert!(cfg.observability.track_stats);
    }

    #[test]
    fn bucket_config_validate_rejects_zero_rate() {
        let mut cfg = BucketConfig::default();
        cfg.rate_per_second = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn bucket_config_validate_rejects_zero_burst() {
        let mut cfg = BucketConfig::default();
        cfg.burst = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn bucket_config_validate_accepts_positive() {
        let cfg = BucketConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn config_serde_round_trip() {
        let cfg = RateLimiterConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: RateLimiterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }
}
