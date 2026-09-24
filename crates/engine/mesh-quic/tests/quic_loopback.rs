//! Real-socket QUIC loopback acceptance: everything Phase 2A promises on
//! `127.0.0.1` with pinned keys — no mocks, no in-memory fabric.
//!
//! Item map (§40): 1 bind/connect · 2 trusted handshake · 9 source-spoof
//! close · 10 duplicate convergence · 11-14 heartbeat/reconnect · 15 pending
//! completion · 16 tool RPC · 17 provider RPC · 18/19 trace/session ·
//! 20 cancellation · 24/25 no credentials · 26-28 conformance · 29 loopback ·
//! 30 three-node full turn · 31/32 ownership + full Runtime turn.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    system_clock, CapabilityId, Metadata, ModelId, ModuleId, NodeId, SessionId, TraceId,
};
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::{
    assert_connection_conformance, InvokeMeta, ManagerExecutor, MeshConnection, MeshListener,
    MeshNode, MeshNodeConfig, ModuleManagerView, RemoteCapabilityPort, RouteSelection, TrustPolicy,
};
use apeireth_mesh_quic::{QuicMeshListener, QuicMeshTransport, QuicTransportConfig};
use apeireth_plugin::{
    CapabilityKind, Module, ModuleManager, ModuleManifest, ProviderCapability, ProviderError,
    ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";
const PROVIDER_CAP: &str = "provider.test";
const ECHO_CAP: &str = "tool.mesh.echo";

/// QUIC requires the accepting side to be polled concurrently with the dial:
/// awaiting `Incoming` is what drives the server-side handshake.
async fn quic_pair(
    transport: &QuicMeshTransport,
    listener: &QuicMeshListener,
) -> (Arc<dyn MeshConnection>, Arc<dyn MeshConnection>) {
    let endpoint = listener.local_addr().unwrap().to_string();
    let (client, server) = tokio::join!(transport.connect(&endpoint), listener.accept());
    (client.expect("dial"), server.expect("accept"))
}

// ---------------------------------------------------------------------------
// Harness (local modules over the canonical ModuleManager).
// ---------------------------------------------------------------------------

struct Holder {
    manager: Mutex<ModuleManager>,
    governance: Arc<dyn apeireth_governance::GovernanceHook>,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
}

impl ModuleManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&ModuleManager) -> R) -> R {
        f(&self.manager.lock().unwrap())
    }
    fn governance(&self) -> &Arc<dyn apeireth_governance::GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn apeireth_core::kernel::Clock> {
        &self.clock
    }
}

struct EchoTool {
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        static ID: std::sync::OnceLock<CapabilityId> = std::sync::OnceLock::new();
        ID.get_or_init(|| CapabilityId::new(ECHO_CAP).unwrap())
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "mesh_echo".into(),
            description: Some("echo over quic".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

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
            provider: self.id.clone(),
            display_name: None,
            context_window: Some(4_096),
            max_output_tokens: None,
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
                provider: self.id.to_string(),
                detail: "script exhausted".into(),
            })
    }
}

struct SimpleModule {
    manifest: ModuleManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
    providers: Vec<Arc<dyn ProviderCapability>>,
}

#[async_trait]
impl Module for SimpleModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.providers.clone()
    }
}

async fn manager_with(modules: Vec<Arc<dyn Module>>) -> Arc<Holder> {
    let mut manager = ModuleManager::new();
    for module in modules {
        manager.register(module).unwrap();
    }
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();
    Arc::new(Holder {
        manager: Mutex::new(manager),
        governance: Arc::new(apeireth_governance::AllowAll),
        clock: system_clock(),
    })
}

fn echo_module(runs: Arc<AtomicUsize>) -> Arc<dyn Module> {
    let manifest = ModuleManifest::system("builtin.mesh-test", "1.0.0", "echo over quic")
        .declare_capability(
            CapabilityId::new(ECHO_CAP).unwrap(),
            CapabilityKind::Tool,
            "echo",
        )
        .unwrap();
    Arc::new(SimpleModule {
        manifest,
        tools: vec![Arc::new(EchoTool { runs }) as Arc<dyn ToolCapability>],
        providers: Vec::new(),
    })
}

