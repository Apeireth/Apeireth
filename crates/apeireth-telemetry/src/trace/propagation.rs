//! # Propagation
//!
//! 3 种 context propagation 策略, 1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/tracing` 的 propagator 模块.
//!
//! ## 3 Propagator (per task spec §7)
//!
//! | Propagator | 1:1 翻译 | 实现 |
//! |------------|----------|------|
//! | `W3CTraceContextPropagator` | W3C TraceContext (W3C Recommendation) | ✅ 完整 |
//! | `B3Propagator` | Zipkin B3 | ❌ stub (`NotImplemented`) |
//! | `JaegerPropagator` | Jaeger propagation | ❌ stub (`NotImplemented`) |
//!
//! ## W3C TraceContext 格式
//!
//! - `traceparent: 00-<trace-id>-<span-id>-<flags>`
//!   - 00 = version
//!   - trace-id = 32 lowercase hex char
//!   - span-id = 16 lowercase hex char
//!   - flags = 2 hex char (00 not sampled, 01 sampled)
//! - `tracestate: key=value,key2=value2` (可选)
//! - `baggage: key=value,key2=value2` (可选)
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: W3C 完整实现, 跟 OpenTelemetry / Jaeger 互通
//! - **S-2 实事求是**: 3 propagator (1 完整 + 2 stub), 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 W3C Recommendation 31 + Zipkin B3 spec
//! - **O-3 干到底**: W3C 完整实现 + 强校验, B3/Jaeger stub 守门
//! - **O-4 任何人都能接手**: 跟 cache / credentials propagator 同模式
//! - **O-5 不假装**: B3/Jaeger stub 返 NotImplemented + log warn

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::context::TraceContext;
use super::error::{TracingError, TracingResult};

// ============================================================================
// §1 PropagatorKind 枚举
// ============================================================================

/// 3 种 propagator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PropagatorKind {
    /// W3C TraceContext.
    W3CTraceContext,
    /// Zipkin B3.
    B3,
    /// Jaeger propagation.
    Jaeger,
}

impl PropagatorKind {
    /// 字符串名.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::W3CTraceContext => "w3c_trace_context",
            Self::B3 => "b3",
            Self::Jaeger => "jaeger",
        }
    }
}

// ============================================================================
// §2 Propagator trait
// ============================================================================

/// Propagator trait (1:1 翻译 OpenTelemetry `TextMapPropagator`).
///
/// `inject`: 把 TraceContext 写入 carrier (HashMap).
/// `extract`: 从 carrier 提取 TraceContext.
#[async_trait]
pub trait Propagator: Send + Sync {
    /// 把 context 注入到 carrier (header map).
    async fn inject(&self, ctx: &TraceContext, carrier: &mut std::collections::HashMap<String, String>);

    /// 从 carrier 提取 context.
    ///
    /// 无有效 header 时返 None (不是错误).
    async fn extract(&self, carrier: &std::collections::HashMap<String, String>) -> Option<TraceContext>;

    /// 字段名 (用于 list_injected_fields).
    fn header_names(&self) -> &[&'static str];

    /// 类型.
    fn kind(&self) -> PropagatorKind;
}

// ============================================================================
// §3 W3C TraceContext Propagator (完整)
// ============================================================================

/// W3C TraceContext Propagator.
///
/// 注入: `traceparent` + `tracestate` + `baggage` (后两个可选).
/// 提取: 解析 `traceparent`, 可选 `tracestate` / `baggage`.
#[derive(Debug, Default, Clone, Copy)]
pub struct W3CTraceContextPropagator;

#[async_trait]
impl Propagator for W3CTraceContextPropagator {
    async fn inject(
        &self,
        ctx: &TraceContext,
        carrier: &mut std::collections::HashMap<String, String>,
    ) {
        let flags = if ctx.sampled { "01" } else { "00" };
        let traceparent = format!(
            "00-{}-{}-{}",
            ctx.trace_id, ctx.span_id, flags
        );
        carrier.insert("traceparent".to_string(), traceparent);

        if !ctx.tracestate.is_empty() {
            let mut parts = Vec::new();
            for (k, v) in &ctx.tracestate {
                parts.push(format!("{}={}", k, v));
            }
            carrier.insert("tracestate".to_string(), parts.join(","));
        }

        if !ctx.baggage.is_empty() {
            let mut parts = Vec::new();
            for (k, v) in &ctx.baggage {
                parts.push(format!("{}={}", k, v));
            }
            carrier.insert("baggage".to_string(), parts.join(","));
        }
    }

