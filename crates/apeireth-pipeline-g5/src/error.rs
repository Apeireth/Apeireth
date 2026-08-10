//! # PipelineError — 通用 5 阶段 pipeline 错误类型
//!
//! 6 variant 设计 (per 5 阶段 + 1 catch-all), 编译期 enum exhaustive match 守门.
//! 借鉴 Golutra `chat_db` 错误分类 (per `docs/stage6/borrowed-from-golutra.md` §3) 简化通用化:
//! - Golutra 把错误分 pipeline_stage_error / serialization_error / throttle_error / policy_denied 4 类
//! - 本 crate 扩到 6 类 (加 StageTypeMismatch + InvalidStageOrder 2 类防御, 防 m3 幻觉把类型/顺序搞错)
//!
//! ## 6 variant 守门 (编译期 hardcode)
//!
//! 1. `Stage { kind, source }` — 单个 stage 处理失败, source 透传 inner error
//! 2. `PolicyDenied { kind, reason }` — Policy 阶段明确拒绝
//! 3. `Throttled { kind, retry_after_ms }` — Throttle 阶段限流
//! 4. `StageTypeMismatch { expected, actual }` — stage I/O 类型不匹配 (防御)
//! 5. `InvalidStageOrder { got, want }` — stage 顺序非法 (防御)
//! 6. `EmptyPipeline` — 0 stage (无 chain 跑)

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::stage::StageKind;

/// **Error K-1 强校验 #1**: PipelineError 6 variant (编译期 hardcode, 跟 5 阶段 + 1 防御对齐).
pub const PIPELINE_ERROR_VARIANT_COUNT: usize = 6;

/// 通用 5 阶段 pipeline 错误.
///
/// 跨 stage 错误统一包装, 错误传播支持 `?` (impl `From<Stage<I,O> Error>` via [`Stage`] trait).
#[derive(Debug, Error)]
pub enum PipelineError {
    /// 单个 stage 处理失败 (透传 inner error).
    #[error("[stage:{kind:?}] {source}")]
    Stage {
        /// 失败的 stage kind (e.g. Dispatch, Normalize, ...).
        kind: StageKind,
        /// 透传的底层错误 (e.g. parse error / IO error / user error).
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Policy 阶段明确拒绝 (e.g. deny-list 命中, quota 超限).
    #[error("[policy denied:{kind:?}] reason={reason}")]
    PolicyDenied {
        /// 拒绝的 stage kind (应当是 `Policy`).
        kind: StageKind,
        /// 拒绝原因 (e.g. "kind='spam' in deny list").
        reason: String,
    },

    /// Throttle 阶段限流 (e.g. rate limit exceeded).
    #[error("[throttled:{kind:?}] retry_after_ms={retry_after_ms}")]
    Throttled {
        /// 限流的 stage kind (应当是 `Throttle`).
        kind: StageKind,
        /// 建议重试间隔 (毫秒, 给上层 backoff 决策).
        retry_after_ms: u64,
    },

    /// Stage I/O 类型不匹配 (防御: 编译期抓不到, 运行时 downcast 抓).
    #[error("[stage type mismatch] expected={expected}, actual={actual}")]
    StageTypeMismatch {
        /// 期望的 I/O 类型 (`std::any::type_name::<T>()`).
        expected: &'static str,
        /// 实际收到的 I/O 类型.
        actual: String,
    },

    /// Stage 顺序非法 (防御: Pipeline 要求 5 阶段按 Dispatch → Normalize → Policy → Reliability → Throttle 排列).
    #[error("[invalid stage order] got={got:?}, want={want:?}")]
    InvalidStageOrder {
        /// 收到的 stage kind.
        got: StageKind,
        /// 期望的下一个 stage kind.
        want: StageKind,
    },

    /// Pipeline 0 stage (空 chain, 无 stage 跑).
    #[error("[empty pipeline] no stage to run")]
    EmptyPipeline,
}

/// 编译期守门: PIPELINE_ERROR_VARIANT_COUNT == 6 (K-1 强校验 #1).
const _: () = assert!(PIPELINE_ERROR_VARIANT_COUNT == 6);

/// 序列化友好的 error summary (用于 structured logging / IPC).
///
/// **不**直接序列化 `PipelineError` (因为 `Box<dyn Error>` 不 Serialize), 用 `PipelineErrorKind` 摘要.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineErrorKind {
    /// Stage 处理失败.
    Stage,
    /// Policy 拒绝.
    PolicyDenied,
    /// 限流.
    Throttled,
    /// Stage I/O 类型不匹配.
    StageTypeMismatch,
    /// Stage 顺序非法.
    InvalidStageOrder,
    /// 空 pipeline.
    EmptyPipeline,
}

impl PipelineError {
    /// 提取错误类别 (序列化友好).
    pub fn kind(&self) -> PipelineErrorKind {
        match self {
            PipelineError::Stage { .. } => PipelineErrorKind::Stage,
            PipelineError::PolicyDenied { .. } => PipelineErrorKind::PolicyDenied,
            PipelineError::Throttled { .. } => PipelineErrorKind::Throttled,
            PipelineError::StageTypeMismatch { .. } => PipelineErrorKind::StageTypeMismatch,
            PipelineError::InvalidStageOrder { .. } => PipelineErrorKind::InvalidStageOrder,
            PipelineError::EmptyPipeline => PipelineErrorKind::EmptyPipeline,
        }
    }

    /// 提取 stage kind (如果是 stage-related error).
    pub fn stage_kind(&self) -> Option<StageKind> {
        match self {
            PipelineError::Stage { kind, .. } => Some(*kind),
            PipelineError::PolicyDenied { kind, .. } => Some(*kind),
            PipelineError::Throttled { kind, .. } => Some(*kind),
            PipelineError::InvalidStageOrder { got, .. } => Some(*got),
            _ => None,
        }
    }
}

impl fmt::Display for PipelineErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PipelineErrorKind::Stage => "stage",
            PipelineErrorKind::PolicyDenied => "policy_denied",
            PipelineErrorKind::Throttled => "throttled",
            PipelineErrorKind::StageTypeMismatch => "stage_type_mismatch",
            PipelineErrorKind::InvalidStageOrder => "invalid_stage_order",
            PipelineErrorKind::EmptyPipeline => "empty_pipeline",
        };
        f.write_str(s)
    }
}
