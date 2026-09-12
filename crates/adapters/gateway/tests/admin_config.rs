//! Integration proof for the `/v1/admin/config` hot-reload contract.
//!
//! The three live effects are proven end-to-end without touching an external
//! provider:
//! - `model`    -> gateway default injection, observed by a mock provider;
//! - `base_url` -> live capability setter, observed by which mock server is hit;
//! - `api_key`  -> the injected credential writer/resolver pair, observed by the
//!                 `Authorization` header on the next provider call.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, CredentialWriter,
    GatewayServices,
};
use apeireth_governance::AllowAll;
use apeireth_plugin::{CredentialResolver, Secret};
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

/// Serializes env-mutating tests (`std::env` is process-global).
static ENV_LOCK: Mutex<()> = Mutex::new(());

const SUCCESS_BODY: &str = r#"{
    "id": "chatcmpl-admin-config",
    "model": "model-b",
    "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello"}, "finish_reason": "stop"}],
    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
}"#;

struct EnvGuard(&'static str, Option<String>);

impl EnvGuard {
    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        Self(key, prev)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.1 {
            Some(value) => std::env::set_var(self.0, value),
            None => std::env::remove_var(self.0),
        }
    }
}

/// A process-local credential backend shared by the gateway's writer and the
/// runtime's resolver, mirroring the production keyring read/write split.
#[derive(Clone, Default)]
struct SharedCredentials(Arc<Mutex<HashMap<String, String>>>);

impl CredentialWriter for SharedCredentials {
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .insert(name.to_string(), value.to_string());
        Ok(())
    }
}

impl CredentialResolver for SharedCredentials {
    fn resolve(&self, name: &str) -> Option<Secret> {
        self.0.lock().unwrap().get(name).map(|value| Secret::new(value.clone()))
    }
}

struct RecordingServer {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl RecordingServer {
    async fn start(body: &'static str) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_clone = Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let requests = Arc::clone(&requests_clone);
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 8192];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    requests
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&buf[..n]).into_owned());
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });
        Self {
            base_url: format!("http://{addr}"),
            requests,
        }
    }

    fn served(&self) -> bool {
        !self.requests.lock().unwrap().is_empty()
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn post_json(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn chat_request(session: SessionId, input: &str) -> Request<Body> {
    post_json(
        "/v1/chat",
        serde_json::json!({ "session": session, "input": input }),
    )
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn admin_config_hot_update_takes_effect_on_the_next_request() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_key = EnvGuard::remove("OPENAI_API_KEY");
    let _g_url = EnvGuard::remove("APEIRETH_OPENAI_URL");
    let _g_model = EnvGuard::remove("APEIRETH_MODEL");

    let server_a = RecordingServer::start(SUCCESS_BODY).await;
    let server_b = RecordingServer::start(SUCCESS_BODY).await;

    let shared = SharedCredentials::default();
    let resolver: Arc<dyn CredentialResolver> = Arc::new(shared.clone());
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(
            server_a.base_url.clone(),
            vec!["model-a".into(), "model-b".into()],
            http,
            2_000,
        )
        .unwrap(),
    );

    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_session_store(Arc::new(InMemorySessionStore::new()))
            .with_governance(Arc::new(AllowAll))
            .with_credentials(resolver)
            .with_plugin(plugin)
            .with_default_model("model-a")
            .build()
            .await
            .unwrap(),
    );

    let mut services = GatewayServices::default();
    services.credentials = Some(Arc::new(shared.clone()));
    let state = build_gateway_state_with_services(runtime, services);
    let router = canonical_router_with_state(state);

    // Initial GET: defaults from env (which this test clears).
    let body = json_body(router.clone().oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(body["provider"], "openai-compatible");
    assert_eq!(body["base_url"], "https://api.openai.com/v1");
    assert!(body["api_key"].is_null(), "{body}");

    let update = serde_json::json!({
        "provider": "openai-compatible",
        "base_url": server_b.base_url,
        "api_key": "sk-new-key-12345c4a",
        "model": "model-b",
        "capabilities": { "streaming": true }
    });
    let response = router
        .clone()
        .oneshot(post_json("/v1/admin/config", update))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["ok"], true);
    assert!(body["warnings"].is_array(), "{body}");

    // GET reflects the new values with a masked api_key.
    let body = json_body(router.clone().oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(body["provider"], "openai-compatible");
    assert_eq!(body["base_url"], server_b.base_url);
    assert_eq!(body["api_key"], "sk-****c4a");
    assert_eq!(body["model"], "model-b");
    assert_eq!(body["capabilities"]["streaming"], true);

    // The next request (no model) reads all three hot values.
    let session = SessionId::from_uuid(Uuid::from_u128(77));
    let response = router
        .clone()
        .oneshot(chat_request(session, "hello"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["served_by"], "provider.openai-compatible");

    assert!(
        !server_a.served(),
        "the old base_url must not be hit after the hot update"
    );
    assert!(
        server_b.served(),
        "the new base_url must serve the next request"
    );
    let request = server_b.requests().into_iter().next().unwrap();
    assert!(
        request.contains("Bearer sk-new-key-12345c4a"),
        "the hot api_key must reach the provider: {request}"
    );
    assert!(
        request.contains("\"model\":\"model-b\""),
        "the hot model must be injected into the next request: {request}"
    );
}

#[tokio::test]
async fn admin_config_rejects_invalid_input_and_keeps_the_old_config() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_url = EnvGuard::remove("APEIRETH_OPENAI_URL");
    let _g_model = EnvGuard::remove("APEIRETH_MODEL");

    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let router = canonical_router_with_state(build_gateway_state_with_services(
        runtime,
        GatewayServices::default(),
    ));

    let bad_base_url = serde_json::json!({ "base_url": "not-a-url" });
    let response = router
        .clone()
        .oneshot(post_json("/v1/admin/config", bad_base_url))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_request");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("base_url"),
        "{body}"
    );
    assert!(!body["error"]["solution"].as_str().unwrap().is_empty());

    let bad_provider = serde_json::json!({ "provider": "bad provider" });
    let response = router
        .clone()
        .oneshot(post_json("/v1/admin/config", bad_provider))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "invalid_request");

    // Neither invalid patch changed the effective config.
    let body = json_body(router.clone().oneshot(get("/v1/admin/config")).await.unwrap()).await;
    assert_eq!(body["provider"], "openai-compatible");
    assert_eq!(body["base_url"], "https://api.openai.com/v1");
}
