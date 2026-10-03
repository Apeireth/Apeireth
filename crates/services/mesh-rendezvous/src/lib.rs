//! Apeireth Mesh Phase 2C1 WAN rendezvous + encrypted relay server.
//!
//! # What this is
//!
//! Connectivity infrastructure. It answers two questions and nothing else:
//!
//! - **Rendezvous** — "where is Node B currently reachable?" It stores signed,
//!   expiring presence records and answers lookups.
//! - **Relay** — "forward opaque bytes between A and B." It wires two nodes
//!   into a circuit and forwards end-to-end ciphertext frames between them.
//!
//! # What this is not
//!
//! Not a peer, not a trust authority, not an execution host. It holds no
//! [`NodeId`](apeireth_mesh_protocol::NodeId)-scoped plugin, no capability, no
//! plugin ownership, no `KnownPeer` trust. Nodes A and B trust *each other*;
//! neither trusts this server. It can lie about presence, drop traffic, or
//! reorder frames — but without a node's private key it cannot impersonate a
//! peer, and because traffic is end-to-end encrypted by the nodes, it cannot
//! read a single `MeshEnvelope`.
//!
//! # Threat model
//!
//! The server is assumed **untrusted** for confidentiality and peer identity:
//! it sees only routing metadata (node ids, circuit ids, byte counts) and
//! opaque ciphertext. It is trusted only for *availability* — a malicious
//! server can deny service, never forge trust or decrypt payloads. Every
//! presence record is signature-checked; every relay circuit is bounded.
//!
//! # Defending the server's own availability
//!
//! A node id is not derived from a key, so the server cannot know who *should*
//! hold an id. It enforces the next best thing:
//!
//! - **First key wins while the lease lives.** A live registration can be
//!   refreshed or replaced only by the same public key; another key is refused
//!   until the lease lapses.
//! - **Leases are monotonic and bounded.** A refresh must carry a later expiry
//!   than the one it replaces (a replayed record cannot rebind a node to the
//!   replayer's connection), and no lease may exceed the protocol maximum.
//! - **Everything is bound to the registered connection.** Withdraw, circuit
//!   open/accept/close and frame injection must come from the connection that
//!   holds the registration (or the circuit's own endpoints).
//! - **Everything is bounded and timed.** Connections (total and per IP),
//!   pre-registration time, idle time, write time, pending-circuit lifetime
//!   and per-node circuits all have explicit limits.
//!
//! The control channel is plain TCP: an on-path attacker can drop or delay
//! traffic and race a captured registration. It still cannot read relayed
//! traffic or impersonate a peer, because the nodes authenticate each other
//! end to end. A TLS control channel is future work.

use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use apeireth_mesh_protocol::observe::{decode_probe, encode_reply, OBSERVE_DATAGRAM_BYTES};
use apeireth_mesh_protocol::rendezvous::{
    decode_frame, encode_control, encode_relay, Frame, PresenceRecord, RelayCircuitId,
    RelayControl, RelayFrame, RendezvousRequest, RendezvousResponse, PRESENCE_MAX_TTL_MS,
};
use apeireth_mesh_protocol::NodeId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Mutex as AsyncMutex;

