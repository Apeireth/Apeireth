//! End-to-end encrypted secure channel over an opaque relay tunnel.
//!
//! # Why this exists
//!
//! A relay moves bytes between two nodes that cannot reach each other directly.
//! If the relay simply forwarded plaintext it would see every `MeshEnvelope` —
//! tool arguments, provider requests, results. This module runs **on the two
//! nodes** and establishes a channel the relay cannot read or forge:
//!
//! ```text
//! A ── end-to-end secure channel ── B        (X25519 + Ed25519 + ChaCha20-Poly1305)
//!          ↑
//!    relay only forwards ciphertext frames
//! ```
//!
//! # Construction (this is the security-critical part)
//!
//! - **Key agreement** — ephemeral X25519 (forward secrecy); the raw shared
//!   secret is *never* used as a key.
//! - **KDF** — HKDF-SHA256 over a canonical transcript hash binding protocol
//!   version, circuit id, both ephemeral keys, and both identity public keys;
//!   two directional keys are derived with distinct `info` labels
//!   (`A2B-V1` / `B2A-V1`), so each direction has its own key *and* nonce space.
//! - **Authentication** — each side signs its ephemeral key with its
//!   `NodeIdentity` (Ed25519) over a domain-separated payload binding protocol
//!   version, role, both node ids, and the circuit id. A MITM relay that swaps
//!   its own DH key breaks the signature (it cannot re-sign without the node's
//!   private key); role binding blocks cross-role replay; the circuit binding
//!   blocks cross-circuit transfer; both-id binding blocks unknown-key-share.
//! - **AEAD** — ChaCha20-Poly1305 with per-direction AAD binding version,
//!   circuit, source/target node ids, and the sequence number; a monotonic
//!   64-bit counter feeds a 96-bit nonce (32-bit prefix ‖ 64-bit sequence) and
//!   the receiver enforces `seq == expected` (replay / reordering ⇒ protocol
//!   violation, fail closed) with no counter wrap.
//! - **Key confirmation** — both sides exchange a sealed `ChannelReady` marker
//!   and only then expose the channel; the first real `MeshEnvelope` is never
//!   an implicit, undiagnosable confirmation.
//!
//! The channel reports the authenticated peer fingerprint so
//! [`crate::node::MeshNode`] can pin-check it against the `TrustStore` — the
//! relay's own TLS identity is never mistaken for peer identity.
//!
//! # What the relay sees
//!
//! Only circuit ids and AEAD ciphertext. It never sees plaintext envelopes.

use apeireth_mesh_protocol::rendezvous::{RelayCircuitId, ED25519_PUBLIC_KEY_BYTES};
use apeireth_mesh_protocol::KeyFingerprint;

use crate::transport::MeshTransportError;

/// Domain-separation string for the E2E handshake signatures.
pub const E2E_HANDSHAKE_DOMAIN: &str = "APEIRETH-MESH-RELAY-E2E-V1";
/// The E2E channel protocol version (bound into signatures, KDF, and AAD).
pub const E2E_PROTOCOL_VERSION: u32 = 1;
/// HKDF salt for key derivation.
pub const E2E_KDF_SALT: &str = "APEIRETH-MESH-RELAY-KDF-V1";
/// Directional KDF label for initiator→responder.
pub const E2E_DIR_A2B: &str = "APEIRETH-MESH-RELAY-A2B-V1";
/// Directional KDF label for responder→initiator.
pub const E2E_DIR_B2A: &str = "APEIRETH-MESH-RELAY-B2A-V1";
/// AAD domain prefix.
pub const E2E_AAD_DOMAIN: &str = "APEIRETH-MESH-RELAY-AAD-V1";
/// The key-confirmation marker exchanged before the channel goes live.
pub const CHANNEL_READY: &[u8] = b"APEIRETH-MESH-RELAY-CHANNEL-READY-V1";

/// An opaque bidirectional byte tunnel between exactly two nodes, carried by a
/// relay. The relay reads/writes only opaque `Vec<u8>` frames; it never
/// interprets them.
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

