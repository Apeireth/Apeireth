//! Mesh Phase 2C1 wire contract: WAN rendezvous presence and encrypted-relay
//! circuit control. Pure data + the signatures that give it integrity — no
//! sockets, no async runtime, no modules, no trust decisions.
//!
//! # Two questions, two vocabularies
//!
//! This module answers only the *infrastructure* questions:
//!
//! - **Rendezvous** — "where is Node B currently reachable?" ([`PresenceRecord`],
//!   [`RendezvousRequest`] / [`RendezvousResponse`]).
//! - **Relay** — "forward opaque bytes between A and B" ([`RelayCircuitId`],
//!   [`RelayControl`], [`RelayFrame`]).
//!
//! Neither answers *who to trust*. A presence record is a signed, expiring,
//! self-describing reachability claim — never a trust grant. The canonical mesh
//! vocabulary (`InvokeCapability`, `CapabilityResult`, adverts) lives in
//! [`crate::envelope`] and is deliberately kept out of this state machine: the
//! rendezvous/relay server must never parse, route, or cache any of it.
//!
//! # What the relay server is allowed to see
//!
//! Opaque ciphertext frames ([`RelayFrame`]) and the minimal routing metadata
//! needed to forward them (circuit id). It can never decrypt the inner
//! `MeshEnvelope`: that is end-to-end encrypted by the two nodes over the
//! circuit (see `apeireth-mesh`'s relay secure channel). Presence is minimized
//! to identity + reachability hints — no capabilities, no tools, no models, no
//! credentials, no prompts.

use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};

pub use apeireth_core::kernel::NodeId;

/// Domain-separation string for presence signatures.
///
/// Every signed presence payload is prefixed with this so a signature produced
/// for one purpose (or one server) can never be replayed as another. Binding the
/// audience into the signed payload is what stops a malicious rendezvous server
/// from replaying a presence captured against server X to redirect a client
/// talking to server Y.
pub const PRESENCE_DOMAIN: &str = "APEIRETH-MESH-PRESENCE-V2";

/// The rendezvous/relay wire protocol version.
///
/// This numbers the *control* protocol (register / heartbeat / lookup /
/// withdraw and the relay circuit control), not the shape of a
/// [`PresenceRecord`]. Adding signed endpoint candidates changed the record's
/// signed payload, which [`PRESENCE_DOMAIN`] versions instead — so this stays
/// at 1 while a version 1 presence record can no longer verify at all.
pub const RENDEZVOUS_PROTOCOL_VERSION: u32 = 1;

/// Maximum endpoint candidates one presence record may carry.
///
/// Small and fixed on purpose: a candidate list is a routing hint, and an
/// unbounded one would turn every lookup into an allocation and amplification
/// vector.
pub const PRESENCE_MAX_CANDIDATES: usize = 8;

/// Shortest accepted presence TTL (milliseconds). Presence is a *lease*, not a
/// registration: the shortest useful window that still tolerates a heartbeat.
pub const PRESENCE_MIN_TTL_MS: u64 = 30_000;

/// Longest accepted presence TTL (milliseconds). A server must never treat a
/// node as online beyond this without a fresh signed heartbeat.
pub const PRESENCE_MAX_TTL_MS: u64 = 120_000;

/// Default presence TTL when a client expresses no preference.
pub const PRESENCE_DEFAULT_TTL_MS: u64 = 60_000;

/// Length of a [`RelayCircuitId`] in bytes (128 bits, random).
pub const RELAY_CIRCUIT_ID_BYTES: usize = 16;

/// Length of a presence nonce in bytes (128 bits, random).
pub const PRESENCE_NONCE_BYTES: usize = 16;

/// Length of an Ed25519 public key in bytes.
pub const ED25519_PUBLIC_KEY_BYTES: usize = 32;

/// Errors produced while building or verifying rendezvous/relay wire data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RendezvousError {
    /// A signature did not verify against the embedded public key.
    #[error("presence signature is invalid")]
    BadSignature,
    /// The presence record's own fields are inconsistent (wrong key length,
    /// malformed ids).
    #[error("presence record is malformed: {0}")]
    Malformed(String),
    /// The presence lease has expired and must be ignored / renewed.
    #[error("presence record expired at {expires_at_ms}")]
    Expired { expires_at_ms: u64 },
    /// The presence was signed for a different rendezvous audience.
    #[error("presence audience mismatch: expected {expected}, got {got}")]
    AudienceMismatch { expected: String, got: String },
    /// A nonce or session id was reused after expiry (replay).
    #[error("presence replay detected")]
    Replay,
}

/// A random, opaque relay circuit identifier.
///
/// Purely a *transport* name for one A↔B forwarding path. It is never an
/// identity: a circuit id cannot be presented as a [`NodeId`], cannot be pinned,
/// and carries no authority. Kept as fixed raw bytes so it round-trips on the
/// wire without ambiguity and is cheap to compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelayCircuitId([u8; RELAY_CIRCUIT_ID_BYTES]);