/// Server configuration. Every limit is explicit — a self-hosted deployment
/// sets these; there is no hard-coded SaaS endpoint anywhere.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// The address to bind (`127.0.0.1:0` picks a free port in tests).
    pub listen: SocketAddr,
    /// Optional UDP address to answer endpoint-observation probes on.
    ///
    /// `None` disables observation; the server is still a complete rendezvous
    /// and relay. `127.0.0.1:0` picks a free port in tests. The probe answer is
    /// a reachability *hint* only — see [`apeireth_mesh_protocol::observe`].
    pub observe_addr: Option<SocketAddr>,
    /// The public audience / URL this server answers for. Presence records are
    /// signed *for* this audience; a mismatch is refused (defeats cross-server
    /// replay).
    pub audience: String,
    /// Presence lease TTL the server enforces (clamped to the protocol bounds).
    pub presence_ttl_ms: u64,
    /// Maximum number of simultaneously registered nodes.
    pub max_nodes: usize,
    /// Maximum number of live relay circuits.
    pub max_circuits: usize,
    /// Maximum circuits a single node may hold open at once.
    pub max_circuits_per_node: usize,
    /// Maximum single frame body (bytes). Oversized frames are refused.
    pub max_frame_bytes: usize,
    /// Maximum simultaneous TCP connections.
    pub max_connections: usize,
    /// Maximum simultaneous TCP connections from one IP address.
    pub max_connections_per_ip: usize,
    /// How long a new connection may stay silent/unregistered before it is
    /// dropped.
    pub registration_timeout_ms: u64,
    /// How long a registered connection may stay silent.
    pub idle_timeout_ms: u64,
    /// How long a single write to a peer may take before that peer is dropped.
    pub write_timeout_ms: u64,
    /// How long an opened circuit may wait for its target to accept.
    pub pending_circuit_ttl_ms: u64,
}

impl ServerConfig {
    /// Also answer endpoint-observation probes on `addr`.
    ///
    /// This is what lets a node behind a NAT learn the public UDP address it
    /// should advertise as a hole-punching candidate. It is a separate UDP
    /// socket because the control channel is TCP and TCP's observed source
    /// says nothing about the UDP mapping a QUIC endpoint will use.
    pub fn with_observe_addr(mut self, addr: SocketAddr) -> Self {
        self.observe_addr = Some(addr);
        self
    }

    /// A config for `audience` binding on a given listen address.
    pub fn new(listen: SocketAddr, audience: impl Into<String>) -> Self {
        Self {
            listen,
            observe_addr: None,
            audience: audience.into(),
            presence_ttl_ms: apeireth_mesh_protocol::rendezvous::PRESENCE_DEFAULT_TTL_MS,
            max_nodes: 1024,
            max_circuits: 4096,
            max_circuits_per_node: 64,
            max_frame_bytes: apeireth_mesh_protocol::rendezvous::MAX_FRAME_BODY_BYTES,
            max_connections: 4096,
            max_connections_per_ip: 32,
            registration_timeout_ms: 10_000,
            idle_timeout_ms: 300_000,
            write_timeout_ms: 5_000,
            pending_circuit_ttl_ms: 30_000,
        }
    }
}

/// Why the server refused a request (surfaced as a `Rejected`/`Refused`).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    /// The server socket failed.
    #[error("rendezvous server i/o: {0}")]
    Io(String),
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A live connection to one node. Owns its write half and, once registered, its
/// claimed node id.
struct Conn {
    write: AsyncMutex<tokio::net::tcp::OwnedWriteHalf>,
    node_id: Mutex<Option<NodeId>>,
    write_timeout: Duration,
    /// Set when a write stalled or failed; the read loop then drops the link.
    dead: tokio::sync::watch::Sender<bool>,
}

impl Conn {
    async fn write_all(&self, bytes: &[u8]) {
        let mut write = self.write.lock().await;
        let written = tokio::time::timeout(self.write_timeout, write.write_all(bytes)).await;
        if !matches!(written, Ok(Ok(()))) {
            self.dead.send_replace(true);
        }
    }

    async fn send_control(&self, response: &RendezvousResponse) {
        if let Ok(json) = serde_json::to_vec(response) {
            self.write_all(&encode_control(&json)).await;
        }
    }

    async fn send_relay_control(&self, control: &RelayControl) {
        if let Ok(json) = serde_json::to_vec(control) {
            self.write_all(&encode_control(&json)).await;
        }
    }

    async fn send_relay_data(&self, frame: &RelayFrame) {
        self.write_all(&encode_relay(frame)).await;
    }

    fn registered_as(&self) -> Option<NodeId> {
        self.node_id.lock().unwrap().clone()
    }
}

