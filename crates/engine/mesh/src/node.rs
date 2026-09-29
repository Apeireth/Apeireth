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
use std::sync::{Arc, Mutex};

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
use crate::route::{RemoteCapability, RouteSelection, RouteTable};
use crate::transport::MeshConnection;
use crate::trust::TrustPolicy;

/// Default silence after which a peer is considered unreachable.
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 15_000;

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
}

impl CallHandle {
    /// Await the raw capability outcome.
    pub async fn outcome(self) -> Result<CapabilityOutcome, MeshError> {
        self.rx
            .await
            .map_err(|_| MeshError::Transport("call completed without an outcome".into()))?
    }

    /// Await the canonical tool result of a tool invocation.
    pub async fn tool_result(self) -> Result<ToolResult, MeshError> {
        match self.outcome().await? {
            CapabilityOutcome::Tool(result) => Ok(result),
            other => Err(MeshError::Remote {
                capability: None,
                code: MeshErrorCode::Unsupported,
                message: format!("unexpected capability outcome shape: {other:?}"),
            }),
        }
    }
}

struct PeerEntry {
    peer: Peer,
    link: Option<Arc<dyn MeshConnection>>,
}

struct PendingCall {
    peer: NodeId,
    tx: oneshot::Sender<Result<CapabilityOutcome, MeshError>>,
}

/// One runtime's mesh presence.
pub struct MeshNode {
    descriptor: NodeDescriptor,
    clock: Arc<dyn Clock>,
    executor: Arc<dyn MeshCapabilityExecutor>,
    trust: TrustPolicy,
    heartbeat_timeout_ms: u64,
    default_deadline_ms: Option<u64>,
    peers: Mutex<BTreeMap<NodeId, PeerEntry>>,
    routes: Mutex<RouteTable>,
    /// Caller-side calls awaiting a remote response.
    calls: Mutex<BTreeMap<RequestId, PendingCall>>,
    /// In-flight executions *in this runtime*, for cooperative cancellation.
    invocations: Arc<Mutex<BTreeMap<RequestId, CancelFlag>>>,
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
            trust: config.trust,
            heartbeat_timeout_ms: config.heartbeat_timeout_ms,
            default_deadline_ms: config.default_deadline_ms,
            peers: Mutex::new(BTreeMap::new()),
            routes: Mutex::new(RouteTable::new()),
            calls: Mutex::new(BTreeMap::new()),
            invocations: Arc::new(Mutex::new(BTreeMap::new())),
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
        self.trust.authorize(&expected)?;
        let now = self.now_ms();
        {
            let mut peers = self.peers.lock().expect("mesh peers poisoned");
            peers.insert(
                expected.clone(),
                PeerEntry {
                    peer: Peer::connecting(expected.clone(), "", now),
                    link: Some(Arc::clone(&link)),
                },
            );
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

        let ack_env = match link.recv().await {
            Ok(envelope) => envelope,
            Err(error) => {
                self.abandon_handshake(&expected);
                return Err(MeshError::Transport(error.to_string()));
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
        if let Err(error) = self.trust.authorize(&ack.node.id) {
            self.abandon_handshake(&expected);
            return Err(error);
        }

        // `verify_hello_ack` pinned the peer to `expected`, so returning it is
        // returning the handshaken identity.
        self.activate_peer(ack.node, Arc::clone(&link));
        self.send_advert_to(&expected, &link).await;
        self.spawn_connection_loop(expected.clone(), link);
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
        let hello_env = match link.recv().await {
            Ok(envelope) => envelope,
            Err(error) => return Err(MeshError::Transport(error.to_string())),
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
                    reason: format!("expected Hello, got {other:?}"),
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
        if let Err(error) = self.trust.authorize(&hello.node.id) {
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

        self.activate_peer(hello.node, Arc::clone(&link));
        self.send_advert_to(&peer_id, &link).await;
        self.spawn_connection_loop(peer_id.clone(), link);
        Ok(peer_id)
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
            apeireth_mesh_protocol::ModuleLocation::Local => {
                let flag = CancelFlag::new();
                self.invocations
                    .lock()
                    .expect("mesh invocations poisoned")
                    .insert(message_id, flag.clone());
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
                    let result = executor
                        .execute_tool(&ctx, &capability, &call)
                        .await
                        .map(CapabilityOutcome::Tool)
                        .map_err(|error| map_exec_error(error));
                    invocations
                        .lock()
                        .expect("mesh invocations poisoned")
                        .remove(&message_id);
                    let _ = tx.send(result);
                });
                Ok(CallHandle {
                    message_id,
                    target: CallTarget::Local,
                    rx,
                })
            }
            apeireth_mesh_protocol::ModuleLocation::Remote(node) => {
                self.ensure_peer_dispatchable(&node)?;
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
                        call: CapabilityCall::Tool(call),
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
            .get(&message_id)
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
        let link = self
            .peers
            .lock()
            .expect("mesh peers poisoned")
            .get(node)
            .and_then(|entry| entry.link.clone());
        if let Some(link) = link {
            if link.send(envelope).await.is_err() {
                link.close();
                self.fail_pending_for_peer(node, "connection lost while sending");
                self.mark_unreachable(node);
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
    }

    /// Spawn a background heartbeat loop. Optional: tests drive
    /// [`MeshNode::heartbeat_once`] and [`MeshNode::sweep_heartbeats`]
    /// deterministically instead.
    pub fn spawn_heartbeat(self: &Arc<Self>, interval_ms: u64) -> tokio::task::JoinHandle<()> {
        let node = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            loop {
                ticker.tick().await;
                node.heartbeat_once().await;
                node.sweep_heartbeats(node.now_ms());
            }
        })
    }

    // ------------------------------------------------------------------
    // Connection loop and execution side
    // ------------------------------------------------------------------

    fn spawn_connection_loop(self: &Arc<Self>, peer: NodeId, link: Arc<dyn MeshConnection>) {
        let node = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                match link.recv().await {
                    Ok(envelope) => {
                        if node.handle_envelope(&peer, envelope, &link).await.is_err() {
                            link.close();
                            node.fail_pending_for_peer(&peer, "connection terminated");
                            node.mark_closed(&peer);
                            return;
                        }
                    }
                    Err(_) => {
                        node.fail_pending_for_peer(&peer, "connection lost");
                        node.mark_unreachable(&peer);
                        return;
                    }
                }
            }
        });
    }

