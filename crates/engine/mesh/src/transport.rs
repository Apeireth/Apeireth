//! The transport boundary, and the in-memory transport that proves the mesh's
//! semantics without a socket in sight.
//!
//! [`MeshTransport`] / [`MeshConnection`] are deliberately tiny: dial a persistent
//! link, send and receive whole envelopes, close. Everything interesting —
//! handshake, trust, routing, cancellation, heartbeats — lives above this
//! boundary, which is exactly why swapping the in-memory pair for TCP or QUIC
//! later changes nothing about mesh semantics. The trait must never leak into
//! `PluginManager` or any capability implementation.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use apeireth_mesh_protocol::MeshEnvelope;

/// A link failed to carry a message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MeshTransportError {
    /// The link is closed; no further messages can move.
    #[error("mesh link closed: {0}")]
    Closed(String),
    /// The underlying medium failed.
    #[error("mesh link i/o failure: {0}")]
    Io(String),
    /// The outbound queue is full (bounded transport backpressure). Retry
    /// later or shed load — never buffer without limit.
    #[error("mesh link backpressure: {0}")]
    Backpressure(String),
    /// A frame violated the transport contract (oversized, truncated,
    /// malformed). All network input is untrusted; such input is refused,
    /// never panicked on.
    #[error("mesh link protocol violation: {0}")]
    ProtocolViolation(String),
}

/// One persistent connection between exactly two nodes.
///
/// Persistent by design: a mesh call must not open a fresh connection per RPC.
/// Envelopes are whole messages; framing concerns belong to the concrete
/// implementation.
#[async_trait]
pub trait MeshConnection: Send + Sync {
    /// Deliver one envelope to the far end.
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), MeshTransportError>;

    /// Receive the next envelope from the far end. `Closed` once the far end
    /// has closed and every queued envelope has been drained.
    async fn recv(&self) -> Result<MeshEnvelope, MeshTransportError>;

    /// Close this end of the link.
    fn close(&self);

    /// Whether this end considers the link closed.
    fn is_closed(&self) -> bool;

    /// Cryptographic identity evidence from the transport handshake, when the
    /// transport has any (a pinned TLS key fingerprint). The in-memory fabric
    /// has none and returns `None`.
    ///
    /// This is how transport-level proof reaches transport-agnostic policy:
    /// [`crate::node::MeshNode`] compares it against the trust pin and fails
    /// closed on any mismatch, so a right-key-wrong-node peer cannot bind.
    fn peer_fingerprint(&self) -> Option<crate::identity::KeyFingerprint> {
        None
    }

    /// A transport-provided location hint for the far end (for persistence
    /// after pairing). Pure metadata; never an identity.
    fn remote_endpoint_hint(&self) -> Option<String> {
        None
    }
}

/// The server side of a transport: accept inbound links.
///
/// Real networking needs bind/listen/accept in addition to connect; connection
/// semantics stay shared on [`MeshConnection`] while transport mechanics stay
/// isolated behind these two traits.
#[async_trait]
pub trait MeshListener: Send + Sync {
    /// The next fully connected link, or `None` once the listener is closed.
    async fn accept(&self) -> Result<Arc<dyn MeshConnection>, MeshTransportError>;
}

/// How a node obtains persistent links to its peers.
#[async_trait]
pub trait MeshTransport: Send + Sync {
    /// Dial `endpoint` and return a connected link. The caller handshakes over
    /// it; the transport never interprets envelopes.
    async fn dial(&self, endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError>;
}

/// One end of an in-memory link pair.
pub struct InMemoryLink {
    tx: Mutex<Option<UnboundedSender<MeshEnvelope>>>,
    // Tokio mutex: the receiver guard is held across `.await` in `recv`, and
    // the connection loop runs in a spawned task.
    rx: tokio::sync::Mutex<UnboundedReceiver<MeshEnvelope>>,
    closed: Arc<AtomicBool>,
}

/// Create two directly connected in-memory links.
///
/// The foundation of every semantic test: two `MeshNode`s over this pair can
/// exercise handshake, advert, invocation, cancellation, and heartbeat with no
/// sockets, so a failure here is a mesh-semantics failure and never a transport
/// failure.
///
/// Closing either end closes the whole pair, like a socket: there is no
/// half-close subtlety for mesh semantics to accidentally depend on.
pub fn linked_pair() -> (InMemoryLink, InMemoryLink) {
    let (tx_to_a, rx_at_a) = unbounded_channel();
    let (tx_to_b, rx_at_b) = unbounded_channel();
    let closed = Arc::new(AtomicBool::new(false));
    (
        InMemoryLink {
            tx: Mutex::new(Some(tx_to_b)),
            rx: tokio::sync::Mutex::new(rx_at_a),
            closed: Arc::clone(&closed),
        },
        InMemoryLink {
            tx: Mutex::new(Some(tx_to_a)),
            rx: tokio::sync::Mutex::new(rx_at_b),
            closed,
        },
    )
}

#[async_trait]
impl MeshConnection for InMemoryLink {
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), MeshTransportError> {
        if self.is_closed() {
            return Err(MeshTransportError::Closed("link closed".into()));
        }
        let tx = self.tx.lock().expect("in-memory link tx poisoned");
        match tx.as_ref() {
            Some(tx) => tx
                .send(envelope)
                .map_err(|_| MeshTransportError::Closed("peer receiver gone".into())),
            None => Err(MeshTransportError::Closed("link closed".into())),
        }
    }

