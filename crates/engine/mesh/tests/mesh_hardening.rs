//! Negative and resource-bound tests for the mesh node: a connected peer is
//! authenticated, not omnipotent. Each test names the property it pins.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{system_clock, CapabilityId, NodeId, PluginId, SessionId, TraceId};
use apeireth_mesh::{
    linked_pair, CapabilityCatalogEntry, Hello, InvokeMeta, KeyFingerprint, MeshCapabilityExecutor,
    MeshConnection, MeshEnvelope, MeshError, MeshErrorCode, MeshExecError, MeshInvokeContext,
    MeshNode, MeshNodeConfig, MeshPayload, MeshTransportError, NodeDescriptor, PeerState,
    RouteSelection, TrustPolicy, MESH_PROTOCOL_VERSION,
};
use apeireth_mesh_protocol::{
    Cancel, CapabilityCall, CapabilityOutcome, CapabilityResult, InvokeCapability,
    MeshCapabilityKind,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;

const ECHO_CAP: &str = "tool.mesh.echo";

/// Executes `tool.mesh.echo` only once released; honours cancellation.
struct GateExecutor {
    released: Arc<AtomicBool>,
}

#[async_trait]
impl MeshCapabilityExecutor for GateExecutor {
    fn catalog(&self) -> Vec<CapabilityCatalogEntry> {
        vec![CapabilityCatalogEntry {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            kind: MeshCapabilityKind::Tool,
            owner: PluginId::new("builtin.mesh-test").unwrap(),
            description: "gated echo".into(),
            available: true,
            declaration: Some(NormalizedTool {
                name: "mesh_echo".into(),
                description: Some("gated echo".into()),
                parameters: ToolParameters::new(),
                strict: false,
            }),
            models: None,
        }]
    }

    async fn execute_tool(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        call: &ToolCall,
    ) -> Result<ToolResult, MeshExecError> {
        loop {
            if ctx.cancelled.is_cancelled() {
                return Err(MeshExecError::Cancelled {
                    capability: capability.clone(),
                });
            }
            if self.released.load(Ordering::SeqCst) {
                return Ok(ToolResult::ok(
                    &call.id,
                    serde_json::json!({ "done": true }),
                ));
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    async fn authorize(
        &self,
        _ctx: &MeshInvokeContext,
        _capability: &CapabilityId,
        _arguments: &serde_json::Value,
    ) -> Result<(), MeshExecError> {
        Ok(())
    }

    async fn execute_provider(
        &self,
        _ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        _request: &apeireth_protocol::canonical::NormalizedRequest,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, MeshExecError> {
        Err(MeshExecError::Internal {
            capability: capability.clone(),
            message: "unused".into(),
        })
    }

    async fn authorize_completion(
        &self,
        _ctx: &MeshInvokeContext,
        _model: &str,
        _message_count: usize,
    ) -> Result<(), MeshExecError> {
        Ok(())
    }
}

fn id(name: &str) -> NodeId {
    NodeId::new(name).unwrap()
}

fn trusting(peers: &[&str]) -> TrustPolicy {
    peers.iter().fold(TrustPolicy::new(), |policy, peer| {
        policy.with_trusted(id(peer), format!("memory://{peer}"))
    })
}

fn gate_node(
    name: &str,
    trust: TrustPolicy,
    released: &Arc<AtomicBool>,
    tune: impl FnOnce(MeshNodeConfig) -> MeshNodeConfig,
) -> Arc<MeshNode> {
    MeshNode::new(
        tune(MeshNodeConfig::new(id(name), name).with_trust(trust)),
        Arc::new(GateExecutor {
            released: Arc::clone(released),
        }),
        system_clock(),
    )
}

async fn connect_pair(a: &Arc<MeshNode>, b: &Arc<MeshNode>) {
    let (link_a, link_b) = linked_pair();
    let link_a: Arc<dyn MeshConnection> = Arc::new(link_a);
    let link_b: Arc<dyn MeshConnection> = Arc::new(link_b);
    let (ra, rb) = tokio::join!(a.connect(link_a, b.node_id().clone()), b.accept(link_b),);
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
}

/// Handshake a hand-driven peer into `node` and drain the node's advert, so the
/// test can then put arbitrary envelopes on the wire as that peer.
async fn raw_peer(node: &Arc<MeshNode>, name: &str) -> Arc<dyn MeshConnection> {
    let (mine, theirs) = linked_pair();
    let mine: Arc<dyn MeshConnection> = Arc::new(mine);
    let accept = tokio::spawn({
        let node = Arc::clone(node);
        let link: Arc<dyn MeshConnection> = Arc::new(theirs);
        async move { node.accept(link).await }
    });
    mine.send(MeshEnvelope::send(
        id(name),
        node.node_id().clone(),
        TraceId::new(),
        MeshPayload::Hello(Hello {
            node: NodeDescriptor::new(id(name), name, MESH_PROTOCOL_VERSION, "2.0.0"),
        }),
    ))
    .await
    .unwrap();
    assert!(matches!(
        mine.recv().await.unwrap().payload,
        MeshPayload::HelloAck(_)
    ));
    accept.await.unwrap().unwrap();
    assert!(matches!(
        mine.recv().await.unwrap().payload,
        MeshPayload::CapabilityAdvert(_)
    ));
    mine
}

fn invoke_envelope(from: &str, to: &NodeId) -> MeshEnvelope {
    MeshEnvelope::send(
        id(from),
        to.clone(),
        TraceId::new(),
        MeshPayload::InvokeCapability(InvokeCapability {
            capability: CapabilityId::new(ECHO_CAP).unwrap(),
            kind: MeshCapabilityKind::Tool,
            call: CapabilityCall::Tool(ToolCall {
                id: "raw".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({}),
            }),
            session: SessionId::new(),
            round: 1,
            deadline_epoch_ms: None,
        }),
    )
}

fn meta() -> InvokeMeta {
    InvokeMeta::new(SessionId::new(), TraceId::new(), 1)
}

fn echo() -> ToolCall {
    ToolCall {
        id: "call".into(),
        name: "mesh_echo".into(),
        arguments: serde_json::json!({}),
    }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn a_reply_from_a_peer_the_call_was_not_sent_to_is_refused() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b", "node-c"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    connect_pair(&a, &b).await;
    let intruder = raw_peer(&a, "node-c").await;

    let handle = a
        .start_call(
            &RouteSelection::Remote(id("node-b")),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo(),
            meta(),
        )
        .await
        .unwrap();

    let mut forged = MeshEnvelope::send(
        id("node-c"),
        id("node-a"),
        TraceId::new(),
        MeshPayload::CapabilityResult(CapabilityResult {
            capability: CapabilityId::new(ECHO_CAP).unwrap(),
            outcome: CapabilityOutcome::Tool(ToolResult::ok(
                "call",
                serde_json::json!({ "forged": true }),
            )),
        }),
    );
    forged.in_reply_to = Some(handle.message_id);
    intruder.send(forged).await.unwrap();
    settle().await;

    assert_eq!(a.peer_state(&id("node-c")), Some(PeerState::Closed));
    assert_eq!(a.peer_state(&id("node-b")), Some(PeerState::Active));

    released.store(true, Ordering::SeqCst);
    let result = handle.tool_result().await.unwrap();
    assert!(
        !format!("{result:?}").contains("forged"),
        "the forged reply must not have completed the call: {result:?}"
    );
}

#[tokio::test]
async fn a_peer_cannot_cancel_another_peers_invocation() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a", "node-c"]), &released, |c| c);
    connect_pair(&a, &b).await;
    let intruder = raw_peer(&b, "node-c").await;

    let handle = a
        .start_call(
            &RouteSelection::Remote(id("node-b")),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo(),
            meta(),
        )
        .await
        .unwrap();
    settle().await;

    intruder
        .send(MeshEnvelope::send(
            id("node-c"),
            id("node-b"),
            TraceId::new(),
            MeshPayload::Cancel(Cancel {
                target: handle.message_id,
            }),
        ))
        .await
        .unwrap();
    settle().await;

    released.store(true, Ordering::SeqCst);
    let outcome = handle.outcome().await;
    assert!(
        matches!(outcome, Ok(CapabilityOutcome::Tool(_))),
        "a cancel from node-c must not reach node-a's invocation: {outcome:?}"
    );
}

#[tokio::test]
async fn inbound_executions_per_peer_are_bounded() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| {
        c.with_max_inbound_per_peer(2)
    });
    connect_pair(&a, &b).await;
    let capability = CapabilityId::new(ECHO_CAP).unwrap();
    let route = RouteSelection::Remote(id("node-b"));

    let first = a
        .start_call(&route, &capability, echo(), meta())
        .await
        .unwrap();
    let second = a
        .start_call(&route, &capability, echo(), meta())
        .await
        .unwrap();
    settle().await;
    let third = a
        .start_call(&route, &capability, echo(), meta())
        .await
        .unwrap();

    match third.outcome().await {
        Err(MeshError::Remote { code, .. }) => assert_eq!(code, MeshErrorCode::Backpressure),
        other => panic!("expected the third call to be refused, got {other:?}"),
    }

    released.store(true, Ordering::SeqCst);
    assert!(first.outcome().await.is_ok());
    assert!(second.outcome().await.is_ok());
}

#[tokio::test]
async fn a_replayed_message_id_while_executing_terminates_the_connection() {
    let released = Arc::new(AtomicBool::new(false));
    let b = gate_node("node-b", trusting(&["node-c"]), &released, |c| c);
    let peer = raw_peer(&b, "node-c").await;

    let request = invoke_envelope("node-c", b.node_id());
    peer.send(request.clone()).await.unwrap();
    settle().await;
    peer.send(request).await.unwrap();
    settle().await;

    assert_eq!(b.peer_state(&id("node-c")), Some(PeerState::Closed));
    released.store(true, Ordering::SeqCst);
}

#[tokio::test]
async fn a_call_that_is_never_answered_times_out_and_frees_its_slot() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| {
        c.with_max_pending_calls(1).with_call_timeout(Some(60))
    });
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    connect_pair(&a, &b).await;
    let capability = CapabilityId::new(ECHO_CAP).unwrap();
    let route = RouteSelection::Remote(id("node-b"));

    let first = a
        .start_call(&route, &capability, echo(), meta())
        .await
        .unwrap();
    let blocked = a.start_call(&route, &capability, echo(), meta()).await;
    assert!(matches!(blocked, Err(MeshError::Backpressure { .. })));

    assert!(matches!(
        first.outcome().await,
        Err(MeshError::DeadlineExceeded)
    ));
    let again = a.start_call(&route, &capability, echo(), meta()).await;
    assert!(again.is_ok(), "the timed-out call must release its slot");
    released.store(true, Ordering::SeqCst);
}

