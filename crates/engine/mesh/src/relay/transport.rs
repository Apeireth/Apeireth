//! The client side of WAN rendezvous + relay: one node's connection to a
//! rendezvous/relay server, and the transports it exposes to the mesh.
//!
//! A node keeps a single **outbound** stream to the server (the target scenario
//! is two nodes behind NAT, both dialing out). That one stream carries three
//! things, tagged so the node demuxes them:
//!
//! - **rendezvous control** — register / heartbeat / lookup;
//! - **relay control** — open / accept / close a circuit;
//! - **relay data** — opaque end-to-end ciphertext frames ([`RelayFrame`]).
//!
//! The server sees only routing metadata (circuit ids) and opaque bytes. Once a
//! circuit is up, the two nodes run the E2E [`SecureChannel`](super::e2e::SecureChannel)
//! over it and present a [`RelayMeshConnection`](super::connection::RelayMeshConnection)
//! to the mesh — so the runtime cannot tell relay from direct.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_mesh_protocol::rendezvous::{
    decode_frame, encode_control, encode_relay, Frame, PresenceRecord, RelayCircuitId,
    RelayControl, RelayFrame, RendezvousRequest, RendezvousResponse,
};
use apeireth_mesh_protocol::NodeId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::identity::NodeIdentity;
use crate::transport::{MeshConnection, MeshListener, MeshTransport, MeshTransportError};

use super::connection::RelayMeshConnection;
use super::e2e::{handshake, HandshakeRole, OpaqueTunnel};
use super::presence::{now_ms, signed_presence};

/// Errors from the rendezvous/relay client surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RelayError {
    /// The link to the server failed or the request was refused at transport.
    #[error("relay transport error: {0}")]
    Transport(String),
    /// The server answered with a rejection.
    #[error("rendezvous rejected the request: {0}")]
    Rejected(String),
    /// No presence / circuit could be established for the target.
    #[error("target {0} is not reachable")]
    Unreachable(String),
    /// The end-to-end handshake failed (identity mismatch or MITM).
    #[error("end-to-end channel failed: {0}")]
    Channel(String),
}

impl From<MeshTransportError> for RelayError {
    fn from(e: MeshTransportError) -> Self {
        RelayError::Transport(e.to_string())
    }
}

/// Shared state across a node's single server connection.
struct EndpointShared {
    write: tokio::sync::Mutex<OwnedWriteHalf>,
    /// circuit id → inbound data queue for that circuit's local end.
    circuits: Mutex<BTreeMap<RelayCircuitId, mpsc::Sender<Vec<u8>>>>,
    /// Pending rendezvous request/response (register/lookup/heartbeat).
    pending_rpc: Mutex<Option<oneshot::Sender<RendezvousResponse>>>,
    /// Pending circuit-establish notifications, keyed by circuit id. Resolved
    /// to `Ok` when the peer accepts, `Err` when refused.
    pending_circuits: Mutex<BTreeMap<RelayCircuitId, oneshot::Sender<Result<(), String>>>>,
    /// Inbound relay *control* opens (and closes) for the listener to consume.
    inbound_opens: mpsc::Sender<RelayControl>,
    closed: AtomicBool,
}

impl EndpointShared {
    async fn send_control(&self, request: &RendezvousRequest) -> Result<(), MeshTransportError> {
        let json = serde_json::to_vec(request)
            .map_err(|e| MeshTransportError::ProtocolViolation(format!("encode: {e}")))?;
        self.write_frame(&encode_control(&json)).await
    }

    async fn send_relay_control(&self, control: &RelayControl) -> Result<(), MeshTransportError> {
        let json = serde_json::to_vec(control)
            .map_err(|e| MeshTransportError::ProtocolViolation(format!("encode: {e}")))?;
        self.write_frame(&encode_control(&json)).await
    }

    async fn write_frame(&self, bytes: &[u8]) -> Result<(), MeshTransportError> {
        let mut w = self.write.lock().await;
        w.write_all(bytes)
            .await
            .map_err(|e| MeshTransportError::Io(format!("write to rendezvous server: {e}")))
    }
}