/// One live relay circuit: the two connection ends being forwarded between.
struct Circuit {
    a: Arc<Conn>,
    b: Arc<Conn>,
    /// The node that opened it, for per-node accounting.
    opener: NodeId,
}

impl Circuit {
    fn has_endpoint(&self, conn: &Arc<Conn>) -> bool {
        Arc::ptr_eq(&self.a, conn) || Arc::ptr_eq(&self.b, conn)
    }
}

/// An open circuit awaiting the target's acceptance.
struct PendingCircuit {
    opener: Arc<Conn>,
    target: Arc<Conn>,
    source: NodeId,
    target_node: NodeId,
    opened_at: Instant,
}

/// Shared server state.
struct State {
    config: ServerConfig,
    /// Registered nodes: id → (live connection, current presence).
    nodes: Mutex<BTreeMap<NodeId, (Arc<Conn>, PresenceRecord)>>,
    /// Live circuits by id.
    circuits: Mutex<HashMap<RelayCircuitId, Circuit>>,
    /// Circuits opened but not yet accepted.
    pending: Mutex<HashMap<RelayCircuitId, PendingCircuit>>,
    /// Open TCP connections per remote IP.
    connections: Mutex<HashMap<IpAddr, usize>>,
}

impl State {
    fn circuit_count(&self) -> usize {
        self.circuits.lock().unwrap().len() + self.pending.lock().unwrap().len()
    }

    /// Circuits (live or pending) attributable to `node`. Derived from the
    /// circuit tables themselves, so it can never drift from reality.
    fn circuits_of(&self, node: &NodeId) -> usize {
        let live = self
            .circuits
            .lock()
            .unwrap()
            .values()
            .filter(|circuit| &circuit.opener == node)
            .count();
        let pending = self
            .pending
            .lock()
            .unwrap()
            .values()
            .filter(|circuit| &circuit.source == node)
            .count();
        live + pending
    }

    fn prune_expired(&self, now: u64) {
        self.nodes
            .lock()
            .unwrap()
            .retain(|_, (_, presence)| presence.expires_at_ms > now);
        let ttl = Duration::from_millis(self.config.pending_circuit_ttl_ms);
        self.pending
            .lock()
            .unwrap()
            .retain(|_, pending| pending.opened_at.elapsed() < ttl);
    }

    /// Admit one more connection from `ip`, or refuse.
    fn admit(self: &Arc<Self>, ip: IpAddr) -> Option<ConnectionSlot> {
        let mut connections = self.connections.lock().unwrap();
        let total: usize = connections.values().sum();
        let from_ip = connections.get(&ip).copied().unwrap_or(0);
        if total >= self.config.max_connections || from_ip >= self.config.max_connections_per_ip {
            return None;
        }
        *connections.entry(ip).or_insert(0) += 1;
        Some(ConnectionSlot {
            state: Arc::clone(self),
            ip,
        })
    }
}

/// Releases a connection's admission when it ends.
struct ConnectionSlot {
    state: Arc<State>,
    ip: IpAddr,
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        let mut connections = self.state.connections.lock().unwrap();
        if let Some(count) = connections.get_mut(&self.ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                connections.remove(&self.ip);
            }
        }
    }
}

/// The rendezvous + relay server.
pub struct RendezvousRelayServer {
    listener: TcpListener,
    observer: Option<Arc<UdpSocket>>,
    state: Arc<State>,
    local: SocketAddr,
}

impl RendezvousRelayServer {
    /// Bind the configured listen address.
    pub async fn bind(config: ServerConfig) -> Result<Self, ServerError> {
        let listener = TcpListener::bind(config.listen)
            .await
            .map_err(|e| ServerError::Io(format!("bind {}: {e}", config.listen)))?;
        let local = listener
            .local_addr()
            .map_err(|e| ServerError::Io(e.to_string()))?;
        let observer = match config.observe_addr {
            Some(addr) => {
                Some(Arc::new(UdpSocket::bind(addr).await.map_err(|e| {
                    ServerError::Io(format!("bind udp {addr}: {e}"))
                })?))
            }
            None => None,
        };
        Ok(Self {
            listener,
            observer,
            state: Arc::new(State {
                config,
                nodes: Mutex::new(BTreeMap::new()),
                circuits: Mutex::new(HashMap::new()),
                pending: Mutex::new(HashMap::new()),
                connections: Mutex::new(HashMap::new()),
            }),
            local,
        })
    }

