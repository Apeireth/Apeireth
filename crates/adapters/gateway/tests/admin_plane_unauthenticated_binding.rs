//! F-2 (2026-11-05): **未认证身份**不得被用户表绑定成 owner。
//!
//! # 缺陷 (真实进程实测)
//!
//! 未认证 / 逃生阀路径在令牌门中间件里插入的是 `ClientIdentity::anonymous()`
//! (`crates/adapters/gateway/src/client_tokens.rs` 的 `ClientIdentity::anonymous()`,
//! 插入点 = 令牌门中间件的未设防放行分支) ⇒ ⇒ 用户表**可以**写 `anonymous`
//! 这个客户端名 ⇒ ⇒ `resolve_user_identity` 的第二步回落
//! `users.identity_for_client("anonymous")` 命中 ⇒ ⇒ **未证明身份被当成 owner**。
//!
//! 实测 (真进程 · 真 socket): `apeireth user add --role owner --clients anonymous`
//! ⇒ ⇒ **零凭据** `POST /v1/admin/config` ⇒ **200** ⇒ `base_url` 被改成攻击者主机
//! ⇒ ⇒ 下一次模型调用把**真实 API key** 发过去 ⇒ ⇒ P0-1 的原始威胁。
//!
//! # 本文件钉住的口径
//!
//! 1. **未认证 + owner 绑 `anonymous` ⇒ 管理面 403** (核心);
//! 2. 未认证调用方的读面/写面与「**没绑任何用户**」的调用方**逐字一致**
//!    (未认证身份 ≡ 未绑定身份 ⇒ 同一拒因族、同一帧 ⇒ **零回归**);
//! 3. **已认证** (env 单令牌面, 署名名 `env`) + owner 绑 `env` ⇒ 管理面**照旧 200**;
//! 4. **已认证** + owner 绑 `anonymous` ⇒ 仍 403 (署名键是**令牌绑定名**, 不是自称);
//! 5. **已认证** (客户端令牌表条目) + owner 绑该条目名 ⇒ 管理面**照旧 200**。
//!
//! 全部走**真实 HTTP** (`tower::ServiceExt::oneshot` 打真实路由)。
//!
//! ⚠️ **写面口径说明** (免得后人当成 bug): 未认证调用方打写面 (`POST /v1/chat`)
//! 是 **403** —— 这正是「**未绑定用户**」早已有的那道裁决 (W3-② fail-closed,
//! 另见 `user_authorization.rs` 的 `an_unbound_client_is_refused_on_gated_faces_fail_closed`)
//! ⇒ ⇒ 本次修复**没有新增**拒绝面, 只是拿掉了「未证明身份被当成 owner」这条非法提权。

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
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// env 单令牌面的令牌 (形似凭据以便泄漏显形, 非真实秘密)。
const ENV_TOKEN: &str = "env-token-0123456789abcdef";
/// 客户端令牌表条目的令牌。
const TABLE_TOKEN: &str = "table-token-0123456789abcdef";

/// 客户端令牌表条目的名字 (署名键 = 条目名)。
const TABLE_CLIENT: &str = "desktop-shell";
/// env 单令牌面的署名名。
const ENV_CLIENT: &str = GatewayTokenGate::ENV_FACE_CLIENT_NAME;

const OWNER_USER: &str = "user-owner";
/// 未设防口径下过门的署名名 (令牌门插入 `ClientIdentity::anonymous()`)。
const ANONYMOUS: &str = "anonymous";

/// 攻击者的端点 (F-2 实测: 这里曾经被零凭据改成攻击者主机)。
const ATTACKER_BASE_URL: &str = "https://attacker.example/v1";

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

/// 一份网关装配 (显式门口径 + 显式令牌表 + 显式角色面, 不动进程环境)。
async fn harness(gate: GatewayTokenGate, clients: ClientTokenTable, users: UserTable) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    state.client_tokens = clients;
    state.user_access = UserAccess::from_table(users);
    Harness { state, audit }
}