    /// Dispatch one inbound envelope. `Err` terminates the connection.
    async fn handle_envelope(
        &self,
        peer: &NodeId,
        envelope: MeshEnvelope,
        link: &Arc<dyn MeshConnection>,
    ) -> Result<(), MeshError> {
        self.check_inbound_envelope(&envelope, peer)?;
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
                self.spawn_invocation(peer.clone(), envelope, invoke, Arc::clone(link));
                Ok(())
            }
            MeshPayload::CapabilityResult(result) => {
                self.complete_call(envelope.in_reply_to, Ok(result.outcome));
                Ok(())
            }
            MeshPayload::CapabilityError(error) => {
                self.complete_call(
                    envelope.in_reply_to,
                    Err(MeshError::from_capability_error(error)),
                );
                Ok(())
            }
            MeshPayload::Heartbeat(_) => Ok(()),
            MeshPayload::Cancel(cancel) => {
                if let Some(flag) = self
                    .invocations
                    .lock()
                    .expect("mesh invocations poisoned")
                    .get(&cancel.target)
                {
                    flag.cancel();
                }
                Ok(())
            }
            other => Err(MeshError::Malformed {
                sender: peer.to_string(),
                reason: format!("unexpected post-handshake message: {other:?}"),
            }),
        }
    }

    fn spawn_invocation(
        &self,
        peer: NodeId,
        envelope: MeshEnvelope,
        invoke: InvokeCapability,
        link: Arc<dyn MeshConnection>,
    ) {
        let flag = CancelFlag::new();
        self.invocations
            .lock()
            .expect("mesh invocations poisoned")
            .insert(envelope.message_id, flag.clone());

        let executor = Arc::clone(&self.executor);
        let invocations = Arc::clone(&self.invocations);
        tokio::spawn(async move {
            let message_id = envelope.message_id;
            let trace = envelope.trace_id;
            // `reply` reuses the request envelope: same trace, reversed
            // direction, `in_reply_to` correlation — trace propagation by
            // construction rather than by remembering to copy fields.
            let reply_template = envelope.clone();
            let outcome = execute_remote(&executor, &peer, trace, invoke, flag).await;
            invocations
                .lock()
                .expect("mesh invocations poisoned")
                .remove(&message_id);
            let payload = match outcome {
                Ok((capability, result)) => MeshPayload::CapabilityResult(CapabilityResult {
                    capability,
                    outcome: CapabilityOutcome::Tool(result),
                }),
                Err(error) => MeshPayload::CapabilityError(error),
            };
            let _ = link.send(reply_template.reply(payload)).await;
        });
    }

    fn complete_call(
        &self,
        in_reply_to: Option<RequestId>,
        result: Result<CapabilityOutcome, MeshError>,
    ) {
        let Some(message_id) = in_reply_to else {
            return;
        };
        let pending = self
            .calls
            .lock()
            .expect("mesh calls poisoned")
            .remove(&message_id);
        if let Some(pending) = pending {
            let _ = pending.tx.send(result);
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

    fn activate_peer(&self, node: NodeDescriptor, link: Arc<dyn MeshConnection>) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
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
            },
        );
    }

    fn abandon_handshake(&self, node: &NodeId) {
        self.peers.lock().expect("mesh peers poisoned").remove(node);
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

    fn mark_unreachable(&self, node: &NodeId) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if let Some(entry) = peers.get_mut(node) {
            if entry.peer.state != PeerState::Closed {
                entry.peer.state = PeerState::Unreachable;
            }
            entry.link = None;
        }
    }

    fn mark_closed(&self, node: &NodeId) {
        let mut peers = self.peers.lock().expect("mesh peers poisoned");
        if let Some(entry) = peers.get_mut(node) {
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
) -> Result<(CapabilityId, ToolResult), CapabilityError> {
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
            .map(|result| (capability.clone(), result))
            .map_err(|error| error.to_capability_error()),
        other => Err(CapabilityError {
            capability: Some(capability),
            code: MeshErrorCode::Unsupported,
            message: format!("unsupported invocation shape: {other:?}"),
        }),
    }
}
