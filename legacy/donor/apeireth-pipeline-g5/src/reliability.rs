//! # Reliability Stage — 5 阶段 pipeline 第 3 阶段 (可靠性)
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline/reliability.rs` 思想 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2):
//! - Golutra reliability: 重试 (retry) / 幂等 (idempotency) / 限频重连 (reconnect)
//! - 本 crate 通用化: 4 重可靠性 (attempt-counter / idempotency-key / backoff / circuit-breaker-stub)
//!
//! ## R21 续补 (per 整合 #3 决策 F-3 — sandbox 真接 6 API)
//!
//! 整合 #3 报告 `integrate-3-impact-analysis-2026-08-06.md` F-3 决策:
//! "sandbox 真接 6 API + 19 tests (含 pipeline-g5 Reliability 阶段集成)"
//!
//! Reliability 阶段的 `IDEMPOTENCY_KEY_PREFIX = "sandbox-"` 已对齐 sandbox 集成 schema
//! (R21+ 续真接 `apeireth-sandbox` crate, 当前 skeleton 0 假装已集成, 仅 schema 对齐).
//!
//! ## 1 例子: Reliability 阶段给 `apeireth-http-client` 用
//!
//! 用户给 HTTP client 写自定义 Reliability (示意, **不**引 `apeireth-http-client` crate dep):
//!
//! ```ignore
//! // 伪代码 — 阶段 6 skeleton 故意不引 workspace dep, 留 R21 续真接
//! use apeireth_http_client::HttpClient;
//! use apeireth_pipeline_g5::{Stage, StageKind, PipelineError, PipelineMessage};
//!
//! pub struct HttpReliability {
//!     client: HttpClient,
//!     max_retries: u32,
//! }
//!
//! impl Stage<PipelineMessage, PipelineMessage> for HttpReliability {
//!     fn kind(&self) -> StageKind { StageKind::Reliability }
//!     fn process(&self, mut msg: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
//!         loop {
//!             match self.client.fetch(&msg) {
//!                 Ok(resp) => return Ok(PipelineMessage { payload: resp, ..msg }),
//!                 Err(e) if msg.attempt < self.max_retries => {
//!                     msg.attempt += 1;
//!                     continue; // retry
//!                 }
//!                 Err(e) => return Err(PipelineError::Stage {
//!                     kind: StageKind::Reliability,
//!                     source: Box::new(e),
//!                 }),
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## 编译期守门 (5 项, K-1 强校验)
//!
//! 1. `MAX_RETRY_ATTEMPTS == 5` (最大重试 5 次, 防无限重试 DoS)
//! 2. `RETRY_BACKOFF_MS` 编译期 hardcode (4 步: 100 / 200 / 500 / 1000 ms, 借鉴 Golutra exponential backoff)
//! 3. `IDEMPOTENCY_KEY_PREFIX` 编译期 hardcode (e.g. "pl-g5-")
//! 4. `CIRCUIT_BREAKER_THRESHOLD == 10` (连续失败 10 次触发, 防雪崩)
//! 5. `kind()` 永远返回 `StageKind::Reliability`

use std::fmt;

use crate::error::PipelineError;
use crate::message::PipelineMessage;
use crate::stage::{Stage, StageKind};

/// **Hardcode #1**: 最大重试次数 (5, 防无限重试 DoS).
pub const MAX_RETRY_ATTEMPTS: u32 = 5;

/// **Hardcode #2**: 重试 backoff 4 步 (100 / 200 / 500 / 1000 ms, 借鉴 Golutra exponential backoff).
///
/// 阶段 6 skeleton 给 4 步固定值, R21 续做动态 backoff.
pub const RETRY_BACKOFF_MS: &[u64] = &[100, 200, 500, 1000];

/// **Hardcode #3**: 幂等 key 前缀 (`"sandbox-"`, 跟整合 #3 决策 F-3 sandbox 真接 schema 对齐).
///
/// R21+ 续真接 `apeireth-sandbox` crate (6 API: ContainerCreate / ProcessExec / WasmRun / 等),
/// 当前 skeleton 0 假装已集成, 仅 schema 名称跟 sandbox 对齐. 跨 stage 唯一标识一个 retry attempt.
pub const IDEMPOTENCY_KEY_PREFIX: &str = "sandbox-";

