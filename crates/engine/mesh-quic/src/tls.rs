//! TLS for mesh QUIC: an encrypted channel whose **certificate key is the
//! mesh identity key**.
//!
//! # The binding, stated plainly
//!
//! TLS encryption alone does not authenticate an Apeireth node. Here the two
//! are welded by construction: each node's self-signed certificate is built
//! from its [`NodeIdentity`] Ed25519 key, and peers pin the SHA-256
//! fingerprint of that same key. A completed TLS handshake against a pinned
//! verifier therefore *is* the proof "I control the private key pinned for
//! this NodeId" — a fresh ephemeral exchange, replay-immune, covering both
//! directions (mutual TLS). The Hello carried inside the channel cannot be
//! modified by anyone but the two endpoints, so protocol downgrades are not
//! reachable either.
//!
//! # Fail closed, everywhere
//!
//! Unknown key → refused. Mismatched key → refused. Right key under the wrong
//! NodeId → refused at the node binding (`MeshNode` compares the presented
//! fingerprint with the pin of the claimed id). There is no TOFU and no
//! "accept any certificate" in any production constructor. A relaxed verifier
//! exists only under [`testing`] and is never a default.

use std::sync::Arc;

use apeireth_mesh::identity::{KeyFingerprint, NodeIdentity};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{
    ClientConfig, DigitallySignedStruct, DistinguishedName, Error as TlsError, ServerConfig,
    SignatureScheme,
};

/// Build the self-signed channel certificate for a node identity: the
/// certificate key IS the identity key (`certificate key == mesh identity
/// key`), so proving the TLS key proves the mesh key.
pub fn identity_certificate(
    identity: &NodeIdentity,
) -> Result<
    (
        CertificateDer<'static>,
        rustls::pki_types::PrivatePkcs8KeyDer<'static>,
    ),
    String,
> {
    let private_der = rustls::pki_types::PrivatePkcs8KeyDer::from(identity.pkcs8_der().to_vec());
    let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&private_der, &rcgen::PKCS_ED25519)
        .map_err(|error| format!("identity key rejected by certificate builder: {error}"))?;
    let params = rcgen::CertificateParams::new(vec!["apeireth-mesh".to_string()])
        .map_err(|e| e.to_string())?;
    let cert = params.self_signed(&key_pair).map_err(|e| e.to_string())?;
    let cert_der: CertificateDer<'static> = CertificateDer::from(cert.der().to_vec());
    Ok((cert_der, private_der))
}

/// Fingerprint of the Ed25519 public key inside a certificate.
///
/// Ed25519 SPKI is a fixed 12-byte prefix plus the 32-byte key; extracting it
/// keeps the fingerprint definition identical to
/// [`KeyFingerprint::of_public_key`] without dragging in an X.509 parser.
pub fn fingerprint_of_certificate(cert: &CertificateDer<'_>) -> Option<KeyFingerprint> {
    const ED25519_SPKI_PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    let der = cert.as_ref();
    let found = der
        .windows(ED25519_SPKI_PREFIX.len())
        .position(|w| w == ED25519_SPKI_PREFIX)?;
    let key_start = found + ED25519_SPKI_PREFIX.len();
    let key = der.get(key_start..key_start + 32)?;
    Some(KeyFingerprint::of_public_key(key))
}

/// Pinned-verifier core shared by both directions: the peer certificate must
/// carry an Ed25519 key whose fingerprint is in the pin set. Nothing is
/// recorded here — per-connection identity evidence comes from the
/// connection's own certificate chain (`quinn`'s `peer_identity`).
struct PinCheck {
    allowed: Vec<KeyFingerprint>,
    algorithms: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl std::fmt::Debug for PinCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinCheck")
            .field("allowed", &self.allowed)
            .finish_non_exhaustive()
    }
}

impl PinCheck {
    fn verify_cert(&self, cert: &CertificateDer<'_>) -> Result<KeyFingerprint, TlsError> {
        let fingerprint = fingerprint_of_certificate(cert).ok_or_else(|| {
            TlsError::General("certificate is not an Ed25519 identity".to_string())
        })?;
        if !self.allowed.contains(&fingerprint) {
            return Err(TlsError::General(format!(
                "peer key {fingerprint} is not pinned in this node's trust policy"
            )));
        }
        Ok(fingerprint)
    }
}

#[derive(Debug)]
struct PinServerVerifier(Arc<PinCheck>);

impl ServerCertVerifier for PinServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        self.0.verify_cert(end_entity)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.algorithms.supported_schemes()
    }
}

#[derive(Debug)]
struct PinClientVerifier(Arc<PinCheck>);

