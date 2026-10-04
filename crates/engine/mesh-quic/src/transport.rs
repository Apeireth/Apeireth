//! QUIC endpoints: dialing and listening for persistent mesh connections.
//!
//! `connect` performs the QUIC+TLS handshake against **pinned** identity keys
//! and returns an established [`MeshConnection`]; the mesh-level Hello/HelloAck
//! handshake then runs above this boundary exactly as it does over the
//! in-memory fabric. Transport mechanics are isolated here; connection
//! semantics stay shared.

use std::net::SocketAddr;
use std::sync::Arc;

use apeireth_mesh::identity::{KeyFingerprint, NodeIdentity};
use apeireth_mesh::{MeshConnection, MeshListener, MeshTransport, MeshTransportError};
use async_trait::async_trait;

use crate::config::QuicTransportConfig;
use crate::connection::QuicMeshConnection;
use crate::tls;

fn transport_config(config: &QuicTransportConfig) -> quinn::TransportConfig {
    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(
        std::time::Duration::from_millis(config.idle_timeout_ms)
            .try_into()
            .unwrap_or_default(),
    ));
    transport.keep_alive_interval(Some(std::time::Duration::from_millis(
        config.keepalive_interval_ms,
    )));
    transport
}

/// The peer's identity fingerprint, read from the connection's own
/// certificate chain — per-connection evidence, never a shared recorder.
fn peer_fingerprint(connection: &quinn::Connection) -> Option<KeyFingerprint> {
    let identity = connection.peer_identity()?;
    let chain = identity.downcast_ref::<Vec<rustls::pki_types::CertificateDer<'static>>>()?;
    chain
        .first()
        .and_then(|cert| tls::fingerprint_of_certificate(cert))
}

/// A dialing endpoint bound to one node identity.
pub struct QuicMeshTransport {
    endpoint: quinn::Endpoint,
    config: QuicTransportConfig,
}

impl QuicMeshTransport {
    /// Build a client endpoint that will only accept the given pinned peer
    /// keys. `bind` is a local socket address (`127.0.0.1:0` for tests);
    /// IPv4 and IPv6 are both fine — the implementation is not
    /// loopback-shaped.
    pub fn new(
        bind: SocketAddr,
        identity: Arc<NodeIdentity>,
        expected_peer_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let client_tls =
            tls::client_config(&identity, expected_peer_pins).map_err(MeshTransportError::Io)?;
        Self::new_with_tls(bind, client_tls, config)
    }

    /// Build with an explicitly chosen TLS config (the `testing` module is the
    /// only caller that passes a relaxed verifier).
    pub(crate) fn new_with_tls(
        bind: SocketAddr,
        client_tls: rustls::ClientConfig,
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let mut endpoint =
            quinn::Endpoint::client(bind).map_err(|e| MeshTransportError::Io(e.to_string()))?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_tls)
                .map_err(|e| MeshTransportError::Io(e.to_string()))?,
        ));
        client_config.transport_config(Arc::new(transport_config(&config)));
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint, config })
    }

    /// Build a client endpoint on a socket the caller already bound.
    ///
    /// The socket is taken **exactly as given**: nothing is re-bound, so the
    /// local port — and therefore the NAT mapping an observer just reported —
    /// is preserved. This is what makes a server-reflexive candidate mean
    /// anything at all.
    pub fn from_bound_socket(
        socket: std::net::UdpSocket,
        identity: Arc<NodeIdentity>,
        expected_peer_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let client_tls =
            tls::client_config(&identity, expected_peer_pins).map_err(MeshTransportError::Io)?;
        Self::from_bound_socket_with_tls(socket, client_tls, config)
    }

    /// [`QuicMeshTransport::from_bound_socket`] with an explicit TLS config
    /// (the `testing` module is the only caller that passes a relaxed verifier).
    pub(crate) fn from_bound_socket_with_tls(
        socket: std::net::UdpSocket,
        client_tls: rustls::ClientConfig,
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let mut client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_tls)
                .map_err(|e| MeshTransportError::Io(e.to_string()))?,
        ));
        client_config.transport_config(Arc::new(transport_config(&config)));
        let runtime = quinn::default_runtime().ok_or_else(|| {
            MeshTransportError::Io("quinn: no async runtime to wrap the socket".to_string())
        })?;
        // `None` server config: this endpoint dials only. An endpoint that must
        // also accept inbound initial packets (simultaneous open) needs both
        // halves and is a separate constructor.
        let mut endpoint =
            quinn::Endpoint::new(quinn::EndpointConfig::default(), None, socket, runtime)
                .map_err(|e| MeshTransportError::Io(format!("quic endpoint from socket: {e}")))?;
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint, config })
    }

    /// Pairing-mode dial: the server must present an authenticated identity
    /// key but need not be pinned yet — pinning is what pairing produces.
    pub fn new_for_pairing(
        bind: SocketAddr,
        identity: Arc<NodeIdentity>,
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let client_tls = tls::pairing_client_config(&identity).map_err(MeshTransportError::Io)?;
        Self::new_with_tls(bind, client_tls, config)
    }

    /// The local socket actually bound (useful with port 0).
    pub fn local_addr(&self) -> Result<SocketAddr, MeshTransportError> {
        self.endpoint
            .local_addr()
            .map_err(|e| MeshTransportError::Io(e.to_string()))
    }

    /// Close the endpoint and every connection on it.
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"mesh: endpoint closed");
    }

    /// Connect to `endpoint` (`host:port`), waiting out the configured
    /// connect timeout. The dial target's key must match a pin supplied at
    /// construction — no TOFU at any level.
    pub async fn connect(
        &self,
        endpoint: &str,
    ) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        let server_addr: SocketAddr = endpoint.parse().map_err(|_| {
            MeshTransportError::ProtocolViolation(format!("invalid endpoint {endpoint:?}"))
        })?;
        let connecting = self
            .endpoint
            .connect(server_addr, "apeireth-mesh")
            .map_err(|error| MeshTransportError::Io(format!("quic connect failed: {error}")))?;

        let connection = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.connect_timeout_ms),
            connecting,
        )
        .await
        .map_err(|_| MeshTransportError::Io("connect handshake timeout".to_string()))?
        .map_err(|error| MeshTransportError::Io(format!("quic handshake failed: {error}")))?;

        let fingerprint = peer_fingerprint(&connection);
        let link = QuicMeshConnection::establish(connection, &self.config, fingerprint).await?;
        Ok(link as Arc<dyn MeshConnection>)
    }
}

