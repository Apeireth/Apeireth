//! P0-1 (2026-11-04): 网关**管理面 fail-closed**。
//!
//! # 背景 (静态安全审计复核)
//!
//! `UserAccess::is_active()` = `users.is_some()` —— 未注入用户表 ⇒ **角色面未
//! 激活** ⇒ 角色门 (`require_user_capability`) 逐字放行 ⇒ 管理面端点
//! (`/v1/admin/*` + 面板族管理面) 在**零认证**的默认态下对本机任何进程可达。
//!
//! 管理面能热改 provider `base_url` (只校验「https 或环回 http」, **无 host
//! allowlist**) ⇒ 下一次模型调用把**真实 API key** 发往任意主机 ⇒ 管理面的
//! 风险等级与普通端点**不同** ⇒ 「零回归」口径不适用于**能外泄凭据的面**。
//!
//! # 裁决 (总指挥拍板)
//!
//! * 角色面**未激活** ⇒ 要求 `UserCapability::AdminControl` 的端点**一律
//!   403** (`permission_denied`) + 一行 `gateway.user_action` 审计留痕;
//! * 角色面**已激活** ⇒ 照旧走治理层 role 权限矩阵 (owner 照旧可用);
//! * 其余端点 (读面 / 写面) **逐字不变** (零回归兼容路径不动)。
//!
//! 这里用**真实 HTTP** (`tower::ServiceExt::oneshot` 打真实路由) 钉这三条。

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand,
    ClientTokenConfig, ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayState,
    GatewayTokenGate, UserAccess, USER_ACTION_AUDIT_EVENT,
};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// 测试令牌 (形似凭据以便泄漏显形, 非真实秘密)。
const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const OWNER_TOKEN: &str = "owner-token-0123456789abcdef";

const OWNER_CLIENT: &str = "desktop-shell";
const OWNER_USER: &str = "user-owner";

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

/// 一份网关装配 (显式门口径 + 显式令牌表 + **可选**角色面, 不动进程环境)。
///
/// `users = None` ⇒ **角色面未激活** (兼容路径); `Some(table)` ⇒ 角色面激活。
async fn harness(users: Option<UserTable>) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens =
        ClientTokenTable::from_config(ClientTokenConfig::new(vec![ClientTokenSpec {
            name: OWNER_CLIENT.to_string(),
            token: OWNER_TOKEN.to_string(),
            user_id: Some(OWNER_USER.to_string()),
        }]))
        .expect("valid token table");
    state.user_access = match users {
        Some(table) => UserAccess::from_table(table),
        None => UserAccess::inactive(),
    };
    Harness { state, audit }
}

/// 一名 owner (绑定 `desktop-shell` 客户端名) 的用户表。
fn owner_table() -> UserTable {
    let users = UserTable::new(vec![UserRecord {
        user_id: OWNER_USER.to_string(),
        role: "owner".to_string(),
        client_names: vec![OWNER_CLIENT.to_string()],
        domain: None,
    }]);
    users.validate().expect("valid user table");
    users
}

fn request(method: &str, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {OWNER_TOKEN}"))
        .body(body)
        .expect("request")
}

fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {OWNER_TOKEN}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// 403 拒绝帧的公共断言: 码是 `permission_denied`, 且零令牌泄漏。
async fn assert_denied(response: axum::http::Response<Body>, label: &str) -> String {
    let (status, text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{label} 必须 403: {text}");
    assert!(text.contains("permission_denied"), "{label}: {text}");
    assert!(
        !text.contains(OWNER_TOKEN),
        "{label} 403 帧不得回显令牌: {text}"
    );
    text
}

// ---------------------------------------------------------------------------
// 用例 1: 角色面未激活 ⇒ 管理面 fail-closed
// ---------------------------------------------------------------------------

/// **角色面未激活 ⇒ 管理面 403**。
///
/// 钉住: (a) 状态码是 403 `permission_denied`; (b) 帧里**说清原因**
/// (「角色面未激活」而不是笼统的「not allowed」); (c) 审计留痕一行
/// `gateway.user_action` (`reason=user_role_surface_inactive`)。
#[tokio::test]
async fn the_admin_plane_is_fail_closed_while_the_role_surface_is_inactive() {
    let harness = harness(None).await;
    let router = canonical_router_with_state(harness.state.clone());

    // 管理面写口 (能热改 provider base_url ⇒ 能外泄 API key 的那一面)。
    let text = assert_denied(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/config",
                serde_json::json!({ "base_url": "https://attacker.example/v1" }),
            ))
            .await
            .unwrap(),
        "POST /v1/admin/config",
    )
    .await;
    assert!(
        text.contains("role surface is not active"),
        "拒绝帧必须说清「角色面未激活」这条真因: {text}"
    );

    // 管理面读口同样 fail-closed (同一族端点, 判定口径一致)。
    let text = assert_denied(
        router
            .clone()
            .oneshot(request("GET", "/v1/admin/config", Body::empty()))
            .await
            .unwrap(),
        "GET /v1/admin/config",
    )
    .await;
    assert!(
        text.contains("role surface is not active"),
        "拒绝帧必须说清「角色面未激活」这条真因: {text}"
    );

    // admin 族其余端点 + 面板族管理面 (同一裁决口径)。
    for (method, uri) in [
        ("POST", "/v1/admin/shutdown"),
        ("POST", "/v1/admin/users"),
        ("GET", "/v1/panel/users"),
        ("GET", "/v1/panel/domains"),
    ] {
        let body = if method == "POST" {
            json_request(method, uri, serde_json::json!({}))
        } else {
            request(method, uri, Body::empty())
        };
        let response = router.clone().oneshot(body).await.unwrap();
        assert_denied(response, &format!("{method} {uri}")).await;
    }

    // 审计留痕: 一行 `gateway.user_action`, 带端点与真因。
    harness.state.observations.flush().await;
    let details = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        details
            .iter()
            .any(|d| d.contains("endpoint=/v1/admin/config")
                && d.contains("reason=user_role_surface_inactive")),
        "管理面拒绝必须留审计痕: {details:?}"
    );
}

// ---------------------------------------------------------------------------
// 用例 2: 角色面未激活 ⇒ 其它端点逐字不变 (零回归)
// ---------------------------------------------------------------------------

/// **角色面未激活 ⇒ 读面 / 写面照旧放行** (零回归兼容路径不动)。
///
/// 这一条是本次裁决的**另一半**: 只收口管理面, 其余端点的行为逐字不变。
#[tokio::test]
async fn the_read_and_write_faces_stay_open_while_the_role_surface_is_inactive() {
    let harness = harness(None).await;
    let router = canonical_router_with_state(harness.state.clone());

    // 读面: 放行 (业务状态码, 不是鉴权失败)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("GET", "/v1/providers", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "读面必须照旧可达: {text}");
    assert_ne!(status, StatusCode::FORBIDDEN, "{text}");

    // 读面 (`/v1/models`): 未配置 provider ⇒ 业务 401 (缺 key), 但**不是**
    // 角色门拒绝 —— 角色面未激活不得给它新增拒绝面。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("GET", "/v1/models", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(status, StatusCode::FORBIDDEN, "读面不得被角色门拒: {text}");

    // 写面 (`/v1/chat`): 角色面未激活 ⇒ 写面**维持现状** (仍放行; 无 provider
    // ⇒ 业务 503, 不是 403)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/chat",
                serde_json::json!({ "input": "hello" }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "写面维持现状 (仍放行): {text}"
    );
}

// ---------------------------------------------------------------------------
// 用例 3: 角色面已激活 ⇒ owner 照旧可用
// ---------------------------------------------------------------------------

/// **角色面已激活 + owner ⇒ 管理面照旧可用** (裁决没有把管理面焊死)。
#[tokio::test]
async fn an_owner_reaches_the_admin_plane_once_the_role_surface_is_active() {
    let harness = harness(Some(owner_table())).await;
    let router = canonical_router_with_state(harness.state.clone());

    // 管理面读口: owner 全权 ⇒ 200。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("GET", "/v1/admin/config", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner 管理面读必须 200: {text}");

    // 管理面写口: owner 全权 ⇒ 200 (热配置照旧生效)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/config",
                serde_json::json!({ "model": "model-b" }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner 管理面写必须 200: {text}");
    assert!(text.contains("\"ok\":true"), "{text}");

    // 角色面激活 ⇒ 拒绝面回到治理层 role 矩阵 (owner 不被误拒)。
    harness.state.observations.flush().await;
    let details = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        !details
            .iter()
            .any(|d| d.contains("reason=user_role_surface_inactive")),
        "角色面激活时不得再按「角色面未激活」拒: {details:?}"
    );
}
