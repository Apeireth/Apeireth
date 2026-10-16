//! W2-② 认证面全网关化: 令牌门从两个敏感端点扩到**全 API 面**后的面契约。
//!
//! 这里钉四件事:
//! 1. 强制态 401 矩阵 —— chat / 事件流 / 面板 / 审批 / admin 族逐家过门
//!    (缺 Bearer = 401 `auth_missing_key`, 伪造 Bearer = 401 `auth_invalid_key`);
//! 2. 健康探活豁免 —— `/health` 无认证可用 (探测刚需), 且是**唯一**豁免
//!    (豁免表本身就是权力边界, 直接钉住);
//! 3. 现行为兼容路径 —— 未设口径下无令牌客户端零断 (放行 + 警告帧 + 审计),
//!    桌面自铸令牌过门路径零回归;
//! 4. 客户端令牌表 —— 单令牌族多枚, 过门审计带客户端名; 令牌表在场即强制。

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand,
    ClientTokenConfig, ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayState,
    GatewayTokenGate, UNAUTHENTICATED_PROBE_PATHS,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// 测试令牌 (形似凭据以便泄漏显形, 非真实秘密)。
const TOKEN: &str = "desktop-minted-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";

/// 审计档案 fake: 记录观测攒批面落下的每一条事实。
#[derive(Debug, Default)]
struct RecordingAudit {
    facts: Mutex<Vec<(String, Option<String>)>>,
}

impl RecordingAudit {
    fn details_of(&self, event: &str) -> Vec<String> {
        self.facts
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name == event)
            .filter_map(|(_, detail)| detail.clone())
            .collect()
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

/// 一份网关装配 (显式门口径 + 显式令牌表, 不动进程环境)。
async fn harness(gate: GatewayTokenGate, clients: ClientTokenTable) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    state.client_tokens = clients;
    Harness { state, audit }
}

fn table(entries: &[(&str, &str)]) -> ClientTokenTable {
    ClientTokenTable::from_config(ClientTokenConfig::new(
        entries
            .iter()
            .map(|(name, token)| ClientTokenSpec {
                name: (*name).to_string(),
                token: (*token).to_string(),
                user_id: None,
            })
            .collect(),
    ))
    .expect("valid table")
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .expect("request");
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer"),
        );
    }
    request
}

fn json_body() -> Body {
    Body::from(
        serde_json::to_vec(&serde_json::json!({
            "input": "hello",
            "messages": [{"role": "user", "content": "hello"}],
            "session": "00000000-0000-0000-0000-000000000042",
            "session_id": "00000000-0000-0000-0000-000000000042",
            "decision": "approve",
            "approval": "00000000-0000-0000-0000-000000000043",
        }))
        .expect("body"),
    )
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// 一家 API 面的样本请求 (标签, 方法, 路径, 是否带 JSON 正文)。
type FaceSample = (&'static str, &'static str, &'static str, bool);

const CHAT_FACE: &[FaceSample] = &[
    ("native chat", "POST", "/v1/chat", true),
    ("openai chat", "POST", "/v1/chat/completions", true),
];
const EVENTS_FACE: &[FaceSample] = &[("events stream", "GET", "/v1/apeireth/events", false)];
const PANEL_FACE: &[FaceSample] = &[
    ("panel sessions", "GET", "/v1/panel/sessions", false),
    ("panel memory", "GET", "/v1/panel/memory/episodes", false),
    ("tools list", "GET", "/v1/tools/list", false),
];
const APPROVAL_FACE: &[FaceSample] = &[
    (
        "approval inbox",
        "GET",
        "/v1/approvals?session=00000000-0000-0000-0000-000000000042",
        false,
    ),
    ("approval resolve", "POST", "/v1/approvals/resolve", true),
];
const ADMIN_FACE: &[FaceSample] = &[
    ("admin config", "GET", "/v1/admin/config", false),
    ("admin shutdown", "POST", "/v1/admin/shutdown", false),
    (
        "session settings",
        "GET",
        "/v1/sessions/00000000-0000-0000-0000-000000000042/settings",
        false,
    ),
    ("models", "GET", "/v1/models", false),
];

/// 强制态矩阵: 一家 API 面的每个样本, 缺 Bearer = 401 `auth_missing_key`,
/// 伪造 Bearer = 401 `auth_invalid_key` (且 401 帧不回显任何令牌值)。
async fn assert_face_is_gated(harness: &Harness, face: &[FaceSample]) {
    let router = canonical_router_with_state(harness.state.clone());
    for &(label, method, uri, has_body) in face {
        let body = if has_body { json_body() } else { Body::empty() };
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(request(method, uri, None, body))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{label} 缺 Bearer 必须 401: {text}"
        );
        assert!(text.contains("auth_missing_key"), "{label}: {text}");

        let body = if has_body { json_body() } else { Body::empty() };
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(request(method, uri, Some(FORGED_TOKEN), body))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{label} 伪造 Bearer 必须 401: {text}"
        );
        assert!(text.contains("auth_invalid_key"), "{label}: {text}");
        assert!(!text.contains(TOKEN), "{label} 401 帧不得回显令牌: {text}");
        assert!(
            !text.contains(FORGED_TOKEN),
            "{label} 401 帧不得回显伪令牌: {text}"
        );
    }
}

