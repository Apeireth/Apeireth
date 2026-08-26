//! Deterministic proof that the real HTTP gateway entry reaches canonical
//! execution through the **migrated** openai-compatible provider.
//!
//! Mirrors `canonical_anthropic_entry.rs` but wires the real
//! `OpenAiCompatibleProviderPlugin` (a canonical `ProviderCapability`, not
//! `LegacyLlmCapability`) against a local mock vendor HTTP server speaking the
//! OpenAI Chat Completions protocol. The chain under test:
//!
//! ```text
//!   POST /v1/chat
//!     -> gateway transport adapter
//!     -> Runtime::execute
//!     -> ProviderRouter
//!     -> OpenAiCompatibleProviderCapability (canonical)
//!     -> CredentialResolver (StaticCredentials, fake key)
//!     -> mock vendor HTTP server
//!     -> canonical response -> HTTP JSON
//! ```
//!
//! No Internet, no real API key. The gateway entry adapter is transport-neutral;
//! it does not know which protocol the provider speaks (§44/§64).

use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_gateway::canonical_router;
use apeireth_governance::AllowAll;
use apeireth_plugin::{CredentialResolver, StaticCredentials};
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY;
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

const FAKE_KEY: &str = "sk-openai-gateway-key";
const MODEL: &str = "gpt-4o-mini";

struct MockServer {
    base_url: String,
    served: Arc<Mutex<bool>>,
}

impl MockServer {
    async fn start(body: String) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = Arc::new(Mutex::new(false));
        let served_clone = Arc::clone(&served);
        tokio::spawn(async move {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let mut out = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            out.push_str(&body);
            let _ = socket.write_all(out.as_bytes()).await;
            let _ = socket.flush().await;
            *served_clone.lock().unwrap() = true;
        });
        Self {
            base_url: format!("http://{addr}"),
            served,
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn served(&self) -> bool {
        *self.served.lock().unwrap()
    }
}

fn openai_success_body() -> String {
    serde_json::json!({
        "id": "chatcmpl-gateway",
        "model": MODEL,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hello via openai-compatible gateway"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 6, "completion_tokens": 3, "total_tokens": 9}
    })
    .to_string()
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn native_request(session: SessionId, input: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/chat")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({
                "session": session,
                "input": input
            }))
            .unwrap(),
        ))
        .unwrap()
}

#[tokio::test]
async fn the_real_gateway_entry_serves_through_the_openai_compatible_provider() {
    let server = MockServer::start(openai_success_body()).await;
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(server.base_url(), vec![MODEL.into()], http, 2_000)
            .unwrap(),
    );
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(StaticCredentials::new().with(OPENAI_COMPATIBLE_API_KEY, FAKE_KEY));
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver)
        .with_plugin(plugin)
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");

    let session = SessionId::from_uuid(Uuid::from_u128(21));
    let response = canonical_router(Arc::new(runtime))
        .oneshot(native_request(session, "say hello"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["served_by"], "provider.openai-compatible");
    assert_eq!(body["text"], "hello via openai-compatible gateway");
    assert_eq!(body["rounds"], 1);
    assert_eq!(body["usage"]["prompt_tokens"], 6);
    assert_eq!(body["usage"]["completion_tokens"], 3);
    assert_eq!(body["usage"]["total_tokens"], 9);
    assert_eq!(body["session"], session.to_string());
    assert_eq!(body["trace"]["session"], session.to_string());
    assert_eq!(body["trace_id"], body["trace"]["trace"]);
    assert!(server.served(), "the mock vendor endpoint was actually hit");
}

#[tokio::test]
async fn the_gateway_reports_an_openai_compatible_missing_credential_as_unavailable() {
    let server = MockServer::start(openai_success_body()).await;
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(server.base_url(), vec![MODEL.into()], http, 2_000)
            .unwrap(),
    );
    let resolver: Arc<dyn CredentialResolver> = Arc::new(apeireth_plugin::NoCredentials);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver)
        .with_plugin(plugin)
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");

    let session = SessionId::from_uuid(Uuid::from_u128(22));
    let response = canonical_router(Arc::new(runtime))
        .oneshot(native_request(session, "say hello"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let err = body["error"].as_str().unwrap_or("");
    assert!(
        err.contains("api_key") || err.contains("auth"),
        "the error must name the credential problem: {err}"
    );
    assert!(
        !server.served(),
        "no HTTP must be attempted when the key is missing"
    );
}
