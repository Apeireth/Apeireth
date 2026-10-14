//! G-P1 指标端点的面契约: 真路由过 W2-② 令牌门 + `GET /v1/panel/metrics` 的
//! JSON 合同 (含空态 "—" 语义)。
//!
//! 这里钉三件事:
//! 1. 空态 = 显式空 (计数 0 / 分布 `null` / `sessions: []`),不是缺字段或 501;
//! 2. 有数据时会话 + 全局两层聚合按合同形状出现 (分布 = 均值/P50/P95/计数);
//! 3. 指标端点与面板族同在令牌门内: 缺 Bearer 401 `auth_missing_key`,
//!    伪造 Bearer 401 `auth_invalid_key`,真令牌 200。

use apeireth_core::kernel::SessionId;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices, GatewayState,
    GatewayTokenGate,
};
use apeireth_runtime::canonical::{Runtime, TurnMetrics, TurnOutcomeKind};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "panel-metrics-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";

async fn harness(gate: GatewayTokenGate) -> GatewayState {
    let runtime = std::sync::Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime, GatewayServices::default());
    state.token_gate = gate;
    state
}

fn request(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer"),
        );
    }
    request
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// 空态合同: 显式空 + "—" 语义 (`null`),字段一个不少。
#[tokio::test]
async fn metrics_endpoint_empty_state_is_explicit_and_dash_honest() {
    let state = harness(GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/metrics", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["retention"], "in_process_rolling");
    assert_eq!(body["window_turns"], 50);
    let layer = &body["global"];
    assert_eq!(layer["turns"], 0);
    assert_eq!(layer["succeeded"], 0);
    assert_eq!(layer["failed"], 0);
    assert_eq!(layer["pending_approval"], 0);
    for column in [
        "latency_ms",
        "input_tokens",
        "output_tokens",
        "rounds",
        "tool_calls",
    ] {
        assert_eq!(layer[column]["count"], 0, "{column} 计数显式 0");
        assert_eq!(
            layer[column]["mean"],
            serde_json::Value::Null,
            "{column} 空态 —"
        );
        assert_eq!(layer[column]["p50"], serde_json::Value::Null);
        assert_eq!(layer[column]["p95"], serde_json::Value::Null);
        assert_eq!(layer[column]["sum"], serde_json::Value::Null);
    }
    assert_eq!(layer["cache_hit_tokens"], serde_json::Value::Null);
    assert_eq!(layer["cache_hit_rate"], serde_json::Value::Null);
    assert_eq!(layer["cost_micros"], serde_json::Value::Null);
    assert_eq!(body["sessions"], serde_json::json!([]));
}

/// 有数据: 会话 + 全局两层聚合按合同形状出现,未知列仍如实 `null`。
#[tokio::test]
async fn metrics_endpoint_reports_both_layers_with_nearest_rank_distribution() {
    let state = harness(GatewayTokenGate::UnprotectedDefault).await;
    let session = SessionId::new();
    for (latency, input) in [(10u64, 100u64), (20, 200), (30, 300)] {
        state.runtime.metrics().record(
            session,
            TurnMetrics {
                outcome: TurnOutcomeKind::Succeeded,
                latency_ms: latency,
                input_tokens: Some(input),
                output_tokens: Some(input / 10),
                cache_hit_tokens: None,
                cache_hit_rate: None,
                tool_calls: Some(1),
                rounds: Some(1),
                cost_micros: None,
            },
        );
    }
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/metrics", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["global"]["turns"], 3);
    assert_eq!(body["global"]["succeeded"], 3);
    assert_eq!(body["global"]["latency_ms"]["count"], 3);
    assert_eq!(body["global"]["latency_ms"]["p50"], 20, "最近秩 P50");
    assert_eq!(body["global"]["latency_ms"]["p95"], 30, "最近秩 P95");
    assert_eq!(body["global"]["latency_ms"]["sum"], 60);
    assert_eq!(body["global"]["input_tokens"]["sum"], 600);
    assert_eq!(body["global"]["cache_hit_tokens"], serde_json::Value::Null);
    let sessions = body["sessions"].as_array().expect("sessions array");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["session_id"], session.to_string());
    assert_eq!(sessions[0]["aggregate"]["turns"], 3);
    assert_eq!(sessions[0]["aggregate"]["tool_calls"]["sum"], 3);
}

/// 令牌门同面: 强制态下指标端点与面板族同一门 (401 矩阵 + 真令牌 200)。
#[tokio::test]
async fn metrics_endpoint_is_behind_the_gateway_token_gate() {
    let state = harness(GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = canonical_router_with_state(state);

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/metrics", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_missing_key"), "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/metrics", Some(FORGED_TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_invalid_key"), "{text}");
    assert!(!text.contains(TOKEN), "401 帧不得回显令牌: {text}");

    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/metrics", Some(TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("\"schema_version\""), "{text}");
}
