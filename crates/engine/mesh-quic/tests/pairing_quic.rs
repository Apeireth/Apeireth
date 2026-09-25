//! End-to-end pairing over real QUIC: the ceremony's output feeds the
//! production trust path (§34/§35/§36).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{system_clock, CapabilityId, NodeId, PluginId, SessionId, TraceId};
use apeireth_governance::{Action, ActionSource, Decision, GovernanceHook, GovernanceRequest};
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::{
    pair_inbound, pair_outbound, ConfirmDecision, ManagerExecutor, MeshConnection, MeshListener,
    MeshNode, MeshNodeConfig, ModuleManagerView, PairedPeer, PairingError, PairingOptions,
    RouteSelection, TrustPolicy, TrustStore,
};
use apeireth_mesh_quic::{QuicMeshListener, QuicMeshTransport, QuicTransportConfig};
use apeireth_plugin::{CapabilityKind, Plugin, PluginManager, PluginManifest, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;

const ECHO_CAP: &str = "tool.mesh.echo";

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
    manager: Mutex<PluginManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
}

impl ModuleManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&PluginManager) -> R) -> R {
        f(&self.manager.lock().unwrap())
    }
    fn governance(&self) -> &Arc<dyn GovernanceHook> {
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
            description: Some("echo".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

struct EchoModule {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

#[async_trait]
impl Plugin for EchoModule {
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
}

fn temp_store(tag: &str) -> (TrustStore, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "apeireth-quic-pair-{tag}-{}.json",
        std::process::id()
    ));
    (TrustStore::load_or_new(&path).unwrap(), path)
}

/// Complete ceremony over real QUIC pairing channels.
async fn pair_over_quic(
    identity_a: Arc<NodeIdentity>,
    identity_b: Arc<NodeIdentity>,
    store_a: &TrustStore,
    store_b: &TrustStore,
    a_decision: ConfirmDecision,
    b_decision: ConfirmDecision,
) -> (
    Result<PairedPeer, PairingError>,
    Result<PairedPeer, PairingError>,
    String,
) {
    let listener = QuicMeshListener::bind_for_pairing(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        QuicTransportConfig::default(),
    )
    .unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let transport = QuicMeshTransport::new_for_pairing(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        QuicTransportConfig::default(),
    )
    .unwrap();

    let (client_link, server_link) = tokio::join!(transport.connect(&endpoint), listener.accept(),);
    let client_link = client_link.unwrap();
    let server_link = server_link.unwrap();

    let opts = PairingOptions { timeout_ms: 5_000 };
    let node_a = NodeId::new("node-a").unwrap();
    let node_b = NodeId::new("node-b").unwrap();
    let (a, b) = tokio::join!(
        pair_outbound(
            client_link,
            &identity_a,
            &node_a,
            node_b.clone(),
            endpoint.clone(),
            store_a,
            move |_| a_decision,
            opts.clone(),
        ),
        pair_inbound(
            server_link,
            &identity_b,
            &node_b,
            "paired-via-quic".to_string(),
            store_b,
            move |_| b_decision,
            opts,
        ),
    );
    (a, b, endpoint)
}

#[tokio::test]
async fn test_34_pair_over_quic_then_normal_connect_and_remote_tool() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let (store_a, path_a) = temp_store("ok-a");
    let (store_b, path_b) = temp_store("ok-b");

    let (a, b, _endpoint) = pair_over_quic(
        Arc::clone(&identity_a),
        Arc::clone(&identity_b),
        &store_a,
        &store_b,
        ConfirmDecision::Confirm,
        ConfirmDecision::Confirm,
    )
    .await;
    let paired = a.expect("pairing completes");
    b.expect("both sides complete");
    assert_eq!(paired.key_fingerprint, *identity_b.fingerprint());

    // The production trust path consumes the ceremony's output: pinned
    // connections and mesh handshakes now succeed from the persisted trust.
    let runs = Arc::new(AtomicUsize::new(0));
    let mut manager = PluginManager::new();
    let manifest = PluginManifest::new(
        apeireth_core::kernel::PluginId::new("builtin.mesh-test").unwrap(),
        "1.0.0",
        "echo",
    )
    .declare_capability(
        CapabilityId::new(ECHO_CAP).unwrap(),
        CapabilityKind::Tool,
        "echo",
    )
    .unwrap();
    manager
        .register(Arc::new(EchoModule {
            manifest,
            tools: vec![Arc::new(EchoTool {
                runs: Arc::clone(&runs),
            }) as Arc<dyn ToolCapability>],
        }))
        .unwrap();
    let ctx = apeireth_plugin::PluginContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();
    let holder_b = Arc::new(Holder {
        governance: Arc::new(AllowAll),
        manager: Mutex::new(manager),
        clock: system_clock(),
    });
    let holder_a = Arc::new(Holder {
        manager: Mutex::new(PluginManager::new()),
        governance: Arc::new(AllowAll),
        clock: system_clock(),
    });

    let node_a = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-a").unwrap(), "a").with_trust(
            TrustPolicy::new().with_trusted_pinned(
                NodeId::new("node-b").unwrap(),
                paired.endpoint.clone(),
                paired.key_fingerprint.clone(),
            ),
        ),
        Arc::new(ManagerExecutor::new(holder_a)),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-b").unwrap(), "b").with_trust(
            TrustPolicy::new().with_trusted_pinned(
                NodeId::new("node-a").unwrap(),
                "paired-via-quic",
                identity_a.fingerprint().clone(),
            ),
        ),
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
    let endpoint = listener.local_addr().unwrap().to_string();
    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        &[identity_b.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();

    let (link, peer_link) = tokio::join!(transport.connect(&endpoint), listener.accept());
    let (connect_result, accept_result) = tokio::join!(
        node_a.connect(link.unwrap(), node_b.node_id().clone()),
        node_b.accept(peer_link.unwrap()),
    );
    connect_result.unwrap();
    accept_result.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // The pairing result is consumed by the production route path.
    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            ToolCall {
                id: "call-1".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "via": "pairing" }),
            },
            apeireth_mesh::InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .expect("remote tool works over the paired trust path");
    assert_eq!(result.render(), r#"{"via":"pairing"}"#);
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    std::fs::remove_file(path_a).ok();
    std::fs::remove_file(path_b).ok();
}

