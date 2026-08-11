//! `apeireth-observability` 集成测试 (per R20 阶段 1 Fixture 5 模式)
//!
//! 4 K-1 强校验 + 1 PII 脱敏 fixture + 1 m3 fixture, 全部 in-process 跑.
//! 0 引 OpenTelemetry SDK (留 R20 阶段 3 续).

use apeireth_observability::{
    health::{health_check, HealthConfig, HealthEndpoint},
    logging::{log_structured, LogEntry, LogLevel},
    metrics::{parse_prometheus, render_prometheus, MetricsRegistry},
    redact_pii,
    tracing_integration::{parse_traceparent, render_traceparent, trace_span, TraceContext},
    validate_tool_call, HealthResponse, HealthStatus, MetricKind, MetricSample,
    ObservabilityBus, ObservabilityError, SpanContext, SpanId, TraceId, HEALTH_ENDPOINTS,
    LOG_FORMAT_JSON, OBSERVABILITY_SCHEMA_VERSION, OTLP_DEFAULT_ENDPOINT, PLATFORM_NAME,
    PROMETHEUS_DEFAULT_PORT, SUPPORTED_METRICS, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
    TRACING_SAMPLE_RATE,
};

// ============================================================================
// K-1 强校验 4 条 (per task spec)
// ============================================================================

/// K-1 fixture #1: 平台名 = "apeireth" (5 K-1 字样 #1).
#[test]
fn k1_platform_name_is_apeireth() {
    assert_eq!(PLATFORM_NAME, "apeireth", "PLATFORM_NAME 必须 = 'apeireth' (5 K-1 字样 #1)");
}

/// K-1 fixture #2: 3 MetricKind 枚举 (5 K-1 字样 #2: 'observability').
#[test]
fn k1_metric_kind_has_3_variants() {
    assert_eq!(SUPPORTED_METRICS.len(), 3, "SUPPORTED_METRICS 必须 3 项 (K-1 强校验 #2)");
    assert!(SUPPORTED_METRICS.contains(&MetricKind::Counter));
    assert!(SUPPORTED_METRICS.contains(&MetricKind::Gauge));
    assert!(SUPPORTED_METRICS.contains(&MetricKind::Histogram));
}

/// K-1 fixture #3: TOOL_WHITELIST 8 工具名 + 5 K-1 字样 命中 ("apeireth" / "observability" / "trace" / "metric" / "must-do").
#[test]
fn k1_tool_whitelist_8_with_5_key_words() {
    assert_eq!(TOOL_WHITELIST.len(), TOOL_WHITELIST_COUNT, "TOOL_WHITELIST_COUNT 必 8");
    assert_eq!(TOOL_WHITELIST_COUNT, 8, "TOOL_WHITELIST_COUNT hardcode = 8");

    // 5 K-1 字样: "apeireth" / "observability" / "trace" / "metric" / "must-do"
    let body = TOOL_WHITELIST.join(",");
    assert!(body.contains("apeireth"), "5 K-1 字样 #1: 'apeireth'");
    assert!(body.contains("observability"), "5 K-1 字样 #2: 'observability'");
    assert!(body.contains("trace"), "5 K-1 字样 #3: 'trace'");
    assert!(body.contains("metric"), "5 K-1 字样 #4: 'metric'");

    // "must-do" 字样在源码注释 / 文档 / 错误信息中体现 (K-1 #5)
    let src = include_str!("../src/lib.rs");
    assert!(src.contains("must-do"), "5 K-1 字样 #5: 'must-do' 必须在源码注释中");

    // 8 工具名硬枚举 (8 名字全验)
    let expected_8 = [
        "apeireth_observability_trace",
        "apeireth_observability_metric_counter",
        "apeireth_observability_metric_gauge",
        "apeireth_observability_metric_histogram",
        "apeireth_observability_health_check",
        "apeireth_observability_pii_redact",
        "apeireth_observability_log_structured",
        "apeireth_observability_otlp_push",
    ];
    for name in expected_8 {
        assert!(TOOL_WHITELIST.contains(&name), "TOOL_WHITELIST 缺 {name}");
    }
}

