//! V2-b 双入口同源: CLI `apeireth user` 与 HTTP `/v1/panel/users`、
//! `/v1/admin/users*` 共用**同一个 UserTable 服务** (`apeireth_gateway::
//! UserAdminService`), 改**同一份** `users.json`。
//!
//! 这里钉三件事:
//! 1. **互见** —— CLI 的写 HTTP 即见, HTTP 的写 CLI 即见 (一份事实, 零双写);
//! 2. **同语义** —— 引导口径 (首用户必须 owner) 与末 owner 保护在两个入口逐字
//!    一致 (同一把闸, 不是两套近似实现);
//! 3. **问责链不断** —— 两入口都过发令牌序 (删用户前先摘令牌绑定)。

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_cli::user_admin::run_user_admin_at;
use apeireth_core::stored_doc::StoredDoc;
use apeireth_core::user_domain::{DomainRegistry, DomainRole};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, ClientTokenConfig,
    ClientTokenSpec, ClientTokenTable, EpisodeMutationDto, GatewayServices, GatewayTokenGate,
    MemoryGovernanceCommand, PrincipalForgetDto, UserAccess, UserAdminService,
    CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION, CLIENT_TOKENS_FILE,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{owner_record, write_users};

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const OWNER_TOKEN: &str = "tok-owner-0123456789abcdef";

/// HTTP 侧纯合成的完整遗忘端口；双入口测试只验证共享删户状态机与同一用户表。
struct CompleteForget;

#[async_trait::async_trait]
impl MemoryGovernanceCommand for CompleteForget {
    async fn protect_episode(
        &self,
        _id: &str,
        _expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        Err("unused synthetic protect".to_string())
    }

    async fn unprotect_episode(
        &self,
        _id: &str,
        _expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        Err("unused synthetic unprotect".to_string())
    }

    async fn forget_episode(
        &self,
        _id: &str,
        _expected_rev: u64,
        _reason: Option<&str>,
    ) -> Result<EpisodeMutationDto, String> {
        Err("unused synthetic episode forget".to_string())
    }

    async fn forget_principal(
        &self,
        principal_id: &str,
        _reason: Option<&str>,
    ) -> Result<PrincipalForgetDto, String> {
        Ok(PrincipalForgetDto {
            principal_id: principal_id.to_string(),
            episode: true,
            preference: true,
            commitment: true,
            temporal: true,
            persona: true,
            refused: false,
            reason: None,
        })
    }
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-user-admin-dual-{tag}-{}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

fn seed_memory_attribution(path: &Path, principal_id: &str) {
    let store = apeireth_memory::SqliteMemoryStore::open(path).expect("synthetic memory store");
    apeireth_memory::principal_sessions::record_principal_session(
        &store,
        principal_id,
        "synthetic-session",
        1,
    )
    .expect("synthetic attribution");
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
        CLIENT_TOKENS_DOC_NAME,
        CLIENT_TOKENS_DOC_VERSION,
        vec![CLIENT_TOKENS_DOC_VERSION],
        ClientTokenConfig::new(clients),
    );
    fs_err::write(
        dir.join(CLIENT_TOKENS_FILE),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
}

/// 网关 HTTP 入口的装配 (管理服务指向同一数据目录 = 同一 UserTable 事实)。
async fn router_over(dir: &Path) -> axum::Router {
    std::env::remove_var(apeireth_gateway::CLIENT_TOKENS_ENV);
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        memory_governance: Some(Arc::new(CompleteForget)),
        user_admin: Some(Arc::new(UserAdminService::new(dir))),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::load(Some(dir)).expect("token table");
    state.user_access = UserAccess::load(dir, &state.client_tokens).expect("user role face");
    canonical_router_with_state(state)
}

fn json_request(method: &str, uri: &str, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .expect("request");
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {OWNER_TOKEN}").parse().expect("bearer"),
    );
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

/// 互见全链: CLI 建的用户 HTTP 即见, HTTP 建/改/删的 CLI 即见 (一份 `users.json`)。
#[tokio::test]
async fn cli_and_http_administer_one_shared_user_table() {
    let dir = temp_dir("shared");
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);

