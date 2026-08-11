//! # 集成测试: tracing in-process 行为 (25+ 测试)
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main/chunks/tracing` 测试, 验证 4 SpanKind +
//! 4 SpanEventKind + 4 Exporter + 4 Sampler + 3 Propagator + W3C TraceContext
//! 解析 + K-1 强校验 in-process 行为.
//!
//! ## 测试组织 (per task spec §8)
//!
//! - §1 编译期常量 (2 测试)
//! - §2 4 SpanKind (1 测试)
//! - §3 4 SpanEventKind (1 测试)
//! - §4 W3C traceparent / tracestate / baggage 格式 (8 测试)
//! - §5 4 Sampler (5 测试)
//! - §6 4 Exporter (5 测试)
//! - §7 3 Propagator (4 测试)
//! - §8 K-1 强校验 (3 测试)
//! - §9 Span 操作 (4 测试)
//! - §10 跨服务 context 传播 (1 测试)
//!
//! - **总计: 30+ 测试**

use apeireth_tracing::{
    context::TraceContext,
    error::TracingError,
    exporter::{ExporterKind, FileExporter, JaegerExporter, OtlpGrpcExporter, SpanExporter, StdoutExporter},
    inject_context, extract_context,
    parse_w3c_traceparent, parse_traceparent,
    propagation::{is_valid_span_id, is_valid_trace_id, parse_kv_list, B3Propagator, JaegerPropagator, Propagator, PropagatorKind, W3CTraceContextPropagator},
    sampler::{AlwaysOffSampler, AlwaysOnSampler, ParentBasedSampler, Sampler, SamplerKind, TraceIdRatioBasedSampler},
    span::{Span, SpanEvent, SpanEventKind, SpanKind, SpanStatus},
    trace::Trace,
    validate_service_name, validate_span_id, validate_trace_id,
    ExporterConfig, ResourceConfig, SamplerConfig, ServiceConfig, TracingConfig,
    K1_STRONG_VALIDATION_COUNT, TRACING_CONFIG_SECTION_COUNT, TRACING_ERROR_VARIANT_COUNT,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::NamedTempFile;

// ============================================================================
// §1 编译期常量
// ============================================================================

#[test]
fn test_constants() {
    assert_eq!(TRACING_CONFIG_SECTION_COUNT, 4);
    assert_eq!(TRACING_ERROR_VARIANT_COUNT, 10);
    assert_eq!(K1_STRONG_VALIDATION_COUNT, 3);
}

#[test]
fn test_re_exports() {
    // 验证 4+4+4+3+10 全部 re-export
    let _k1 = SpanKind::Client;
    let _k2 = SpanKind::Server;
    let _k3 = SpanKind::Producer;
    let _k4 = SpanKind::Consumer;
    let _e1 = SpanEventKind::Log;
    let _e2 = SpanEventKind::Exception;
    let _e3 = SpanEventKind::Event;
    let _e4 = SpanEventKind::Message;
    let _x1 = ExporterKind::Stdout;
    let _x2 = ExporterKind::File;
    let _x3 = ExporterKind::OtlpGrpc;
    let _x4 = ExporterKind::Jaeger;
    let _s1 = SamplerKind::AlwaysOn;
    let _s2 = SamplerKind::AlwaysOff;
    let _s3 = SamplerKind::TraceIdRatioBased;
    let _s4 = SamplerKind::ParentBased;
    let _p1 = PropagatorKind::W3CTraceContext;
    let _p2 = PropagatorKind::B3;
    let _p3 = PropagatorKind::Jaeger;
}

// ============================================================================
// §2 4 SpanKind
// ============================================================================

#[test]
fn test_span_4_kinds() {
    assert_eq!(SpanKind::Client.as_str(), "client");
    assert_eq!(SpanKind::Server.as_str(), "server");
    assert_eq!(SpanKind::Producer.as_str(), "producer");
    assert_eq!(SpanKind::Consumer.as_str(), "consumer");
}

// ============================================================================
// §3 4 SpanEventKind
// ============================================================================

#[test]
fn test_event_4_kinds() {
    let e1 = SpanEvent::log("test");
    assert_eq!(e1.kind, SpanEventKind::Log);
    let e2 = SpanEvent::exception("io", "at foo");
    assert_eq!(e2.kind, SpanEventKind::Exception);
    let e3 = SpanEvent::event("checkpoint", HashMap::new());
    assert_eq!(e3.kind, SpanEventKind::Event);
    let e4 = SpanEvent::message("orders", "{}");
    assert_eq!(e4.kind, SpanEventKind::Message);
}

// ============================================================================
// §4 W3C traceparent / tracestate / baggage 格式
// ============================================================================

#[test]
fn test_w3c_traceparent_format() {
    let s = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let ctx = parse_traceparent(s).unwrap();
    assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    assert_eq!(ctx.span_id, "b7ad6b7169203331");
    assert!(ctx.sampled);
}

#[test]
fn test_w3c_traceparent_parse() {
    let s = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let ctx = parse_w3c_traceparent(s).unwrap();
    assert!(ctx.sampled);

    let s2 = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00";
    let ctx2 = parse_w3c_traceparent(s2).unwrap();
    assert!(!ctx2.sampled);
}

#[test]
fn test_w3c_tracestate_format() {
    let s = "vendor1=value1,vendor2=value2";
    let m = parse_kv_list(s);
    assert_eq!(m.len(), 2);
    assert_eq!(m.get("vendor1").unwrap(), "value1");
    assert_eq!(m.get("vendor2").unwrap(), "value2");
}

#[test]
fn test_w3c_baggage_format() {
    let s = "userId=alice,region=us-east-1";
    let m = parse_kv_list(s);
    assert_eq!(m.get("userId").unwrap(), "alice");
    assert_eq!(m.get("region").unwrap(), "us-east-1");
}

#[test]
fn test_w3c_traceparent_inject_extract() {
    let p = W3CTraceContextPropagator;
    let original = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let mut carrier = HashMap::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        p.inject(&original, &mut carrier).await;
    });
    let extracted = rt.block_on(async { p.extract(&carrier).await.unwrap() });
    assert_eq!(extracted.trace_id, original.trace_id);
    assert_eq!(extracted.span_id, original.span_id);
}

#[test]
fn test_w3c_traceparent_with_tracestate_baggage() {
    let p = W3CTraceContextPropagator;
    let mut ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    ctx.tracestate.insert("vendor".into(), "v1".into());
    ctx.baggage.insert("user".into(), "alice".into());
    let mut carrier = HashMap::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        p.inject(&ctx, &mut carrier).await;
    });
    assert!(carrier.contains_key("tracestate"));
    assert!(carrier.contains_key("baggage"));
    let extracted = rt.block_on(async { p.extract(&carrier).await.unwrap() });
    assert_eq!(extracted.tracestate.get("vendor").unwrap(), "v1");
    assert_eq!(extracted.baggage.get("user").unwrap(), "alice");
}

#[test]
fn test_w3c_traceparent_invalid_version() {
    let r = parse_traceparent("ff-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
    assert!(matches!(r, Err(TracingError::InvalidHeader { .. })));
}

#[test]
fn test_w3c_traceparent_invalid_format() {
    let r = parse_traceparent("not-a-valid-header");
    assert!(r.is_err());
}

// ============================================================================
// §5 4 Sampler
// ============================================================================

#[test]
fn test_sampler_4_kinds() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        assert!(AlwaysOnSampler.should_sample("0af7651916cd43dd8448eb211c80319c", None).await);
        assert!(!AlwaysOffSampler.should_sample("0af7651916cd43dd8448eb211c80319c", None).await);
        let ratio = TraceIdRatioBasedSampler::new(0.5).unwrap();
        let _ = ratio.should_sample("0af7651916cd43dd8448eb211c80319c", None).await;
        let parent = ParentBasedSampler::new();
        let _ = parent.should_sample("0af7651916cd43dd8448eb211c80319c", Some(true)).await;
    });
}

#[test]
fn test_sampler_always_on_kind() {
    let s = AlwaysOnSampler;
    assert_eq!(s.kind(), SamplerKind::AlwaysOn);
}

#[test]
fn test_sampler_always_off_kind() {
    let s = AlwaysOffSampler;
    assert_eq!(s.kind(), SamplerKind::AlwaysOff);
}

#[test]
fn test_sampler_ratio_10_percent() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let s = TraceIdRatioBasedSampler::new(0.1).unwrap();
        let mut sampled = 0;
        for _ in 0..10_000u32 {
            // 用 UUIDv4 派生 32 hex, 末 8 char 均匀分布
            let u1 = uuid::Uuid::new_v4();
            let u2 = uuid::Uuid::new_v4();
            let mut id = format!("{}{}", u1.simple(), u2.simple());
            id.truncate(32);
            if s.should_sample(&id, None).await {
                sampled += 1;
            }
        }
        let rate = sampled as f64 / 10_000.0;
        assert!((0.05..=0.15).contains(&rate), "10% sampler rate={}", rate);
    });
}

#[test]
fn test_sampler_parent_based_fallback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 无父 + 默认 root (AlwaysOn) = true
        let s = ParentBasedSampler::new();
        assert!(s.should_sample("0af7651916cd43dd8448eb211c80319c", None).await);
        // 父决策 false = 子也 false
        assert!(!s.should_sample("0af7651916cd43dd8448eb211c80319c", Some(false)).await);
    });
}

// ============================================================================
// §6 4 Exporter
// ============================================================================

#[tokio::test]
async fn test_exporter_4_kinds() {
    // 4 ExporterKind 枚举值
    let kinds = [
        ExporterKind::Stdout,
        ExporterKind::File,
        ExporterKind::OtlpGrpc,
        ExporterKind::Jaeger,
    ];
    assert_eq!(kinds.len(), 4);
    for k in &kinds {
        assert!(!k.as_str().is_empty());
    }
}

#[tokio::test]
async fn test_stdout_exporter_write() {
    let e = StdoutExporter;
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let span = Span::new("test", SpanKind::Client, ctx).unwrap();
    let r = e.export(&span).await;
    assert!(r.is_ok());
    e.flush().await.unwrap();
    e.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_file_exporter_write() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    let e = FileExporter::new(path.clone());
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let span = Span::new("test.write", SpanKind::Client, ctx).unwrap();
    e.export(&span).await.unwrap();
    e.flush().await.unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("test.write"));
    assert!(content.contains("trace_id"));
}

#[tokio::test]
async fn test_otlp_exporter_not_implemented() {
    let e = OtlpGrpcExporter::new("http://collector:4317");
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let span = Span::new("test", SpanKind::Client, ctx).unwrap();
    let r = e.export(&span).await;
    assert!(matches!(r, Err(TracingError::NotImplemented(_))));
}

#[tokio::test]
async fn test_jaeger_exporter_not_implemented() {
    let e = JaegerExporter::new("http://jaeger:14268");
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let span = Span::new("test", SpanKind::Client, ctx).unwrap();
    let r = e.export(&span).await;
    assert!(matches!(r, Err(TracingError::NotImplemented(_))));
}

// ============================================================================
// §7 3 Propagator
// ============================================================================

#[test]
fn test_propagation_3_kinds() {
    let kinds = [
        PropagatorKind::W3CTraceContext,
        PropagatorKind::B3,
        PropagatorKind::Jaeger,
    ];
    assert_eq!(kinds.len(), 3);
    for k in &kinds {
        assert!(!k.as_str().is_empty());
    }
}

#[test]
fn test_b3_propagation_not_implemented() {
    let p = B3Propagator;
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut carrier = HashMap::new();
        p.inject(&ctx, &mut carrier).await;
        // B3 stub 不注入任何 header
        assert!(carrier.is_empty());
        let r = p.extract(&carrier).await;
        assert!(r.is_none());
    });
}

#[test]
fn test_jaeger_propagation_not_implemented() {
    let p = JaegerPropagator;
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut carrier = HashMap::new();
        p.inject(&ctx, &mut carrier).await;
        assert!(carrier.is_empty());
        let r = p.extract(&carrier).await;
        assert!(r.is_none());
    });
}

#[test]
fn test_w3c_propagation_roundtrip() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let original = TraceContext::new(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "b7ad6b7169203331".into(),
            true,
        );
        let mut carrier = HashMap::new();
        inject_context(&original, &mut carrier).await;
        let extracted = extract_context(&carrier).await.unwrap();
        assert_eq!(extracted.trace_id, original.trace_id);
        assert_eq!(extracted.span_id, original.span_id);
    });
}

// ============================================================================
// §8 K-1 强校验
// ============================================================================

#[test]
fn test_k1_trace_id_invalid() {
    // 太短
    assert!(validate_trace_id("0af76519").is_err());
    // 大写 (W3C 要求 lowercase)
    assert!(validate_trace_id("0AF7651916CD43DD8448EB211C80319C").is_err());
    // 非 hex
    assert!(validate_trace_id("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    // 32 字符但非 hex
    assert!(validate_trace_id("0000000000000000000000000000000g").is_err());
    // 合法
    assert!(validate_trace_id("0af7651916cd43dd8448eb211c80319c").is_ok());
    // 全 0 合法
    assert!(validate_trace_id("00000000000000000000000000000000").is_ok());
    assert!(!is_valid_trace_id("0af7651916cd43dd8448eb211c80319cXX"));
}

#[test]
fn test_k1_span_id_invalid() {
    assert!(validate_span_id("b7ad6b716920333").is_err()); // 太短
    assert!(validate_span_id("B7AD6B7169203331").is_err()); // 大写
    assert!(validate_span_id("zzzzzzzzzzzzzzzz").is_err()); // 非 hex
    assert!(validate_span_id("b7ad6b7169203331").is_ok());
    assert!(validate_span_id("0000000000000000").is_ok());
}

#[test]
fn test_k1_service_name_empty() {
    assert!(validate_service_name("").is_err());
    assert!(validate_service_name("   ").is_err());
    assert!(validate_service_name("\t").is_err());
    assert!(validate_service_name("apeireth-api").is_ok());
    // TracingConfig 也会拒
    let cfg = TracingConfig {
        service: ServiceConfig {
            name: "".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(cfg.validate().is_err());
}

// ============================================================================
// §9 Span 操作
// ============================================================================

#[test]
fn test_span_attribute_set_get() {
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let s = Span::new("db.query", SpanKind::Client, ctx)
        .unwrap()
        .set_attribute("db.statement", "SELECT 1")
        .set_attribute("db.system", "postgresql");
    assert_eq!(s.attributes.get("db.statement").unwrap(), "SELECT 1");
    assert_eq!(s.attributes.get("db.system").unwrap(), "postgresql");
}

#[test]
fn test_span_event_log() {
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let mut s = Span::new("op", SpanKind::Client, ctx).unwrap();
    s.log("starting query");
    assert_eq!(s.events.len(), 1);
    assert_eq!(s.events[0].kind, SpanEventKind::Log);
    assert_eq!(
        s.events[0].attributes.get("message").unwrap(),
        "starting query"
    );
}

#[test]
fn test_span_event_exception() {
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let mut s = Span::new("op", SpanKind::Client, ctx).unwrap();
    s.exception("io timeout", "at foo() at bar()");
    assert_eq!(s.events[0].kind, SpanEventKind::Exception);
    assert_eq!(s.events[0].attributes.get("error").unwrap(), "io timeout");
    assert!(s.events[0].attributes.contains_key("stack"));
}

#[test]
fn test_span_status_ok_error() {
    let ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "b7ad6b7169203331".into(),
        true,
    );
    let mut s = Span::new("op", SpanKind::Client, ctx).unwrap();
    assert_eq!(s.status, SpanStatus::Unset);
    s.set_ok();
    assert_eq!(s.status, SpanStatus::Ok);
    s.set_error("oops");
    assert!(matches!(s.status, SpanStatus::Error { .. }));
}

// ============================================================================
// §10 跨服务 context 传播
// ============================================================================

#[tokio::test]
async fn test_context_propagation_through_channel() {
    // 模拟: 服务 A 注入 context → 服务 B 提取
    let service_a_ctx = TraceContext::new(
        "0af7651916cd43dd8448eb211c80319c".into(),
        "aaaaaaaaaaaaaaaa".into(),
        true,
    );

    // 服务 A: 注入到 header
    let mut headers = HashMap::new();
    inject_context(&service_a_ctx, &mut headers).await;
    let tp = headers.get("traceparent").unwrap();
    assert!(tp.starts_with("00-0af7651916cd43dd8448eb211c80319c"));

    // 服务 B: 提取 + 派生 child span
    let extracted = extract_context(&headers).await.unwrap();
    assert_eq!(extracted.trace_id, service_a_ctx.trace_id);
    let child = extracted.child("bbbbbbbbbbbbbbbb".into());
    assert_eq!(child.trace_id, service_a_ctx.trace_id);
    assert_eq!(child.span_id, "bbbbbbbbbbbbbbbb");
}