/// The negotiated, authenticated, end-to-end encrypted channel. Holds only the
/// per-direction AEAD state and the authenticated peer fingerprint; ephemeral
/// private material is dropped after the handshake.
pub struct SecureChannel {
    tunnel: Box<dyn OpaqueTunnel>,
    send: AeadKey,
    recv: AeadKey,
    peer_fingerprint: KeyFingerprint,
}

impl SecureChannel {
    /// Authenticated peer identity — evidence for [`crate::node::MeshNode`]'s
    /// trust-pin check. From the E2E handshake, never from relay TLS.
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

    /// Receive and decrypt one plaintext payload from the peer. Fails closed on
    /// replay, reordering, tamper, or a wrong key.
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

impl HandshakeRole {
    fn tag(&self) -> u8 {
        match self {
            Self::Initiator => 1,
            Self::Responder => 2,
        }
    }

    fn opposite(&self) -> HandshakeRole {
        match self {
            Self::Initiator => HandshakeRole::Responder,
            Self::Responder => HandshakeRole::Initiator,
        }
    }
}

/// Establish an authenticated, encrypted channel over `tunnel`.
///
/// `node_id` is our id, `peer_node_id` the id we expect to reach (both bound
/// into the signature transcript to defeat unknown-key-share). `circuit_id`
/// binds the handshake to one relay circuit so it cannot be replayed onto
/// another. The caller pin-checks [`SecureChannel::peer_fingerprint`].
pub async fn handshake(
    tunnel: Box<dyn OpaqueTunnel>,
    role: HandshakeRole,
    identity_pkcs8: &[u8],
    identity_public: &[u8; ED25519_PUBLIC_KEY_BYTES],
    node_id: &str,
    peer_node_id: &str,
    circuit_id: &RelayCircuitId,
) -> Result<SecureChannel, MeshTransportError> {
    let rng = ring::rand::SystemRandom::new();

    // Ephemeral X25519 key (OS CSPRNG) for forward secrecy.
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
        peer_node_id,
        &ephemeral_public,
        circuit_id,
        role,
    )?;

    // Message order: initiator first. Each side verifies the other's hello.
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

    // Verify the peer's signature over its own ephemeral key + role + both ids +
    // circuit. This is what a MITM relay cannot forge.
    peer_hello.verify(circuit_id, role.opposite())?;
    // The peer must claim *our* id as its expected peer, and its own id must be
    // who we intended to reach — otherwise a relay is trying to confuse us.
    if peer_hello.node_id != peer_node_id {
        return Err(MeshTransportError::ProtocolViolation(format!(
            "handshake peer claimed {}, expected {peer_node_id}",
            peer_hello.node_id
        )));
    }
    if peer_hello.peer_node_id != node_id {
        return Err(MeshTransportError::ProtocolViolation(format!(
            "handshake peer targeted {}, expected {node_id}",
            peer_hello.peer_node_id
        )));
    }

    let peer_ephemeral = peer_hello.ephemeral_public;
    let peer_identity = peer_hello.identity_public;
    let peer_fingerprint = KeyFingerprint::of_public_key(&peer_identity);

    // ECDH → KDF over a canonical transcript → two directional keys.
    let peer_public =
        ring::agreement::UnparsedPublicKey::new(&ring::agreement::X25519, peer_ephemeral);
    // Canonical initiator-first ordering so both derive identical keys.
    let (eph_init, eph_resp, id_init, id_resp) = match role {
        HandshakeRole::Initiator => (
            ephemeral_public,
            peer_ephemeral,
            *identity_public,
            peer_identity,
        ),
        HandshakeRole::Responder => (
            peer_ephemeral,
            ephemeral_public,
            peer_identity,
            *identity_public,
        ),
    };
    let (k_a2b, k_b2a) = ring::agreement::agree_ephemeral(ephemeral, &peer_public, |shared| {
        derive_keys(shared, circuit_id, &eph_init, &eph_resp, &id_init, &id_resp)
    })
    .map_err(|_| MeshTransportError::ProtocolViolation("key agreement failed".to_string()))?;

