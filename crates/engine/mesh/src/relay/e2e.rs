//! End-to-end encrypted secure channel over an opaque relay tunnel.
//!
//! # Why this exists
//!
//! A relay moves bytes between two nodes that cannot reach each other directly.
//! If the relay simply forwarded plaintext, it would see every `MeshEnvelope` —
//! tool arguments, provider requests, results. That is forbidden. This module
//! runs **on the two nodes** and establishes a channel the relay cannot read:
//!
//! ```text
//! A ── end-to-end secure channel ── B        (X25519 + Ed25519 + AEAD)
//!          ↑
//!    relay only forwards ciphertext frames
//! ```
//!
//! # Handshake
//!
//! A signed ephemeral Diffie–Hellman exchange (`X25519`), each side signing its
//! ephemeral public key with its long-lived `NodeIdentity` (Ed25519) over a
//! domain-separated payload bound to the circuit id. This gives three things at
//! once:
//!
//! - **Forward secrecy** — the AEAD keys come from *ephemeral* DH, so a later
//!   identity-key compromise does not decrypt past traffic;
//! - **Authentication** — a signature proves the peer holds the private key for
//!   the identity it claims (possession, not just a claimed id);
//! - **MITM resistance** — the signature binds the ephemeral key to the
//!   identity, so a relay that swaps in its own DH key breaks the signature and
//!   is caught. It cannot re-sign without the node's private key.
//!
//! The channel reports the authenticated peer fingerprint so
//! [`crate::node::MeshNode`] can pin-check it against the `TrustStore` — the
//! relay's own TLS identity is never mistaken for peer identity.
//!
//! # What the relay sees
//!
//! Only the circuit id and AEAD ciphertext. It never sees plaintext envelopes.

use apeireth_mesh_protocol::rendezvous::{RelayCircuitId, ED25519_PUBLIC_KEY_BYTES};
use apeireth_mesh_protocol::KeyFingerprint;

use crate::transport::MeshTransportError;

/// Domain-separation string for the E2E channel handshake signatures.
pub const E2E_HANDSHAKE_DOMAIN: &str = "APEIRETH-MESH-RELAY-E2E-V1";

/// An opaque bidirectional byte tunnel between exactly two nodes, carried by a
/// relay. The relay reads/writes only opaque `Vec<u8>` frames; it never
/// interprets them. This is the seam between "moving ciphertext" (the relay)
/// and "exchanging plaintext envelopes" (the secure channel).
#[async_trait::async_trait]
pub trait OpaqueTunnel: Send + Sync {
    /// Push one opaque frame toward the peer.
    async fn send_opaque(&self, payload: Vec<u8>) -> Result<(), MeshTransportError>;
    /// Pull the next opaque frame from the peer. `Closed` once drained.
    async fn recv_opaque(&self) -> Result<Vec<u8>, MeshTransportError>;
    /// Close the tunnel.
    fn close_opaque(&self);
    /// Whether the tunnel is closed.
    fn is_opaque_closed(&self) -> bool;
}

/// The negotiated, authenticated, end-to-end encrypted channel.
///
/// Holds only what it needs to encrypt/decrypt frames in each direction plus
/// the authenticated peer fingerprint. Private key material used to *establish*
/// the channel is dropped after the handshake.
pub struct SecureChannel {
    tunnel: Box<dyn OpaqueTunnel>,
    send: AeadKey,
    recv: AeadKey,
    peer_fingerprint: KeyFingerprint,
}

impl SecureChannel {
    /// Authenticated peer identity — evidence for [`crate::node::MeshNode`]'s
    /// trust-pin check. Derived from the E2E handshake, never from relay TLS.
    pub fn peer_fingerprint(&self) -> KeyFingerprint {
        self.peer_fingerprint.clone()
    }

    /// The underlying opaque tunnel.
    pub fn tunnel(&self) -> &dyn OpaqueTunnel {
        self.tunnel.as_ref()
    }

