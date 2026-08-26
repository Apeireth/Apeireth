//! R20 阶段 6 估补 — 通用 5 阶段 pipeline (整合 #3 B-7 R21 续补 1:1 重建).
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline` 5 阶段思想 (Dispatch → Normalize → Policy →
//! Reliability → Throttle), 通用化: 任何模型 (chat / task / memory / MCP / ...) 都
//! 能挂上 `Pipeline<T, I, O>`. 5 阶段 enum + 编译期 hardcode 守门 17+ (K-1 强校验).
//!
//! ## R21 G-2 续补 (整合 #4 拍板范畴)
//!
//! - 9 src 真实实现 (dispatch / normalize / policy / reliability / throttle / stage / error / message / pipeline)
//! - 13 集成测试 (test_5_stage_chain_success + ... + test_reliability_backoff)
//! - 编译期 hardcode 守门 (STAGE_KIND_COUNT == 5 / MAX_RETRY_ATTEMPTS == 5 / RETRY_BACKOFF_MS 4 步 /
//!   CIRCUIT_BREAKER_THRESHOLD == 10 / IDEMPOTENCY_KEY_PREFIX == "sandbox-" / ...)

#![allow(missing_docs)]
#![allow(dead_code)]

pub mod bounded_reliability; // R204: DefaultReliability + CircuitBreaker 集成: 真 Circuit Breaker (替换 reliability.rs 的 stub)
pub mod circuit_breaker; // R198
pub mod dispatch;
pub mod error;
pub mod message;
pub mod normalize;
pub mod pipeline;
pub mod policy;
pub mod reliability;
pub mod stage;
pub mod throttle;
// R177: pipeline-g5 invariants (10 tests + 2 Kani proofs)
mod organ_kani_proofs;

/// 5 阶段 pipeline 名称 (R21 G-2 真实 enum, K-1 强校验).
pub const FIVE_STAGES: [&str; 5] = [
    "0 Dispatch",
    "1 Normalize",
    "2 Policy",
    "3 Reliability",
    "4 Throttle",
];

pub use dispatch::DefaultDispatch;
pub use error::{PipelineError, PipelineErrorKind, PIPELINE_ERROR_VARIANT_COUNT};
pub use message::{PipelineMessage, MAX_KIND_LEN, MAX_PAYLOAD_LEN, MAX_TRACE_ID_LEN};
pub use normalize::DefaultNormalize;
pub use pipeline::{
    Pipeline, PipelineConfig, PIPELINE_MAX_STAGES, PIPELINE_MIN_STAGES, PIPELINE_STAGE_NAME_MAX_LEN,
};
pub use policy::{
    DefaultPolicy, MAX_POLICY_ATTEMPTS, MAX_POLICY_PAYLOAD_SIZE, POLICY_DENY_KINDS,
    POLICY_REQUIRE_KIND,
};
pub use reliability::{
    DefaultReliability, CIRCUIT_BREAKER_THRESHOLD, IDEMPOTENCY_KEY_PREFIX, MAX_RETRY_ATTEMPTS,
    RETRY_BACKOFF_MS,
};
pub use stage::{Stage, StageEntry, StageKind, StageOp, STAGE_KIND_COUNT, STAGE_ORDER};
pub use throttle::{DefaultThrottle, MAX_BURST, MAX_CONCURRENT, MAX_QPS, TOKEN_BUCKET_REFILL_SECS};

// R21 估补 placeholder 守门常量 (per BORROW_FROM_GOLUTRA.md §8 P2 schema 概念, R21+ 续
// 真接 apeireth-pipeline-g5 6 阶段 schema). 当前 placeholder 跟 sandbox 同步, 后续 R21+ 续.
pub const PLATFORM_NAME: &str = "apeireth";
pub const PIPELINE_G5_SCHEMA_VERSION: &str = "1";
pub const PIPELINE_G5_STAGE_COUNT: usize = STAGE_KIND_COUNT;
pub const PIPELINE_G5_MAX_STAGES: usize = PIPELINE_MAX_STAGES;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_stages_hardcoded() {
        assert_eq!(FIVE_STAGES.len(), 5);
        assert_eq!(STAGE_KIND_COUNT, 5);
        assert_eq!(STAGE_ORDER.len(), 5);
    }
}