impl RelayCircuitId {
    /// Mint a circuit id from explicit bytes (tests / framing).
    pub fn from_bytes(bytes: [u8; RELAY_CIRCUIT_ID_BYTES]) -> Self {
        Self(bytes)
    }

    /// Mint a fresh random circuit id from a caller-supplied RNG byte slice.
    ///
    /// Takes the randomness as an argument so this pure-data crate never has to
    /// reach for an OS RNG (the caller uses `ring::rand`).
    pub fn from_random(random: [u8; RELAY_CIRCUIT_ID_BYTES]) -> Self {
        Self(random)
    }

    /// The raw bytes.
    pub fn as_bytes(&self) -> &[u8; RELAY_CIRCUIT_ID_BYTES] {
        &self.0
    }

    /// Lowercase hex form (for logs and endpoint strings; never an identity).
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for RelayCircuitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// A signed, expiring, self-describing reachability claim for one node.
///
/// This is the *entire* answer to "where is Node B right now" — and it is
/// explicitly **not** a trust grant. A client that receives a presence record
/// must still (a) verify this signature against the embedded public key, and
/// (b) check that the fingerprint matches the pin in its local `TrustStore`.
/// Presence can never add, remove, or alter a trust pin.
///
/// # Minimized
///
/// The record carries identity + reachability hints only. No capabilities, no
/// tools, no models, no memory, no session/user data, no credentials, no agent
/// metadata. The rendezvous server learns *that* a node is online and *how* to
/// reach it, and nothing about what it can do.
/// How a candidate address was learned.
///
/// The kind is part of the *signed* claim, so a peer cannot relabel an address
/// after the fact. Relay reachability is deliberately **not** a candidate kind:
/// a relay is an intermediary with its own identity and allocation metadata,
/// not an address the destination answers at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointCandidateKind {
    /// An address on a local interface, as this node sees it.
    Host,
    /// An address an outside observer saw: this node's NAT mapping.
    ServerReflexive,
}

impl EndpointCandidateKind {
    /// Stable wire discriminant. These values are part of the signed payload
    /// and must never be reordered.
    pub fn wire_tag(self) -> u8 {
        match self {
            Self::Host => 0,
            Self::ServerReflexive => 1,
        }
    }

    /// Dial preference: a host address is the cheapest and needs no NAT
    /// traversal, so it is always tried before a server-reflexive one.
    pub fn preference(self) -> u8 {
        self.wire_tag()
    }
}

/// One place a peer claims it might be reachable.
///
/// A **routing hint**, never an identity: it says *where to dial*, not *who is
/// there*. Whatever answers at this address must still prove the pinned key
/// during the transport handshake, so a candidate can only ever cause a failed
/// attempt, never a wrong peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointCandidate {
    /// How this address was learned.
    pub kind: EndpointCandidateKind,
    /// The address to dial.
    pub addr: SocketAddr,
}

impl EndpointCandidate {
    /// Build a candidate, rejecting an address that must never be dialled.
    pub fn new(kind: EndpointCandidateKind, addr: SocketAddr) -> Result<Self, RendezvousError> {
        let candidate = Self { kind, addr };
        candidate.validate()?;
        Ok(candidate)
    }

    /// Reject anything that must never be dialled.
    ///
    /// Unspecified, multicast and IPv4-broadcast addresses are refused for
    /// every kind. Loopback, private and link-local addresses are **allowed**:
    /// a host candidate is exactly "the LAN address I can be reached at", and
    /// refusing private space would break the case candidates exist for.
    pub fn validate(&self) -> Result<(), RendezvousError> {
        if self.addr.port() == 0 {
            return Err(RendezvousError::Malformed(
                "candidate port must not be zero".to_string(),
            ));
        }
        let dialable = match self.addr.ip() {
            IpAddr::V4(v4) => !(v4.is_unspecified() || v4.is_multicast() || v4.is_broadcast()),
            IpAddr::V6(v6) => !(v6.is_unspecified() || v6.is_multicast()),
        };
        if !dialable {
            return Err(RendezvousError::Malformed(
                "candidate address is not dialable".to_string(),
            ));
        }
        Ok(())
    }

    /// Total canonical order: kind preference, address family, address bytes,
    /// then port.
    ///
    /// Deliberately independent of `SocketAddr`'s own ordering so the signed
    /// candidate order can never drift when the standard library changes.
    pub fn sort_key(&self) -> (u8, u8, [u8; 16], u16) {
        match self.addr {
            SocketAddr::V4(v4) => {
                let mut octets = [0u8; 16];
                octets[..4].copy_from_slice(&v4.ip().octets());
                (self.kind.preference(), 4, octets, v4.port())
            }
            SocketAddr::V6(v6) => (self.kind.preference(), 6, v6.ip().octets(), v6.port()),
        }
    }
}

