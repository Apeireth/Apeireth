//! [`MeshNode`]: one Apeireth runtime's presence in the mesh.
//!
//! # The one idea
//!
//! A mesh node does not own capabilities. It connects peers, exchanges
//! read-only capability projections, and moves invocations to wherever the
//! canonical `PluginManager` that owns the implementation actually lives. Local
//! or remote, an invocation ends in the same place — governance first, then the
//! owning module — so the network only ever changes *where execution happens*.
//!
//! # Lifecycle of a connection
//!
//! `connect`/`accept` handshakes (identity + protocol version, fail closed),
//! then both sides exchange a full-snapshot [`CapabilityAdvert`] and enter the
//! connection loop: invocations fan out to the executor, results and errors
//! complete pending calls by `in_reply_to`, `Cancel` sets the target
//! invocation's [`CancelFlag`], and every inbound message refreshes liveness.
//! `sweep_heartbeats` demotes silent peers to `Unreachable`, which makes their
//! routes undispatchable without deleting their projection or their known-peer
//! identity — reconnect restores everything.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

use apeireth_core::kernel::{CapabilityId, Clock, NodeId, RequestId, SessionId, TraceId};
use apeireth_governance::ActionSource;
use apeireth_mesh_protocol::{
    CapabilityAdvert, CapabilityCall, CapabilityError, CapabilityOutcome, CapabilityResult, Hello,
    HelloAck, InvokeCapability, MeshCapabilityKind, MeshEnvelope, MeshErrorCode, MeshPayload,
    NodeDescriptor, MESH_PROTOCOL_VERSION,
};
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use tokio::sync::oneshot;

use crate::error::{MeshError, MeshExecError};
use crate::executor::{catalog_to_modules, CancelFlag, MeshCapabilityExecutor, MeshInvokeContext};
use crate::peer::{Peer, PeerState};
use crate::presence_store::{PresenceError, PresenceStore, PresenceUpdate};
use crate::punch::{punch, PunchCoordinator};
use crate::route::{RemoteCapability, RouteSelection, RouteTable};
use crate::transport::{MeshConnection, MeshListener, MeshTransport};
use crate::trust::{KnownPeer, PeerTrust, TrustPolicy};
use crate::VerifiedPresence;

/// Default silence after which a peer is considered unreachable.
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 15_000;

/// Default whole-handshake deadline: a connection that cannot authenticate
/// promptly is closed rather than left half-open.
pub const DEFAULT_HANDSHAKE_TIMEOUT_MS: u64 = 10_000;

/// Default cap on in-flight calls per node.
pub const DEFAULT_MAX_PENDING_CALLS: usize = 256;

/// Most inbound handshakes in flight at once; further connections are shed
/// instead of queued, so a flood of unauthenticated sockets cannot pile up.
pub const MAX_CONCURRENT_HANDSHAKES: usize = 64;

/// Default cap on concurrently executing invocations accepted from one peer.
pub const DEFAULT_MAX_INBOUND_PER_PEER: usize = 64;

/// Default caller-side bound on waiting for a remote outcome. A live peer that
/// never answers must not pin a pending-call entry forever.
pub const DEFAULT_CALL_TIMEOUT_MS: u64 = 300_000;

/// Identity and policy for one mesh node.
#[derive(Debug, Clone)]
pub struct MeshNodeConfig {
    /// This node's canonical identity. Persist it across restarts.
    pub node_id: NodeId,
    /// Display name. Cosmetic; never an identity.
    pub name: String,
    /// Runtime build version reported during the handshake.
    pub runtime_version: String,
    /// Which peers this node admits. Empty denies everyone.
    pub trust: TrustPolicy,
    /// Silence after which a peer becomes [`PeerState::Unreachable`].
    pub heartbeat_timeout_ms: u64,
    /// Deadline applied to calls that do not set their own.
    pub default_deadline_ms: Option<u64>,
    /// Whole-handshake deadline (transport established → peer Active). On
    /// expiry the connection closes and no projection is taken.
    pub handshake_timeout_ms: u64,
    /// Cap on in-flight calls per node (bounded transport discipline).
    pub max_pending_calls: usize,
    /// Cap on invocations this node executes concurrently for one peer.
    pub max_inbound_per_peer: usize,
    /// How long a caller waits for a remote outcome before giving up.
    pub call_timeout_ms: Option<u64>,
}

impl MeshNodeConfig {
    /// A node with fail-closed defaults.
    pub fn new(node_id: NodeId, name: impl Into<String>) -> Self {
        Self {
            node_id,
            name: name.into(),
            runtime_version: env!("CARGO_PKG_VERSION").to_string(),
            trust: TrustPolicy::new(),
            heartbeat_timeout_ms: DEFAULT_HEARTBEAT_TIMEOUT_MS,
            default_deadline_ms: None,
            handshake_timeout_ms: DEFAULT_HANDSHAKE_TIMEOUT_MS,
            max_pending_calls: DEFAULT_MAX_PENDING_CALLS,
            max_inbound_per_peer: DEFAULT_MAX_INBOUND_PER_PEER,
            call_timeout_ms: Some(DEFAULT_CALL_TIMEOUT_MS),
        }
    }

    /// Install the known-peer trust policy.
    #[must_use]
    pub fn with_trust(mut self, trust: TrustPolicy) -> Self {
        self.trust = trust;
        self
    }

    /// Set the heartbeat silence timeout.
    #[must_use]
    pub fn with_heartbeat_timeout(mut self, timeout_ms: u64) -> Self {
        self.heartbeat_timeout_ms = timeout_ms;
        self
    }

    /// Set the default call deadline.
    #[must_use]
    pub fn with_default_deadline(mut self, deadline_ms: Option<u64>) -> Self {
        self.default_deadline_ms = deadline_ms;
        self
    }

    /// Set the whole-handshake deadline.
    #[must_use]
    pub fn with_handshake_timeout(mut self, timeout_ms: u64) -> Self {
        self.handshake_timeout_ms = timeout_ms;
        self
    }

    /// Set the per-node in-flight call cap.
    #[must_use]
    pub fn with_max_pending_calls(mut self, max: usize) -> Self {
        self.max_pending_calls = max;
        self
    }

    /// Set the per-peer inbound execution cap.
    #[must_use]
    pub fn with_max_inbound_per_peer(mut self, max: usize) -> Self {
        self.max_inbound_per_peer = max;
        self
    }

    /// Set the caller-side wait bound (`None` waits indefinitely).
    #[must_use]
    pub fn with_call_timeout(mut self, timeout_ms: Option<u64>) -> Self {
        self.call_timeout_ms = timeout_ms;
        self
    }

    fn descriptor(&self) -> NodeDescriptor {
        NodeDescriptor::new(
            self.node_id.clone(),
            self.name.clone(),
            MESH_PROTOCOL_VERSION,
            self.runtime_version.clone(),
        )
    }
}

/// Caller-supplied invocation metadata.
#[derive(Debug, Clone)]
pub struct InvokeMeta {
    /// The calling side's session.
    pub session: SessionId,
    /// The causal span to propagate into the execution.
    pub trace: TraceId,
    /// The calling side's agent-loop round.
    pub round: u32,
    /// Deadline in epoch milliseconds, or the node default when `None`.
    pub deadline_epoch_ms: Option<u64>,
}

impl InvokeMeta {
    /// Invocation metadata under one session/trace/round.
    pub fn new(session: SessionId, trace: TraceId, round: u32) -> Self {
        Self {
            session,
            trace,
            round,
            deadline_epoch_ms: None,
        }
    }

    /// Set an explicit deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline_epoch_ms: Option<u64>) -> Self {
        self.deadline_epoch_ms = deadline_epoch_ms;
        self
    }
}