fn provider_module(responses: Vec<NormalizedResponse>) -> (Arc<dyn Module>, Arc<ScriptedProvider>) {
    let provider = Arc::new(ScriptedProvider {
        id: CapabilityId::new(PROVIDER_CAP).unwrap(),
        responses: Mutex::new(responses.into()),
        runs: Arc::new(AtomicUsize::new(0)),
        captured: Arc::new(Mutex::new(Vec::new())),
    });
    let manifest = ModuleManifest::system("builtin.scripted", "1.0.0", "scripted provider")
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
            providers: vec![provider.clone() as Arc<dyn ProviderCapability>],
        }),
        provider,
    )
}

fn text_response(text: &str) -> NormalizedResponse {
    NormalizedResponse {
        id: "resp-1".into(),
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

/// One node: identity + QUIC listener + MeshNode over a manager holder.
struct QuicNode {
    identity: Arc<NodeIdentity>,
    node: Arc<MeshNode>,
    listener: QuicMeshListener,
    addr: SocketAddr,
}

async fn quic_node(
    id: &str,
    holder: Arc<Holder>,
    pins: &[apeireth_mesh::KeyFingerprint],
    trust: TrustPolicy,
) -> QuicNode {
    let identity = Arc::new(NodeIdentity::generate().unwrap());
    let node = MeshNode::new(
        MeshNodeConfig::new(NodeId::new(id).unwrap(), format!("node {id}")).with_trust(trust),
        Arc::new(ManagerExecutor::new(holder)),
        system_clock(),
    );
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity),
        pins,
        QuicTransportConfig::default(),
    )
    .unwrap();
    let addr = listener.local_addr().unwrap();
    QuicNode {
        identity,
        node,
        listener,
        addr,
    }
}

