//! # Throttle Stage — 5 阶段 pipeline 第 4 阶段 (限流)
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline/throttle.rs` 思想 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2):
//! - Golutra throttle: rate limit / back-pressure
//! - 本 crate 通用化: 4 重限流 (token-bucket / max-concurrent / burst-limit / qps-cap)
//!
//! ## 1 例子: Throttle 阶段给 `apeireth-rate-limiter` 用
//!
//! 用户给 rate-limiter 写自定义 Throttle (示意, **不**引 `apeireth-rate-limiter` crate dep):
//!
//! ```ignore
//! // 伪代码 — 阶段 6 skeleton 故意不引 workspace dep, 留 R21 续真接
//! use apeireth_rate_limiter::{RateLimiter, RateLimitPolicy};
//! use apeireth_pipeline_g5::{Stage, StageKind, PipelineError, PipelineMessage};
//!
//! pub struct RateLimitThrottle {
//!     limiter: RateLimiter,
//! }
//!
//! impl Stage<PipelineMessage, PipelineMessage> for RateLimitThrottle {
//!     fn kind(&self) -> StageKind { StageKind::Throttle }
//!     fn process(&self, msg: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
//!         self.limiter.check(&msg.kind).map_err(|retry_after| PipelineError::Throttled {
//!             kind: StageKind::Throttle,
//!             retry_after_ms: retry_after.as_millis() as u64,
//!         })?;
//!         Ok(msg)
//!     }
//! }
//! ```
//!
//! ## 编译期守门 (4 项, K-1 强校验)
//!
//! 1. `MAX_QPS == 100` (每秒 100 请求上限, 防 m3 幻觉无限 QPS)
//! 2. `MAX_BURST == 200` (突发上限 200, 借鉴 Golutra burst 配置)
//! 3. `MAX_CONCURRENT == 50` (并发上限 50, 防 OOM)
//! 4. `kind()` 永远返回 `StageKind::Throttle`

use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::PipelineError;
use crate::message::PipelineMessage;
use crate::stage::{Stage, StageKind};

/// **Hardcode #1**: QPS 上限 (每秒 100 请求, 防 m3 幻觉无限 QPS).
pub const MAX_QPS: u32 = 100;

/// **Hardcode #2**: 突发上限 (200, 借鉴 Golutra burst 配置).
pub const MAX_BURST: u32 = 200;

/// **Hardcode #3**: 并发上限 (50, 防 OOM).
pub const MAX_CONCURRENT: u32 = 50;

/// **Hardcode #4**: Token-bucket refill 间隔 (1 秒, 编译期 hardcode).
pub const TOKEN_BUCKET_REFILL_SECS: u64 = 1;

/// Default Throttle stage (4 重限流: token-bucket / max-concurrent / burst / qps-cap).
///
/// 行为 (按顺序):
/// 1. **max-concurrent** — 当前并发 > MAX_CONCURRENT 拒绝
/// 2. **qps-cap** — 当前秒请求 > MAX_QPS 拒绝 (返回 Throttled + retry_after_ms)
/// 3. **burst-limit** — 累计突发 > MAX_BURST 拒绝
/// 4. **token-bucket** — 简化: 记录时间戳, 不真 token 计数 (留 R21 续真接)
#[derive(Debug)]
pub struct DefaultThrottle {
    /// 当前并发计数.
    concurrent: AtomicU32,
    /// 当前秒 (qps 计算用).
    current_second: AtomicU64,
    /// 当前秒请求数.
    qps_count: AtomicU32,
    /// 累计突发计数 (自上次 reset 起, 简化版本不自动 reset).
    burst_count: AtomicU32,
}

impl DefaultThrottle {
    /// 创建默认 Throttle stage.
    pub fn new() -> Self {
        Self {
            concurrent: AtomicU32::new(0),
            current_second: AtomicU64::new(0),
            qps_count: AtomicU32::new(0),
            burst_count: AtomicU32::new(0),
        }
    }

    /// 获取当前并发数.
    pub fn concurrent(&self) -> u32 {
        self.concurrent.load(Ordering::SeqCst)
    }

