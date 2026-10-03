//! Provider-over-Relay and Full-Turn-over-Relay acceptance (Phase 2C2 close-out).
//!
//! Two explicit, real integration tests the earlier rounds only argued by
//! construction:
//!
//! - **Provider over relay** — A sends a `NormalizedRequest` to B's provider
//!   module through an end-to-end encrypted relay circuit and gets back a full
//!   `NormalizedResponse` with every canonical field intact (content, model,
//!   finish_reason, usage, tool_calls, raw_metadata).
//! - **Full turn over relay (three nodes)** — a real `Runtime` on A runs
//!   `execute_outcome`: provider on B (via relay) returns a tool call, the tool
//!   runs on C (via relay), the provider is consulted again, and the turn
//!   completes — while agent authority (session/transcript/loop) stays on A and
//!   the relay owns none of it.
//!
//! No direct connection exists between any pair; every hop is the relay.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use apeireth_core::kernel::{
    system_clock, CapabilityId, Clock, Metadata, ModelId, NodeId, SessionId, TraceId,
};
use apeireth_governance::{Decision, GovernanceHook, GovernanceRequest};
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::relay::{signed_presence, RelayEndpoint, RelayMeshListener, RelayMeshTransport};
use apeireth_mesh::{
    InvokeMeta, ManagerExecutor, MeshListener, MeshNode, MeshNodeConfig, PluginManagerView,
    RemoteCapabilityPort, RouteSelection, TrustPolicy,
};
use apeireth_mesh_protocol::KeyFingerprint;
use apeireth_mesh_rendezvous::{RendezvousRelayServer, ServerConfig};
use apeireth_plugin::{
    CapabilityKind, NoCredentials, Plugin, PluginContext, PluginManager, PluginManifest,
    ProviderCapability, ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use async_trait::async_trait;

const ECHO_CAP: &str = "tool.mesh.echo";
const PROVIDER_CAP: &str = "provider.test";
const MODEL: &str = "remote-wan-model";

// ---------------------------------------------------------------------------
// Harness: a canonical PluginManager behind the mesh + runtime port.
// ---------------------------------------------------------------------------

struct AllowAll;
#[async_trait]
impl GovernanceHook for AllowAll {
    fn name(&self) -> &str {
        "allow_all"
    }
    async fn evaluate(&self, _r: &GovernanceRequest<'_>) -> Decision {
        Decision::Allow
    }
}

struct Holder {
    manager: Mutex<PluginManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn Clock>,
}

impl PluginManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&PluginManager) -> R) -> R {
        f(&self.manager.lock().unwrap())
    }
    fn governance(&self) -> &Arc<dyn GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

struct EchoTool {
    id: CapabilityId,
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "mesh_echo".into(),
            description: Some("echo over relay".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

/// A provider that pops canned responses off a queue and records every request.
struct ScriptedProvider {
    id: CapabilityId,
    responses: Mutex<VecDeque<NormalizedResponse>>,
    runs: Arc<AtomicUsize>,
    captured: Arc<Mutex<Vec<NormalizedRequest>>>,
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor {
            id: ModelId::new(MODEL).unwrap(),
            provider: CapabilityId::new(PROVIDER_CAP).unwrap(),
            display_name: Some("remote wan model".into()),
            context_window: Some(8_192),
            max_output_tokens: Some(1_024),
            features: vec![ModelFeature::ToolCalls],
            metadata: Metadata::new(),
        }]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        self.captured.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(ProviderError::Refused {
                provider: PROVIDER_CAP.to_string(),
                detail: "script exhausted".into(),
            })
    }
}

struct SimpleModule {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
    providers: Vec<Arc<dyn ProviderCapability>>,
}

#[async_trait]
impl Plugin for SimpleModule {
    async fn initialize(
        &self,
        _ctx: &apeireth_plugin::PluginContext,
    ) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.providers.clone()
    }
}

async fn manager_with(modules: Vec<Arc<dyn Plugin>>) -> Arc<Holder> {
    let mut manager = PluginManager::new();
    for module in modules {
        manager.register(module).unwrap();
    }
    let ctx = PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new());
    manager.start_all(&ctx).await.unwrap();
    Arc::new(Holder {
        manager: Mutex::new(manager),
        governance: Arc::new(AllowAll),
        clock: system_clock(),
    })
}

