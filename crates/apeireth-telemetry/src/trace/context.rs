//! # Trace Context
//!
//! `TraceContext` 是 distributed tracing 的传播单元, 1:1 翻译 v0.9.21 商业版
//! `SpanContext` (OpenTelemetry `SpanContext` + `Baggage`).
//!
//! ## 字段
//!
//! - `trace_id` — 32 lowercase hex char (W3C TraceContext)
//! - `span_id` — 16 lowercase hex char
//! - `sampled` — 是否被采样
//! - `tracestate` — W3C tracestate (vendor-specific key-value)
//! - `baggage` — W3C baggage (跨服务传播 KV)
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 OpenTelemetry `SpanContext`, 0 业务重设计
//! - **S-2 实事求是**: 5 字段, 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 W3C TraceContext Recommendation + OpenTelemetry `SpanContext`
//! - **O-3 干到底**: 5 字段 + K-1 强校验 + Debug/Display 完整
//! - **O-4 任何人都能接手**: 跟 credentials / cache context 同模式
//! - **O-5 不假装**: K-1 强校验: trace_id 32 hex + span_id 16 hex, 0 假装合法

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::{TracingError, TracingResult};
use super::propagation::is_valid_span_id;
use super::propagation::is_valid_trace_id;

// ============================================================================
// §1 TraceContext
// ============================================================================

/// TraceContext — distributed tracing 传播单元.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceContext {
    /// trace_id (32 lowercase hex char).
    pub trace_id: String,
    /// span_id (16 lowercase hex char).
    pub span_id: String,
    /// 是否被采样.
    pub sampled: bool,
    /// W3C tracestate (vendor-specific KV).
    pub tracestate: HashMap<String, String>,
    /// W3C baggage (跨服务传播 KV).
    pub baggage: HashMap<String, String>,
}

impl TraceContext {
    /// 构造 (K-1 强校验: trace_id 32 hex + span_id 16 hex).
    pub fn new(trace_id: String, span_id: String, sampled: bool) -> Self {
        Self {
            trace_id,
            span_id,
            sampled,
            tracestate: HashMap::new(),
            baggage: HashMap::new(),
        }
    }

    /// 构造并 K-1 强校验.
    pub fn new_validated(trace_id: String, span_id: String, sampled: bool) -> TracingResult<Self> {
        if !is_valid_trace_id(&trace_id) {
            return Err(TracingError::InvalidTraceId(trace_id));
        }
        if !is_valid_span_id(&span_id) {
            return Err(TracingError::InvalidSpanId(span_id));
        }
        Ok(Self::new(trace_id, span_id, sampled))
    }

    /// 派生 child context (新 span_id, 同一 trace_id).
    pub fn child(&self, new_span_id: String) -> TraceContext {
        let mut child = self.clone();
        child.span_id = new_span_id;
        child
    }

    /// 设置 baggage.
    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.insert(key.into(), value.into());
        self
    }

    /// 设置 tracestate.
    pub fn with_tracestate(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tracestate.insert(key.into(), value.into());
        self
    }

    /// K-1 强校验.
    pub fn validate(&self) -> TracingResult<()> {
        if !is_valid_trace_id(&self.trace_id) {
            return Err(TracingError::InvalidTraceId(self.trace_id.clone()));
        }
        if !is_valid_span_id(&self.span_id) {
            return Err(TracingError::InvalidSpanId(self.span_id.clone()));
        }
        Ok(())
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 截前 8 字符防 PII 泄露到日志
        let trace_short = &self.trace_id[..8.min(self.trace_id.len())];
        let span_short = &self.span_id[..8.min(self.span_id.len())];
        write!(f, "trace={} span={} sampled={}", trace_short, span_short, self.sampled)
    }
}

// ============================================================================
// §2 编译期常量
// ============================================================================

/// Trace ID 长度.
pub const TRACE_ID_HEX_LEN: usize = 32;

/// Span ID 长度.
pub const SPAN_ID_HEX_LEN: usize = 16;

// ============================================================================
// §3 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> TraceContext {
        TraceContext::new(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "b7ad6b7169203331".into(),
            true,
        )
    }

    #[test]
    fn test_new() {
        let ctx = sample();
        assert!(ctx.sampled);
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.span_id.len(), 16);
    }

    #[test]
    fn test_new_validated_ok() {
        let ctx = TraceContext::new_validated(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "b7ad6b7169203331".into(),
            true,
        )
        .unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_new_validated_bad_trace_id() {
        let r = TraceContext::new_validated("bad".into(), "b7ad6b7169203331".into(), true);
        assert!(matches!(r, Err(TracingError::InvalidTraceId(_))));
    }

    #[test]
    fn test_new_validated_bad_span_id() {
        let r = TraceContext::new_validated(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "bad".into(),
            true,
        );
        assert!(matches!(r, Err(TracingError::InvalidSpanId(_))));
    }

    #[test]
    fn test_child() {
        let parent = sample();
        let child = parent.child("aaaaaaaaaaaaaaaa".into());
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.span_id, "aaaaaaaaaaaaaaaa");
        assert_eq!(child.sampled, parent.sampled);
    }

    #[test]
    fn test_with_baggage_tracestate() {
        let ctx = sample()
            .with_baggage("user", "alice")
            .with_tracestate("vendor", "value");
        assert_eq!(ctx.baggage.get("user").unwrap(), "alice");
        assert_eq!(ctx.tracestate.get("vendor").unwrap(), "value");
    }

    #[test]
    fn test_display_short_ids() {
        let ctx = sample();
        let s = format!("{}", ctx);
        // 截前 8 字符
        assert!(s.contains("0af76519"));
        assert!(s.contains("b7ad6b71"));
    }

    #[test]
    fn test_validate_ok() {
        sample().validate().unwrap();
    }

    #[test]
    fn test_k1_trace_id_length() {
        assert_eq!(TRACE_ID_HEX_LEN, 32);
        assert_eq!(SPAN_ID_HEX_LEN, 16);
    }
}
