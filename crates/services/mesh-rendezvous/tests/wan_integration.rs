//! Real WAN-style loopback integration: one rendezvous/relay server and two
//! nodes that reach each other *only* through the relay (no direct link),
//! carrying a full mesh — presence → lookup → relay circuit → end-to-end peer
//! auth → capability advert → remote tool — plus the pin-failure case.
//!
//! The two nodes never share a socket. Everything between them is end-to-end
//! encrypted; the server forwards opaque ciphertext and can neither read it
//! (see the `e2e` unit tests) nor forge a peer's identity (this file).

use std::sync::{Arc, RwLock};

use apeireth_core::kernel::{system_clock, CapabilityId, Clock, NodeId, SessionId, TraceId};
use apeireth_governance::{Decision, GovernanceHook, GovernanceRequest};
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::relay::{signed_presence, RelayEndpoint, RelayMeshListener, RelayMeshTransport};
use apeireth_mesh::{
    InvokeMeta, ManagerExecutor, MeshError, MeshListener, MeshNode, MeshNodeConfig,
    ModuleManagerView, RouteSelection, TrustPolicy,
};
use apeireth_mesh_protocol::KeyFingerprint;
use apeireth_mesh_rendezvous::{RendezvousRelayServer, ServerConfig};
use apeireth_plugin::{CapabilityKind, Module, ModuleManager, ModuleManifest, ToolCapability};
use apeireth_protocol::canonical::{
    NormalizedTool, ToolCall, ToolOutcome, ToolParameters, ToolResult,
};
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Harness: a canonical ModuleManager behind the mesh port (mirrors the
// in-memory semantic tests, so a relay failure is never a runtime failure).
// ---------------------------------------------------------------------------

const ECHO_CAP: &str = "tool.mesh.echo";
const TEST_MODULE: &str = "builtin.mesh-test";

struct EchoTool {
    id: CapabilityId,
    name: String,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.clone(),
            description: Some("echo across the mesh".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

struct TestModule {
    manifest: ModuleManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

#[async_trait]
impl Module for TestModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
    async fn add_tool(&self, _tool: Arc<dyn ToolCapability>) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
}

struct AllowAll;
#[async_trait]
impl GovernanceHook for AllowAll {
    fn name(&self) -> &str {
        "allow_all"
    }
    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::Allow
    }
}

struct Holder {
    manager: RwLock<ModuleManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn Clock>,
}

impl ModuleManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&ModuleManager) -> R) -> R {
        f(&self.manager.read().unwrap())
    }
    fn governance(&self) -> &Arc<dyn GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

/// A node's identity + mesh presence + its canonical executor world.
struct TestNode {
    node: Arc<MeshNode>,
    identity: Arc<NodeIdentity>,
}

async fn build_node(id: &str, trust: TrustPolicy, identity: Arc<NodeIdentity>) -> TestNode {
    let mut manager = ModuleManager::new();
    let manifest = ModuleManifest::system(TEST_MODULE, "1.0.0", "builtin mesh test module")
        .declare_capability(
            CapabilityId::new(ECHO_CAP).unwrap(),
            CapabilityKind::Tool,
            "echo across the mesh",
        )
        .unwrap();
    let module: Arc<dyn Module> = Arc::new(TestModule {
        manifest,
        tools: vec![Arc::new(EchoTool {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            name: "mesh_echo".into(),
        })],
    });
    manager.register(module).unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();

    let holder = Arc::new(Holder {
        manager: RwLock::new(manager),
        governance: Arc::new(AllowAll),
        clock: system_clock(),
    });
    let node_id = NodeId::new(id).unwrap();
    let config = MeshNodeConfig::new(node_id, format!("node {id}")).with_trust(trust);
    let node = MeshNode::new(
        config,
        Arc::new(ManagerExecutor::new(Arc::clone(&holder))),
        system_clock(),
    );
    TestNode { node, identity }
}

fn meta() -> InvokeMeta {
    InvokeMeta::new(SessionId::new(), TraceId::new(), 1)
}

async fn start_server(audience: &str) -> std::net::SocketAddr {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap(), audience);
    let server = Arc::new(RendezvousRelayServer::bind(config).await.unwrap());
    let addr = server.local_addr();
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    addr
}

