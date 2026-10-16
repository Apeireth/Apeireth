//! W3-① 多户数据隔离 (星群数据层): 网关面的域上下文路由与越权拒绝即帧。
//!
//! 这里钉四件事:
//! 1. **无用户 = 默认域** (单户兼容): 未绑定 user_id 的客户端走默认域,
//!    既有面逐字不变 (零回归口径);
//! 2. **资源级跨域读拒/写拒**: 他域会话 id 与不存在的 id **同帧** (404,
//!    存在性不泄露), 写拒且他域账本零改动;
//! 3. **域级点名拒**: 自称域头 (`x-apeireth-domain`) ≠ 本域 → 403
//!    `domain_forbidden` **恒定帧** (目标域存在与否同帧);
//! 4. **面闸**: 非默认域只开按域路由的面 (`/v1/sessions/*`), 其余面 403
//!    fail-closed (面未按域开放, 不出默认域数据)。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{system_clock, SessionId};
use apeireth_core::user_domain::{DomainRegistry, DomainRole};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, ClientTokenConfig,
    ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayTokenGate,
};
use apeireth_runtime::canonical::{
    DomainSessionPlane, InMemorySessionStore, Runtime, Session, SessionManager,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const ALICE_TOKEN: &str = "alice-token-0123456789abcdef";
const BOB_TOKEN: &str = "bob-token-0123456789abcdef";
const STRANGER_TOKEN: &str = "stranger-token-0123456789abcdef";

const ALICE_CLIENT: &str = "tui";
const BOB_CLIENT: &str = "desk";
const STRANGER_CLIENT: &str = "stranger";

fn spec(name: &str, token: &str, user_id: Option<&str>) -> ClientTokenSpec {
    ClientTokenSpec {
        name: name.to_string(),
        token: token.to_string(),
        user_id: user_id.map(str::to_string),
    }
}

struct Harness {
    state: apeireth_gateway::GatewayState,
    alice_session: SessionId,
    bob_session: SessionId,
    default_session: SessionId,
}

/// 一份带域面的网关装配: 默认域 = 运行时那把写闸 (同一实例), 两个绑定用户
/// 各持自己的域与会话写闸; 用户表未配置 (角色面未激活, 本套件只判域不判权)。
async fn harness() -> Harness {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "apeireth-gateway-domain-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("user-alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry
        .create("user-bob", "Bob", DomainRole::Member)
        .unwrap();

    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut state = build_gateway_state_with_services(runtime.clone(), GatewayServices::default());
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::from_config(ClientTokenConfig::new(vec![
        spec(ALICE_CLIENT, ALICE_TOKEN, Some("user-alice")),
        spec(BOB_CLIENT, BOB_TOKEN, Some("user-bob")),
        spec(STRANGER_CLIENT, STRANGER_TOKEN, None),
    ]))
    .expect("valid token table");

    // 域平面: 默认域与运行时共用同一把写闸 (W2-③ 承诺不拆)。
    let alice_manager = Arc::new(SessionManager::new(
        Arc::new(InMemorySessionStore::new()),
        system_clock(),
    ));
    let bob_manager = Arc::new(SessionManager::new(
        Arc::new(InMemorySessionStore::new()),
        system_clock(),
    ));
    let mut managers = BTreeMap::new();
    managers.insert(
        apeireth_core::user_domain::DomainUserId::new("user-alice").unwrap(),
        alice_manager.clone(),
    );
    managers.insert(
        apeireth_core::user_domain::DomainUserId::new("user-bob").unwrap(),
        bob_manager.clone(),
    );
    managers.insert(
        apeireth_core::user_domain::DomainUserId::default_domain(),
        runtime.sessions_arc().clone(),
    );
    state.domains = Some(Arc::new(DomainSessionPlane::new(registry, managers)));

    // 每域一条会话 (跨域读写拒的靶子)。
    let alice_session = Session::new(SessionId::new(), system_clock().as_ref());
    alice_manager.save(&alice_session).await.unwrap();
    let bob_session = Session::new(SessionId::new(), system_clock().as_ref());
    bob_manager.save(&bob_session).await.unwrap();
    let default_session = Session::new(SessionId::new(), system_clock().as_ref());
    state
        .runtime
        .sessions()
        .save(&default_session)
        .await
        .unwrap();

    Harness {
        state,
        alice_session: alice_session.id,
        bob_session: bob_session.id,
        default_session: default_session.id,
    }
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();
    if let Some(token) = token {
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
    }
    request
}

fn claimed_domain(request: Request<Body>, domain: &str) -> Request<Body> {
    let mut request = request;
    request
        .headers_mut()
        .insert("x-apeireth-domain", domain.parse().unwrap());
    request
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

async fn router(harness: &Harness) -> axum::Router {
    canonical_router_with_state(harness.state.clone())
}

#[tokio::test]
async fn an_unbound_client_stays_on_the_default_domain_legacy_surface() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 无 user_id 绑定 = 默认域: 既有会话面逐字可用 (零回归)。
    let response = router
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.default_session),
            Some(STRANGER_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_bytes(response).await;
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["session_id"], harness.default_session.to_string());
}

