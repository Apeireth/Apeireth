//! The production punch path, end to end — on loopback, through the real
//! connection flow, with no test-only shortcut.
//!
//! What is proven here: two `MeshNode`s holding each other's **verified**
//! presence can establish a mutually authenticated connection by
//! simultaneous open driven by [`MeshNode::connect_punched`] — the production
//! entry point — over one dual-role QUIC socket, converge to exactly one
//! live link, and then run a real remote tool invocation whose *result
//! content* is checked, with the executing side counting exactly one
//! execution (two physical links must not become two calls).
//!
//! What is NOT proven here: NAT traversal. Loopback has no NAT; the punch
//! mechanics are exercised but no mapping is crossed. The Linux namespace
//! harness (`scripts/test-mesh-nat.sh`) is the NAT evidence, and it must be
//! run on Linux before anything may be called traversal-supported.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::{system_clock, CapabilityId, Clock, PluginId, SessionId, TraceId};
use apeireth_governance::GovernanceHook;
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::relay::{now_ms, signed_presence_with_candidates};
use apeireth_mesh::{
    InvokeMeta, ManagerExecutor, MeshError, MeshNode, MeshNodeConfig, PluginManagerView,
    RouteSelection, TrustPolicy,
};
use apeireth_mesh_protocol::rendezvous::{
    canonicalize_candidates, EndpointCandidate, EndpointCandidateKind, PresenceRecord,
    VerifiedPresence, PRESENCE_NONCE_BYTES,
};
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::QuicPeerEndpoint;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManager, PluginManifest, ToolCapability,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;

const ECHO_CAP: &str = "tool.mesh.echo";
const TEST_MODULE: &str = "builtin.mesh-test";
const AUDIENCE: &str = "loopback.test";
const WINDOW_MS: u64 = 10_000;

// ---------------------------------------------------------------------------
// Harness: a canonical PluginManager behind the mesh port.
// ---------------------------------------------------------------------------

struct Holder {
    manager: Mutex<PluginManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn Clock>,
    echo_runs: Arc<AtomicUsize>,
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
            description: Some("mesh echo test capability".into()),
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
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
    async fn initialize(&self, _ctx: &PluginContext) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
}

/// A node with the canonical echo module and a governance hook that allows.
async fn node_with_echo(id: &str, trust: TrustPolicy) -> (Arc<MeshNode>, Arc<Holder>) {
    let runs = Arc::new(AtomicUsize::new(0));
    let manifest = PluginManifest::new(
        PluginId::new(TEST_MODULE).unwrap(),
        "1.0.0",
        "builtin mesh test module",
    )
    .declare_capability(
        CapabilityId::new(ECHO_CAP).unwrap(),
        CapabilityKind::Tool,
        "echo across the mesh",
    )
    .unwrap();
    let module: Arc<dyn Plugin> = Arc::new(EchoModule {
        manifest,
        tools: vec![Arc::new(EchoTool {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            runs: Arc::clone(&runs),
        }) as Arc<dyn ToolCapability>],
    });
    let mut manager = PluginManager::new();
    manager.register(module).unwrap();
    let ctx = PluginContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();

    let holder = Arc::new(Holder {
        manager: Mutex::new(manager),
        governance: Arc::new(apeireth_governance::AllowAll),
        clock: system_clock(),
        echo_runs: runs,
    });
    let node = MeshNode::new(
        MeshNodeConfig::new(apeireth_mesh::NodeId::new(id).unwrap(), format!("node {id}"))
            .with_trust(trust),
        Arc::new(ManagerExecutor::new(Arc::clone(&holder))),
        system_clock(),
    );
    (node, holder)
}

/// A signed, verified presence advertising one host candidate.
fn presence_for(
    identity: &NodeIdentity,
    node: &apeireth_mesh::NodeId,
    session_id: u64,
    ttl_ms: u64,
    addr: SocketAddr,
) -> VerifiedPresence {
    let candidates = canonicalize_candidates(vec![EndpointCandidate::new(
        EndpointCandidateKind::Host,
        addr,
    )
    .unwrap()])
    .unwrap();
    let record = signed_presence_with_candidates(
        identity,
        node.clone(),
        AUDIENCE.to_string(),
        session_id,
        ttl_ms,
        true,
        candidates,
    )
    .unwrap();
    VerifiedPresence::verify(record, now_ms(), AUDIENCE).unwrap()
}

