//! G10 hardening: the loopback token gate on the sensitive endpoints
//! (`/v1/admin/config`, `/v1/approvals/resolve`).
//!
//! The gate has three modes (see `apeireth_gateway::GatewayTokenGate`), and
//! these tests pin each one's behavior plus the two cross-cutting invariants:
//! forged origin headers still earn no CORS grant (existing allowlist has zero
//! regression), and no token value ever appears in frames, audit facts, or
//! debug rendering (脱敏).

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, GatewayServices,
    GatewayState, GatewayTokenGate,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Test token value. Deliberately shaped like a credential so a leak in any
/// assertion output is obvious; never a real secret.
const TOKEN: &str = "loopback-token-0123456789abcdef";
const WRONG_TOKEN: &str = "forged-token-fedcba9876543210";

/// Audit archive fake: records every fact flushed through the observation sink.
#[derive(Debug, Default)]
struct RecordingAudit {
    facts: Mutex<Vec<(String, Option<String>)>>,
}

impl RecordingAudit {
    fn events(&self) -> Vec<String> {
        self.facts
            .lock()
            .unwrap()
            .iter()
            .map(|(event, _)| event.clone())
            .collect()
    }

    fn detail_of(&self, event: &str) -> Option<String> {
        self.facts
            .lock()
            .unwrap()
            .iter()
            .find(|(name, _)| name == event)
            .and_then(|(_, detail)| detail.clone())
    }
}

#[async_trait::async_trait]
impl AuditCommand for RecordingAudit {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.facts
            .lock()
            .unwrap()
            .push((event.to_string(), detail.map(str::to_string)));
    }
}

struct Harness {
    state: GatewayState,
    audit: Arc<RecordingAudit>,
}

/// One gateway assembly with an explicit gate mode (no env mutation).
async fn harness(gate: GatewayTokenGate) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    Harness { state, audit }
}

fn admin_get(token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("GET")
        .uri("/v1/admin/config")
        .body(Body::empty())
        .unwrap();
    if let Some(token) = token {
        request
            .headers_mut()
            .insert("authorization", token_header(token));
    }
    request
}

fn admin_get_with_origin(token: Option<&str>, origin: &str) -> Request<Body> {
    let mut request = admin_get(token);
    request
        .headers_mut()
        .insert("origin", origin.parse().expect("origin header"));
    request
}

fn resolve_post(token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/approvals/resolve")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({
                "session": uuid::Uuid::new_v4().to_string(),
                "approval": uuid::Uuid::new_v4().to_string(),
                "decision": "approve"
            }))
            .expect("resolve body"),
        ))
        .unwrap();
    if let Some(token) = token {
        request
            .headers_mut()
            .insert("authorization", token_header(token));
    }
    request
}

fn token_header(token: &str) -> axum::http::HeaderValue {
    format!("Bearer {token}").parse().expect("bearer header")
}

async fn json_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

// ---------------------------------------------------------------------------
// 口径解析: env 值 → 三态
// ---------------------------------------------------------------------------

/// env 值到三态口径的映射: 哨兵 `none` = 显式不设防, 非空 = 强制令牌,
/// 缺失/空白 = 收紧后的默认态。
#[test]
fn gate_parse_maps_env_values_to_the_three_modes() {
    assert_eq!(
        GatewayTokenGate::parse(None),
        GatewayTokenGate::UnprotectedDefault
    );
    assert_eq!(
        GatewayTokenGate::parse(Some("   ".to_string())),
        GatewayTokenGate::UnprotectedDefault
    );
    assert_eq!(
        GatewayTokenGate::parse(Some("none".to_string())),
        GatewayTokenGate::UnprotectedDeclared
    );
    assert_eq!(
        GatewayTokenGate::parse(Some("  secret-1  ".to_string())),
        GatewayTokenGate::Enforced("secret-1".to_string())
    );
    assert_eq!(
        GatewayTokenGate::Enforced(TOKEN.to_string()).enforced_token(),
        Some(TOKEN)
    );
    assert_eq!(GatewayTokenGate::UnprotectedDefault.enforced_token(), None);
}

/// 令牌不进 Debug: 任何日志行格式化门口径都拿不到令牌值。
#[test]
fn debug_rendering_never_leaks_the_configured_token() {
    let gate = GatewayTokenGate::Enforced(TOKEN.to_string());
    let rendered = format!("{gate:?}");
    assert!(!rendered.contains(TOKEN), "Debug 必须打码, got {rendered}");
    assert!(rendered.contains("redacted"), "{rendered}");
}