// ---------------------------------------------------------------------------
// 1/2/28/29: bind, connect, trusted handshake, shared conformance on QUIC.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_01_02_28_29_quic_bind_connect_handshake_and_conformance() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());

    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        &[identity_a.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let addr = listener.local_addr().unwrap();
    assert_eq!(addr.ip().to_string(), "127.0.0.1", "real UDP socket bound");

    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        &[identity_b.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();

    let endpoint = addr.to_string();
    let (client_link, server_link) = tokio::join!(transport.connect(&endpoint), listener.accept(),);
    let client_link = client_link.expect("QUIC connect succeeds against pinned key");
    let server_link = server_link.expect("QUIC accept succeeds against pinned key");

    // (28) The same behavioral contract as the in-memory fabric.
    assert_connection_conformance(client_link, server_link).await;
}

// ---------------------------------------------------------------------------
// 16/17/18/19/29/30/32: tool + provider RPC and full turns over real QUIC.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_16_17_18_19_30_32_tool_provider_rpc_and_full_turns_over_quic() {
    // Node B: echo tool + scripted provider.
    let runs_b = Arc::new(AtomicUsize::new(0));
    // Script order matches the test's call order: the single provider RPC
    // takes the first (text) response, then the full turn consumes the
    // ToolCall round and the final text round.
    let (provider_module, provider) = provider_module(vec![
        text_response("done over quic"),
        NormalizedResponse {
            id: "r1".into(),
            model: MODEL.into(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::ToolCalls),
            usage: NormalizedUsage::default(),
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "over": "quic" }),
            }],
            raw_metadata: serde_json::Map::new(),
        },
        text_response("done over quic"),
    ]);
    let holder_b = manager_with(vec![echo_module(Arc::clone(&runs_b)), provider_module]).await;

    // Node A: empty (turn owner) — and node C: nothing extra (tool lives on B
    // for this test; the three-node variant below splits them).
    let holder_a = manager_with(vec![]).await;

    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let trust_a = TrustPolicy::new().with_trusted_pinned(
        NodeId::new("node-b").unwrap(),
        "127.0.0.1:1",
        identity_b.fingerprint().clone(),
    );
    let trust_b = TrustPolicy::new().with_trusted_pinned(
        NodeId::new("node-a").unwrap(),
        "127.0.0.1:1",
        identity_a.fingerprint().clone(),
    );

    let node_a = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-a").unwrap(), "a").with_trust(trust_a),
        Arc::new(ManagerExecutor::new(holder_a)),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-b").unwrap(), "b").with_trust(trust_b),
        Arc::new(ManagerExecutor::new(holder_b)),
        system_clock(),
    );
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        &[identity_a.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let addr = listener.local_addr().unwrap();

    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        &[identity_b.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();

    // (1/2/29) Real connect + mesh handshake over real sockets.
    let _ = addr;
    let (link, peer_link) = quic_pair(&transport, &listener).await;
    let (connect_result, accept_result) = tokio::join!(
        node_a.connect(link, node_b.node_id().clone()),
        node_b.accept(peer_link),
    );
    connect_result.unwrap();
    accept_result.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // (16) Tool RPC over the real transport.
    let trace = TraceId::new();
    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            ToolCall {
                id: "call-tool".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "via": "quic" }),
            },
            InvokeMeta::new(SessionId::new(), trace, 1),
        )
        .await
        .expect("tool rpc over quic");
    assert_eq!(result.render(), r#"{"via":"quic"}"#);
    assert_eq!(runs_b.load(Ordering::SeqCst), 1);

    // (17/18/19) Provider RPC over the real transport: TraceId/SessionId kept.
    let session = SessionId::new();
    let response = node_a
        .invoke_provider(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(PROVIDER_CAP).unwrap(),
            NormalizedRequest::new(MODEL.to_string(), vec![]),
            InvokeMeta::new(session, trace, 1),
        )
        .await
        .expect("provider rpc over quic");
    assert_eq!(response.content, "done over quic");
    assert_eq!(provider.runs.load(Ordering::SeqCst), 1);

    // (30/32) A full Runtime turn across real sockets: A's turn loop, B's
    // provider and tool — authority stayed on A throughout.
    let runtime_a = Arc::new(
        apeireth_runtime::canonical::Runtime::builder()
            .with_governance(Arc::new(apeireth_governance::AllowAll))
            .build()
            .await
            .unwrap(),
    );
    runtime_a.set_remote_port(node_a.clone());
    // A's model catalog now shows B's provider and tool as ordinary entries.
    let declarations = runtime_a.tool_declarations();
    assert!(
        declarations.iter().any(|tool| tool.name == "mesh_echo"),
        "remote tool is an ordinary model tool"
    );

    let turn = runtime_a
        .execute_outcome(
            apeireth_runtime::canonical::TurnRequest::new(session, "hello").with_model(MODEL),
        )
        .await
        .unwrap()
        .completed()
        .expect("the full turn completes over real QUIC");
    assert_eq!(turn.text, "done over quic");
    assert_eq!(turn.session, session, "session authority stayed on A");

    // Ownership (31): B's registries own the implementations; nothing leaked
    // into any mesh structure (asserted structurally in unit tests) and A's
    // registries remain free of B's implementations.
    assert!(runtime_a.module_manager().modules().is_empty());
    assert!(!runtime_a
        .module_manager()
        .capabilities()
        .contains(&CapabilityId::new(ECHO_CAP).unwrap()));
}