    async fn recv(&self) -> Result<MeshEnvelope, MeshTransportError> {
        if self.is_closed() {
            return Err(MeshTransportError::Closed("link closed".into()));
        }
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(envelope) => Ok(envelope),
            None => {
                self.closed.store(true, Ordering::SeqCst);
                Err(MeshTransportError::Closed("peer sender gone".into()))
            }
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.tx.lock().expect("in-memory link tx poisoned").take();
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

/// A named rendezvous point for in-memory dialing.
///
/// Gives the in-memory transport the same dial/listen shape a socket transport
/// has, so moving to TCP later is a substitution and not a redesign.
#[derive(Default)]
pub struct InMemoryMeshNetwork {
    listeners: Mutex<BTreeMap<String, UnboundedSender<Arc<InMemoryLink>>>>,
}

/// The receiving half of [`InMemoryMeshNetwork::listen`].
pub struct InMemoryListener {
    rx: UnboundedReceiver<Arc<InMemoryLink>>,
}

impl InMemoryListener {
    /// The next inbound link, or `None` when the network (and every dialed
    /// link) is gone.
    pub async fn accept(&mut self) -> Option<Arc<InMemoryLink>> {
        self.rx.recv().await
    }
}

impl InMemoryMeshNetwork {
    /// An empty in-memory network.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register `endpoint` and receive the server ends of future dials.
    pub fn listen(self: &Arc<Self>, endpoint: &str) -> InMemoryListener {
        let (tx, rx) = unbounded_channel();
        self.listeners
            .lock()
            .expect("in-memory mesh network poisoned")
            .insert(endpoint.to_string(), tx);
        InMemoryListener { rx }
    }

    /// Create a link pair, delivering the far end to `endpoint`'s listener.
    fn establish(&self, endpoint: &str) -> Result<InMemoryLink, MeshTransportError> {
        let (client, server) = linked_pair();
        let listener = self
            .listeners
            .lock()
            .expect("in-memory mesh network poisoned")
            .get(endpoint)
            .cloned()
            .ok_or_else(|| MeshTransportError::Io(format!("nothing is listening on {endpoint}")))?;
        listener
            .send(Arc::new(server))
            .map_err(|_| MeshTransportError::Closed(format!("listener on {endpoint} is gone")))?;
        Ok(client)
    }
}

/// An in-memory [`MeshTransport`] bound to an [`InMemoryMeshNetwork`].
pub struct InMemoryMeshTransport {
    network: Arc<InMemoryMeshNetwork>,
}

impl InMemoryMeshTransport {
    /// A transport over the given in-memory network.
    pub fn new(network: Arc<InMemoryMeshNetwork>) -> Self {
        Self { network }
    }
}

#[async_trait]
impl MeshTransport for InMemoryMeshTransport {
    async fn dial(&self, endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        let client = self.network.establish(endpoint)?;
        Ok(Arc::new(client) as Arc<dyn MeshConnection>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{NodeId, TraceId};
    use apeireth_mesh_protocol::{Hello, MeshPayload, NodeDescriptor, MESH_PROTOCOL_VERSION};

    fn hello(from: &str) -> MeshEnvelope {
        MeshEnvelope::send(
            NodeId::new(from).unwrap(),
            NodeId::new("node-b").unwrap(),
            TraceId::new(),
            MeshPayload::Hello(Hello {
                node: NodeDescriptor::new(
                    NodeId::new(from).unwrap(),
                    from,
                    MESH_PROTOCOL_VERSION,
                    "2.0.0-rc.1",
                ),
            }),
        )
    }

    #[tokio::test]
    async fn a_link_pair_carries_envelopes_both_ways() {
        let (a, b) = linked_pair();
        a.send(hello("node-a")).await.unwrap();
        b.send(hello("node-b")).await.unwrap();
        assert_eq!(b.recv().await.unwrap().source_node.as_str(), "node-a");
        assert_eq!(a.recv().await.unwrap().source_node.as_str(), "node-b");
    }

    #[tokio::test]
    async fn closing_one_end_closes_the_pair() {
        let (a, b) = linked_pair();
        a.close();
        assert!(a.is_closed());
        assert!(b.is_closed());
        assert!(b.recv().await.is_err());
        assert!(b.send(hello("node-b")).await.is_err());
    }

    #[tokio::test]
    async fn the_transport_dials_a_listening_endpoint() {
        let network = InMemoryMeshNetwork::new();
        let mut listener = network.listen("memory://node-b");
        let transport = InMemoryMeshTransport::new(Arc::clone(&network));

        let client = transport.dial("memory://node-b").await.unwrap();
        let server = listener.accept().await.unwrap();

        client.send(hello("node-a")).await.unwrap();
        let got = server.recv().await.unwrap();
        assert_eq!(got.source_node.as_str(), "node-a");
    }
}