/// K-1 fixture #4: 3 HEALTH_ENDPOINTS (per K-1 #4).
#[test]
fn k1_3_health_endpoints() {
    assert_eq!(HEALTH_ENDPOINTS.len(), 3, "HEALTH_ENDPOINTS 必 3 项 (K-1 强校验 #4)");
    assert!(HEALTH_ENDPOINTS.contains(&"/health"));
    assert!(HEALTH_ENDPOINTS.contains(&"/ready"));
    assert!(HEALTH_ENDPOINTS.contains(&"/metrics"));

    // 编译期常量 (K-1 强校验 #4 增强)
    assert_eq!(PROMETHEUS_DEFAULT_PORT, 9090, "Prometheus port 必 9090");
    assert_eq!(
        OTLP_DEFAULT_ENDPOINT, "http://localhost:4317",
        "OTLP endpoint 必 localhost:4317"
    );
    assert_eq!(TRACING_SAMPLE_RATE, 1.0, "Sampling rate 必 1.0");
    assert!(LOG_FORMAT_JSON, "Log format 必 JSON");
    assert_eq!(OBSERVABILITY_SCHEMA_VERSION, "1", "Schema 必 v1");
}

// ============================================================================
// P0 PII 脱敏 fixture (5 段验证, per P0 安全铁律 + 跟 keyring SecretBytes 一致)
// ============================================================================

/// P0 fixture: PII 脱敏 (per task spec 5 段验证).
#[test]
fn p0_pii_redaction() {
    // 1. password=:xxx 模式 → 脱敏为 password=:***
    let r = redact_pii("password=secret123").expect("redact ok");
    assert!(r.contains("password=***"), "password 脱敏: {r}");
    assert!(!r.contains("secret123"), "明文严禁出现: {r}");

    // 2. token=xxx 模式 → 脱敏为 token=***
    let r = redact_pii("api_token=abc-def").expect("redact ok");
    assert!(r.contains("api_token=***"), "token 脱敏: {r}");
    assert!(!r.contains("abc-def"), "明文严禁出现: {r}");

    // 3. secret=xxx 模式 → 脱敏为 secret=***
    let r = redact_pii("db_secret=topsecret").expect("redact ok");
    assert!(r.contains("db_secret=***"), "secret 脱敏: {r}");
    assert!(!r.contains("topsecret"), "明文严禁出现: {r}");

    // 4. key=xxx 模式 → 脱敏为 key=***
    let r = redact_pii("encryption_key=xyz").expect("redact ok");
    assert!(r.contains("encryption_key=***"), "key 脱敏: {r}");
    assert!(!r.contains("xyz"), "明文严禁出现: {r}");

    // 5. 静态扫描: lib.rs 实际代码行 (排除注释) `redact_pii` 调用 ≥1, 0 错
    let src = include_str!("../src/lib.rs");
    let real_calls: Vec<&str> = src
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//")
                && !t.starts_with("*")
                && !t.starts_with("///")
                && t.contains("redact_pii")
        })
        .collect();
    assert!(
        !real_calls.is_empty(),
        "redact_pii 实际代码行调用 ≥1 (排除注释), 实际 = {real_calls:?}"
    );

    // 5b. 同样在 logging.rs 实际代码行有 redact_pii 调用 (PII 脱敏联动)
    let logging_src = include_str!("../src/logging.rs");
    let logging_calls: Vec<&str> = logging_src
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//")
                && !t.starts_with("*")
                && !t.starts_with("///")
                && t.contains("redact_pii")
        })
        .collect();
    assert!(
        !logging_calls.is_empty(),
        "logging.rs 实际代码行 redact_pii 调用 ≥1, 实际 = {logging_calls:?}"
    );
}

// ============================================================================
// m3 防御 fixture
// ============================================================================