/// **Hardcode #4**: 熔断器触发阈值 (连续失败 10 次, 防雪崩).
pub const CIRCUIT_BREAKER_THRESHOLD: u32 = 10;

/// **Hardcode #5 (R21 续补, K-1 强校验)**: backoff 步数 (4, 跟 `RETRY_BACKOFF_MS.len()` 守门).
///
/// per 任务 spec "Reliability 阶段 ... backoff 4 步" + 整合 #3 决策 F-3 sandbox 集成.
pub const RELIABILITY_RETRY_BACKOFF_STEP_COUNT: usize = 4;

/// Default Reliability stage (4 重: attempt-counter / idempotency-key / backoff / circuit-breaker).
///
/// 行为:
/// 1. **attempt-counter** — 每次重试 +1, 超过 MAX_RETRY_ATTEMPTS 拒绝
/// 2. **idempotency-key** — 给 message 加 trace_id 前缀 (跨 stage 唯一)
/// 3. **backoff** — 按 RETRY_BACKOFF_MS 4 步查 backoff 时间 (查询用, 实际 sleep 留给上层)
/// 4. **circuit-breaker** — 累计 attempt > CIRCUIT_BREAKER_THRESHOLD 触发, 当前仅返回警告
///   (留 R21 续真接, 阶段 6 skeleton 0 真断路)
///
/// **不** derive `Clone` 因为 `AtomicU32` 不实现 `Clone` (per std::sync::atomic 限制).
/// 如要 clone stage 实例, 重新 `new()` 一个.
#[derive(Debug, Default)]
pub struct DefaultReliability {
    /// 累计失败次数 (跨多次 run 累加, 阶段 6 skeleton 用 AtomicU32, 真断路器留 R21).
    failure_count: std::sync::atomic::AtomicU32,
}

impl DefaultReliability {
    /// 创建默认 Reliability stage.
    pub fn new() -> Self {
        Self {
            failure_count: std::sync::atomic::AtomicU32::new(0),
        }
    }

    /// 计算 backoff 时间 (ms, 给 attempt 索引).
    pub fn backoff_ms(attempt: u32) -> u64 {
        if attempt == 0 {
            return 0;
        }
        let idx = (attempt as usize - 1).min(RETRY_BACKOFF_MS.len() - 1);
        RETRY_BACKOFF_MS[idx]
    }

    /// 累计失败计数 +1 (给 circuit-breaker 用).
    pub fn record_failure(&self) -> u32 {
        self.failure_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1
    }

    /// 重置失败计数 (circuit-breaker reset).
    pub fn reset_failures(&self) {
        self.failure_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    /// 获取当前失败计数.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 编译期守门: RETRY_BACKOFF_MS.len() == 4.
    pub const fn validate_backoff_steps() -> bool {
        RETRY_BACKOFF_MS.len() == 4
    }
}

impl Stage<PipelineMessage, PipelineMessage> for DefaultReliability {
    fn kind(&self) -> StageKind {
        StageKind::Reliability
    }

    fn name(&self) -> &str {
        "default-reliability"
    }

    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
        // 守门 1: attempt 计数 (Hardcode #1)
        if input.attempt > MAX_RETRY_ATTEMPTS {
            // 累计失败 (circuit-breaker 计数器)
            let count = self.record_failure();

            // 守门 4: 熔断器 (Hardcode #4, 阶段 6 仅警告, R21 续真断)
            if count > CIRCUIT_BREAKER_THRESHOLD {
                return Err(PipelineError::Stage {
                    kind: StageKind::Reliability,
                    source: Box::new(ReliabilityError::CircuitBreakerOpen {
                        failure_count: count,
                        threshold: CIRCUIT_BREAKER_THRESHOLD,
                    }),
                });
            }

            return Err(PipelineError::Stage {
                kind: StageKind::Reliability,
                source: Box::new(ReliabilityError::MaxRetriesExceeded {
                    attempt: input.attempt,
                    max: MAX_RETRY_ATTEMPTS,
                }),
            });
        }

