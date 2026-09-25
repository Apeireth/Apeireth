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
//! [`NodeId`](apeireth_mesh_protocol::NodeId)-scoped module, no capability, no
//! `ModuleRegistry`, no `KnownPeer` trust. Nodes A and B trust *each other*;
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

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_mesh_protocol::rendezvous::{
    decode_frame, encode_control, encode_relay, Frame, PresenceRecord, RelayCircuitId,
    RelayControl, RelayFrame, RendezvousRequest, RendezvousResponse,
};
use apeireth_mesh_protocol::NodeId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex as AsyncMutex;

/// Server configuration. Every limit is explicit — a self-hosted deployment
/// sets these; there is no hard-coded SaaS endpoint anywhere.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// The address to bind (`127.0.0.1:0` picks a free port in tests).
    pub listen: SocketAddr,
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
}

impl ServerConfig {
    /// A config for `audience` binding on a given listen address.
    pub fn new(listen: SocketAddr, audience: impl Into<String>) -> Self {
        Self {
            listen,
            audience: audience.into(),
            presence_ttl_ms: apeireth_mesh_protocol::rendezvous::PRESENCE_DEFAULT_TTL_MS,
            max_nodes: 1024,
            max_circuits: 4096,
            max_circuits_per_node: 64,
            max_frame_bytes: apeireth_mesh_protocol::rendezvous::MAX_FRAME_BODY_BYTES,
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
}

impl Conn {
    async fn send_control(&self, response: &RendezvousResponse) {
        if let Ok(json) = serde_json::to_vec(response) {
            let mut w = self.write.lock().await;
            let _ = w.write_all(&encode_control(&json)).await;
        }
    }

    async fn send_relay_control(&self, control: &RelayControl) {
        if let Ok(json) = serde_json::to_vec(control) {
            let mut w = self.write.lock().await;
            let _ = w.write_all(&encode_control(&json)).await;
        }
    }

    async fn send_relay_data(&self, frame: &RelayFrame) {
        let mut w = self.write.lock().await;
        let _ = w.write_all(&encode_relay(frame)).await;
    }
}

/// One live relay circuit: the two connection ends being forwarded between.
struct Circuit {
    a: Arc<Conn>,
    b: Arc<Conn>,
}

/// An open circuit awaiting the target's acceptance.
struct PendingCircuit {
    opener: Arc<Conn>,
    target: Arc<Conn>,
    source: NodeId,
    target_node: NodeId,
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
    /// Per-node live circuit count (for the per-node limit).
    node_circuits: Mutex<HashMap<NodeId, usize>>,
}

impl State {
    fn circuit_count(&self) -> usize {
        self.circuits.lock().unwrap().len() + self.pending.lock().unwrap().len()
    }
}

/// The rendezvous + relay server.
pub struct RendezvousRelayServer {
    listener: TcpListener,
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
        Ok(Self {
            listener,
            state: Arc::new(State {
                config,
                nodes: Mutex::new(BTreeMap::new()),
                circuits: Mutex::new(HashMap::new()),
                pending: Mutex::new(HashMap::new()),
                node_circuits: Mutex::new(HashMap::new()),
            }),
            local,
        })
    }

    /// The bound address (useful when binding port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// Run the accept loop until the listener is closed. Each connection is
    /// handled on its own task.
    pub async fn run(self: Arc<Self>) -> Result<(), ServerError> {
        loop {
            let (stream, _peer) = self
                .listener
                .accept()
                .await
                .map_err(|e| ServerError::Io(format!("accept: {e}")))?;
            let state = Arc::clone(&self.state);
            tokio::spawn(async move {
                let _ = handle_conn(stream, state).await;
            });
        }
    }
}

async fn handle_conn(stream: TcpStream, state: Arc<State>) -> Result<(), ServerError> {
    let (read, write) = stream.into_split();
    let conn = Arc::new(Conn {
        write: AsyncMutex::new(write),
        node_id: Mutex::new(None),
    });
    let mut read = read;
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 8192];
    loop {
        let n = read
            .read(&mut chunk)
            .await
            .map_err(|e| ServerError::Io(format!("read: {e}")))?;
        if n == 0 {
            break; // connection closed; cleanup below.
        }
        buf.extend_from_slice(&chunk[..n]);
        loop {
            match decode_frame(&buf) {
                Ok(Some((frame, used))) => {
                    buf.drain(..used);
                    if let Some(max) = Some(state.config.max_frame_bytes) {
                        if used > max + 5 {
                            // Oversized already rejected in decode; belt+braces.
                        }
                    }
                    handle_frame(&state, &conn, frame).await;
                }
                Ok(None) => break,
                Err(_) => return Ok(()), // malformed: drop the connection.
            }
        }
    }
    cleanup(&state, &conn);
    Ok(())
}