    /// The bound address (useful when binding port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// The bound endpoint-observation UDP address, when observation is enabled.
    pub async fn local_observe_addr(&self) -> Option<SocketAddr> {
        match &self.observer {
            Some(socket) => socket.local_addr().ok(),
            None => None,
        }
    }

    /// Run the accept loop until the listener is closed. Each connection is
    /// handled on its own task.
    pub async fn run(self: Arc<Self>) -> Result<(), ServerError> {
        if let Some(observer) = self.observer.clone() {
            tokio::spawn(async move { observe_loop(observer).await });
        }
        loop {
            let (stream, peer) = self
                .listener
                .accept()
                .await
                .map_err(|e| ServerError::Io(format!("accept: {e}")))?;
            let Some(slot) = self.state.admit(peer.ip()) else {
                // Over a connection limit: shed rather than queue.
                drop(stream);
                continue;
            };
            let state = Arc::clone(&self.state);
            tokio::spawn(async move {
                let _ = handle_conn(stream, state).await;
                drop(slot);
            });
        }
    }
}

/// Answer endpoint-observation probes until the socket fails repeatedly.
///
/// Deliberately dumb and stateless: no identity, no registration, no memory
/// between datagrams. It echoes the *observed* source address back to whoever
/// sent a well-formed probe — exactly the "what address does the outside world
/// see me at?" question a node behind a NAT must answer before it can advertise
/// a dialable candidate.
///
/// What a client learns here is a hint and nothing more. It never grants trust,
/// and it cannot add, remove or alter a pin: the address still has to be proven
/// by the pinned QUIC handshake before a link exists.
///
/// Malformed datagrams are never answered, and a reply is never larger than the
/// probe that provoked it, so the socket is not a usable reflector.
async fn observe_loop(socket: Arc<UdpSocket>) {
    let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
    let mut consecutive_errors = 0u32;
    loop {
        let (n, from) = match socket.recv_from(&mut buf).await {
            Ok(pair) => {
                consecutive_errors = 0;
                pair
            }
            // A failing socket must not become a hot loop, but one transient
            // error must not end observation for the whole server either.
            Err(_) => {
                consecutive_errors += 1;
                if consecutive_errors >= 32 {
                    return;
                }
                tokio::task::yield_now().await;
                continue;
            }
        };
        let Some(nonce) = decode_probe(&buf[..n]) else {
            continue;
        };
        let reply = encode_reply(&nonce, from);
        // Best effort: a dropped reply is the sender's problem, not the loop's.
        let _ = socket.send_to(&reply, from).await;
    }
}

async fn handle_conn(stream: TcpStream, state: Arc<State>) -> Result<(), ServerError> {
    let (mut read, write) = stream.into_split();
    let conn = Arc::new(Conn {
        write: AsyncMutex::new(write),
        node_id: Mutex::new(None),
        write_timeout: Duration::from_millis(state.config.write_timeout_ms),
        dead: tokio::sync::watch::channel(false).0,
    });
    let mut dead = conn.dead.subscribe();
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 8192];
    'connection: loop {
        // A connection that never registers gets a short leash; a registered
        // one a longer idle window.
        let window = if conn.registered_as().is_some() {
            state.config.idle_timeout_ms
        } else {
            state.config.registration_timeout_ms
        };
        let read_result = tokio::select! {
            result = tokio::time::timeout(Duration::from_millis(window), read.read(&mut chunk)) => result,
            _ = dead.wait_for(|dead| *dead) => break 'connection,
        };
        let n = match read_result {
            Ok(Ok(0) | Err(_)) | Err(_) => break 'connection,
            Ok(Ok(n)) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        loop {
            match decode_frame(&buf) {
                Ok(Some((frame, used))) => {
                    buf.drain(..used);
                    handle_frame(&state, &conn, frame).await;
                }
                Ok(None) => break,
                // Malformed or oversized: drop the connection (and clean up).
                Err(_) => break 'connection,
            }
        }
    }
    cleanup(&state, &conn);
    Ok(())
}