/// 空令牌表 (逃生阀口径: 令牌表与逃生阀**互斥**, 配置矛盾拒开)。
fn empty_tokens() -> ClientTokenTable {
    ClientTokenTable::from_config(ClientTokenConfig::new(vec![])).expect("valid token table")
}

/// 客户端令牌表: 一条具名条目 (令牌绑定名 = 署名键)。
fn table_tokens() -> ClientTokenTable {
    ClientTokenTable::from_config(ClientTokenConfig::new(vec![ClientTokenSpec {
        name: TABLE_CLIENT.to_string(),
        token: TABLE_TOKEN.to_string(),
        user_id: Some(OWNER_USER.to_string()),
    }]))
    .expect("valid token table")
}

/// 一名 owner, 绑定给定的客户端名。
fn owner_bound_to(client_names: &[&str]) -> UserTable {
    let users = UserTable::new(vec![UserRecord {
        user_id: OWNER_USER.to_string(),
        role: "owner".to_string(),
        client_names: client_names
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        domain: None,
    }]);
    users.validate().expect("valid user table");
    users
}

/// 一个请求 (可选带 Bearer 令牌; `body` 给 `None` ⇒ 无正文)。
fn request(
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer"),
        );
    }
    if let Some(body) = body {
        request.headers_mut().insert(
            "content-type",
            "application/json".parse().expect("content type"),
        );
        *request.body_mut() = Body::from(body.to_string());
    }
    request
}

/// 打一发真实 HTTP, 取回 (状态码, 正文)。
async fn call(
    router: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String) {
    let response = router
        .clone()
        .oneshot(request(method, uri, token, body))
        .await
        .expect("router responds");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// 一帧正文里的**拒因码** (没有错误帧 ⇒ `None`)。
fn error_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value["error"]["code"].as_str().map(str::to_string))
}

/// 403 拒绝帧的公共断言: 码是 `permission_denied`, 且零令牌泄漏。
async fn assert_denied(response: axum::http::Response<Body>, label: &str) -> String {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    assert_eq!(status, StatusCode::FORBIDDEN, "{label} 必须 403: {text}");
    assert!(text.contains("permission_denied"), "{label}: {text}");
    for token in [ENV_TOKEN, TABLE_TOKEN] {
        assert!(!text.contains(token), "{label} 403 帧不得回显令牌: {text}");
    }
    text
}

// ---------------------------------------------------------------------------
// 用例 1: 未认证 + owner 绑 `anonymous` ⇒ 管理面 fail-closed
// ---------------------------------------------------------------------------