/// Register `node`'s presence and return its endpoint (with the inbound-opens
/// channel available for the accepting side).
async fn register_node(
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
    let presence = signed_presence(
        identity,
        node_id.clone(),
        "relay://test".into(),
        1,
        60_000,
        true,
    );
    ep.register(presence).await.unwrap();
    ep
}

// ---------------------------------------------------------------------------
// 1. Rendezvous: register / lookup / absent / malformed.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rendezvous_registers_looks_up_and_finds_unknown_absent() {
    let addr = start_server("relay://test").await;

    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    let nb = NodeId::new("node-b").unwrap();
    let _ep_b = register_node(&addr, &id_b, &nb).await;

    let id_a = Arc::new(NodeIdentity::generate().unwrap());
    let na = NodeId::new("node-a").unwrap();
    let ep_a = register_node(&addr, &id_a, &na).await;

    // A registered peer is discoverable.
    let found = ep_a.lookup(nb.clone()).await.unwrap();
    assert!(found.is_some(), "registered node must be discoverable");

    // An unknown peer is absent (not an error).
    let missing = ep_a.lookup(NodeId::new("node-zzz").unwrap()).await.unwrap();
    assert!(missing.is_none(), "unknown peer must be absent");
}

#[tokio::test]
async fn malformed_presence_is_rejected() {
    let addr = start_server("relay://test").await;
    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    let nb = NodeId::new("node-b").unwrap();
    let ep = register_node(&addr, &id_b, &nb).await;

    // An unsigned / forged presence must be refused — no bare "I am node X".
    let forged = apeireth_mesh_protocol::rendezvous::PresenceRecord::unsigned(
        nb.clone(),
        vec![0u8; 32],
        9,
        u64::MAX - 1,
        "relay://test",
        true,
        vec![0u8; 16],
    );
    assert!(
        ep.register(forged).await.is_err(),
        "unsigned presence must be rejected"
    );
}

#[tokio::test]
async fn a_wrong_audience_presence_is_refused() {
    let addr = start_server("relay://test").await;
    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    let nb = NodeId::new("node-b").unwrap();
    let ep = register_node(&addr, &id_b, &nb).await;

    // A presence signed for a *different* server audience cannot be replayed
    // here — the server refuses it.
    let foreign = signed_presence(
        &id_b,
        nb.clone(),
        "relay://elsewhere".into(),
        2,
        60_000,
        true,
    );
    assert!(
        ep.register(foreign).await.is_err(),
        "cross-server presence replay must be refused"
    );
}