#[tokio::test]
async fn dropping_a_call_handle_frees_its_slot() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| {
        c.with_max_pending_calls(1)
    });
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    connect_pair(&a, &b).await;
    let capability = CapabilityId::new(ECHO_CAP).unwrap();
    let route = RouteSelection::Remote(id("node-b"));

    let handle = a
        .start_call(&route, &capability, echo(), meta())
        .await
        .unwrap();
    drop(handle);
    assert!(a
        .start_call(&route, &capability, echo(), meta())
        .await
        .is_ok());
    released.store(true, Ordering::SeqCst);
}

#[tokio::test]
async fn a_failed_second_dial_does_not_evict_the_live_connection() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| {
        c.with_handshake_timeout(80)
    });
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    connect_pair(&a, &b).await;
    assert_eq!(a.peer_state(&id("node-b")), Some(PeerState::Active));

    // A dial whose far end never answers.
    let (silent_a, _silent_b) = linked_pair();
    let result = a.connect(Arc::new(silent_a), id("node-b")).await;
    assert!(result.is_err());

    assert_eq!(a.peer_state(&id("node-b")), Some(PeerState::Active));
}

/// A link that claims to have authenticated a key.
struct KeyedLink {
    inner: Arc<dyn MeshConnection>,
    fingerprint: KeyFingerprint,
}

