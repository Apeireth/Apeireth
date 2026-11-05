//! admin「**只改端点、不带凭据**」时**如实告警**（2026-11-04 · 端点绑定闸的连带残留）。
//!
//! # 背景
//!
//! 端点绑定闸（`apeireth_provider::endpoint_binding::ensure_credential_bound_to_endpoint`）
//! 的口径是：`managed` = 「有 audience」**或**「有已声明绑定端点」
//! ⇒ ⇒ 两者皆无 ⇒ `Unmanaged` ⇒ ⇒ **不拦**
//! （2026-11-01 P1 的**有意权衡**：否则正常的自建端点会被全部打死）。
//!
//! 而 admin 的热更路径（`/v1/admin/config`）：
//! * **同笔**带 `base_url` + `api_key` ⇒ ⇒ `write_with_endpoints` ⇒ **绑定已登记** ✔
//! * **只带** `base_url` ⇒ ⇒ 凭据**保持原绑定**
//!   ⇒ ⇒ 原绑定在旧端点 ⇒ 下一次请求被**闸拒发**（fail-closed，安全）
//!   ⇒ ⇒ 原**从未绑定** ⇒ ⇒ 凭据**静默跟随**到新端点 ⇒ ⇒ **真实 API key 可能外泄**
//!
//! ⇒ ⇒ 缺口不在「换端点」本身（那是运维意图），而在**「未绑定的凭据会静默跟随」**。
//!
//! # 处置（**显式告警**，不是直接拒）
//!
//! 「只换端点、沿用同一把 key」对自建网关（vLLM/Ollama）是**合法**用法
//! ⇒ ⇒ 直接拒 ⇒ 功能回归；「未绑定就拒发」⇒ ⇒ 会打死正常自建端点。
//!
//! ⇒ ⇒ 按本仓纪律「**失败显式化 ⇒ 没有静默降级**」⇒ ⇒
//! **在响应里如实说出「这把凭据未被绑定 ⇒ 它将被发往新端点」**
//! ⇒ ⇒ 并告诉操作者**显式绑定的做法**（与 `base_url` 同笔提交 `api_key`）。

use std::sync::{Arc, Mutex};

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
    bindings: Arc<Mutex<Option<Vec<String>>>>,
    written: Arc<Mutex<Vec<(String, String)>>>,
}

impl StubCredentials {
    fn with_bindings(bindings: Option<Vec<String>>) -> Self {
        Self {
            bindings: Arc::new(Mutex::new(bindings)),
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
        *self.bindings.lock().unwrap() = Some(endpoints.to_vec());
        Ok(())
    }

    fn bound_endpoints(&self, _name: &str) -> Option<Vec<String>> {
        self.bindings.lock().unwrap().clone()
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
async fn state_with(credentials: Arc<StubCredentials>) -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        credentials: Some(credentials),
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

/// ⭐ **未绑定的凭据 + 只改端点** ⇒ 必须**如实告警**（它将跟随到新端点）。
#[tokio::test]
async fn an_unbound_credential_is_reported_when_only_the_endpoint_changes() {
    let credentials = Arc::new(StubCredentials::with_bindings(None));
    let state = state_with(credentials.clone()).await;
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
    assert_eq!(body["ok"], true, "{body}");

    let warnings = warnings_of(&body);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("未声明端点绑定") && w.contains("other-host.example")),
        "未绑定的凭据在只改端点时**必须**被如实告警：{warnings:?}"
    );
    // ⇒ 告警要**可操作**：告诉操作者怎么显式绑定。
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("同笔") && w.contains("api_key")),
        "告警必须告诉操作者**怎么**显式绑定：{warnings:?}"
    );
}

/// ⭐ **已声明绑定、且与新端点一致** ⇒ **不告警**（正常路径不许被噪音淹没）。
#[tokio::test]
async fn a_matching_binding_produces_no_binding_warning() {
    let credentials = Arc::new(StubCredentials::with_bindings(Some(vec![
        "https://other-host.example/v1".to_string(),
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

/// ⭐ **已声明绑定、但与新端点不一致** ⇒ 必须告警「绑定闸会拒发」。
#[tokio::test]
async fn a_mismatching_binding_is_reported_as_going_to_be_refused() {
    let credentials = Arc::new(StubCredentials::with_bindings(Some(vec![
        "https://old-host.example/v1".to_string(),
    ])));
    let state = state_with(credentials).await;
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(post_json(
            "/v1/admin/config",
            serde_json::json!({ "base_url": "https://new-host.example/v1" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let warnings = warnings_of(&body);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("不一致") && w.contains("拒发")),
        "绑定与新端点不一致 ⇒ 必须告警「闸会拒发」：{warnings:?}"
    );
}

/// ⭐ **同笔**带 `base_url` + `api_key` ⇒ 绑定**被登记** ⇒ **不**有绑定告警
/// （「换端点 + 换凭据」= 部署方明确声明 ⇒ 不该被噪音淹没）。
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
        credentials.bindings.lock().unwrap().clone(),
        Some(vec!["https://other-host.example/v1".to_string()]),
        "同笔提交必须**真的**登记绑定"
    );
}
