//! V2-b 用户/域管理端点的面契约与对抗面 (`/v1/panel/users`、`/v1/admin/users*`、
//! `/v1/panel/domains`、`/v1/admin/domains`)。
//!
//! 这里钉五件事:
//! 1. **四端点全链** —— owner 放行 (列表/建户/改角色/删户逐口往返, 词面字段成契约);
//! 2. **owner 专属门** —— member / guest / 未绑定客户端在四个用户口与两个域口
//!    一律 403 `permission_denied` (fail-closed), 令牌门照旧 (无/伪令牌 401);
//! 3. **坏输入拒** —— 路径穿越 user_id / 大小写混淆 id / 未知角色 / 越界域引用
//!    一律 400 `invalid_request`; 重复 id 409 `conflict`;
//! 4. **破坏性不变量** —— 末 owner 不可删/不可降 (HTTP 面同一把闸), 令牌仍引用
//!    的用户删不得 (发令牌序);
//! 5. **审计行** —— 每次管理动作落 `gateway.user_action` (user/role/action/decision),
//!    令牌值零泄漏; 坏 `users.json` 拒开 (500, 不回退空表)。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::stored_doc::StoredDoc;
use apeireth_core::user_domain::DomainRegistry;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand,
    ClientTokenConfig, ClientTokenSpec, ClientTokenTable, DomainAdminService, GatewayServices,
    GatewayState, GatewayTokenGate, UserAccess, UserAdminService, USER_ACTION_AUDIT_EVENT,
};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
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
    dir: PathBuf,
}

static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