#[async_trait]
impl MeshTransport for QuicMeshTransport {
    async fn dial(&self, endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        self.connect(endpoint).await
    }
}

/// A listening endpoint: accept inbound authenticated connections.
pub struct QuicMeshListener {
    endpoint: quinn::Endpoint,
    config: QuicTransportConfig,
}

impl QuicMeshListener {
    /// Bind `addr` (`127.0.0.1:0` in tests) with mutual-TLS pin verification:
    /// a client must present a key in `trusted_client_pins` or the handshake
    /// is refused before any mesh byte moves.
    pub fn bind(
        addr: SocketAddr,
        identity: Arc<NodeIdentity>,
        trusted_client_pins: &[KeyFingerprint],
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let server_tls =
            tls::server_config(&identity, trusted_client_pins).map_err(MeshTransportError::Io)?;
        Self::bind_with_tls(addr, server_tls, config)
    }

    /// Bind with an explicitly chosen TLS config (the `testing` module is the
    /// only caller that passes a relaxed verifier).
    pub(crate) fn bind_with_tls(
        addr: SocketAddr,
        server_tls: rustls::ServerConfig,
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(server_tls)
            .map_err(|e| MeshTransportError::Io(e.to_string()))?;
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
        server_config.transport_config(Arc::new(transport_config(&config)));
        let endpoint = quinn::Endpoint::server(server_config, addr)
            .map_err(|e| MeshTransportError::Io(e.to_string()))?;
        Ok(Self { endpoint, config })
    }

    /// Pairing-mode listener: clients must present an authenticated identity
    /// key but need not be pinned — the ceremony produces the pin.
    pub fn bind_for_pairing(
        addr: SocketAddr,
        identity: Arc<NodeIdentity>,
        config: QuicTransportConfig,
    ) -> Result<Self, MeshTransportError> {
        let server_tls = tls::pairing_server_config(&identity).map_err(MeshTransportError::Io)?;
        Self::bind_with_tls(addr, server_tls, config)
    }

    /// The local socket actually bound (useful with port 0).
    pub fn local_addr(&self) -> Result<SocketAddr, MeshTransportError> {
        self.endpoint
            .local_addr()
            .map_err(|e| MeshTransportError::Io(e.to_string()))
    }

    /// Close the listener; pending accepts end.
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"mesh: listener closed");
    }
}

#[async_trait]
impl MeshListener for QuicMeshListener {
    async fn accept(&self) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or_else(|| MeshTransportError::Closed("listener closed".to_string()))?;
        let connection = incoming
            .await
            .map_err(|error| MeshTransportError::Io(format!("quic accept failed: {error}")))?;
        let fingerprint = peer_fingerprint(&connection);
        let link = QuicMeshConnection::establish(connection, &self.config, fingerprint).await?;
        Ok(link as Arc<dyn MeshConnection>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_endpoint_binds_and_reports_its_socket() {
        let identity = Arc::new(NodeIdentity::generate().unwrap());
        let listener = QuicMeshListener::bind(
            "127.0.0.1:0".parse().unwrap(),
            identity,
            &[],
            QuicTransportConfig::default(),
        )
        .unwrap();
        let addr = listener.local_addr().unwrap();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_ne!(addr.port(), 0);
    }
}

/// **Tests only.** Constructors that use the relaxed TLS verifier — an
/// encrypted channel with no identity binding at the TLS layer. Production
/// code cannot reach these through the pinned constructors; negative tests
/// use them to prove the *node binding* layer still refuses forged identities
/// even when TLS does not.
pub mod testing {
    use super::*;

    /// Bind a listener that accepts any client certificate. Tests only.
    pub fn bind_accept_any(
        addr: SocketAddr,
        identity: Arc<NodeIdentity>,
        config: QuicTransportConfig,
    ) -> Result<QuicMeshListener, MeshTransportError> {
        let server_tls =
            tls::testing::accept_any_server_config(&identity).map_err(MeshTransportError::Io)?;
        QuicMeshListener::bind_with_tls(addr, server_tls, config)
    }

    /// Dial with a client that accepts any server certificate. Tests only.
    pub fn dial_accept_any(
        bind: SocketAddr,
        identity: Arc<NodeIdentity>,
        config: QuicTransportConfig,
    ) -> Result<QuicMeshTransport, MeshTransportError> {
        let client_tls =
            tls::testing::accept_any_client_config(&identity).map_err(MeshTransportError::Io)?;
        QuicMeshTransport::new_with_tls(bind, client_tls, config)
    }
}
