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

/// One DER TLV: `(tag, content, remainder)`.
///
/// Strict on purpose: definite lengths only, minimal length encoding, no
/// multi-byte tags, every length bounds-checked against the input.
fn read_tlv(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = input.split_first()?;
    if tag & 0x1f == 0x1f {
        return None;
    }
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 3 || rest.len() < count {
            return None;
        }
        let (len_bytes, rest) = rest.split_at(count);
        if len_bytes[0] == 0 {
            return None;
        }
        let len = len_bytes
            .iter()
            .fold(0usize, |acc, byte| (acc << 8) | usize::from(*byte));
        if len < 0x80 {
            return None;
        }
        (len, rest)
    };
    if rest.len() < len {
        return None;
    }
    let (content, remainder) = rest.split_at(len);
    Some((tag, content, remainder))
}

/// Fingerprint of the Ed25519 public key inside a certificate.
///
/// The key is read from where X.509 puts it — the `subjectPublicKeyInfo`
/// element of the `tbsCertificate` — and nowhere else. A byte-pattern search
/// would be a bypass: a certificate may carry arbitrary bytes in its issuer
/// or subject, so an attacker could embed a victim's SPKI there (passing the
/// pin) while the TLS handshake signature, which `webpki` verifies against
/// the *real* SPKI, is made with the attacker's own key. Walking the
/// structure makes the key that is pinned and the key that is proven the same
/// key by construction.
///
/// Returns `None` for anything that is not a well-formed certificate whose
/// SPKI is exactly an Ed25519 public key.
pub fn fingerprint_of_certificate(cert: &CertificateDer<'_>) -> Option<KeyFingerprint> {
    const ED25519_SPKI_PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    const SEQUENCE: u8 = 0x30;
    const INTEGER: u8 = 0x02;
    const VERSION: u8 = 0xa0;

    let (tag, certificate, trailing) = read_tlv(cert.as_ref())?;
    if tag != SEQUENCE || !trailing.is_empty() {
        return None;
    }
    let (tag, tbs, _signature_algorithm_and_value) = read_tlv(certificate)?;
    if tag != SEQUENCE {
        return None;
    }

    let mut rest = tbs;
    let (tag, _, after) = read_tlv(rest)?;
    if tag == VERSION {
        rest = after;
    }
    // serialNumber, signature, issuer, validity, subject.
    for expected in [INTEGER, SEQUENCE, SEQUENCE, SEQUENCE, SEQUENCE] {
        let (tag, _, after) = read_tlv(rest)?;
        if tag != expected {
            return None;
        }
        rest = after;
    }

    let (tag, spki_body, _) = read_tlv(rest)?;
    if tag != SEQUENCE || spki_body.len() != ED25519_SPKI_PREFIX.len() + 32 - 2 {
        return None;
    }
    let spki = rest.get(..ED25519_SPKI_PREFIX.len() + 32)?;
    if spki[..ED25519_SPKI_PREFIX.len()] != ED25519_SPKI_PREFIX {
        return None;
    }
    Some(KeyFingerprint::of_public_key(
        &spki[ED25519_SPKI_PREFIX.len()..],
    ))
}

/// The only handshake signature scheme mesh identities use.
fn identity_signature_schemes() -> Vec<SignatureScheme> {
    vec![SignatureScheme::ED25519]
}

/// Verifier core shared by both directions: the peer certificate must carry an
/// Ed25519 identity key, and the TLS handshake signature (checked by the
/// caller against the certificate's own SPKI) proves possession of it. With a
/// pin set the key must additionally be one of the pinned fingerprints; with
/// none (the pairing channel) any proven key is accepted, because pinning is
/// exactly what pairing produces. Nothing is recorded here — per-connection
/// identity evidence comes from the connection's own certificate chain
/// (`quinn`'s `peer_identity`).
struct PinCheck {
    allowed: Option<Vec<KeyFingerprint>>,
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
        if let Some(allowed) = &self.allowed {
            if !allowed.contains(&fingerprint) {
                return Err(TlsError::General(format!(
                    "peer key {fingerprint} is not pinned in this node's trust policy"
                )));
            }
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
        identity_signature_schemes()
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
        identity_signature_schemes()
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

fn pin_check(allowed: Option<Vec<KeyFingerprint>>) -> Arc<PinCheck> {
    Arc::new(PinCheck {
        allowed,
        algorithms: rustls::crypto::ring::default_provider().signature_verification_algorithms,
    })
}

fn build_client_config(
    identity: &NodeIdentity,
    allowed: Option<Vec<KeyFingerprint>>,
) -> Result<ClientConfig, String> {
    let (cert, key) = identity_certificate(identity)?;
    let mut config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinServerVerifier(pin_check(allowed))))
        .with_client_auth_cert(vec![cert], key.into())
        .map_err(|error| error.to_string())?;
    config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
    Ok(config)
}