/// Validate, sort and de-duplicate candidates into their canonical order.
///
/// The signer calls this so a record has exactly one encoding, and
/// [`PresenceRecord::verify`] requires the result so nobody can present a
/// re-ordered or padded list under a valid signature.
pub fn canonicalize_candidates(
    candidates: Vec<EndpointCandidate>,
) -> Result<Vec<EndpointCandidate>, RendezvousError> {
    if candidates.len() > PRESENCE_MAX_CANDIDATES {
        return Err(RendezvousError::Malformed(
            "too many endpoint candidates".to_string(),
        ));
    }
    for candidate in &candidates {
        candidate.validate()?;
    }
    let mut out = candidates;
    out.sort_by_key(EndpointCandidate::sort_key);
    out.dedup();
    Ok(out)
}

/// Append a socket address in the canonical **binary** form the presence
/// signature covers.
///
/// Family tag, exact-width address bytes, fixed-width port. Deliberately not
/// the textual form: `::1` and `0:0:0:0:0:0:0:1` are the same address, and a
/// text encoding would let two encodings of one claim both verify.
fn push_socket_addr(out: &mut Vec<u8>, addr: SocketAddr) {
    match addr {
        SocketAddr::V4(v4) => {
            out.push(4);
            out.extend_from_slice(&v4.ip().octets());
        }
        SocketAddr::V6(v6) => {
            out.push(6);
            out.extend_from_slice(&v6.ip().octets());
        }
    }
    out.extend_from_slice(&addr.port().to_be_bytes());
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceRecord {
    /// Canonical node identity. Stable across restarts and network moves.
    pub node_id: NodeId,
    /// Raw Ed25519 public key (32 bytes) — the same key that authenticates the
    /// mesh channel. Its SHA-256 is the fingerprint a peer pins.
    pub public_key: Vec<u8>,
    /// Ephemeral presence session id (random per registration). Distinguishes
    /// successive leases from the same node so a stale one cannot linger.
    /// `u64` (not `u128`) so it round-trips through serde_json's internally
    /// tagged enum framing.
    pub session_id: u64,
    /// Absolute expiry, epoch milliseconds. Short lease; renewed by heartbeat.
    pub expires_at_ms: u64,
    /// The rendezvous/relay audience (server identity / public URL) this record
    /// was signed for. Prevents cross-server replay.
    pub audience: String,
    /// Whether this node is willing to terminate relay circuits. A reachability
    /// hint only — never authority.
    pub relay_capable: bool,
    /// Endpoint candidates this node is currently reachable at, in canonical
    /// `(kind, addr)` order.
    ///
    /// Signed (see [`PresenceRecord::signing_payload`]): without that, anyone
    /// able to touch the record — the rendezvous server included — could
    /// redirect a peer's dial to an address they control. Signing it does not
    /// make the address trusted; the dial still has to survive the pinned
    /// transport handshake.
    pub candidates: Vec<EndpointCandidate>,
    /// Random per-record nonce (replay freshness).
    pub nonce: Vec<u8>,
    /// Ed25519 signature over the canonical payload. 64 bytes.
    pub signature: Vec<u8>,
}

impl PresenceRecord {
    /// Build an unsigned presence record. The caller signs it with
    /// [`PresenceRecord::sign`]; an unsigned record never verifies.
    pub fn unsigned(
        node_id: NodeId,
        public_key: Vec<u8>,
        session_id: u64,
        expires_at_ms: u64,
        audience: impl Into<String>,
        relay_capable: bool,
        candidates: Vec<EndpointCandidate>,
        nonce: Vec<u8>,
    ) -> Self {
        Self {
            node_id,
            public_key,
            session_id,
            expires_at_ms,
            audience: audience.into(),
            relay_capable,
            candidates,
            nonce,
            signature: Vec::new(),
        }
    }

    /// The canonical, domain-separated byte payload that gets signed.
    ///
    /// Length-prefixed fields, in a fixed order, so no two distinct records can
    /// ever produce the same payload (no delimiter-collision). Covers every
    /// security-relevant field: NodeId, public key (fingerprint), audience,
    /// session id, expiry, relay capability, and nonce.
    pub fn signing_payload(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(PRESENCE_DOMAIN.as_bytes());
        out.push(0);
        push_str(&mut out, self.node_id.as_str());
        push_bytes(&mut out, &self.public_key);
        push_str(&mut out, &self.audience);
        out.extend_from_slice(&self.session_id.to_be_bytes());
        out.extend_from_slice(&self.expires_at_ms.to_be_bytes());
        out.push(u8::from(self.relay_capable));
        out.extend_from_slice(&(self.candidates.len() as u32).to_be_bytes());
        for candidate in &self.candidates {
            out.push(candidate.kind.wire_tag());
            push_socket_addr(&mut out, candidate.addr);
        }
        push_bytes(&mut out, &self.nonce);
        out
    }

    /// Check the candidate list without touching the signature: bounded, every
    /// address dialable, and in canonical order. Canonical order also rejects
    /// duplicates, since a strictly increasing list cannot hold two equal keys.
    fn verify_candidates(&self) -> Result<(), RendezvousError> {
        if self.candidates.len() > PRESENCE_MAX_CANDIDATES {
            return Err(RendezvousError::Malformed(
                "too many endpoint candidates".to_string(),
            ));
        }
        for candidate in &self.candidates {
            candidate.validate()?;
        }
        if self
            .candidates
            .windows(2)
            .any(|pair| pair[0].sort_key() >= pair[1].sort_key())
        {
            return Err(RendezvousError::Malformed(
                "candidates are not in canonical order".to_string(),
            ));
        }
        Ok(())
    }

    /// The addresses this record advertises, in dial order (host first).
    ///
    /// This is the only place signed candidates should become addresses to
    /// dial, and only after [`PresenceRecord::verify`] has succeeded.
    pub fn dialable_candidates(&self) -> Vec<(EndpointCandidateKind, SocketAddr)> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.validate().is_ok())
            .map(|candidate| (candidate.kind, candidate.addr))
            .collect()
    }

    /// Sign this record in place with the Ed25519 private key whose PKCS#8
    /// document is `pkcs8` (a `NodeIdentity`'s `pkcs8_der()`). The embedded
    /// `public_key` must match the signer, or the record will fail
    /// [`PresenceRecord::verify`] for everyone.
    pub fn sign(mut self, pkcs8: &[u8]) -> Result<Self, RendezvousError> {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        let pair = Ed25519KeyPair::from_pkcs8(pkcs8)
            .map_err(|e| RendezvousError::Malformed(format!("signing key rejected: {e}")))?;
        let signer_public = pair.public_key().as_ref().to_vec();
        if signer_public != self.public_key {
            return Err(RendezvousError::Malformed(
                "signer public key does not match record public key".to_string(),
            ));
        }
        self.signature = pair.sign(&self.signing_payload()).as_ref().to_vec();
        Ok(self)
    }

    /// Verify the signature against the embedded public key and check the lease
    /// is still fresh against `now_ms`. Fail closed on every mismatch.
    ///
    /// This proves the record was minted by the holder of the private key for
    /// `public_key` and has not expired. It deliberately does **not** decide
    /// trust — the caller pins the fingerprint against its own `TrustStore`.
    pub fn verify(&self, now_ms: u64) -> Result<(), RendezvousError> {
        if self.public_key.len() != ED25519_PUBLIC_KEY_BYTES {
            return Err(RendezvousError::Malformed(format!(
                "public key must be {ED25519_PUBLIC_KEY_BYTES} bytes"
            )));
        }
        self.verify_candidates()?;
        if self.nonce.len() != PRESENCE_NONCE_BYTES {
            return Err(RendezvousError::Malformed(format!(
                "nonce must be {PRESENCE_NONCE_BYTES} bytes"
            )));
        }
        if self.signature.len() != 64 {
            return Err(RendezvousError::Malformed(
                "signature must be 64 bytes".to_string(),
            ));
        }
        use ring::signature::{UnparsedPublicKey, ED25519};
        UnparsedPublicKey::new(&ED25519, &self.public_key)
            .verify(&self.signing_payload(), &self.signature)
            .map_err(|_| RendezvousError::BadSignature)?;
        if now_ms >= self.expires_at_ms {
            return Err(RendezvousError::Expired {
                expires_at_ms: self.expires_at_ms,
            });
        }
        Ok(())
    }

    /// Verify the record for one specific audience. Combined check used by a
    /// client that knows which rendezvous server it is talking to.
    pub fn verify_for(&self, now_ms: u64, audience: &str) -> Result<(), RendezvousError> {
        if self.audience != audience {
            return Err(RendezvousError::AudienceMismatch {
                expected: audience.to_string(),
                got: self.audience.clone(),
            });
        }
        self.verify(now_ms)
    }

    /// The pinned fingerprint of this record's public key (SHA-256 hex).
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(&self.public_key);
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Requests a node sends to a rendezvous server. Every one is cheap and
/// idempotent where it can be, and every registration carries proof of key
/// possession (a signed [`PresenceRecord`]) — never a bare "I am node X".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RendezvousRequest {
    /// Register (or refresh) this node's presence. Idempotent per `session_id`.
    Register {
        /// The signed presence lease. Its own signature proves key possession.
        presence: PresenceRecord,
    },
    /// Renew a live lease before it expires. Signed again to defeat replay.
    Heartbeat {
        /// A freshly signed presence record with the same `session_id` and a
        /// new `expires_at_ms` / nonce.
        presence: PresenceRecord,
    },
    /// Ask where a node is currently reachable.
    Lookup {
        /// Who is asking (for server-side rate limiting / abuse accounting).
        querant: NodeId,
        /// Who to look up.
        target: NodeId,
    },
    /// Withdraw this node's presence (go offline).
    Withdraw { node_id: NodeId, session_id: u64 },
}

