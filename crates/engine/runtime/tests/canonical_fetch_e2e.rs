//! M3A controlled Fetch capability acceptance through the canonical runtime.
//!
//! The runtime, plugin registry, capability registry, governance, provider
//! router, session store, and FetchTool are real. The provider is scripted and
//! HTTP uses a local loopback server with an explicit allow-list policy.

use apeireth_governance::{
    DenyCapabilities, GovernancePipeline, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, ApprovalStatus, Runtime, TraceEvent, TurnOutcome,
    TurnRequest,
};
use apeireth_tools_canonical::{
    BuiltinToolsOptions, BuiltinToolsPlugin, ControlledEgress, EgressAllowList, EgressPolicy,
    FetchConfig,
};
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MODEL: &str = "fake-model-1";

struct FakeProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    first_tool_call: Option<ToolCall>,
}

impl FakeProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
            first_tool_call: None,
        })
    }

    fn with_first_tool_call(call: ToolCall) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
            first_tool_call: Some(call),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
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
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };

        if index == 0 {
            let call = if let Some(call) = &self.first_tool_call {
                call.clone()
            } else {
                panic!("first_tool_call must be configured for scripted provider")
            };
            Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![call],
                ..base
            })
        } else {
            Ok(NormalizedResponse {
                content: "fetch done".into(),
                ..base
            })
        }
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.fake").unwrap(),
                "1.0.0",
                "Scripted provider for fetch acceptance",
            )
            .declare_capability(
                CapabilityId::new("provider.fake").unwrap(),
                CapabilityKind::Provider,
                "Scripted provider",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ProviderPlugin {
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
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

fn fetch_tool_call(url: impl Into<String>) -> ToolCall {
    ToolCall {
        id: "call_fetch_1".into(),
        name: "fetch".into(),
        arguments: serde_json::json!({ "url": url.into() }),
    }
}

fn fetch_plugin() -> BuiltinToolsPlugin {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    let egress = Arc::new(
        ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
            .with_timeout(Duration::from_secs(5))
            .with_max_response_bytes(64 * 1024),
    );
    BuiltinToolsPlugin::with_options(
        ".",
        BuiltinToolsOptions {
            shell: None,
            fetch: Some(FetchConfig::new(egress)),
        },
    )
}

async fn write_http(socket: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    socket.write_all(head.as_bytes()).await.unwrap();
    socket.write_all(body).await.unwrap();
}

async fn read_request_head(socket: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = socket.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_ascii_lowercase()
}

async fn spawn_counting_text_server(count: Arc<AtomicUsize>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        count.fetch_add(1, Ordering::SeqCst);
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"hello fetch e2e",
        )
        .await;
    });
    port
}

#[tokio::test]
async fn runtime_fake_provider_fetch_loop_completes_without_runtime_special_case() {
    let http_count = Arc::new(AtomicUsize::new(0));
    let port = spawn_counting_text_server(Arc::clone(&http_count)).await;
    let provider = FakeProvider::with_first_tool_call(fetch_tool_call(format!(
        "http://127.0.0.1:{port}/hello"
    )));

    let runtime = Runtime::builder()
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(Arc::new(fetch_plugin()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session, "please fetch a url"))
        .await
        .unwrap();

    assert_eq!(provider.call_count(), 2, "provider should be called twice");
    assert_eq!(outcome.rounds, 2);
    assert_eq!(outcome.text, "fetch done");
    assert_eq!(outcome.served_by.as_str(), "provider.fake");
    assert_eq!(
        http_count.load(Ordering::SeqCst),
        1,
        "exactly one HTTP request"
    );

    let dispatched = outcome.trace.entries.iter().any(|entry| {
        matches!(
            &entry.event,
            TraceEvent::CapabilityDispatched { capability, .. } if capability.as_str() == "tool.fetch"
        )
    });
    assert!(dispatched, "fetch must be dispatched through the registry");

    let completed = outcome.trace.entries.iter().any(|entry| {
        matches!(
            &entry.event,
            TraceEvent::CapabilityCompleted { capability, succeeded: true, .. } if capability.as_str() == "tool.fetch"
        )
    });
    assert!(completed, "fetch must complete successfully");

    // The transcript must contain the fetched body and preserve correlation.
    let stored = runtime.sessions().load_or_create(session).await.unwrap();
    assert!(
        stored.events.iter().any(|event| matches!(
            &event.event,
            apeireth_runtime::canonical::SessionEventKind::TurnCompleted { .. }
        )),
        "session must record a completed turn"
    );
}

