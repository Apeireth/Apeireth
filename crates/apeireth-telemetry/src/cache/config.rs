//! # CacheConfig — cache 配置 (5 字段)
//!
//! 5 字段配置 (per task spec §3):
//! 1. `max_size: usize` — 最大 item 数
//! 2. `default_ttl: Duration` — 默认 TTL
//! 3. `policy: EvictionPolicy` — 5 淘汰策略之一
//! 4. `shards: usize` — 16-256 分片数
//! 5. `backend: BackendKind` — 4 后端之一
//!
//! ## K-1 强校验
//!
//! - max_size > 0
//! - default_ttl > Duration::ZERO
//! - shards 16..=256
//! - backend.is_implemented() (或显式接受 stub)
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::backend::BackendKind;
use super::policy::EvictionPolicy;

// ============================================================================
// §1 CacheConfig 结构 (5 字段)
// ============================================================================

/// Cache 配置 (5 字段).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 最大 item 数 (K-1 强校验: > 0).
    pub max_size: usize,

    /// 默认 TTL (K-1 强校验: > Duration::ZERO).
    pub default_ttl: Duration,

    /// 5 淘汰策略之一.
    pub policy: EvictionPolicy,

    /// 分片数 (K-1 强校验: 16..=256).
    pub shards: usize,

    /// 4 后端之一 (R20 阶段 6 仅 Memory 实现).
    pub backend: BackendKind,
}

impl CacheConfig {
    /// 默认配置.
    pub fn default_config() -> Self {
        Self {
            max_size: super::DEFAULT_MAX_SIZE,
            default_ttl: Duration::from_secs(super::DEFAULT_TTL_SECS),
            policy: EvictionPolicy::Lru,
            shards: super::DEFAULT_SHARDS,
            backend: BackendKind::Memory,
        }
    }

    /// 校验 (K-1 强校验).
    pub fn validate(&self) -> crate::cache::error::CacheResult<()> {
        use crate::cache::error::CacheError;

        if self.max_size == 0 {
            return Err(CacheError::InvalidMaxSize(0));
        }
        if self.default_ttl == Duration::ZERO {
            return Err(CacheError::InvalidTtl(Duration::ZERO));
        }
        super::shard::validate_shard_count(self.shards)?;
        Ok(())
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

// ============================================================================
// §2 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::error::CacheError;

    /// 守门 #1: CacheConfig 5 字段.
    #[test]
    fn cache_config_5_fields() {
        let c = CacheConfig {
            max_size: 100,
            default_ttl: Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 32,
            backend: BackendKind::Memory,
        };
        // 5 字段赋值
        assert_eq!(c.max_size, 100);
        assert_eq!(c.default_ttl, Duration::from_secs(60));
        assert_eq!(c.policy, EvictionPolicy::Lru);
        assert_eq!(c.shards, 32);
        assert_eq!(c.backend, BackendKind::Memory);
    }

    /// 守门 #2: 默认配置合法.
    #[test]
    fn cache_config_default_validates() {
        let c = CacheConfig::default_config();
        assert!(c.validate().is_ok());
    }

    /// 守门 #3: K-1 max_size = 0 拒.
    #[test]
    fn k1_max_size_zero_rejected() {
        let c = CacheConfig {
            max_size: 0,
            default_ttl: Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 32,
            backend: BackendKind::Memory,
        };
        assert!(matches!(c.validate(), Err(CacheError::InvalidMaxSize(0))));
    }

    /// 守门 #4: K-1 default_ttl = 0 拒.
    #[test]
    fn k1_default_ttl_zero_rejected() {
        let c = CacheConfig {
            max_size: 100,
            default_ttl: Duration::ZERO,
            policy: EvictionPolicy::Lru,
            shards: 32,
            backend: BackendKind::Memory,
        };
        assert!(matches!(c.validate(), Err(CacheError::InvalidTtl(_))));
    }

    /// 守门 #5: shards 8 (< 16) 拒.
    #[test]
    fn k1_shards_8_rejected() {
        let c = CacheConfig {
            max_size: 100,
            default_ttl: Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 8,
            backend: BackendKind::Memory,
        };
        assert!(matches!(
            c.validate(),
            Err(CacheError::InvalidShardCount(8))
        ));
    }

    /// 守门 #6: shards 257 (> 256) 拒.
    #[test]
    fn k1_shards_257_rejected() {
        let c = CacheConfig {
            max_size: 100,
            default_ttl: Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 257,
            backend: BackendKind::Memory,
        };
        assert!(matches!(
            c.validate(),
            Err(CacheError::InvalidShardCount(257))
        ));
    }

    /// 守门 #7: 5 policy 都能作为 CacheConfig.policy.
    #[test]
    fn cache_config_supports_5_policies() {
        for p in EvictionPolicy::ALL {
            let c = CacheConfig {
                max_size: 100,
                default_ttl: Duration::from_secs(60),
                policy: p,
                shards: 32,
                backend: BackendKind::Memory,
            };
            assert!(c.validate().is_ok(), "policy {p} 失败");
        }
    }

    /// 守门 #8: Serialize + Deserialize roundtrip.
    #[test]
    fn cache_config_serde_roundtrip() {
        let c = CacheConfig {
            max_size: 2048,
            default_ttl: Duration::from_secs(120),
            policy: EvictionPolicy::TinyLfu,
            shards: 64,
            backend: BackendKind::Memory,
        };
        let s = serde_json::to_string(&c).unwrap();
        let parsed: CacheConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(c, parsed);
    }

    /// 守门 #9: 4 backend 都能作为 CacheConfig.backend (validate 不拒, 实际使用是 construct 时拒 stub).
    #[test]
    fn cache_config_4_backends_pass_validate() {
        for b in BackendKind::ALL {
            let c = CacheConfig {
                max_size: 100,
                default_ttl: Duration::from_secs(60),
                policy: EvictionPolicy::Lru,
                shards: 32,
                backend: b,
            };
            // validate 不拒 (backend stub 守门在 MemoryCache::new 里)
            assert!(c.validate().is_ok(), "backend {b} validate 失败");
        }
    }

    /// 守门 #10: Default = default_config.
    #[test]
    fn cache_config_default() {
        let c: CacheConfig = Default::default();
        assert_eq!(c, CacheConfig::default_config());
    }
}