/// What the rendezvous server answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RendezvousResponse {
    /// Registration / heartbeat accepted; the lease is live until `expires_at_ms`.
    Registered { expires_at_ms: u64 },
    /// The target is currently reachable via relay; here is its presence.
    /// `null` means "absent" (unknown, offline, or expired).
    Presence { presence: Option<PresenceRecord> },
    /// A request was refused (malformed, over limit, bad signature).
    Rejected { reason: String },
}

/// Control-plane messages for opening and closing relay circuits. These move on
/// the client↔server control channel; the circuit *data* is opaque
/// ([`RelayFrame`]) and never appears here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelayControl {
    /// A wants a circuit to B. `source` proves its own key; the server pairs it
    /// with B's live connection.
    Open {
        /// The chosen circuit id (transport name only).
        circuit_id: RelayCircuitId,
        /// Who wants to reach whom.
        source: NodeId,
        target: NodeId,
        /// Signed proof of `source`'s key possession (a minimal presence).
        source_proof: PresenceRecord,
    },
    /// B accepts the inbound circuit from A.
    Accept {
        circuit_id: RelayCircuitId,
        /// Signed proof of `target`'s key possession.
        target_proof: PresenceRecord,
    },
    /// Either side (or the server) closes the circuit.
    Close {
        circuit_id: RelayCircuitId,
        reason: String,
    },
    /// The server refuses a relay action.
    Refused {
        circuit_id: Option<RelayCircuitId>,
        reason: String,
    },
}

