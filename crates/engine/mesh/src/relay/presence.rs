//! Building signed presence records from a node's identity.
//!
//! A presence record is how a node says "I am reachable, and here is proof it is
//! really me" to a rendezvous server — and to any peer that looks it up. The
//! signature proves key possession; the short TTL keeps the claim a *lease*;
//! the audience binds it to one server. None of that is trust: a peer still
//! pins the fingerprint in its own [`crate::trust_store::TrustStore`].

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_mesh_protocol::rendezvous::{
    canonicalize_candidates, EndpointCandidate, PresenceRecord, RendezvousError,
    PRESENCE_NONCE_BYTES,
};
use apeireth_mesh_protocol::NodeId;

use crate::identity::NodeIdentity;

/// Wall-clock milliseconds. Presence is infrastructure state (not mesh
/// semantics), so it reads the real clock rather than an injected [`Clock`].
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Build and sign a fresh presence record for `identity`.
///
/// Every call mints a new random nonce and a new expiry, so a heartbeat is a
/// genuinely fresh signed claim (defeats replaying a captured registration).
pub fn signed_presence(
    identity: &NodeIdentity,
    node_id: NodeId,
    audience: String,
    session_id: u64,
    ttl_ms: u64,
    relay_capable: bool,
) -> PresenceRecord {
    let nonce = random_nonce();
    let expires_at_ms = now_ms() + ttl_ms;
    let record = PresenceRecord::unsigned(
        node_id,
        identity.public_key().to_vec(),
        session_id,
        expires_at_ms,
        audience,
        relay_capable,
        Vec::new(),
        nonce,
    );
    // Signing a record whose public key matches the identity cannot fail here.
    record
        .sign(identity.pkcs8_der())
        .expect("identity key signs its own presence")
}

/// Build and sign a presence record that advertises endpoint candidates.
///
/// Candidates are canonicalized (validated, sorted, de-duplicated) before
/// signing, so the record has exactly one encoding and a verifier can require
/// canonical order. An invalid candidate is an error rather than a silently
/// dropped field: a node that believes it advertised an address must not be
/// surprised to find it missing.
pub fn signed_presence_with_candidates(
    identity: &NodeIdentity,
    node_id: NodeId,
    audience: String,
    session_id: u64,
    ttl_ms: u64,
    relay_capable: bool,
    candidates: Vec<EndpointCandidate>,
) -> Result<PresenceRecord, RendezvousError> {
    let candidates = canonicalize_candidates(candidates)?;
    let nonce = random_nonce();
    let expires_at_ms = now_ms() + ttl_ms;
    let record = PresenceRecord::unsigned(
        node_id,
        identity.public_key().to_vec(),
        session_id,
        expires_at_ms,
        audience,
        relay_capable,
        candidates,
        nonce,
    );
    record.sign(identity.pkcs8_der())
}

fn random_nonce() -> Vec<u8> {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut b = vec![0u8; PRESENCE_NONCE_BYTES];
    SystemRandom::new().fill(&mut b).expect("rng");
    b
}