/// A node's handle to a rendezvous/relay server: presence plus relay circuits.
pub struct RelayEndpoint {
    shared: Arc<EndpointShared>,
    identity: Arc<NodeIdentity>,
    node_id: NodeId,
    audience: String,
    session_id: u64,
    /// Inbound relay-circuit opens, handed to a listener via
    /// [`RelayEndpoint::take_inbound_opens`].
    inbound_opens_rx: Mutex<Option<mpsc::Receiver<RelayControl>>>,
}

impl RelayEndpoint {
    /// Connect to a rendezvous/relay server over TCP and start the demux loop.
    pub async fn connect(
        addr: &str,
        identity: Arc<NodeIdentity>,
        node_id: NodeId,
        audience: impl Into<String>,
    ) -> Result<Arc<Self>, RelayError> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| RelayError::Transport(format!("connect {addr}: {e}")))?;
        let (read, write) = stream.into_split();
        let (open_tx, open_rx) = mpsc::channel(64);
        let shared = Arc::new(EndpointShared {
            write: tokio::sync::Mutex::new(write),
            circuits: Mutex::new(BTreeMap::new()),
            pending_rpc: Mutex::new(None),
            pending_circuits: Mutex::new(BTreeMap::new()),
            inbound_opens: open_tx,
            closed: AtomicBool::new(false),
        });
        {
            let shared = Arc::clone(&shared);
            tokio::spawn(async move {
                let _ = demux_loop(read, shared).await;
            });
        }
        Ok(Arc::new(Self {
            shared,
            identity,
            node_id,
            audience: audience.into(),
            session_id: random_u64(),
            // hand the open receiver to whoever builds a listener; see
            // `take_inbound_opens`.
            inbound_opens_rx: Mutex::new(Some(open_rx)),
        }))
    }

    /// This endpoint's node id.
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// A fresh signed presence record for this node.
    pub fn signed_presence(&self, ttl_ms: u64, relay_capable: bool) -> PresenceRecord {
        signed_presence(
            &self.identity,
            self.node_id.clone(),
            self.audience.clone(),
            self.session_id,
            ttl_ms,
            relay_capable,
        )
    }

    /// Register (or refresh) presence. Returns the server-confirmed expiry.
    pub async fn register(&self, presence: PresenceRecord) -> Result<u64, RelayError> {
        match self.rpc(RendezvousRequest::Register { presence }).await? {
            RendezvousResponse::Registered { expires_at_ms } => Ok(expires_at_ms),
            RendezvousResponse::Rejected { reason } => Err(RelayError::Rejected(reason)),
            other => Err(RelayError::Transport(format!(
                "unexpected register response: {other:?}"
            ))),
        }
    }

    /// Renew the lease with a freshly signed presence record.
    pub async fn heartbeat(&self, presence: PresenceRecord) -> Result<u64, RelayError> {
        match self.rpc(RendezvousRequest::Heartbeat { presence }).await? {
            RendezvousResponse::Registered { expires_at_ms } => Ok(expires_at_ms),
            RendezvousResponse::Rejected { reason } => Err(RelayError::Rejected(reason)),
            other => Err(RelayError::Transport(format!(
                "unexpected heartbeat response: {other:?}"
            ))),
        }
    }

    /// Look up a target's presence. `None` means absent / unknown / expired.
    pub async fn lookup(&self, target: NodeId) -> Result<Option<PresenceRecord>, RelayError> {
        match self
            .rpc(RendezvousRequest::Lookup {
                querant: self.node_id.clone(),
                target,
            })
            .await?
        {
            RendezvousResponse::Presence { presence } => Ok(presence),
            RendezvousResponse::Rejected { reason } => Err(RelayError::Rejected(reason)),
            other => Err(RelayError::Transport(format!(
                "unexpected lookup response: {other:?}"
            ))),
        }
    }

    /// Take the receiver of inbound relay-circuit opens (to build a listener).
    pub fn take_inbound_opens(&self) -> Option<mpsc::Receiver<RelayControl>> {
        self.inbound_opens_rx.lock().unwrap().take()
    }

    /// Open a relay circuit to `target`, returning an opaque tunnel once the
    /// target accepts. Proof of our key possession rides along.
    pub async fn open_circuit(&self, target: NodeId) -> Result<RelayCircuit, RelayError> {
        let circuit_id = RelayCircuitId::from_random(random_16());
        let (inbound_tx, inbound_rx) = mpsc::channel(64);
        self.shared
            .circuits
            .lock()
            .unwrap()
            .insert(circuit_id, inbound_tx);
        let (establish_tx, establish_rx) = oneshot::channel();
        self.shared
            .pending_circuits
            .lock()
            .unwrap()
            .insert(circuit_id, establish_tx);

        let source_proof = self.signed_presence(60_000, true);
        self.shared
            .send_relay_control(&RelayControl::Open {
                circuit_id,
                source: self.node_id.clone(),
                target,
                source_proof,
            })
            .await?;

        // Wait for the target to accept (demux resolves this), or refuse.
        match establish_rx.await {
            Ok(Ok(())) => Ok(RelayCircuit {
                shared: Arc::clone(&self.shared),
                circuit_id,
                inbound: tokio::sync::Mutex::new(inbound_rx),
                closed: AtomicBool::new(false),
            }),
            Ok(Err(reason)) => Err(RelayError::Rejected(reason)),
            Err(_) => Err(RelayError::Transport(
                "circuit establish channel closed".to_string(),
            )),
        }
    }

    /// One control request/response round-trip (serialized).
    async fn rpc(&self, request: RendezvousRequest) -> Result<RendezvousResponse, RelayError> {
        let (tx, rx) = oneshot::channel();
        {
            let mut slot = self.shared.pending_rpc.lock().unwrap();
            if slot.is_some() {
                return Err(RelayError::Transport(
                    "a control request is already in flight".to_string(),
                ));
            }
            *slot = Some(tx);
        }
        self.shared.send_control(&request).await?;
        rx.await
            .map_err(|_| RelayError::Transport("control response channel closed".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Demux: split the single server stream into control responses, relay-control
// events, and per-circuit data.
// ---------------------------------------------------------------------------

async fn demux_loop(
    mut read: OwnedReadHalf,
    shared: Arc<EndpointShared>,
) -> Result<(), MeshTransportError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = read
            .read(&mut chunk)
            .await
            .map_err(|e| MeshTransportError::Io(format!("read from rendezvous server: {e}")))?;
        if n == 0 {
            shared.closed.store(true, Ordering::SeqCst);
            fail_all(&shared, "server closed the connection");
            return Err(MeshTransportError::Closed(
                "rendezvous server closed the connection".into(),
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
        loop {
            match decode_frame(&buf) {
                Ok(Some((frame, used))) => {
                    buf.drain(..used);
                    handle_frame(&shared, frame).await;
                }
                Ok(None) => break,
                Err(e) => {
                    shared.closed.store(true, Ordering::SeqCst);
                    fail_all(&shared, &e.to_string());
                    return Err(MeshTransportError::ProtocolViolation(e.to_string()));
                }
            }
        }
    }
}

fn fail_all(shared: &EndpointShared, reason: &str) {
    if let Some(tx) = shared.pending_rpc.lock().unwrap().take() {
        let _ = tx.send(RendezvousResponse::Rejected {
            reason: reason.to_string(),
        });
    }
    let pending = std::mem::take(&mut *shared.pending_circuits.lock().unwrap());
    for (_id, tx) in pending {
        let _ = tx.send(Err(reason.to_string()));
    }
}

async fn handle_frame(shared: &Arc<EndpointShared>, frame: Frame) {
    match frame {
        Frame::Control(json) => {
            // Rendezvous response resolves the pending RPC.
            if let Ok(response) = serde_json::from_slice::<RendezvousResponse>(&json) {
                if let Some(tx) = shared.pending_rpc.lock().unwrap().take() {
                    let _ = tx.send(response);
                }
                return;
            }
            let Ok(control) = serde_json::from_slice::<RelayControl>(&json) else {
                return;
            };
            match &control {
                RelayControl::Accept { circuit_id, .. } => {
                    if let Some(tx) = shared.pending_circuits.lock().unwrap().remove(circuit_id) {
                        let _ = tx.send(Ok(()));
                    }
                }
                RelayControl::Refused { circuit_id, reason } => {
                    if let Some(cid) = circuit_id {
                        if let Some(tx) = shared.pending_circuits.lock().unwrap().remove(cid) {
                            let _ = tx.send(Err(reason.clone()));
                        }
                    }
                }
                _ => {}
            }
            // Surface every relay-control event to the listener as well.
            let _ = shared.inbound_opens.send(control).await;
        }
        Frame::Relay(frame) => {
            let tx = shared
                .circuits
                .lock()
                .unwrap()
                .get(&frame.circuit_id)
                .cloned();
            if let Some(tx) = tx {
                let _ = tx.send(frame.payload).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OpaqueTunnel over one relay circuit.
// ---------------------------------------------------------------------------

/// One end of a relay circuit: an opaque byte tunnel between two nodes, carried
/// by the server. Implements [`OpaqueTunnel`] for the E2E channel.
pub struct RelayCircuit {
    shared: Arc<EndpointShared>,
    circuit_id: RelayCircuitId,
    inbound: tokio::sync::Mutex<mpsc::Receiver<Vec<u8>>>,
    closed: AtomicBool,
}

impl RelayCircuit {
    /// The circuit's transport identifier (never an identity).
    pub fn id(&self) -> RelayCircuitId {
        self.circuit_id
    }
}

#[async_trait::async_trait]
impl OpaqueTunnel for RelayCircuit {
    async fn send_opaque(&self, payload: Vec<u8>) -> Result<(), MeshTransportError> {
        if self.is_opaque_closed() {
            return Err(MeshTransportError::Closed("circuit closed".into()));
        }
        self.shared
            .write_frame(&encode_relay(&RelayFrame {
                circuit_id: self.circuit_id,
                payload,
            }))
            .await
    }

    async fn recv_opaque(&self) -> Result<Vec<u8>, MeshTransportError> {
        let mut inbound = self.inbound.lock().await;
        inbound
            .recv()
            .await
            .ok_or_else(|| MeshTransportError::Closed("circuit closed".into()))
    }

    fn close_opaque(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        self.shared
            .circuits
            .lock()
            .unwrap()
            .remove(&self.circuit_id);
        let shared = Arc::clone(&self.shared);
        let circuit_id = self.circuit_id;
        tokio::spawn(async move {
            let _ = shared
                .send_relay_control(&RelayControl::Close {
                    circuit_id,
                    reason: "local close".into(),
                })
                .await;
        });
    }

    fn is_opaque_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) || self.shared.closed.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// MeshTransport / MeshListener over relay.
// ---------------------------------------------------------------------------

/// Dial trusted peers over the relay when a direct connection is unavailable.
///
/// `dial("relay://<node>")` opens a circuit to `<node>`, runs the E2E handshake
/// (proving the peer's `NodeIdentity`), and returns a [`RelayMeshConnection`].
pub struct RelayMeshTransport {
    endpoint: Arc<RelayEndpoint>,
    identity: Arc<NodeIdentity>,
}

impl RelayMeshTransport {
    /// A relay transport bound to a live endpoint and this node's identity.
    pub fn new(endpoint: Arc<RelayEndpoint>, identity: Arc<NodeIdentity>) -> Self {
        Self { endpoint, identity }
    }

    /// Dial `target` over the relay and complete the E2E handshake as initiator.
    pub async fn dial_node(&self, target: NodeId) -> Result<Arc<dyn MeshConnection>, RelayError> {
        // Presence first: the relay is only useful if the target is reachable.
        let _ = self
            .endpoint
            .lookup(target.clone())
            .await?
            .ok_or_else(|| RelayError::Unreachable(target.to_string()))?;
        let peer_node_id = target.to_string();
        let circuit = self.endpoint.open_circuit(target).await?;
        let circuit_id = circuit.id();
        let channel = handshake(
            Box::new(circuit),
            HandshakeRole::Initiator,
            self.identity.pkcs8_der(),
            self.identity.public_key(),
            self.endpoint.node_id().as_str(),
            &peer_node_id,
            &circuit_id,
        )
        .await
        .map_err(|e| RelayError::Channel(e.to_string()))?;
        Ok(Arc::new(RelayMeshConnection::new(channel)) as Arc<dyn MeshConnection>)
    }
}

#[async_trait::async_trait]
impl MeshTransport for RelayMeshTransport {
    async fn dial(&self, endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        let target = endpoint.strip_prefix("relay://").unwrap_or(endpoint);
        let node = NodeId::new(target)
            .map_err(|e| MeshTransportError::ProtocolViolation(format!("bad relay target: {e}")))?;
        self.dial_node(node)
            .await
            .map_err(|e| MeshTransportError::Io(e.to_string()))
    }
}

/// Accept inbound relay circuits: the target side of a relayed connection.
pub struct RelayMeshListener {
    endpoint: Arc<RelayEndpoint>,
    identity: Arc<NodeIdentity>,
    events: tokio::sync::Mutex<mpsc::Receiver<RelayControl>>,
}

impl RelayMeshListener {
    /// Build a listener over the endpoint's inbound-circuit channel.
    pub fn new(
        endpoint: Arc<RelayEndpoint>,
        identity: Arc<NodeIdentity>,
        events: mpsc::Receiver<RelayControl>,
    ) -> Self {
        Self {
            endpoint,
            identity,
            events: tokio::sync::Mutex::new(events),
        }
    }
}

#[async_trait::async_trait]
impl MeshListener for RelayMeshListener {
    async fn accept(&self) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        loop {
            let event = {
                let mut events = self.events.lock().await;
                events.recv().await
            };
            let Some(event) = event else {
                return Err(MeshTransportError::Closed("relay listener closed".into()));
            };
            let RelayControl::Open {
                circuit_id,
                source,
                source_proof,
                ..
            } = event
            else {
                continue;
            };
            // Abuse gate: the opener must prove key possession. Real trust (the
            // pinned fingerprint) is checked by the mesh after the E2E handshake.
            if source_proof.verify(now_ms()).is_err() {
                let _ = self
                    .endpoint
                    .shared
                    .send_relay_control(&RelayControl::Refused {
                        circuit_id: Some(circuit_id),
                        reason: "invalid source proof".into(),
                    })
                    .await;
                continue;
            }
            let (inbound_tx, inbound_rx) = mpsc::channel(64);
            self.endpoint
                .shared
                .circuits
                .lock()
                .unwrap()
                .insert(circuit_id, inbound_tx);
            let target_proof = self.endpoint.signed_presence(60_000, true);
            self.endpoint
                .shared
                .send_relay_control(&RelayControl::Accept {
                    circuit_id,
                    target_proof,
                })
                .await?;
            let circuit = RelayCircuit {
                shared: Arc::clone(&self.endpoint.shared),
                circuit_id,
                inbound: tokio::sync::Mutex::new(inbound_rx),
                closed: AtomicBool::new(false),
            };
            let channel = handshake(
                Box::new(circuit),
                HandshakeRole::Responder,
                self.identity.pkcs8_der(),
                self.identity.public_key(),
                self.endpoint.node_id().as_str(),
                source.as_str(),
                &circuit_id,
            )
            .await
            .map_err(|e| MeshTransportError::Io(e.to_string()))?;
            return Ok(Arc::new(RelayMeshConnection::new(channel)) as Arc<dyn MeshConnection>);
        }
    }
}

// --- helpers ---------------------------------------------------------------

fn random_16() -> [u8; 16] {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut b = [0u8; 16];
    SystemRandom::new().fill(&mut b).expect("rng");
    b
}

fn random_u64() -> u64 {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut b = [0u8; 8];
    SystemRandom::new().fill(&mut b).expect("rng");
    u64::from_be_bytes(b)
}
