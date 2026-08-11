//! # tracing 集成 (per-request trace_id + span_id 注入)
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main` observability 集成 (per blueprint §2.5.3).
//! 商业版用 `@opentelemetry/api` 注入, 我们 skeleton 阶段用 std `tracing` 0.1 + 自家 `TraceId` / `SpanId`.
//!
//! R20 阶段 3 续 OpenTelemetry SDK 时, 把 `tracing_opentelemetry` 桥接即可, 公开 API 兼容.
//!
//! ## 设计要点
//!
//! - `next_trace_id()`: 每次新请求开 trace, 16 字节随机 → 32 hex (W3C Trace Context 兼容)
//! - `trace_span(name)`: 创顶层 span, 自动注入 trace_id/span_id 到 tracing span fields
//! - `TraceContext`: 携带 trace_id + 当前 span_id, async fn 间传递 (per-request)

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{ObservabilityError, ObservabilityResult, SpanContext, SpanId, TraceId};
use tracing::{info, instrument};

/// Trace 上下文 (携带 trace_id + 当前 span_id, async fn 间传递).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    /// 整条 trace 的 trace_id (跨多个 span 一致)
    pub trace_id: TraceId,
    /// 当前活跃 span_id
    pub span_id: SpanId,
    /// Span 名
    pub span_name: String,
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "trace_id={} span_id={} name={}",
            self.trace_id, self.span_id, self.span_name
        )
    }
}

impl TraceContext {
    /// 新建根 trace context (无父).
    #[must_use]
    pub fn new_root(span_name: impl Into<String>) -> Self {
        let trace_id = TraceId::new();
        let span_id = SpanId::new();
        Self {
            trace_id,
            span_id,
            span_name: span_name.into(),
        }
    }

    /// 新建子 trace context (继承 trace_id, 新 span_id).
    #[must_use]
    pub fn new_child(span_name: impl Into<String>, parent: &Self) -> Self {
        Self {
            trace_id: parent.trace_id.clone(),
            span_id: SpanId::new(),
            span_name: span_name.into(),
        }
    }

    /// 转 SpanContext (含 parent_span_id, 顶层时 None).
    #[must_use]
    pub fn to_span_context(&self, parent_span_id: Option<SpanId>) -> SpanContext {
        SpanContext {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            name: self.span_name.clone(),
            parent_span_id,
            start_time: chrono::Utc::now(),
        }
    }
}

/// 生成下一个 trace_id (新请求入口调用, 16 字节随机 / 32 hex).
///
/// 1:1 翻译 OpenTelemetry `trace.getTraceId()`, skeleton 阶段不引 OpenTelemetry SDK.
#[must_use]
pub fn next_trace_id() -> TraceId {
    TraceId::new()
}

/// 创建顶层 trace span (自动注入 trace_id/span_id 到 tracing span).
///
/// 1:1 翻译 OpenTelemetry `tracer.startSpan(name)`, skeleton 阶段用 `tracing::info_span!`.
///
/// # Examples
///
/// ```
/// use apeireth_telemetry::observability::tracing_integration::trace_span;
/// let _span = trace_span("http_request");
/// // span 退出时自动 record 结束时间
/// ```
#[instrument(name = "observability.trace_span", skip_all)]
pub fn trace_span(name: &str) -> TraceContext {
    let ctx = TraceContext::new_root(name);
    info!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        name = %ctx.span_name,
        "tracing: span started"
    );
    ctx
}

/// 记录 span 事件 (log message + trace 上下文).
///
/// 1:1 翻译 OpenTelemetry `span.addEvent(name, attributes)`.
#[instrument(name = "observability.span_event", skip_all, fields(trace_id, span_id))]
pub fn span_event(ctx: &TraceContext, event_name: &str, attributes: Option<&str>) {
    info!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        event = %event_name,
        attributes = attributes.unwrap_or(""),
        "tracing: span event"
    );
}

/// 结束 trace span (记录结束时间 + status).
#[instrument(name = "observability.span_end", skip_all, fields(trace_id, span_id))]
pub fn span_end(ctx: &TraceContext, status: &str) {
    info!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        status = %status,
        "tracing: span ended"
    );
}

/// 从 W3C `traceparent` header 解析 trace_id + span_id (32/16 hex).
///
/// R20 阶段 3 续 OpenTelemetry SDK 时, 这函数就是 OTLP HTTP/gRPC `traceparent` 解析的入口.
pub fn parse_traceparent(header: &str) -> ObservabilityResult<TraceContext> {
    // W3C 格式: `00-<trace_id 32 hex>-<span_id 16 hex>-<flags 2 hex>`
    // 例: `00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01`
    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() != 4 {
        return Err(ObservabilityError::TracingContextMissing(format!(
            "traceparent must have 4 parts, got {}",
            parts.len()
        )));
    }
    let trace_id = TraceId::from_hex(parts[1])?;
    let span_id = SpanId::from_hex(parts[2])?;
    Ok(TraceContext {
        trace_id,
        span_id,
        span_name: "w3c_traceparent".to_string(),
    })
}

/// 序列化为 W3C `traceparent` header (32/16 hex + flags `01`).
#[must_use]
pub fn render_traceparent(ctx: &TraceContext) -> String {
    format!("00-{}-{}-01", ctx.trace_id, ctx.span_id)
}

// ============================================================================
// 单元测试 (in-module)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_trace_id_returns_32_hex() {
        let id = next_trace_id();
        assert_eq!(id.as_hex().len(), 32);
    }

    #[test]
    fn trace_span_returns_context() {
        let ctx = trace_span("test_span");
        assert_eq!(ctx.span_name, "test_span");
        assert_eq!(ctx.trace_id.as_hex().len(), 32);
        assert_eq!(ctx.span_id.as_hex().len(), 16);
    }

    #[test]
    fn trace_context_root_and_child_inherit_trace_id() {
        let root = TraceContext::new_root("http_request");
        let child = TraceContext::new_child("db_query", &root);
        assert_eq!(root.trace_id, child.trace_id);
        assert_ne!(root.span_id, child.span_id);
    }

    #[test]
    fn parse_traceparent_w3c_format() {
        let header = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = parse_traceparent(header).expect("parse ok");
        assert_eq!(ctx.trace_id.as_hex(), "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.span_id.as_hex(), "b7ad6b7169203331");
    }

    #[test]
    fn parse_traceparent_invalid_format() {
        let err = parse_traceparent("garbage").unwrap_err();
        assert!(matches!(err, ObservabilityError::TracingContextMissing(_)));
    }

    #[test]
    fn render_traceparent_roundtrip() {
        let ctx = TraceContext::new_root("test");
        let header = render_traceparent(&ctx);
        let parsed = parse_traceparent(&header).expect("parse ok");
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.span_id, ctx.span_id);
    }

    #[test]
    fn span_event_does_not_panic() {
        let ctx = TraceContext::new_root("test");
        span_event(&ctx, "db.query.start", Some("table=users"));
        span_event(&ctx, "db.query.end", None);
    }

    #[test]
    fn span_end_does_not_panic() {
        let ctx = TraceContext::new_root("test");
        span_end(&ctx, "ok");
    }
}
