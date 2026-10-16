//! W3-② 用户身份与角色: 角色门 (令牌→用户→角色) 的面契约与对抗面。
//!
//! 这里钉四件事:
//! 1. **角色面未激活 = 兼容路径** —— 未注入用户表时 W2-② 行为逐字不变
//!    (零回归口径, 不新增拒绝面);
//! 2. **role 权限矩阵在网关面逐档生效** —— owner/member/guest × 写面/管理面/
//!    审批面 (guest 零写面, member 无管理面, owner 全权);
//! 3. **对抗面 (敌人⑤ 多户越权)** —— 成员冒名 owner 拒 / guest 越权写拒 /
//!    跨用户审批拒 / 未绑定客户端 fail-closed;
//! 4. **`gateway.user_action` 审计行** —— user/role/action 落账, allow 与 deny
//!    都留痕, 令牌值零泄漏。

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand,
    ClientTokenConfig, ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayState,
    GatewayTokenGate, UserAccess, CLIENT_NAME_CLAIM_HEADER, USER_ACTION_AUDIT_EVENT,
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
const MEMBER_TOKEN: &str = "member-token-0123456789abcdef";
const MEMBER_B_TOKEN: &str = "member-b-token-0123456789abcdef";
const GUEST_TOKEN: &str = "guest-token-0123456789abcdef";
const STRANGER_TOKEN: &str = "stranger-token-0123456789abcdef";

const OWNER_CLIENT: &str = "desktop-shell";
const MEMBER_CLIENT: &str = "tui";
const MEMBER_B_CLIENT: &str = "tui-b";
const GUEST_CLIENT: &str = "phone-web";
const STRANGER_CLIENT: &str = "stranger";

const ALL_TOKENS: &[&str] = &[
    GATE_TOKEN,
    OWNER_TOKEN,
    MEMBER_TOKEN,
    MEMBER_B_TOKEN,
    GUEST_TOKEN,
    STRANGER_TOKEN,
];

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
async fn harness(
    gate: GatewayTokenGate,
    clients: ClientTokenTable,
    users: Option<UserTable>,
) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    state.client_tokens = clients;
    state.user_access = match users {
        Some(table) => UserAccess::from_table(table),
        None => UserAccess::inactive(),
    };
    Harness { state, audit }
}

fn spec(name: &str, token: &str, user_id: Option<&str>) -> ClientTokenSpec {
    ClientTokenSpec {
        name: name.to_string(),
        token: token.to_string(),
        user_id: user_id.map(str::to_string),
    }
}

fn record(user_id: &str, role: &str, clients: &[&str]) -> UserRecord {
    UserRecord {
        user_id: user_id.to_string(),
        role: role.to_string(),
        client_names: clients.iter().map(|c| (*c).to_string()).collect(),
        domain: None,
    }
}

/// 四用户 / 五令牌的标准夹具 (owner + 两 member + guest + 未绑定客户端)。
async fn fixture() -> Harness {
    let clients = ClientTokenTable::from_config(ClientTokenConfig::new(vec![
        spec(OWNER_CLIENT, OWNER_TOKEN, Some("user-owner")),
        spec(MEMBER_CLIENT, MEMBER_TOKEN, Some("user-member-a")),
        spec(MEMBER_B_CLIENT, MEMBER_B_TOKEN, Some("user-member-b")),
        spec(GUEST_CLIENT, GUEST_TOKEN, Some("user-guest")),
        spec(STRANGER_CLIENT, STRANGER_TOKEN, None),
    ]))
    .expect("valid token table");
    let users = UserTable::new(vec![
        record("user-owner", "owner", &[OWNER_CLIENT]),
        record("user-member-a", "member", &[MEMBER_CLIENT]),
        record("user-member-b", "member", &[MEMBER_B_CLIENT]),
        record("user-guest", "guest", &[GUEST_CLIENT]),
    ]);
    users.validate().expect("valid user table");
    harness(
        GatewayTokenGate::Enforced(GATE_TOKEN.to_string()),
        clients,
        Some(users),
    )
    .await
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

/// 带 JSON 正文的请求 (axum `Json` 提取器要求 content-type)。
fn json_request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = request(method, uri, token, body);
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    request
}