fn echo_module(runs: Arc<AtomicUsize>) -> Arc<dyn Plugin> {
    let manifest = PluginManifest::new(
        apeireth_core::kernel::PluginId::new("builtin.mesh-test").unwrap(),
        "1.0.0",
        "echo over relay",
    )
    .declare_capability(
        CapabilityId::new(ECHO_CAP).unwrap(),
        CapabilityKind::Tool,
        "echo",
    )
    .unwrap();
    Arc::new(SimpleModule {
        manifest,
        tools: vec![Arc::new(EchoTool {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            runs,
        })],
        providers: Vec::new(),
    })
}

fn provider_module(responses: Vec<NormalizedResponse>) -> (Arc<dyn Plugin>, Arc<ScriptedProvider>) {
    let provider = Arc::new(ScriptedProvider {
        id: CapabilityId::new(PROVIDER_CAP).unwrap(),
        responses: Mutex::new(responses.into()),
        runs: Arc::new(AtomicUsize::new(0)),
        captured: Arc::new(Mutex::new(Vec::new())),
    });
    let manifest = PluginManifest::new(
        apeireth_core::kernel::PluginId::new("builtin.scripted").unwrap(),
        "1.0.0",
        "scripted provider",
    )
    .declare_capability(
        CapabilityId::new(PROVIDER_CAP).unwrap(),
        CapabilityKind::Provider,
        "scripted provider",
    )
    .unwrap();
    (
        Arc::new(SimpleModule {
            manifest,
            tools: Vec::new(),
            providers: vec![provider.clone()],
        }),
        provider,
    )
}

/// A response carrying every canonical field set, for the fidelity assertion.
fn rich_response() -> NormalizedResponse {
    let mut raw = serde_json::Map::new();
    raw.insert("served_by".into(), serde_json::json!("node-b-scripted"));
    raw.insert("reasoning".into(), serde_json::json!({"chain": "step-1"}));
    NormalizedResponse {
        id: "resp-wan-1".into(),
        model: MODEL.into(),
        content: "hello from a remote-wan provider".into(),
        finish_reason: Some(NormalizedFinishReason::ToolCalls),
        usage: NormalizedUsage {
            prompt_tokens: 11,
            completion_tokens: 22,
            total_tokens: 33,
        },
        tool_calls: vec![ToolCall {
            id: "tc-1".into(),
            name: "mesh_echo".into(),
            arguments: serde_json::json!({"text": "from-provider"}),
        }],
        raw_metadata: raw,
    }
}

fn text_response(text: &str) -> NormalizedResponse {
    NormalizedResponse {
        id: "resp-2".into(),
        model: MODEL.into(),
        content: text.into(),
        finish_reason: Some(NormalizedFinishReason::Stop),
        usage: NormalizedUsage {
            prompt_tokens: 1,
            completion_tokens: 2,
            total_tokens: 3,
        },
        tool_calls: Vec::new(),
        raw_metadata: serde_json::Map::new(),
    }
}

fn request() -> NormalizedRequest {
    NormalizedRequest {
        model: MODEL.into(),
        messages: vec![NormalizedMessage::user("hi there")],
        temperature: Some(0.5),
        max_tokens: Some(256),
        stream: false,
        stop: Vec::new(),
        tools: Vec::new(),
        tool_choice: None,
        metadata: Default::default(),
    }
}

fn meta() -> InvokeMeta {
    InvokeMeta::new(SessionId::new(), TraceId::new(), 1)
}

async fn start_server() -> std::net::SocketAddr {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap(), "relay://test");
    let server = Arc::new(RendezvousRelayServer::bind(config).await.unwrap());
    let addr = server.local_addr();
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    addr
}

/// Register `node`'s presence and return its endpoint.
async fn register(
    addr: &std::net::SocketAddr,
    identity: &Arc<NodeIdentity>,
    node_id: &NodeId,
) -> Arc<RelayEndpoint> {
    let ep = RelayEndpoint::connect(
        &addr.to_string(),
        Arc::clone(identity),
        node_id.clone(),
        "relay://test",
    )
    .await
    .unwrap();
    ep.register(signed_presence(
        identity,
        node_id.clone(),
        "relay://test".into(),
        1,
        60_000,
        true,
    ))
    .await
    .unwrap();
    ep
}