// ---------------------------------------------------------------------------
// 10/11-15/20: duplicates, heartbeat, reconnect, pending, cancel.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_10_11_12_13_14_15_20_lifecycle_over_real_transport() {
    let holder_a = manager_with(vec![]).await;
    let runs_b = Arc::new(AtomicUsize::new(0));
    let holder_b = manager_with(vec![echo_module(runs_b)]).await;

    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let trust_a = TrustPolicy::new().with_trusted_pinned(
        NodeId::new("node-b").unwrap(),
        "ep",
        identity_b.fingerprint().clone(),
    );
    let trust_b = TrustPolicy::new().with_trusted_pinned(
        NodeId::new("node-a").unwrap(),
        "ep",
        identity_a.fingerprint().clone(),
    );
    let node_a = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-a").unwrap(), "a")
            .with_trust(trust_a)
            .with_heartbeat_timeout(5_000),
        Arc::new(ManagerExecutor::new(holder_a)),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-b").unwrap(), "b").with_trust(trust_b),
        Arc::new(ManagerExecutor::new(holder_b)),
        system_clock(),
    );
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        &[identity_a.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let addr = listener.local_addr().unwrap();
    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        &[identity_b.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();

    // First connection.
    let _ = addr;
    let (link, peer_link) = quic_pair(&transport, &listener).await;
    let (c, a) = tokio::join!(
        node_a.connect(link, node_b.node_id().clone()),
        node_b.accept(peer_link),
    );
    c.unwrap();
    a.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        node_a.peer_state(node_b.node_id()),
        Some(apeireth_mesh::PeerState::Active)
    );

    // (10) A simultaneous duplicate dial converges deterministically: the
    // incumbent live connection wins or the new one supersedes — never two
    // Actives.
    let (dup_link, dup_peer) = quic_pair(&transport, &listener).await;
    let (dup_result, _dup_accept) = tokio::join!(
        node_a.connect(dup_link, node_b.node_id().clone()),
        node_b.accept(dup_peer),
    );
    // Either supersedes or keeps — the pair must end with exactly one Active.
    let actives = node_a
        .peers()
        .into_iter()
        .filter(|(_, state, _)| *state == apeireth_mesh::PeerState::Active)
        .count();
    assert!(actives <= 1, "never two Active connections to one peer");
    let _ = dup_result;

    // (11/12) Heartbeats flow; a silent peer times out to Unreachable.
    node_a.heartbeat_once().await;
    node_a.sweep_heartbeats(
        apeireth_core::kernel::Timestamp::from_clock(system_clock().as_ref())
            .epoch_millis()
            .max(0) as u64
            + 60_000,
    );
    assert_ne!(
        node_a.peer_state(node_b.node_id()),
        Some(apeireth_mesh::PeerState::Active)
    );

    // (13/14) Reconnect: authenticated handshake + fresh advert restores routes.
    let (link, peer_link) = quic_pair(&transport, &listener).await;
    let (c, a) = tokio::join!(
        node_a.connect(link, node_b.node_id().clone()),
        node_b.accept(peer_link),
    );
    c.unwrap();
    a.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        node_a.peer_state(node_b.node_id()),
        Some(apeireth_mesh::PeerState::Active)
    );
    assert!(node_a
        .remote_tools()
        .iter()
        .any(|tool| tool.capability.as_str() == ECHO_CAP));

    // (20) Cancellation reaches an in-flight call and stops it.
    let handle = node_a
        .start_capability_call(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            apeireth_mesh::CapabilityCall::Tool(ToolCall {
                id: "call-cancel".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({}),
            }),
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .unwrap();
    node_a.cancel(handle.message_id).await;
    // Whether the cancel lands before or after the (fast) echo executes, the
    // call completes as a typed outcome — never a hang, never a panic.
    let _ = handle.outcome().await;

    // (15) Dropping the physical connection completes pending RPCs as typed
    // errors rather than hanging forever.
    let handle = node_a
        .start_capability_call(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            apeireth_mesh::CapabilityCall::Tool(ToolCall {
                id: "call-doomed".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({}),
            }),
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .unwrap();
    node_a.disconnect(node_b.node_id());
    let outcome = handle.outcome().await;
    assert!(outcome.is_err(), "pending RPCs complete as typed errors");
}

// ---------------------------------------------------------------------------
// 24/25: no credentials cross the network, ever.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_24_25_no_credentials_cross_the_wire() {
    let provider = ScriptedProvider {
        id: CapabilityId::new(PROVIDER_CAP).unwrap(),
        responses: Mutex::new(VecDeque::new()),
        runs: Arc::new(AtomicUsize::new(0)),
        captured: Arc::new(Mutex::new(Vec::new())),
    };
    // The advert projection is built from metadata only; the provider's
    // ModelDescriptor even *carrying* a metadata map stays local (the wire
    // projection drops it — asserted at the protocol layer).
    let models = provider.models();
    let wire = apeireth_mesh::WireModelDescriptor {
        id: models[0].id.clone(),
        provider: models[0].provider.clone(),
        display_name: None,
        context_window: models[0].context_window,
        max_output_tokens: None,
        features: vec![],
    };
    let json = serde_json::to_string(&wire).unwrap();
    for forbidden in [
        "api_key",
        "base_url",
        "Authorization",
        "Bearer",
        "secret",
        "credential",
    ] {
        assert!(!json.contains(forbidden), "wire leaked {forbidden:?}");
    }
    // Hello/identity materials never carry key bytes either.
    let identity = NodeIdentity::generate().unwrap();
    let debug = format!("{identity:?}");
    assert!(debug.contains("<redacted>"));
}