/// ⭐ **未认证 + owner 绑 `anonymous` ⇒ 管理面 403** (F-2 核心)。
///
/// 钉住: (a) 零凭据打管理面写口 = 403 `permission_denied`, 帧里说清
/// 「未绑定任何用户」; (b) **攻击者的 `base_url` 没落进热配置** (那正是
/// 「真实 API key 被发往攻击者主机」的根); (c) **停机端点也拒** (⇒ 逃生阀下
/// 不存在「绑 `anonymous` 就能停机」); (d) 审计留痕 `user=unbound` —— 未认证
/// 身份**不得**被记成 owner。
#[tokio::test]
async fn an_unauthenticated_caller_bound_to_anonymous_cannot_reach_the_admin_plane() {
    // 逃生阀 (`APEIRETH_GATEWAY_TOKEN=none`): 未设防口径 ⇒ 过门身份是
    // `ClientIdentity::anonymous()` (未认证)。
    let harness = harness(
        GatewayTokenGate::UnprotectedDeclared,
        empty_tokens(),
        owner_bound_to(&[ANONYMOUS]),
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());
    let base_url_before = harness.state.hot_config.read().unwrap().base_url.clone();

    // 零凭据打管理面写口 (F-2 实测: 这里曾经 200)。
    let text = assert_denied(
        router
            .clone()
            .oneshot(request(
                "POST",
                "/v1/admin/config",
                None,
                Some(serde_json::json!({ "base_url": ATTACKER_BASE_URL })),
            ))
            .await
            .unwrap(),
        "POST /v1/admin/config (零凭据)",
    )
    .await;
    assert!(
        text.contains("not bound to any user"),
        "拒绝帧必须说清「未绑定任何用户」这条真因: {text}"
    );

    // ⇒ ⇒ 攻击者的端点**没落进**热配置 (真实 API key 不会被发往攻击者主机)。
    let base_url_after = harness.state.hot_config.read().unwrap().base_url.clone();
    assert_eq!(
        base_url_after, base_url_before,
        "base_url 不得被零凭据请求改成攻击者主机"
    );
    assert_ne!(base_url_after, ATTACKER_BASE_URL);

    // 管理面其余端点 + 面板族管理面: 同一裁决口径 ⇒ 一律 403。
    for (method, uri) in [
        ("GET", "/v1/admin/config"),
        ("POST", "/v1/admin/shutdown"),
        ("POST", "/v1/admin/users"),
        ("GET", "/v1/panel/users"),
    ] {
        let body = if method == "POST" {
            Some(serde_json::json!({}))
        } else {
            None
        };
        let response = router
            .clone()
            .oneshot(request(method, uri, None, body))
            .await
            .unwrap();
        assert_denied(response, &format!("{method} {uri}")).await;
    }

    // ⇒ ⇒ 停机触发器**没被拉下** (逃生阀下没有「绑 anonymous 就能停机」)。
    assert!(
        !harness.state.shutdown.is_triggered(),
        "零凭据不得触发优雅停机"
    );

    // 审计留痕: 一行 `gateway.user_action`, `user=unbound` + `decision=deny`
    // ⇒ ⇒ 未认证身份**不得**被记成 owner。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|row| {
            row.contains("user=unbound")
                && row.contains("action=admin_control")
                && row.contains("endpoint=/v1/admin/config")
                && row.contains("decision=deny")
        }),
        "管理面拒绝必须留审计痕: {rows:?}"
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.contains(&format!("user={OWNER_USER}"))),
        "未认证身份不得被记成 owner: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 用例 2: 读面/写面零回归 (未认证身份 ≡ 未绑定身份)
// ---------------------------------------------------------------------------

