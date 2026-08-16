//! Observability bus (apeireth-observability 3 端点 + PII 脱敏 + trace_id 跨 crate 继承)
//!
//! 覆盖 `apeireth-observability` 3 健康端点 (1:1 翻译 Prometheus / Kubernetes 工业标准):
//! - `/health` (liveness)
//! - `/ready` (readiness, 走 current_status)
//! - `/metrics` (Prometheus scrape)
//!
//! 跨 crate trace_id 继承 (per W3C Trace Context spec):
//! - root span → child span 共享 trace_id, 新 span_id
//! - `traceparent` header round-trip 32/16 hex
//!
//! PII 脱敏 (2 重防御):
//! - `redact_pii` 把 `password=xxx` / `token:xxx` 模式中的值替换为 `***`
//! - `keyring::SecretBytes` Serialize 脱敏 `***REDACTED***`
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md §3`

use apeireth_observability::{
    redact_pii, render_prometheus, HealthEndpoint, HealthResponse, HealthStatus, MetricKind,
    MetricSample, ObservabilityBus, ObservabilityConfig, SpanContext, TraceId, SpanId,
    HEALTH_CHECK_TIMEOUT_MS, HEALTH_ENDPOINTS, HEALTH_ENDPOINTS_COUNT, PII_REDACTION_PATTERN,
    PLATFORM_NAME, SUPPORTED_METRICS, TOOL_WHITELIST, TOOL_WHITELIST_COUNT, OBSERVABILITY_SCHEMA_VERSION,
    k1_all_pass, k1_check_platform_name, k1_check_health_endpoints, k1_check_metrics_count,
    k1_check_tool_whitelist,
};

/// 9 测试覆盖 observability bus 集成
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observability_3_endpoints_compile_time_hardcoded() {
        // K-1 强校验: HEALTH_ENDPOINTS_COUNT = 3 (1:1 翻译 /health /ready /metrics)
        assert_eq!(HEALTH_ENDPOINTS_COUNT, 3, "3 健康端点编译期守门");
        assert!(HEALTH_ENDPOINTS.contains(&"/health"));
        assert!(HEALTH_ENDPOINTS.contains(&"/ready"));
        assert!(HEALTH_ENDPOINTS.contains(&"/metrics"));
    }

    #[test]
    fn observability_tool_whitelist_count_is_8() {
        // m3 hallucination 防御: 8 工具白名单编译期守门
        assert_eq!(TOOL_WHITELIST_COUNT, 8);
        assert_eq!(TOOL_WHITELIST.len(), 8);
    }

    #[test]
    fn observability_k1_all_4_invariants_pass() {
        // K-1 强校验 4 条 (per 主人 19:37 拍板"走在前人肩上")
        assert!(k1_check_platform_name(), "PLATFORM_NAME = 'apeireth'");
        assert!(k1_check_metrics_count(), "SUPPORTED_METRICS 数量守门");
        assert!(k1_check_tool_whitelist(), "TOOL_WHITELIST 长度守门");
        assert!(k1_check_health_endpoints(), "HEALTH_ENDPOINTS 3 端点守门");
        assert!(k1_all_pass(), "K-1 4 强校验全部 PASS");
    }

    #[test]
    fn observability_pii_redaction_in_message() {
        // PII 脱敏: password=xxx / token:xxx → 替换为 ***
        let pwd = redact_pii("password=secret123").expect("PII 脱敏应返 Some");
        assert!(pwd.contains("password=***"), "password 值应脱敏: {pwd}");
        assert!(!pwd.contains("secret123"), "明文严禁出现");

        let tok = redact_pii("api_token=abc-def-ghi").expect("PII 脱敏应返 Some");
        assert!(tok.contains("api_token=***"), "token 值应脱敏: {tok}");
        assert!(!tok.contains("abc-def-ghi"), "明文严禁出现");

        // 非 PII 应原样保留
        let no_pii = redact_pii("hello world").expect("非 PII 应原样");
        assert_eq!(no_pii, "hello world", "非 PII 模式不变");
    }

    #[test]
    fn observability_pii_pattern_matches_password_secret_token_key() {
        // PII_REDACTION_PATTERN 覆盖 4 关键字 (case-insensitive)
        for kw in ["password", "secret", "token", "key"] {
            let input = format!("{kw}=value123");
            let redacted = redact_pii(&input).expect("应脱敏");
            assert!(redacted.contains(&format!("{kw}=***")), "keyword '{kw}' 应匹配: {redacted}");
        }
    }

    #[test]
    fn observability_trace_id_inheritance_root_to_child() {
        // W3C Trace Context: 根 span → 子 span 共享 trace_id, 新 span_id
        let root = SpanContext::new_root("http_request");
        let child = SpanContext::new_child("db_query", &root);
        assert_eq!(root.trace_id, child.trace_id, "trace_id 跨 span 继承");
        assert_ne!(root.span_id, child.span_id, "span_id 跨 span 唯一");
        // 子 span 的 parent 应是 root 的 span_id
        assert_eq!(child.parent_span_id.as_ref(), Some(&root.span_id));
    }

    #[test]
    fn observability_health_status_http_codes() {
        // 健康 → 200, 降级 → 200, 不健康 → 503
        assert_eq!(HealthStatus::Healthy.http_status_code(), 200);
        assert_eq!(HealthStatus::Degraded.http_status_code(), 200);
        assert_eq!(HealthStatus::Unhealthy.http_status_code(), 503);
        // 字符串
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn observability_health_endpoint_round_trip() {
        // 路径 ↔ enum 双向
        for ep in [HealthEndpoint::Health, HealthEndpoint::Ready, HealthEndpoint::Metrics] {
            let path = ep.as_path();
            let back = HealthEndpoint::from_path(path).expect("应解析");
            assert_eq!(back, ep, "path <-> enum 双向 round-trip");
        }
    }

    #[test]
    fn observability_bus_push_pop_span_round_trip() {
        // ObservabilityBus: push_span / pop_span / current_span
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let bus = ObservabilityBus::new(ObservabilityConfig::default());
            // push 2 spans
            let span1 = SpanContext::new_root("span1");
            let span2 = SpanContext::new_root("span2");
            bus.push_span(span1.clone()).await;
            bus.push_span(span2.clone()).await;
            // current span 应该是后入的 span2 (栈顶)
            let current = bus.current_span().await.expect("current span 应有值");
            assert_eq!(current.name, "span2", "栈顶应为 span2");
            // pop span2 → current 变 span1
            let popped = bus.pop_span().await.expect("pop 应有值");
            assert_eq!(popped.name, "span2");
            let current = bus.current_span().await.expect("pop 后 current 仍应有值");
            assert_eq!(current.name, "span1", "pop 后栈顶应为 span1");
        });
    }

    #[test]
    fn observability_metric_render_prometheus_text_format() {
        // 渲染 Prometheus 文本格式 (3 metric kinds)
        let mut labels = std::collections::HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        let samples = vec![
            MetricSample::new("http_requests_total", MetricKind::Counter, 100.0)
                .with_label("method", "GET"),
            MetricSample::new("memory_bytes", MetricKind::Gauge, 1024.0),
            MetricSample::new("request_duration_ms", MetricKind::Histogram, 50.0),
        ];
        let output = render_prometheus(&samples);
        assert!(output.contains("# platform: apeireth"));
        assert!(output.contains("http_requests_total{method=\"GET\"} 100"));
        assert!(output.contains("memory_bytes 1024"));
        assert!(output.contains("request_duration_ms 50"));
        // type 注释
        assert!(output.contains("# TYPE http_requests_total counter"));
        assert!(output.contains("# TYPE memory_bytes gauge"));
        assert!(output.contains("# TYPE request_duration_ms histogram"));
    }

    #[test]
    fn observability_supported_metrics_covers_3_kinds() {
        // 1:1 翻译 Prometheus 4 类 (Counter/Gauge/Histogram/Summary),
        // skeleton 阶段只支持 3 类 (Summary 留 R20 阶段 3 续)
        assert!(SUPPORTED_METRICS.contains(&MetricKind::Counter));
        assert!(SUPPORTED_METRICS.contains(&MetricKind::Gauge));
        assert!(SUPPORTED_METRICS.contains(&MetricKind::Histogram));
    }

    #[test]
    fn observability_constants_compile_time_hardcoded() {
        // K-1 强校验: 平台名 / schema version / 超时都是编译期 hardcode
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(OBSERVABILITY_SCHEMA_VERSION, "1");
        assert_eq!(HEALTH_CHECK_TIMEOUT_MS, 5_000);
        assert!(PII_REDACTION_PATTERN.contains("password"));
        assert!(PII_REDACTION_PATTERN.contains("token"));
    }

    #[test]
    fn observability_trace_id_32_hex_span_id_16_hex() {
        // W3C Trace Context: trace_id 32 hex / span_id 16 hex
        let trace = TraceId::new();
        assert_eq!(trace.as_hex().len(), 32, "trace_id 32 hex");
        let span = SpanId::new();
        assert_eq!(span.as_hex().len(), 16, "span_id 16 hex");
        // round-trip
        let trace2 = TraceId::from_hex(trace.as_hex()).expect("应解析");
        assert_eq!(trace, trace2);
        let span2 = SpanId::from_hex(span.as_hex()).expect("应解析");
        assert_eq!(span, span2);
    }
}