/// A signed presence record with an explicit expiry (for stale/expired cases).
fn record_with_expiry(
    identity: &NodeIdentity,
    node: &apeireth_mesh::NodeId,
    session_id: u64,
    expires_at_ms: u64,
    addr: SocketAddr,
) -> PresenceRecord {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut nonce = vec![0u8; PRESENCE_NONCE_BYTES];
    SystemRandom::new().fill(&mut nonce).expect("rng");
    let candidates = canonicalize_candidates(vec![EndpointCandidate::new(
        EndpointCandidateKind::Host,
        addr,
    )
    .unwrap()])
    .unwrap();
    PresenceRecord::unsigned(
        node.clone(),
        identity.public_key().to_vec(),
        session_id,
        expires_at_ms,
        AUDIENCE.to_string(),
        true,
        candidates,
        nonce,
    )
    .sign(identity.pkcs8_der())
    .expect("identity signs its own presence")
}

// ---------------------------------------------------------------------------
// The production path: VerifiedPresence -> punch -> authenticated QUIC ->
// MeshNode activation -> canonical remote invocation.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_punched_peers_authenticate_and_run_one_remote_call() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let (node_a_id, node_b_id) = (
        apeireth_mesh::NodeId::new("node-a").unwrap(),
        apeireth_mesh::NodeId::new("node-b").unwrap(),
    );

    // Each side pins exactly the other — the same trust the pairing
    // ceremony would have produced.
    let trust_a = TrustPolicy::new().with_trusted_pinned(
        node_b_id.clone(),
        "127.0.0.1:1",
        identity_b.fingerprint().clone(),
    );
    let trust_b = TrustPolicy::new().with_trusted_pinned(
        node_a_id.clone(),
        "127.0.0.1:1",
        identity_a.fingerprint().clone(),
    );

    let (node_a, _holder_a) = node_with_echo("node-a", trust_a).await;
    let (node_b, holder_b) = node_with_echo("node-b", trust_b).await;

    // One dual-role socket per side: it observes, dials, and accepts on the
    // same port — the shape a NAT mapping requires.
    let a_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_a),
            &[identity_b.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_b),
            &[identity_a.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );

    // Each side holds the other's current, verified reachability.
    let a_presence = presence_for(&identity_a, &node_a_id, 11, 600_000, a_peer.local_addr().unwrap());
    let b_presence = presence_for(&identity_b, &node_b_id, 22, 600_000, b_peer.local_addr().unwrap());

    // The one high-level act, from both sides: each asks for the other over
    // its own verified presence, on its own one socket.
    let (a_result, b_result) = tokio::join!(
        node_a.connect_punched(
            node_b_id.clone(),
            &b_presence,
            11,
            Arc::clone(&a_peer),
            WINDOW_MS,
        ),
        node_b.connect_punched(
            node_a_id.clone(),
            &a_presence,
            22,
            Arc::clone(&b_peer),
            WINDOW_MS,
        ),
    );
    a_result.expect("A's punched connection authenticates");
    b_result.expect("B's punched connection authenticates");

    // Exactly one live link per side, both Active.
    for (node, peer_id) in [(&node_a, &node_b_id), (&node_b, &node_a_id)] {
        let entries: Vec<_> = node
            .peers()
            .into_iter()
            .filter(|(id, _, _)| id == peer_id)
            .collect();
        assert_eq!(entries.len(), 1, "one logical connection to {peer_id}");
        assert!(
            entries[0].1.is_dispatchable(),
            "the surviving link is Active, got {:?}",
            entries[0].1
        );
    }

    // The advert lands over the punched link, so the remote capability is
    // dispatchable from A.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(node_a
        .projection(&node_b_id)
        .iter()
        .any(|cap| cap.id.as_str() == ECHO_CAP));

    // A real remote invocation, with a distinctive payload: the result
    // content must come back verbatim, and B must have executed it exactly
    // once — two physical punch links must never become two calls.
    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b_id.clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            ToolCall {
                id: "call-punched".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "path": "punched-direct" }),
            },
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .expect("remote invocation over the punched link");
    assert_eq!(result.render(), r#"{"path":"punched-direct"}"#);
    assert_eq!(
        holder_b.echo_runs.load(Ordering::SeqCst),
        1,
        "exactly one execution on the executing node"
    );

    // The coordination state is spent: no attempt outlives its window.
    assert!(node_a.punch_coordinator().is_empty() || {
        node_a.punch_coordinator().prune_expired(now_ms() + WINDOW_MS + 1) > 0
    });

    a_peer.close();
    b_peer.close();
}