// ---------------------------------------------------------------------------
// 1. 强制态 401 矩阵 (chat / 事件流 / 面板 / 审批 / admin 各一)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enforced_gate_refuses_unauthenticated_chat_face() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    assert_face_is_gated(&harness, CHAT_FACE).await;
}

#[tokio::test]
async fn enforced_gate_refuses_unauthenticated_events_stream() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    assert_face_is_gated(&harness, EVENTS_FACE).await;
}

#[tokio::test]
async fn enforced_gate_refuses_unauthenticated_panel_face() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    assert_face_is_gated(&harness, PANEL_FACE).await;
}

#[tokio::test]
async fn enforced_gate_refuses_unauthenticated_approval_face() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    assert_face_is_gated(&harness, APPROVAL_FACE).await;
}

#[tokio::test]
async fn enforced_gate_refuses_unauthenticated_admin_face() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    assert_face_is_gated(&harness, ADMIN_FACE).await;
}

/// 未匹配路径的 fallback 也在门内 (强制态下未知路径同样 401, 不借 404/401
/// 差别泄露路由存在性)。
#[tokio::test]
async fn enforced_gate_also_covers_unmatched_paths() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    let router = canonical_router_with_state(harness.state.clone());
    let (status, text) = status_and_body(
        router
            .oneshot(request(
                "GET",
                "/v1/definitely/not/a/route",
                None,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_missing_key"), "{text}");
}

// ---------------------------------------------------------------------------
// 2. 健康探活豁免 (探测刚需 + 显式豁免表)
// ---------------------------------------------------------------------------

/// `/health` 无认证可用 —— 三态口径下都不校验、不告警、不审计; 豁免面恰好
/// 只有这一条 (豁免表 = 认证面的权力边界)。
#[tokio::test]
async fn health_probe_stays_unauthenticated_in_every_gate_mode() {
    assert_eq!(
        UNAUTHENTICATED_PROBE_PATHS,
        &["/health"],
        "认证面唯一豁免 = 健康探活; 改这张表 = 改权力边界"
    );

    for (gate, clients) in [
        (GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])),
        (GatewayTokenGate::UnprotectedDefault, table(&[])),
        (GatewayTokenGate::UnprotectedDeclared, table(&[])),
        (
            GatewayTokenGate::Enforced(TOKEN.to_string()),
            table(&[("tui", "tok-tui")]),
        ),
    ] {
        let harness = harness(gate, clients).await;
        let mut frames = harness.state.events.subscribe();
        let router = canonical_router_with_state(harness.state.clone());
        let (status, _) = status_and_body(
            router
                .oneshot(request("GET", "/health", None, Body::empty()))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "探活必须无认证可用");

        assert!(
            frames.try_recv().is_err(),
            "探活不得触发警告帧 (探针会刷屏)"
        );
        harness.state.observations.flush().await;
        assert!(
            harness
                .audit
                .details_of("gateway.auth_unprotected")
                .is_empty(),
            "探活不得留放行审计: {:?}",
            harness.audit.details_of("gateway.auth_unprotected")
        );
        assert!(
            harness.audit.details_of("gateway.auth_client").is_empty(),
            "探活不进过门留痕"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. 现行为兼容路径 (未设口径无令牌零断) + 桌面自铸令牌路径零回归
// ---------------------------------------------------------------------------

/// 未设口径 (env 未设且令牌表为空) 的兼容路径: 无令牌客户端照旧进 handler
/// (不是 401), 但每次放行记警告帧 + 审计 —— 与扩面之前的口径逐字一致。
#[tokio::test]
async fn unset_gate_keeps_the_no_token_compat_path_working_with_warnings() {
    let harness = harness(GatewayTokenGate::UnprotectedDefault, table(&[])).await;
    let router = canonical_router_with_state(harness.state.clone());
    let mut frames = harness.state.events.subscribe();

    // 面板族: 无令牌调用进 handler (无面板后端 = 501 unsupported, 诚实降级)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request("GET", "/v1/panel/sessions", None, Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_IMPLEMENTED,
        "兼容路径必须进 handler, 不是 401"
    );

    // chat 族: 无令牌调用进 handler (无 provider = 503, 不是 401)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request("POST", "/v1/chat", None, json_body()))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::UNAUTHORIZED,
        "未设口径下无令牌 chat 不得 401"
    );

    // 每次放行: 警告帧 + 审计事实 (不再静默)。
    let frame = frames.try_recv().expect("warning frame");
    assert_eq!(frame.event, "gateway_auth_unprotected");
    assert_eq!(frame.data["mode"], "env_unset");
    harness.state.observations.flush().await;
    let details = harness.audit.details_of("gateway.auth_unprotected");
    assert!(
        details
            .iter()
            .any(|d| d.contains("/v1/panel/sessions") && d.contains("mode=env_unset")),
        "{details:?}"
    );
    assert!(
        details.iter().any(|d| d.contains("/v1/chat")),
        "{details:?}"
    );
}

/// 桌面自铸令牌路径零回归: 强制态下带令牌的调用全部进 handler
/// (状态码是业务语义, 永远不是 401/403), `/v1/admin/config` 照旧 200。
#[tokio::test]
async fn the_desktop_minted_token_path_keeps_reaching_every_handler() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    let router = canonical_router_with_state(harness.state.clone());

    // 敏感面 (W1-③ 既有口径): 带令牌读配置 = 200。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/admin/config",
                Some(TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "桌面管理面零回归");

    // chat / 面板 / 会话设置: 过门进 handler (业务状态码, 非鉴权失败)。
    for &(label, method, uri, has_body) in
        CHAT_FACE.iter().chain(PANEL_FACE).chain(&[ADMIN_FACE[2]])
    {
        let body = if has_body { json_body() } else { Body::empty() };
        let (status, _) = status_and_body(
            router
                .clone()
                .oneshot(request(method, uri, Some(TOKEN), body))
                .await
                .unwrap(),
        )
        .await;
        assert!(
            !matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN),
            "{label} 带令牌必须过门, got {status}"
        );
    }

    // 过门留痕带客户端名 (env 面单令牌 = `env`)。
    harness.state.observations.flush().await;
    let details = harness.audit.details_of("gateway.auth_client");
    assert!(
        details
            .iter()
            .any(|d| d.contains("client=env") && d.contains("/v1/admin/config")),
        "过门审计必须带客户端名: {details:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. 客户端令牌表 (单令牌族多枚 / 审计带名 / 表在场即强制)
// ---------------------------------------------------------------------------

/// 令牌表多枚各具名: 每一枚都可过门, 审计各自署名; env 单令牌仍以 `env` 署名。
#[tokio::test]
async fn every_named_client_token_passes_and_the_audit_names_the_client() {
    let harness = harness(
        GatewayTokenGate::Enforced(TOKEN.to_string()),
        table(&[
            ("tui", "tok-tui-0123456789abcdef"),
            ("desktop-shell", "tok-desktop-0123456789abcdef"),
            ("im-bridge", "tok-im-0123456789abcdef"),
        ]),
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());

    for (token, name) in [
        (TOKEN, "env"),
        ("tok-tui-0123456789abcdef", "tui"),
        ("tok-desktop-0123456789abcdef", "desktop-shell"),
        ("tok-im-0123456789abcdef", "im-bridge"),
    ] {
        let (status, _) = status_and_body(
            router
                .clone()
                .oneshot(request(
                    "GET",
                    "/v1/admin/config",
                    Some(token),
                    Body::empty(),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "客户端 {name} 的令牌必须过门");
    }

    // 表外伪令牌: 401, 不冒名。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/admin/config",
                Some("tok-stranger-0123456789abcdef"),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    harness.state.observations.flush().await;
    let details = harness.audit.details_of("gateway.auth_client");
    for name in ["env", "tui", "desktop-shell", "im-bridge"] {
        assert!(
            details
                .iter()
                .any(|d| d.contains(&format!("client={name}"))),
            "审计必须给 {name} 署名: {details:?}"
        );
    }
    // 审计只署名, 不带令牌值。
    for token in [TOKEN, "tok-tui-0123456789abcdef"] {
        assert!(
            details.iter().all(|d| !d.contains(token)),
            "审计不得携带令牌值: {details:?}"
        );
    }
}

/// 令牌表在场即强制: env 单令牌未设但表非空 = 只认表内具名令牌
/// (无令牌 401, 伪令牌 401, 正确令牌过门署名)。
#[tokio::test]
async fn a_configured_token_table_enforces_the_gate_even_without_the_env_token() {
    let harness = harness(
        GatewayTokenGate::UnprotectedDefault,
        table(&[("tui", "tok-tui-0123456789abcdef")]),
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("GET", "/v1/panel/sessions", None, Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "令牌族在场即强制: {text}");
    assert!(text.contains("auth_missing_key"), "{text}");

    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                Some(FORGED_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "伪造 Bearer 401");

    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                Some("tok-tui-0123456789abcdef"),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_IMPLEMENTED,
        "具名令牌过门进 handler"
    );

    // 强制路径不发警告帧 (警告帧只属于未设口径)。
    harness.state.observations.flush().await;
    assert!(
        harness
            .audit
            .details_of("gateway.auth_unprotected")
            .is_empty(),
        "强制路径不得记未设痕"
    );
    let details = harness.audit.details_of("gateway.auth_client");
    assert!(
        details.iter().any(|d| d.contains("client=tui")),
        "{details:?}"
    );
}

// ---------------------------------------------------------------------------
// 5. 对抗面 (敌人③: 伪来源头 / 无令牌调 chat / 伪造 Bearer)
// ---------------------------------------------------------------------------

/// 敌人③绕过三连: 伪来源头拿不到 CORS 放行, 无令牌调 chat 401, 伪造 Bearer
/// 401 —— 三者同时成立 (伪来源头 + 伪令牌的组合也一样被拒)。
#[tokio::test]
async fn forged_origin_and_forged_bearer_are_refused_together() {
    let harness = harness(GatewayTokenGate::Enforced(TOKEN.to_string()), table(&[])).await;
    let router = canonical_router_with_state(harness.state.clone());

    for origin in [
        "https://evil.example.com",
        "http://tauri.localhost.evil.com",
        "http://127.0.0.1.evil.example",
        "null",
    ] {
        for (method, uri, body) in [
            ("POST", "/v1/chat/completions", json_body()),
            ("GET", "/v1/panel/sessions", Body::empty()),
        ] {
            let mut request = request(method, uri, Some(FORGED_TOKEN), body);
            request
                .headers_mut()
                .insert("origin", origin.parse().expect("origin"));
            let response = router.clone().oneshot(request).await.unwrap();
            let allow_origin = response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_string();
            assert!(
                allow_origin.is_empty(),
                "伪来源头 {origin} 不得拿到 CORS 放行: {allow_origin}"
            );
            let (status, text) = status_and_body(response).await;
            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "伪来源头 + 伪 Bearer 也不得过门: {text}"
            );
            assert!(text.contains("auth_invalid_key"), "{text}");
        }
    }

    // 本地桌面来源照旧放行 (桌面壳 UI 的 CORS 面零回归)。
    let mut request = request("GET", "/v1/admin/config", Some(TOKEN), Body::empty());
    request
        .headers_mut()
        .insert("origin", "http://tauri.localhost".parse().expect("origin"));
    let response = router.oneshot(request).await.unwrap();
    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(
        allow_origin.contains("tauri.localhost") || allow_origin == "*",
        "本地来源必须继续放行: {allow_origin}"
    );
}

/// 逃生阀 (`none`) 的扩面语义: 全 API 面放行、不逐次告警、每次放行留审计痕。
#[tokio::test]
async fn the_escape_valve_traces_every_pass_across_the_whole_face() {
    let harness = harness(GatewayTokenGate::UnprotectedDeclared, table(&[])).await;
    let router = canonical_router_with_state(harness.state.clone());
    let mut frames = harness.state.events.subscribe();

    for (label, method, uri, has_body) in CHAT_FACE.iter().chain(PANEL_FACE) {
        let (label, method, uri, has_body) = (*label, *method, *uri, *has_body);
        let body = if has_body { json_body() } else { Body::empty() };
        let (status, _) = status_and_body(
            router
                .clone()
                .oneshot(request(method, uri, None, body))
                .await
                .unwrap(),
        )
        .await;
        assert_ne!(status, StatusCode::UNAUTHORIZED, "{label} 逃生阀放行");
    }

    assert!(frames.try_recv().is_err(), "逃生阀不逐次告警");
    harness.state.observations.flush().await;
    let details = harness.audit.details_of("gateway.auth_unprotected");
    assert!(
        details
            .iter()
            .any(|d| d.contains("/v1/chat") && d.contains("mode=declared_none")),
        "{details:?}"
    );
    assert!(
        details
            .iter()
            .any(|d| d.contains("/v1/panel/sessions") && d.contains("mode=declared_none")),
        "{details:?}"
    );
}