fn build_server_config(
    identity: &NodeIdentity,
    allowed: Option<Vec<KeyFingerprint>>,
) -> Result<ServerConfig, String> {
    let (cert, key) = identity_certificate(identity)?;
    let mut config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(PinClientVerifier(pin_check(allowed))))
        .with_single_cert(vec![cert], key.into())
        .map_err(|error| error.to_string())?;
    config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
    Ok(config)
}

/// Production client config: mutual TLS with pin verification in both
/// directions. `expected_peer_pins` are the only acceptable server keys —
/// anything else is refused before a single mesh byte moves.
pub fn client_config(
    identity: &NodeIdentity,
    expected_peer_pins: &[KeyFingerprint],
) -> Result<ClientConfig, String> {
    build_client_config(identity, Some(expected_peer_pins.to_vec()))
}

/// Production server config: mutual TLS; clients must present a pinned
/// identity key.
pub fn server_config(
    identity: &NodeIdentity,
    trusted_client_pins: &[KeyFingerprint],
) -> Result<ServerConfig, String> {
    build_server_config(identity, Some(trusted_client_pins.to_vec()))
}

/// Pairing-channel client config: the peer must prove possession of **some**
/// Ed25519 identity key (the handshake signature is verified against the
/// certificate's own SPKI) but no pin exists yet — pinning is exactly what the
/// pairing ceremony produces. The pairing protocol binds claims to this
/// channel's proven key; nothing else runs on it.
pub fn pairing_client_config(identity: &NodeIdentity) -> Result<ClientConfig, String> {
    build_client_config(identity, None)
}