// ---------------------------------------------------------------------------
// Fail-closed gates on the same path.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unadmitted_peer_is_refused_before_any_punch() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let node_b_id = apeireth_mesh::NodeId::new("node-b").unwrap();

    // A trusts nobody: the punch must not even start.
    let (node_a, _) = node_with_echo("node-a", TrustPolicy::new()).await;
    let (node_b, _) = node_with_echo(
        "node-b",
        TrustPolicy::new().with_trusted_pinned(
            apeireth_mesh::NodeId::new("node-a").unwrap(),
            "127.0.0.1:1",
            identity_a.fingerprint().clone(),
        ),
    )
    .await;

    let a_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_a),
            &[identity_b.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_b),
            &[identity_a.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b_presence = presence_for(
        &identity_b,
        &node_b_id,
        22,
        600_000,
        b_peer.local_addr().unwrap(),
    );

    let error = node_a
        .connect_punched(node_b_id.clone(), &b_presence, 11, a_peer, WINDOW_MS)
        .await
        .expect_err("an unadmitted peer must be refused");
    assert!(
        matches!(error, MeshError::Untrusted { .. }),
        "refused at the trust gate, got {error}"
    );
    assert!(
        node_a.punch_coordinator().is_empty(),
        "no coordination state may be created for an unadmitted peer"
    );
    assert!(
        node_b.peers().is_empty(),
        "nothing may arrive at the far side either"
    );
    b_peer.close();
}

#[tokio::test]
async fn a_presence_describing_another_node_cannot_dial_the_target() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let node_a_id = apeireth_mesh::NodeId::new("node-a").unwrap();
    let node_b_id = apeireth_mesh::NodeId::new("node-b").unwrap();

    let (node_a, _) = node_with_echo(
        "node-a",
        TrustPolicy::new().with_trusted_pinned(
            node_b_id.clone(),
            "127.0.0.1:1",
            identity_b.fingerprint().clone(),
        ),
    )
    .await;
    let a_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_a),
            &[identity_b.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );

    // A's OWN presence, handed over as if it were B's: the addresses are
    // signed and verified, but they describe the wrong peer.
    let wrong = presence_for(
        &identity_a,
        &node_a_id,
        11,
        600_000,
        a_peer.local_addr().unwrap(),
    );
    let error = node_a
        .connect_punched(node_b_id.clone(), &wrong, 11, a_peer, WINDOW_MS)
        .await
        .expect_err("another node's presence must not dial this peer");
    assert!(
        matches!(error, MeshError::PunchFailed { .. }),
        "got {error}"
    );
    assert!(
        error.to_string().contains("node-a"),
        "the refusal names the mismatch: {error}"
    );
}

#[tokio::test]
async fn a_presence_that_expired_between_verifying_and_dialing_is_refused() {
    let identity_a = Arc::new(NodeIdentity::generate().unwrap());
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let node_b_id = apeireth_mesh::NodeId::new("node-b").unwrap();

    let (node_a, _) = node_with_echo(
        "node-a",
        TrustPolicy::new().with_trusted_pinned(
            node_b_id.clone(),
            "127.0.0.1:1",
            identity_b.fingerprint().clone(),
        ),
    )
    .await;
    let a_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_a),
            &[identity_b.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&identity_b),
            &[identity_a.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );

    // Verified while live; the lease lapses before the dial is attempted.
    let record = record_with_expiry(
        &identity_b,
        &node_b_id,
        22,
        now_ms() + 40,
        b_peer.local_addr().unwrap(),
    );
    let presence = VerifiedPresence::verify(record, now_ms(), AUDIENCE).unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;

    let error = node_a
        .connect_punched(node_b_id.clone(), &presence, 11, a_peer, WINDOW_MS)
        .await
        .expect_err("an expired lease must not dial");
    assert!(
        matches!(error, MeshError::PunchFailed { .. }),
        "got {error}"
    );
    assert!(
        error.to_string().contains("expired"),
        "the refusal says why: {error}"
    );
    b_peer.close();
}

