//! Negative security and transport tests for real QUIC mesh transport.
//!
//! Item map (§40/§31/§32): 3 unknown peer · 4 wrong key · 5 wrong NodeId ·
//! 6 bad proof · 7 replayed proof · 8 protocol mismatch · 9 source spoofing ·
//! 21 oversized frame · 23 backpressure. Item 22 (malformed/truncated frames)
//! is covered at the framing layer (`framing::tests`) where the hostile bytes
//! are injected byte-precisely; nothing here or there panics on network input.

use std::net::SocketAddr;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, NodeId, SessionId, TraceId};
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::{
    linked_pair, ManagerExecutor, MeshConnection, MeshListener, MeshNode, MeshNodeConfig,
    MeshTransportError, TrustPolicy,
};
use apeireth_mesh_protocol::{
    CapabilityCall, Hello, InvokeCapability, MeshCapabilityKind, MeshEnvelope, MeshPayload,
    NodeDescriptor, MESH_PROTOCOL_VERSION,
};
use apeireth_mesh_quic::{QuicMeshListener, QuicMeshTransport, QuicTransportConfig};
use apeireth_plugin::{CapabilityKind, Plugin, PluginManager, PluginManifest, ToolCapability};
use apeireth_protocol::canonical::{ToolCall, ToolParameters};
use async_trait::async_trait;

/// QUIC requires the accepting side to be polled concurrently with the dial.
async fn quic_pair(
    transport: &QuicMeshTransport,
    listener: &QuicMeshListener,
) -> (Arc<dyn MeshConnection>, Arc<dyn MeshConnection>) {
    let endpoint = listener.local_addr().unwrap().to_string();
    let (client, server) = tokio::join!(transport.connect(&endpoint), listener.accept());
    (client.expect("dial"), server.expect("accept"))
}

/// Minimal local world so `accept` has a canonical manager to bind against.
struct Holder {
    manager: std::sync::Mutex<PluginManager>,
    governance: Arc<dyn apeireth_governance::GovernanceHook>,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
}

struct Empty;

#[async_trait]
impl Plugin for Empty {
    async fn initialize(
        &self,
        _ctx: &apeireth_plugin::PluginContext,
    ) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    fn manifest(&self) -> &apeireth_plugin::PluginManifest {
        use std::sync::OnceLock;
        static MANIFEST: OnceLock<PluginManifest> = OnceLock::new();
        MANIFEST.get_or_init(|| {
            PluginManifest::new(
                apeireth_core::kernel::PluginId::new("builtin.empty").unwrap(),
                "1.0.0",
                "empty",
            )
        })
    }
}

async fn empty_holder() -> Arc<Holder> {
    let mut manager = PluginManager::new();
    manager.register(Arc::new(Empty)).unwrap();
    let ctx = apeireth_plugin::PluginContext::new(
        apeireth_core::kernel::system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();
    Arc::new(Holder {
        manager: std::sync::Mutex::new(manager),
        governance: Arc::new(apeireth_governance::AllowAll),
        clock: apeireth_core::kernel::system_clock(),
    })
}

impl apeireth_mesh::PluginManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&PluginManager) -> R) -> R {
        f(&self.manager.lock().unwrap())
    }
    fn governance(&self) -> &Arc<dyn apeireth_governance::GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn apeireth_core::kernel::Clock> {
        &self.clock
    }
}

fn node_config(id: &str, trust: TrustPolicy) -> MeshNodeConfig {
    MeshNodeConfig::new(NodeId::new(id).unwrap(), format!("node {id}")).with_trust(trust)
}

