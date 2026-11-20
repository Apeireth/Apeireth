//! Operation idempotency over the real HTTP entry (C2-1).
//!
//! These tests drive the same router the product serves: the header is decoded
//! at the entry, admission happens before the session write gate, and a replay
//! returns the stored canonical outcome without executing the turn again.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router, canonical_router_with_state,
    GatewayServices, GatewayTokenGate,
};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as AutoConnBuilder,
    service::TowerToHyperService,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{oneshot, Notify, Semaphore},
    time::{sleep, timeout},
};
use tower::ServiceExt;
use uuid::Uuid;

const MODEL: &str = "fake-model-1";
const OPERATION_HEADER: &str = "x-apeireth-operation";

struct SingleShotProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    text: &'static str,
    gate: Option<ProviderGate>,
}

struct ProviderGate {
    entered: Notify,
    release: Semaphore,
}

impl SingleShotProvider {
    fn new(text: &'static str) -> Arc<Self> {
        Self::with_gate(text, None)
    }

    fn gated(text: &'static str) -> Arc<Self> {
        Self::with_gate(
            text,
            Some(ProviderGate {
                entered: Notify::new(),
                release: Semaphore::new(0),
            }),
        )
    }

    fn with_gate(text: &'static str, gate: Option<ProviderGate>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").expect("valid capability id"),
            calls: AtomicUsize::new(0),
            text,
            gate,
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProviderCapability for SingleShotProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).expect("valid model id"),
            self.id.clone(),
        )
        .with_feature(ModelFeature::ToolCalls)]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.gate {
            gate.entered.notify_one();
            gate.release.acquire().await.unwrap().forget();
        }
        Ok(NormalizedResponse {
            id: format!("response-{}", call + 1),
            model: request.model.clone(),
            content: self.text.to_string(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct SingleShotPlugin {
    manifest: PluginManifest,
    provider: Option<Arc<SingleShotProvider>>,
}

impl SingleShotPlugin {
    fn new(provider: Arc<SingleShotProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.single_shot").expect("valid plugin id"),
                "1.0.0",
                "deterministic single-shot provider",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "single-shot completion provider",
            )
            .expect("declared provider capability"),
            provider: Some(provider),
        })
    }
}

#[async_trait]
impl Plugin for SingleShotPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.provider
            .as_ref()
            .map(|provider| vec![Arc::clone(provider) as Arc<dyn ProviderCapability>])
            .unwrap_or_default()
    }
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .expect("valid timestamp")
            .as_datetime(),
    ))
}

async fn runtime_with(provider: Arc<SingleShotProvider>) -> Arc<Runtime> {
    let store = Arc::new(InMemorySessionStore::new());
    Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_session_store(store)
            .with_governance(Arc::new(AllowAll))
            .with_plugin(SingleShotPlugin::new(provider))
            .with_default_model(MODEL)
            .build()
            .await
            .expect("runtime builds"),
    )
}

fn chat_request(session: SessionId, input: &str, operation: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/chat")
        .header("content-type", "application/json");
    if let Some(operation) = operation {
        builder = builder.header(OPERATION_HEADER, operation);
    }
    builder
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({
                "session": session,
                "input": input,
            }))
            .expect("request serializes"),
        ))
        .expect("request builds")
}

async fn json_body(response: axum::http::Response<Body>) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

async fn transcript(runtime: &Runtime, session: SessionId) -> (usize, usize, usize) {
    let loaded = runtime
        .sessions()
        .load(&session)
        .await
        .expect("session load succeeds")
        .expect("the session exists");
    (loaded.messages.len(), loaded.events.len(), loaded.log.len())
}

#[tokio::test]
async fn the_same_key_and_payload_replays_without_executing_again() {
    let provider = SingleShotProvider::new("idempotent answer");
    let runtime = runtime_with(provider.clone()).await;
    let session = SessionId::from_uuid(Uuid::from_u128(101));

    let (first_status, first_body) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "hello", Some("op-1")))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(first_body["text"], "idempotent answer");
    assert_eq!(provider.call_count(), 1);
    let after_first = transcript(runtime.as_ref(), session).await;

    let (second_status, second_body) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "hello", Some("op-1")))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(
        second_body, first_body,
        "the replay must be the same canonical outcome as the first response"
    );
    assert_eq!(
        provider.call_count(),
        1,
        "a replay must not execute the turn again"
    );
    assert_eq!(
        transcript(runtime.as_ref(), session).await,
        after_first,
        "a replay must not append transcript, events, or log entries"
    );
}

#[tokio::test]
async fn a_different_payload_on_the_same_key_is_a_conflict() {
    let provider = SingleShotProvider::new("first answer");
    let runtime = runtime_with(provider.clone()).await;
    let session = SessionId::from_uuid(Uuid::from_u128(102));

    let (first_status, _) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "hello", Some("op-2")))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);

    let (status, body) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "goodbye", Some("op-2")))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "operation_conflict");
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn an_invalid_key_is_a_bad_request_before_session_admission() {
    let provider = SingleShotProvider::new("never served");
    let runtime = runtime_with(provider.clone()).await;
    let session = SessionId::from_uuid(Uuid::from_u128(103));

    let (status, body) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "hello", Some("Bad Key")))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
    let message = body["error"]["message"].as_str().expect("a message");
    assert!(message.contains("operation key"), "{message}");
    assert_eq!(provider.call_count(), 0);
    assert!(
        !runtime.sessions().contains(&session).await.unwrap(),
        "a rejected key must not leave a session behind"
    );
}

