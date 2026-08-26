//! Deterministic proof that the real HTTP gateway entry reaches canonical
//! execution through the **migrated** minimax provider.
//!
//! This mirrors `canonical_entry_e2e.rs`, but instead of a scripted
//! `FakeProvider` it wires the real `MinimaxProviderPlugin` (a canonical
//! `ProviderCapability`, not `LegacyLlmCapability`) against a local mock vendor
//! HTTP server. The chain under test:
//!
//! ```text
//!   POST /v1/chat
//!     -> gateway transport adapter
//!     -> Runtime::execute
//!     -> ProviderRouter
//!     -> MinimaxProviderCapability (canonical)
//!     -> CredentialResolver (StaticCredentials, fake key)
//!     -> mock vendor HTTP server
//!     -> canonical response -> HTTP JSON
//! ```
//!
//! No Internet, no real API key. This proves production wiring, not merely
//! provider unit behaviour (§24).

use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_gateway::canonical_router;
use apeireth_governance::AllowAll;
use apeireth_plugin::{CredentialResolver, StaticCredentials};
use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
use apeireth_provider::credentials::MINIMAX_API_KEY;
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

const FAKE_KEY: &str = "sk-fake-gateway-key";
const MODEL: &str = "MiniMax-M3";

// ---------------------------------------------------------------------------
// Minimal one-shot mock vendor HTTP server.
// ---------------------------------------------------------------------------

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
            "message": {"role": "assistant", "content": "hello via gateway"},
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
async fn the_real_gateway_entry_serves_through_the_migrated_provider() {
    let server = MockServer::start(openai_success_body()).await;
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        MinimaxProviderPlugin::new(server.base_url(), vec![MODEL.into()], http, 2_000).unwrap(),
    );
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(StaticCredentials::new().with(MINIMAX_API_KEY, FAKE_KEY));
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

    let session = SessionId::from_uuid(Uuid::from_u128(7));
    let response = canonical_router(Arc::new(runtime))
        .oneshot(native_request(session, "say hello"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // The migrated canonical capability served the turn through the real HTTP
    // gateway entry — not a compat.* bridge id.
    assert_eq!(body["served_by"], "provider.minimax");
    assert_eq!(body["text"], "hello via gateway");
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
async fn the_gateway_reports_a_missing_credential_as_unavailable() {
    // No credential: the provider fails permanently with AuthFailed, which the
    // gateway maps to 502 (provider failure) — never a silent success and never
    // a fallback to a mock response.
    let server = MockServer::start(openai_success_body()).await;
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        MinimaxProviderPlugin::new(server.base_url(), vec![MODEL.into()], http, 2_000).unwrap(),
    );
    // NoCredentials: resolves nothing.
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

    let session = SessionId::from_uuid(Uuid::from_u128(8));
    let response = canonical_router(Arc::new(runtime))
        .oneshot(native_request(session, "say hello"))
        .await
        .unwrap();

    // AuthFailed is permanent -> RuntimeError::Provider -> HTTP 502 Bad Gateway.
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
