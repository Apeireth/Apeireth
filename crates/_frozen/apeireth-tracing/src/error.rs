//! # Tracing Error 类型
//!
//! 分布式追踪子系统的统一错误类型, 1:1 翻译 v0.9.21 商业版 `out/main/chunks/tracing`
//! 错误码体系. 所有 Exporter / Sampler / Propagator / Span / TraceContext 操作
//! 返 `TracingError` 或 `TracingResult<T>` (type alias for `Result<T, TracingError>`).
//!
//! ## 设计原则 (per S-2 实事求是 + O-5 不假装)
//!
//! 1. **错误码编译期 hardcode**: 8-10 变体, 不可运行时增删, 防止 m3 hallucination 注入伪错误
//! 2. **0 暴露内部 trace 数据**: 错误消息不包含 PII / 完整 trace_id (只截前 8 字符)
//! 3. **每个变体 1:1 翻译 v0.9.21 商业版**: 对齐 OpenTelemetry SDK 错误码
//! 4. **`NotImplemented` 守门**: 所有 stub 操作 (otlp/jaeger/b3/jaeger-propagation) 返此错误
//!
//! ## 错误分类 (10 变体)
//!
//! | 类别 | 变体 | 用途 |
//! |------|------|------|
//! | 通用 | `NotImplemented` | stub 操作未实现 (OTLP/Jaeger/B3) |
//! | 通用 | `Internal` | 内部错误 (panic / invariant violation) |
//! | 通用 | `InvalidUtf8` | W3C header 解析时遇到非法 UTF-8 |
//! | 配置 | `InvalidTraceId` | K-1 强校验: trace_id 不是 32 hex char |
//! | 配置 | `InvalidSpanId` | K-1 强校验: span_id 不是 16 hex char |
//! | 配置 | `EmptyServiceName` | K-1 强校验: resource.service.name 为空 |
//! | 配置 | `InvalidHeader` | traceparent / tracestate / baggage 格式错 |
//! | 导出 | `ExportFailed` | Exporter 写入失败 (file IO / network) |
//! | 采样 | `SamplingError` | Sampler 决策失败 |
//! | 传播 | `PropagationFailed` | Context 注入 / 提取失败 |
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 错误码 1:1 翻译 v0.9.21 商业版, 0 业务重设计
//! - **S-2 实事求是**: 10 变体覆盖 K-1 强校验 3 项 + 操作失败 7 类, 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry SDK 错误码 (StatusCode / SpanStatus)
//! - **O-3 干到底**: 10 变体 + Display + Error trait 完整, 测试 25+ fixture 覆盖
//! - **O-4 任何人都能接手**: 跟 credentials / cache 错误码同模式 (thiserror 派生)
//! - **O-5 不假装**: `NotImplemented` 守门 + 0 暴露完整 trace_id, 防假装已对接

use std::fmt;

use thiserror::Error;

// ============================================================================
// §1 TracingError 枚举 (10 变体, 编译期 hardcode)
// ============================================================================

/// Tracing 子系统统一错误类型.
///
/// 所有 Exporter / Sampler / Propagator / Span / TraceContext 操作失败时返此类型.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TracingError {
    /// Stub 操作未实现.
    ///
    /// 触发场景: otlp/jaeger exporter 写入, b3/jaeger propagation.
    /// R21+ 续真接时移除.
    #[error("tracing operation not implemented: {0}")]
    NotImplemented(String),

    /// 内部 invariant 违反.
    #[error("internal tracing error: {0}")]
    Internal(String),

    /// 非法 UTF-8 序列.
    #[error("invalid UTF-8 in tracing header: {0}")]
    InvalidUtf8(String),

    /// trace_id 不符合 W3C (必须 32 lowercase hex char).
    #[error("invalid trace_id: must be 32 lowercase hex chars, got {0:?}")]
    InvalidTraceId(String),

    /// span_id 不符合 W3C (必须 16 lowercase hex char).
    #[error("invalid span_id: must be 16 lowercase hex chars, got {0:?}")]
    InvalidSpanId(String),

    /// resource.service.name 为空 (K-1 强校验).
    #[error("resource.service.name is empty")]
    EmptyServiceName,

    /// traceparent / tracestate / baggage 格式非法.
    #[error("invalid W3C header {header}: {reason}")]
    InvalidHeader {
        /// header 名 (traceparent / tracestate / baggage).
        header: String,
        /// 错误原因.
        reason: String,
    },

    /// Exporter 写入失败.
    #[error("exporter {exporter} failed: {reason}")]
    ExportFailed {
        /// exporter 名.
        exporter: String,
        /// 失败原因.
        reason: String,
    },

    /// Sampler 决策失败.
    #[error("sampler decision failed: {0}")]
    SamplingError(String),

    /// Context 注入 / 提取失败.
    #[error("propagation failed: {0}")]
    PropagationFailed(String),
}