/// Wire A→B over relay and complete both mesh handshakes. Returns (node_a link).
async fn connect_a_to_b(
    addr: &std::net::SocketAddr,
    a: &(Arc<MeshNode>, Arc<NodeIdentity>),
    a_id: &NodeId,
    b: &(Arc<MeshNode>, Arc<NodeIdentity>),
    b_id: &NodeId,
) {
    let ep_a = register(addr, &a.1, a_id).await;
    let ep_b = register(addr, &b.1, b_id).await;
    let opens = ep_b.take_inbound_opens().unwrap();
    let listener = RelayMeshListener::new(ep_b.clone(), b.1.clone(), opens);
    let transport = RelayMeshTransport::new(ep_a.clone(), a.1.clone());

    let b_node = b.0.clone();
    let b_task = tokio::spawn(async move {
        let conn = listener.accept().await.unwrap();
        b_node.accept(conn).await.unwrap()
    });
    let a_conn = transport.dial_node(b_id.clone()).await.unwrap();
    a.0.connect(a_conn, b_id.clone()).await.unwrap();
    b_task.await.unwrap();
    let _ = a_id;
}

// ---------------------------------------------------------------------------
// Part II: Provider-over-relay with full response fidelity.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn provider_over_relay_preserves_full_response_fidelity() {
    let addr = start_server().await;
    let (provider_module, _scripted) = provider_module(vec![rich_response()]);
    let holder_b = manager_with(vec![provider_module]).await;
    let holder_a = manager_with(vec![]).await;

    let id_a = Arc::new(NodeIdentity::generate().unwrap());
    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    let na = NodeId::new("node-a").unwrap();
    let nb = NodeId::new("node-b").unwrap();
    let trust_a = TrustPolicy::new().with_trusted_pinned(
        nb.clone(),
        "relay://b",
        KeyFingerprint::of_public_key(id_b.public_key()),
    );
    let trust_b = TrustPolicy::new().with_trusted_pinned(
        na.clone(),
        "relay://a",
        KeyFingerprint::of_public_key(id_a.public_key()),
    );
    let node_a = MeshNode::new(
        MeshNodeConfig::new(na.clone(), "a").with_trust(trust_a),
        Arc::new(ManagerExecutor::new(holder_a)),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(nb.clone(), "b").with_trust(trust_b),
        Arc::new(ManagerExecutor::new(holder_b)),
        system_clock(),
    );

    connect_a_to_b(
        &addr,
        &(node_a.clone(), id_a),
        &na,
        &(node_b.clone(), id_b),
        &nb,
    )
    .await;

    // Wait for B's provider advert to land on A.
    for _ in 0..100 {
        if !node_a.remote_providers().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        !node_a.remote_providers().is_empty(),
        "B's provider must project over relay"
    );

    let response = node_a
        .invoke_provider(
            &RouteSelection::Remote(nb.clone()),
            &CapabilityId::new(PROVIDER_CAP).unwrap(),
            request(),
            meta(),
        )
        .await
        .expect("provider rpc over relay");

    // Part II #8: assert every canonical field survived the relay intact.
    assert_eq!(response.id, "resp-wan-1");
    assert_eq!(response.model, MODEL, "model metadata preserved");
    assert_eq!(response.content, "hello from a remote-wan provider");
    assert_eq!(
        response.finish_reason,
        Some(NormalizedFinishReason::ToolCalls)
    );
    assert_eq!(response.usage.prompt_tokens, 11);
    assert_eq!(response.usage.completion_tokens, 22);
    assert_eq!(response.usage.total_tokens, 33);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "mesh_echo");
    assert_eq!(
        response.tool_calls[0].arguments,
        serde_json::json!({"text": "from-provider"})
    );
    assert_eq!(
        response
            .raw_metadata
            .get("served_by")
            .and_then(|v| v.as_str()),
        Some("node-b-scripted"),
        "served_by metadata preserved"
    );
    assert!(
        response.raw_metadata.contains_key("reasoning"),
        "reasoning/raw_metadata preserved"
    );
}