#[tokio::test]
async fn cross_domain_session_reads_are_indistinguishable_from_missing() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 本域读 = 200。
    let own = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.alice_session),
            Some(ALICE_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(own.status(), StatusCode::OK);

    // 他域会话 id → 404; **同一 id** 在他域被删后再查 → 404。同 id 两态
    // (他域存在 / 全局不存在) 的跨域响应必须逐字节一致 —— 存在性不泄露
    // (帧内只回显调用方自己给的 id, 随 id 值而变的部分不含任何存在信息)。
    let foreign = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.bob_session),
            Some(ALICE_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
    let foreign_bytes = body_bytes(foreign).await;
    let value: serde_json::Value = serde_json::from_slice(&foreign_bytes).unwrap();
    assert_eq!(
        value["error"]["code"], "session_not_found",
        "跨域读必须与「不存在」共用同一错误码"
    );

    // 经 bob 自己的写面删掉那条会话, 同 id 再查 → 帧不得有任何变化。
    let deleted = router
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/v1/sessions/{}", harness.bob_session),
            Some(BOB_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);
    let gone = router
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.bob_session),
            Some(ALICE_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        foreign_bytes,
        body_bytes(gone).await,
        "跨域帧不得随他域存在性而变"
    );
}

#[tokio::test]
async fn cross_domain_session_writes_are_refused_and_leave_the_other_domain_untouched() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 跨域写 (PATCH / DELETE) → 404 同帧, 且他域会话零改动。
    let patch = request(
        "PATCH",
        &format!("/v1/sessions/{}/settings", harness.bob_session),
        Some(ALICE_TOKEN),
        Body::from(r#"{"model":"attacker/model"}"#),
    );
    let mut patch = patch;
    patch
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    let patched = router.clone().oneshot(patch).await.unwrap();
    assert_eq!(patched.status(), StatusCode::NOT_FOUND);
    let deleted = router
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/v1/sessions/{}", harness.bob_session),
            Some(ALICE_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::NOT_FOUND);

    // 对抗断言: bob 会话原样 (模型设置未被改, 会话未被删)。
    let bob_view = router
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.bob_session),
            Some(BOB_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(bob_view.status(), StatusCode::OK);
    let body = body_bytes(bob_view).await;
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["model"], serde_json::Value::Null);
}

#[tokio::test]
async fn a_claimed_domain_refusal_never_leaks_target_existence() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 自称他域 (存在) 与自称未知域 (不存在) → 403 恒定同帧。
    let claimed_existing = router
        .clone()
        .oneshot(claimed_domain(
            request(
                "GET",
                &format!("/v1/sessions/{}/settings", harness.alice_session),
                Some(ALICE_TOKEN),
                Body::empty(),
            ),
            "user-bob",
        ))
        .await
        .unwrap();
    assert_eq!(claimed_existing.status(), StatusCode::FORBIDDEN);
    let claimed_missing = router
        .clone()
        .oneshot(claimed_domain(
            request(
                "GET",
                &format!("/v1/sessions/{}/settings", harness.alice_session),
                Some(ALICE_TOKEN),
                Body::empty(),
            ),
            "nobody",
        ))
        .await
        .unwrap();
    assert_eq!(claimed_missing.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        body_bytes(claimed_existing).await,
        body_bytes(claimed_missing).await,
        "自称域帧必须恒定: 目标域存在与否不得外泄"
    );

    // 自称本域 = 放行 (自称名头只做绑定校验, 不改路由)。
    let claimed_own = router
        .oneshot(claimed_domain(
            request(
                "GET",
                &format!("/v1/sessions/{}/settings", harness.alice_session),
                Some(ALICE_TOKEN),
                Body::empty(),
            ),
            "user-alice",
        ))
        .await
        .unwrap();
    assert_eq!(claimed_own.status(), StatusCode::OK);
}