/// m3 fixture: validate_tool_call 拒绝白名单外.
#[test]
fn m3_validate_tool_call_rejects_unknown() {
    // 白名单内 — 8 工具全验
    for tool in TOOL_WHITELIST {
        assert!(
            validate_tool_call(tool, &serde_json::json!({})).is_ok(),
            "{tool} 应通过白名单"
        );
    }

    // 白名单外 (m3 hallucination)
    let err = validate_tool_call("apeireth_observability_evil", &serde_json::json!({}))
        .unwrap_err();
    assert!(matches!(err, ObservabilityError::ToolNotWhitelisted(_)));

    // 跨 crate 工具名 (估 m3 调 apeireth_keyring 工具误传)
    let err = validate_tool_call("apeireth_keyring_set", &serde_json::json!({}))
        .unwrap_err();
    assert!(matches!(err, ObservabilityError::ToolNotWhitelisted(_)));

    // 空字符串
    let err = validate_tool_call("", &serde_json::json!({})).unwrap_err();
    assert!(matches!(err, ObservabilityError::ToolNotWhitelisted(_)));
}

// ============================================================================
// 单元覆盖 fixture
// ============================================================================

/// TraceId / SpanId 格式.
#[test]
fn trace_and_span_id_format() {
    let trace_id = TraceId::new();
    let span_id = SpanId::new();
    assert_eq!(trace_id.as_hex().len(), 32, "trace_id 必 32 hex");
    assert_eq!(span_id.as_hex().len(), 16, "span_id 必 16 hex");
}

/// SpanContext root + child 继承 trace_id.
#[test]
fn span_context_root_and_child() {
    let root = SpanContext::new_root("http_request");
    let child = SpanContext::new_child("db_query", &root);
    assert_eq!(root.trace_id, child.trace_id, "child 继承 trace_id");
    assert_ne!(root.span_id, child.span_id, "child 新 span_id");
    assert_eq!(child.parent_span_id.as_ref().unwrap(), &root.span_id);
}

/// W3C traceparent 序列化 / 解析 roundtrip.
#[test]
fn traceparent_roundtrip() {
    let ctx = trace_span("test");
    let header = render_traceparent(&ctx);
    let parsed = parse_traceparent(&header).expect("parse ok");
    assert_eq!(parsed.trace_id, ctx.trace_id);
    assert_eq!(parsed.span_id, ctx.span_id);
}

/// Prometheus render + parse roundtrip.
#[test]
fn prometheus_render_parse_roundtrip() {
    let samples = vec![
        MetricSample::new("requests_total", MetricKind::Counter, 42.0)
            .with_label("endpoint", "/api/v1"),
        MetricSample::new("active_connections", MetricKind::Gauge, 7.0),
    ];
    let text = render_prometheus(&samples);
    let parsed = parse_prometheus(&text).expect("parse ok");
    assert_eq!(parsed.len(), 2);
    let total = parsed.iter().find(|s| s.name == "requests_total").unwrap();
    assert_eq!(total.value, 42.0);
    assert_eq!(
        total.labels.get("endpoint").map(|s| s.as_str()),
        Some("/api/v1")
    );
}

/// MetricsRegistry counter 累加 + gauge 覆盖.
#[tokio::test]
async fn metrics_registry_counter_and_gauge() {
    let r = MetricsRegistry::new();
    let mut labels = std::collections::HashMap::new();
    labels.insert("endpoint".to_string(), "/api".to_string());

    r.counter("requests_total", 1.0, labels.clone()).await;
    r.counter("requests_total", 2.0, labels.clone()).await;
    r.gauge("active", 5.0, std::collections::HashMap::new()).await;
    r.gauge("active", 10.0, std::collections::HashMap::new()).await;

    let snap = r.snapshot().await;
    let total = snap.iter().find(|s| s.name == "requests_total").unwrap();
    assert_eq!(total.value, 3.0, "counter 必累加");
    let active = snap.iter().find(|s| s.name == "active").unwrap();
    assert_eq!(active.value, 10.0, "gauge 必覆盖");
}