/// Where a started call will execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallTarget {
    /// Inside this runtime.
    Local,
    /// Inside the named peer's runtime.
    Remote(NodeId),
}

/// A started call: its cancellation handle and its pending outcome.
pub struct CallHandle {
    /// The wire message id; the argument to [`MeshNode::cancel`].
    pub message_id: RequestId,
    /// Where the call executes.
    pub target: CallTarget,
    rx: oneshot::Receiver<Result<CapabilityOutcome, MeshError>>,
    timeout: Option<std::time::Duration>,
    _pending: Option<PendingGuard>,
}

/// Removes a call's pending entry when its handle goes away, so an abandoned
/// or timed-out call cannot leak a slot against the per-peer cap.
struct PendingGuard {
    calls: PendingCalls,
    id: RequestId,
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.remove(&self.id);
        }
    }
}

impl CallHandle {
    /// Await the raw capability outcome.
    ///
    /// Bounded by the node's call timeout: expiry yields
    /// [`MeshError::DeadlineExceeded`] and frees the pending slot.
    pub async fn outcome(self) -> Result<CapabilityOutcome, MeshError> {
        let Self {
            rx,
            timeout,
            _pending,
            ..
        } = self;
        let received = match timeout {
            Some(limit) => tokio::time::timeout(limit, rx)
                .await
                .map_err(|_elapsed| MeshError::DeadlineExceeded)?,
            None => rx.await,
        };
        received.map_err(|_| MeshError::Transport("call completed without an outcome".into()))?
    }

    /// Await the canonical tool result of a tool invocation.
    pub async fn tool_result(self) -> Result<ToolResult, MeshError> {
        match self.outcome().await? {
            CapabilityOutcome::Tool(result) => Ok(result),
            other => Err(MeshError::Remote {
                capability: None,
                code: MeshErrorCode::Unsupported,
                message: format!(
                    "unexpected capability outcome shape: {}",
                    crate::kinds::outcome_kind(&other)
                ),
            }),
        }
    }
}

struct PeerEntry {
    peer: Peer,
    link: Option<Arc<dyn MeshConnection>>,
    /// Which node initiated this connection (duplicate-connection policy).
    initiator: NodeId,
    /// Connection generation: stale loops cannot clobber newer epochs.
    generation: u64,
}

struct PendingCall {
    peer: NodeId,
    tx: oneshot::Sender<Result<CapabilityOutcome, MeshError>>,
}

type PendingCalls = Arc<Mutex<BTreeMap<RequestId, PendingCall>>>;

/// `(origin, message id)`: `None` is a call started by this node; `Some(peer)`
/// is an execution a peer asked for. Keyed by origin so one peer can neither
/// cancel nor collide with another peer's (or a local) invocation.
type InvocationKey = (Option<NodeId>, RequestId);
type Invocations = Arc<Mutex<BTreeMap<InvocationKey, CancelFlag>>>;

/// One runtime's mesh presence.
pub struct MeshNode {
    descriptor: NodeDescriptor,
    clock: Arc<dyn Clock>,
    executor: Arc<dyn MeshCapabilityExecutor>,
    trust: RwLock<TrustPolicy>,
    heartbeat_timeout_ms: u64,
    default_deadline_ms: Option<u64>,
    handshake_timeout_ms: u64,
    max_pending_calls: usize,
    max_inbound_per_peer: usize,
    call_timeout: Option<std::time::Duration>,
    peers: Mutex<BTreeMap<NodeId, PeerEntry>>,
    routes: Mutex<RouteTable>,
    /// Caller-side calls awaiting a remote response.
    calls: PendingCalls,
    /// In-flight executions *in this runtime*, for cooperative cancellation.
    invocations: Invocations,
    /// Monotonic connection generation allocator.
    generation: std::sync::atomic::AtomicU64,
    /// Flips to `true` once on [`MeshNode::shutdown`]; background loops watch it.
    shutdown: tokio::sync::watch::Sender<bool>,
    /// Simultaneous-open coordination state, driven by
    /// [`MeshNode::connect_punched`].
    punch: Arc<PunchCoordinator>,
    /// The current verified reachability of each known peer. Feeds
    /// [`MeshNode::connect_punched`]; never a trust or liveness signal.
    presence: Mutex<PresenceStore>,
}

impl MeshNode {
    /// Assemble a node over an executor (the canonical dispatch port) and a
    /// time source.
    pub fn new(
        config: MeshNodeConfig,
        executor: Arc<dyn MeshCapabilityExecutor>,
        clock: Arc<dyn Clock>,
    ) -> Arc<Self> {
        Arc::new(Self {
            descriptor: config.descriptor(),
            clock,
            executor,
            trust: RwLock::new(config.trust),
            heartbeat_timeout_ms: config.heartbeat_timeout_ms,
            default_deadline_ms: config.default_deadline_ms,
            handshake_timeout_ms: config.handshake_timeout_ms,
            max_pending_calls: config.max_pending_calls,
            max_inbound_per_peer: config.max_inbound_per_peer,
            call_timeout: config.call_timeout_ms.map(std::time::Duration::from_millis),
            peers: Mutex::new(BTreeMap::new()),
            routes: Mutex::new(RouteTable::new()),
            calls: Arc::new(Mutex::new(BTreeMap::new())),
            invocations: Arc::new(Mutex::new(BTreeMap::new())),
            generation: std::sync::atomic::AtomicU64::new(0),
            shutdown: tokio::sync::watch::channel(false).0,
            punch: Arc::new(PunchCoordinator::default()),
            presence: Mutex::new(PresenceStore::new()),
        })
    }

    /// This node's canonical identity.
    pub fn node_id(&self) -> &NodeId {
        &self.descriptor.id
    }

    /// This node's handshake descriptor.
    pub fn descriptor(&self) -> &NodeDescriptor {
        &self.descriptor
    }

    fn authorize(&self, node: &NodeId) -> Result<(), MeshError> {
        self.trust
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorize(node)
    }

    fn pinned(&self, node: &NodeId) -> Option<crate::identity::KeyFingerprint> {
        self.trust
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pinned_fingerprint(node)
            .cloned()
    }

    /// Whether this peer is currently trusted by the live policy.
    pub fn is_trusted(&self, peer: &NodeId) -> bool {
        self.authorize(peer).is_ok()
    }

    /// Admit (or update) a peer in the live trust policy, e.g. right after a
    /// pairing ceremony persisted it. Takes effect for the next handshake.
    pub fn admit_peer(&self, peer: KnownPeer) {
        self.trust
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .upsert(peer);
    }

    /// Withdraw trust from `node` *now*: the live policy marks it untrusted
    /// and drops its pin (so no key can reconnect under that id), its
    /// connection closes, pending calls fail, and its projected capabilities
    /// disappear. Persisted removal is [`crate::TrustStore::forget`].
    ///
    /// Everything *derived* from the peer's reachability is dropped with it:
    /// its stored presence and any punch attempt bound to either side's
    /// incarnation. A revoked peer must not keep dialling anything.
    pub fn revoke_peer(&self, node: &NodeId) {
        {
            let mut trust = self
                .trust
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let endpoint = trust
                .known_peer(node)
                .map(|peer| peer.endpoint.clone())
                .unwrap_or_default();
            trust.upsert(KnownPeer {
                node: node.clone(),
                endpoint,
                trust: PeerTrust::Untrusted,
                key_fingerprint: None,
            });
        }
        self.forget_peer(node);
        self.presence
            .lock()
            .expect("mesh presence poisoned")
            .remove(node);
        self.punch.invalidate_peer(node);
    }

    /// The simultaneous-open coordination state this node drives.
    ///
    /// Shared with [`MeshNode::connect_punched`]: a caller that learns about
    /// coordination out of band (an existing relay link, a rendezvous hint)
    /// can wait on the same coordinator the dialer requests on.
    pub fn punch_coordinator(&self) -> Arc<PunchCoordinator> {
        Arc::clone(&self.punch)
    }

