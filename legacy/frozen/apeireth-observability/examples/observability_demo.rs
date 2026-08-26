//! # observability_demo — 演示 trace + metrics + health + PII 脱敏 + 3 端点
//!
//! 1:1 翻译 v0.9.21 商业版 observability 集成, skeleton 阶段演示用.
//! R20 阶段 3 续 OpenTelemetry SDK 时, 公开 API 兼容, 例子只需改 import 即可.

use std::collections::HashMap;

use apeireth_observability::{
    health::health_check,
    health::HealthEndpoint,
    logging::{log_structured, LogLevel},
    metrics::MetricsRegistry,
    next_trace_id, redact_pii,
    tracing_integration::{parse_traceparent, render_traceparent, span_end, span_event, trace_span},
    HealthResponse, HealthStatus, MetricKind, MetricSample, ObservabilityBus, SpanContext,
    TraceContext, OBSERVABILITY_SCHEMA_VERSION, PLATFORM_NAME, PROMETHEUS_DEFAULT_PORT,
    OTLP_DEFAULT_ENDPOINT, TRACING_SAMPLE_RATE, LOG_FORMAT_JSON,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-observability demo (skeleton) ===\n");

    // 1. 平台信息
    println!("[1] 平台配置 (编译期 hardcode):");
    println!("    PLATFORM_NAME              = {PLATFORM_NAME}");
    println!("    OBSERVABILITY_SCHEMA_VERSION = {OBSERVABILITY_SCHEMA_VERSION}");
    println!("    PROMETHEUS_DEFAULT_PORT    = {PROMETHEUS_DEFAULT_PORT}");
    println!("    OTLP_DEFAULT_ENDPOINT      = {OTLP_DEFAULT_ENDPOINT}");
    println!("    TRACING_SAMPLE_RATE        = {TRACING_SAMPLE_RATE}");
    println!("    LOG_FORMAT_JSON            = {LOG_FORMAT_JSON}");
    println!();

    // 2. K-1 强校验
    println!("[2] K-1 强校验 4 条:");
    println!(
        "    #1 PLATFORM_NAME = \"apeireth\": {}",
        PLATFORM_NAME == "apeireth"
    );
    println!(
        "    #2 SUPPORTED_METRICS.len() = 3: {}",
        apeireth_observability::SUPPORTED_METRICS.len() == 3
    );
    println!(
        "    #3 TOOL_WHITELIST.len() = 8: {}",
        apeireth_observability::TOOL_WHITELIST.len() == 8
    );
    println!(
        "    #4 HEALTH_ENDPOINTS.len() = 3: {}",
        apeireth_observability::HEALTH_ENDPOINTS.len() == 3
    );
    println!();

    // 3. Trace 集成
    println!("[3] Trace 集成 (per-request trace_id + span_id):");
    let trace_id = next_trace_id();
    println!("    next_trace_id()      = {trace_id}");

    let root_ctx = trace_span("http_request");
    println!("    trace_span(\"http_request\") = {root_ctx}");

    // 子 span
    let db_ctx = TraceContext::new_child("db_query", &root_ctx);
    println!("    child db_query       = {db_ctx}");

    // W3C traceparent 序列化 / 解析
    let header = render_traceparent(&root_ctx);
    println!("    W3C traceparent      = {header}");
    let parsed = parse_traceparent(&header).expect("parse ok");
    println!("    parsed trace_id      = {}", parsed.trace_id);
    println!("    parsed span_id       = {}", parsed.span_id);
    span_event(&db_ctx, "db.query.start", Some("table=users"));
    span_end(&db_ctx, "ok");
    println!();

    // 4. Metrics 集成
    println!("[4] Metrics 集成 (Prometheus 文本格式):");
    let registry = MetricsRegistry::new();
    let mut labels = HashMap::new();
    labels.insert("endpoint".to_string(), "/api/v1".to_string());
    registry.counter("requests_total", 1.0, labels.clone()).await;
    registry.counter("requests_total", 2.0, labels).await;
    registry.gauge("active_connections", 7.0, HashMap::new()).await;
    let snapshot = registry.snapshot().await;
    println!("    counter requests_total = {}", snapshot.iter().find(|s| s.name == "requests_total").map(|s| s.value).unwrap_or(0.0));
    println!("    gauge   active_connections = {}", snapshot.iter().find(|s| s.name == "active_connections").map(|s| s.value).unwrap_or(0.0));
    println!();
    let text = apeireth_observability::metrics::render_prometheus(&snapshot);
    println!("    Prometheus 文本格式:");
    for line in text.lines() {
        println!("      {line}");
    }
    println!();

    // 5. PII 脱敏
    println!("[5] PII 脱敏 (PII_REDACTION_PATTERN 编译期 hardcode):");
    let samples_pii = [
        ("password=secret123", "password=***"),
        ("api_token=abc-def", "api_token=***"),
        ("secret=topsecret", "secret=***"),
        ("key=value", "key=***"),
    ];
    for (input, expected) in samples_pii {
        let redacted = redact_pii(input).unwrap_or_else(|| "NONE".to_string());
        let ok = redacted.contains(expected);
        println!(
            "    redact(\"{input}\") = \"{redacted}\"  (expected \"{expected}\")  [{}]",
            if ok { "✓" } else { "✗" }
        );
    }
    println!();

    // 6. Structured JSON logging
    println!("[6] Structured JSON logging (PII 自动脱敏):");
    let mut fields = HashMap::new();
    fields.insert("user_id".to_string(), serde_json::json!(42));
    fields.insert("endpoint".to_string(), serde_json::json!("/api/v1"));
    let entry = log_structured(
        LogLevel::Info,
        "apeireth_observability::demo",
        "request handled, password=secret123",
        Some(fields),
    );
    println!("    message (PII redacted): {}", entry.message);
    println!("    timestamp: {}", entry.timestamp);
    println!("    level:     {}", entry.level);
    println!();

    // 7. Health 3 端点
    println!("[7] Health 3 端点 (/health /ready /metrics):");
    for endpoint in [
        HealthEndpoint::Health,
        HealthEndpoint::Ready,
        HealthEndpoint::Metrics,
    ] {
        let resp = health_check(endpoint, HealthStatus::Healthy, &snapshot).await;
        let json = serde_json::to_string_pretty(&resp).expect("serialize");
        println!("    GET {} → {} (HTTP {})", endpoint, resp.status, resp.status.http_status_code());
        println!("    {json}");
        println!();
    }

    // 8. Observability Bus (集成入口)
    println!("[8] Observability Bus (集成入口):");
    let bus = ObservabilityBus::default();
    let span_root = SpanContext::new_root("demo_span");
    bus.push_span(span_root.clone()).await;
    println!("    push span:    trace_id={} span_id={}", span_root.trace_id, span_root.span_id);
    let sample = MetricSample::new("demo_total", MetricKind::Counter, 1.0)
        .with_label("demo", "true");
    bus.record_metric(sample).await;
    bus.set_health(HealthStatus::Healthy).await;
    let current = bus.current_span().await.unwrap();
    let metrics_count = bus.list_metrics().await.len();
    let health = bus.health().await;
    println!("    current span: {current:?}");
    println!("    metrics:      {metrics_count} samples");
    println!("    health:       {health}");
    bus.pop_span().await;
    println!();

    // 9. 3 端点 path 一致性
    println!("[9] 3 端点 path 编译期对齐 HEALTH_ENDPOINTS:");
    for ep in [
        HealthEndpoint::Health,
        HealthEndpoint::Ready,
        HealthEndpoint::Metrics,
    ] {
        let in_list = apeireth_observability::HEALTH_ENDPOINTS.contains(&ep.as_path());
        println!(
            "    {}  in HEALTH_ENDPOINTS = {}",
            ep,
            in_list
        );
    }
    println!();

    // 10. 关闭
    println!("[10] MetricKind 3 类支持:");
    for k in [
        MetricKind::Counter,
        MetricKind::Gauge,
        MetricKind::Histogram,
    ] {
        println!("    MetricKind::{k} → Prometheus \"{}\"", k.as_str());
    }
    println!();

    let _ = HealthResponse::new("/ready", HealthStatus::Healthy); // 验证 re-export
    println!("=== demo done ===");
}