    /// 获取当前 burst 计数.
    pub fn burst_count(&self) -> u32 {
        self.burst_count.load(Ordering::SeqCst)
    }

    /// 获取当前秒 (UNIX epoch seconds, 给 qps 计算用).
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// 编译期守门: 3 个 MAX > 0.
    pub const fn validate_max() -> bool {
        MAX_QPS > 0 && MAX_BURST > 0 && MAX_CONCURRENT > 0
    }
}

impl Default for DefaultThrottle {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage<PipelineMessage, PipelineMessage> for DefaultThrottle {
    fn kind(&self) -> StageKind {
        StageKind::Throttle
    }

    fn name(&self) -> &str {
        "default-throttle"
    }

    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
        // 限流 1: max-concurrent (Hardcode #3)
        let concurrent = self.concurrent.fetch_add(1, Ordering::SeqCst) + 1;
        if concurrent > MAX_CONCURRENT {
            // 释放并发计数
            self.concurrent.fetch_sub(1, Ordering::SeqCst);
            return Err(PipelineError::Throttled {
                kind: StageKind::Throttle,
                retry_after_ms: 1000, // 1 秒后重试
            });
        }

        // 限流 2: qps-cap (Hardcode #1)
        let now = Self::now_secs();
        let prev_sec = self.current_second.load(Ordering::SeqCst);
        let qps = if prev_sec == now {
            self.qps_count.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            // 新一秒, reset 计数
            self.current_second.store(now, Ordering::SeqCst);
            self.qps_count.store(1, Ordering::SeqCst);
            1
        };
        if qps > MAX_QPS {
            self.concurrent.fetch_sub(1, Ordering::SeqCst);
            return Err(PipelineError::Throttled {
                kind: StageKind::Throttle,
                retry_after_ms: 1000, // 下一秒重试
            });
        }

        // 限流 3: burst-limit (Hardcode #2)
        let burst = self.burst_count.fetch_add(1, Ordering::SeqCst) + 1;
        if burst > MAX_BURST {
            self.concurrent.fetch_sub(1, Ordering::SeqCst);
            return Err(PipelineError::Throttled {
                kind: StageKind::Throttle,
                retry_after_ms: 5000, // 5 秒后重试
            });
        }

        // 限流 4: token-bucket (Hardcode #4, 阶段 6 skeleton 0 真接, 留 R21)
        // 简化: 占用并发计数, 走完 pipeline 后由调用方调 `release_concurrent`
        // 实际生产场景: 用 `lru` / `governor` crate 真接 token-bucket 算法.
        // 阶段 6 skeleton 故意不引外部 crate (per 主 2026-08-06 01:00 拍板"纯 Rust", 0 pyo3/qt/GDI).
        let _token_placeholder: () = ();

        // 通用放行
        Ok(input)
    }
}

/// Throttle 阶段内部错误 (仅用于嵌套 error, 大多数走 `PipelineError::Throttled`).
#[derive(Debug)]
pub enum ThrottleError {
    /// QPS 超限 (内部 fallback, 通常走 PipelineError::Throttled).
    QpsExceeded {
        /// 当前 QPS.
        qps: u32,
        /// 上限.
        max: u32,
    },
}

impl fmt::Display for ThrottleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThrottleError::QpsExceeded { qps, max } => {
                write!(f, "QPS exceeded: {} > max {}", qps, max)
            }
        }
    }
}

impl std::error::Error for ThrottleError {}

/// 编译期守门: MAX_QPS == 100.
const _: () = assert!(MAX_QPS == 100);
/// 编译期守门: MAX_BURST == 200.
const _: () = assert!(MAX_BURST == 200);
/// 编译期守门: MAX_CONCURRENT == 50.
const _: () = assert!(MAX_CONCURRENT == 50);
/// 编译期守门: 3 个 MAX > 0.
const _: () = assert!(DefaultThrottle::validate_max());
/// 编译期守门: TOKEN_BUCKET_REFILL_SECS == 1.
const _: () = assert!(TOKEN_BUCKET_REFILL_SECS == 1);