fn cleanup(state: &State, conn: &Arc<Conn>) {
    // Drop any presence this connection registered.
    let node = conn.registered_as();
    if let Some(node) = node {
        let mut nodes = state.nodes.lock().unwrap();
        if let Some((existing, _)) = nodes.get(&node) {
            if Arc::ptr_eq(existing, conn) {
                nodes.remove(&node);
            }
        }
    }
    // Close circuits this connection was part of.
    state
        .circuits
        .lock()
        .unwrap()
        .retain(|_, circuit| !circuit.has_endpoint(conn));
    state
        .pending
        .lock()
        .unwrap()
        .retain(|_, p| !Arc::ptr_eq(&p.opener, conn) && !Arc::ptr_eq(&p.target, conn));
}

async fn handle_frame(state: &Arc<State>, conn: &Arc<Conn>, frame: Frame) {
    match frame {
        Frame::Control(json) => {
            if let Ok(request) = serde_json::from_slice::<RendezvousRequest>(&json) {
                handle_rendezvous(state, conn, request).await;
                return;
            }
            if let Ok(control) = serde_json::from_slice::<RelayControl>(&json) {
                handle_relay_control(state, conn, control).await;
            }
        }
        Frame::Relay(frame) => forward_data(state, conn, frame).await,
    }
}

/// Decide whether `presence` may be installed for `conn`. Pure policy over the
/// current table, so every refusal reason is testable and explicit.
fn admit_presence(
    state: &State,
    conn: &Arc<Conn>,
    presence: &PresenceRecord,
    now: u64,
) -> Result<u64, String> {
    if presence.verify_for(now, &state.config.audience).is_err() {
        return Err("invalid presence signature or audience".into());
    }
    if presence.expires_at_ms > now + PRESENCE_MAX_TTL_MS {
        return Err("presence lease is longer than the protocol maximum".into());
    }
    if let Some(bound) = conn.registered_as() {
        if bound != presence.node_id {
            return Err("this connection is already registered as another node".into());
        }
    }
    let mut nodes = state.nodes.lock().unwrap();
    nodes.retain(|_, (_, existing)| existing.expires_at_ms > now);
    match nodes.get(&presence.node_id) {
        Some((holder, existing)) => {
            if existing.public_key != presence.public_key {
                return Err("node id is held by a different key".into());
            }
            let same_connection = Arc::ptr_eq(holder, conn);
            let advances = if same_connection {
                presence.expires_at_ms >= existing.expires_at_ms
            } else {
                presence.expires_at_ms > existing.expires_at_ms
            };
            if !advances {
                return Err("stale or replayed presence".into());
            }
        }
        None if nodes.len() >= state.config.max_nodes => {
            return Err("server node limit reached".into());
        }
        None => {}
    }
    let expires = presence
        .expires_at_ms
        .min(now + state.config.presence_ttl_ms);
    nodes.insert(
        presence.node_id.clone(),
        (Arc::clone(conn), presence.clone()),
    );
    drop(nodes);
    *conn.node_id.lock().unwrap() = Some(presence.node_id.clone());
    Ok(expires)
}