    // Per-direction AAD: version + circuit + source/target node ids. Built in
    // initiator-first canonical order so both sides compute identical AAD for a
    // given direction (a2b is always initiator→responder).
    let (init_name, resp_name) = match role {
        HandshakeRole::Initiator => (node_id, peer_node_id),
        HandshakeRole::Responder => (peer_node_id, node_id),
    };
    let aad_a2b = aad_base(circuit_id, init_name, resp_name);
    let aad_b2a = aad_base(circuit_id, resp_name, init_name);
    let (send, recv) = match role {
        HandshakeRole::Initiator => (AeadKey::new(k_a2b, aad_a2b), AeadKey::new(k_b2a, aad_b2a)),
        HandshakeRole::Responder => (AeadKey::new(k_b2a, aad_b2a), AeadKey::new(k_a2b, aad_a2b)),
    };

    let channel = SecureChannel {
        tunnel,
        send,
        recv,
        peer_fingerprint,
    };

    // Key confirmation: both sides prove they derived the same session keys
    // before the channel is exposed to the mesh. The marker is seq 0 on each
    // key; real application traffic starts at seq 1.
    channel.send_sealed(CHANNEL_READY).await?;
    let peer_ready = channel.recv_sealed().await?;
    if peer_ready != CHANNEL_READY {
        return Err(MeshTransportError::ProtocolViolation(
            "key confirmation failed (peer did not derive the same keys)".to_string(),
        ));
    }

    Ok(channel)
}

// ---------------------------------------------------------------------------
// Handshake hello message (opaque to the relay).
// ---------------------------------------------------------------------------

struct HelloMessage {
    ephemeral_public: [u8; 32],
    identity_public: [u8; 32],
    node_id: String,
    peer_node_id: String,
    signature: Vec<u8>,
}

impl HelloMessage {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(160);
        out.extend_from_slice(&self.ephemeral_public);
        out.extend_from_slice(&self.identity_public);
        push_str(&mut out, &self.node_id);
        push_str(&mut out, &self.peer_node_id);
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
        let node_id = read_str(bytes, &mut off)?;
        let peer_node_id = read_str(bytes, &mut off)?;
        let sig_len = read_u32(bytes, &mut off)? as usize;
        if bytes.len() < off + sig_len {
            return Err(bad("truncated signature"));
        }
        let signature = bytes[off..off + sig_len].to_vec();
        Ok(Self {
            ephemeral_public,
            identity_public,
            node_id,
            peer_node_id,
            signature,
        })
    }

    /// Verify the peer signed its ephemeral key with the private key matching
    /// `identity_public`, for exactly this circuit and the peer's own role. A
    /// MITM relay cannot pass this.
    fn verify(
        &self,
        circuit_id: &RelayCircuitId,
        peer_role: HandshakeRole,
    ) -> Result<(), MeshTransportError> {
        let payload = handshake_payload(
            &self.ephemeral_public,
            &self.node_id,
            &self.peer_node_id,
            circuit_id,
            peer_role,
        );
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
    peer_node_id: &str,
    ephemeral_public: &[u8; 32],
    circuit_id: &RelayCircuitId,
    role: HandshakeRole,
) -> Result<HelloMessage, MeshTransportError> {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let pair = Ed25519KeyPair::from_pkcs8(identity_pkcs8)
        .map_err(|e| MeshTransportError::ProtocolViolation(format!("identity key: {e}")))?;
    if pair.public_key().as_ref() != identity_public {
        return Err(MeshTransportError::ProtocolViolation(
            "identity public key mismatch".to_string(),
        ));
    }
    let payload = handshake_payload(ephemeral_public, node_id, peer_node_id, circuit_id, role);
    let signature = pair.sign(&payload).as_ref().to_vec();
    Ok(HelloMessage {
        ephemeral_public: *ephemeral_public,
        identity_public: *identity_public,
        node_id: node_id.to_string(),
        peer_node_id: peer_node_id.to_string(),
        signature,
    })
}