/// 每例独立数据目录 (并行安全)。
fn temp_dir(tag: &str) -> PathBuf {
    let n = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "apeireth-user-admin-endpoints-{tag}-{}-{n}",
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

/// 四用户 / 四令牌的标准夹具 (owner + member + guest + 未绑定客户端), 装配口径
/// 与生产一致: 令牌表与用户表都从数据目录装载 (装载期绑定校验), 管理面服务
/// 指向同一数据目录。
async fn fixture(tag: &str) -> Harness {
    let dir = temp_dir(tag);
    // 服务每操作重读 `APEIRETH_GATEWAY_TOKENS` 优先的令牌族; 测试要钉数据目录
    // 事实, 清掉进程环境面 (本测试文件不设该变量, 并行清理是幂等的)。
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
    harness_from(&dir).await
}

/// 从一个已有数据目录装配网关 (双入口同源的"入口面"就是这里的管理服务)。
async fn harness_from(dir: &Path) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        user_admin: Some(Arc::new(UserAdminService::new(dir))),
        domain_admin: Some(Arc::new(DomainAdminService::new(
            DomainRegistry::open(dir).unwrap(),
        ))),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::load(Some(dir)).expect("token table");
    state.user_access = UserAccess::load(dir, &state.client_tokens).expect("user role face");
    Harness {
        state,
        audit,
        dir: dir.to_path_buf(),
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

/// 带 JSON 正文的请求 (axum `Json` 提取器要求 content-type)。
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

/// 四端点矩阵 (方法 + 路径 + 请求体)。
fn user_endpoints() -> Vec<(&'static str, &'static str, Body)> {
    vec![
        ("GET", "/v1/panel/users", Body::empty()),
        (
            "POST",
            "/v1/admin/users",
            json_body(serde_json::json!({
                "user_id": "user-new", "role": "guest", "client_names": []
            })),
        ),
        (
            "PATCH",
            "/v1/admin/users/user-member-a",
            json_body(serde_json::json!({ "role": "guest" })),
        ),
        ("DELETE", "/v1/admin/users/user-guest", Body::empty()),
    ]
}

// ---------------------------------------------------------------------------
// 1. 四端点全链 (owner 放行)
// ---------------------------------------------------------------------------

/// owner 全链: 列表 → 建户 → 改角色 → 删户, 每口真值往返, 词面字段成契约。
#[tokio::test]
async fn owner_walks_the_full_user_admin_chain_over_http() {
    let harness = fixture("owner-chain").await;
    let router = canonical_router_with_state(harness.state.clone());

    // 列表: 三用户 + 计数 + 字段词面 (id/角色/域引用/客户端数/绑定名)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/users",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["count"], 3, "{text}");
    let owner = body["users"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["user_id"] == "user-owner")
        .expect("owner row");
    assert_eq!(owner["role"], "owner");
    assert_eq!(owner["client_count"], 1);
    assert_eq!(owner["client_names"][0], OWNER_CLIENT);
    assert!(
        owner.get("domain").is_none(),
        "无域引用的行不编域字段: {text}"
    );

    // 建户 (201 + 行体)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/users",
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({
                    "user_id": "user-member-b", "role": "member",
                    "client_names": [], "domain": "home"
                })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["user"]["user_id"], "user-member-b");
    assert_eq!(body["user"]["role"], "member");
    assert_eq!(body["user"]["domain"], "home");
    assert_eq!(body["user"]["client_count"], 0);

    // 改角色 (200 + 新词面)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "PATCH",
                "/v1/admin/users/user-member-b",
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({ "role": "guest" })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["user"]["role"], "guest");

    // 删户 (200 + deleted 事实)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "DELETE",
                "/v1/admin/users/user-member-b",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["deleted"], true);
    assert_eq!(body["user_id"], "user-member-b");

    // 落盘可回读 (同一 UserTable 事实)。
    let table = UserTable::load(&harness.dir).unwrap().expect("table");
    assert_eq!(table.users.len(), 3);
    assert!(table.identity_for_user("user-member-b").is_none());

    // 审计: 三个管理动作各一行 (user/role/action/decision=allow)。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    for action in ["user_add", "user_role_change", "user_remove"] {
        assert!(
            rows.iter().any(|detail| {
                detail.contains("user=user-owner")
                    && detail.contains("role=owner")
                    && detail.contains(&format!("action={action}"))
                    && detail.contains("decision=allow")
            }),
            "管理动作 {action} 必须落审计: {rows:?}"
        );
    }
    for token in ALL_TOKENS {
        assert!(
            rows.iter().all(|detail| !detail.contains(token)),
            "审计不得携带令牌值: {rows:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. owner 专属门 (member / guest / 未绑定拒) + 令牌门
// ---------------------------------------------------------------------------

/// 四端点 × member/guest: 一律 403 `permission_denied` (管理面 owner 专属)。
#[tokio::test]
async fn member_and_guest_are_refused_on_every_user_admin_endpoint() {
    let harness = fixture("role-gate").await;
    let router = canonical_router_with_state(harness.state.clone());
    for (token, role) in [(MEMBER_TOKEN, "member"), (GUEST_TOKEN, "guest")] {
        for (method, uri, body) in user_endpoints() {
            let label = format!("{role} × {method} {uri}");
            let response = router
                .clone()
                .oneshot(json_request(method, uri, Some(token), body))
                .await
                .unwrap();
            assert_denied(response, &label).await;
        }
    }
    // 拒后用户表原样 (拒不留痕)。
    let table = UserTable::load(&harness.dir).unwrap().expect("table");
    assert_eq!(table.users.len(), 3);
}

/// 未绑定客户端 (令牌在表内但没有用户): 管理面 fail-closed 拒, 帧说清原因。
#[tokio::test]
async fn an_unbound_client_is_refused_on_the_admin_faces() {
    let harness = fixture("unbound").await;
    let router = canonical_router_with_state(harness.state.clone());
    for (method, uri, body) in user_endpoints() {
        let response = router
            .clone()
            .oneshot(json_request(method, uri, Some(STRANGER_TOKEN), body))
            .await
            .unwrap();
        let text = assert_denied(response, &format!("未绑定 × {method} {uri}")).await;
        assert!(text.contains("not bound"), "拒绝帧应说清未绑定: {text}");
    }
    for (method, uri, body) in [
        ("GET", "/v1/panel/domains", Body::empty()),
        (
            "POST",
            "/v1/admin/domains",
            json_body(serde_json::json!({
                "user_id": "alice", "display_name": "Alice"
            })),
        ),
    ] {
        let response = router
            .clone()
            .oneshot(json_request(method, uri, Some(STRANGER_TOKEN), body))
            .await
            .unwrap();
        assert_denied(response, &format!("未绑定 × {method} {uri}")).await;
    }
}

/// 令牌门照旧包住管理面: 无令牌 401 `auth_missing_key`, 伪令牌 401 `auth_invalid_key`。
#[tokio::test]
async fn the_token_gate_still_covers_the_new_admin_faces() {
    let harness = fixture("token-gate").await;
    let router = canonical_router_with_state(harness.state.clone());
    for (token, code) in [
        (None, "auth_missing_key"),
        (Some("tok-forged-0123456789abcdef"), "auth_invalid_key"),
    ] {
        for (method, uri, body) in user_endpoints() {
            let (status, text) = status_and_body(
                router
                    .clone()
                    .oneshot(json_request(method, uri, token, body))
                    .await
                    .unwrap(),
            )
            .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}: {text}");
            assert!(text.contains(code), "{method} {uri}: {text}");
        }
    }
}

// ---------------------------------------------------------------------------
// 3. 坏输入拒 (路径安全 / 词表冻结 / 重复 id) + 破坏性不变量
// ---------------------------------------------------------------------------

/// 坏输入矩阵: 路径穿越/大小写混淆 user_id、未知角色、角色大小写变体、越界域
/// 引用一律 400 `invalid_request`; 重复 user_id 409 `conflict`。
#[tokio::test]
async fn bad_input_is_refused_before_any_table_change() {
    let harness = fixture("bad-input").await;
    let router = canonical_router_with_state(harness.state.clone());

    // 路径穿越 / 词面越界的 user_id (复用 W3-① DomainUserId 校验)。
    for uri in [
        "/v1/admin/users/..%2Fescape",
        "/v1/admin/users/A-1",
        "/v1/admin/users/..",
    ] {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(
                    "PATCH",
                    uri,
                    Some(OWNER_TOKEN),
                    json_body(serde_json::json!({ "role": "member" })),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "PATCH {uri}: {text}");
        assert!(text.contains("invalid_request"), "PATCH {uri}: {text}");
    }

    // 未知角色 / 角色大小写变体 (词表冻结, 不就近映射)。
    for role in ["admin", "Owner", "OWNER", "owner "] {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(
                    "POST",
                    "/v1/admin/users",
                    Some(OWNER_TOKEN),
                    json_body(serde_json::json!({
                        "user_id": "user-new", "role": role, "client_names": []
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "role {role:?}: {text}");
        assert!(text.contains("invalid_request"), "role {role:?}: {text}");
    }
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "PATCH",
                "/v1/admin/users/user-member-a",
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({ "role": "root" })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{text}");

    // 建户 id 词面越界 / 域引用越界。
    for body in [
        serde_json::json!({ "user_id": "../evil", "role": "owner", "client_names": [] }),
        serde_json::json!({ "user_id": "user-new", "role": "owner", "client_names": [], "domain": "a/b" }),
    ] {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(
                    "POST",
                    "/v1/admin/users",
                    Some(OWNER_TOKEN),
                    json_body(body.clone()),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {text}");
    }

    // 重复 user_id (问责署名会歧义) → 409。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/users",
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({
                    "user_id": "user-owner", "role": "member", "client_names": []
                })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}");
    assert!(text.contains("conflict"), "{text}");

    // 全部拒绝后用户表原样。
    let table = UserTable::load(&harness.dir).unwrap().expect("table");
    assert_eq!(table.users.len(), 3);
}

/// 破坏性不变量 (HTTP 面同一把闸): 末 owner 不可删/不可降; 令牌仍引用的用户
/// 删不得 (发令牌序: 先摘绑定后删用户)。
#[tokio::test]
async fn destructive_invariants_hold_on_the_http_face_too() {
    let harness = fixture("invariants").await;
    let router = canonical_router_with_state(harness.state.clone());

    // 末 owner 不可降级、不可删除 → 409。
    for (method, uri, body) in [
        (
            "PATCH",
            "/v1/admin/users/user-owner",
            json_body(serde_json::json!({ "role": "member" })),
        ),
        ("DELETE", "/v1/admin/users/user-owner", Body::empty()),
    ] {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(method, uri, Some(OWNER_TOKEN), body))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{method} {uri}: {text}");
        assert!(
            text.contains("at least one owner"),
            "{method} {uri}: {text}"
        );
    }

    // 令牌仍引用 user-member-a: 删不得 → 409 (问责链不得断)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "DELETE",
                "/v1/admin/users/user-member-a",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}");
    assert!(text.contains("inconsistent"), "{text}");

    // 先摘令牌绑定, 再删用户: 成功 (发令牌序)。
    write_tokens(
        &harness.dir,
        &[(OWNER_CLIENT, OWNER_TOKEN, Some("user-owner"))],
    );
    let harness = harness_from(&harness.dir).await;
    let router = canonical_router_with_state(harness.state.clone());
    let (status, text) = status_and_body(
        router
            .oneshot(request(
                "DELETE",
                "/v1/admin/users/user-member-a",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let table = UserTable::load(&harness.dir).unwrap().expect("table");
    assert_eq!(table.users.len(), 2);
}

// ---------------------------------------------------------------------------
// 4. 域管理面 (列表 + 开域)
// ---------------------------------------------------------------------------

/// 域两口全链: 列表带计数 (default 采用域在场), 开域 201 并入列;
/// 重复域 / 保留域 / 越界域 id 一律拒。
#[tokio::test]
async fn owner_lists_and_opens_domains_with_counts() {
    let harness = fixture("domains").await;
    let router = canonical_router_with_state(harness.state.clone());

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/domains",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    let domains = body["domains"].as_array().unwrap();
    assert!(
        domains.iter().any(|row| row["user_id"] == "default"),
        "{text}"
    );
    assert_eq!(body["count"], domains.len(), "{text}");
    assert!(
        domains.iter().all(|row| row["user_count"].is_number()),
        "用户管理面在场时计数是真数: {text}"
    );

    // 开域 (201 + 入列)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/domains",
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({
                    "user_id": "alice", "display_name": "Alice"
                })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["domain"]["user_id"], "alice");
    assert_eq!(body["domain"]["role"], "owner");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/domains",
                Some(OWNER_TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(
        body["domains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["user_id"] == "alice"),
        "开域必须入列: {text}"
    );

    // 重复域 → 409; 保留域 id `default` → 400; 越界域 id → 400。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/domains",
                Some(OWNER_TOKEN),
                json_body(serde_json::json!({
                    "user_id": "alice", "display_name": "Alice again"
                })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}");
    for bad_id in ["default", "../escape"] {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(
                    "POST",
                    "/v1/admin/domains",
                    Some(OWNER_TOKEN),
                    json_body(serde_json::json!({
                        "user_id": bad_id, "display_name": "X"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad_id}: {text}");
    }

    // 开域审计行。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of(USER_ACTION_AUDIT_EVENT);
    assert!(
        rows.iter().any(|detail| {
            detail.contains("action=domain_create")
                && detail.contains("endpoint=/v1/admin/domains")
                && detail.contains("decision=allow")
        }),
        "开域必须落审计: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 5. 坏配置拒开 + 端口诚实
// ---------------------------------------------------------------------------

/// 坏 `users.json` 拒开 (不回退空表, 否则会绕过 owner 门): 管理面 500 拒,
/// 域列表的计数源同样拒 (fail-closed)。装配面本身也拒开坏表 (装载期闸),
/// 所以这里钉的是**运行中变坏**的那一刻: 每操作重读的管理面立刻拒。
#[tokio::test]
async fn a_broken_user_table_refuses_to_open_on_the_admin_faces() {
    let harness = fixture("broken-config").await;
    let router = canonical_router_with_state(harness.state.clone());
    fs_err::write(harness.dir.join("users.json"), b"{ not json").unwrap();

    for (method, uri, body) in [
        ("GET", "/v1/panel/users", Body::empty()),
        (
            "POST",
            "/v1/admin/users",
            json_body(serde_json::json!({
                "user_id": "user-new", "role": "owner", "client_names": []
            })),
        ),
        ("GET", "/v1/panel/domains", Body::empty()),
    ] {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(method, uri, Some(OWNER_TOKEN), body))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{method} {uri}: {text}"
        );
        assert!(
            text.contains("refused to open"),
            "{method} {uri} 必须报拒开真因: {text}"
        );
    }
}

/// 端口诚实: 管理服务未注入 = 501 `unsupported` (不假装有数据源)。
#[tokio::test]
async fn without_the_admin_services_the_faces_stay_unsupported() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state = build_gateway_state_with_services(runtime, GatewayServices::default());
    let state = GatewayState {
        token_gate: GatewayTokenGate::Enforced(GATE_TOKEN.to_string()),
        ..state
    };
    let router = canonical_router_with_state(state);
    for (method, uri, body) in user_endpoints() {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(json_request(method, uri, Some(GATE_TOKEN), body))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_IMPLEMENTED,
            "{method} {uri}: {text}"
        );
        assert!(text.contains("unsupported"), "{method} {uri}: {text}");
    }
}