async fn handle_rendezvous(state: &Arc<State>, conn: &Arc<Conn>, request: RendezvousRequest) {
    let now = now_ms();
    match request {
        RendezvousRequest::Register { presence } | RendezvousRequest::Heartbeat { presence } => {
            match admit_presence(state, conn, &presence, now) {
                Ok(expires) => {
                    conn.send_control(&RendezvousResponse::Registered {
                        expires_at_ms: expires,
                    })
                    .await
                }
                Err(reason) => {
                    conn.send_control(&RendezvousResponse::Rejected { reason })
                        .await
                }
            }
        }
        RendezvousRequest::Lookup { target, .. } => {
            let presence = {
                let mut nodes = state.nodes.lock().unwrap();
                nodes.retain(|_, (_, existing)| existing.expires_at_ms > now);
                nodes.get(&target).map(|(_, p)| p.clone())
            };
            conn.send_control(&RendezvousResponse::Presence { presence })
                .await;
        }
        RendezvousRequest::Withdraw {
            node_id,
            session_id,
        } => {
            {
                let mut nodes = state.nodes.lock().unwrap();
                // Only the connection that holds the registration may end it.
                let owned = nodes
                    .get(&node_id)
                    .map(|(holder, p)| Arc::ptr_eq(holder, conn) && p.session_id == session_id)
                    .unwrap_or(false);
                if owned {
                    nodes.remove(&node_id);
                }
            }
            conn.send_control(&RendezvousResponse::Registered { expires_at_ms: now })
                .await;
        }
    }
}

/// The presence `node` currently holds on this server, if live.
fn registered_presence(state: &State, node: &NodeId, now: u64) -> Option<PresenceRecord> {
    state
        .nodes
        .lock()
        .unwrap()
        .get(node)
        .filter(|(_, presence)| presence.expires_at_ms > now)
        .map(|(_, presence)| presence.clone())
}

async fn refuse(conn: &Arc<Conn>, circuit_id: RelayCircuitId, reason: &str) {
    conn.send_relay_control(&RelayControl::Refused {
        circuit_id: Some(circuit_id),
        reason: reason.into(),
    })
    .await;
}