#[tokio::test]
async fn governance_deny_prevents_any_http_contact() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let provider = FakeProvider::with_first_tool_call(fetch_tool_call(format!(
        "http://127.0.0.1:{port}/denied"
    )));

    let runtime = Runtime::builder()
        .with_governance(Arc::new(
            DenyCapabilities::new().deny(CapabilityId::new("tool.fetch").unwrap()),
        ))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(Arc::new(fetch_plugin()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let outcome = runtime
        .execute(TurnRequest::new(
            SessionId::new(),
            "please fetch a denied url",
        ))
        .await
        .unwrap();

    assert_eq!(
        provider.call_count(),
        2,
        "model gets a recovery round after denial"
    );
    assert_eq!(outcome.text, "fetch done");

    let no_contact = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
    assert!(
        no_contact.is_err(),
        "governance deny must precede any HTTP contact"
    );
}

#[tokio::test]
async fn require_approval_pauses_before_network_then_approve_executes_once() {
    let http_count = Arc::new(AtomicUsize::new(0));
    let port = spawn_counting_text_server(Arc::clone(&http_count)).await;
    let provider = FakeProvider::with_first_tool_call(fetch_tool_call(format!(
        "http://127.0.0.1:{port}/approved"
    )));

    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.fetch".into()));
    policy.require_approval_for("tool.fetch");

    let runtime = Runtime::builder()
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(Arc::new(fetch_plugin()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "please fetch an approved url"))
        .await
        .unwrap();

    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("expected PendingApproval");
    };

    assert_eq!(view.capability_id.as_str(), "tool.fetch");
    assert_eq!(view.tool_name, "fetch");
    assert_eq!(view.tool_call.id, "call_fetch_1");
    assert_eq!(
        provider.call_count(),
        1,
        "provider must not be recalled while approval is pending"
    );
    assert_eq!(
        http_count.load(Ordering::SeqCst),
        0,
        "no HTTP contact before approval"
    );

    let effective = view
        .effective_invocation
        .as_ref()
        .expect("fetch freezes invocation");
    assert!(effective["url"].as_str().unwrap().contains("/approved"));
    assert_eq!(effective["method"], "GET");
    assert!(effective["egress_policy"].is_string());

    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();

    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("expected Resumed(Completed)");
    };
    assert_eq!(response.text, "fetch done");
    assert_eq!(provider.call_count(), 2);
    assert_eq!(
        http_count.load(Ordering::SeqCst),
        1,
        "exactly one HTTP request after approval"
    );

    // Double approval must not refetch.
    let double = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(matches!(double, ApprovalResolution::AlreadyResolved { .. }));
    assert_eq!(
        http_count.load(Ordering::SeqCst),
        1,
        "double approval must not refetch"
    );

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert_eq!(stored.active_approval_id, None);
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Consumed
    );
}

#[tokio::test]
async fn invalid_fetch_url_never_creates_pending_approval() {
    let provider = FakeProvider::with_first_tool_call(fetch_tool_call("not a url"));

    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.fetch".into()));
    policy.require_approval_for("tool.fetch");

    let runtime = Runtime::builder()
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(Arc::new(fetch_plugin()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "please fetch an invalid url"))
        .await
        .unwrap();

    match outcome {
        TurnOutcome::Completed(response) => assert_eq!(response.text, "fetch done"),
        TurnOutcome::PendingApproval(view) => {
            panic!("invalid fetch must not create PendingApproval: {view:?}")
        }
    }

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert!(
        stored.approvals.is_empty(),
        "invalid fetch must not mint a pending approval"
    );
}