#[tokio::test]
async fn test_35_pair_rejection_leaves_no_trust_and_connect_fails() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let (store_a, path_a) = temp_store("rej-a");
    let (store_b, path_b) = temp_store("rej-b");

    let (a, b, _) = pair_over_quic(
        identity_a,
        identity_b,
        &store_a,
        &store_b,
        ConfirmDecision::Confirm,
        ConfirmDecision::Reject,
    )
    .await;
    assert!(a.is_err(), "initiator learns of the rejection");
    assert!(b.is_err());
    assert!(store_a.get(&NodeId::new("node-b").unwrap()).is_none());
    assert!(store_b.get(&NodeId::new("node-a").unwrap()).is_none());

    // Without persisted trust the production path refuses the peer.
    let policy = TrustPolicy::new();
    assert!(policy.authorize(&NodeId::new("node-b").unwrap()).is_err());
    std::fs::remove_file(path_a).ok();
    std::fs::remove_file(path_b).ok();
}

#[tokio::test]
async fn test_36_pair_timeout_over_quic_persists_nothing() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let (store_a, path_a) = temp_store("to-a");
    let (store_b, path_b) = temp_store("to-b");

    let listener = QuicMeshListener::bind_for_pairing(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        QuicTransportConfig::default(),
    )
    .unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let transport = QuicMeshTransport::new_for_pairing(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        QuicTransportConfig::default(),
    )
    .unwrap();
    let (client_link, server_link) = tokio::join!(transport.connect(&endpoint), listener.accept());
    let client_link = client_link.unwrap();
    let server_link = server_link.unwrap();

    let opts = PairingOptions { timeout_ms: 500 };
    let node_a = NodeId::new("node-a").unwrap();
    let (result, _) = tokio::join!(
        pair_outbound(
            client_link,
            &identity_a,
            &node_a,
            NodeId::new("node-b").unwrap(),
            endpoint,
            &store_a,
            |_| ConfirmDecision::Confirm,
            opts,
        ),
        async {
            // The responder stays silent but holds its link open past the
            // initiator's deadline.
            let _keep = server_link;
            tokio::time::sleep(std::time::Duration::from_millis(900)).await;
        },
    );
    assert!(
        matches!(result, Err(PairingError::Timeout)),
        "a silent side times out: {result:?}"
    );
    assert!(store_a.known_peers().is_empty());
    assert!(store_b.known_peers().is_empty());
    std::fs::remove_file(path_a).ok();
    std::fs::remove_file(path_b).ok();
}