async fn handle_relay_control(state: &Arc<State>, conn: &Arc<Conn>, control: RelayControl) {
    let now = now_ms();
    match control {
        RelayControl::Open {
            circuit_id,
            source,
            target,
            source_proof,
        } => {
            // Abuse gate: the opener proves key possession *as the node this
            // connection is registered as*. Trust is NOT decided here — the two
            // nodes authenticate each other end-to-end.
            if source_proof.node_id != source
                || source_proof
                    .verify_for(now, &state.config.audience)
                    .is_err()
            {
                refuse(conn, circuit_id, "invalid source proof").await;
                return;
            }
            let registered = registered_presence(state, &source, now);
            let bound = conn.registered_as().as_ref() == Some(&source);
            if !bound || registered.is_none_or(|p| p.public_key != source_proof.public_key) {
                refuse(
                    conn,
                    circuit_id,
                    "source is not registered on this connection",
                )
                .await;
                return;
            }
            if source == target {
                refuse(conn, circuit_id, "a node cannot open a circuit to itself").await;
                return;
            }
            state.prune_expired(now);
            if state.circuits_of(&source) >= state.config.max_circuits_per_node {
                refuse(conn, circuit_id, "per-node circuit limit").await;
                return;
            }
            if state.circuit_count() >= state.config.max_circuits {
                refuse(conn, circuit_id, "server circuit limit").await;
                return;
            }
            if state.circuits.lock().unwrap().contains_key(&circuit_id)
                || state.pending.lock().unwrap().contains_key(&circuit_id)
            {
                refuse(conn, circuit_id, "circuit id already in use").await;
                return;
            }
            let target_conn = state
                .nodes
                .lock()
                .unwrap()
                .get(&target)
                .map(|(c, _)| Arc::clone(c));
            let Some(target_conn) = target_conn else {
                refuse(conn, circuit_id, &format!("target {target} is offline")).await;
                return;
            };
            state.pending.lock().unwrap().insert(
                circuit_id,
                PendingCircuit {
                    opener: Arc::clone(conn),
                    target: Arc::clone(&target_conn),
                    source: source.clone(),
                    target_node: target.clone(),
                    opened_at: Instant::now(),
                },
            );
            target_conn
                .send_relay_control(&RelayControl::Open {
                    circuit_id,
                    source,
                    target,
                    source_proof,
                })
                .await;
        }
        RelayControl::Accept {
            circuit_id,
            target_proof,
        } => {
            let pending = {
                let mut pending = state.pending.lock().unwrap();
                match pending.get(&circuit_id) {
                    // Only the addressed target may accept.
                    Some(candidate) if Arc::ptr_eq(&candidate.target, conn) => {
                        pending.remove(&circuit_id)
                    }
                    _ => None,
                }
            };
            let Some(pending) = pending else {
                refuse(conn, circuit_id, "no such pending circuit").await;
                return;
            };
            let proven = target_proof.node_id == pending.target_node
                && target_proof.verify_for(now, &state.config.audience).is_ok()
                && registered_presence(state, &pending.target_node, now)
                    .is_some_and(|p| p.public_key == target_proof.public_key);
            if !proven {
                refuse(conn, circuit_id, "invalid target proof").await;
                refuse(&pending.opener, circuit_id, "target proof was invalid").await;
                return;
            }
            state.circuits.lock().unwrap().insert(
                circuit_id,
                Circuit {
                    a: Arc::clone(&pending.opener),
                    b: Arc::clone(&pending.target),
                    opener: pending.source.clone(),
                },
            );
            pending
                .opener
                .send_relay_control(&RelayControl::Accept {
                    circuit_id,
                    target_proof,
                })
                .await;
        }
        RelayControl::Close { circuit_id, .. } => {
            let circuit = {
                let mut circuits = state.circuits.lock().unwrap();
                // Only an endpoint may close its own circuit.
                if circuits
                    .get(&circuit_id)
                    .is_some_and(|c| c.has_endpoint(conn))
                {
                    circuits.remove(&circuit_id)
                } else {
                    None
                }
            };
            if let Some(circuit) = circuit {
                let other = if Arc::ptr_eq(&circuit.a, conn) {
                    &circuit.b
                } else {
                    &circuit.a
                };
                other
                    .send_relay_control(&RelayControl::Close {
                        circuit_id,
                        reason: "peer closed".into(),
                    })
                    .await;
                return;
            }
            let mut pending = state.pending.lock().unwrap();
            if pending
                .get(&circuit_id)
                .is_some_and(|p| Arc::ptr_eq(&p.opener, conn) || Arc::ptr_eq(&p.target, conn))
            {
                pending.remove(&circuit_id);
            }
        }
        RelayControl::Refused { circuit_id, .. } => {
            // The addressed target declining a pending circuit.
            if let Some(circuit_id) = circuit_id {
                let declined = {
                    let mut pending = state.pending.lock().unwrap();
                    if pending
                        .get(&circuit_id)
                        .is_some_and(|p| Arc::ptr_eq(&p.target, conn))
                    {
                        pending.remove(&circuit_id)
                    } else {
                        None
                    }
                };
                if let Some(declined) = declined {
                    refuse(&declined.opener, circuit_id, "target declined").await;
                }
            }
        }
    }
}

async fn forward_data(state: &Arc<State>, conn: &Arc<Conn>, frame: RelayFrame) {
    let destination = {
        let circuits = state.circuits.lock().unwrap();
        circuits.get(&frame.circuit_id).and_then(|circuit| {
            // Only the circuit's own endpoints may put frames on it. Forward to
            // the end that did NOT send this frame; opaque, never inspected.
            if Arc::ptr_eq(&circuit.a, conn) {
                Some(Arc::clone(&circuit.b))
            } else if Arc::ptr_eq(&circuit.b, conn) {
                Some(Arc::clone(&circuit.a))
            } else {
                None
            }
        })
    };
    if let Some(destination) = destination {
        destination.send_relay_data(&frame).await;
    }
}
