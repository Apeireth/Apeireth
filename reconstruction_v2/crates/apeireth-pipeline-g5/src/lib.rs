//! `apeireth-pipeline-g5` — v2 generic 5-stage pipeline substrate.
//!
//! v1 API surface preserved: `FIVE_STAGES`, `Pipeline`, `PipelineConfig`,
//! `PipelineMessage`, `Stage`, `StageKind`, `StageOp`, `StageEntry`,
//! `STAGE_KIND_COUNT`, `STAGE_ORDER`, `PIPELINE_MIN_STAGES`, `PIPELINE_MAX_STAGES`,
//! `PIPELINE_STAGE_NAME_MAX_LEN`, `MAX_KIND_LEN`, `MAX_PAYLOAD_LEN`, `MAX_TRACE_ID_LEN`,
//! `DefaultDispatch`, `DefaultNormalize`, `DefaultPolicy`, `DefaultReliability`,
//! `DefaultThrottle`, `CIRCUIT_BREAKER_THRESHOLD`, `IDEMPOTENCY_KEY_PREFIX`,
//! `MAX_RETRY_ATTEMPTS`, `RETRY_BACKOFF_MS`, `MAX_POLICY_ATTEMPTS`,
//! `MAX_POLICY_PAYLOAD_SIZE`, `POLICY_DENY_KINDS`, `POLICY_REQUIRE_KIND`,
//! `MAX_BURST`, `MAX_CONCURRENT`, `MAX_QPS`, `TOKEN_BUCKET_REFILL_SECS`,
//! `CircuitBreaker`, `BoundedReliability`, `PipelineError`, `PipelineErrorKind`,
//! `PIPELINE_ERROR_VARIANT_COUNT`, `PLATFORM_NAME`, `PIPELINE_G5_SCHEMA_VERSION`,
//! `PIPELINE_G5_STAGE_COUNT`, `PIPELINE_G5_MAX_STAGES`.

#![allow(dead_code)]

pub mod bounded_reliability;
pub mod circuit_breaker;
pub mod dispatch;
pub mod error;
pub mod message;
pub mod normalize;
pub mod pipeline;
pub mod policy;
pub mod reliability;
pub mod stage;
pub mod throttle;

pub use dispatch::DefaultDispatch;
pub use error::{PipelineError, PipelineErrorKind, PIPELINE_ERROR_VARIANT_COUNT};
pub use message::{PipelineMessage, MAX_KIND_LEN, MAX_PAYLOAD_LEN, MAX_TRACE_ID_LEN};
pub use normalize::DefaultNormalize;
pub use pipeline::{Pipeline, PipelineConfig, PIPELINE_MAX_STAGES, PIPELINE_MIN_STAGES, PIPELINE_STAGE_NAME_MAX_LEN};
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

/// 5 stages hardcoded names.
pub const FIVE_STAGES: [&str; 5] = [
    "0 Dispatch",
    "1 Normalize",
    "2 Policy",
    "3 Reliability",
    "4 Throttle",
];

/// Platform name.
pub const PLATFORM_NAME: &str = "apeireth";
/// Schema version.
pub const PIPELINE_G5_SCHEMA_VERSION: &str = "1";
/// Stage count (alias).
pub const PIPELINE_G5_STAGE_COUNT: usize = STAGE_KIND_COUNT;
/// Max stages alias.
pub const PIPELINE_G5_MAX_STAGES: usize = PIPELINE_MAX_STAGES;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_stages_hardcoded() {
        assert_eq!(FIVE_STAGES.len(), 5);
        assert_eq!(STAGE_KIND_COUNT, 5);
        assert_eq!(STAGE_ORDER.len(), 5);
        assert_eq!(STAGE_ORDER, FIVE_STAGES);
    }

    #[test]
    fn platform_constants() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PIPELINE_G5_SCHEMA_VERSION, "1");
        assert_eq!(PIPELINE_G5_STAGE_COUNT, 5);
    }
}
