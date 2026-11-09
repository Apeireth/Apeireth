//! C-D1 对象授权**宿主激活口**的面契约与对抗面
//! (`POST /v1/panel/grants/approve`、`POST /v1/panel/grants/deny`)。
//!
//! 这里钉七件事:
//! 1. **owner 放行** —— approve / deny 返回 `{"ok": true}`, 且**审批人身份来自
//!    令牌绑定链** (owner 的 user_id), 不是请求体;
//! 2. **owner 专属门** —— 两个路径**已登记进 `panel_admin_face`** ⇒ member /
//!    guest 一律 403 `permission_denied` (fail-closed), 且**端口零调用**;
//! 3. **未绑定客户端 403** —— 管理面 fail-closed (角色面判不了"谁"就不放行);
//! 4. **未绑定审批人 403** —— 端口**自身**再拒一道 (纵深防御; 生产路径上角色门
//!    已先拒, 这里用「不挂角色门」的最小装配单独钉住面内那一道);
//! 5. **请求体不得自带审批人** —— 带 `approver` / `decided_by` 一律拒 (未知字段
//!    拒), **绝不采信**; `reason` **只给 deny** (approve 带 `reason` 也拒);
//! 6. **审计两层** —— 面板侧每次裁决 (allow **和** deny) 都落一行
//!    `gateway.user_action` (`action=object_grant_approve|object_grant_deny` +
//!    `decision=allow|deny`); 令牌值零泄漏;
//! 7. **端口未接线 = 501 unsupported** —— 不假装有激活口。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::stored_doc::StoredDoc;
use apeireth_gateway::client_tokens::UserContext;
use apeireth_gateway::panels::{panel_admin_face, panel_routes};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand,
    ClientTokenConfig, ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayState,
    GatewayTokenGate, GrantDto, GrantQuery, ObjectGrantActivation, UserAccess,
    USER_ACTION_AUDIT_EVENT,
};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const OWNER_TOKEN: &str = "owner-token-0123456789abcdef";
const MEMBER_TOKEN: &str = "member-token-0123456789abcdef";
const GUEST_TOKEN: &str = "guest-token-0123456789abcdef";
const STRANGER_TOKEN: &str = "stranger-token-0123456789abcdef";

const OWNER_CLIENT: &str = "desktop-shell";
const MEMBER_CLIENT: &str = "tui";
const GUEST_CLIENT: &str = "phone-web";
const STRANGER_CLIENT: &str = "stranger";

const ALL_TOKENS: &[&str] = &[
    GATE_TOKEN,
    OWNER_TOKEN,
    MEMBER_TOKEN,
    GUEST_TOKEN,
    STRANGER_TOKEN,
];

const APPROVE_PATH: &str = "/v1/panel/grants/approve";
const DENY_PATH: &str = "/v1/panel/grants/deny";

/// 端口调用记录 (一次调用 = 一条)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActivationCall {
    action: &'static str,
    grant_id: String,
    approver: String,
    reason: Option<String>,
}

/// 端口假实现: 记下每次调用; `fail_with` 非空 ⇒ 该次调用返回真因。
#[derive(Debug, Default)]
struct StubActivation {
    calls: Mutex<Vec<ActivationCall>>,
    fail_with: Mutex<Option<String>>,
}

impl StubActivation {
    fn calls(&self) -> Vec<ActivationCall> {
        self.calls.lock().unwrap().clone()
    }

    fn fail_next(&self, reason: &str) {
        *self.fail_with.lock().unwrap() = Some(reason.to_string());
    }
}

impl ObjectGrantActivation for StubActivation {
    fn approve(&self, grant_id: &str, approver: &str) -> Result<(), String> {
        self.calls.lock().unwrap().push(ActivationCall {
            action: "approve",
            grant_id: grant_id.to_string(),
            approver: approver.to_string(),
            reason: None,
        });
        match self.fail_with.lock().unwrap().clone() {
            Some(reason) => Err(reason),
            None => Ok(()),
        }
    }

    fn deny(&self, grant_id: &str, approver: &str, reason: Option<&str>) -> Result<(), String> {
        self.calls.lock().unwrap().push(ActivationCall {
            action: "deny",
            grant_id: grant_id.to_string(),
            approver: approver.to_string(),
            reason: reason.map(ToOwned::to_owned),
        });
        match self.fail_with.lock().unwrap().clone() {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }
}

/// 授权读口假实现 (让 `permissions_unavailable` 闸放行; 读面与激活面同族)。
#[derive(Debug, Default)]
struct StubGrantQuery;

#[async_trait::async_trait]
impl GrantQuery for StubGrantQuery {
    async fn list_grants(&self) -> Result<Vec<GrantDto>, String> {
        Ok(Vec::new())
    }
}

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
    activation: Arc<StubActivation>,
    dir: PathBuf,
}