    /// Encrypt and send one plaintext payload toward the peer.
    pub async fn send_sealed(&self, plaintext: &[u8]) -> Result<(), MeshTransportError> {
        let sealed = self.send.seal(plaintext)?;
        self.tunnel.send_opaque(sealed).await
    }

    /// Receive and decrypt one plaintext payload from the peer.
    pub async fn recv_sealed(&self) -> Result<Vec<u8>, MeshTransportError> {
        let frame = self.tunnel.recv_opaque().await?;
        self.recv.open(&frame)
    }

    /// Close the channel (and its tunnel).
    pub fn close(&self) {
        self.tunnel.close_opaque();
    }

    /// Whether the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.tunnel.is_opaque_closed()
    }
}

/// Which side of the handshake we are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRole {
    /// The dialing node: sends its hello first.
    Initiator,
    /// The accepting node: replies after receiving the initiator hello.
    Responder,
}

/// Establish an authenticated, encrypted channel over `tunnel`.
///
/// `identity` is this node's `NodeIdentity` (to sign our ephemeral key);
/// `circuit_id` binds the handshake to one relay circuit so a captured
/// handshake cannot be replayed onto a different circuit. The caller performs
/// the `TrustStore` pin check on [`SecureChannel::peer_fingerprint`].
pub async fn handshake(
    tunnel: Box<dyn OpaqueTunnel>,
    role: HandshakeRole,
    identity_pkcs8: &[u8],
    identity_public: &[u8; ED25519_PUBLIC_KEY_BYTES],
    node_id: &str,
    circuit_id: &RelayCircuitId,
) -> Result<SecureChannel, MeshTransportError> {
    let rng = ring::rand::SystemRandom::new();

    // Ephemeral X25519 key for forward secrecy.
    let ephemeral = ring::agreement::EphemeralPrivateKey::generate(&ring::agreement::X25519, &rng)
        .map_err(|e| MeshTransportError::ProtocolViolation(format!("ephemeral keygen: {e}")))?;
    let ephemeral_public = ephemeral
        .compute_public_key()
        .map_err(|e| MeshTransportError::ProtocolViolation(format!("ephemeral pub: {e}")))?;
    let ephemeral_public = *as_32(ephemeral_public.as_ref())?;

    let my_hello = sign_hello(
        identity_pkcs8,
        identity_public,
        node_id,
        &ephemeral_public,
        circuit_id,
    )?;

    // Message order: initiator first, responder answers. Each side verifies the
    // other's hello before deriving keys.
    let peer_hello = match role {
        HandshakeRole::Initiator => {
            tunnel.send_opaque(my_hello.encode()).await?;
            let bytes = tunnel.recv_opaque().await?;
            HelloMessage::decode(&bytes)?
        }
        HandshakeRole::Responder => {
            let bytes = tunnel.recv_opaque().await?;
            let peer = HelloMessage::decode(&bytes)?;
            tunnel.send_opaque(my_hello.encode()).await?;
            peer
        }
    };

    // Verify the peer's signature over its own ephemeral key — this is what a
    // MITM relay cannot forge. Bind-check the circuit id to block replay.
    peer_hello.verify(circuit_id)?;

    let peer_ephemeral = peer_hello.ephemeral_public;
    let peer_identity = peer_hello.identity_public;
    let peer_fingerprint = KeyFingerprint::of_public_key(&peer_identity);

    // ECDH → shared secret → directional AEAD keys. The KDF binds both
    // identities and both ephemeral keys in *initiator-first* canonical order,
    // so both sides derive the same two keys regardless of who calls first.
    let peer_public =
        ring::agreement::UnparsedPublicKey::new(&ring::agreement::X25519, peer_ephemeral);
    let (eph_init, eph_resp, id_init, id_resp) = match role {
        HandshakeRole::Initiator => (
            ephemeral_public,
            peer_ephemeral,
            node_id.to_string(),
            peer_hello.node_id.clone(),
        ),
        HandshakeRole::Responder => (
            peer_ephemeral,
            ephemeral_public,
            peer_hello.node_id.clone(),
            node_id.to_string(),
        ),
    };
    let (k_ir, k_ri) = ring::agreement::agree_ephemeral(ephemeral, &peer_public, |shared| {
        derive_keys(shared, &eph_init, &eph_resp, &id_init, &id_resp)
    })
    .map_err(|_| MeshTransportError::ProtocolViolation("key agreement failed".to_string()))?;

    // The initiator always sends with k_ir; the responder sends with k_ri.
    let (send, recv) = match role {
        HandshakeRole::Initiator => (k_ir, k_ri),
        HandshakeRole::Responder => (k_ri, k_ir),
    };

    Ok(SecureChannel {
        tunnel,
        send,
        recv,
        peer_fingerprint,
    })
}