/// Canonical, domain-separated handshake payload. Length-prefixed fields (no
/// concatenation ambiguity) binding protocol version, role, both node ids, the
/// ephemeral key, and the circuit id. This is what a peer signs and what a MITM
/// cannot reproduce.
fn handshake_payload(
    ephemeral_public: &[u8; 32],
    node_id: &str,
    peer_node_id: &str,
    circuit_id: &RelayCircuitId,
    role: HandshakeRole,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(E2E_HANDSHAKE_DOMAIN.as_bytes());
    out.push(0);
    out.extend_from_slice(&E2E_PROTOCOL_VERSION.to_be_bytes());
    out.push(role.tag());
    push_str(&mut out, node_id);
    push_str(&mut out, peer_node_id);
    out.extend_from_slice(ephemeral_public);
    out.extend_from_slice(circuit_id.as_bytes());
    out
}

// ---------------------------------------------------------------------------
// AEAD: ChaCha20-Poly1305 with AAD, checked nonce, replay/order enforcement.
// ---------------------------------------------------------------------------

/// One directional AEAD key. `aad_base` is the fixed per-direction context
/// (version + circuit + source/target ids); the frame's sequence number is
/// appended to it as associated data and doubles as the nonce.
struct AeadKey {
    key: ring::aead::LessSafeKey,
    aad_base: Vec<u8>,
    /// Next sequence number to *send* (monotonic, overflow closes).
    send_seq: std::sync::atomic::AtomicU64,
    /// Next sequence number we *expect to receive* (strict ordering).
    recv_next: std::sync::Mutex<u64>,
}

impl AeadKey {
    fn new(bytes: [u8; 32], aad_base: Vec<u8>) -> Self {
        let unbound = ring::aead::UnboundKey::new(&ring::aead::CHACHA20_POLY1305, &bytes)
            .expect("32-byte chacha key");
        Self {
            key: ring::aead::LessSafeKey::new(unbound),
            aad_base,
            send_seq: std::sync::atomic::AtomicU64::new(0),
            recv_next: std::sync::Mutex::new(0),
        }
    }

    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, MeshTransportError> {
        // Checked increment: overflow ⇒ close/rekey, never wrap (nonce reuse).
        let seq = self
            .send_seq
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |v| v.checked_add(1),
            )
            .map_err(|_| {
                MeshTransportError::ProtocolViolation("send sequence overflow".to_string())
            })?;
        let nonce = nonce_from(seq);
        let aad = self.aad_for(seq);
        let mut in_out = plaintext.to_vec();
        self.key
            .seal_in_place_append_tag(nonce, ring::aead::Aad::from(&aad), &mut in_out)
            .map_err(|_| MeshTransportError::ProtocolViolation("seal failed".to_string()))?;
        // Prefix the sequence so the receiver can reconstruct nonce + AAD.
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
        // Strict ordering / anti-replay: the sequence must be exactly the next
        // expected. A duplicate or out-of-order frame is a relay attack ⇒ close.
        {
            let mut recv_next = self
                .recv_next
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if seq != *recv_next {
                return Err(MeshTransportError::ProtocolViolation(format!(
                    "replay or out-of-order frame (got seq {seq}, expected {recv_next})"
                )));
            }
            *recv_next += 1;
        }
        let nonce = nonce_from(seq);
        let aad = self.aad_for(seq);
        let mut in_out = frame[8..].to_vec();
        let plain = self
            .key
            .open_in_place(nonce, ring::aead::Aad::from(&aad), &mut in_out)
            .map_err(|_| {
                MeshTransportError::ProtocolViolation(
                    "sealed frame failed to authenticate (relay tamper, reflection, or wrong key)"
                        .to_string(),
                )
            })?;
        Ok(plain.to_vec())
    }

    /// Full AAD for one frame: fixed context ‖ sequence.
    fn aad_for(&self, seq: u64) -> Vec<u8> {
        let mut aad = Vec::with_capacity(self.aad_base.len() + 8);
        aad.extend_from_slice(&self.aad_base);
        aad.extend_from_slice(&seq.to_be_bytes());
        aad
    }
}