    /// Record a peer's current, already-verified reachability.
    ///
    /// This is metadata about *where to dial*, never trust: it is upserted
    /// under the same rules as [`PresenceStore`] (a restated incarnation with
    /// different reachability, an already-expired record, or a lease that
    /// does not advance are all refused). When a new incarnation replaces an
    /// old one, every punch attempt bound to the previous incarnation is
    /// invalidated here — an attempt may only ever dial the snapshot it was
    /// created for.
    pub fn note_presence(
        &self,
        presence: VerifiedPresence,
    ) -> Result<PresenceUpdate, PresenceError> {
        let peer = presence.node_id().clone();
        let update = self
            .presence
            .lock()
            .expect("mesh presence poisoned")
            .upsert(presence, self.now_ms())?;
        if let PresenceUpdate::Replaced {
            previous_session_id,
            ..
        } = update
        {
            self.punch.invalidate_incarnation(&peer, previous_session_id);
        }
        Ok(update)
    }

    /// This peer's current verified presence, or `None` when it is unknown or
    /// its lease has lapsed.
    pub fn peer_presence(&self, peer: &NodeId) -> Option<VerifiedPresence> {
        self.presence
            .lock()
            .expect("mesh presence poisoned")
            .current(peer, self.now_ms())
            .cloned()
    }

    /// Whether [`MeshNode::shutdown`] has been called.
    pub fn is_shutdown(&self) -> bool {
        *self.shutdown.borrow()
    }

    /// Stop the node: refuse new connections, close every link, fail every
    /// pending call, cancel every inbound execution, and let `serve` and
    /// heartbeat loops return. Idempotent.
    pub fn shutdown(&self) {
        self.shutdown.send_replace(true);
        let links: Vec<_> = {
            let mut peers = self.peers.lock().expect("mesh peers poisoned");
            peers
                .values_mut()
                .filter_map(|entry| {
                    entry.peer.state = PeerState::Closed;
                    entry.link.take()
                })
                .collect()
        };
        for link in links {
            link.close();
        }
        for (_, pending) in std::mem::take(&mut *self.calls.lock().expect("mesh calls poisoned")) {
            let _ = pending
                .tx
                .send(Err(MeshError::Transport("node shut down".to_string())));
        }
        for flag in self
            .invocations
            .lock()
            .expect("mesh invocations poisoned")
            .values()
        {
            flag.cancel();
        }
    }

    fn now_ms(&self) -> u64 {
        apeireth_core::kernel::Timestamp::from_clock(self.clock.as_ref())
            .epoch_millis()
            .max(0) as u64
    }

    // ------------------------------------------------------------------
    // Connection lifecycle
    // ------------------------------------------------------------------

    /// Dial-side handshake over an already-connected link.
    ///
    /// `expected` is the peer identity this runtime intends to reach — from the
    /// known-peer configuration, never guessed from the transport. Fail closed
    /// on trust, protocol version, or identity mismatch.
    pub async fn connect(
        self: &Arc<Self>,
        link: Arc<dyn MeshConnection>,
        expected: NodeId,
    ) -> Result<NodeId, MeshError> {
        if self.is_shutdown() {
            link.close();
            return Err(MeshError::Transport("node is shut down".to_string()));
        }
        self.authorize(&expected)?;
        let now = self.now_ms();
        {
            let mut peers = self.peers.lock().expect("mesh peers poisoned");
            // A handshake placeholder must never overwrite a live entry (the
            // duplicate-connection case keeps the incumbent until the new
            // connection is actually activated).
            peers.entry(expected.clone()).or_insert(PeerEntry {
                peer: Peer::connecting(expected.clone(), "", now),
                link: Some(Arc::clone(&link)),
                initiator: self.node_id().clone(),
                generation: 0,
            });
        }

        let hello = MeshEnvelope::send(
            self.node_id().clone(),
            expected.clone(),
            TraceId::new(),
            MeshPayload::Hello(Hello {
                node: self.descriptor.clone(),
            }),
        );
        if let Err(error) = link.send(hello).await {
            self.abandon_handshake(&expected);
            return Err(MeshError::Transport(error.to_string()));
        }

        let ack_env = match tokio::time::timeout(
            std::time::Duration::from_millis(self.handshake_timeout_ms),
            link.recv(),
        )
        .await
        {
            Ok(Ok(envelope)) => envelope,
            Ok(Err(error)) => {
                self.abandon_handshake(&expected);
                return Err(MeshError::Transport(error.to_string()));
            }
            Err(_elapsed) => {
                link.close();
                self.abandon_handshake(&expected);
                return Err(MeshError::Transport("handshake timed out".to_string()));
            }
        };

        let handshake = self.check_inbound_envelope(&ack_env, &expected);
        let ack = match (handshake, ack_env.payload) {
            (Ok(()), MeshPayload::HelloAck(ack)) => ack,
            (Err(error), _) => {
                self.abandon_handshake(&expected);
                return Err(error);
            }
            (Ok(()), _) => {
                self.abandon_handshake(&expected);
                return Err(MeshError::Malformed {
                    sender: expected.to_string(),
                    reason: "expected HelloAck after Hello".to_string(),
                });
            }
        };

        if let Err(error) =
            apeireth_mesh_protocol::verify_hello_ack(&self.descriptor, &ack, &expected)
        {
            self.abandon_handshake(&expected);
            return Err(error.into());
        }
        if let Err(error) = self.authorize(&ack.node.id) {
            self.abandon_handshake(&expected);
            return Err(error);
        }
        if let Err(error) = self.verify_pinned_identity(&ack.node.id, &link) {
            link.close();
            self.abandon_handshake(&expected);
            return Err(error);
        }

        // `verify_hello_ack` pinned the peer to `expected`, so returning it is
        // returning the handshaken identity.
        let initiator = self.node_id().clone();
        let Some(generation) = self.activate_peer(ack.node, Arc::clone(&link), initiator) else {
            self.abandon_handshake(&expected);
            return Err(MeshError::Superseded {
                kept: expected.to_string(),
            });
        };
        self.send_advert_to(&expected, &link).await;
        self.spawn_connection_loop(expected.clone(), link, generation);
        Ok(expected)
    }