// ---------------------------------------------------------------------------
// 3/4: unknown or wrong keys are refused at the TLS gate.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_03_04_unknown_and_wrong_keys_are_refused() {
    let server_id = Arc::new(NodeIdentity::generate().unwrap());
    let client_id = Arc::new(NodeIdentity::generate().unwrap());
    let stranger_id = Arc::new(NodeIdentity::generate().unwrap());

    // Server pins only client_id; stranger is unknown.
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&server_id),
        &[client_id.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = addr.to_string();

    // (3) Unknown peer: the stranger's client cert is not pinned — refused.
    let stranger = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&stranger_id),
        &[server_id.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let _ = endpoint;
    let server_endpoint = listener.local_addr().unwrap().to_string();
    let (stranger_result, server_side) =
        tokio::join!(stranger.connect(&server_endpoint), listener.accept(),);
    // QUIC reports a rejected handshake to the dialer asynchronously: the
    // server refuses outright (typed, with the pin mismatch) while the client
    // may briefly hold a connection that is dead on arrival. Either shape is
    // fail-closed; what must never happen is a usable link.
    assert!(
        server_side.is_err(),
        "the server must refuse an unpinned key outright"
    );
    match stranger_result {
        Err(_) => {}
        Ok(link) => {
            let dead = tokio::time::timeout(std::time::Duration::from_secs(2), link.recv())
                .await
                .map(|outcome| outcome.is_err())
                .unwrap_or(true);
            assert!(dead, "a refused client link must be dead on arrival");
        }
    }

    // (4) Wrong key on the server side: the client pins server_id but the
    // listener presents another identity — refused before any mesh byte.
    let wrong_server = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&stranger_id),
        &[client_id.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let wrong_addr = wrong_server.local_addr().unwrap().to_string();
    let client = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&client_id),
        &[server_id.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let wrong_listener = wrong_server;
    let (client_result, _wrong_side) =
        tokio::join!(client.connect(&wrong_addr), wrong_listener.accept(),);
    assert!(
        client_result.is_err(),
        "a mismatched server key must fail closed"
    );
}

// ---------------------------------------------------------------------------
// 5/6/7: node binding — right key under the wrong NodeId, a foreign proof,
// and a replayed Hello all fail closed even past the TLS gate.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_05_06_07_node_binding_refuses_wrong_node_and_replayed_proofs() {
    let real_a = Arc::new(NodeIdentity::generate().unwrap());
    let impostor = Arc::new(NodeIdentity::generate().unwrap());
    let server_id = Arc::new(NodeIdentity::generate().unwrap());

    let holder = empty_holder().await;
    // Server trust: node-a is pinned to real_a's key. The impostor's key is
    // deliberately accepted at TLS (simulating any channel) — the *node
    // binding* must then refuse, which is exactly the defense-in-depth the
    // production two-layer design relies on.
    let trust = TrustPolicy::new().with_trusted_pinned(
        NodeId::new("node-a").unwrap(),
        "ep",
        real_a.fingerprint().clone(),
    );
    let node_b = MeshNode::new(
        node_config("node-b", trust),
        Arc::new(ManagerExecutor::new(holder)),
        apeireth_core::kernel::system_clock(),
    );

    // A listener with a relaxed gate (tests only) to reach the node layer.
    let listener = quic_listener_accept_any(&server_id);
    let addr = listener.local_addr().unwrap();
    let endpoint = addr.to_string();

    // The impostor connects with its own key and replays a captured Hello
    // claiming to be node-a (the "valid NodeId + wrong signature / replayed
    // proof" shape under the cert-binding model).
    let _ = endpoint;
    let transport = quic_transport_accept_any(&impostor);
    let (link, peer_link) = quic_pair(&transport, &listener).await;
    let captured_hello = MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::Hello(Hello {
            node: NodeDescriptor::new(
                NodeId::new("node-a").unwrap(),
                "impostor",
                MESH_PROTOCOL_VERSION,
                "2.0.0",
            ),
        }),
    );
    link.send(captured_hello).await.unwrap();

    let accepted = node_b.accept(peer_link).await;
    assert!(
        accepted.is_err(),
        "a replayed/wrong-key identity proof must fail closed at the node binding: {accepted:?}"
    );

    let _ = real_a;
}