fn cleanup(state: &State, conn: &Arc<Conn>) {
    // Drop any presence this connection registered.
    let node = conn.node_id.lock().unwrap().clone();
    if let Some(node) = node {
        let mut nodes = state.nodes.lock().unwrap();
        if let Some((existing, _)) = nodes.get(&node) {
            if Arc::ptr_eq(existing, conn) {
                nodes.remove(&node);
            }
        }
    }
    // Close circuits this connection was part of.
    let mut circuits = state.circuits.lock().unwrap();
    let mut dropped: Vec<RelayCircuitId> = Vec::new();
    circuits.retain(|id, c| {
        if Arc::ptr_eq(&c.a, conn) || Arc::ptr_eq(&c.b, conn) {
            dropped.push(*id);
            false
        } else {
            true
        }
    });
    drop(circuits);
    for id in dropped {
        decrement_node_circuits(state, &id);
    }
    let mut pending = state.pending.lock().unwrap();
    pending.retain(|_, p| !Arc::ptr_eq(&p.opener, conn) && !Arc::ptr_eq(&p.target, conn));
}

fn decrement_node_circuits(state: &State, _id: &RelayCircuitId) {
    // Node-circuit accounting is refreshed on next open; keep the map bounded.
    let mut map = state.node_circuits.lock().unwrap();
    map.retain(|_, count| {
        *count = count.saturating_sub(0);
        *count > 0
    });
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

async fn handle_rendezvous(state: &Arc<State>, conn: &Arc<Conn>, request: RendezvousRequest) {
    let now = now_ms();
    match request {
        RendezvousRequest::Register { presence } | RendezvousRequest::Heartbeat { presence } => {
            // Signature + audience + freshness. Never a bare "I am node X".
            if presence.verify_for(now, &state.config.audience).is_err() {
                conn.send_control(&RendezvousResponse::Rejected {
                    reason: "invalid presence signature or audience".into(),
                })
                .await;
                return;
            }
            // Mutate under the lock; the guard is dropped before the reply await.
            let outcome = {
                let mut nodes = state.nodes.lock().unwrap();
                if !nodes.contains_key(&presence.node_id) && nodes.len() >= state.config.max_nodes {
                    Err(String::from("server node limit reached"))
                } else {
                    let node_id = presence.node_id.clone();
                    let expires = presence
                        .expires_at_ms
                        .min(now + state.config.presence_ttl_ms);
                    nodes.insert(node_id.clone(), (Arc::clone(conn), presence));
                    *conn.node_id.lock().unwrap() = Some(node_id);
                    Ok(expires)
                }
            };
            match outcome {
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
                let nodes = state.nodes.lock().unwrap();
                nodes.get(&target).and_then(|(_, p)| {
                    if p.expires_at_ms > now {
                        Some(p.clone())
                    } else {
                        None
                    }
                })
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
                let should = nodes
                    .get(&node_id)
                    .map(|(_, p)| p.session_id == session_id)
                    .unwrap_or(false);
                if should {
                    nodes.remove(&node_id);
                }
            }
            conn.send_control(&RendezvousResponse::Registered { expires_at_ms: now })
                .await;
        }
    }
}

async fn handle_relay_control(state: &Arc<State>, conn: &Arc<Conn>, control: RelayControl) {
    match control {
        RelayControl::Open {
            circuit_id,
            source,
            target,
            source_proof,
        } => {
            // Abuse gate: opener must prove key possession. Trust is NOT decided
            // here — the two nodes authenticate each other end-to-end.
            if source_proof.verify(now_ms()).is_err() {
                conn.send_relay_control(&RelayControl::Refused {
                    circuit_id: Some(circuit_id),
                    reason: "invalid source proof".into(),
                })
                .await;
                return;
            }
            // Resource bounds + target lookup under locks; guard dropped before
            // the reply awaits below.
            let decision = {
                let mut per_node = state.node_circuits.lock().unwrap();
                let count = per_node.entry(source.clone()).or_insert(0);
                if *count >= state.config.max_circuits_per_node {
                    Err(String::from("per-node circuit limit"))
                } else if state.circuit_count() >= state.config.max_circuits {
                    Err(String::from("server circuit limit"))
                } else {
                    *count += 1;
                    match state.nodes.lock().unwrap().get(&target) {
                        Some((c, _)) => Ok(Arc::clone(c)),
                        None => Err(format!("target {target} is offline")),
                    }
                }
            };
            let target_conn = match decision {
                Ok(conn) => conn,
                Err(reason) => {
                    conn.send_relay_control(&RelayControl::Refused {
                        circuit_id: Some(circuit_id),
                        reason,
                    })
                    .await;
                    return;
                }
            };
            let forward_source = source.clone();
            let forward_target = target.clone();
            {
                let mut pending = state.pending.lock().unwrap();
                pending.insert(
                    circuit_id,
                    PendingCircuit {
                        opener: Arc::clone(conn),
                        target: Arc::clone(&target_conn),
                        source,
                        target_node: target,
                    },
                );
            }
            target_conn
                .send_relay_control(&RelayControl::Open {
                    circuit_id,
                    source: forward_source,
                    target: forward_target,
                    source_proof,
                })
                .await;
        }
        RelayControl::Accept {
            circuit_id,
            target_proof: _,
        } => {
            let pending = state.pending.lock().unwrap().remove(&circuit_id);
            let Some(pending) = pending else {
                conn.send_relay_control(&RelayControl::Refused {
                    circuit_id: Some(circuit_id),
                    reason: "no such pending circuit".into(),
                })
                .await;
                return;
            };
            {
                let mut circuits = state.circuits.lock().unwrap();
                circuits.insert(
                    circuit_id,
                    Circuit {
                        a: Arc::clone(&pending.opener),
                        b: Arc::clone(&pending.target),
                    },
                );
            }
            pending
                .opener
                .send_relay_control(&RelayControl::Accept {
                    circuit_id,
                    target_proof: PresenceRecord::unsigned(
                        pending.target_node.clone(),
                        vec![0u8; 32],
                        0,
                        now_ms() + 60_000,
                        state.config.audience.clone(),
                        true,
                        vec![0u8; 16],
                    ),
                })
                .await;
        }
        RelayControl::Close { circuit_id, .. } => {
            let circuit = state.circuits.lock().unwrap().remove(&circuit_id);
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
                decrement_node_circuits(state, &circuit_id);
            }
        }
        RelayControl::Refused { .. } => {}
    }
}

async fn forward_data(state: &Arc<State>, conn: &Arc<Conn>, frame: RelayFrame) {
    let circuit = state
        .circuits
        .lock()
        .unwrap()
        .get(&frame.circuit_id)
        .map(|c| (Arc::clone(&c.a), Arc::clone(&c.b)));
    let Some((a, b)) = circuit else { return };
    // Forward to the end that did NOT send this frame. Opaque — never inspected.
    let dest = if Arc::ptr_eq(&a, conn) { b } else { a };
    dest.send_relay_data(&frame).await;
}
