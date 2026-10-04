//! Bind once, observe, then hand the *same* socket to QUIC.
//!
//! # Why this exists
//!
//! A server-reflexive candidate is only meaningful if the address an outside
//! observer saw belongs to the socket that will later do the dialing. If
//! observation runs on socket A and QUIC binds socket B, the candidate
//! describes A's NAT mapping and is worthless for B: different local port, and
//! on most NATs a different mapping entirely.
//!
//! So the lifetime is strictly sequential and single-owner:
//!
//! ```text
//! UdpSocket::bind
//!       |
//!   observe()           <- probe and reply both on this socket
//!       |
//!   into_quic_client()  <- the socket is MOVED into quinn
//!       v
//!   quinn::Endpoint     <- from here on quinn owns every datagram
//! ```
//!
//! Observation happens strictly *before* the handoff, so nothing ever competes
//! with quinn for the same datagrams. [`PreparedMeshEndpoint::into_quic_client`]
//! consumes `self`, so "observe after handoff" is not expressible rather than
//! merely discouraged.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::{KeyFingerprint, NodeIdentity};
use apeireth_mesh::MeshTransportError;
use apeireth_mesh_protocol::observe::{
    decode_reply, encode_probe, OBSERVE_DATAGRAM_BYTES, OBSERVE_NONCE_BYTES,
};

use crate::config::QuicTransportConfig;
use crate::transport::QuicMeshTransport;

/// Default deadline for one observation attempt.
pub const DEFAULT_OBSERVE_TIMEOUT_MS: u64 = 2_000;

/// Why an observation attempt did not produce a server-reflexive address.
///
/// A caller may carry on without a server-reflexive candidate. What it must not
/// do is pretend it has one.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ObserveError {
    /// The observer did not answer before the deadline.
    #[error("observer did not answer within {timeout_ms} ms")]
    Timeout {
        /// The deadline that elapsed.
        timeout_ms: u64,
    },
    /// The socket failed to send or receive.
    #[error("observation io failed: {0}")]
    Io(String),
    /// A well-formed reply arrived that did not echo this node's nonce, so it
    /// is stale or off-path and its address must not be adopted.
    #[error("observer reply did not echo the probe nonce")]
    NonceMismatch,
}

/// A bound UDP socket that has not yet been given to QUIC.
///
/// Owns the socket outright. [`PreparedMeshEndpoint::observe`] borrows it;
/// [`PreparedMeshEndpoint::into_quic_client`] consumes `self`.
pub struct PreparedMeshEndpoint {
    socket: tokio::net::UdpSocket,
}