        // 守门 2: idempotency-key (Hardcode #3, 阶段 6 仅加 trace_id 前缀, 留 R21 续真接)
        let trace_id = if input.trace_id.is_empty() {
            format!("{}{}", IDEMPOTENCY_KEY_PREFIX, input.attempt)
        } else {
            input.trace_id
        };

        // 守门 3: backoff (Hardcode #2, 阶段 6 仅查询, 不真 sleep, 留给上层 async runtime)
        let backoff = Self::backoff_ms(input.attempt);
        // 阶段 6 skeleton 故意 0 真 sleep: sync framework, 真 sleep 用 std::thread::sleep 会
        // 阻塞, 不适合 server 场景. R21 续做时用 tokio::time::sleep 异步 backoff.
        let _backoff_ms: u64 = backoff;

        // 通用放行, attempt +1 (给下游 stage 知道这是第 N 次)
        Ok(PipelineMessage {
            kind: input.kind,
            payload: input.payload,
            attempt: input.attempt + 1,
            trace_id,
        })
    }
}

/// Reliability 阶段内部错误.
#[derive(Debug)]
pub enum ReliabilityError {
    /// 超过最大重试次数.
    MaxRetriesExceeded {
        /// 实际 attempt.
        attempt: u32,
        /// 上限 MAX_RETRY_ATTEMPTS.
        max: u32,
    },
    /// 熔断器触发 (阶段 6 skeleton 触发后真拒绝).
    CircuitBreakerOpen {
        /// 累计失败次数.
        failure_count: u32,
        /// 熔断阈值.
        threshold: u32,
    },
}

impl fmt::Display for ReliabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReliabilityError::MaxRetriesExceeded { attempt, max } => {
                write!(f, "max retries exceeded: attempt {} > max {}", attempt, max)
            }
            ReliabilityError::CircuitBreakerOpen {
                failure_count,
                threshold,
            } => {
                write!(
                    f,
                    "circuit breaker open: failure_count {} > threshold {}",
                    failure_count, threshold
                )
            }
        }
    }
}

impl std::error::Error for ReliabilityError {}

/// 编译期字符串相等比较 (per std::str::eq 不是 const-stable, 自实现字节比较).
const fn const_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 编译期守门: MAX_RETRY_ATTEMPTS == 5.
const _: () = assert!(MAX_RETRY_ATTEMPTS == 5);
/// 编译期守门: RETRY_BACKOFF_MS.len() == 4.
const _: () = assert!(DefaultReliability::validate_backoff_steps());
/// 编译期守门: RELIABILITY_RETRY_BACKOFF_STEP_COUNT == 4 (R21 续补, K-1 强校验).
const _: () = assert!(RELIABILITY_RETRY_BACKOFF_STEP_COUNT == 4);
/// 编译期守门: RELIABILITY_RETRY_BACKOFF_STEP_COUNT == RETRY_BACKOFF_MS.len() (R21 续补, 防 m3 幻觉改一边不改另一边).
const _: () = assert!(RELIABILITY_RETRY_BACKOFF_STEP_COUNT == RETRY_BACKOFF_MS.len());
/// 编译期守门: RETRY_BACKOFF_MS[0] == 100.
const _: () = assert!(RETRY_BACKOFF_MS[0] == 100);
/// 编译期守门: IDEMPOTENCY_KEY_PREFIX == "sandbox-" (R21 续补, 跟整合 #3 决策 F-3 sandbox 真接对齐).
const _: () = assert!(const_str_eq(IDEMPOTENCY_KEY_PREFIX, "sandbox-"));
/// 编译期守门: CIRCUIT_BREAKER_THRESHOLD == 10.
const _: () = assert!(CIRCUIT_BREAKER_THRESHOLD == 10);
