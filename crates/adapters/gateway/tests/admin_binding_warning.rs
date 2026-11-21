//! Admin credential-to-endpoint transfer defenses.
//!
//! A cross-origin `base_url` change may reuse an existing credential only when
//! the target origin was already declared for that credential. Custom endpoints
//! remain supported by an explicit `base_url` + `api_key` compound write. A
//! writer that cannot commit that compound operation must leave every side
//! unchanged.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, ClientTokenConfig,
    ClientTokenSpec, ClientTokenTable, CredentialWriter, GatewayServices, GatewayState,
    GatewayTokenGate, UserAccess,
};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const OWNER_TOKEN: &str = "owner-token-0123456789abcdef";
const OWNER_CLIENT: &str = "desktop-shell";
const OWNER_USER: &str = "user-owner";

/// 凭据写入端：**可配置**它「已声明的绑定端点」。
#[derive(Clone, Default)]
struct StubCredentials {
    bindings: Arc<Mutex<HashMap<String, Vec<String>>>>,
    written: Arc<Mutex<Vec<(String, String)>>>,
}

impl StubCredentials {
    fn with_bindings(bindings: Option<Vec<String>>) -> Self {
        let mut declared = HashMap::new();
        if let Some(bindings) = bindings {
            declared.insert("provider.openai-compatible.api_key".to_string(), bindings);
        }
        Self {
            bindings: Arc::new(Mutex::new(declared)),
            written: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl CredentialWriter for StubCredentials {
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        self.written
            .lock()
            .unwrap()
            .push((name.to_string(), value.to_string()));
        Ok(())
    }

    fn write_with_endpoints(
        &self,
        name: &str,
        value: &str,
        endpoints: &[String],
    ) -> Result<(), String> {
        self.written
            .lock()
            .unwrap()
            .push((name.to_string(), value.to_string()));
        self.bindings
            .lock()
            .unwrap()
            .insert(name.to_string(), endpoints.to_vec());
        Ok(())
    }

    fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
        self.bindings.lock().unwrap().get(name).cloned()
    }
}

#[derive(Clone, Default)]
struct WriteOnlyCredentials {
    written: Arc<Mutex<Vec<(String, String)>>>,
}

impl CredentialWriter for WriteOnlyCredentials {
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        self.written
            .lock()
            .unwrap()
            .push((name.to_string(), value.to_string()));
        Ok(())
    }
}

struct BlockingCredentials {
    inner: StubCredentials,
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    blocked_once: AtomicBool,
}

impl CredentialWriter for BlockingCredentials {
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        self.inner.write(name, value)
    }

    fn write_with_endpoints(
        &self,
        name: &str,
        value: &str,
        endpoints: &[String],
    ) -> Result<(), String> {
        self.inner.write_with_endpoints(name, value, endpoints)
    }

    fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
        if !self.blocked_once.swap(true, Ordering::SeqCst) {
            self.entered.wait();
            self.release.wait();
        }
        self.inner.bound_endpoints(name)
    }
}

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

/// 装配：显式令牌门 + 显式令牌表 + **角色面激活**（owner）
/// ⇒ 管理面可达（P0-1：角色面未激活 ⇒ 管理面 fail-closed）。
async fn state_with(credentials: Arc<dyn CredentialWriter>) -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        credentials: Some(credentials),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    {
        let mut config = state.hot_config.write().unwrap();
        config.provider = "openai-compatible".to_string();
        config.base_url = "https://api.openai.com/v1".to_string();
        config.api_key = None;
    }
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens =
        ClientTokenTable::from_config(ClientTokenConfig::new(vec![ClientTokenSpec {
            name: OWNER_CLIENT.to_string(),
            token: OWNER_TOKEN.to_string(),
            user_id: Some(OWNER_USER.to_string()),
        }]))
        .expect("valid token table");
    state.user_access = UserAccess::from_table(owner_table());
    state
}