impl PreparedMeshEndpoint {
    /// Bind `bind` (use port 0 to let the OS choose a port) and keep the socket.
    pub async fn bind(bind: SocketAddr) -> Result<Self, MeshTransportError> {
        let socket = std::net::UdpSocket::bind(bind)
            .map_err(|e| MeshTransportError::Io(format!("bind {bind}: {e}")))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| MeshTransportError::Io(format!("set nonblocking: {e}")))?;
        let socket = tokio::net::UdpSocket::from_std(socket)
            .map_err(|e| MeshTransportError::Io(format!("wrap socket: {e}")))?;
        Ok(Self { socket })
    }

    /// The address actually bound (useful with port 0).
    pub fn local_addr(&self) -> Result<SocketAddr, MeshTransportError> {
        self.socket
            .local_addr()
            .map_err(|e| MeshTransportError::Io(e.to_string()))
    }

    /// Ask `observer` what public address it sees for **this** socket.
    ///
    /// The probe is sent from the socket's own local port, so the answer
    /// describes the mapping a QUIC dial from this socket will use. A single
    /// datagram is easy to lose, so probes are re-sent inside the window; a
    /// datagram that is not our reply is ignored rather than fatal.
    pub async fn observe(
        &self,
        observer: SocketAddr,
        timeout: Duration,
    ) -> Result<SocketAddr, ObserveError> {
        let nonce = random_nonce();
        let probe = encode_probe(&nonce);
        let timeout_ms = timeout.as_millis() as u64;
        let deadline = tokio::time::Instant::now() + timeout;
        let resend_every = (timeout / 3).max(Duration::from_millis(100));
        let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];

        loop {
            self.socket
                .send_to(&probe, observer)
                .await
                .map_err(|e| ObserveError::Io(e.to_string()))?;

            let now = tokio::time::Instant::now();
            if now >= deadline {
                return Err(ObserveError::Timeout { timeout_ms });
            }
            let wait = deadline.saturating_duration_since(now).min(resend_every);

            match tokio::time::timeout(wait, self.socket.recv_from(&mut buf)).await {
                Ok(Ok((n, _from))) => {
                    let Some((echoed, observed)) = decode_reply(&buf[..n]) else {
                        // Not an observation reply at all: keep waiting.
                        continue;
                    };
                    if echoed != nonce {
                        return Err(ObserveError::NonceMismatch);
                    }
                    return Ok(observed);
                }
                Ok(Err(error)) => return Err(ObserveError::Io(error.to_string())),
                // The resend window elapsed with nothing usable; try again.
                Err(_) => continue,
            }
        }
    }

    /// Hand this exact socket to a QUIC endpoint that both dials and accepts.
    ///
    /// This is the simultaneous-open form: one socket, one mapping, both roles.
    pub fn into_quic_peer_endpoint(
        self,
        identity: Arc<NodeIdentity>,
        trusted_peer_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<QuicPeerEndpoint, MeshTransportError> {
        let socket = self
            .socket
            .into_std()
            .map_err(|e| MeshTransportError::Io(format!("unwrap socket: {e}")))?;
        QuicPeerEndpoint::from_bound_socket(socket, identity, trusted_peer_pins, config)
    }

    /// Hand this exact socket to a QUIC client endpoint.
    ///
    /// Nothing is re-bound, so the local port the observer just reported is the
    /// port QUIC will dial from.
    pub fn into_quic_client(
        self,
        identity: Arc<NodeIdentity>,
        expected_peer_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<QuicMeshTransport, MeshTransportError> {
        let socket = self
            .socket
            .into_std()
            .map_err(|e| MeshTransportError::Io(format!("unwrap socket: {e}")))?;
        QuicMeshTransport::from_bound_socket(socket, identity, expected_peer_pins, config)
    }
}

fn random_nonce() -> [u8; OBSERVE_NONCE_BYTES] {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut nonce = [0u8; OBSERVE_NONCE_BYTES];
    SystemRandom::new()
        .fill(&mut nonce)
        .expect("system rng fills a 16 byte nonce");
    nonce
}

/// Hand this exact socket to a QUIC endpoint that can both dial and accept.
///
/// Simultaneous open needs one socket in both roles: hole punching works only
/// if the peer sends its first packet from the mapping an observer saw *and*
/// can answer an inbound first packet on that same mapping. A dial-only
/// endpoint and a listener on two sockets would be two mappings, which is the
/// thing hole punching exists to avoid.
pub struct QuicPeerEndpoint {
    endpoint: quinn::Endpoint,
    config: QuicTransportConfig,
}

impl QuicPeerEndpoint {
    /// Bind `addr` and prepare both roles over the one socket.
    pub fn bind(
        addr: SocketAddr,
        identity: Arc<NodeIdentity>,
        trusted_peer_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let socket = std::net::UdpSocket::bind(addr)
            .map_err(|e| MeshTransportError::Io(format!("bind {addr}: {e}")))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| MeshTransportError::Io(format!("set nonblocking: {e}")))?;
        Self::from_bound_socket(socket, identity, trusted_peer_pins, config)
    }

    /// Build both roles on a socket the caller already bound.
    pub fn from_bound_socket(
        socket: std::net::UdpSocket,
        identity: Arc<NodeIdentity>,
        trusted_peer_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let client_tls = crate::tls::client_config(&identity, trusted_peer_pins)
            .map_err(MeshTransportError::Io)?;
        let server_tls = crate::tls::server_config(&identity, trusted_peer_pins)
            .map_err(MeshTransportError::Io)?;
        Self::from_bound_socket_with_tls(socket, client_tls, server_tls, config)
    }

    /// Both roles with explicit TLS configs (the `testing` module is the only
    /// caller that passes relaxed verifiers).
    pub(crate) fn from_bound_socket_with_tls(
        socket: std::net::UdpSocket,
        client_tls: rustls::ClientConfig,
        server_tls: rustls::ServerConfig,
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let mut client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_tls)
                .map_err(|e| MeshTransportError::Io(e.to_string()))?,
        ));
        client_config.transport_config(Arc::new(crate::transport::transport_config(&config)));
        let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(server_tls)
            .map_err(|e| MeshTransportError::Io(e.to_string()))?;
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
        server_config.transport_config(Arc::new(crate::transport::transport_config(&config)));
        let runtime = quinn::default_runtime().ok_or_else(|| {
            MeshTransportError::Io("quinn: no async runtime to wrap the socket".to_string())
        })?;
        let mut endpoint = quinn::Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(server_config),
            socket,
            runtime,
        )
        .map_err(|e| MeshTransportError::Io(format!("quic peer endpoint from socket: {e}")))?;
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint, config })
    }

    /// The address actually bound (useful with port 0).
    pub fn local_addr(&self) -> Result<SocketAddr, MeshTransportError> {
        self.endpoint
            .local_addr()
            .map_err(|e| MeshTransportError::Io(e.to_string()))
    }

    /// Dial `endpoint` (`host:port`) in the QUIC client role.
    pub async fn connect(
        &self,
        endpoint: &str,
    ) -> Result<Arc<dyn apeireth_mesh::MeshConnection>, MeshTransportError> {
        let server_addr: SocketAddr = endpoint.parse().map_err(|_| {
            MeshTransportError::ProtocolViolation(format!("invalid endpoint {endpoint:?}"))
        })?;
        let connecting = self
            .endpoint
            .connect(server_addr, "apeireth-mesh")
            .map_err(|e| MeshTransportError::Io(format!("quic connect failed: {e}")))?;
        let connection = tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout_ms),
            connecting,
        )
        .await
        .map_err(|_| MeshTransportError::Io("connect handshake timeout".to_string()))?
        .map_err(|e| MeshTransportError::Io(format!("quic handshake failed: {e}")))?;
        let fingerprint = crate::transport::peer_fingerprint(&connection);
        let link =
            crate::connection::QuicMeshConnection::establish(connection, &self.config, fingerprint)
                .await?;
        Ok(link as Arc<dyn apeireth_mesh::MeshConnection>)
    }

    /// Accept one inbound connection in the QUIC server role.
    ///
    /// Completing TLS is not admission: the returned link still has to pass the
    /// mesh trust check above this boundary, exactly like a dialed one.
    pub async fn accept(
        &self,
    ) -> Result<Arc<dyn apeireth_mesh::MeshConnection>, MeshTransportError> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or_else(|| MeshTransportError::Closed("peer endpoint closed".to_string()))?;
        let connection = incoming
            .await
            .map_err(|e| MeshTransportError::Io(format!("quic accept failed: {e}")))?;
        let fingerprint = crate::transport::peer_fingerprint(&connection);
        let link =
            crate::connection::QuicMeshConnection::establish(connection, &self.config, fingerprint)
                .await?;
        Ok(link as Arc<dyn apeireth_mesh::MeshConnection>)
    }

    /// Close the endpoint: pending accepts end, live connections close.
    pub fn close(&self) {
        self.endpoint
            .close(0u32.into(), b"mesh: peer endpoint closed");
    }
}

#[async_trait::async_trait]
impl apeireth_mesh::MeshTransport for QuicPeerEndpoint {
    async fn dial(
        &self,
        endpoint: &str,
    ) -> Result<Arc<dyn apeireth_mesh::MeshConnection>, MeshTransportError> {
        self.connect(endpoint).await
    }
}

#[async_trait::async_trait]
impl apeireth_mesh::MeshListener for QuicPeerEndpoint {
    async fn accept(&self) -> Result<Arc<dyn apeireth_mesh::MeshConnection>, MeshTransportError> {
        QuicPeerEndpoint::accept(self).await
    }
}
