//! [`RelayMeshConnection`]: a full [`MeshConnection`] carried over an
//! end-to-end encrypted relay tunnel.
//!
//! This is the piece that makes relay *transparent* to the mesh. It presents
//! exactly the same `MeshConnection` surface a direct QUIC link does — send and
//! receive whole [`MeshEnvelope`]s, close, report a peer fingerprint — so
//! [`crate::node::MeshNode::connect`] / [`accept`](crate::node::MeshNode::accept)
//! run over it **unchanged**. The runtime never learns whether a peer is direct
//! or relayed.
//!
//! Underneath, every envelope is serialized and sealed with the
//! [`SecureChannel`](super::e2e::SecureChannel)'s AEAD key before it enters the
//! tunnel, so the relay only ever forwards ciphertext. The `peer_fingerprint`
//! is the E2E-authenticated `NodeIdentity` key — never the relay's TLS identity.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use apeireth_mesh_protocol::KeyFingerprint;
use apeireth_mesh_protocol::MeshEnvelope;
use async_trait::async_trait;

use crate::transport::{MeshConnection, MeshTransportError};

use super::e2e::SecureChannel;

/// One persistent, end-to-end encrypted connection between exactly two nodes,
/// carried over a relay. Semantically identical to a direct link.
pub struct RelayMeshConnection {
    channel: SecureChannel,
    closed: AtomicBool,
}

impl RelayMeshConnection {
    /// Wrap an established, handshaken secure channel as a mesh connection.
    pub fn new(channel: SecureChannel) -> Self {
        Self {
            channel,
            closed: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl MeshConnection for RelayMeshConnection {
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), MeshTransportError> {
        if self.is_closed() {
            return Err(MeshTransportError::Closed("relay connection closed".into()));
        }
        let bytes = serde_json::to_vec(&envelope).map_err(|error| {
            MeshTransportError::ProtocolViolation(format!("envelope not encodable: {error}"))
        })?;
        self.channel.send_sealed(&bytes).await
    }

    async fn recv(&self) -> Result<MeshEnvelope, MeshTransportError> {
        if self.is_closed() {
            return Err(MeshTransportError::Closed("relay connection closed".into()));
        }
        let bytes = self.channel.recv_sealed().await?;
        serde_json::from_slice::<MeshEnvelope>(&bytes).map_err(|error| {
            // A decrypted-but-unparseable payload means a broken peer or a
            // protocol version gap — fail closed, never a half-alive link.
            MeshTransportError::ProtocolViolation(format!("envelope not decodable: {error}"))
        })
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.channel.close();
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) || self.channel.is_closed()
    }

    fn peer_fingerprint(&self) -> Option<KeyFingerprint> {
        Some(self.channel.peer_fingerprint())
    }

    fn remote_endpoint_hint(&self) -> Option<String> {
        // A relayed peer has no fixed socket address; the circuit is only a
        // reachability hint (never an identity).
        Some("relay".to_string())
    }
}