static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

/// 每例独立数据目录 (并行安全)。
fn temp_dir(tag: &str) -> PathBuf {
    let n = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "apeireth-object-grant-activation-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 写一份客户端令牌表 (`client-tokens.json`, StoredDoc 信封)。
fn write_tokens(dir: &Path, entries: &[(&str, &str, Option<&str>)]) {
    let clients = entries
        .iter()
        .map(|(name, token, user_id)| ClientTokenSpec {
            name: (*name).to_string(),
            token: (*token).to_string(),
            user_id: user_id.map(str::to_string),
        })
        .collect();
    let doc = StoredDoc::new(
        apeireth_gateway::CLIENT_TOKENS_DOC_NAME,
        apeireth_gateway::CLIENT_TOKENS_DOC_VERSION,
        vec![apeireth_gateway::CLIENT_TOKENS_DOC_VERSION],
        ClientTokenConfig::new(clients),
    );
    fs_err::write(
        dir.join(apeireth_gateway::CLIENT_TOKENS_FILE),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
}

fn record(user_id: &str, role: &str, clients: &[&str]) -> UserRecord {
    UserRecord {
        user_id: user_id.to_string(),
        role: role.to_string(),
        client_names: clients.iter().map(|c| (*c).to_string()).collect(),
        domain: None,
    }
}

/// 标准夹具: owner + member + guest + 未绑定客户端, 端口已接线。
async fn fixture(tag: &str) -> Harness {
    let dir = temp_dir(tag);
    std::env::remove_var(apeireth_gateway::CLIENT_TOKENS_ENV);
    write_tokens(
        &dir,
        &[
            (OWNER_CLIENT, OWNER_TOKEN, Some("user-owner")),
            (MEMBER_CLIENT, MEMBER_TOKEN, Some("user-member-a")),
            (GUEST_CLIENT, GUEST_TOKEN, Some("user-guest")),
            (STRANGER_CLIENT, STRANGER_TOKEN, None),
        ],
    );
    let users = UserTable::new(vec![
        record("user-owner", "owner", &[OWNER_CLIENT]),
        record("user-member-a", "member", &[MEMBER_CLIENT]),
        record("user-guest", "guest", &[GUEST_CLIENT]),
    ]);
    users.save(&dir).expect("users.json");

    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let activation = Arc::new(StubActivation::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        grants: Some(Arc::new(StubGrantQuery)),
        grant_activation: Some(activation.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::load(Some(&dir)).expect("token table");
    state.user_access = UserAccess::load(&dir, &state.client_tokens).expect("user role face");
    Harness {
        state,
        audit,
        activation,
        dir,
    }
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

fn json_request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = request(method, uri, token, body);
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    request
}

fn json_body(value: serde_json::Value) -> Body {
    Body::from(serde_json::to_vec(&value).expect("body"))
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
// 1. owner 放行 + 审批人身份来自令牌绑定链
// ---------------------------------------------------------------------------

/// owner approve / deny ⇒ 200 `{"ok": true}`; 端口看到的审批人是**令牌绑定链**
/// 解析出的 user_id (不是请求体里的任何自称)。
#[tokio::test]
async fn the_owner_can_approve_and_deny_and_the_approver_comes_from_the_token_chain() {
    let harness = fixture("owner-allow").await;
    let router = canonical_router_with_state(harness.state.clone());

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                APPROVE_PATH,
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({ "grant_id": "grant-1" })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["ok"], true, "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                DENY_PATH,
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({ "grant_id": "grant-2", "reason": "越权目标" })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");

    let calls = harness.activation.calls();
    assert_eq!(calls.len(), 2, "两次裁决各落一次端口调用: {calls:?}");
    assert_eq!(calls[0].action, "approve");
    assert_eq!(calls[0].grant_id, "grant-1");
    assert_eq!(
        calls[0].approver, "user-owner",
        "审批人必须来自令牌绑定链: {calls:?}"
    );
    assert_eq!(calls[1].action, "deny");
    assert_eq!(calls[1].grant_id, "grant-2");
    assert_eq!(calls[1].approver, "user-owner");
    assert_eq!(
        calls[1].reason.as_deref(),
        Some("越权目标"),
        "否决理由必须传给端口: {calls:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. owner 专属门 (两个路径必须登记进 panel_admin_face)
// ---------------------------------------------------------------------------

/// 两个路径**必须**登记进 `panel_admin_face` ⇒ 非 owner 一律 403 且端口零调用。
#[tokio::test]
async fn the_two_paths_are_registered_in_the_admin_face_and_non_owners_are_refused() {
    assert!(
        panel_admin_face(APPROVE_PATH),
        "{APPROVE_PATH} 必须登记进管理面 (否则退化成普通写面 = 治理倒退)"
    );
    assert!(
        panel_admin_face(DENY_PATH),
        "{DENY_PATH} 必须登记进管理面 (否则退化成普通写面 = 治理倒退)"
    );

    let harness = fixture("admin-face").await;
    let router = canonical_router_with_state(harness.state.clone());
    for (token, role) in [(MEMBER_TOKEN, "member"), (GUEST_TOKEN, "guest")] {
        for (path, body) in [
            (APPROVE_PATH, serde_json::json!({ "grant_id": "grant-x" })),
            (
                DENY_PATH,
                serde_json::json!({ "grant_id": "grant-x", "reason": "r" }),
            ),
        ] {
            let label = format!("{role} × {path}");
            let response = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    path,
                    Some(token),
                    json_body(body.clone()),
                ))
                .await
                .unwrap();
            assert_denied(response, &label).await;
        }
    }
    assert!(
        harness.activation.calls().is_empty(),
        "非 owner 的裁决不得触达端口: {:?}",
        harness.activation.calls()
    );
}

/// 未绑定客户端 (令牌在表内但没有用户) ⇒ 管理面 fail-closed 403, 端口零调用。
#[tokio::test]
async fn an_unbound_client_is_refused_and_the_port_is_never_reached() {
    let harness = fixture("unbound").await;
    let router = canonical_router_with_state(harness.state.clone());
    for path in [APPROVE_PATH, DENY_PATH] {
        let response = router
            .clone()
            .oneshot(json_request(
                "POST",
                path,
                Some(STRANGER_TOKEN),
                json_body(serde_json::json!({ "grant_id": "grant-x" })),
            ))
            .await
            .unwrap();
        let text = assert_denied(response, &format!("未绑定 × {path}")).await;
        assert!(text.contains("not bound"), "拒绝帧应说清未绑定: {text}");
    }
    assert!(
        harness.activation.calls().is_empty(),
        "未绑定客户端不得触达端口: {:?}",
        harness.activation.calls()
    );
}

/// 未绑定审批人 ⇒ **面内自己**再拒一道 (纵深防御; 生产路径上角色门已先拒)。
///
/// 这里用「不挂角色门」的最小装配 (只挂面板路由 + 注入空 `UserContext`)
/// ⇒ 单独钉住面内那一道 403。
#[tokio::test]
async fn an_unbound_approver_is_refused_by_the_face_itself() {
    let harness = fixture("unbound-approver").await;
    let router: Router = panel_routes()
        .layer(axum::Extension(UserContext::unbound()))
        .with_state(harness.state.clone());

    for path in [APPROVE_PATH, DENY_PATH] {
        let response = router
            .clone()
            .oneshot(json_request(
                "POST",
                path,
                None,
                json_body(serde_json::json!({ "grant_id": "grant-x" })),
            ))
            .await
            .unwrap();
        let text = assert_denied(response, &format!("未绑定审批人 × {path}")).await;
        assert!(
            text.contains("not bound"),
            "拒绝帧应说清审批人未绑定: {text}"
        );
    }
    assert!(
        harness.activation.calls().is_empty(),
        "未绑定审批人不得触达端口: {:?}",
        harness.activation.calls()
    );
}

// ---------------------------------------------------------------------------
// 3. 请求体不得自带审批人
// ---------------------------------------------------------------------------

/// 请求体自带 `approver` / `decided_by` ⇒ **拒** (未知字段拒) 且**绝不采信**。
#[tokio::test]
async fn a_request_body_carrying_an_approver_is_refused_and_never_trusted() {
    let harness = fixture("body-approver").await;
    let router = canonical_router_with_state(harness.state.clone());
    for path in [APPROVE_PATH, DENY_PATH] {
        for field in ["approver", "decided_by"] {
            let mut body = serde_json::json!({ "grant_id": "grant-x" });
            body[field] = serde_json::json!("user-owner");
            let response = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    path,
                    Some(OWNER_TOKEN),
                    json_body(body),
                ))
                .await
                .unwrap();
            let (status, text) = status_and_body(response).await;
            assert!(
                status.is_client_error(),
                "自带 {field} 的请求必须被拒 ({path}): {status} {text}"
            );
        }
    }
    assert!(
        harness.activation.calls().is_empty(),
        "自带审批人的请求不得触达端口: {:?}",
        harness.activation.calls()
    );
}

/// `reason` **只给 deny** ⇒ approve 带 `reason` 也拒 (词面闭合)。
#[tokio::test]
async fn the_reason_field_is_accepted_only_by_deny() {
    let harness = fixture("reason-shape").await;
    let router = canonical_router_with_state(harness.state.clone());
    let response = router
        .clone()
        .oneshot(json_request(
            "POST",
            APPROVE_PATH,
            Some(OWNER_TOKEN),
            json_body(serde_json::json!({ "grant_id": "grant-x", "reason": "不该在这里" })),
        ))
        .await
        .unwrap();
    let (status, text) = status_and_body(response).await;
    assert!(
        status.is_client_error(),
        "approve 带 reason 必须被拒: {status} {text}"
    );
    assert!(
        harness.activation.calls().is_empty(),
        "被拒的请求不得触达端口: {:?}",
        harness.activation.calls()
    );
}

// ---------------------------------------------------------------------------
// 4. 审计两层 (面板侧 allow 与 deny 都记)
// ---------------------------------------------------------------------------

/// 面板侧审计: allow 与 deny **都**落一行 `gateway.user_action`; 失败裁决
/// (账本拒) 也落一行 `decision=deny`; 令牌值零泄漏。
#[tokio::test]
async fn every_decision_lands_one_user_action_audit_row() {
    let harness = fixture("audit").await;
    let router = canonical_router_with_state(harness.state.clone());

    // ① 通过。
    let response = router
        .clone()
        .oneshot(json_request(
            "POST",
            APPROVE_PATH,
            Some(OWNER_TOKEN),
            json_body(serde_json::json!({ "grant_id": "grant-1" })),
        ))
        .await
        .unwrap();
    assert_eq!(status_and_body(response).await.0, StatusCode::OK);

    // ② 否决。
    let response = router
        .clone()
        .oneshot(json_request(
            "POST",
            DENY_PATH,
            Some(OWNER_TOKEN),
            json_body(serde_json::json!({ "grant_id": "grant-2", "reason": "r" })),
        ))
        .await
        .unwrap();
    assert_eq!(status_and_body(response).await.0, StatusCode::OK);

    // ③ 账本拒 (真因) ⇒ 5xx + 一行 decision=deny。
    harness.activation.fail_next("账本拒: 授权不存在");
    let response = router
        .clone()
        .oneshot(json_request(
            "POST",
            APPROVE_PATH,
            Some(OWNER_TOKEN),
            json_body(serde_json::json!({ "grant_id": "grant-3" })),
        ))
        .await
        .unwrap();
    let (status, text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{text}");
    assert!(text.contains("账本拒"), "真因必须进错误帧: {text}");

    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter()
            .any(|detail| detail.contains("action=object_grant_approve")
                && detail.contains("decision=allow")
                && detail.contains("user=user-owner")),
        "通过裁决必须落审计: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|detail| detail.contains("action=object_grant_deny")
                && detail.contains("decision=allow")
                && detail.contains("user=user-owner")),
        "否决裁决必须落审计: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|detail| detail.contains("action=object_grant_approve")
                && detail.contains("decision=deny")),
        "失败裁决也必须落审计 (不得静默): {rows:?}"
    );
    for token in ALL_TOKENS {
        assert!(
            rows.iter().all(|detail| !detail.contains(token)),
            "审计不得携带令牌值: {rows:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. 端口未接线 = 501 unsupported
// ---------------------------------------------------------------------------

/// 端口未注入 ⇒ `501 unsupported` (不假装有激活口), 且两个面各自报自己的面名。
#[tokio::test]
async fn the_face_is_unsupported_when_the_port_is_not_wired() {
    let harness = fixture("unsupported").await;
    let mut services = GatewayServices {
        grants: Some(Arc::new(StubGrantQuery)),
        ..GatewayServices::default()
    };
    services.grant_activation = None;
    let mut state = build_gateway_state_with_services(
        Arc::new(Runtime::builder().build().await.unwrap()),
        services,
    );
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::load(Some(&harness.dir)).expect("token table");
    state.user_access = UserAccess::load(&harness.dir, &state.client_tokens).expect("role face");
    let router = canonical_router_with_state(state);

    for (path, label) in [
        (APPROVE_PATH, "permissions.approve"),
        (DENY_PATH, "permissions.deny"),
    ] {
        let response = router
            .clone()
            .oneshot(json_request(
                "POST",
                path,
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({ "grant_id": "grant-x" })),
            ))
            .await
            .unwrap();
        let (status, text) = status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{path}: {text}");
        assert!(text.contains("unsupported"), "{path}: {text}");
        assert!(text.contains(label), "{path} 应点名自己的面名: {text}");
    }
}