#[tokio::test]
async fn a_request_without_the_header_runs_twice() {
    let provider = SingleShotProvider::new("unkeyed answer");
    let runtime = runtime_with(provider.clone()).await;
    let session = SessionId::from_uuid(Uuid::from_u128(104));

    let (first_status, first_body) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "hello", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    let (second_status, second_body) = json_body(
        canonical_router(runtime.clone())
            .oneshot(chat_request(session, "hello", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(provider.call_count(), 2);
    assert_ne!(
        second_body["request"], first_body["request"],
        "without a key each request is its own turn"
    );
}

#[tokio::test]
async fn disconnected_unkeyed_native_turn_commits_with_the_original_identity() {
    disconnect_and_read_back(None).await;
}

#[tokio::test]
async fn disconnected_keyed_native_turn_retains_idempotency() {
    disconnect_and_read_back(Some("operation-before-disconnect")).await;
}

async fn disconnect_and_read_back(operation: Option<&str>) {
    const TOKEN: &str = "native-disconnect-fixture-token";
    const TEXT: &str = "answer after the client disconnected";
    const DIGEST: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    const DEADLINE: Duration = Duration::from_secs(5);
    let provider = SingleShotProvider::gated(TEXT);
    let runtime = runtime_with(provider.clone()).await;
    let mut state = build_gateway_state_with_services(runtime.clone(), GatewayServices::default());
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    let router = canonical_router_with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (closed, connection_closed) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let builder = AutoConnBuilder::new(TokioExecutor::new());
        let _ = builder
            .serve_connection_with_upgrades(
                TokioIo::new(stream),
                TowerToHyperService::new(router.clone()),
            )
            .await;
        closed.send(()).unwrap();
        axum::serve(listener, router).await.unwrap();
    });
    let session = SessionId::new();
    let body = serde_json::json!({"session":session,"input":"hello"}).to_string();
    let operation_header = operation
        .map(|key| format!("{OPERATION_HEADER}: {key}\r\n"))
        .unwrap_or_default();
    let request = format!("POST /v1/chat HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{operation_header}Connection: close\r\n\r\n{body}", body.len());
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(request.as_bytes()).await.unwrap();
    let gate = provider.gate.as_ref().unwrap();
    timeout(DEADLINE, gate.entered.notified()).await.unwrap();
    drop(socket);
    // The server drops its HTTP waiter before the provider yields a result.
    timeout(DEADLINE, connection_closed).await.unwrap().unwrap();

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(DEADLINE)
        .build()
        .unwrap();
    let read_uri = format!("http://{address}/v1/sessions/{session}/turn-result");
    let read = |request: Option<String>, trace: Option<String>| {
        let mut call = client
            .get(&read_uri)
            .bearer_auth(TOKEN)
            .query(&[("input_sha256", DIGEST)]);
        if let Some(request) = request {
            call = call.query(&[("request", request)]);
        }
        if let Some(trace) = trace {
            call = call.query(&[("trace_id", trace)]);
        }
        call
    };
    let pending = read(None, None).send().await.unwrap();
    assert_eq!(pending.status(), StatusCode::OK);
    let pending: serde_json::Value = pending.json().await.unwrap();
    assert_eq!(pending["state"], "unconfirmed");
    assert!(pending["text"].is_null());
    let request = pending["request"].as_str().unwrap().to_owned();
    let trace = pending["trace_id"].as_str().unwrap().to_owned();

    let post = || {
        client
            .post(format!("http://{address}/v1/chat"))
            .bearer_auth(TOKEN)
            .header(OPERATION_HEADER, operation.unwrap())
            .header("content-type", "application/json")
            .body(body.clone())
    };
    if operation.is_some() {
        let in_flight = post().send().await.unwrap();
        assert_eq!(in_flight.status(), StatusCode::CONFLICT);
        let in_flight: serde_json::Value = in_flight.json().await.unwrap();
        assert_eq!(in_flight["error"]["code"], "operation_in_progress");
    }
    gate.release.add_permits(1);
    let completed = timeout(DEADLINE, async {
        loop {
            let result = read(Some(request.clone()), Some(trace.clone()))
                .send()
                .await
                .unwrap();
            assert_eq!(result.status(), StatusCode::OK);
            let result: serde_json::Value = result.json().await.unwrap();
            if result["state"] == "completed" {
                break result;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the disconnected admitted turn must commit its result");
    assert_eq!(completed["text"], TEXT);
    assert_eq!(completed["session"], session.to_string());
    assert_eq!(completed["request"], request);
    assert_eq!(completed["trace_id"], trace);
    assert_eq!(completed["input_sha256"], DIGEST);

    let after = transcript(runtime.as_ref(), session).await;
    if operation.is_some() {
        let replay = post().send().await.unwrap();
        assert_eq!(replay.status(), StatusCode::OK);
        let replay: serde_json::Value = replay.json().await.unwrap();
        assert_eq!(replay["request"], request);
        assert_eq!(replay["trace_id"], trace);
        assert_eq!(replay["text"], TEXT);
    }
    let history = client
        .get(format!("http://{address}/v1/sessions/{session}/turns"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(history.status(), StatusCode::OK);
    let history: serde_json::Value = history.json().await.unwrap();
    let turns = history["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["state"], "completed");
    assert_eq!(turns[0]["request"], request);
    assert_eq!(turns[0]["trace_id"], trace);
    assert_eq!(turns[0]["text"], TEXT);
    assert_eq!(transcript(runtime.as_ref(), session).await, after);
    assert_eq!(provider.call_count(), 1);
    server.abort();
    let _ = server.await;
}