/// Pairing-channel server config: same contract as [`pairing_client_config`].
pub fn pairing_server_config(identity: &NodeIdentity) -> Result<ServerConfig, String> {
    build_server_config(identity, None)
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

    fn spki_of(identity: &NodeIdentity) -> Vec<u8> {
        let der = rustls::pki_types::PrivatePkcs8KeyDer::from(identity.pkcs8_der().to_vec());
        rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&der, &rcgen::PKCS_ED25519)
            .unwrap()
            .public_key_der()
    }

    /// A self-signed certificate made with `signer`'s key whose subject
    /// carries `smuggled` bytes verbatim.
    fn certificate_smuggling(signer: &NodeIdentity, smuggled: Vec<u8>) -> CertificateDer<'static> {
        let der = rustls::pki_types::PrivatePkcs8KeyDer::from(signer.pkcs8_der().to_vec());
        let key_pair =
            rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&der, &rcgen::PKCS_ED25519).unwrap();
        let mut params = rcgen::CertificateParams::new(vec!["apeireth-mesh".to_string()]).unwrap();
        params
            .custom_extensions
            .push(rcgen::CustomExtension::from_oid_content(
                &[1, 3, 6, 1, 4, 1, 99999, 1],
                smuggled,
            ));
        CertificateDer::from(params.self_signed(&key_pair).unwrap().der().to_vec())
    }

    #[test]
    fn a_victim_key_smuggled_into_an_extension_is_not_the_certificate_key() {
        let victim = NodeIdentity::generate().unwrap();
        let attacker = NodeIdentity::generate().unwrap();
        let victim_spki = spki_of(&victim);
        let forged = certificate_smuggling(&attacker, victim_spki.clone());

        assert!(
            forged
                .as_ref()
                .windows(victim_spki.len())
                .any(|window| window == victim_spki.as_slice()),
            "the forged certificate must really contain the victim SPKI bytes"
        );
        let fingerprint = fingerprint_of_certificate(&forged).unwrap();
        assert_eq!(&fingerprint, attacker.fingerprint());
        assert_ne!(&fingerprint, victim.fingerprint());
    }

    #[test]
    fn malformed_certificates_have_no_fingerprint() {
        let identity = NodeIdentity::generate().unwrap();
        let (cert, _) = identity_certificate(&identity).unwrap();
        let bytes = cert.as_ref().to_vec();
        for cut in [0, 1, 4, bytes.len() / 2, bytes.len() - 1] {
            assert!(
                fingerprint_of_certificate(&CertificateDer::from(bytes[..cut].to_vec())).is_none()
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(fingerprint_of_certificate(&CertificateDer::from(trailing)).is_none());
    }

    fn handshake(client: ClientConfig, server: ServerConfig) -> Result<(), String> {
        let name = ServerName::try_from("apeireth-mesh").unwrap();
        let mut client = rustls::ClientConnection::new(Arc::new(client), name).unwrap();
        let mut server = rustls::ServerConnection::new(Arc::new(server)).unwrap();
        for _ in 0..16 {
            let mut buffer = Vec::new();
            while client.wants_write() {
                client.write_tls(&mut buffer).unwrap();
            }
            if !buffer.is_empty() {
                server
                    .read_tls(&mut buffer.as_slice())
                    .map_err(|e| e.to_string())?;
                server.process_new_packets().map_err(|e| e.to_string())?;
            }
            let mut buffer = Vec::new();
            while server.wants_write() {
                server.write_tls(&mut buffer).unwrap();
            }
            if !buffer.is_empty() {
                client
                    .read_tls(&mut buffer.as_slice())
                    .map_err(|e| e.to_string())?;
                client.process_new_packets().map_err(|e| e.to_string())?;
            }
            if !client.is_handshaking() && !server.is_handshaking() {
                return Ok(());
            }
        }
        Err("handshake did not complete".to_string())
    }

    #[derive(Debug)]
    struct StolenCertificate(Arc<rustls::sign::CertifiedKey>);

    impl rustls::client::ResolvesClientCert for StolenCertificate {
        fn resolve(
            &self,
            _root_hint_subjects: &[&[u8]],
            _sigschemes: &[SignatureScheme],
        ) -> Option<Arc<rustls::sign::CertifiedKey>> {
            Some(self.0.clone())
        }

        fn has_certs(&self) -> bool {
            true
        }
    }

    /// Presents the victim's certificate while signing with the attacker's key.
    fn client_presenting_a_stolen_certificate(
        victim: &NodeIdentity,
        attacker: &NodeIdentity,
    ) -> ClientConfig {
        let (victim_cert, _) = identity_certificate(victim).unwrap();
        let attacker_key =
            rustls::pki_types::PrivatePkcs8KeyDer::from(attacker.pkcs8_der().to_vec());
        let signing_key = rustls::crypto::ring::sign::any_eddsa_type(&attacker_key).unwrap();
        let certified = Arc::new(rustls::sign::CertifiedKey::new(
            vec![victim_cert],
            signing_key,
        ));
        let mut config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PinServerVerifier(pin_check(None))))
            .with_client_cert_resolver(Arc::new(StolenCertificate(certified)));
        config.alpn_protocols = vec![crate::config::QuicTransportConfig::ALPN.to_vec()];
        config
    }

    #[test]
    fn the_pairing_channel_accepts_an_honest_unpinned_peer() {
        let client = NodeIdentity::generate().unwrap();
        let server = NodeIdentity::generate().unwrap();
        handshake(
            pairing_client_config(&client).unwrap(),
            pairing_server_config(&server).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn the_pairing_channel_rejects_a_peer_that_does_not_hold_the_key() {
        let victim = NodeIdentity::generate().unwrap();
        let attacker = NodeIdentity::generate().unwrap();
        let server = NodeIdentity::generate().unwrap();
        let error = handshake(
            client_presenting_a_stolen_certificate(&victim, &attacker),
            pairing_server_config(&server).unwrap(),
        )
        .unwrap_err();
        assert!(!error.contains("did not complete"), "{error}");
    }

    #[test]
    fn a_stolen_certificate_also_fails_against_a_pinned_server() {
        let victim = NodeIdentity::generate().unwrap();
        let attacker = NodeIdentity::generate().unwrap();
        let server = NodeIdentity::generate().unwrap();
        let config = server_config(&server, &[victim.fingerprint().clone()]).unwrap();
        assert!(handshake(
            client_presenting_a_stolen_certificate(&victim, &attacker),
            config
        )
        .is_err());
    }

    #[test]
    fn a_pinned_server_refuses_an_unpinned_honest_client() {
        let client = NodeIdentity::generate().unwrap();
        let server = NodeIdentity::generate().unwrap();
        let stranger = NodeIdentity::generate().unwrap();
        let config = server_config(&server, &[stranger.fingerprint().clone()]).unwrap();
        assert!(handshake(pairing_client_config(&client).unwrap(), config).is_err());
    }
}