// ---------------------------------------------------------------------------
// Handshake hello message (opaque to the relay).
// ---------------------------------------------------------------------------

struct HelloMessage {
    ephemeral_public: [u8; 32],
    identity_public: [u8; 32],
    node_id: String,
    signature: Vec<u8>,
}

impl HelloMessage {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(&self.ephemeral_public);
        out.extend_from_slice(&self.identity_public);
        out.extend_from_slice(&(self.node_id.len() as u32).to_be_bytes());
        out.extend_from_slice(self.node_id.as_bytes());
        out.extend_from_slice(&(self.signature.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.signature);
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self, MeshTransportError> {
        let bad = |why: &str| MeshTransportError::ProtocolViolation(format!("hello: {why}"));
        if bytes.len() < 32 + 32 + 4 + 4 {
            return Err(bad("truncated"));
        }
        let mut ephemeral_public = [0u8; 32];
        ephemeral_public.copy_from_slice(&bytes[..32]);
        let mut identity_public = [0u8; 32];
        identity_public.copy_from_slice(&bytes[32..64]);
        let mut off = 64;
        let id_len = read_u32(bytes, &mut off)? as usize;
        if bytes.len() < off + id_len + 4 {
            return Err(bad("truncated node id"));
        }
        let node_id = std::str::from_utf8(&bytes[off..off + id_len])
            .map_err(|_| bad("node id not utf8"))?
            .to_string();
        off += id_len;
        let sig_len = read_u32(bytes, &mut off)? as usize;
        if bytes.len() < off + sig_len {
            return Err(bad("truncated signature"));
        }
        let signature = bytes[off..off + sig_len].to_vec();
        Ok(Self {
            ephemeral_public,
            identity_public,
            node_id,
            signature,
        })
    }

    /// Verify the peer signed its ephemeral key with the private key matching
    /// `identity_public`, for exactly this circuit. This is the check a MITM
    /// relay cannot pass.
    fn verify(&self, circuit_id: &RelayCircuitId) -> Result<(), MeshTransportError> {
        let payload = handshake_payload(&self.ephemeral_public, &self.node_id, circuit_id);
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, &self.identity_public)
            .verify(&payload, &self.signature)
            .map_err(|_| {
                MeshTransportError::ProtocolViolation(
                    "peer handshake signature invalid (possible relay MITM)".to_string(),
                )
            })
    }
}

fn sign_hello(
    identity_pkcs8: &[u8],
    identity_public: &[u8; 32],
    node_id: &str,
    ephemeral_public: &[u8; 32],
    circuit_id: &RelayCircuitId,
) -> Result<HelloMessage, MeshTransportError> {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let pair = Ed25519KeyPair::from_pkcs8(identity_pkcs8)
        .map_err(|e| MeshTransportError::ProtocolViolation(format!("identity key: {e}")))?;
    if pair.public_key().as_ref() != identity_public {
        return Err(MeshTransportError::ProtocolViolation(
            "identity public key mismatch".to_string(),
        ));
    }
    let payload = handshake_payload(ephemeral_public, node_id, circuit_id);
    let signature = pair.sign(&payload).as_ref().to_vec();
    Ok(HelloMessage {
        ephemeral_public: *ephemeral_public,
        identity_public: *identity_public,
        node_id: node_id.to_string(),
        signature,
    })
}