/// One opaque relayed frame. The `payload` is end-to-end ciphertext between the
/// two nodes; the relay server treats it as opaque bytes and only reads
/// `circuit_id` to decide where to forward. It can never see the inner
/// `MeshEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayFrame {
    /// Which forwarding path this belongs to (transport name, never identity).
    pub circuit_id: RelayCircuitId,
    /// End-to-end encrypted bytes. Opaque to the server.
    pub payload: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Framing for the client↔server stream.
//
// Both the client (`apeireth-mesh` relay) and the server
// (`apeireth-mesh-rendezvous`) speak this. It is length-prefixed binary with a
// one-byte tag so control (JSON) and relay data (opaque) share one stream
// without the server ever having to parse the data plane.
// ---------------------------------------------------------------------------

/// Frame tag: a JSON-encoded control message.
pub const FRAME_TAG_CONTROL: u8 = 0;
/// Frame tag: an opaque relay data frame.
pub const FRAME_TAG_RELAY: u8 = 1;

/// A decoded stream frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A JSON control message (rendezvous or relay control).
    Control(Vec<u8>),
    /// An opaque relay data frame.
    Relay(RelayFrame),
}

/// Encode a control frame: `[u32 len][tag 0][json bytes]`.
pub fn encode_control(json: &[u8]) -> Vec<u8> {
    encode_frame(FRAME_TAG_CONTROL, json)
}

/// Encode a relay data frame: `[u32 len][tag 1][16-byte circuit][payload]`.
pub fn encode_relay(frame: &RelayFrame) -> Vec<u8> {
    let mut body = Vec::with_capacity(RELAY_CIRCUIT_ID_BYTES + frame.payload.len());
    body.extend_from_slice(frame.circuit_id.as_bytes());
    body.extend_from_slice(&frame.payload);
    encode_frame(FRAME_TAG_RELAY, &body)
}

/// Encode `[u32 BE len][tag][body]` where `len` counts tag+body.
fn encode_frame(tag: u8, body: &[u8]) -> Vec<u8> {
    let len = (body.len() + 1) as u32;
    let mut out = Vec::with_capacity(5 + body.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.push(tag);
    out.extend_from_slice(body);
    out
}

/// The maximum bytes a single frame body may carry. Bounds server memory and
/// refuses oversized frames before allocating.
pub const MAX_FRAME_BODY_BYTES: usize = 4 * 1024 * 1024;

/// Decode one frame from `buf`, returning the frame and the number of bytes
/// consumed. Returns `Ok(None)` when `buf` does not yet hold a complete frame.
/// Rejects oversized frames outright.
pub fn decode_frame(buf: &[u8]) -> Result<Option<(Frame, usize)>, RendezvousError> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if len == 0 {
        return Err(RendezvousError::Malformed("zero-length frame".to_string()));
    }
    if len > MAX_FRAME_BODY_BYTES {
        return Err(RendezvousError::Malformed(format!(
            "frame of {len} bytes exceeds limit {MAX_FRAME_BODY_BYTES}"
        )));
    }
    if buf.len() < 4 + len {
        return Ok(None);
    }
    let tag = buf[4];
    let body = &buf[5..4 + len];
    let consumed = 4 + len;
    match tag {
        FRAME_TAG_CONTROL => Ok(Some((Frame::Control(body.to_vec()), consumed))),
        FRAME_TAG_RELAY => {
            if body.len() < RELAY_CIRCUIT_ID_BYTES {
                return Err(RendezvousError::Malformed(
                    "relay frame shorter than its circuit id".to_string(),
                ));
            }
            let mut cid = [0u8; RELAY_CIRCUIT_ID_BYTES];
            cid.copy_from_slice(&body[..RELAY_CIRCUIT_ID_BYTES]);
            Ok(Some((
                Frame::Relay(RelayFrame {
                    circuit_id: RelayCircuitId::from_bytes(cid),
                    payload: body[RELAY_CIRCUIT_ID_BYTES..].to_vec(),
                }),
                consumed,
            )))
        }
        other => Err(RendezvousError::Malformed(format!(
            "unknown frame tag {other}"
        ))),
    }
}