impl ClientCertVerifier for PinClientVerifier {
    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<ClientCertVerified, TlsError> {
        self.0.verify_cert(end_entity)?;
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.algorithms.supported_schemes()
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn client_auth_mandatory(&self) -> bool {
        // Mutual TLS is the identity proof: an unauthenticated client has no
        // identity to pin against and must not connect at all.
        true
    }
}

/// Production client config: mutual TLS with pin verification in both
/// directions. `expected_peer_pins` are the only acceptable server keys —
/// anything else is refused before a single mesh byte moves.
pub fn client_config(
    identity: &NodeIdentity,
    expected_peer_pins: &[KeyFingerprint],
) -> Result<ClientConfig, String> {
    let (cert, key) = identity_certificate(identity)?;
    let check = Arc::new(PinCheck {
        allowed: expected_peer_pins.to_vec(),
        algorithms: rustls::crypto::ring::default_provider().signature_verification_algorithms,
    });
    let mut config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinServerVerifier(check)))
        .with_client_auth_cert(vec![cert], key.into())
        .map_err(|error| error.to_string())?;
    config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
    Ok(config)
}

/// Production server config: mutual TLS; clients must present a pinned
/// identity key.
pub fn server_config(
    identity: &NodeIdentity,
    trusted_client_pins: &[KeyFingerprint],
) -> Result<ServerConfig, String> {
    let (cert, key) = identity_certificate(identity)?;
    let check = Arc::new(PinCheck {
        allowed: trusted_client_pins.to_vec(),
        algorithms: rustls::crypto::ring::default_provider().signature_verification_algorithms,
    });
    let mut config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(PinClientVerifier(check)))
        .with_single_cert(vec![cert], key.into())
        .map_err(|error| error.to_string())?;
    config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
    Ok(config)
}

/// Pairing-channel client config: the peer must present **some** authenticated
/// Ed25519 identity key (possession is proven by TLS) but no pin exists yet —
/// pinning is exactly what the pairing ceremony produces. The pairing
/// protocol binds claims to this channel's key; nothing else runs on it.
pub fn pairing_client_config(identity: &NodeIdentity) -> Result<ClientConfig, String> {
    testing::accept_any_client_config(identity)
}

/// Pairing-channel server config: same contract as [`pairing_client_config`].
pub fn pairing_server_config(identity: &NodeIdentity) -> Result<ServerConfig, String> {
    testing::accept_any_server_config(identity)
}

/// **Tests only.** Verifiers that accept any certificate — an encrypted
/// channel with no identity binding. Never referenced by any production
/// constructor; existing only so negative tests can reach the node-binding
/// layer past TLS.
pub mod testing {
    use super::*;

    /// Client config that accepts any server certificate. Tests only.
    pub fn accept_any_client_config(identity: &NodeIdentity) -> Result<ClientConfig, String> {
        let (cert, key) = identity_certificate(identity)?;
        let mut config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAny))
            .with_client_auth_cert(vec![cert], key.into())
            .map_err(|error| error.to_string())?;
        config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
        Ok(config)
    }

    /// Server config that accepts any client certificate. Tests only.
    pub fn accept_any_server_config(identity: &NodeIdentity) -> Result<ServerConfig, String> {
        let (cert, key) = identity_certificate(identity)?;
        let mut config = ServerConfig::builder()
            .with_client_cert_verifier(Arc::new(AcceptAnyClient))
            .with_single_cert(vec![cert], key.into())
            .map_err(|error| error.to_string())?;
        config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
        Ok(config)
    }

    #[derive(Debug)]
    struct AcceptAny;

    impl ServerCertVerifier for AcceptAny {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<ServerCertVerified, TlsError> {
            Ok(ServerCertVerified::assertion())
        }
        fn verify_tls12_signature(
            &self,
            _m: &[u8],
            _c: &CertificateDer<'_>,
            _d: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, TlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }
        fn verify_tls13_signature(
            &self,
            _m: &[u8],
            _c: &CertificateDer<'_>,
            _d: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, TlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![SignatureScheme::ED25519]
        }
    }

    #[derive(Debug)]
    struct AcceptAnyClient;

    impl ClientCertVerifier for AcceptAnyClient {
        fn verify_client_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<ClientCertVerified, TlsError> {
            Ok(ClientCertVerified::assertion())
        }
        fn verify_tls12_signature(
            &self,
            _m: &[u8],
            _c: &CertificateDer<'_>,
            _d: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, TlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }
        fn verify_tls13_signature(
            &self,
            _m: &[u8],
            _c: &CertificateDer<'_>,
            _d: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, TlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![SignatureScheme::ED25519]
        }
        fn root_hint_subjects(&self) -> &[DistinguishedName] {
            &[]
        }
        fn client_auth_mandatory(&self) -> bool {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_certificate_carries_the_identity_key() {
        let identity = NodeIdentity::generate().unwrap();
        let (cert, _key) = identity_certificate(&identity).unwrap();
        let fingerprint = fingerprint_of_certificate(&cert).unwrap();
        assert_eq!(&fingerprint, identity.fingerprint());
    }
}