/// Canonical, domain-separated handshake payload. Binds the ephemeral key, the
/// claimer's node id, and the circuit id (replay/transfer protection).
fn handshake_payload(
    ephemeral_public: &[u8; 32],
    node_id: &str,
    circuit_id: &RelayCircuitId,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(96);
    out.extend_from_slice(E2E_HANDSHAKE_DOMAIN.as_bytes());
    out.push(0);
    out.extend_from_slice(&(node_id.len() as u32).to_be_bytes());
    out.extend_from_slice(node_id.as_bytes());
    out.extend_from_slice(ephemeral_public);
    out.extend_from_slice(circuit_id.as_bytes());
    out
}

// ---------------------------------------------------------------------------
// AEAD + KDF.
// ---------------------------------------------------------------------------

/// One directional AEAD key (ChaCha20-Poly1305) with a nonce that increments
/// per message, so no two frames ever reuse a (key, nonce) pair.
struct AeadKey {
    key: ring::aead::LessSafeKey,
    seq: std::sync::atomic::AtomicU64,
}

impl AeadKey {
    fn new(bytes: [u8; 32]) -> Self {
        let unbound = ring::aead::UnboundKey::new(&ring::aead::CHACHA20_POLY1305, &bytes)
            .expect("32-byte chacha key");
        Self {
            key: ring::aead::LessSafeKey::new(unbound),
            seq: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, MeshTransportError> {
        let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let nonce = nonce_from(seq);
        let mut in_out = plaintext.to_vec();
        self.key
            .seal_in_place_append_tag(nonce, ring::aead::Aad::empty(), &mut in_out)
            .map_err(|_| MeshTransportError::ProtocolViolation("seal failed".to_string()))?;
        // Prefix the nonce so the receiver can open without shared state.
        let mut framed = Vec::with_capacity(8 + in_out.len());
        framed.extend_from_slice(&seq.to_be_bytes());
        framed.extend_from_slice(&in_out);
        Ok(framed)
    }

    fn open(&self, frame: &[u8]) -> Result<Vec<u8>, MeshTransportError> {
        if frame.len() < 8 {
            return Err(MeshTransportError::ProtocolViolation(
                "sealed frame truncated".to_string(),
            ));
        }
        let mut seq_bytes = [0u8; 8];
        seq_bytes.copy_from_slice(&frame[..8]);
        let seq = u64::from_be_bytes(seq_bytes);
        let nonce = nonce_from(seq);
        let mut in_out = frame[8..].to_vec();
        let plain = self
            .key
            .open_in_place(nonce, ring::aead::Aad::empty(), &mut in_out)
            .map_err(|_| {
                MeshTransportError::ProtocolViolation(
                    "sealed frame failed to authenticate (relay tamper or wrong key)".to_string(),
                )
            })?;
        Ok(plain.to_vec())
    }
}

fn nonce_from(seq: u64) -> ring::aead::Nonce {
    let mut bytes = [0u8; 12];
    bytes[4..].copy_from_slice(&seq.to_be_bytes());
    ring::aead::Nonce::assume_unique_for_key(bytes)
}

/// Derive the two directional AEAD keys from the ECDH shared secret. Inputs are
/// passed in initiator-first canonical order so both sides derive identical
/// keys. Returns `(initiator→responder, responder→initiator)`.
fn derive_keys(
    shared: &[u8],
    eph_initiator: &[u8; 32],
    eph_responder: &[u8; 32],
    id_initiator: &str,
    id_responder: &str,
) -> (AeadKey, AeadKey) {
    let salt = ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, b"APEIRETH-MESH-RELAY-KDF-V1");
    let okm = salt.extract(shared);
    let mut info = Vec::with_capacity(128);
    info.extend_from_slice(eph_initiator);
    info.extend_from_slice(eph_responder);
    info.extend_from_slice(&(id_initiator.len() as u32).to_be_bytes());
    info.extend_from_slice(id_initiator.as_bytes());
    info.extend_from_slice(&(id_responder.len() as u32).to_be_bytes());
    info.extend_from_slice(id_responder.as_bytes());

    let mut k_ir = [0u8; 32];
    let mut k_ri = [0u8; 32];
    expand(&okm, &info, &mut k_ir);
    let mut info2 = info.clone();
    info2.push(1);
    expand(&okm, &info2, &mut k_ri);
    (AeadKey::new(k_ir), AeadKey::new(k_ri))
}

fn expand(okm: &ring::hkdf::Prk, info: &[u8], out: &mut [u8; 32]) {
    struct OkmLen;
    impl ring::hkdf::KeyType for OkmLen {
        fn len(&self) -> usize {
            32
        }
    }
    okm.expand(&[info], OkmLen)
        .and_then(|okm| okm.fill(out))
        .expect("hkdf expand");
}

fn as_32(bytes: &[u8]) -> Result<&[u8; 32], MeshTransportError> {
    <&[u8; 32]>::try_from(bytes)
        .map_err(|_| MeshTransportError::ProtocolViolation("expected 32 bytes".to_string()))
}

fn read_u32(bytes: &[u8], off: &mut usize) -> Result<u32, MeshTransportError> {
    if bytes.len() < *off + 4 {
        return Err(MeshTransportError::ProtocolViolation(
            "truncated length".to_string(),
        ));
    }
    let v = u32::from_be_bytes([
        bytes[*off],
        bytes[*off + 1],
        bytes[*off + 2],
        bytes[*off + 3],
    ]);
    *off += 4;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    /// An in-memory opaque tunnel pair that records every frame the relay would
    /// carry — so a test can assert the relay never sees plaintext.
    struct MemTunnel {
        tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        rx: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>,
        closed: Arc<AtomicBool>,
        wire: Arc<Mutex<Vec<u8>>>,
    }

    #[async_trait::async_trait]
    impl OpaqueTunnel for MemTunnel {
        async fn send_opaque(&self, payload: Vec<u8>) -> Result<(), MeshTransportError> {
            self.wire.lock().unwrap().extend_from_slice(&payload);
            let _ = self.tx.send(payload);
            Ok(())
        }
        async fn recv_opaque(&self) -> Result<Vec<u8>, MeshTransportError> {
            let mut rx = self.rx.lock().await;
            rx.recv()
                .await
                .ok_or_else(|| MeshTransportError::Closed("tunnel closed".into()))
        }
        fn close_opaque(&self) {
            self.closed.store(true, Ordering::SeqCst);
        }
        fn is_opaque_closed(&self) -> bool {
            self.closed.load(Ordering::SeqCst)
        }
    }

    fn tunnel_pair() -> (
        Box<dyn OpaqueTunnel>,
        Box<dyn OpaqueTunnel>,
        Arc<Mutex<Vec<u8>>>,
    ) {
        let (tx_a, rx_b) = tokio::sync::mpsc::unbounded_channel();
        let (tx_b, rx_a) = tokio::sync::mpsc::unbounded_channel();
        let closed = Arc::new(AtomicBool::new(false));
        let wire = Arc::new(Mutex::new(Vec::new()));
        let a = MemTunnel {
            tx: tx_a,
            rx: tokio::sync::Mutex::new(rx_a),
            closed: Arc::clone(&closed),
            wire: Arc::clone(&wire),
        };
        let b = MemTunnel {
            tx: tx_b,
            rx: tokio::sync::Mutex::new(rx_b),
            closed,
            wire,
        };
        let wire_log = Arc::clone(&a.wire);
        (Box::new(a), Box::new(b), wire_log)
    }

    fn idkey() -> (Vec<u8>, [u8; 32]) {
        let rng = ring::rand::SystemRandom::new();
        let doc = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8 = doc.as_ref().to_vec();
        let pair = ring::signature::Ed25519KeyPair::from_pkcs8(&pkcs8).unwrap();
        use ring::signature::KeyPair;
        let mut pubk = [0u8; 32];
        pubk.copy_from_slice(pair.public_key().as_ref());
        (pkcs8, pubk)
    }

    /// (A channel, B channel, A pub, B pub, relay-visible wire log).
    async fn established() -> (
        SecureChannel,
        SecureChannel,
        [u8; 32],
        [u8; 32],
        Arc<Mutex<Vec<u8>>>,
    ) {
        let (ta, tb, wire) = tunnel_pair();
        let (a_pk, a_pub) = idkey();
        let (b_pk, b_pub) = idkey();
        let cid = RelayCircuitId::from_bytes([7; 16]);
        let (ca, cb) = tokio::join!(
            handshake(ta, HandshakeRole::Initiator, &a_pk, &a_pub, "node-a", &cid),
            handshake(tb, HandshakeRole::Responder, &b_pk, &b_pub, "node-b", &cid),
        );
        (ca.unwrap(), cb.unwrap(), a_pub, b_pub, wire)
    }

    #[tokio::test]
    async fn each_side_authenticates_the_other_identity_key() {
        let (ca, cb, a_pub, b_pub, _wire) = established().await;
        // A pins B's key; B pins A's key — from the E2E handshake, not relay TLS.
        assert_eq!(ca.peer_fingerprint(), KeyFingerprint::of_public_key(&b_pub));
        assert_eq!(cb.peer_fingerprint(), KeyFingerprint::of_public_key(&a_pub));
        assert_ne!(ca.peer_fingerprint(), cb.peer_fingerprint());
    }

    #[tokio::test]
    async fn sealed_round_trip_and_the_relay_never_sees_plaintext() {
        let (ca, cb, _a, _b, wire) = established().await;
        let secret = b"mesh tool args: {\"password\":\"hunter2\"}";
        ca.send_sealed(secret).await.unwrap();
        let got = cb.recv_sealed().await.unwrap();
        assert_eq!(got, secret);

        let seen = wire.lock().unwrap().clone();
        assert!(
            !seen.windows(secret.len()).any(|w| w == secret),
            "relay-visible wire contained the plaintext"
        );
    }

    #[tokio::test]
    async fn a_tampered_ciphertext_fails_to_authenticate() {
        let (ca, cb, _a, _b, _wire) = established().await;
        // Seal directly (same directional key A→B) and flip a byte, as a
        // malicious relay would. The AEAD open must reject the tampered frame
        // and accept the intact one.
        let mut tampered = ca.send.seal(b"hello").unwrap();
        if let Some(last) = tampered.last_mut() {
            *last ^= 0xff;
        }
        assert!(
            cb.recv.open(&tampered).is_err(),
            "tampered ciphertext must not open"
        );
        let intact = ca.send.seal(b"hello").unwrap();
        assert_eq!(cb.recv.open(&intact).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn a_mitm_relay_cannot_forge_a_hello_signature() {
        // A relay that swaps in its own ephemeral key cannot re-sign it with the
        // node's identity key — the handshake signature check catches it.
        let (eph, _priv) = {
            let rng = ring::rand::SystemRandom::new();
            let e = ring::agreement::EphemeralPrivateKey::generate(&ring::agreement::X25519, &rng)
                .unwrap();
            let pubk = *as_32(e.compute_public_key().unwrap().as_ref()).unwrap();
            (pubk, e)
        };
        let (_attacker_pk, attacker_pub) = idkey();
        let (victim_pk, _victim_pub) = idkey();
        let cid = RelayCircuitId::from_bytes([1; 16]);
        // Attacker signs its own ephemeral key with its own identity…
        let forged = sign_hello(&_attacker_pk, &attacker_pub, "node-b", &eph, &cid).unwrap();
        // …but claims to be the victim's node with the victim's identity key.
        let mut impersonating = forged;
        impersonating.identity_public = {
            let mut v = [0u8; 32];
            let pair = ring::signature::Ed25519KeyPair::from_pkcs8(&victim_pk).unwrap();
            use ring::signature::KeyPair;
            v.copy_from_slice(pair.public_key().as_ref());
            v
        };
        // The signature was made with the attacker's key, so verifying against
        // the (swapped-in) victim identity key fails.
        assert!(impersonating.verify(&cid).is_err());
    }
}
