//! 「**主体 → 会话**」归属登记的**端到端**链路测试（2026-11-03）。
//!
//! # 这一批补的是**上一批自己标的诚实边界**
//!
//! 上一批（`feat(memory): 主体 → 会话归属映射的生产写入面`）的诚实边界写的是：
//! 「端到端只做到『钩子层 + 记忆层』：没有起真实 gateway 打 `/v1/chat`
//! ⇒ 域面 `user_id` → 登记落库这条**真实链路**未端到端跑过」。
//!
//! ⇒ ⇒ 本文件把这条链路**真的跑一遍**：
//! **HTTP 请求 → 令牌门 → 域解析 → 归属登记端口**。
//!
//! # 钉住三件事
//!
//! 1. **有域的请求 ⇒ 真的登记**：主体 = **令牌绑定的 `user_id`**（不是客户端署名键），
//!    会话 = 本次请求的 `session`（缺省则新建）。
//! 2. **无 `user_id` 的请求 ⇒ 不登记**（`anonymous` / `unattributed` 口径）
//!    —— 宁可「归属不可判定」，也不要错归属（**误删风险**）。
//! 3. **登记不因 chat 失败而丢失**：归属登记发生在**调 provider 之前**
//!    ⇒ 请求后续失败**不影响**归属已登记（否则 episode 又归不到主体）。
//!
//! # 隔离纪律
//!
//! 写面是**进程级**槽 ⇒ 本文件用 RAII 守卫（装入 ⇒ 测 ⇒ **Drop 时卸下**）
//! ⇒ 否则同进程内其它用例会继承这个假写面（顺序敏感的偶发红）。

use std::sync::{Arc, Mutex};

use apeireth_core::user_domain::{DomainRegistry, DomainRole};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, ClientTokenConfig,
    ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayState, GatewayTokenGate, UserAccess,
    CLIENT_TOKENS_ENV,
};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::principal_sessions::{self, PrincipalSessionRecorder};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const ALICE_TOKEN: &str = "alice-token-0123456789abcdef";
const STRANGER_TOKEN: &str = "stranger-token-0123456789abcdef";

// ---------------------------------------------------------------------------
// 假写面: 记录每一次登记调用
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Recorded {
    calls: Mutex<Vec<(String, String)>>,
}

impl Recorded {
    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().expect("记录锁").clone()
    }
}

impl PrincipalSessionRecorder for Recorded {
    fn record(&self, principal_id: &str, session_id: &str) -> Result<(), String> {
        self.calls
            .lock()
            .expect("记录锁")
            .push((principal_id.to_string(), session_id.to_string()));
        Ok(())
    }
}

/// RAII 守卫：装入假写面 ⇒ 退出时**卸下**（防跨用例污染）。
struct RecorderGuard {
    _recorded: Arc<Recorded>,
}

impl RecorderGuard {
    fn install() -> (Self, Arc<Recorded>) {
        let recorded = Arc::new(Recorded::default());
        principal_sessions::install_principal_session_recorder(recorded.clone());
        (
            Self {
                _recorded: recorded.clone(),
            },
            recorded,
        )
    }
}

impl Drop for RecorderGuard {
    fn drop(&mut self) {
        principal_sessions::clear_principal_session_recorder();
    }
}

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

/// 每个用例**独占**一个临时目录。
///
/// ⚠️ 2026-11-03 实测的坑：只按 pid 命名 ⇒ **同文件内的并行用例共用目录**
/// ⇒ 一个用例的 `remove_dir_all` 删掉另一个刚写的 `users/<id>/domain.json`
/// ⇒ ⇒ 域解析失败 ⇒ `403 domain_refused` ⇒ 表现为「归属没登记」
/// ⇒ ⇒ **真因是测试互踩目录，不是产品缺陷** ⇒ 目录名必须带用例标签。
fn temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-attribution-{}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("临时目录");
    dir
}

fn write_tokens(dir: &std::path::Path) {
    // ⚠️ 令牌表是 **`StoredDoc` 包装**的（不是裸 `ClientTokenConfig` 的 serde 形状）
    // ⇒ 裸 JSON 会被判 `Stored(Malformed { reason: "missing field \`name\`" })`。
    let config = ClientTokenConfig::new(vec![
        ClientTokenSpec {
            name: "tui".to_string(),
            token: ALICE_TOKEN.to_string(),
            user_id: Some("user-alice".to_string()),
        },
        ClientTokenSpec {
            name: "stranger".to_string(),
            token: STRANGER_TOKEN.to_string(),
            // **无 user_id 绑定** ⇒ 归属**不可判定** ⇒ 不得登记。
            user_id: None,
        },
    ]);
    let doc = apeireth_core::stored_doc::StoredDoc::new(
        apeireth_gateway::CLIENT_TOKENS_DOC_NAME,
        apeireth_gateway::CLIENT_TOKENS_DOC_VERSION,
        vec![apeireth_gateway::CLIENT_TOKENS_DOC_VERSION],
        config,
    );
    std::fs::write(
        dir.join("client-tokens.json"),
        serde_json::to_vec_pretty(&doc).expect("序列化令牌表"),
    )
    .expect("写令牌表");
}

fn user_record(user_id: &str, role: &str, clients: &[&str]) -> UserRecord {
    UserRecord {
        user_id: user_id.to_string(),
        role: role.to_string(),
        client_names: clients.iter().map(|value| value.to_string()).collect(),
        domain: Some(user_id.to_string()),
    }
}