fn json_body() -> Body {
    Body::from(
        serde_json::to_vec(&serde_json::json!({
            "input": "hello",
            "session": "00000000-0000-0000-0000-000000000051",
        }))
        .expect("body"),
    )
}

fn approval_body(session: &str) -> Body {
    Body::from(
        serde_json::to_vec(&serde_json::json!({
            "session": session,
            "approval": "00000000-0000-0000-0000-000000000052",
            "decision": "approve",
        }))
        .expect("body"),
    )
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
    for token in ALL_TOKENS {
        assert!(!text.contains(token), "{label} 403 帧不得回显令牌: {text}");
    }
    text
}

// ---------------------------------------------------------------------------
// 1. 角色面未激活 = 兼容路径 (零回归口径)
// ---------------------------------------------------------------------------

/// 未注入用户表: 角色门整体休眠 —— 写面照旧进 handler (不是 403), 且
/// 不产生任何 `gateway.user_action` 审计行 (W2-② 行为逐字不变)。
#[tokio::test]
async fn without_a_user_table_the_role_face_stays_inactive() {
    let clients = ClientTokenTable::from_config(ClientTokenConfig::new(vec![spec(
        MEMBER_CLIENT,
        MEMBER_TOKEN,
        None,
    )]))
    .unwrap();
    let harness = harness(
        GatewayTokenGate::Enforced(GATE_TOKEN.to_string()),
        clients,
        None,
    )
    .await;
    let router = canonical_router_with_state(harness.state.clone());

    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/chat",
                Some(MEMBER_TOKEN),
                json_body(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(status, StatusCode::FORBIDDEN, "角色面未激活不得新增拒绝面");
    assert_ne!(status, StatusCode::UNAUTHORIZED, "令牌门照旧生效");

    // 管理面同样休眠 (兼容路径: 谁过令牌门谁可达, 与 W2-② 逐字一致)。
    let (status, _) = status_and_body(
        router
            .oneshot(request(
                "GET",
                "/v1/admin/config",
                Some(MEMBER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "兼容路径下管理面读照旧可达");

    harness.state.observations.flush().await;
    assert!(
        harness.audit.details_of(USER_ACTION_AUDIT_EVENT).is_empty(),
        "角色面未激活不得伪造 user_action 审计行: {:?}",
        harness.audit.details_of(USER_ACTION_AUDIT_EVENT)
    );
}

// ---------------------------------------------------------------------------
// 2. role 权限矩阵逐档 (owner / member / guest × 写 / 管理 / 审批)
// ---------------------------------------------------------------------------

/// owner = 全权: 写面 / 管理面 / 审批面全部过闸进 handler (业务状态码, 非 403)。
#[tokio::test]
async fn owner_passes_write_admin_and_approval_faces() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    // 写面: chat 进 handler (无 provider = 503 业务语义, 不是鉴权失败)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/chat",
                Some(OWNER_TOKEN),
                json_body(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "owner 写面必须过闸: {status}"
    );
    assert_ne!(status, StatusCode::UNAUTHORIZED, "{status}");

    // 管理面: 配置读 200 + 停机放闸 200 (W1-③ 口径零回归)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/admin/config",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner 管理面读必须 200");
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "POST",
                "/v1/admin/shutdown",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner 停机端点必须放闸");

    // 审批面: 过闸进 handler (approval 不在 = 400 业务语义, 不是 403)。
    let (status, _) = status_and_body(
        router
            .oneshot(json_request(
                "POST",
                "/v1/approvals/resolve",
                Some(OWNER_TOKEN),
                approval_body("00000000-0000-0000-0000-000000000053"),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "owner 审批面必须过闸: {status}"
    );
}

/// member = 域内读写, 无管理面: chat 照常, admin 族全 403。
#[tokio::test]
async fn member_can_write_but_never_touches_the_admin_face() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    // 写面放行 (域内常规操作)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/chat",
                Some(MEMBER_TOKEN),
                json_body(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "member 写面必须过闸: {status}"
    );

    // 管理面三族全拒 (admin 配置读写 / 停机 / 授权热撤)。
    for (method, uri) in [
        ("GET", "/v1/admin/config"),
        ("POST", "/v1/admin/config"),
        ("POST", "/v1/admin/shutdown"),
        ("POST", "/v1/panel/grants/revoke"),
    ] {
        let response = router
            .clone()
            .oneshot(request(method, uri, Some(MEMBER_TOKEN), Body::empty()))
            .await
            .unwrap();
        let text = assert_denied(response, &format!("member {method} {uri}")).await;
        assert!(
            text.contains("member") || text.contains("not bound") || text.contains("role"),
            "拒绝帧应说清角色原因: {text}"
        );
    }
}

/// guest = 只读 + 零写面: 读面照常进 handler, 写面 (chat / 会话设置) 全 403。
#[tokio::test]
async fn guest_has_zero_write_surface_but_keeps_read_access() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    // 写面全拒。
    for (method, uri, body) in [
        ("POST", "/v1/chat", json_body()),
        ("POST", "/v1/chat/completions", json_body()),
        (
            "PATCH",
            "/v1/sessions/00000000-0000-0000-0000-000000000054/settings",
            Body::empty(),
        ),
        (
            "DELETE",
            "/v1/sessions/00000000-0000-0000-0000-000000000054",
            Body::empty(),
        ),
    ] {
        let response = router
            .clone()
            .oneshot(json_request(method, uri, Some(GUEST_TOKEN), body))
            .await
            .unwrap();
        assert_denied(response, &format!("guest {method} {uri}")).await;
    }

    // 读面放行 (无面板后端 = 501 诚实降级, 不是鉴权失败)。
    for (method, uri) in [
        ("GET", "/v1/panel/sessions"),
        ("GET", "/v1/tools/list"),
        ("GET", "/v1/apeireth/capabilities"),
    ] {
        let (status, _) = status_and_body(
            router
                .clone()
                .oneshot(request(method, uri, Some(GUEST_TOKEN), Body::empty()))
                .await
                .unwrap(),
        )
        .await;
        assert_ne!(
            status,
            StatusCode::FORBIDDEN,
            "guest 只读面不得拒: {method} {uri}"
        );
        assert_ne!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }

    // 审批面 = L2 档: guest 无审批权, resolve 同样 403。
    let response = router
        .oneshot(json_request(
            "POST",
            "/v1/approvals/resolve",
            Some(GUEST_TOKEN),
            approval_body("00000000-0000-0000-0000-000000000054"),
        ))
        .await
        .unwrap();
    assert_denied(response, "guest approvals/resolve").await;
}

// ---------------------------------------------------------------------------
// 3. 对抗面 (敌人⑤ 多户越权)
// ---------------------------------------------------------------------------

/// 成员冒名 owner: 自称 owner 的客户端名 → name↔token 绑定校验 401 拒
/// (署名不可冒名); 角色只来自令牌绑定的解析链, 请求无从自称角色。
#[tokio::test]
async fn a_member_claiming_the_owner_client_name_is_refused() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    let mut request = request("GET", "/v1/admin/config", Some(MEMBER_TOKEN), Body::empty());
    request.headers_mut().insert(
        CLIENT_NAME_CLAIM_HEADER,
        OWNER_CLIENT.parse().expect("claimed name"),
    );
    let (status, text) = status_and_body(router.oneshot(request).await.unwrap()).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "冒名客户端名必须 401: {text}"
    );
    assert!(text.contains("auth_invalid_key"), "{text}");

    // 冒名不成, 内部身份仍是 member: 管理面照拒 (自称改不了角色)。
    harness.state.observations.flush().await;
    let denied = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        denied
            .iter()
            .all(|detail| !detail.contains("decision=allow") || !detail.contains("admin_control")),
        "冒名不得换到 owner 权限: {denied:?}"
    );
}