#[async_trait]
impl MeshConnection for KeyedLink {
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), MeshTransportError> {
        self.inner.send(envelope).await
    }
    async fn recv(&self) -> Result<MeshEnvelope, MeshTransportError> {
        self.inner.recv().await
    }
    fn close(&self) {
        self.inner.close();
    }
    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    fn peer_fingerprint(&self) -> Option<KeyFingerprint> {
        Some(self.fingerprint.clone())
    }
}

#[tokio::test]
async fn a_trusted_peer_without_a_pin_is_refused_on_a_keyed_transport() {
    let released = Arc::new(AtomicBool::new(false));
    // `node-c` is trusted by id but no key is pinned for it.
    let b = gate_node("node-b", trusting(&["node-c"]), &released, |c| c);
    let (mine, theirs) = linked_pair();
    let mine: Arc<dyn MeshConnection> = Arc::new(mine);
    let keyed: Arc<dyn MeshConnection> = Arc::new(KeyedLink {
        inner: Arc::new(theirs),
        fingerprint: KeyFingerprint::of_public_key(&[7u8; 32]),
    });
    let accept = tokio::spawn({
        let b = Arc::clone(&b);
        async move { b.accept(keyed).await }
    });
    mine.send(MeshEnvelope::send(
        id("node-c"),
        id("node-b"),
        TraceId::new(),
        MeshPayload::Hello(Hello {
            node: NodeDescriptor::new(id("node-c"), "c", MESH_PROTOCOL_VERSION, "2.0.0"),
        }),
    ))
    .await
    .unwrap();

    let error = accept.await.unwrap().unwrap_err();
    assert!(matches!(error, MeshError::Untrusted { .. }), "{error}");
    assert_eq!(b.peer_state(&id("node-c")), None);
}