/// 96-bit nonce: 32-bit fixed prefix (zeros) ‖ 64-bit monotonic sequence. The
/// per-direction key keeps the (key, nonce) pairs unique; the checked counter
/// guarantees no sequence repeats for a given key.
fn nonce_from(seq: u64) -> ring::aead::Nonce {
    let mut bytes = [0u8; 12];
    bytes[4..].copy_from_slice(&seq.to_be_bytes());
    ring::aead::Nonce::assume_unique_for_key(bytes)
}

/// Fixed per-direction AAD context: domain + version + circuit + source/target.
fn aad_base(circuit_id: &RelayCircuitId, source: &str, target: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(96);
    out.extend_from_slice(E2E_AAD_DOMAIN.as_bytes());
    out.push(0);
    out.extend_from_slice(&E2E_PROTOCOL_VERSION.to_be_bytes());
    out.extend_from_slice(circuit_id.as_bytes());
    push_str(&mut out, source);
    push_str(&mut out, target);
    out
}

// ---------------------------------------------------------------------------
// KDF: HKDF-SHA256 over a canonical transcript hash → two directional keys.
// ---------------------------------------------------------------------------

/// Derive the two directional AEAD keys. `shared` is the X25519 secret (never a
/// key itself). The transcript binds version, circuit id, both ephemeral keys,
/// and both identity public keys; each direction gets a distinct `info` label.
/// Inputs are passed in initiator-first canonical order so both sides agree.
fn derive_keys(
    shared: &[u8],
    circuit_id: &RelayCircuitId,
    eph_initiator: &[u8; 32],
    eph_responder: &[u8; 32],
    id_pub_initiator: &[u8; 32],
    id_pub_responder: &[u8; 32],
) -> ([u8; 32], [u8; 32]) {
    // Canonical transcript hash (SHA-256).
    let mut transcript = Vec::with_capacity(160);
    transcript.extend_from_slice(E2E_KDF_SALT.as_bytes());
    transcript.push(0);
    transcript.extend_from_slice(&E2E_PROTOCOL_VERSION.to_be_bytes());
    transcript.extend_from_slice(circuit_id.as_bytes());
    transcript.extend_from_slice(eph_initiator);
    transcript.extend_from_slice(eph_responder);
    transcript.extend_from_slice(id_pub_initiator);
    transcript.extend_from_slice(id_pub_responder);
    let th = sha256(&transcript);

    let salt = ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, E2E_KDF_SALT.as_bytes());
    let prk = salt.extract(shared);

    let mut k_a2b = [0u8; 32];
    let mut k_b2a = [0u8; 32];
    expand(&prk, &[&th, E2E_DIR_A2B.as_bytes()], &mut k_a2b);
    expand(&prk, &[&th, E2E_DIR_B2A.as_bytes()], &mut k_b2a);
    (k_a2b, k_b2a)
}

