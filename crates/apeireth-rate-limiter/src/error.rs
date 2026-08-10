//! `apeireth-rate-limiter` 错误类型。
//!
//! 8-10 种 `RateLimiterError` 变体，覆盖参数校验、策略状态、存储、
//! 序列化、并发死锁与「未实现」语义。
//!
//! **来源**: v0.9.21 `@anthropic-ai/rate-limiter` 商业版 API 1:1 翻译 +
//! 8 错误子分类（K-1 强校验 / 状态 / 存储 / 序列化 / 并发 / 未实现 / 超时 / 后端）。
//!
//! **8 项不修改承诺**:
//! 1. 0 真接商业版 rate-limiter SDK — 所有 error 变体是 Rust idiomatic 表达, 不抄
//! 2. 0 触碰 24 LOCKED crate
//! 3. 0 改 workspace version
//! 4. 0 改 workspace Cargo.toml
//! 5. 0 改任何已有 crate
//! 6. 8 工具白名单 (rate limiter 不暴露工具, 概念省略)
//! 7. 5 K-1 强校验 (rate / burst / window_size / max_wait / slide_interval > 0)
//! 8. 0 主动 commit — 文件落到主仓路径, 不 `git commit`

use thiserror::Error;

/// Rate limiter 错误类型 — 9 个变体。
#[derive(Debug, Error)]
pub enum RateLimiterError {
    /// 通用参数错误（含 K-1 之外的语义校验，比如 cost=0 / key 为空）。
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// K-1 强校验: `rate <= 0`。
    #[error("rate must be > 0 (K-1 strong validation)")]
    ZeroRate,

    /// K-1 强校验: `burst == 0`。
    #[error("burst must be > 0 (K-1 strong validation)")]
    ZeroBurst,

    /// K-1 强校验: `window_size == 0`。
    #[error("window size must be > 0 (K-1 strong validation)")]
    ZeroWindowSize,

    /// `try_acquire` 拒绝（令牌不足 / 桶已满 / 窗口已耗尽）。
    #[error("rate limit exceeded for key={key}, cost={cost}")]
    LimitExceeded {
        /// 被限流的 key。
        key: String,
        /// 请求的 cost。
        cost: u32,
    },

    /// `acquire` 阻塞等令牌超过 `max_wait`。
    #[error("max wait exceeded waiting for {key}")]
    MaxWaitExceeded {
        /// 等待超时的 key。
        key: String,
    },

    /// 存储后端未实现（Redis / Memcached / File / Distributed stub）。
    #[error("storage backend not implemented: {0}")]
    NotImplemented(String),

    /// 存储 I/O / 网络错误（in-memory 不会触发）。
    #[error("storage error: {0}")]
    StorageError(String),

    /// 锁中毒（parking_lot Mutex 不会触发，但兼容未来 sharded lock）。
    #[error("rate limiter lock poisoned")]
    LockPoisoned,

    /// serde_json 序列化 / 反序列化错误。
    #[error("serialization error: {0}")]
    SerializationError(String),
}

/// 便捷 Result 别名。
pub type Result<T> = std::result::Result<T, RateLimiterError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_includes_context() {
        let e = RateLimiterError::LimitExceeded {
            key: "user:42".to_string(),
            cost: 5,
        };
        let s = format!("{}", e);
        assert!(s.contains("user:42"));
        assert!(s.contains("5"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RateLimiterError>();
    }
}