    /// Listen-side handshake over an already-connected link.
    ///
    /// Refuses (explicitly, before closing) unknown or untrusted initiators and
    /// every version/identity mismatch.
    pub async fn accept(
        self: &Arc<Self>,
        link: Arc<dyn MeshConnection>,
    ) -> Result<NodeId, MeshError> {
        if self.is_shutdown() {
            link.close();
            return Err(MeshError::Transport("node is shut down".to_string()));
        }
        let hello_env = match tokio::time::timeout(
            std::time::Duration::from_millis(self.handshake_timeout_ms),
            link.recv(),
        )
        .await
        {
            Ok(Ok(envelope)) => envelope,
            Ok(Err(error)) => return Err(MeshError::Transport(error.to_string())),
            Err(_elapsed) => {
                link.close();
                return Err(MeshError::Transport("handshake timed out".to_string()));
            }
        };

        let reject = |reason: String| {
            let link = Arc::clone(&link);
            let reply = hello_env.reply(MeshPayload::HelloAck(HelloAck {
                node: self.descriptor.clone(),
                accepted: false,
                reason: Some(reason),
            }));
            async move {
                let _ = link.send(reply).await;
                link.close();
            }
        };

        if hello_env.target_node != *self.node_id() {
            reject(format!(
                "handshake addressed to {}, not {}",
                hello_env.target_node,
                self.node_id()
            ))
            .await;
            return Err(MeshError::Malformed {
                sender: hello_env.source_node.to_string(),
                reason: "handshake addressed to another node".to_string(),
            });
        }

        let hello = match &hello_env.payload {
            MeshPayload::Hello(hello) => hello.clone(),
            other => {
                reject("expected Hello".to_string()).await;
                return Err(MeshError::Malformed {
                    sender: hello_env.source_node.to_string(),
                    reason: format!("expected Hello, got {}", crate::kinds::payload_kind(other)),
                });
            }
        };

        if let Err(error) = self.check_inbound_envelope(&hello_env, &hello.node.id) {
            reject(error.to_string()).await;
            return Err(error);
        }
        if let Err(error) = apeireth_mesh_protocol::verify_hello(&self.descriptor, &hello) {
            reject(error.to_string()).await;
            return Err(error.into());
        }
        if let Err(error) = self.authorize(&hello.node.id) {
            reject(error.to_string()).await;
            return Err(error);
        }
        if let Err(error) = self.verify_pinned_identity(&hello.node.id, &link) {
            reject(error.to_string()).await;
            return Err(error);
        }

        let peer_id = hello.node.id.clone();
        let ack = hello_env.reply(MeshPayload::HelloAck(HelloAck {
            node: self.descriptor.clone(),
            accepted: true,
            reason: None,
        }));
        if let Err(error) = link.send(ack).await {
            return Err(MeshError::Transport(error.to_string()));
        }

        let initiator = peer_id.clone();
        let Some(generation) = self.activate_peer(hello.node, Arc::clone(&link), initiator) else {
            return Err(MeshError::Superseded {
                kept: peer_id.to_string(),
            });
        };
        self.send_advert_to(&peer_id, &link).await;
        self.spawn_connection_loop(peer_id.clone(), link, generation);
        Ok(peer_id)
    }

    /// Establish a direct connection to a trusted peer by **simultaneous
    /// open**, driven from the production connection path.
    ///
    /// This is the seam the hole-punch machinery was missing: everything
    /// below it already existed (`PresenceStore`, `PunchCoordinator`,
    /// `punch`, pinned QUIC, `activate_peer` convergence); nothing called it.
    /// The steps, in order, each one the existing production step rather
    /// than a test shortcut:
    ///
    /// 1. **Trust first.** An unadmitted peer is refused before any packet
    ///    is sent — a punch authorizes nothing.
    /// 2. **Presence is recorded** through [`MeshNode::note_presence`], so a
    ///    replayed, expired, or lease-stale record fails closed here instead
    ///    of poisoning later dials. The record must describe `peer` and must
    ///    still be inside its lease.
    /// 3. **Coordination is requested** on this node's own
    ///    [`PunchCoordinator`], bound to both current incarnations: our own
    ///    (`local_presence_session`) and the peer's (`remote_presence`).
    /// 4. **Both halves run on one endpoint.** `endpoint` must implement
    ///    *both* [`MeshTransport`] and [`MeshListener`] — one bound socket in
    ///    both roles, so the NAT mapping the peer's candidates describe is
    ///    the mapping this node dials from and accepts on. The generic bound
    ///    is what makes "two uncoordinated sockets" inexpressible rather
    ///    than merely discouraged.
    /// 5. **The surviving link is handshaked** by the ordinary connection
    ///    path: the dial side through [`MeshNode::connect`], the accept side
    ///    through [`MeshNode::accept`] — pinned identity, protocol version,
    ///    adverts, activation and duplicate-connection convergence are
    ///    exactly what any other connection gets.
    ///
    /// Failure is bounded and explicit: [`MeshError::PunchFailed`] means no
    /// authenticated connection appeared inside the window, which is the
    /// caller's signal to fall back to the relay path — never to retry
    /// forever. A [`MeshError::Superseded`] from step 5 is reported as
    /// success: the peer's dial won the convergence race and the pair has
    /// exactly one live link either way.
    pub async fn connect_punched<E>(
        self: &Arc<Self>,
        peer: NodeId,
        remote_presence: &VerifiedPresence,
        local_presence_session: u64,
        endpoint: Arc<E>,
        window_ms: u64,
    ) -> Result<NodeId, MeshError>
    where
        E: MeshTransport + MeshListener + 'static,
    {
        if self.is_shutdown() {
            return Err(MeshError::Transport("node is shut down".to_string()));
        }
        // 1. Trust gate, before a single datagram.
        self.authorize(&peer)?;

        // 2. Presence must be current, ours to dial, and about this peer.
        let now = self.now_ms();
        if remote_presence.node_id() != &peer {
            return Err(MeshError::PunchFailed {
                peer: peer.to_string(),
                reason: format!(
                    "presence describes {}, not {peer}",
                    remote_presence.node_id()
                ),
            });
        }
        if remote_presence.expires_at_ms() <= now {
            return Err(MeshError::PunchFailed {
                peer: peer.to_string(),
                reason: format!(
                    "presence incarnation {} expired at {} (now {now})",
                    remote_presence.session_id(),
                    remote_presence.expires_at_ms()
                ),
            });
        }
        self.note_presence(remote_presence.clone())
            .map_err(|error| MeshError::PunchFailed {
                peer: peer.to_string(),
                reason: format!("presence refused: {error}"),
            })?;

        // 3. One bounded attempt, bound to both incarnations.
        let session = self
            .punch
            .request(
                self.node_id(),
                &peer,
                local_presence_session,
                remote_presence.session_id(),
                now,
                window_ms,
            )
            .map_err(|error| MeshError::PunchFailed {
                peer: peer.to_string(),
                reason: error.to_string(),
            })?;

        // 4. Both halves over the one dual-role socket.
        let dialer: Arc<dyn MeshTransport> = endpoint.clone();
        let listener: Arc<dyn MeshListener> = endpoint.clone();
        let outcome =
            punch(&session, dialer, listener, remote_presence, now)
                .await
                .map_err(|error| MeshError::PunchFailed {
                    peer: peer.to_string(),
                    reason: error.to_string(),
                })?;

        // 5. The survivor goes through the ordinary handshake — the dial side
        // through `connect`, the accept side through `accept`, exactly as if
        // each had arrived on its own — and then the ordinary convergence.
        // Nothing about a punched link is special above this line.
        let handshaken = match outcome.kept_direction {
            crate::Direction::Outbound => self.connect(outcome.kept, peer.clone()).await,
            crate::Direction::Inbound => self.accept(outcome.kept).await,
        };
        match handshaken {
            Ok(connected) => Ok(connected),
            // The peer's dial won the convergence race: exactly one live
            // authenticated link exists either way.
            Err(MeshError::Superseded { .. }) => Ok(peer),
            Err(error) => Err(error),
        }
    }

    /// Forget a peer entirely (§18): drop its connection and its capability
    /// projections so no route survives. Persisted trust removal is the
    /// [`crate::TrustStore::forget`] half of unpairing; the local node
    /// identity is never touched — unpairing is about the *other* side.
    pub fn forget_peer(&self, node: &NodeId) {
        self.disconnect(node);
        self.routes
            .lock()
            .expect("mesh routes poisoned")
            .forget_projection(node);
    }