/// ⭐ **未认证调用方的读面/写面逐字不变** (零回归)。
///
/// 「逐字不变」的**可比基准** = 「**没绑任何用户**」的调用方 (同一门口径下
/// 另一个 owner 绑定名)。⇒ ⇒ 本次修复把未认证身份**归入**未绑定身份这一族,
/// 于是两者的读面/写面/管理面**逐字一致** ⇒ ⇒ **没有新增拒绝面**。
///
/// 钉住: (a) 读面 (`GET /v1/providers`) 照旧 200 (零读面 ≠ 零读面);
/// (b) 写面 (`POST /v1/chat`) 与未绑定基准**逐字一致** (W3-② 既有裁决);
/// (c) 管理面与未绑定基准**逐字一致** (同一拒因族);
/// (d) **已认证**调用方的读面/写面**照旧放行** (不是 403) —— 修复没把正常
/// 路径一起堵死。
#[tokio::test]
async fn the_read_and_write_faces_keep_their_unbound_verdicts() {
    // 基准: 未认证调用方, 但 owner 绑的是**别的**客户端名 ⇒ 调用方未绑定任何用户。
    let baseline = harness(
        GatewayTokenGate::UnprotectedDeclared,
        empty_tokens(),
        owner_bound_to(&[TABLE_CLIENT]),
    )
    .await;
    let baseline_router = canonical_router_with_state(baseline.state.clone());

    // 被测: 未认证调用方, owner 绑 `anonymous` (F-2 的那条绑定)。
    let bound = harness(
        GatewayTokenGate::UnprotectedDeclared,
        empty_tokens(),
        owner_bound_to(&[ANONYMOUS]),
    )
    .await;
    let bound_router = canonical_router_with_state(bound.state.clone());

    // (a) 读面照旧 200。
    let (status, text) = call(&bound_router, "GET", "/v1/providers", None, None).await;
    assert_eq!(status, StatusCode::OK, "读面必须照旧可达: {text}");

    // (b)+(c) 写面与管理面: 与「没绑任何用户」的基准**逐字一致**。
    for (method, uri, body) in [
        (
            "POST",
            "/v1/chat",
            Some(serde_json::json!({ "input": "hello" })),
        ),
        ("GET", "/v1/admin/config", None),
        (
            "POST",
            "/v1/admin/config",
            Some(serde_json::json!({ "model": "m" })),
        ),
    ] {
        let (expected_status, expected_body) =
            call(&baseline_router, method, uri, None, body.clone()).await;
        let (actual_status, actual_body) = call(&bound_router, method, uri, None, body).await;
        assert_eq!(
            actual_status, expected_status,
            "{method} {uri}: 未认证身份的状态码必须与「没绑任何用户」的基准一致"
        );
        assert_eq!(
            error_code(&actual_body),
            error_code(&expected_body),
            "{method} {uri}: 未认证身份的拒因码必须与「没绑任何用户」的基准一致"
        );
    }

    // 管理面 (正文确定): 与「没绑任何用户」的基准**逐字一致**。
    for (method, uri, body) in [
        ("GET", "/v1/admin/config", None),
        (
            "POST",
            "/v1/admin/config",
            Some(serde_json::json!({ "model": "m" })),
        ),
    ] {
        let (_, expected_body) = call(&baseline_router, method, uri, None, body.clone()).await;
        let (_, actual_body) = call(&bound_router, method, uri, None, body).await;
        assert_eq!(
            actual_body, expected_body,
            "{method} {uri}: 未认证身份必须与「没绑任何用户」的基准逐字一致"
        );
    }

    // 写面的具体裁决: 与「未绑定用户」的既有口径同族 (W3-② fail-closed)。
    let (status, text) = call(
        &bound_router,
        "POST",
        "/v1/chat",
        None,
        Some(serde_json::json!({ "input": "hello" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "未认证调用方的写面裁决必须与未绑定用户同族: {text}"
    );
    assert!(
        text.contains("permission_denied") && text.contains("not bound to any user"),
        "写面拒绝帧必须说清「未绑定任何用户」: {text}"
    );

    // (d) 已认证调用方 (env 单令牌面): 读面/写面**照旧放行** ⇒ 零回归没被破坏。
    let owner = harness(
        GatewayTokenGate::Enforced(ENV_TOKEN.to_string()),
        empty_tokens(),
        owner_bound_to(&[ENV_CLIENT]),
    )
    .await;
    let owner_router = canonical_router_with_state(owner.state.clone());
    let (status, text) = call(&owner_router, "GET", "/v1/providers", Some(ENV_TOKEN), None).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "已认证调用方的读面必须照旧可达: {text}"
    );
    let (status, text) = call(
        &owner_router,
        "POST",
        "/v1/chat",
        Some(ENV_TOKEN),
        Some(serde_json::json!({ "input": "hello" })),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "已认证调用方的写面不得被拒 (修复没把正常路径堵死): {text}"
    );
}

// ---------------------------------------------------------------------------
// 用例 3: 已认证 (env 单令牌面) + owner 绑 `env` ⇒ 管理面照旧 200
// ---------------------------------------------------------------------------

/// ⭐ **已认证 (env 单令牌面, 署名名 `env`) + owner 绑 `env` ⇒ 管理面照旧 200**。
///
/// 钉住: 修复**没有把正常路径一起堵死** —— 已认证的 owner 照旧能读写管理面;
/// 审计留痕 `user=<owner> role=owner decision=allow`。
#[tokio::test]
async fn an_authenticated_env_face_owner_still_reaches_the_admin_plane() {
    let harness = harness(
        GatewayTokenGate::Enforced(ENV_TOKEN.to_string()),
        empty_tokens(),
        owner_bound_to(&[ENV_CLIENT]),
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());

    // 管理面读口: owner 全权 ⇒ 200。
    let (status, text) = call(&router, "GET", "/v1/admin/config", Some(ENV_TOKEN), None).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "已认证 owner 的管理面读必须 200: {text}"
    );

    // 管理面写口: owner 全权 ⇒ 200 (热配置照旧生效)。
    let (status, text) = call(
        &router,
        "POST",
        "/v1/admin/config",
        Some(ENV_TOKEN),
        Some(serde_json::json!({ "model": "model-b" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "已认证 owner 的管理面写必须 200: {text}"
    );
    assert!(text.contains("\"ok\":true"), "{text}");

    // 审计留痕: allow + 真用户真角色。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|row| {
            row.contains(&format!("user={OWNER_USER}"))
                && row.contains("role=owner")
                && row.contains("action=admin_control")
                && row.contains("endpoint=/v1/admin/config")
                && row.contains("decision=allow")
        }),
        "已认证 owner 的放行必须留审计痕: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 用例 4: 已认证 + owner 绑 `anonymous` ⇒ 仍 403
// ---------------------------------------------------------------------------

/// ⭐ **已认证 + owner 绑 `anonymous` ⇒ 管理面仍 403**。
///
/// 钉住: 匿名名**不能**因为「调用方已认证」就变成 owner —— 署名键是**令牌
/// 绑定名** (`env` 面单令牌 ⇒ `env`), 不是用户表里的自称/别名。
/// ⇒ ⇒ 「把 `anonymous` 绑给 owner」这条捷径在**任何**门口径下都不成立。
#[tokio::test]
async fn an_authenticated_caller_bound_to_anonymous_is_not_the_owner() {
    let harness = harness(
        GatewayTokenGate::Enforced(ENV_TOKEN.to_string()),
        empty_tokens(),
        owner_bound_to(&[ANONYMOUS]),
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());

    for (method, uri) in [
        ("GET", "/v1/admin/config"),
        ("POST", "/v1/admin/config"),
        ("POST", "/v1/admin/shutdown"),
    ] {
        let body = if method == "POST" {
            Some(serde_json::json!({ "base_url": ATTACKER_BASE_URL }))
        } else {
            None
        };
        let response = router
            .clone()
            .oneshot(request(method, uri, Some(ENV_TOKEN), body))
            .await
            .unwrap();
        let text = assert_denied(response, &format!("{method} {uri}")).await;
        assert!(
            text.contains("not bound to any user"),
            "{method} {uri}: 拒绝帧必须说清「未绑定任何用户」: {text}"
        );
    }

    // 审计: 已认证但**未绑定** ⇒ `user=unbound` (不是 owner)。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|row| {
            row.contains("user=unbound")
                && row.contains("action=admin_control")
                && row.contains("decision=deny")
        }),
        "已认证但未绑定的调用方必须留 deny 痕: {rows:?}"
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.contains(&format!("user={OWNER_USER}"))),
        "已认证调用方不得因为 owner 绑了 `anonymous` 就被记成 owner: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 用例 5: 已认证 (客户端令牌表条目) ⇒ 管理面照旧 200
// ---------------------------------------------------------------------------

/// ⭐ **已认证 (客户端令牌表条目) + owner 绑该条目名 ⇒ 管理面照旧 200**。
///
/// 钉住: 已认证路径的**两面** (env 单令牌面 + 客户端令牌表) 都**照旧可用**
/// ⇒ ⇒ 修复只收「未认证身份被绑定成 owner」这条口子, 没碰已认证路径。
#[tokio::test]
async fn an_authenticated_client_token_owner_still_reaches_the_admin_plane() {
    let harness = harness(
        GatewayTokenGate::Enforced(ENV_TOKEN.to_string()),
        table_tokens(),
        owner_bound_to(&[TABLE_CLIENT]),
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());

    let (status, text) = call(&router, "GET", "/v1/admin/config", Some(TABLE_TOKEN), None).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "客户端令牌表条目的 owner 管理面读必须 200: {text}"
    );

    let (status, text) = call(
        &router,
        "POST",
        "/v1/admin/config",
        Some(TABLE_TOKEN),
        Some(serde_json::json!({ "model": "model-c" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "客户端令牌表条目的 owner 管理面写必须 200: {text}"
    );
    assert!(text.contains("\"ok\":true"), "{text}");
}