#[tokio::test]
async fn revoking_a_peer_drops_its_link_calls_routes_and_future_handshakes() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    connect_pair(&a, &b).await;
    assert!(!a.projection(&id("node-b")).is_empty());

    let handle = a
        .start_call(
            &RouteSelection::Remote(id("node-b")),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo(),
            meta(),
        )
        .await
        .unwrap();

    a.revoke_peer(&id("node-b"));

    assert!(matches!(
        handle.outcome().await,
        Err(MeshError::Transport(_))
    ));
    assert_eq!(a.peer_state(&id("node-b")), Some(PeerState::Closed));
    assert!(a.projection(&id("node-b")).is_empty());

    // The revoked node cannot come back under the same id.
    let (link_b, link_a) = linked_pair();
    let link_b: Arc<dyn MeshConnection> = Arc::new(link_b);
    let link_a: Arc<dyn MeshConnection> = Arc::new(link_a);
    let (dialled, accepted) = tokio::join!(b.connect(link_b, id("node-a")), a.accept(link_a));
    assert!(
        matches!(accepted, Err(MeshError::Untrusted { .. })),
        "{accepted:?}"
    );
    assert!(dialled.is_err());
    released.store(true, Ordering::SeqCst);
}

#[tokio::test]
async fn admitting_a_peer_makes_it_connectable_without_a_restart() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", TrustPolicy::new(), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);

    let (link_a, link_b) = linked_pair();
    let (refused, _) = tokio::join!(
        a.connect(Arc::new(link_a), id("node-b")),
        b.accept(Arc::new(link_b))
    );
    assert!(matches!(refused, Err(MeshError::Untrusted { .. })));

    a.admit_peer(apeireth_mesh::KnownPeer {
        node: id("node-b"),
        endpoint: "memory://node-b".into(),
        trust: apeireth_mesh::PeerTrust::Trusted,
        key_fingerprint: None,
    });
    connect_pair(&a, &b).await;
    assert_eq!(a.peer_state(&id("node-b")), Some(PeerState::Active));
}