// --- little-endian-free canonical field helpers ----------------------------

fn push_str(out: &mut Vec<u8>, s: &str) {
    push_bytes(out, s.as_bytes());
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;

    fn keypair() -> (Vec<u8>, Vec<u8>) {
        let rng = SystemRandom::new();
        let doc = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8 = doc.as_ref().to_vec();
        let pair = ring::signature::Ed25519KeyPair::from_pkcs8(&pkcs8).unwrap();
        use ring::signature::KeyPair;
        let pubk = pair.public_key().as_ref().to_vec();
        (pkcs8, pubk)
    }

    fn record(now: u64) -> (PresenceRecord, Vec<u8>) {
        let (pkcs8, pubk) = keypair();
        let r = PresenceRecord::unsigned(
            NodeId::new("node-b").unwrap(),
            pubk,
            7,
            now + 60_000,
            "relay.example",
            true,
            Vec::new(),
            vec![0u8; PRESENCE_NONCE_BYTES],
        );
        (r, pkcs8)
    }

    fn fixture_candidates() -> Vec<EndpointCandidate> {
        canonicalize_candidates(vec![
            EndpointCandidate::new(
                EndpointCandidateKind::ServerReflexive,
                "203.0.113.5:51820".parse().unwrap(),
            )
            .unwrap(),
            EndpointCandidate::new(
                EndpointCandidateKind::Host,
                "192.168.1.9:51820".parse().unwrap(),
            )
            .unwrap(),
        ])
        .unwrap()
    }

    fn signed_with_candidates(now: u64) -> PresenceRecord {
        let (pkcs8, pubk) = keypair();
        let record = PresenceRecord::unsigned(
            NodeId::new("node-b").unwrap(),
            pubk,
            7,
            now + 60_000,
            "relay.example",
            true,
            fixture_candidates(),
            vec![0u8; PRESENCE_NONCE_BYTES],
        );
        record.sign(&pkcs8).unwrap()
    }

    #[test]
    fn signed_candidates_round_trip_in_dial_order() {
        let now = 1_000u64;
        let signed = signed_with_candidates(now);
        assert!(signed.verify(now).is_ok());
        assert_eq!(
            signed.dialable_candidates(),
            vec![
                (
                    EndpointCandidateKind::Host,
                    "192.168.1.9:51820".parse().unwrap()
                ),
                (
                    EndpointCandidateKind::ServerReflexive,
                    "203.0.113.5:51820".parse().unwrap()
                ),
            ]
        );
    }

    #[test]
    fn a_tampered_candidate_address_fails_the_signature() {
        let now = 1_000u64;
        let mut signed = signed_with_candidates(now);
        // Still canonical, still dialable: only the signature can tell that
        // this is not the address the node actually signed.
        signed.candidates[0].addr = "192.168.1.9:51999".parse().unwrap();
        assert_eq!(signed.verify(now), Err(RendezvousError::BadSignature));
    }

    #[test]
    fn a_tampered_candidate_kind_fails_the_signature() {
        let now = 1_000u64;
        let mut signed = signed_with_candidates(now);
        signed.candidates[1].kind = EndpointCandidateKind::Host;
        assert_eq!(signed.verify(now), Err(RendezvousError::BadSignature));
    }

    #[test]
    fn adding_a_candidate_after_signing_is_forgery() {
        let now = 1_000u64;
        let mut signed = signed_with_candidates(now);
        // Chosen to keep the list canonically ordered, so the failure is the
        // signature and not the order check.
        signed.candidates.push(
            EndpointCandidate::new(
                EndpointCandidateKind::ServerReflexive,
                "203.0.113.6:51820".parse().unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(signed.verify(now), Err(RendezvousError::BadSignature));
    }

    #[test]
    fn a_reordered_candidate_list_is_rejected() {
        let now = 1_000u64;
        let mut signed = signed_with_candidates(now);
        signed.candidates.reverse();
        assert!(matches!(
            signed.verify(now),
            Err(RendezvousError::Malformed(_))
        ));
    }

    #[test]
    fn duplicate_candidates_are_collapsed() {
        let dup = EndpointCandidate::new(
            EndpointCandidateKind::Host,
            "192.168.1.9:51820".parse().unwrap(),
        )
        .unwrap();
        let out = canonicalize_candidates(vec![
            EndpointCandidate::new(
                EndpointCandidateKind::ServerReflexive,
                "203.0.113.5:51820".parse().unwrap(),
            )
            .unwrap(),
            dup.clone(),
            dup,
        ])
        .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, EndpointCandidateKind::Host);
        assert_eq!(out[1].kind, EndpointCandidateKind::ServerReflexive);
    }

    #[test]
    fn too_many_candidates_are_rejected() {
        let many: Vec<EndpointCandidate> = (1..=PRESENCE_MAX_CANDIDATES + 1)
            .map(|i| {
                EndpointCandidate::new(
                    EndpointCandidateKind::Host,
                    format!("192.168.1.{i}:51820").parse().unwrap(),
                )
                .unwrap()
            })
            .collect();
        assert!(canonicalize_candidates(many.clone()).is_err());

        // A signer that skips canonicalization is still refused at verify time.
        let now = 1_000u64;
        let (pkcs8, pubk) = keypair();
        let record = PresenceRecord::unsigned(
            NodeId::new("node-b").unwrap(),
            pubk,
            7,
            now + 60_000,
            "relay.example",
            true,
            many,
            vec![0u8; PRESENCE_NONCE_BYTES],
        );
        let signed = record.sign(&pkcs8).unwrap();
        assert!(matches!(
            signed.verify(now),
            Err(RendezvousError::Malformed(_))
        ));
    }

    #[test]
    fn undialable_candidate_addresses_are_rejected() {
        for raw in [
            "0.0.0.0:51820",
            "[::]:51820",
            "192.168.1.9:0",
            "239.1.1.1:51820",
            "255.255.255.255:51820",
            "[ff02::1]:51820",
        ] {
            let addr = raw.parse().unwrap();
            assert!(
                EndpointCandidate::new(EndpointCandidateKind::Host, addr).is_err(),
                "{raw} must not be a dialable candidate"
            );
        }
    }

    #[test]
    fn private_and_loopback_host_candidates_are_allowed() {
        for raw in [
            "192.168.1.9:51820",
            "10.0.0.5:51820",
            "127.0.0.1:51820",
            "[fe80::1]:51820",
        ] {
            let addr = raw.parse().unwrap();
            assert!(
                EndpointCandidate::new(EndpointCandidateKind::Host, addr).is_ok(),
                "{raw} is a legitimate host candidate"
            );
        }
    }

    #[test]
    fn a_json_round_trip_preserves_the_signed_payload() {
        let now = 1_000u64;
        let signed = signed_with_candidates(now);
        let text = serde_json::to_string(&signed).unwrap();
        let back: PresenceRecord = serde_json::from_str(&text).unwrap();
        // The wire form is text, but the signed bytes are binary and family
        // tagged, so re-encoding cannot change what the signature covers.
        assert_eq!(back.signing_payload(), signed.signing_payload());
        assert!(back.verify(now).is_ok());
    }

    #[test]
    fn presence_is_version_two_but_the_control_protocol_is_not() {
        assert_eq!(PRESENCE_DOMAIN, "APEIRETH-MESH-PRESENCE-V2");
        // Candidates changed the record's signed payload, not the rendezvous
        // control protocol, so the control version deliberately stays at 1.
        assert_eq!(RENDEZVOUS_PROTOCOL_VERSION, 1);
    }

    #[test]
    fn a_version_one_domain_signature_cannot_verify() {
        let now = 1_000u64;
        let (pkcs8, pubk) = keypair();
        let record = PresenceRecord::unsigned(
            NodeId::new("node-b").unwrap(),
            pubk,
            7,
            now + 60_000,
            "relay.example",
            true,
            Vec::new(),
            vec![0u8; PRESENCE_NONCE_BYTES],
        );
        let payload = record.signing_payload();
        assert!(payload.starts_with(b"APEIRETH-MESH-PRESENCE-V2"));

        // Same fields, old domain prefix: the old signature must not be
        // accepted as a version 2 record that happens to have no candidates.
        use ring::signature::{Ed25519KeyPair, KeyPair};
        let pair = Ed25519KeyPair::from_pkcs8(&pkcs8).unwrap();
        let mut v1_payload = payload.clone();
        v1_payload[.."APEIRETH-MESH-PRESENCE-V1".len()]
            .copy_from_slice(b"APEIRETH-MESH-PRESENCE-V1");
        let mut forged = record;
        forged.signature = pair.sign(&v1_payload).as_ref().to_vec();
        assert_eq!(forged.verify(now), Err(RendezvousError::BadSignature));
    }

    #[test]
    fn a_signed_presence_record_verifies() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let signed = r.sign(&pkcs8).unwrap();
        assert!(signed.verify(now).is_ok());
    }

    #[test]
    fn a_modified_node_id_fails_the_signature() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let mut signed = r.sign(&pkcs8).unwrap();
        signed.node_id = NodeId::new("node-evil").unwrap();
        assert_eq!(signed.verify(now), Err(RendezvousError::BadSignature));
    }

    #[test]
    fn a_modified_fingerprint_fails_the_signature() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let mut signed = r.sign(&pkcs8).unwrap();
        signed.public_key[0] ^= 0xff;
        assert_eq!(signed.verify(now), Err(RendezvousError::BadSignature));
    }

    #[test]
    fn an_expired_presence_is_rejected() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let signed = r.sign(&pkcs8).unwrap();
        assert_eq!(
            signed.verify(now + 60_001),
            Err(RendezvousError::Expired {
                expires_at_ms: now + 60_000
            })
        );
    }

    #[test]
    fn the_wrong_audience_is_rejected() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let signed = r.sign(&pkcs8).unwrap();
        assert!(matches!(
            signed.verify_for(now, "other.example"),
            Err(RendezvousError::AudienceMismatch { .. })
        ));
        assert!(signed.verify_for(now, "relay.example").is_ok());
    }

    #[test]
    fn a_replayed_record_past_expiry_cannot_verify() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let signed = r.sign(&pkcs8).unwrap();
        // Even a correctly signed record is dead once its lease lapses.
        assert!(signed.verify(now + 60_000).is_err());
    }

    #[test]
    fn a_record_signed_with_a_foreign_key_is_rejected() {
        let now = 1_000u64;
        let (r, _pkcs8) = record(now);
        let (_other_pkcs8, other_pub) = keypair();
        let foreign = PresenceRecord::unsigned(
            r.node_id.clone(),
            other_pub,
            r.session_id,
            r.expires_at_ms,
            r.audience.clone(),
            r.relay_capable,
            r.candidates.clone(),
            r.nonce.clone(),
        );
        // Signing with a key that doesn't match the embedded public key fails.
        let wrong = {
            let rng = SystemRandom::new();
            ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap()
        };
        assert!(foreign.sign(wrong.as_ref()).is_err());
    }

    #[test]
    fn presence_carries_no_capabilities_and_no_credentials() {
        let now = 1_000u64;
        let (r, pkcs8) = record(now);
        let signed = r.sign(&pkcs8).unwrap();
        let json = serde_json::to_string(&signed).unwrap();
        // The wire form must stay minimal: identity + reachability, nothing more.
        for forbidden in [
            "capabilit",
            "credential",
            "tool",
            "model",
            "prompt",
            "secret",
        ] {
            assert!(
                !json.contains(forbidden),
                "presence leaked {forbidden}: {json}"
            );
        }
    }

    #[test]
    fn a_circuit_id_is_not_a_node_id() {
        let cid = RelayCircuitId::from_bytes([0xAB; RELAY_CIRCUIT_ID_BYTES]);
        // Circuit ids are raw bytes with a hex form; they cannot be parsed or
        // presented as NodeIds.
        assert_eq!(cid.to_hex().len(), RELAY_CIRCUIT_ID_BYTES * 2);
        assert!(NodeId::new(&cid.to_hex()).is_ok()); // syntactically an id string…
                                                     // …but it is a *different* value namespace: equality with a real node is
                                                     // never assumed anywhere, and the type system keeps them apart.
        let _ = cid.as_bytes();
    }

    #[test]
    fn frames_round_trip_and_oversized_frames_are_refused() {
        // Control frame.
        let ctrl = encode_control(b"{\"kind\":\"lookup\"}");
        match decode_frame(&ctrl).unwrap().unwrap() {
            (Frame::Control(bytes), n) => {
                assert_eq!(bytes, b"{\"kind\":\"lookup\"}");
                assert_eq!(n, ctrl.len());
            }
            other => panic!("expected control, got {other:?}"),
        }
        // Relay frame.
        let relay = RelayFrame {
            circuit_id: RelayCircuitId::from_bytes([1; 16]),
            payload: vec![9, 9, 9],
        };
        let enc = encode_relay(&relay);
        match decode_frame(&enc).unwrap().unwrap() {
            (Frame::Relay(f), n) => {
                assert_eq!(f, relay);
                assert_eq!(n, enc.len());
            }
            other => panic!("expected relay, got {other:?}"),
        }
        // Incomplete frame.
        assert!(decode_frame(&enc[..3]).unwrap().is_none());
        // Oversized declared length.
        let mut huge = (MAX_FRAME_BODY_BYTES as u32 + 1).to_be_bytes().to_vec();
        huge.push(FRAME_TAG_RELAY);
        assert!(decode_frame(&huge).is_err());
    }
}