    /// Close the connection to `peer`. Its projection survives as last-known
    /// state; its known-peer identity is never deleted here.
    pub fn disconnect(&self, node: &NodeId) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if let Some(entry) = peers.get_mut(node) {
            entry.peer.state = PeerState::Closed;
            if let Some(link) = entry.link.take() {
                link.close();
            }
        }
        drop(peers);
        self.fail_pending_for_peer(node, "connection closed");
    }

    /// Current state of a peer connection.
    pub fn peer_state(&self, node: &NodeId) -> Option<PeerState> {
        self.peers
            .lock()
            .expect("mesh peers poisoned")
            .get(node)
            .map(|entry| entry.peer.state)
    }

    /// All known connections, in node-id order.
    pub fn peers(&self) -> Vec<(NodeId, PeerState, u64)> {
        self.peers
            .lock()
            .expect("mesh peers poisoned")
            .values()
            .map(|entry| {
                (
                    entry.peer.node.clone(),
                    entry.peer.state,
                    entry.peer.last_seen_ms,
                )
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // Projections and routing
    // ------------------------------------------------------------------

    /// Routes known for `capability`: the local one (from the canonical index,
    /// read through at query time) plus each remote projection.
    pub fn routes_for(&self, capability: &CapabilityId) -> Vec<crate::route::CapabilityRoute> {
        let catalog = self.executor.catalog();
        self.routes
            .lock()
            .expect("mesh routes poisoned")
            .routes_for(capability, &catalog)
    }

    /// Resolve `capability` under an explicit selection. Never crosses
    /// namespaces: a remote advert cannot shadow a local capability and a local
    /// capability cannot hide a remote one.
    pub fn resolve(
        &self,
        capability: &CapabilityId,
        selection: &RouteSelection,
    ) -> Result<crate::route::CapabilityRoute, MeshError> {
        let catalog = self.executor.catalog();
        self.routes
            .lock()
            .expect("mesh routes poisoned")
            .resolve(capability, selection, &catalog)
    }

    /// One peer's projected capabilities (metadata only).
    pub fn projection(&self, node: &NodeId) -> Vec<RemoteCapability> {
        self.routes
            .lock()
            .expect("mesh routes poisoned")
            .projection(node)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Every projected remote capability (metadata only).
    pub fn remote_capabilities(&self) -> Vec<RemoteCapability> {
        self.routes
            .lock()
            .expect("mesh routes poisoned")
            .remote_capabilities()
            .cloned()
            .collect()
    }

    /// Push a full-snapshot capability advert to every active peer.
    ///
    /// Call this after the local capability surface changes (for example after
    /// a dynamic tool is admitted or removed): the snapshot semantics mean
    /// removals propagate by omission.
    pub async fn publish_advert(&self) {
        let advert = self.advert();
        for (node, link) in self.active_links() {
            let envelope = MeshEnvelope::send(
                self.node_id().clone(),
                node,
                TraceId::new(),
                MeshPayload::CapabilityAdvert(advert.clone()),
            );
            let _ = link.send(envelope).await;
        }
    }

    fn advert(&self) -> CapabilityAdvert {
        CapabilityAdvert {
            modules: catalog_to_modules(self.executor.catalog()),
        }
    }

    async fn send_advert_to(&self, node: &NodeId, link: &Arc<dyn MeshConnection>) {
        let envelope = MeshEnvelope::send(
            self.node_id().clone(),
            node.clone(),
            TraceId::new(),
            MeshPayload::CapabilityAdvert(self.advert()),
        );
        let _ = link.send(envelope).await;
    }

    // ------------------------------------------------------------------
    // Invocation
    // ------------------------------------------------------------------

    /// Invoke a capability under an explicit route selection and await the
    /// canonical tool result.
    ///
    /// The high-level explicit API (debug, SDK, orchestration): it performs
    /// requester-side governance exactly once for remote routes before
    /// delegating to the raw primitive. Local execution is governed exactly
    /// once inside the executor. Callers that already hold their own
    /// authorization — the canonical runtime dispatch path — use
    /// [`MeshNode::start_call`] (or the `RemoteCapabilityPort`) instead, so no
    /// node ever evaluates requester governance twice.
    pub async fn invoke_tool(
        &self,
        selection: &RouteSelection,
        capability: &CapabilityId,
        call: ToolCall,
        meta: InvokeMeta,
    ) -> Result<ToolResult, MeshError> {
        if let RouteSelection::Remote(_) = selection {
            let ctx = MeshInvokeContext {
                session: meta.session,
                trace: meta.trace,
                round: meta.round,
                source: ActionSource::Local,
                deadline_epoch_ms: meta.deadline_epoch_ms.or(self.default_deadline_ms),
                cancelled: CancelFlag::new(),
            };
            self.executor
                .authorize(&ctx, capability, &call.arguments)
                .await
                .map_err(map_exec_error)?;
        }
        self.start_call(selection, capability, call, meta)
            .await?
            .tool_result()
            .await
    }

    /// High-level explicit provider invocation (debug, SDK, benchmarks): the
    /// same governance shape as [`MeshNode::invoke_tool`] — one requester-side
    /// canonical `Action::Completion` evaluation for remote routes, then the
    /// raw primitive. Local execution is governed exactly once inside the
    /// executor.
    pub async fn invoke_provider(
        &self,
        selection: &RouteSelection,
        capability: &CapabilityId,
        request: apeireth_protocol::canonical::NormalizedRequest,
        meta: InvokeMeta,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, MeshError> {
        if let RouteSelection::Remote(_) = selection {
            let ctx = MeshInvokeContext {
                session: meta.session,
                trace: meta.trace,
                round: meta.round,
                source: ActionSource::Local,
                deadline_epoch_ms: meta.deadline_epoch_ms.or(self.default_deadline_ms),
                cancelled: CancelFlag::new(),
            };
            self.executor
                .authorize_completion(&ctx, &request.model, request.messages.len())
                .await
                .map_err(map_exec_error)?;
        }
        match self
            .start_capability_call(
                selection,
                capability,
                CapabilityCall::Provider(request),
                meta,
            )
            .await?
            .outcome()
            .await?
        {
            CapabilityOutcome::Provider(response) => Ok(response),
            other => Err(MeshError::Remote {
                capability: Some(capability.clone()),
                code: apeireth_mesh_protocol::MeshErrorCode::Unsupported,
                message: format!(
                    "unexpected provider outcome shape: {}",
                    crate::kinds::outcome_kind(&other)
                ),
            }),
        }
    }

    /// The raw execution primitive: start an invocation and return its handle
    /// (for cancellation or the outcome).
    ///
    /// Performs **no requester-side governance** — callers must have completed
    /// their own authorization (this is the path the canonical runtime
    /// dispatch takes after its single governance evaluation, and the path
    /// [`MeshNode::invoke_tool`] delegates to after its own). Local and remote
    /// calls get identical semantics: same `message_id` cancellation, same
    /// canonical result shape.
    ///
    /// Async because the remote request is placed on the wire before the
    /// handle is returned: a `Cancel` sent immediately afterwards therefore
    /// cannot overtake its `InvokeCapability` on the same link.
    pub async fn start_call(
        &self,
        selection: &RouteSelection,
        capability: &CapabilityId,
        call: ToolCall,
        meta: InvokeMeta,
    ) -> Result<CallHandle, MeshError> {
        self.start_capability_call(selection, capability, CapabilityCall::Tool(call), meta)
            .await
    }

    /// The generalized raw execution primitive behind both tool and provider
    /// paths: **no requester-side governance** (callers hold their own
    /// authorization), identical `message_id` cancellation and correlation
    /// semantics for every call shape.
    pub async fn start_capability_call(
        &self,
        selection: &RouteSelection,
        capability: &CapabilityId,
        call: CapabilityCall,
        meta: InvokeMeta,
    ) -> Result<CallHandle, MeshError> {
        let catalog = self.executor.catalog();
        let route = self
            .routes
            .lock()
            .expect("mesh routes poisoned")
            .resolve(capability, selection, &catalog)?;
        let message_id = RequestId::new();
        let deadline = meta.deadline_epoch_ms.or(self.default_deadline_ms);
        let (tx, rx) = oneshot::channel();

        match route.location {
            apeireth_mesh_protocol::PluginLocation::Local => {
                let flag = CancelFlag::new();
                self.invocations
                    .lock()
                    .expect("mesh invocations poisoned")
                    .insert((None, message_id), flag.clone());
                let executor = Arc::clone(&self.executor);
                let invocations = Arc::clone(&self.invocations);
                let ctx = MeshInvokeContext {
                    session: meta.session,
                    trace: meta.trace,
                    round: meta.round,
                    source: ActionSource::Local,
                    deadline_epoch_ms: deadline,
                    cancelled: flag,
                };
                let capability = capability.clone();
                tokio::spawn(async move {
                    let result = match &call {
                        CapabilityCall::Tool(tool_call) => executor
                            .execute_tool(&ctx, &capability, tool_call)
                            .await
                            .map(CapabilityOutcome::Tool),
                        CapabilityCall::Provider(request) => executor
                            .execute_provider(&ctx, &capability, request)
                            .await
                            .map(CapabilityOutcome::Provider),
                        // `CapabilityCall` is non-exhaustive across crates.
                        other => Err(MeshExecError::Internal {
                            capability: capability.clone(),
                            message: format!(
                                "unsupported invocation shape: {}",
                                crate::kinds::call_kind(&other)
                            ),
                        }),
                    }
                    .map_err(map_exec_error);
                    invocations
                        .lock()
                        .expect("mesh invocations poisoned")
                        .remove(&(None, message_id));
                    let _ = tx.send(result);
                });
                Ok(CallHandle {
                    message_id,
                    target: CallTarget::Local,
                    rx,
                    timeout: None,
                    _pending: None,
                })
            }
            apeireth_mesh_protocol::PluginLocation::Remote(node) => {
                self.ensure_peer_dispatchable(&node)?;
                {
                    let pending_for_peer = self
                        .calls
                        .lock()
                        .expect("mesh calls poisoned")
                        .values()
                        .filter(|call| call.peer == node)
                        .count();
                    if pending_for_peer >= self.max_pending_calls {
                        return Err(MeshError::Backpressure {
                            node: node.to_string(),
                            in_flight: pending_for_peer,
                            cap: self.max_pending_calls,
                        });
                    }
                }
                self.calls.lock().expect("mesh calls poisoned").insert(
                    message_id,
                    PendingCall {
                        peer: node.clone(),
                        tx,
                    },
                );
                let envelope = MeshEnvelope {
                    protocol_version: MESH_PROTOCOL_VERSION,
                    message_id,
                    in_reply_to: None,
                    trace_id: meta.trace,
                    source_node: self.node_id().clone(),
                    target_node: node.clone(),
                    payload: MeshPayload::InvokeCapability(InvokeCapability {
                        capability: capability.clone(),
                        kind: kind_of(capability),
                        call,
                        session: meta.session,
                        round: meta.round,
                        deadline_epoch_ms: deadline,
                    }),
                };
                // Sent inline, so a `Cancel` for this call can never overtake
                // it on the wire.
                self.send_to_peer(&node, envelope).await;
                Ok(CallHandle {
                    message_id,
                    target: CallTarget::Remote(node),
                    rx,
                    timeout: self.call_timeout,
                    _pending: Some(PendingGuard {
                        calls: Arc::clone(&self.calls),
                        id: message_id,
                    }),
                })
            }
        }
    }

    /// Best-effort cancellation of one started call.
    ///
    /// Local executions get their [`CancelFlag`] set; remote executions also
    /// receive a wire `Cancel` carrying the call's `message_id`. A capability
    /// that never checks its flag runs to completion — cancellation is
    /// cooperative everywhere, local and remote alike.
    pub async fn cancel(&self, message_id: RequestId) {
        if let Some(flag) = self
            .invocations
            .lock()
            .expect("mesh invocations poisoned")
            .get(&(None, message_id))
        {
            flag.cancel();
        }
        let peer = self
            .calls
            .lock()
            .expect("mesh calls poisoned")
            .get(&message_id)
            .map(|call| call.peer.clone());
        if let Some(peer) = peer {
            let envelope = MeshEnvelope::send(
                self.node_id().clone(),
                peer.clone(),
                TraceId::new(),
                MeshPayload::Cancel(apeireth_mesh_protocol::Cancel { target: message_id }),
            );
            self.send_to_peer(&peer, envelope).await;
        }
    }

    fn ensure_peer_dispatchable(&self, node: &NodeId) -> Result<(), MeshError> {
        let peers = self.peers.lock().expect("mesh peers poisoned");
        let entry = peers.get(node).ok_or_else(|| MeshError::PeerUnavailable {
            node: node.to_string(),
            state: "unknown",
        })?;
        if entry.peer.state.is_dispatchable() {
            Ok(())
        } else {
            Err(MeshError::PeerUnavailable {
                node: node.to_string(),
                state: entry.peer.state.as_str(),
            })
        }
    }

    async fn send_to_peer(&self, node: &NodeId, envelope: MeshEnvelope) {
        let (link, generation) = {
            let peers = self.peers.lock().expect("mesh peers poisoned");
            match peers.get(node) {
                Some(entry) => (entry.link.clone(), entry.generation),
                None => (None, 0),
            }
        };
        if let Some(link) = link {
            if link.send(envelope).await.is_err() {
                link.close();
                self.fail_pending_for_peer(node, "connection lost while sending");
                self.mark_unreachable(node, generation);
            }
        }
    }

    // ------------------------------------------------------------------
    // Heartbeat and liveness
    // ------------------------------------------------------------------

    /// Send one heartbeat to every active peer.
    pub async fn heartbeat_once(&self) {
        let sent_at_ms = self.now_ms();
        for (node, link) in self.active_links() {
            let envelope = MeshEnvelope::send(
                self.node_id().clone(),
                node,
                TraceId::new(),
                MeshPayload::Heartbeat(apeireth_mesh_protocol::Heartbeat { sent_at_ms }),
            );
            let _ = link.send(envelope).await;
        }
    }

    /// Demote silent peers to [`PeerState::Unreachable`].
    ///
    /// Their routes stop dispatching; their projections and known-peer
    /// identities survive, so a reconnect restores service without pretending
    /// anything was permanently lost.
    ///
    /// Lapsed presence leases are pruned in the same sweep, and every punch
    /// attempt bound to a pruned peer is invalidated: an attempt may only
    /// ever dial a snapshot that still exists.
    pub fn sweep_heartbeats(&self, now_ms: u64) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        let mut gone = Vec::new();
        for entry in peers.values_mut() {
            if entry.peer.state == PeerState::Active
                && now_ms.saturating_sub(entry.peer.last_seen_ms) > self.heartbeat_timeout_ms
            {
                entry.peer.state = PeerState::Unreachable;
                gone.push(entry.peer.node.clone());
            }
        }
        drop(peers);
        for node in gone {
            self.fail_pending_for_peer(&node, "peer unreachable (heartbeat timeout)");
        }
        for expired in self
            .presence
            .lock()
            .expect("mesh presence poisoned")
            .prune_expired(now_ms)
        {
            self.punch.invalidate_peer(&expired);
        }
    }

    /// Spawn a background heartbeat loop. Optional: tests drive
    /// [`MeshNode::heartbeat_once`] and [`MeshNode::sweep_heartbeats`]
    /// deterministically instead.
    pub fn spawn_heartbeat(self: &Arc<Self>, interval_ms: u64) -> tokio::task::JoinHandle<()> {
        let node = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            let mut stop = node.shutdown.subscribe();
            loop {
                tokio::select! {
                    _ = ticker.tick() => {}
                    _ = stop.wait_for(|closed| *closed) => return,
                }
                node.heartbeat_once().await;
                node.sweep_heartbeats(node.now_ms());
            }
        })
    }

    // ------------------------------------------------------------------
    // Connection loop and execution side
    // ------------------------------------------------------------------

    fn spawn_connection_loop(
        self: &Arc<Self>,
        peer: NodeId,
        link: Arc<dyn MeshConnection>,
        generation: u64,
    ) {
        let node = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                match link.recv().await {
                    Ok(envelope) => {
                        if node.handle_envelope(&peer, envelope, &link).await.is_err() {
                            link.close();
                            node.fail_pending_for_peer(&peer, "connection terminated");
                            node.mark_closed(&peer, generation);
                            return;
                        }
                    }
                    Err(_) => {
                        node.fail_pending_for_peer(&peer, "connection lost");
                        node.mark_unreachable(&peer, generation);
                        return;
                    }
                }
            }
        });
    }

    /// Serve inbound connections from a listener until it closes.
    ///
    /// The accept loop outlives any single handshake: a connection that cannot
    /// authenticate promptly is closed by its own handshake deadline without
    /// disturbing the listener.
    pub fn serve(
        self: &Arc<Self>,
        listener: Arc<dyn crate::transport::MeshListener>,
    ) -> tokio::task::JoinHandle<()> {
        let node = Arc::clone(self);
        let handshakes = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_HANDSHAKES));
        tokio::spawn(async move {
            let mut stop = node.shutdown.subscribe();
            loop {
                let accepted = tokio::select! {
                    accepted = listener.accept() => accepted,
                    _ = stop.wait_for(|closed| *closed) => return,
                };
                match accepted {
                    Ok(link) => {
                        let Ok(permit) = Arc::clone(&handshakes).try_acquire_owned() else {
                            link.close();
                            continue;
                        };
                        let per_connection = Arc::clone(&node);
                        tokio::spawn(async move {
                            let _ = per_connection.accept(link).await;
                            drop(permit);
                        });
                    }
                    Err(_) => return,
                }
            }
        })
    }

    /// Dispatch one inbound envelope. `Err` terminates the connection.
    async fn handle_envelope(
        &self,
        peer: &NodeId,
        envelope: MeshEnvelope,
        link: &Arc<dyn MeshConnection>,
    ) -> Result<(), MeshError> {
        self.check_inbound_envelope(&envelope, peer)?;
        // Trust can be withdrawn while a connection is up.
        self.authorize(peer)?;
        self.note_seen(peer);

        // Match on a clone of the payload so the envelope itself survives whole
        // into the invocation path, which needs it to build the correlated
        // reply.
        match envelope.payload.clone() {
            MeshPayload::CapabilityAdvert(advert) => self
                .routes
                .lock()
                .expect("mesh routes poisoned")
                .ingest_advert(peer, &advert),
            MeshPayload::InvokeCapability(invoke) => {
                self.spawn_invocation(peer.clone(), envelope, invoke, Arc::clone(link))
            }
            MeshPayload::CapabilityResult(result) => {
                self.complete_call(peer, envelope.in_reply_to, Ok(result.outcome))
            }
            MeshPayload::CapabilityError(error) => self.complete_call(
                peer,
                envelope.in_reply_to,
                Err(MeshError::from_capability_error(error)),
            ),
            MeshPayload::Heartbeat(_) => Ok(()),
            MeshPayload::Cancel(cancel) => {
                if let Some(flag) = self
                    .invocations
                    .lock()
                    .expect("mesh invocations poisoned")
                    .get(&(Some(peer.clone()), cancel.target))
                {
                    flag.cancel();
                }
                Ok(())
            }
            MeshPayload::Pair(_) => Err(MeshError::Malformed {
                sender: peer.to_string(),
                reason: "pairing messages are not allowed on established mesh connections"
                    .to_string(),
            }),
            other => Err(MeshError::Malformed {
                sender: peer.to_string(),
                reason: format!(
                    "unexpected post-handshake message: {}",
                    crate::kinds::payload_kind(&other)
                ),
            }),
        }
    }

    fn spawn_invocation(
        &self,
        peer: NodeId,
        envelope: MeshEnvelope,
        invoke: InvokeCapability,
        link: Arc<dyn MeshConnection>,
    ) -> Result<(), MeshError> {
        let key: InvocationKey = (Some(peer.clone()), envelope.message_id);
        let flag = CancelFlag::new();
        let saturated = {
            let mut invocations = self.invocations.lock().expect("mesh invocations poisoned");
            if invocations.contains_key(&key) {
                return Err(MeshError::Malformed {
                    sender: peer.to_string(),
                    reason: format!("message id {} is already executing", envelope.message_id),
                });
            }
            let running = invocations
                .keys()
                .filter(|(origin, _)| origin.as_ref() == Some(&peer))
                .count();
            if running >= self.max_inbound_per_peer {
                true
            } else {
                invocations.insert(key.clone(), flag.clone());
                false
            }
        };
        if saturated {
            let capability = invoke.capability.clone();
            let cap = self.max_inbound_per_peer;
            tokio::spawn(async move {
                let refusal = MeshPayload::CapabilityError(CapabilityError {
                    capability: Some(capability),
                    code: MeshErrorCode::Backpressure,
                    message: format!("peer already has {cap} invocations executing here"),
                });
                let _ = link.send(envelope.reply(refusal)).await;
            });
            return Ok(());
        }

        let executor = Arc::clone(&self.executor);
        let invocations = Arc::clone(&self.invocations);
        tokio::spawn(async move {
            let trace = envelope.trace_id;
            // `reply` reuses the request envelope: same trace, reversed
            // direction, `in_reply_to` correlation — trace propagation by
            // construction rather than by remembering to copy fields.
            let reply_template = envelope.clone();
            let outcome = execute_remote(&executor, &peer, trace, invoke, flag).await;
            invocations
                .lock()
                .expect("mesh invocations poisoned")
                .remove(&key);
            let payload = match outcome {
                Ok((capability, outcome)) => MeshPayload::CapabilityResult(CapabilityResult {
                    capability,
                    outcome,
                }),
                Err(error) => MeshPayload::CapabilityError(error),
            };
            let _ = link.send(reply_template.reply(payload)).await;
        });
        Ok(())
    }

    /// Complete a pending call from a reply. A reply is honoured only from the
    /// peer the call was sent to: another connected peer that guesses or
    /// observes a message id must not be able to answer for it. A reply to an
    /// unknown id is ignored (late reply after a timeout or cancel).
    fn complete_call(
        &self,
        peer: &NodeId,
        in_reply_to: Option<RequestId>,
        result: Result<CapabilityOutcome, MeshError>,
    ) -> Result<(), MeshError> {
        let Some(message_id) = in_reply_to else {
            return Ok(());
        };
        let mut calls = self.calls.lock().expect("mesh calls poisoned");
        match calls.get(&message_id) {
            Some(pending) if &pending.peer != peer => Err(MeshError::Malformed {
                sender: peer.to_string(),
                reason: format!("reply to {message_id}, a call that was not sent to this peer"),
            }),
            Some(_) => {
                if let Some(pending) = calls.remove(&message_id) {
                    let _ = pending.tx.send(result);
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn fail_pending_for_peer(&self, peer: &NodeId, reason: &str) {
        let mut calls = self.calls.lock().expect("mesh calls poisoned");
        let stranded: Vec<RequestId> = calls
            .iter()
            .filter(|(_, call)| &call.peer == peer)
            .map(|(id, _)| *id)
            .collect();
        for id in stranded {
            if let Some(pending) = calls.remove(&id) {
                let _ = pending
                    .tx
                    .send(Err(MeshError::Transport(reason.to_string())));
            }
        }
    }

    fn check_inbound_envelope(
        &self,
        envelope: &MeshEnvelope,
        expected_peer: &NodeId,
    ) -> Result<(), MeshError> {
        if envelope.protocol_version != MESH_PROTOCOL_VERSION {
            return Err(MeshError::Malformed {
                sender: envelope.source_node.to_string(),
                reason: format!(
                    "envelope protocol version {} != {}",
                    envelope.protocol_version, MESH_PROTOCOL_VERSION
                ),
            });
        }
        if &envelope.source_node != expected_peer {
            return Err(MeshError::Malformed {
                sender: envelope.source_node.to_string(),
                reason: format!(
                    "envelope source is not the handshaken peer {expected_peer} (identity confusion)"
                ),
            });
        }
        if envelope.target_node != *self.node_id() {
            return Err(MeshError::Malformed {
                sender: envelope.source_node.to_string(),
                reason: format!(
                    "envelope addressed to {}, not {}",
                    envelope.target_node,
                    self.node_id()
                ),
            });
        }
        Ok(())
    }

    /// Enforce the trust pin against transport identity evidence.
    ///
    /// A pinned peer must present exactly the pinned fingerprint — the right
    /// key under the wrong NodeId, a wrong key, and a keyless "proof" all fail
    /// closed. An unpinned entry is only usable on a link that makes no
    /// identity claim (the in-memory test fabric): a link that *does* present
    /// a key for an unpinned node is refused, never trusted on first use.
    fn verify_pinned_identity(
        &self,
        node: &NodeId,
        link: &Arc<dyn MeshConnection>,
    ) -> Result<(), MeshError> {
        let Some(pin) = self.pinned(node) else {
            return match link.peer_fingerprint() {
                None => Ok(()),
                Some(presented) => Err(MeshError::Untrusted {
                    node: node.to_string(),
                    reason: format!("no key is pinned for this node; presented {presented}"),
                }),
            };
        };
        match link.peer_fingerprint() {
            Some(presented) if presented == pin => Ok(()),
            Some(presented) => Err(MeshError::Untrusted {
                node: node.to_string(),
                reason: format!("pinned key mismatch: expected {pin}, presented {presented}"),
            }),
            None => Err(MeshError::Untrusted {
                node: node.to_string(),
                reason: "pinned peer presented no transport identity".to_string(),
            }),
        }
    }

    /// Install an authenticated connection, resolving duplicates
    /// deterministically. A dead incumbent always yields to the new
    /// connection (reconnect races); between two *live* connections the one
    /// initiated by the lexicographically smaller NodeId wins, and both ends
    /// compute the same winner — simultaneous dials converge on exactly one
    /// live link. Returns the connection generation when installed, `None`
    /// when superseded.
    fn activate_peer(
        &self,
        node: NodeDescriptor,
        link: Arc<dyn MeshConnection>,
        initiator: NodeId,
    ) -> Option<u64> {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if let Some(existing) = peers.get(&node.id) {
            // `generation == 0` entries are handshake placeholders.
            //
            // Duplicate policy, computed identically on both ends so
            // simultaneous dials converge on the same surviving link:
            //   - different initiators: the connection initiated by the
            //     lexicographically smaller NodeId wins;
            //   - same initiator (a reconnect): the newest wins.
            // A dead incumbent always yields, and whatever loses is closed
            // here so the loser's loop tears down symmetrically.
            let live =
                existing.generation != 0 && existing.link.as_ref().is_some_and(|l| !l.is_closed());
            // Stated once, in `crate::convergence`, so the rule that both
            // ends evaluate is a single tested function rather than an inline
            // comparison each side could drift away from.
            let keep_new = !live
                || crate::convergence::new_connection_survives(&existing.initiator, &initiator);
            if !keep_new {
                link.close();
                return None;
            }
            // Close the outgoing *real* connection being replaced — never the
            // handshake placeholder, whose link is the very one activating.
            if existing.generation != 0 {
                if let Some(old_link) = existing.link.clone() {
                    old_link.close();
                }
            }
        }
        let generation = self
            .generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        let now = self.now_ms();
        peers.insert(
            node.id.clone(),
            PeerEntry {
                peer: Peer {
                    node: node.id,
                    name: node.name,
                    state: PeerState::Active,
                    last_seen_ms: now,
                },
                link: Some(link),
                initiator,
                generation,
            },
        );
        Some(generation)
    }

    /// Drop a failed handshake's placeholder. A live entry (generation > 0) is
    /// never touched: a failed second dial must not evict the working link.
    fn abandon_handshake(&self, node: &NodeId) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if peers.get(node).is_some_and(|entry| entry.generation == 0) {
            peers.remove(node);
        }
    }

    fn note_seen(&self, node: &NodeId) {
        let now = self.now_ms();
        if let Some(entry) = self
            .peers
            .lock()
            .expect("mesh peers poisoned")
            .get_mut(node)
        {
            entry.peer.note_seen(now);
        }
    }

    /// Stale-epoch guard: a superseded connection's teardown must not clobber
    /// the state of the connection that replaced it.
    fn mark_unreachable(&self, node: &NodeId, generation: u64) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if let Some(entry) = peers.get_mut(node) {
            if entry.generation != generation {
                return;
            }
            if entry.peer.state != PeerState::Closed {
                entry.peer.state = PeerState::Unreachable;
            }
            entry.link = None;
        }
    }

    /// Stale-epoch guard, as [`MeshNode::mark_unreachable`].
    fn mark_closed(&self, node: &NodeId, generation: u64) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if let Some(entry) = peers.get_mut(node) {
            if entry.generation != generation {
                return;
            }
            entry.peer.state = PeerState::Closed;
            entry.link = None;
        }
    }

    fn active_links(&self) -> Vec<(NodeId, Arc<dyn MeshConnection>)> {
        self.peers
            .lock()
            .expect("mesh peers poisoned")
            .values()
            .filter(|entry| entry.peer.state.is_dispatchable())
            .filter_map(|entry| {
                entry
                    .link
                    .clone()
                    .map(|link| (entry.peer.node.clone(), link))
            })
            .collect()
    }
}