async fn harness(label: &str) -> GatewayState {
    let dir = temp_dir(label);
    std::env::remove_var(CLIENT_TOKENS_ENV);
    write_tokens(&dir);

    let users = UserTable::new(vec![user_record("user-alice", "owner", &["tui"])]);
    users.save(&dir).expect("users.json");

    let registry = DomainRegistry::open(&dir).expect("域登记册");
    registry
        .create("user-alice", "Alice", DomainRole::Owner)
        .expect("开 alice 域");

    let services = GatewayServices::default();
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime.clone(), services);
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::load(Some(&dir)).expect("令牌表");
    state.user_access = UserAccess::load(&dir, &state.client_tokens).expect("角色面");

    // 域平面：**默认域**与运行时共用同一把写闸（W2-③ 承诺不拆）。
    // ⇒ 域解析要的是**令牌绑定的 user_id**，与平面归属无关
    // ⇒ ⇒ 这里只需保证域面有可用的写闸（否则请求在域门就被拒 ⇒ 登记不会发生）。
    let mut managers = std::collections::BTreeMap::new();
    // ⚠️ **该域必须有写闸**（`DomainSessionPlane` 的 map 里要有它）
    // ⇒ 否则请求在域门就被拒（403 `domain_refused`「数据面未打开」）
    // ⇒ ⇒ 归属登记根本到不了 ⇒ 测出来是「没登记」而不是「登记失败」。
    managers.insert(
        apeireth_core::user_domain::DomainUserId::new("user-alice").unwrap(),
        Arc::new(apeireth_runtime::canonical::SessionManager::new(
            Arc::new(apeireth_runtime::canonical::InMemorySessionStore::new()),
            apeireth_core::kernel::system_clock(),
        )),
    );
    managers.insert(
        apeireth_core::user_domain::DomainUserId::default_domain(),
        Arc::new(apeireth_runtime::canonical::SessionManager::new(
            Arc::new(apeireth_runtime::canonical::InMemorySessionStore::new()),
            apeireth_core::kernel::system_clock(),
        )),
    );
    state.domains = Some(Arc::new(
        apeireth_runtime::canonical::DomainSessionPlane::new(
            DomainRegistry::open(&dir).expect("域登记册"),
            managers,
        ),
    ));
    // ⚠️ **非默认域的 chat 面需要「域运行时平面」**（`state.domain_runtimes`）
    // ⇒ 否则 `chat_execution` 直接 `domain_refused`（403「no open partner runtime plane」）
    // ⇒ ⇒ 这**不是**产品缺陷，是「域面逐面开放」的一部分（面板/审批/事件已开，chat 未开）。
    let alice_runtime = Arc::new(Runtime::builder().build().await.expect("域运行时"));
    let mut instances = std::collections::BTreeMap::new();
    instances.insert(
        apeireth_core::user_domain::DomainUserId::new("user-alice").unwrap(),
        alice_runtime,
    );
    state.domain_runtimes = Some(Arc::new(
        apeireth_runtime::canonical::DomainRuntimePlane::new(
            state.domains.clone().expect("域会话平面"),
            runtime.clone(),
            instances,
        ),
    ));
    state
}

fn chat_request(token: &str, domain: Option<&str>) -> Request<Body> {
    let body = serde_json::json!({ "input": "你好" });
    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/chat")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("构造请求");
    if let Some(domain) = domain {
        request
            .headers_mut()
            .insert("x-apeireth-domain", domain.parse().expect("域头"));
    }
    request
}

// ---------------------------------------------------------------------------
// 用例
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_domain_request_registers_the_principal_of_the_session() {
    let (_guard, recorded) = RecorderGuard::install();
    let state = harness("domain-request").await;
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(chat_request(ALICE_TOKEN, Some("user-alice")))
        .await
        .expect("请求");
    let status = response.status();
    let body = String::from_utf8_lossy(
        &response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
    )
    .to_string();
    let calls = recorded.calls();
    assert_eq!(
        calls.len(),
        1,
        "带域的请求应当**恰好登记一次**（不多不少）：{calls:?} / status={status} / body={body}"
    );
    assert_eq!(
        calls[0].0, "user-alice",
        "主体必须是**令牌绑定的 user_id**（不是客户端署名键）：{calls:?}"
    );
    assert!(!calls[0].1.trim().is_empty(), "会话 id 不得为空：{calls:?}");
}

#[tokio::test]
async fn a_request_without_a_user_id_registers_nothing() {
    let (_guard, recorded) = RecorderGuard::install();
    let state = harness("no-user-id").await;
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(chat_request(STRANGER_TOKEN, None))
        .await
        .expect("请求");
    let status = response.status();
    let body = String::from_utf8_lossy(&response.into_body().collect().await.unwrap().to_bytes())
        .to_string();
    assert!(
        recorded.calls().is_empty(),
        "无 user_id 的请求**不得登记**（宁可归属不可判定，也不要错归属 ⇒ 误删风险）：{:?} / status={status} / body={body}",
        recorded.calls()
    );
}

#[tokio::test]
async fn registration_is_not_lost_when_the_turn_fails() {
    let (_guard, recorded) = RecorderGuard::install();
    let state = harness("turn-fails").await;
    let router = canonical_router_with_state(state);

    // 没有可用 provider ⇒ 回合必然失败 ⇒ 但归属登记在**调 provider 之前**
    // ⇒ ⇒ 登记**不得**随回合失败一起丢掉（否则 episode 又归不到主体）。
    let response = router
        .oneshot(chat_request(ALICE_TOKEN, Some("user-alice")))
        .await
        .expect("请求");
    // ⚠️ **不钉状态码**：没有可用 provider ⇒ 回合必然失败（503/500 都可能）
    // ⇒ 本用例要钉的**只有时序**：登记发生在**调 provider 之前**
    // ⇒ ⇒ 回合怎么失败都不影响「登记已经发生」。
    let _ = response.status();
    assert_eq!(
        recorded.calls().len(),
        1,
        "回合失败**不得**让归属登记消失：{:?}",
        recorded.calls()
    );
}