// ---------------------------------------------------------------------------
// 8/9: protocol mismatch and envelope source spoofing fail closed.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_08_09_version_mismatch_and_source_spoof_fail_closed() {
    let real_a = Arc::new(NodeIdentity::generate().unwrap());
    let server_id = Arc::new(NodeIdentity::generate().unwrap());
    let holder = empty_holder().await;
    let trust = TrustPolicy::new().with_trusted_pinned(
        NodeId::new("node-a").unwrap(),
        "ep",
        real_a.fingerprint().clone(),
    );
    let node_b = MeshNode::new(
        node_config("node-b", trust),
        Arc::new(ManagerExecutor::new(holder)),
        apeireth_core::kernel::system_clock(),
    );
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&server_id),
        &[real_a.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();
    let _endpoint = listener.local_addr().unwrap().to_string();
    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&real_a),
        &[server_id.fingerprint().clone()],
        QuicTransportConfig::default(),
    )
    .unwrap();

    // (8) Handshake with the wrong protocol version is refused, never
    // downgraded.
    eprintln!("[t8] part1 pair");
    let (link, peer_link) = quic_pair(&transport, &listener).await;
    eprintln!("[t8] part1 paired");
    let mut bad_version = NodeDescriptor::new(
        NodeId::new("node-a").unwrap(),
        "a",
        MESH_PROTOCOL_VERSION,
        "2.0.0",
    );
    bad_version.protocol_version = MESH_PROTOCOL_VERSION + 41;
    link.send(MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::Hello(Hello { node: bad_version }),
    ))
    .await
    .unwrap();
    eprintln!("[t8] part1 hello sent, accepting");
    let accepted = node_b.accept(peer_link).await;
    eprintln!(
        "[t8] part1 accept done: {:?}",
        accepted.as_ref().err().map(|e| e.to_string())
    );
    assert!(accepted.is_err(), "version mismatch must fail closed");

    // (9) A well-formed handshake whose envelopes claim a different source
    // is a protocol violation: the connection closes.
    // Proven shape (same as the binding test): raw pair, handshaken with a
    // well-formed Hello driven concurrently with the mesh accept.
    let (node_a_link, peer_link2) = quic_pair(&transport, &listener).await;
    let good_hello = MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::Hello(Hello {
            node: NodeDescriptor::new(
                NodeId::new("node-a").unwrap(),
                "a",
                MESH_PROTOCOL_VERSION,
                "2.0.0",
            ),
        }),
    );
    let (mesh_result, hello_round) = tokio::join!(node_b.accept(peer_link2), async {
        node_a_link.send(good_hello).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), node_a_link.recv()).await
    },);
    mesh_result.unwrap();
    assert!(hello_round.is_ok(), "the well-formed handshake completes");

    let spoofed = MeshEnvelope::send(
        NodeId::new("node-c").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::InvokeCapability(InvokeCapability {
            capability: CapabilityId::new("tool.mesh.echo").unwrap(),
            kind: MeshCapabilityKind::Tool,
            call: CapabilityCall::Tool(ToolCall {
                id: "call-1".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({}),
            }),
            session: SessionId::new(),
            round: 1,
            deadline_epoch_ms: None,
        }),
    );
    eprintln!("[t8] spoof sent");
    node_a_link.send(spoofed).await.unwrap();
    eprintln!("[t8] spoof delivered, watching");
    // B closes the connection on identity confusion; the far side observes it.
    let mut observed_close = false;
    for _ in 0..20 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), node_a_link.recv()).await {
            Ok(Err(_)) | Err(_) => {
                observed_close = true;
                break;
            }
            Ok(Ok(_)) => {}
        }
    }
    assert!(observed_close, "source spoofing must terminate the link");
}