// ---------------------------------------------------------------------------
// Enforced (桌面自铸令牌注入后的生产口径)
// ---------------------------------------------------------------------------

/// 带令牌调用通过 / 缺令牌 401 `auth_missing_key` / 错令牌 401
/// `auth_invalid_key` —— 两个敏感端点同一口径。
#[tokio::test]
async fn enforced_gate_rejects_missing_and_invalid_bearer_and_passes_the_matching_one() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = || canonical_router_with_state(harness.state.clone());

    // 带正确令牌: 进入 handler (GET admin config 返回配置视图)。
    let (status, _) = json_body(router().oneshot(admin_get(Some(TOKEN))).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "带令牌调用必须通过");

    // 缺 Bearer: 401 auth_missing_key。
    let (status, body) = json_body(router().oneshot(admin_get(None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("auth_missing_key"), "{body}");

    // 伪令牌: 401 auth_invalid_key。
    let (status, body) = json_body(
        router()
            .oneshot(admin_get(Some(WRONG_TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("auth_invalid_key"), "{body}");

    // 审批解决端点同门同口径。
    let (status, body) = json_body(router().oneshot(resolve_post(None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("auth_missing_key"), "{body}");

    let (status, _) = json_body(router().oneshot(resolve_post(Some(TOKEN))).await.unwrap()).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "过门后进 handler (未知审批)"
    );
}

/// 拒绝帧只说门语义, 不回显任何令牌值 (期望值与呈现值都不进正文)。
#[tokio::test]
async fn rejection_frames_never_echo_any_token_value() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = canonical_router_with_state(harness.state.clone());

    let (_, body) = json_body(router.oneshot(admin_get(Some(WRONG_TOKEN))).await.unwrap()).await;
    assert!(!body.contains(WRONG_TOKEN), "呈现的伪令牌不得回显: {body}");
    assert!(!body.contains(TOKEN), "期望令牌不得外泄: {body}");

    // 审计留痕有事实、无令牌。
    harness.state.observations.flush().await;
    let detail = harness.audit.detail_of("gateway.auth_rejected");
    assert!(detail.as_deref().unwrap_or("").contains("reason=invalid"));
    assert!(!detail.unwrap_or_default().contains(TOKEN), "审计不含令牌");
}

// ---------------------------------------------------------------------------
// UnprotectedDefault (env 未设 = 收紧后的默认态: 放行 + 警告帧 + 审计)
// ---------------------------------------------------------------------------

/// 未设令牌: 无认证调用仍放行 (无令牌客户端兼容路径零断), 但每次放行记
/// 警告帧 + 审计事实 —— 不再静默。
#[tokio::test]
async fn unset_gate_accepts_unauthenticated_calls_with_warning_frame_and_audit() {
    let harness = harness(GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(harness.state.clone());
    let mut frames = harness.state.events.subscribe();

    // 无令牌客户端兼容路径: 调用到达 handler (未知审批 → 400), 不是 401。
    let (status, _) = json_body(router.oneshot(resolve_post(None)).await.unwrap()).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "未设令牌时无认证调用必须放行到 handler"
    );

    // 警告帧: 端点 + 模式, 不含令牌面。
    let frame = frames.try_recv().expect("warning frame");
    assert_eq!(frame.event, "gateway_auth_unprotected");
    assert_eq!(frame.data["endpoint"], "/v1/approvals/resolve");
    assert_eq!(frame.data["mode"], "env_unset");

    // 审计事实同行留痕。
    harness.state.observations.flush().await;
    let detail = harness.audit.detail_of("gateway.auth_unprotected");
    assert_eq!(
        detail.as_deref(),
        Some("endpoint=/v1/approvals/resolve mode=env_unset")
    );
}

// ---------------------------------------------------------------------------
// UnprotectedDeclared (逃生阀 `APEIRETH_GATEWAY_TOKEN=none`: 明示不设防)
// ---------------------------------------------------------------------------

/// 逃生阀: 显式 `none` = 明示不设防 —— 放行且不逐次告警, 但声明本身与
/// 每次放行都入审计 (留痕)。
#[tokio::test]
async fn declared_unprotected_escape_valve_stays_quiet_but_audit_traced() {
    let harness = harness(GatewayTokenGate::UnprotectedDeclared).await;
    let router = canonical_router_with_state(harness.state.clone());
    let mut frames = harness.state.events.subscribe();

    let (status, _) = json_body(router.oneshot(admin_get(None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "逃生阀下放行");

    assert!(frames.try_recv().is_err(), "明示不设防不逐次发警告帧");

    harness.state.observations.flush().await;
    assert!(
        harness
            .audit
            .events()
            .contains(&"gateway.auth_unprotected".to_string()),
        "放行必须留审计痕: {:?}",
        harness.audit.events()
    );
    let detail = harness.audit.detail_of("gateway.auth_unprotected");
    assert_eq!(
        detail.as_deref(),
        Some("endpoint=/v1/admin/config mode=declared_none")
    );
}

/// 逃生阀的显式声明本身在装配期入审计 (留痕): env `APEIRETH_GATEWAY_TOKEN=none`
/// 一装配就记 `gateway.auth_declared_unprotected`, 不等第一次调用。
#[tokio::test]
async fn the_escape_valve_declaration_itself_is_audit_traced_at_assembly() {
    let previous = std::env::var(GatewayTokenGate::ENV).ok();
    std::env::set_var(GatewayTokenGate::ENV, "none");
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let state = build_gateway_state_with_services(runtime, services);
    match previous {
        Some(value) => std::env::set_var(GatewayTokenGate::ENV, value),
        None => std::env::remove_var(GatewayTokenGate::ENV),
    }

    assert_eq!(state.token_gate, GatewayTokenGate::UnprotectedDeclared);
    state.observations.flush().await;
    let detail = audit.detail_of("gateway.auth_declared_unprotected");
    assert!(
        detail
            .as_deref()
            .unwrap_or_default()
            .contains("escape valve"),
        "声明必须留痕, got {detail:?}"
    );
}

// ---------------------------------------------------------------------------
// 白名单零回归 + 双端点同门
// ---------------------------------------------------------------------------

/// 伪来源头仍拿不到 CORS 放行 (来源白名单语义零回归): 无论令牌门口径,
/// 跨源恶意页面读不到 `access-control-allow-origin`。
#[tokio::test]
async fn forged_origin_headers_still_earn_no_cors_grant() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = canonical_router_with_state(harness.state.clone());

    for origin in [
        "https://evil.example.com",
        "http://tauri.localhost.evil.com",
        "http://127.0.0.1.evil.com",
        "null",
    ] {
        let response = router
            .clone()
            .oneshot(admin_get_with_origin(Some(TOKEN), origin))
            .await
            .unwrap();
        let allow_origin = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        assert!(
            allow_origin.is_empty(),
            "伪来源头 {origin} 不得拿到 CORS 放行, got: {allow_origin}"
        );
    }

    // 本地桌面来源照旧放行 (桌面壳 UI 零断)。
    let response = router
        .clone()
        .oneshot(admin_get_with_origin(Some(TOKEN), "http://tauri.localhost"))
        .await
        .unwrap();
    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(
        allow_origin.contains("tauri.localhost") || allow_origin == "*",
        "本地来源必须继续放行, got: {allow_origin}"
    );
}

/// 令牌门是端点级的: 审批解决与 admin 配置两个敏感端点同一门同一口径
/// (本组用未设态钉住"两个端点都过警告+审计", Enforced 组钉住"都 401")。
#[tokio::test]
async fn both_sensitive_endpoints_share_one_gate_semantics() {
    let harness = harness(GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(harness.state.clone());

    let (_, _) = json_body(router.clone().oneshot(admin_get(None)).await.unwrap()).await;
    let (_, _) = json_body(router.oneshot(resolve_post(None)).await.unwrap()).await;

    harness.state.observations.flush().await;
    let facts = harness.audit.facts.lock().unwrap().clone();
    let unprotected: Vec<_> = facts
        .iter()
        .filter(|(event, _)| event == "gateway.auth_unprotected")
        .collect();
    assert_eq!(unprotected.len(), 2, "两个敏感端点各留一痕: {facts:?}");
    let endpoints: Vec<_> = unprotected
        .iter()
        .filter_map(|(_, detail)| detail.as_deref())
        .collect();
    assert!(
        endpoints.iter().any(|d| d.contains("/v1/admin/config")),
        "{endpoints:?}"
    );
    assert!(
        endpoints
            .iter()
            .any(|d| d.contains("/v1/approvals/resolve")),
        "{endpoints:?}"
    );
}