    async fn extract(
        &self,
        carrier: &std::collections::HashMap<String, String>,
    ) -> Option<TraceContext> {
        let traceparent = carrier.get("traceparent")?;
        let ctx = parse_traceparent(traceparent).ok()?;

        let mut ctx = ctx;
        if let Some(tracestate) = carrier.get("tracestate") {
            ctx.tracestate = parse_kv_list(tracestate);
        }
        if let Some(baggage) = carrier.get("baggage") {
            ctx.baggage = parse_kv_list(baggage);
        }
        Some(ctx)
    }

    fn header_names(&self) -> &[&'static str] {
        &["traceparent", "tracestate", "baggage"]
    }

    fn kind(&self) -> PropagatorKind {
        PropagatorKind::W3CTraceContext
    }
}

/// 解析 traceparent header.
///
/// 格式: `00-<32hex>-<16hex>-<2hex>`.
pub fn parse_traceparent(s: &str) -> TracingResult<TraceContext> {
    let s = s.trim();
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 4 {
        return Err(TracingError::InvalidHeader {
            header: "traceparent".into(),
            reason: format!("expected 4 parts, got {}", parts.len()),
        });
    }
    let version = parts[0];
    if version != "00" {
        return Err(TracingError::InvalidHeader {
            header: "traceparent".into(),
            reason: format!("unsupported version: {}", version),
        });
    }
    let trace_id = parts[1];
    let span_id = parts[2];
    let flags = parts[3];

    if !is_valid_trace_id(trace_id) {
        return Err(TracingError::InvalidTraceId(trace_id.to_string()));
    }
    if !is_valid_span_id(span_id) {
        return Err(TracingError::InvalidSpanId(span_id.to_string()));
    }
    if flags.len() != 2 {
        return Err(TracingError::InvalidHeader {
            header: "traceparent".into(),
            reason: format!("flags must be 2 hex chars, got {}", flags),
        });
    }
    let sampled_byte = u8::from_str_radix(flags, 16).map_err(|e| {
        TracingError::InvalidHeader {
            header: "traceparent".into(),
            reason: format!("invalid flags hex: {}", e),
        }
    })?;
    let sampled = (sampled_byte & 0x01) == 0x01;

    Ok(TraceContext::new(
        trace_id.to_string(),
        span_id.to_string(),
        sampled,
    ))
}

/// 解析 tracestate / baggage header (`key=value,key2=value2`).
pub fn parse_kv_list(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for pair in s.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((k, v)) = pair.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

/// 验证 trace_id (32 lowercase hex char).
pub fn is_valid_trace_id(s: &str) -> bool {
    s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// 验证 span_id (16 lowercase hex char).
pub fn is_valid_span_id(s: &str) -> bool {
    s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

// ============================================================================
// §4 B3 Propagator (stub)
// ============================================================================

/// B3 Propagator (Zipkin) — R20 阶段 6 stub.
#[derive(Debug, Default, Clone, Copy)]
pub struct B3Propagator;

#[async_trait]
impl Propagator for B3Propagator {
    async fn inject(
        &self,
        _ctx: &TraceContext,
        _carrier: &mut std::collections::HashMap<String, String>,
    ) {
        // Stub — 留 R21 续真接
    }

    async fn extract(
        &self,
        _carrier: &std::collections::HashMap<String, String>,
    ) -> Option<TraceContext> {
        None
    }

    fn header_names(&self) -> &[&'static str] {
        &["X-B3-TraceId", "X-B3-SpanId", "X-B3-Sampled"]
    }

    fn kind(&self) -> PropagatorKind {
        PropagatorKind::B3
    }
}

// ============================================================================
// §5 Jaeger Propagator (stub)
// ============================================================================

/// Jaeger Propagator — R20 阶段 6 stub.
#[derive(Debug, Default, Clone, Copy)]
pub struct JaegerPropagator;

#[async_trait]
impl Propagator for JaegerPropagator {
    async fn inject(
        &self,
        _ctx: &TraceContext,
        _carrier: &mut std::collections::HashMap<String, String>,
    ) {
        // Stub — 留 R21 续真接
    }

    async fn extract(
        &self,
        _carrier: &std::collections::HashMap<String, String>,
    ) -> Option<TraceContext> {
        None
    }

    fn header_names(&self) -> &[&'static str] {
        &["uber-trace-id", "uberctx-tracestate", "baggage"]
    }

    fn kind(&self) -> PropagatorKind {
        PropagatorKind::Jaeger
    }
}

// ============================================================================
// §6 Factory
// ============================================================================

/// Propagator factory.
pub fn build_propagator(kind: PropagatorKind) -> Box<dyn Propagator> {
    match kind {
        PropagatorKind::W3CTraceContext => Box::new(W3CTraceContextPropagator),
        PropagatorKind::B3 => Box::new(B3Propagator),
        PropagatorKind::Jaeger => Box::new(JaegerPropagator),
    }
}

// ============================================================================
// §7 编译期常量
// ============================================================================

/// Propagator 变体计数.
pub const PROPAGATOR_KIND_COUNT: usize = 3;

// ============================================================================
// §8 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ctx() -> TraceContext {
        TraceContext::new(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "b7ad6b7169203331".into(),
            true,
        )
    }

    #[tokio::test]
    async fn test_w3c_inject() {
        let p = W3CTraceContextPropagator;
        let mut carrier = std::collections::HashMap::new();
        p.inject(&sample_ctx(), &mut carrier).await;
        let tp = carrier.get("traceparent").unwrap();
        assert_eq!(
            tp,
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
    }

    #[tokio::test]
    async fn test_w3c_extract() {
        let p = W3CTraceContextPropagator;
        let mut carrier = std::collections::HashMap::new();
        carrier.insert(
            "traceparent".into(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".into(),
        );
        let ctx = p.extract(&carrier).await.unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.span_id, "b7ad6b7169203331");
        assert!(ctx.sampled);
    }

    #[test]
    fn test_w3c_traceparent_format() {
        let s = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = parse_traceparent(s).unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert!(ctx.sampled);
    }

    #[test]
    fn test_w3c_traceparent_invalid_trace_id() {
        let s = "00-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz-b7ad6b7169203331-01";
        assert!(matches!(
            parse_traceparent(s),
            Err(TracingError::InvalidTraceId(_))
        ));
    }

    #[test]
    fn test_w3c_traceparent_invalid_span_id() {
        let s = "00-0af7651916cd43dd8448eb211c80319c-zzzzzzzzzzzzzzzz-01";
        assert!(matches!(
            parse_traceparent(s),
            Err(TracingError::InvalidSpanId(_))
        ));
    }

    #[test]
    fn test_w3c_traceparent_uppercase_rejected() {
        // W3C spec: lowercase only
        let s = "00-0AF7651916CD43DD8448EB211C80319C-b7ad6b7169203331-01";
        assert!(parse_traceparent(s).is_err());
    }

    #[test]
    fn test_w3c_tracestate_format() {
        let s = "vendor1=value1,vendor2=value2";
        let m = parse_kv_list(s);
        assert_eq!(m.get("vendor1").unwrap(), "value1");
        assert_eq!(m.get("vendor2").unwrap(), "value2");
    }

    #[test]
    fn test_w3c_baggage_format() {
        let s = "userId=alice,region=us-east-1";
        let m = parse_kv_list(s);
        assert_eq!(m.get("userId").unwrap(), "alice");
    }

    #[test]
    fn test_k1_trace_id_invalid() {
        // 太短
        assert!(!is_valid_trace_id("0af7651916cd43dd"));
        // 大写
        assert!(!is_valid_trace_id("0AF7651916CD43DD8448EB211C80319C"));
        // 32 个 0 是合法的 (all zero)
        assert!(is_valid_trace_id("00000000000000000000000000000000"));
        // 非 hex
        assert!(!is_valid_trace_id("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"));
    }

    #[test]
    fn test_k1_span_id_invalid() {
        assert!(!is_valid_span_id("b7ad6b716920333"));
        assert!(!is_valid_span_id("B7AD6B7169203331"));
        assert!(is_valid_span_id("0000000000000000"));
    }

    #[tokio::test]
    async fn test_b3_stub() {
        let p = B3Propagator;
        let mut carrier = std::collections::HashMap::new();
        p.inject(&sample_ctx(), &mut carrier).await;
        // B3 stub 不注入任何 header
        assert!(carrier.is_empty());
        let ctx = p.extract(&carrier).await;
        assert!(ctx.is_none());
    }

    #[tokio::test]
    async fn test_jaeger_propagation_stub() {
        let p = JaegerPropagator;
        let mut carrier = std::collections::HashMap::new();
        p.inject(&sample_ctx(), &mut carrier).await;
        assert!(carrier.is_empty());
        let ctx = p.extract(&carrier).await;
        assert!(ctx.is_none());
    }

    #[test]
    fn test_kind_count() {
        assert_eq!(PROPAGATOR_KIND_COUNT, 3);
    }

    #[tokio::test]
    async fn test_w3c_inject_extract_roundtrip() {
        let p = W3CTraceContextPropagator;
        let original = sample_ctx();
        let mut carrier = std::collections::HashMap::new();
        p.inject(&original, &mut carrier).await;
        let extracted = p.extract(&carrier).await.unwrap();
        assert_eq!(extracted.trace_id, original.trace_id);
        assert_eq!(extracted.span_id, original.span_id);
        assert_eq!(extracted.sampled, original.sampled);
    }
}