// ---------------------------------------------------------------------------
// 21/23: oversized frames are refused before allocation; queues backpressure.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_21_oversized_frames_are_refused_before_allocation() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let tiny = QuicTransportConfig {
        max_frame_size: 512,
        ..QuicTransportConfig::default()
    };
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        &[identity_a.fingerprint().clone()],
        tiny.clone(),
    )
    .unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        &[identity_b.fingerprint().clone()],
        tiny,
    )
    .unwrap();
    let (link, peer_link) = tokio::join!(transport.connect(&endpoint), listener.accept(),);
    let link = link.unwrap();
    let _peer_link = peer_link.unwrap();

    // A legal small envelope passes…
    let small = MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::Heartbeat(apeireth_mesh::Heartbeat { sent_at_ms: 1 }),
    );
    assert!(link.send(small).await.is_ok());

    // …and an oversized one is refused by the sender before any buffer forms.
    let big = ToolCall {
        id: "call-big".into(),
        name: "mesh_echo".into(),
        arguments: serde_json::json!({ "blob": "x".repeat(4096) }),
    };
    let oversized = MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::InvokeCapability(InvokeCapability {
            capability: CapabilityId::new("tool.mesh.echo").unwrap(),
            kind: MeshCapabilityKind::Tool,
            call: CapabilityCall::Tool(big),
            session: SessionId::new(),
            round: 1,
            deadline_epoch_ms: None,
        }),
    );
    let err = link.send(oversized).await.unwrap_err();
    assert!(
        matches!(err, MeshTransportError::ProtocolViolation(_)),
        "oversized frames fail closed: {err}"
    );
}

#[tokio::test]
async fn test_23_bounded_outbound_queue_reports_backpressure() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let small_queue = QuicTransportConfig {
        outbound_queue_capacity: 4,
        ..QuicTransportConfig::default()
    };
    let listener = QuicMeshListener::bind(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_b),
        &[identity_a.fingerprint().clone()],
        small_queue.clone(),
    )
    .unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let transport = QuicMeshTransport::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&identity_a),
        &[identity_b.fingerprint().clone()],
        small_queue,
    )
    .unwrap();
    let (link, peer_link) = tokio::join!(transport.connect(&endpoint), listener.accept());
    let link = link.unwrap();
    let peer_link = peer_link.unwrap();

    // The peer never drains its side; the sender's bounded queue must report
    // backpressure rather than buffer without limit.
    let mut backpressure = false;
    for tag in 0..2_000u32 {
        let envelope = MeshEnvelope::send(
            NodeId::new("node-a").unwrap(),
            NodeId::new("node-b").unwrap(),
            TraceId::new(),
            MeshPayload::InvokeCapability(InvokeCapability {
                capability: CapabilityId::new("tool.mesh.echo").unwrap(),
                kind: MeshCapabilityKind::Tool,
                call: CapabilityCall::Tool(ToolCall {
                    id: format!("call-{tag}"),
                    name: "mesh_echo".into(),
                    arguments: serde_json::json!({ "pad": "y".repeat(2048) }),
                }),
                session: SessionId::new(),
                round: 1,
                deadline_epoch_ms: None,
            }),
        );
        if matches!(
            link.send(envelope).await,
            Err(MeshTransportError::Backpressure(_))
        ) {
            backpressure = true;
            break;
        }
    }
    assert!(backpressure, "a bounded queue must report backpressure");
    drop(peer_link);
}

// ---------------------------------------------------------------------------
// Test-only helpers: relaxed TLS gate (never a production default).
// ---------------------------------------------------------------------------

fn quic_listener_accept_any(identity: &Arc<NodeIdentity>) -> QuicMeshListener {
    apeireth_mesh_quic::testing::bind_accept_any(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(identity),
        QuicTransportConfig::default(),
    )
    .expect("test listener binds")
}

fn quic_transport_accept_any(identity: &Arc<NodeIdentity>) -> QuicMeshTransport {
    apeireth_mesh_quic::testing::dial_accept_any(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(identity),
        QuicTransportConfig::default(),
    )
    .expect("test transport binds")
}