/// 成员拿自己的令牌直接闯管理面 (自称 owner 语义) = 403, 且审计署真名真角色。
#[tokio::test]
async fn a_member_self_declaring_owner_rights_is_refused_on_management() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    let response = router
        .oneshot(request(
            "POST",
            "/v1/admin/config",
            Some(MEMBER_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_denied(response, "member 自称 owner 闯管理面").await;

    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|detail| {
            detail.contains("user=user-member-a")
                && detail.contains("role=member")
                && detail.contains("action=admin_control")
                && detail.contains("decision=deny")
        }),
        "审计必须署真名真角色: {rows:?}"
    );
}

/// 跨用户审批拒: member B 不得解决 member A 会话的审批 (复用他人审批 =
/// 越权); 本人与 owner 可 (本人过闸进 handler, owner 全权)。
#[tokio::test]
async fn cross_user_approval_resolution_is_refused_for_non_owners() {
    let harness = fixture().await;
    let session = apeireth_core::kernel::SessionId::new();
    harness
        .state
        .user_access
        .bind_session_user(session, "user-member-a");
    let router = canonical_router_with_state(harness.state.clone());
    let uri = "/v1/approvals/resolve";

    // 跨用户 (member B → member A 的会话): 403 + 归属审计。
    let response = router
        .clone()
        .oneshot(json_request(
            "POST",
            uri,
            Some(MEMBER_B_TOKEN),
            approval_body(&session.to_string()),
        ))
        .await
        .unwrap();
    assert_denied(response, "跨用户审批 (member B → A 的会话)").await;

    // 本人 (member A) 过归属闸: 进 handler (approval 不在 = 400 业务语义)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                uri,
                Some(MEMBER_TOKEN),
                approval_body(&session.to_string()),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(status, StatusCode::FORBIDDEN, "本人审批必须过闸: {status}");

    // owner 全权: 跨用户也可批。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                uri,
                Some(OWNER_TOKEN),
                approval_body(&session.to_string()),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "owner 跨用户审批必须过闸: {status}"
    );

    // 归属审计落账。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|detail| {
            detail.contains("user=user-member-b")
                && detail.contains("action=approval_ownership")
                && detail.contains("decision=deny")
        }),
        "跨用户审批拒必须留痕: {rows:?}"
    );
}