    // CLI 引导首 owner (免令牌引导口, 服务强制"首用户必须 owner")。
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("cli 引导 owner");

    let router = router_over(&dir).await;

    // HTTP 侧看得见 CLI 的写 (同一份 users.json)。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request("GET", "/v1/panel/users", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("owner-1"), "CLI 的写 HTTP 必须即见: {text}");

    // HTTP 建户 → CLI 侧即见。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/users",
                json_body(serde_json::json!({
                    "user_id": "member-1", "role": "member", "client_names": []
                })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{text}");
    let listing = run_user_admin_at(&dir, &args(&["list", "--token", OWNER_TOKEN])).expect("list");
    assert!(
        listing.contains("member-1\tmember"),
        "HTTP 的写 CLI 必须即见: {listing}"
    );

    // CLI 改角色 → HTTP 侧即见。
    run_user_admin_at(
        &dir,
        &args(&[
            "role",
            "--user-id",
            "member-1",
            "--role",
            "guest",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("cli 改角色");
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request("GET", "/v1/panel/users", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    let member = body["users"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["user_id"] == "member-1")
        .expect("member row");
    assert_eq!(member["role"], "guest", "CLI 的改 HTTP 必须即见: {text}");

    // HTTP 删户 → CLI 侧即见。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "DELETE",
                "/v1/admin/users/member-1",
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let listing = run_user_admin_at(&dir, &args(&["list", "--token", OWNER_TOKEN])).expect("list");
    assert!(
        !listing.contains("member-1"),
        "HTTP 的删 CLI 必须即见: {listing}"
    );
}

/// 同语义: 引导口径与末 owner 保护在两个入口逐字一致 (同一把闸)。
///
/// # 前置条件 (P0-1 · 2026-11-04 管理面 fail-closed)
///
/// `/v1/admin/users*` 属**管理面** (要求 `UserCapability::AdminControl`) ⇒
/// 角色面**未激活** (无 `users.json`) 时一律 403 ⇒ 本用例要打管理面, 必须
/// 先按真实产品前置条件把角色面装上: owner-1 在册 + 令牌表条目 `tui` 声明
/// `user_id=owner-1` ⇒ 令牌→用户→角色链解析出 owner (持 `AdminControl`)。
///
/// 引导口径 (空表首用户必须 owner) 在 HTTP 面的可见性依赖**产品既有的两段
/// 语义**: 角色门按**装载期快照**判定 (「令牌/角色门的运行期判定仍按
/// W2-②/W3-② 装载期快照语义」), 而管理服务每个动作**重读** `users.json`
/// (「操作前重读、校验后落盘」) ⇒ 装载之后名册被重置为空表 ⇒ 管理面仍对
/// owner 令牌可达, 而引导口径在 HTTP 面同样生效 (同一把闸, 同一拒因)。
#[tokio::test]
async fn the_bootstrap_and_last_owner_rules_read_the_same_on_both_entries() {
    let dir = temp_dir("same-rules");
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);

    // 引导口径 · CLI 面 (空表 ⇒ 首用户必须 owner): 免令牌引导口, 拒因同源。
    let error = run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--clients",
            "tui",
        ]),
    )
    .expect_err("CLI 首用户非 owner 必须拒");
    assert!(error.contains("first user must be owner"), "{error}");

    // 角色面激活: owner-1 在册 (绑 `tui` —— 令牌表条目 `tui` 声明 user_id
    // owner-1 ⇒ 令牌→用户→角色链解析出 owner ⇒ 管理面可达)。
    write_users(&dir, vec![owner_record("owner-1", &["tui"])]);
    let router = router_over(&dir).await;

    // 名册复位为空表 (运营侧重置) ⇒ 引导口径复位。
    write_users(&dir, Vec::new());
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/v1/admin/users",
                json_body(serde_json::json!({
                    "user_id": "member-1", "role": "member", "client_names": []
                })),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{text}");
    assert!(text.contains("first user must be owner"), "{text}");

    // 任一入口引导 owner 后, 末 owner 保护在另一入口同样生效。
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("cli 引导 owner");
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "DELETE",
                "/v1/admin/users/owner-1",
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}");
    assert!(text.contains("at least one owner"), "{text}");
}