#[tokio::test]
async fn non_default_domains_are_confined_to_the_domain_routed_surface() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 非默认域: 未按域开放的面一律 403 `domain_forbidden` (不出默认域数据)。
    // (W3-③ 演进注记: `/v1/chat` 面已按域开放并走该域伙伴运行时, 见
    // `domain_partner_chat.rs` 全链证据; 本例保留未开放面的收敛断言。)
    for (method, uri, body) in [
        ("GET", "/v1/models", Body::empty()),
        ("GET", "/v1/panel/sessions", Body::empty()),
    ] {
        let response = router
            .clone()
            .oneshot(request(method, uri, Some(ALICE_TOKEN), body))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} 对非默认域必须拒绝"
        );
        let body = body_bytes(response).await;
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error"]["code"], "domain_forbidden");
    }

    // W3-③: chat 面已按域开放, 但本装配未注入伙伴运行时实例面 —— 非默认域
    // chat 仍拒开且**不用默认域代跑** (403 `domain_refused` fail-closed)。
    let mut chat = request(
        "POST",
        "/v1/chat",
        Some(ALICE_TOKEN),
        Body::from(r#"{"input":"hi"}"#),
    );
    chat.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    let response = router.clone().oneshot(chat).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = body_bytes(response).await;
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        value["error"]["code"], "domain_refused",
        "无伙伴实例的域 chat 必须拒开, 绝不代跑"
    );

    // 默认域 (未绑定) 的同一批面不受域门影响 (零回归)。
    let response = router
        .oneshot(request(
            "GET",
            "/v1/models",
            Some(STRANGER_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_bound_user_whose_domain_config_is_broken_is_refused_with_domain_refused() {
    let harness = harness().await;
    // 坏域配置 (畸形标记) —— 域面拒开, 真因入审计、响应码区分于域级越权。
    std::fs::write(
        harness
            .state
            .domains
            .as_ref()
            .unwrap()
            .registry()
            .layout()
            .domain_file(&apeireth_core::user_domain::DomainUserId::new("user-alice").unwrap()),
        b"{ not json",
    )
    .unwrap();
    let router = router(&harness).await;

    let response = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.alice_session),
            Some(ALICE_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = body_bytes(response).await;
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "domain_refused");

    // 其余域不受牵连 (一户坏配置不砖化数据面)。
    let bob = router
        .oneshot(request(
            "GET",
            &format!("/v1/sessions/{}/settings", harness.bob_session),
            Some(BOB_TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(bob.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 角色宽接口接缝 (域面建立角色 → 治理层消费, W3-② 交付记录接缝)
// ---------------------------------------------------------------------------

#[test]
fn the_domain_role_feeds_the_governance_identity_wide_interface() {
    use apeireth_core::user_domain::{DomainUserId, UserDomain};

    for (role, expected) in [
        (DomainRole::Owner, "owner"),
        (DomainRole::Member, "member"),
        (DomainRole::Guest, "guest"),
    ] {
        let domain = UserDomain::new(
            DomainUserId::new("u-1").unwrap(),
            "U",
            1_728_000_000_000,
            role,
        )
        .unwrap();
        let identity = apeireth_governance::UserIdentity::from_wide(
            domain.user_id.as_str(),
            Some(domain.role_str()),
            vec!["tui".to_string()],
        )
        .expect("域面建立的角色必须被治理层宽接口接受");
        assert_eq!(identity.role.as_str(), expected);
    }
}