fn kind_of(capability: &CapabilityId) -> MeshCapabilityKind {
    MeshCapabilityKind::from_id(capability).unwrap_or(MeshCapabilityKind::Extension)
}

fn map_exec_error(error: MeshExecError) -> MeshError {
    MeshError::from_capability_error(error.to_capability_error())
}
/// Execute one inbound invocation against the canonical executor, preserving
/// the caller's trace and marking failures with wire-accurate codes.
async fn execute_remote(
    executor: &Arc<dyn MeshCapabilityExecutor>,
    peer: &NodeId,
    trace: TraceId,
    invoke: InvokeCapability,
    flag: CancelFlag,
) -> Result<(CapabilityId, CapabilityOutcome), CapabilityError> {
    let capability = invoke.capability.clone();

    if kind_of(&capability) != invoke.kind {
        return Err(CapabilityError {
            capability: Some(capability),
            code: MeshErrorCode::Unsupported,
            message: "capability kind disagrees with its id prefix".to_string(),
        });
    }

    let ctx = MeshInvokeContext {
        session: invoke.session,
        // The caller's trace crosses the hop unchanged — a remote execution is
        // part of the same causal span as the local work that caused it.
        trace,
        round: invoke.round,
        source: ActionSource::Remote { node: peer.clone() },
        deadline_epoch_ms: invoke.deadline_epoch_ms,
        cancelled: flag,
    };

    match invoke.call {
        CapabilityCall::Tool(call) => executor
            .execute_tool(&ctx, &capability, &call)
            .await
            .map(|result| (capability.clone(), CapabilityOutcome::Tool(result)))
            .map_err(|error| error.to_capability_error()),
        CapabilityCall::Provider(request) => executor
            .execute_provider(&ctx, &capability, &request)
            .await
            .map(|response| (capability.clone(), CapabilityOutcome::Provider(response)))
            .map_err(|error| error.to_capability_error()),
        other => Err(CapabilityError {
            capability: Some(capability),
            code: MeshErrorCode::Unsupported,
            message: format!(
                "unsupported invocation shape: {}",
                crate::kinds::call_kind(&other)
            ),
        }),
    }
}
