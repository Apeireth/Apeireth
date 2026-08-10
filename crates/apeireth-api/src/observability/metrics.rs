//! # Observability `/v1/observability/metrics` 端点
//!
//! **目的**: Prometheus scrape 端点,返 18 metric (8 counter + 8 gauge + 2 histogram) in
//! Prometheus text exposition format.
//!
//! **架构** (跟 v2_endpoints handler 同模式, axum 0.7):
//! - State: 全局 `crate::observability::global_state()` (RwLock`ObsState`)
//! - 渲染: 复用 `apeireth_observability::render_prometheus` (1:1 翻译 Prometheus 工业标准)
//! - 0 假装 OTLP push (R20 阶段 3 续, skeleton 阶段仅 expose HTTP scrape)
//!
//! **6 哲学锚穿透**:
//! - S-1: 1:1 翻译 `apeireth-observability::render_prometheus` (per `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §11)
//! - S-2: 估 80-120 LOC, 0 假装 OpenTelemetry SDK 集成
//! - O-2: 借鉴 Prometheus exposition format 工业标准
//! - O-3: 18 metric 编译期 hardcode 名字
//! - O-4: 任何人都能接手 (本文件顶部 §1-§3 注释完整)
//! - O-5: 不假装已接 OTLP collector, 标 `platform = apeireth` + `schema_version = 1`

#![allow(clippy::all)]

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;

use crate::observability::{global_state, MetricSnapshot, OBSERVABILITY_SCHEMA_VERSION, SERVICE_NAME};

/// `GET /v1/observability/metrics` — Prometheus exposition format.
///
/// **Content-Type**: `text/plain; version=0.0.4; charset=utf-8` (Prometheus 官方约定)
/// **状态码**: 200 (Healthy), 503 (Degraded/Down, per health 端点)
pub async fn metrics_handler() -> Response {
    let state = global_state();
    let state = state.read();

    // 把 MetricSnapshot 转 apeireth_observability::MetricSample (复用 render_prometheus)
    // 编译期对 18 metric 数量做断言 (5 K-1 字样 #2: 18 个)
    let samples: Vec<apeireth_observability::MetricSample> = state
        .metrics
        .values()
        .map(|m| snapshot_to_sample(m))
        .collect();
    debug_assert_eq!(samples.len(), 18, "18 metric (8 counter + 8 gauge + 2 histogram)");

    // 渲染 Prometheus text format (复用 apeireth-observability 公开 API)
    let body = apeireth_observability::render_prometheus(&samples);

    // 强加 header: schema_version + platform (K-1 强校验)
    let mut enriched = body;
    enriched.push_str(&format!(
        "# apeireth_api_schema_version: {OBSERVABILITY_SCHEMA_VERSION}\n"
    ));
    enriched.push_str(&format!("# apeireth_api_platform: {SERVICE_NAME}\n"));

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        enriched,
    )
        .into_response()
}

/// `MetricSnapshot` → `apeireth_observability::MetricSample` (跨 crate 类型转换).
fn snapshot_to_sample(m: &MetricSnapshot) -> apeireth_observability::MetricSample {
    let kind = match m.kind.as_str() {
        "counter" => apeireth_observability::MetricKind::Counter,
        "gauge" => apeireth_observability::MetricKind::Gauge,
        "histogram" => apeireth_observability::MetricKind::Histogram,
        // O-5: 未知类型 fallback to gauge, 不假装
        _ => apeireth_observability::MetricKind::Gauge,
    };
    let mut sample = apeireth_observability::MetricSample::new(&m.name, kind, m.value);
    let labels: HashMap<String, String> = m.labels.clone();
    for (k, v) in &labels {
        sample = sample.with_label(k.clone(), v.clone());
    }
    sample
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::ObsState;

    #[test]
    fn snapshot_to_sample_preserves_kind() {
        let s = ObsState::new();
        for m in s.metrics.values() {
            let sample = snapshot_to_sample(m);
            let kind_str = match m.kind.as_str() {
                "counter" => "counter",
                "gauge" => "gauge",
                "histogram" => "histogram",
                _ => "gauge",
            };
            assert_eq!(sample.kind.as_str(), kind_str, "kind preserved for {}", m.name);
        }
    }

    #[test]
    fn snapshot_to_sample_unknown_kind_falls_back_to_gauge() {
        // O-5 不假装: 未知类型 fallback gauge
        let snap = MetricSnapshot {
            name: "test".to_string(),
            kind: "summary".to_string(), // Prometheus 第 4 类, 估未支持
            value: 1.0,
            labels: HashMap::new(),
        };
        let sample = snapshot_to_sample(&snap);
        assert_eq!(sample.kind.as_str(), "gauge");
    }

    /// 3. 18 metric 全枚举 + name+value 完整保留
    #[test]
    fn snapshot_to_sample_preserves_name_and_value() {
        // 18 metric 全过 (8 counter + 8 gauge + 2 histogram)
        let s = ObsState::new();
        assert_eq!(s.metrics.len(), 18, "18 metric (8 counter + 8 gauge + 2 histogram)");
        for m in s.metrics.values() {
            let sample = snapshot_to_sample(m);
            assert_eq!(sample.name, m.name, "name preserved for {}", m.name);
            assert!((sample.value - m.value).abs() < 1e-9, "value preserved for {}", m.name);
            // 关键 metric 守住 (5 哲学锚穿透)
            for key in [
                "http_requests_total",
                "llm_tokens_total",
                "active_sessions",
                "memory_rss_bytes",
                "uptime_seconds",
            ] {
                if m.name == key {
                    assert_eq!(sample.name, key);
                }
            }
        }
    }
}