/// 未绑定客户端 (令牌在表内但没有用户): 角色面激活时写面 fail-closed 拒,
/// 拒绝帧说清原因; 读面仍放行 (矩阵全员持 Read)。
#[tokio::test]
async fn an_unbound_client_is_refused_on_gated_faces_fail_closed() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    let response = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/chat",
            Some(STRANGER_TOKEN),
            json_body(),
        ))
        .await
        .unwrap();
    let text = assert_denied(response, "未绑定客户端写面").await;
    assert!(text.contains("not bound"), "拒绝帧应说清未绑定: {text}");

    // 读面放行 (零写面 ≠ 零读面)。
    let (status, _) = status_and_body(
        router
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                Some(STRANGER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_ne!(status, StatusCode::FORBIDDEN, "未绑定客户端读面不得拒");
}

// ---------------------------------------------------------------------------
// 4. 审计行 (user/role/action)
// ---------------------------------------------------------------------------

/// `gateway.user_action` 审计行: user / role / action 齐全, allow 与 deny
/// 都留痕, 且令牌值零泄漏。
#[tokio::test]
async fn user_action_audit_rows_name_user_role_and_action() {
    let harness = fixture().await;
    let router = canonical_router_with_state(harness.state.clone());

    // 一拒 (member × 管理面) 一放 (member × 写面)。
    let _ = router
        .clone()
        .oneshot(request(
            "POST",
            "/v1/admin/shutdown",
            Some(MEMBER_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    let _ = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/chat",
            Some(MEMBER_TOKEN),
            json_body(),
        ))
        .await
        .unwrap();
    harness.state.observations.flush().await;

    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|detail| {
            detail.contains("user=user-member-a")
                && detail.contains("role=member")
                && detail.contains("action=admin_control")
                && detail.contains("endpoint=/v1/admin/shutdown")
                && detail.contains("decision=deny")
        }),
        "拒必须落账: {rows:?}"
    );
    assert!(
        rows.iter().any(|detail| {
            detail.contains("user=user-member-a")
                && detail.contains("role=member")
                && detail.contains("action=write")
                && detail.contains("endpoint=/v1/chat")
                && detail.contains("decision=allow")
        }),
        "放必须落账: {rows:?}"
    );
    for token in ALL_TOKENS {
        assert!(
            rows.iter().all(|detail| !detail.contains(token)),
            "审计不得携带令牌值: {rows:?}"
        );
    }
}