/// Health 3 端点.
#[tokio::test]
async fn health_3_endpoints() {
    let samples = vec![MetricSample::new("test", MetricKind::Gauge, 1.0)];
    let health_resp = health_check(HealthEndpoint::Health, HealthStatus::Healthy, &[]).await;
    assert_eq!(health_resp.endpoint, "/health");
    assert_eq!(health_resp.status, HealthStatus::Healthy);

    let ready_resp = health_check(HealthEndpoint::Ready, HealthStatus::Degraded, &[]).await;
    assert_eq!(ready_resp.endpoint, "/ready");
    assert_eq!(ready_resp.status, HealthStatus::Degraded);

    let metrics_resp = health_check(HealthEndpoint::Metrics, HealthStatus::Healthy, &samples).await;
    assert_eq!(metrics_resp.endpoint, "/metrics");
    assert_eq!(
        metrics_resp.details.get("metric_count").map(|s| s.as_str()),
        Some("1")
    );
}

/// HealthEndpoint 路径解析.
#[test]
fn health_endpoint_path_parse() {
    for ep in [
        HealthEndpoint::Health,
        HealthEndpoint::Ready,
        HealthEndpoint::Metrics,
    ] {
        let parsed = HealthEndpoint::from_path(ep.as_path()).expect("parse");
        assert_eq!(parsed, ep);
    }
    let err = HealthEndpoint::from_path("/unknown").unwrap_err();
    assert!(matches!(err, ObservabilityError::HealthEndpointUnknown(_)));
}

/// HealthConfig 默认 3 端点.
#[test]
fn health_config_default() {
    let cfg = HealthConfig::default();
    assert!(cfg.validate());
    assert_eq!(cfg.endpoints.len(), 3);
}

/// LogEntry PII 脱敏联动.
#[test]
fn log_entry_pii_redaction() {
    let entry = LogEntry::new(LogLevel::Info, "test", "password=abc");
    assert!(entry.message.contains("password=***"));
    assert!(!entry.message.contains("abc"));

    let mut fields = std::collections::HashMap::new();
    fields.insert("k".to_string(), serde_json::json!("v"));
    let entry = log_structured(LogLevel::Info, "test", "token=xyz", Some(fields));
    assert!(entry.message.contains("token=***"));
    assert!(!entry.message.contains("xyz"));
}

/// HealthResponse serde roundtrip.
#[test]
fn health_response_serde_roundtrip() {
    let resp = HealthResponse::new("/ready", HealthStatus::Healthy)
        .with_detail("k", "v");
    let json = serde_json::to_string(&resp).expect("serialize");
    let back: HealthResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.status, HealthStatus::Healthy);
    assert_eq!(back.endpoint, "/ready");
    assert_eq!(back.details.get("k").map(|s| s.as_str()), Some("v"));
}

/// HealthStatus HTTP 状态码.
#[test]
fn health_status_http_codes() {
    assert_eq!(HealthStatus::Healthy.http_status_code(), 200);
    assert_eq!(HealthStatus::Degraded.http_status_code(), 200);
    assert_eq!(HealthStatus::Unhealthy.http_status_code(), 503);
}

/// ObservabilityBus 集成入口 (push span + record metric + set health).
#[tokio::test]
async fn observability_bus_full_cycle() {
    let bus = ObservabilityBus::default();

    let root = SpanContext::new_root("integration_test");
    bus.push_span(root.clone()).await;
    let current = bus.current_span().await.unwrap();
    assert_eq!(current.name, "integration_test");

    bus.record_metric(MetricSample::new(
        "bus_counter",
        MetricKind::Counter,
        1.0,
    ))
    .await;
    assert_eq!(bus.list_metrics().await.len(), 1);

    bus.set_health(HealthStatus::Degraded).await;
    assert_eq!(bus.health().await, HealthStatus::Degraded);

    bus.pop_span().await;
    assert!(bus.current_span().await.is_none());
}

/// TraceContext child 继承 trace_id.
#[test]
fn trace_context_inherit() {
    let root = TraceContext::new_root("root");
    let child = TraceContext::new_child("child", &root);
    assert_eq!(root.trace_id, child.trace_id);
    assert_ne!(root.span_id, child.span_id);
}