#[tokio::test]
async fn a_replayed_lease_is_refused_rather_than_shortening_the_stored_one() {
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let node_b_id = apeireth_mesh::NodeId::new("node-b").unwrap();
    let (node_a, _) = node_with_echo("node-a", TrustPolicy::new()).await;

    // The stored incarnation has a long lease.
    let long = presence_for(
        &identity_b,
        &node_b_id,
        22,
        600_000,
        "127.0.0.1:9000".parse().unwrap(),
    );
    assert!(node_a.note_presence(long).is_ok());

    // A "new" incarnation with a shorter lease is a replay, not a move.
    let short = presence_for(
        &identity_b,
        &node_b_id,
        23,
        1_000,
        "127.0.0.1:9001".parse().unwrap(),
    );
    let error = node_a
        .note_presence(short)
        .expect_err("a non-advancing lease is refused");
    assert!(
        error.to_string().contains("does not advance"),
        "got {error}"
    );
    // The stored reachability is untouched by the replay.
    let current = node_a.peer_presence(&node_b_id).expect("presence stored");
    assert_eq!(current.session_id(), 22);
}

#[tokio::test]
async fn a_replaced_incarnation_invalidates_the_punch_bound_to_the_old_one() {
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let node_b_id = apeireth_mesh::NodeId::new("node-b").unwrap();
    let (node_a, _) = node_with_echo("node-a", TrustPolicy::new()).await;
    let node_a_id = node_a.node_id().clone();

    // An attempt bound to B's incarnation 22.
    let session = node_a
        .punch_coordinator()
        .request(&node_a_id, &node_b_id, 11, 22, now_ms(), WINDOW_MS)
        .unwrap();
    assert!(node_a.punch_coordinator().session_for(&node_a_id, &node_b_id).is_some());

    // B's incarnation 22 is what A currently holds.
    let first = presence_for(
        &identity_b,
        &node_b_id,
        22,
        600_000,
        "127.0.0.1:9002".parse().unwrap(),
    );
    assert!(matches!(
        node_a.note_presence(first).unwrap(),
        apeireth_mesh::PresenceUpdate::Inserted { session_id: 22 }
    ));

    // B mints incarnation 23: everything derived from 22 is stale.
    let replaced = presence_for(
        &identity_b,
        &node_b_id,
        23,
        900_000,
        "127.0.0.1:9002".parse().unwrap(),
    );
    let update = node_a.note_presence(replaced).unwrap();
    assert!(
        matches!(update, apeireth_mesh::PresenceUpdate::Replaced { previous_session_id: 22, .. }),
        "got {update:?}"
    );
    assert!(
        node_a
            .punch_coordinator()
            .session_for(&node_a_id, &node_b_id)
            .is_none(),
        "the attempt bound to the replaced incarnation must not survive"
    );
    let _ = session;
}

#[tokio::test]
async fn revoking_a_peer_drops_its_presence_and_its_punch_attempts() {
    let identity_b = Arc::new(NodeIdentity::generate().unwrap());
    let node_b_id = apeireth_mesh::NodeId::new("node-b").unwrap();
    let (node_a, _) = node_with_echo(
        "node-a",
        TrustPolicy::new().with_trusted_pinned(
            node_b_id.clone(),
            "127.0.0.1:1",
            identity_b.fingerprint().clone(),
        ),
    )
    .await;
    let node_a_id = node_a.node_id().clone();

    let presence = presence_for(
        &identity_b,
        &node_b_id,
        22,
        600_000,
        "127.0.0.1:9003".parse().unwrap(),
    );
    node_a.note_presence(presence).unwrap();
    node_a
        .punch_coordinator()
        .request(&node_a_id, &node_b_id, 11, 22, now_ms(), WINDOW_MS)
        .unwrap();

    node_a.revoke_peer(&node_b_id);

    assert!(node_a.peer_presence(&node_b_id).is_none());
    assert!(node_a
        .punch_coordinator()
        .session_for(&node_a_id, &node_b_id)
        .is_none());
}