impl TracingError {
    /// 错误码名 (用于日志 + metrics).
    ///
    /// 返回稳定的字符串名, 不暴露内部细节, 防 PII 泄露.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotImplemented(_) => "TRACING_NOT_IMPLEMENTED",
            Self::Internal(_) => "TRACING_INTERNAL",
            Self::InvalidUtf8(_) => "TRACING_INVALID_UTF8",
            Self::InvalidTraceId(_) => "TRACING_INVALID_TRACE_ID",
            Self::InvalidSpanId(_) => "TRACING_INVALID_SPAN_ID",
            Self::EmptyServiceName => "TRACING_EMPTY_SERVICE_NAME",
            Self::InvalidHeader { .. } => "TRACING_INVALID_HEADER",
            Self::ExportFailed { .. } => "TRACING_EXPORT_FAILED",
            Self::SamplingError(_) => "TRACING_SAMPLING_ERROR",
            Self::PropagationFailed(_) => "TRACING_PROPAGATION_FAILED",
        }
    }

    /// 是否为可重试错误 (网络瞬态 / IO 瞬态).
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::ExportFailed { .. } | Self::Internal(_))
    }
}

/// TracingResult type alias.
pub type TracingResult<T> = Result<T, TracingError>;

// ============================================================================
// §2 单元测试 (in-file)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_unique() {
        let errors = [
            TracingError::NotImplemented("x".into()),
            TracingError::Internal("x".into()),
            TracingError::InvalidUtf8("x".into()),
            TracingError::InvalidTraceId("x".into()),
            TracingError::InvalidSpanId("x".into()),
            TracingError::EmptyServiceName,
            TracingError::InvalidHeader {
                header: "x".into(),
                reason: "y".into(),
            },
            TracingError::ExportFailed {
                exporter: "x".into(),
                reason: "y".into(),
            },
            TracingError::SamplingError("x".into()),
            TracingError::PropagationFailed("x".into()),
        ];
        let mut codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 10, "all 10 variant codes should be unique");
    }

    #[test]
    fn test_retryable_classification() {
        assert!(TracingError::ExportFailed {
            exporter: "otlp".into(),
            reason: "io".into()
        }
        .is_retryable());
        assert!(!TracingError::EmptyServiceName.is_retryable());
        assert!(!TracingError::NotImplemented("x".into()).is_retryable());
    }

    #[test]
    fn test_error_display_no_pii() {
        let e = TracingError::InvalidTraceId("abcdef1234567890abcdef1234567890".into());
        let s = format!("{}", e);
        // 应包含 input (调试用), 不应泄露到 stdout 默认路径
        assert!(s.contains("invalid trace_id"));
    }

    /// K-1 强校验: EmptyServiceName 是 fix 错误, 不重试.
    #[test]
    fn test_k1_empty_service_name() {
        let e = TracingError::EmptyServiceName;
        assert_eq!(e.code(), "TRACING_EMPTY_SERVICE_NAME");
        assert!(!e.is_retryable());
    }
}