// ---------------------------------------------------------------------------
// Part III: three-node full turn over relay, agent authority on A.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn three_node_full_turn_over_relay_keeps_agent_authority() {
    let addr = start_server().await;
    // B = provider (round 1 issues a tool call; round 2 returns the final text).
    let (provider_module, _scripted) = provider_module(vec![
        {
            let mut first = rich_response();
            first.content = String::new();
            first.finish_reason = Some(NormalizedFinishReason::ToolCalls);
            first
        },
        text_response("done over relay"),
    ]);
    let holder_b = manager_with(vec![provider_module]).await;
    // C = tool (echo).
    let runs_c = Arc::new(AtomicUsize::new(0));
    let holder_c = manager_with(vec![echo_module(Arc::clone(&runs_c))]).await;
    let holder_a = manager_with(vec![]).await;

    let id_a = Arc::new(NodeIdentity::generate().unwrap());
    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    let id_c = Arc::new(NodeIdentity::generate().unwrap());
    let na = NodeId::new("node-a").unwrap();
    let nb = NodeId::new("node-b").unwrap();
    let nc = NodeId::new("node-c").unwrap();
    let fp = |i: &Arc<NodeIdentity>| KeyFingerprint::of_public_key(i.public_key());
    let trust_a = TrustPolicy::new()
        .with_trusted_pinned(nb.clone(), "relay://b", fp(&id_b))
        .with_trusted_pinned(nc.clone(), "relay://c", fp(&id_c));
    let trust_b = TrustPolicy::new().with_trusted_pinned(na.clone(), "relay://a", fp(&id_a));
    let trust_c = TrustPolicy::new().with_trusted_pinned(na.clone(), "relay://a", fp(&id_a));

    let node_a = MeshNode::new(
        MeshNodeConfig::new(na.clone(), "a").with_trust(trust_a),
        Arc::new(ManagerExecutor::new(holder_a)),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(nb.clone(), "b").with_trust(trust_b),
        Arc::new(ManagerExecutor::new(holder_b)),
        system_clock(),
    );
    let node_c = MeshNode::new(
        MeshNodeConfig::new(nc.clone(), "c").with_trust(trust_c),
        Arc::new(ManagerExecutor::new(holder_c)),
        system_clock(),
    );

    // A reaches B and C only through the relay (two circuits).
    connect_a_to_b(
        &addr,
        &(node_a.clone(), id_a.clone()),
        &na,
        &(node_b.clone(), id_b),
        &nb,
    )
    .await;
    connect_a_to_b(
        &addr,
        &(node_a.clone(), id_a.clone()),
        &na,
        &(node_c.clone(), id_c),
        &nc,
    )
    .await;

    // Wait for B's provider and C's tool to project onto A.
    for _ in 0..100 {
        if !node_a.remote_providers().is_empty() && !node_a.remote_tools().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        !node_a.remote_providers().is_empty(),
        "B's provider projects over relay"
    );
    assert!(
        !node_a.remote_tools().is_empty(),
        "C's tool projects over relay"
    );

    // A real agent runtime drives the turn loop.
    let runtime_a = Arc::new(
        apeireth_runtime::canonical::Runtime::builder()
            .with_governance(Arc::new(AllowAll))
            .build()
            .await
            .unwrap(),
    );
    runtime_a.set_remote_port(node_a.clone());
    assert!(
        runtime_a
            .tool_declarations()
            .iter()
            .any(|t| t.name == "mesh_echo"),
        "C's remote tool is an ordinary model tool on A"
    );

    let session = SessionId::new();
    let outcome = runtime_a
        .execute_outcome(
            apeireth_runtime::canonical::TurnRequest::new(session, "hello").with_model(MODEL),
        )
        .await
        .unwrap();
    let turn = outcome
        .completed()
        .expect("the full turn completes over relay");
    assert_eq!(turn.text, "done over relay");
    assert_eq!(turn.session, session, "session authority stayed on A");

    // The provider on B was consulted (rounds), and the tool ran on C.
    assert_eq!(runs_c.load(Ordering::SeqCst), 1, "the tool ran once on C");
    // Authority: A owns the loop/transcript and its registries hold none of the
    // remote implementations.
    assert!(runtime_a.plugins().plugins().is_empty());
    assert!(
        !runtime_a
            .plugins()
            .capabilities()
            .contains(&CapabilityId::new(ECHO_CAP).unwrap()),
        "A's registries stay free of C's tool implementation"
    );
    assert!(
        !runtime_a
            .plugins()
            .capabilities()
            .contains(&CapabilityId::new(PROVIDER_CAP).unwrap()),
        "A's registries stay free of B's provider implementation"
    );
}
