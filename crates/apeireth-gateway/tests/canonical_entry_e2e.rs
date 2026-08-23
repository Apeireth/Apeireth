//! Deterministic proof that the real HTTP entry reaches canonical execution.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_gateway::canonical_router;
use apeireth_governance::{AllowAll, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, ModelFeature, NormalizedFinishReason,
    NormalizedRequest, NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall,
    ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, SessionEventKind};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

const MODEL: &str = "fake-model-1";

struct FakeProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    seen: Mutex<Vec<NormalizedRequest>>,
    tool_arguments: serde_json::Value,
    final_text: &'static str,
}

impl FakeProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
            tool_arguments: serde_json::json!({ "a": 1, "b": 1 }),
            final_text: "The result is 2.",
        })
    }

    fn failing_tool() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
            tool_arguments: serde_json::json!({ "a": "invalid" }),
            final_text: "The calculator failed.",
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn request(&self, index: usize) -> NormalizedRequest {
        self.seen.lock().unwrap()[index].clone()
    }
}

#[async_trait]
impl ProviderCapability for FakeProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(request.clone());

        let mut response = NormalizedResponse {
            id: format!("response-{}", call + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        match call {
            0 => {
                response.finish_reason = Some(NormalizedFinishReason::ToolCalls);
                response.tool_calls.push(ToolCall {
                    id: "call-1".into(),
                    name: "calculator".into(),
                    arguments: self.tool_arguments.clone(),
                });
            }
            1 => response.content = self.final_text.into(),
            unexpected => panic!("fake provider called an unexpected third time: {unexpected}"),
        }
        Ok(response)
    }
}

struct Calculator {
    id: CapabilityId,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for Calculator {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "calculator".into(),
            description: Some("Add two integers".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let (Some(a), Some(b)) = (call.arguments["a"].as_i64(), call.arguments["b"].as_i64())
        else {
            return ToolResult::permanent_error(&call.id, "expected integer fields a and b");
        };
        ToolResult::ok(&call.id, serde_json::json!((a + b).to_string()))
    }
}

struct TestPlugin {
    manifest: PluginManifest,
    provider: Option<Arc<FakeProvider>>,
    calculator_calls: Option<Arc<AtomicUsize>>,
}

impl TestPlugin {
    fn provider(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.fake_provider").unwrap(),
                "1.0.0",
                "deterministic provider",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "fake completion provider",
            )
            .unwrap(),
            provider: Some(provider),
            calculator_calls: None,
        })
    }

    fn calculator(calls: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.calculator").unwrap(),
                "1.0.0",
                "deterministic calculator",
            )
            .declare_capability(
                CapabilityId::new("tool.calculator").unwrap(),
                CapabilityKind::Tool,
                "add two integers",
            )
            .unwrap(),
            provider: None,
            calculator_calls: Some(calls),
        })
    }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.calculator_calls
            .as_ref()
            .map(|calls| {
                vec![Arc::new(Calculator {
                    id: CapabilityId::new("tool.calculator").unwrap(),
                    calls: Arc::clone(calls),
                }) as Arc<dyn ToolCapability>]
            })
            .unwrap_or_default()
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
            .unwrap()
            .as_datetime(),
    ))
}

fn message_text(message: &apeireth_protocol::canonical::NormalizedMessage) -> String {
    ContentPart::join_text(&message.content)
}

struct DenyInput;

#[async_trait]
impl GovernanceHook for DenyInput {
    fn name(&self) -> &str {
        "gateway.test.deny"
    }

    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::deny("blocked at canonical governance")
    }
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
async fn real_http_entry_closes_the_canonical_tool_loop() {
    let provider = FakeProvider::new();
    let calculator_calls = Arc::new(AtomicUsize::new(0));
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store.clone())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(TestPlugin::provider(provider.clone()))
        .with_plugin(TestPlugin::calculator(calculator_calls.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let session = SessionId::from_uuid(Uuid::from_u128(42));
    let request = native_request(session, "calculate 1 + 1");

    // The invocation under test is the real HTTP router, not Runtime::execute.
    let response = canonical_router(Arc::new(runtime))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["text"], "The result is 2.");
    assert_eq!(body["rounds"], 2);
    assert_eq!(body["served_by"], "provider.fake");
    assert_eq!(body["session"], session.to_string());
    assert_eq!(body["trace"]["session"], session.to_string());
    assert_eq!(body["trace_id"], body["trace"]["trace"]);
    assert_eq!(body["request"], body["trace"]["request"]);

    assert_eq!(provider.call_count(), 2);
    assert_eq!(calculator_calls.load(Ordering::SeqCst), 1);
    let second = provider.request(1);
    let tool_result = second
        .messages
        .iter()
        .find(|message| message.role == MessageRole::Tool)
        .expect("round two must carry the calculator result");
    assert_eq!(tool_result.tool_call_id.as_deref(), Some("call-1"));
    assert_eq!(message_text(tool_result), "2");

    assert_eq!(store.len().await, 1, "the entry must not fork sessions");
    let entries = body["trace"]["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
    assert!(entries
        .iter()
        .all(|entry| entry["at"].as_str() == Some("2023-11-14T22:13:20Z")));
}

#[tokio::test]
async fn real_http_entry_preserves_a_governance_denial() {
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(DenyInput))
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let session_id = SessionId::from_uuid(Uuid::from_u128(43));

    let response = canonical_router(runtime.clone())
        .oneshot(native_request(session_id, "blocked input"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(message_text(&session.messages[0]), "blocked input");
    assert!(session.revision > 0);
    assert!(session.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::GovernanceDenied {
            hook,
            reason,
            round: 1,
            ..
        } if hook == "gateway.test.deny" && reason.contains("blocked")
    )));
}

#[tokio::test]
async fn real_http_entry_preserves_a_provider_failure() {
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let session_id = SessionId::from_uuid(Uuid::from_u128(44));

    let response = canonical_router(runtime.clone())
        .oneshot(native_request(session_id, "provider attempt"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["session_id"], session_id.to_string());

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(message_text(&session.messages[0]), "provider attempt");
    assert!(session.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::ProviderFailed { error, round: 1 }
            if error.contains("no provider")
    )));
}

#[tokio::test]
async fn real_http_entry_preserves_a_tool_failure() {
    let provider = FakeProvider::failing_tool();
    let calculator_calls = Arc::new(AtomicUsize::new(0));
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(TestPlugin::provider(provider))
            .with_plugin(TestPlugin::calculator(calculator_calls.clone()))
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let session_id = SessionId::from_uuid(Uuid::from_u128(45));

    let response = canonical_router(runtime.clone())
        .oneshot(native_request(session_id, "bad calculator input"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(calculator_calls.load(Ordering::SeqCst), 1);

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert!(session.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::ToolFailed {
            capability: Some(capability),
            tool_call_id,
            error,
            round: 1,
        } if capability.as_str() == "tool.calculator"
            && tool_call_id == "call-1"
            && error.contains("integer fields")
    )));
    assert!(session.messages.iter().any(|message| {
        message.role == MessageRole::Tool && message_text(message).contains("integer fields")
    }));
}