/// 问责链不断: 令牌仍引用的用户在两个入口都删不得 (发令牌序)。
#[tokio::test]
async fn a_user_still_referenced_by_a_token_cannot_be_removed_from_either_entry() {
    let dir = temp_dir("bindings");
    // 发令牌序 (问责链不得断): 令牌声明 `user_id` 的, **先建用户再写绑定** ——
    // 悬空绑定会让一切管理动作拒开, 所以这里先建档、后挂绑定。
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("cli 引导 owner");
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--clients",
            "phone-web",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("owner 建 member");
    write_tokens(
        &dir,
        &[
            ("tui", OWNER_TOKEN, Some("owner-1")),
            ("phone-web", "tok-member-0123456789abcdef", Some("member-1")),
        ],
    );

    let router = router_over(&dir).await;
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "DELETE",
                "/v1/admin/users/member-1",
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}");
    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("CLI 侧同样删不得");
    assert!(error.contains("inconsistent"), "{error}");

    // 先摘令牌绑定: 两入口先后删一次, 事实只有一份。
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(json_request(
                "DELETE",
                "/v1/admin/users/member-1",
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("事实只有一份: 第二次删除必须找不到人");
    assert!(error.contains("not in the user table"), "{error}");
}

/// CLI 不能再直删账号：权威库缺席不创建空库，归属不完整不删；补齐归属后同一
/// 共享状态机才允许删除。
#[test]
fn cli_remove_keeps_a_retry_entry_until_memory_is_fully_confirmed() {
    let dir = temp_dir("cli-retry");
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("add deletion target");

    let memory_db = dir.join("cognitive.sqlite3");
    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("missing authority must retain account");
    assert!(error.contains("memory.forget_unavailable"), "{error}");
    assert!(!memory_db.exists(), "缺席权威不得被创建为空库");
    assert!(apeireth_governance::UserTable::load(&dir)
        .unwrap()
        .unwrap()
        .identity_for_user("member-1")
        .is_some());

    // 库在但没有主体→会话归属：五账 DTO 的 episode/preference 为 false，仍拒删。
    drop(apeireth_memory::SqliteMemoryStore::open(&memory_db).expect("empty authority"));
    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("unattributable outcome must retain account");
    assert!(error.contains("memory.forget_incomplete"), "{error}");
    assert!(error.contains("episode=false"), "{error}");

    seed_memory_attribution(&memory_db, "member-1");
    let summary = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect("retry after attribution");
    assert!(summary.contains("memory_forgotten=true"), "{summary}");
    assert!(apeireth_governance::UserTable::load(&dir)
        .unwrap()
        .unwrap()
        .identity_for_user("member-1")
        .is_none());
}

/// 已开非默认域必须使用自己的 cognitive.sqlite3；即使默认库能给完整结果，也
/// 不能代替目标库。目标库缺席先拒，补齐目标库后才可删。
#[test]
fn cli_remove_never_uses_default_memory_for_a_non_default_domain() {
    let dir = temp_dir("cli-domain-scope");
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");
    let registry = DomainRegistry::open(&dir).unwrap();
    let private = registry
        .create("private-domain", "Private", DomainRole::Owner)
        .unwrap();
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--domain",
            "private-domain",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("add scoped target");

    let default_db = dir.join("cognitive.sqlite3");
    seed_memory_attribution(&default_db, "member-1");
    let private_db = registry
        .layout()
        .data_dir(&private.user_id)
        .join("cognitive.sqlite3");
    assert!(!private_db.exists());
    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("default authority must not substitute for target domain");
    assert!(error.contains("memory.forget_unavailable"), "{error}");
    assert!(error.contains("private-domain"), "{error}");
    assert!(!private_db.exists(), "目标库缺席不得创建空库");
    assert!(apeireth_governance::UserTable::load(&dir)
        .unwrap()
        .unwrap()
        .identity_for_user("member-1")
        .is_some());

    fs_err::create_dir_all(private_db.parent().unwrap()).unwrap();
    seed_memory_attribution(&private_db, "member-1");
    let summary = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect("target-scoped authority permits retry");
    assert!(summary.contains("memory_forgotten=true"), "{summary}");
}