// ---------------------------------------------------------------------------
// 2. WAN remote tool over relay-only: the headline transparency test.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_remote_tool_works_over_relay_only() {
    let addr = start_server("relay://test").await;
    let nb = NodeId::new("node-b").unwrap();
    let na = NodeId::new("node-a").unwrap();

    // Mutual trust with a *correct* pin is established out-of-band (pairing).
    // Here we pin each peer's real fingerprint so the mesh accepts it.
    // We need the fingerprints before building the nodes, so generate identities
    // first.
    let id_a = Arc::new(NodeIdentity::generate().unwrap());
    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    let trust_a = TrustPolicy::new().with_trusted_pinned(
        nb.clone(),
        "relay://node-b",
        KeyFingerprint::of_public_key(id_b.public_key()),
    );
    let trust_b = TrustPolicy::new().with_trusted_pinned(
        na.clone(),
        "relay://node-a",
        KeyFingerprint::of_public_key(id_a.public_key()),
    );
    let mut a = build_node("node-a", trust_a, Arc::clone(&id_a)).await;
    let mut b = build_node("node-b", trust_b, Arc::clone(&id_b)).await;

    let ep_a = register_node(&addr, &id_a, &na).await;
    let ep_b = register_node(&addr, &id_b, &nb).await;

    let opens = ep_b.take_inbound_opens().unwrap();
    let listener = RelayMeshListener::new(ep_b.clone(), id_b.clone(), opens);
    let transport = RelayMeshTransport::new(ep_a.clone(), id_a.clone());

    // B: accept a relay link (E2E handshake), then the mesh handshake.
    let b_task = tokio::spawn({
        let node = b.node.clone();
        async move {
            let conn = listener.accept().await.unwrap();
            node.accept(conn).await.unwrap()
        }
    });

    // A: dial over the relay (E2E handshake), then the mesh handshake.
    let a_conn = transport.dial_node(nb.clone()).await.unwrap();
    a.node.connect(a_conn, nb.clone()).await.unwrap();

    let peer = b_task.await.unwrap();
    assert_eq!(peer, na, "B must have authenticated A end-to-end");

    // The capability advert lands asynchronously after the handshake; wait for
    // B's projection to become routable before invoking.
    for _ in 0..100 {
        if !a.node.remote_capabilities().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        !a.node.remote_capabilities().is_empty(),
        "B's capability advert must land over the relay"
    );

    // The remote tool is routable and transparently invocable over the relay.
    let call = ToolCall {
        id: "call-relay-1".into(),
        name: "mesh_echo".into(),
        arguments: serde_json::json!({ "text": "hello-over-relay" }),
    };
    let result = a
        .node
        .invoke_tool(
            &RouteSelection::Remote(nb.clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            call,
            meta(),
        )
        .await
        .unwrap();
    match result.outcome {
        ToolOutcome::Ok { value } => {
            assert_eq!(value, serde_json::json!({ "text": "hello-over-relay" }))
        }
        other => panic!("expected ok outcome, got {other:?}"),
    }

    // The reverse direction works on the very same circuit.
    let call_b = ToolCall {
        id: "call-relay-2".into(),
        name: "mesh_echo".into(),
        arguments: serde_json::json!({ "text": "back" }),
    };
    let result_b = b
        .node
        .invoke_tool(
            &RouteSelection::Remote(na.clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            call_b,
            meta(),
        )
        .await
        .unwrap();
    match result_b.outcome {
        ToolOutcome::Ok { value } => assert_eq!(value, serde_json::json!({ "text": "back" })),
        other => panic!("expected ok outcome, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. Malicious relay / key substitution: the pin check fails closed.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_wrong_pin_fails_the_relay_connection() {
    let addr = start_server("relay://test").await;
    let nb = NodeId::new("node-b").unwrap();
    let na = NodeId::new("node-a").unwrap();

    let id_a = Arc::new(NodeIdentity::generate().unwrap());
    let id_b = Arc::new(NodeIdentity::generate().unwrap());
    // A pins the WRONG key for B (an attacker's key), as a MITM relay would
    // have A believe. B's real identity will not match this pin.
    let attacker = Arc::new(NodeIdentity::generate().unwrap());
    let trust_a = TrustPolicy::new().with_trusted_pinned(
        nb.clone(),
        "relay://node-b",
        KeyFingerprint::of_public_key(attacker.public_key()),
    );
    let trust_b = TrustPolicy::new().with_trusted_pinned(
        na.clone(),
        "relay://node-a",
        KeyFingerprint::of_public_key(id_a.public_key()),
    );
    let mut a = build_node("node-a", trust_a, Arc::clone(&id_a)).await;
    let mut b = build_node("node-b", trust_b, Arc::clone(&id_b)).await;

    let ep_a = register_node(&addr, &id_a, &na).await;
    let ep_b = register_node(&addr, &id_b, &nb).await;
    let opens = ep_b.take_inbound_opens().unwrap();
    let listener = RelayMeshListener::new(ep_b.clone(), id_b.clone(), opens);
    let transport = RelayMeshTransport::new(ep_a.clone(), id_a.clone());

    let b_task = tokio::spawn({
        let node = b.node.clone();
        async move {
            let conn = listener.accept().await.unwrap();
            node.accept(conn).await
        }
    });

    let a_conn = transport.dial_node(nb.clone()).await.unwrap();
    // A's mesh handshake must fail: B's real key does not match the pin.
    let err = a.node.connect(a_conn, nb.clone()).await.unwrap_err();
    assert!(
        matches!(err, MeshError::Untrusted { .. }),
        "pin mismatch must be a trust failure, got {err:?}"
    );

    // B's side also fails (or is abandoned) since the handshake never completed.
    let _ = b_task.await;
}