fn post_json(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {OWNER_TOKEN}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {OWNER_TOKEN}"))
        .body(Body::empty())
        .unwrap()
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn warnings_of(body: &serde_json::Value) -> Vec<String> {
    body["warnings"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// An undeclared cross-origin destination is rejected before config mutation.
#[tokio::test]
async fn an_unbound_credential_cannot_follow_an_endpoint_change() {
    let credentials = Arc::new(StubCredentials::with_bindings(None));
    let state = state_with(credentials.clone()).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({ "base_url": "https://other-host.example/v1" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_request", "{body}");
    assert!(
        body["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("api_key")),
        "{body}"
    );
    let current = json_body(router.oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(current["base_url"], "https://api.openai.com/v1");
    assert!(credentials.written.lock().unwrap().is_empty());
}

/// A binding is origin-scoped, so a different path at that origin is allowed.
#[tokio::test]
async fn a_matching_binding_produces_no_binding_warning() {
    let credentials = Arc::new(StubCredentials::with_bindings(Some(vec![
        "https://other-host.example/declared/path".to_string(),
    ])));
    let state = state_with(credentials).await;
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({ "base_url": "https://other-host.example/v1" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let warnings = warnings_of(&body);
    assert!(
        !warnings.iter().any(|w| w.contains("绑定")),
        "绑定与新端点一致 ⇒ 不该有绑定告警：{warnings:?}"
    );
}

/// A binding for another origin does not authorize credential transfer.
#[tokio::test]
async fn a_mismatching_binding_is_refused_before_the_endpoint_changes() {
    let credentials = Arc::new(StubCredentials::with_bindings(Some(vec![
        "https://old-host.example/v1".to_string(),
    ])));
    let state = state_with(credentials).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({ "base_url": "https://new-host.example/v1" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_request", "{body}");
    let current = json_body(router.oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(current["base_url"], "https://api.openai.com/v1");
}

/// Reusing the previous provider's URL does not authorize a different provider key.
#[tokio::test]
async fn a_provider_switch_with_the_old_url_still_requires_an_explicit_key_binding() {
    let credentials = Arc::new(StubCredentials::with_bindings(Some(vec![
        "https://api.openai.com/v1".to_string(),
    ])));
    let state = state_with(credentials).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({
                "provider": "custom-provider",
                "base_url": "https://api.openai.com/v1",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_request", "{body}");

    let current = json_body(router.oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(current["provider"], "openai-compatible");
    assert_eq!(current["base_url"], "https://api.openai.com/v1");
}

/// An explicit custom endpoint and credential remain supported.
#[tokio::test]
async fn declaring_the_binding_in_the_same_patch_produces_no_warning() {
    let credentials = Arc::new(StubCredentials::with_bindings(None));
    let state = state_with(credentials.clone()).await;
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({
                "base_url": "https://other-host.example/v1",
                "api_key": "sk-test-key-1234567890",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let warnings = warnings_of(&body);
    assert!(
        !warnings.iter().any(|w| w.contains("绑定")),
        "同笔提交 api_key ⇒ 绑定已登记 ⇒ 不该有绑定告警：{warnings:?}"
    );
    // ⇒ 绑定确实登记了（不是「静默丢弃绑定」）。
    assert_eq!(
        credentials
            .bindings
            .lock()
            .unwrap()
            .get("provider.openai-compatible.api_key")
            .cloned(),
        Some(vec!["https://other-host.example/v1".to_string()]),
        "同笔提交必须**真的**登记绑定"
    );
}

/// Same-origin path changes do not move a credential to a new destination.
#[tokio::test]
async fn an_unbound_credential_can_keep_using_the_same_origin() {
    let credentials = Arc::new(StubCredentials::with_bindings(None));
    let state = state_with(credentials).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({ "base_url": "https://api.openai.com/custom/v1" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let current = json_body(router.oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(current["base_url"], "https://api.openai.com/custom/v1");
}

/// Validation and mutation share one hot-config guard, so two admin patches
/// cannot both authorize themselves against the same stale destination.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_endpoint_patches_are_serialized_through_validation() {
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let credentials = Arc::new(BlockingCredentials {
        inner: StubCredentials::with_bindings(Some(vec![
            "https://other-host.example/v1".to_string()
        ])),
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
        blocked_once: AtomicBool::new(false),
    });
    let state = state_with(credentials).await;
    let router = canonical_router_with_state(state);

    let first_router = router.clone();
    let first = tokio::spawn(async move {
        first_router
            .oneshot(post_json(
                "/v1/admin/config",
                serde_json::json!({ "base_url": "https://other-host.example/v1" }),
            ))
            .await
            .unwrap()
    });
    tokio::task::spawn_blocking(move || entered.wait())
        .await
        .unwrap();

    let second_router = router.clone();
    let second = tokio::spawn(async move {
        second_router
            .oneshot(post_json(
                "/v1/admin/config",
                serde_json::json!({ "base_url": "https://api.openai.com/alternate" }),
            ))
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let second_finished_while_first_blocked = second.is_finished();

    tokio::task::spawn_blocking(move || release.wait())
        .await
        .unwrap();
    assert_eq!(first.await.unwrap().status(), StatusCode::OK);
    assert_eq!(second.await.unwrap().status(), StatusCode::CONFLICT);
    assert!(
        !second_finished_while_first_blocked,
        "the second patch must wait while the first validates under the config write guard"
    );
}

/// The default compound-write implementation is fail-closed and side-effect free.
#[tokio::test]
async fn an_unsupported_compound_write_does_not_partially_mutate() {
    let credentials = Arc::new(WriteOnlyCredentials::default());
    let state = state_with(credentials.clone()).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({
                "base_url": "https://custom-endpoint.example/v1",
                "api_key": "synthetic-test-credential",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "internal", "{body}");
    assert!(
        !body.to_string().contains("synthetic-test-credential"),
        "credential values must not enter error bodies: {body}"
    );
    assert!(credentials.written.lock().unwrap().is_empty());

    let current = json_body(router.oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(current["base_url"], "https://api.openai.com/v1");
    assert!(current["api_key"].is_null(), "{current}");
}