#[tokio::test]
async fn shutdown_fails_pending_calls_closes_links_and_stops_loops() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    connect_pair(&a, &b).await;
    let heartbeat = a.spawn_heartbeat(10);

    let handle = a
        .start_call(
            &RouteSelection::Remote(id("node-b")),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo(),
            meta(),
        )
        .await
        .unwrap();

    a.shutdown();
    a.shutdown();

    assert!(a.is_shutdown());
    assert!(matches!(
        handle.outcome().await,
        Err(MeshError::Transport(_))
    ));
    assert_eq!(a.peer_state(&id("node-b")), Some(PeerState::Closed));
    tokio::time::timeout(Duration::from_secs(2), heartbeat)
        .await
        .expect("the heartbeat loop must stop on shutdown")
        .unwrap();

    let (link_a, _link_b) = linked_pair();
    assert!(a.connect(Arc::new(link_a), id("node-b")).await.is_err());
    released.store(true, Ordering::SeqCst);
}

/// Dials by wiring a fresh in-memory pair to `target.accept`; fails the first
/// `failures` dials.
struct LoopbackTransport {
    target: Arc<MeshNode>,
    failures: std::sync::atomic::AtomicUsize,
    dials: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl apeireth_mesh::MeshTransport for LoopbackTransport {
    async fn dial(&self, _endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        self.dials.fetch_add(1, Ordering::SeqCst);
        let remaining = self.failures.load(Ordering::SeqCst);
        if remaining > 0 {
            self.failures.store(remaining - 1, Ordering::SeqCst);
            return Err(MeshTransportError::Io("simulated outage".into()));
        }
        let (mine, theirs) = linked_pair();
        let target = Arc::clone(&self.target);
        tokio::spawn(async move {
            let _ = target.accept(Arc::new(theirs)).await;
        });
        Ok(Arc::new(mine))
    }
}

fn quick_policy() -> apeireth_mesh::ReconnectPolicy {
    apeireth_mesh::ReconnectPolicy {
        initial_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(40),
        jitter: 0.0,
        check_interval: Duration::from_millis(10),
    }
}

async fn wait_for(mut condition: impl FnMut() -> bool) -> bool {
    for _ in 0..200 {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test]
async fn a_maintained_connection_survives_outages_and_link_loss() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    let transport = Arc::new(LoopbackTransport {
        target: Arc::clone(&b),
        failures: std::sync::atomic::AtomicUsize::new(2),
        dials: std::sync::atomic::AtomicUsize::new(0),
    });
    let task = a.maintain_connection(
        id("node-b"),
        "memory://node-b".into(),
        transport.clone(),
        quick_policy(),
    );

    // Two failed dials, then a connection.
    assert!(wait_for(|| a.peer_state(&id("node-b")) == Some(PeerState::Active)).await);
    assert_eq!(transport.dials.load(Ordering::SeqCst), 3);

    // The far end drops the link; the maintainer redials.
    b.disconnect(&id("node-a"));
    assert!(wait_for(|| transport.dials.load(Ordering::SeqCst) >= 4).await);
    assert!(wait_for(|| a.peer_state(&id("node-b")) == Some(PeerState::Active)).await);

    // Revocation ends the loop and no further dial happens.
    a.revoke_peer(&id("node-b"));
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("the maintainer must stop once the peer is untrusted")
        .unwrap();
    let dials = transport.dials.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(transport.dials.load(Ordering::SeqCst), dials);
}

#[tokio::test]
async fn a_maintained_connection_stops_on_shutdown() {
    let released = Arc::new(AtomicBool::new(false));
    let a = gate_node("node-a", trusting(&["node-b"]), &released, |c| c);
    let b = gate_node("node-b", trusting(&["node-a"]), &released, |c| c);
    let transport = Arc::new(LoopbackTransport {
        target: b,
        failures: std::sync::atomic::AtomicUsize::new(usize::MAX),
        dials: std::sync::atomic::AtomicUsize::new(0),
    });
    let task = a.maintain_connection(
        id("node-b"),
        "memory://node-b".into(),
        transport,
        quick_policy(),
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    a.shutdown();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("the maintainer must stop on shutdown")
        .unwrap();
}