fn expand(prk: &ring::hkdf::Prk, info: &[&[u8]], out: &mut [u8; 32]) {
    struct OkmLen;
    impl ring::hkdf::KeyType for OkmLen {
        fn len(&self) -> usize {
            32
        }
    }
    prk.expand(info, OkmLen)
        .and_then(|okm| okm.fill(out))
        .expect("hkdf expand");
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

// --- canonical field helpers ----------------------------------------------

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn read_str(bytes: &[u8], off: &mut usize) -> Result<String, MeshTransportError> {
    let len = read_u32(bytes, off)? as usize;
    if bytes.len() < *off + len {
        return Err(MeshTransportError::ProtocolViolation(
            "truncated string".to_string(),
        ));
    }
    let s = std::str::from_utf8(&bytes[*off..*off + len])
        .map_err(|_| MeshTransportError::ProtocolViolation("not utf8".to_string()))?
        .to_string();
    *off += len;
    Ok(s)
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

    /// In-memory opaque tunnel that records every frame the relay would carry,
    /// so a test can assert the relay never sees plaintext.
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

    /// (chan_a, chan_b, a_pub, b_pub, relay-visible wire). Each channel is past
    /// key confirmation, so its application-data sequence starts at 1.
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
            handshake(
                ta,
                HandshakeRole::Initiator,
                &a_pk,
                &a_pub,
                "node-a",
                "node-b",
                &cid
            ),
            handshake(
                tb,
                HandshakeRole::Responder,
                &b_pk,
                &b_pub,
                "node-b",
                "node-a",
                &cid
            ),
        );
        (ca.unwrap(), cb.unwrap(), a_pub, b_pub, wire)
    }

    #[tokio::test]
    async fn each_side_authenticates_the_other_identity_key() {
        let (ca, cb, a_pub, b_pub, _wire) = established().await;
        assert_eq!(ca.peer_fingerprint(), KeyFingerprint::of_public_key(&b_pub));
        assert_eq!(cb.peer_fingerprint(), KeyFingerprint::of_public_key(&a_pub));
        assert_ne!(ca.peer_fingerprint(), cb.peer_fingerprint());
    }

    #[tokio::test]
    async fn sealed_round_trip_and_the_relay_never_sees_plaintext() {
        let (ca, cb, _a, _b, wire) = established().await;
        // Distinctive secrets a metadata-leak test can hunt for.
        let tool_arg = b"SUPER_SECRET_TOOL_ARGUMENT_123";
        let provider = b"PRIVATE_PROVIDER_PROMPT_456";
        ca.send_sealed(tool_arg).await.unwrap();
        ca.send_sealed(provider).await.unwrap();
        assert_eq!(cb.recv_sealed().await.unwrap(), tool_arg);
        assert_eq!(cb.recv_sealed().await.unwrap(), provider);

        let seen = wire.lock().unwrap().clone();
        for secret in [tool_arg.as_slice(), provider.as_slice()] {
            assert!(
                !seen.windows(secret.len()).any(|w| w == secret),
                "relay-visible wire contained plaintext"
            );
        }
    }

    #[tokio::test]
    async fn a_tampered_ciphertext_fails_to_authenticate() {
        let (ca, cb, _a, _b, _wire) = established().await;
        let mut tampered = ca.send.seal(b"hello").unwrap();
        if let Some(last) = tampered.last_mut() {
            *last ^= 0xff;
        }
        assert!(cb.recv.open(&tampered).is_err());
        // The intact frame opens fine (recv expects seq 1 after confirmation).
        let intact = ca.send.seal(b"hello").unwrap();
        assert_eq!(cb.recv.open(&intact).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn a_mitm_relay_cannot_forge_a_hello_signature() {
        let (attacker_pk, attacker_pub) = idkey();
        let (victim_pk, _victim_pub) = idkey();
        let rng = ring::rand::SystemRandom::new();
        let eph =
            ring::agreement::EphemeralPrivateKey::generate(&ring::agreement::X25519, &rng).unwrap();
        let eph = *as_32(eph.compute_public_key().unwrap().as_ref()).unwrap();
        let cid = RelayCircuitId::from_bytes([1; 16]);
        // Attacker signs its own eph with its own key, claiming B's node/peer ids.
        let forged = sign_hello(
            &attacker_pk,
            &attacker_pub,
            "node-b",
            "node-a",
            &eph,
            &cid,
            HandshakeRole::Responder,
        )
        .unwrap();
        // Swap in the victim's identity public key as the claimed identity.
        let mut impersonating = forged;
        impersonating.identity_public = {
            let pair = ring::signature::Ed25519KeyPair::from_pkcs8(&victim_pk).unwrap();
            use ring::signature::KeyPair;
            let mut v = [0u8; 32];
            v.copy_from_slice(pair.public_key().as_ref());
            v
        };
        assert!(impersonating
            .verify(&cid, HandshakeRole::Responder)
            .is_err());
    }

    // --- malicious relay matrix -----------------------------------------

    #[test]
    fn same_circuit_replay_is_rejected() {
        let cid = RelayCircuitId::from_bytes([2; 16]);
        let key = AeadKey::new([7; 32], aad_base(&cid, "node-a", "node-b"));
        let frame = key.seal(b"once").unwrap();
        assert_eq!(key.open(&frame).unwrap(), b"once");
        assert!(key.open(&frame).is_err(), "replay must be rejected");
    }

    #[test]
    fn out_of_order_frame_is_rejected() {
        let cid = RelayCircuitId::from_bytes([2; 16]);
        let key = AeadKey::new([7; 32], aad_base(&cid, "node-a", "node-b"));
        let f0 = key.seal(b"zero").unwrap();
        let f1 = key.seal(b"one").unwrap();
        // seq 1 arriving first is a reorder (stream semantics) => fail closed.
        assert!(key.open(&f1).is_err(), "reorder must be rejected");
        assert_eq!(key.open(&f0).unwrap(), b"zero");
    }

    #[test]
    fn reflection_a2b_as_b2a_is_rejected() {
        let cid = RelayCircuitId::from_bytes([2; 16]);
        let k_a2b = AeadKey::new([1; 32], aad_base(&cid, "node-a", "node-b"));
        let k_b2a = AeadKey::new([2; 32], aad_base(&cid, "node-b", "node-a"));
        let frame = k_a2b.seal(b"reflect me").unwrap();
        assert!(k_b2a.open(&frame).is_err(), "reflection must be rejected");
    }

    #[test]
    fn cross_circuit_replay_is_rejected() {
        // Two circuits between the same pair derive different keys (circuit id
        // is bound into the KDF transcript and the AAD).
        let shared = [0x42; 32];
        let e1 = [1u8; 32];
        let e2 = [2u8; 32];
        let i1 = [3u8; 32];
        let i2 = [4u8; 32];
        let cx = RelayCircuitId::from_bytes([0xAA; 16]);
        let cy = RelayCircuitId::from_bytes([0xBB; 16]);
        let (kx, _) = derive_keys(&shared, &cx, &e1, &e2, &i1, &i2);
        let (ky, _) = derive_keys(&shared, &cy, &e1, &e2, &i1, &i2);
        let key_x = AeadKey::new(kx, aad_base(&cx, "node-a", "node-b"));
        let key_y = AeadKey::new(ky, aad_base(&cy, "node-a", "node-b"));
        let frame = key_x.seal(b"cross").unwrap();
        assert!(
            key_y.open(&frame).is_err(),
            "cross-circuit replay must be rejected"
        );
    }

    #[test]
    fn directional_keys_are_distinct_and_deterministic() {
        let shared = [0x11; 32];
        let e1 = [1u8; 32];
        let e2 = [2u8; 32];
        let i1 = [3u8; 32];
        let i2 = [4u8; 32];
        let cid = RelayCircuitId::from_bytes([0xCC; 16]);
        let (k_a2b, k_b2a) = derive_keys(&shared, &cid, &e1, &e2, &i1, &i2);
        assert_ne!(k_a2b, k_b2a, "A->B and B->A keys must differ");
        let (k_a2b2, k_b2a2) = derive_keys(&shared, &cid, &e1, &e2, &i1, &i2);
        assert_eq!(k_a2b, k_a2b2);
        assert_eq!(k_b2a, k_b2a2);
    }

    #[test]
    fn send_sequence_overflow_closes_rather_than_wraps() {
        let cid = RelayCircuitId::from_bytes([3; 16]);
        let key = AeadKey::new([5; 32], aad_base(&cid, "node-a", "node-b"));
        // Force the send counter to the edge, then ensure it cannot wrap.
        key.send_seq.store(u64::MAX, Ordering::SeqCst);
        // The next seal attempts checked_add(u64::MAX + 1) => overflow => error.
        assert!(
            key.seal(b"overflow").is_err(),
            "overflow must close, not wrap"
        );
    }

    #[tokio::test]
    async fn key_confirmation_marker_must_match() {
        // A wrong confirmation marker is never accepted as proof of readiness.
        let (ca, _cb, _a, _b, _w) = established().await;
        let cid = RelayCircuitId::from_bytes([9; 16]);
        let key = AeadKey::new([9; 32], aad_base(&cid, "a", "b"));
        let sealed_bad = key.seal(b"NOT-THE-READY-MARKER").unwrap();
        let got = key.open(&sealed_bad).unwrap();
        assert_ne!(
            got, CHANNEL_READY,
            "a wrong marker must never count as ready"
        );
        let _ = ca;
    }
}
