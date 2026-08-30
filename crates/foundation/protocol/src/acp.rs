//! ACP (Agent Communication Protocol) envelope schema: agent-to-agent message
//! envelope with integrity checksum, routing classification, and deterministic
//! JSON serialization.
//!
//! Recovered from the legacy `apeireth-acp` crate (R23 P1 #5) as a *schema*
//! only: the donor's transport lives in a pre-v2 runtime this workspace no
//! longer has, but the envelope contract — fields, validation, checksum,
//! unicast/broadcast routing classification, sequence numbering, strict JSON
//! round trip — is pure protocol vocabulary. Note this is a different envelope
//! from `adapters/sdk::wire::Envelope` (which is the cross-language SDK wire
//! format `v/kind/id/body`); the ACP envelope is agent-addressed
//! (`sender/recipient/kind/payload`).
//!
//! Checksum honesty (carried over from the donor): the integrity digest uses
//! the standard library's `DefaultHasher` (SipHash 1-3), which is stable
//! within a Rust toolchain but is NOT collision-resistant against an
//! adversary. It is an in-process tamper-evidence check, not a security
//! boundary; cross-host integrity must use SHA-256/HMAC at the transport
//! layer.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use thiserror::Error;

/// ACP envelope errors (donor 1:1).
#[derive(Debug, Error)]
pub enum AcpError {
    /// Sender (or recipient) is empty after trimming.
    #[error("acp: sender `{0}` is empty")]
    EmptySender(String),
    /// Digest mismatch between expected and actual checksum.
    #[error("acp: checksum mismatch, expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    /// JSON serialization / deserialization failure.
    #[error("acp: serialization failed: {0}")]
    SerializationError(String),
}

/// Result alias for ACP operations.
pub type AcpResult<T> = Result<T, AcpError>;

/// Agent-to-agent envelope (donor `apeireth_acp::Envelope`, 1:1).
///
/// `recipient = "*"` means broadcast; any other non-empty recipient is
/// unicast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpEnvelope {
    /// Sending agent id (non-empty).
    pub sender: String,
    /// Recipient agent id, or `"*"` for broadcast (non-empty).
    pub recipient: String,
    /// Envelope kind (message-type tag, e.g. `"ping"`, `"request"`).
    pub kind: String,
    /// Arbitrary JSON payload.
    pub payload: serde_json::Value,
}

impl AcpEnvelope {
    /// Construct an envelope.
    pub fn new(
        sender: impl Into<String>,
        recipient: impl Into<String>,
        kind: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            sender: sender.into(),
            recipient: recipient.into(),
            kind: kind.into(),
            payload,
        }
    }

    /// Validate addressing: both sender and recipient must be non-empty.
    pub fn validate(&self) -> AcpResult<()> {
        if self.sender.trim().is_empty() {
            return Err(AcpError::EmptySender(self.sender.clone()));
        }
        if self.recipient.trim().is_empty() {
            return Err(AcpError::EmptySender(format!("recipient={}", self.recipient)));
        }
        Ok(())
    }
}

/// Compute the envelope's integrity digest (16 lowercase hex chars).
///
/// Donor 1:1: canonical field-ordered JSON, prefixed with `'E'` (seeded by
/// type to avoid cross-envelope-type collisions), hashed with `DefaultHasher`.
/// See the module docs for the honesty note on SipHash.
pub fn checksum(env: &AcpEnvelope) -> AcpResult<String> {
    env.validate()?;
    // Fixed field order + types: { sender, recipient, kind, payload }.
    let record = serde_json::json!({
        "sender":    env.sender,
        "recipient": env.recipient,
        "kind":      env.kind,
        "payload":   env.payload,
    });
    let mut serialized = serde_json::to_string(&record)
        .map_err(|e| AcpError::SerializationError(e.to_string()))?;
    serialized.insert(0, 'E');
    let mut hasher = DefaultHasher::new();
    serialized.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

/// Verify an envelope against an expected digest.
pub fn verify(env: &AcpEnvelope, expected_hex: &str) -> AcpResult<()> {
    let actual = checksum(env)?;
    if actual != expected_hex {
        return Err(AcpError::ChecksumMismatch {
            expected: expected_hex.into(),
            actual,
        });
    }
    Ok(())
}

/// Whether the envelope targets a single recipient (`recipient != "*"`).
#[must_use]
pub fn is_unicast(env: &AcpEnvelope) -> bool {
    env.recipient != "*" && !env.recipient.is_empty()
}

/// Whether the envelope is a broadcast (`recipient == "*"`).
#[must_use]
pub fn is_broadcast(env: &AcpEnvelope) -> bool {
    env.recipient == "*"
}

/// Deterministic sequence id from sender + a caller counter.
///
/// Donor 1:1: replaces random UUIDs with a reproducible id for replay
/// scenarios; the value is stable for the same `(sender, counter)` pair.
pub fn sequence_number(env: &AcpEnvelope, counter: u64) -> AcpResult<u64> {
    env.validate()?;
    let mut hasher = DefaultHasher::new();
    env.sender.hash(&mut hasher);
    counter.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Two envelopes are payload-equivalent when their payloads are equal
/// (sender / recipient / kind may differ).
#[must_use]
pub fn payload_equivalent(a: &AcpEnvelope, b: &AcpEnvelope) -> bool {
    a.payload == b.payload
}

/// Whether the envelope matches a `(sender, kind)` pair.
#[must_use]
pub fn matches_pair(env: &AcpEnvelope, sender: &str, kind: &str) -> bool {
    env.sender == sender && env.kind == kind
}

/// Serialize to a strict JSON string (validates addressing first).
pub fn to_json_string(env: &AcpEnvelope) -> AcpResult<String> {
    env.validate()?;
    serde_json::to_string(env).map_err(|e| AcpError::SerializationError(e.to_string()))
}

/// Deserialize from a JSON string, validating addressing immediately.
pub fn from_json_string(s: &str) -> AcpResult<AcpEnvelope> {
    let env: AcpEnvelope = serde_json::from_str(s)
        .map_err(|e| AcpError::SerializationError(format!("json decode: {e}")))?;
    env.validate()?;
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Tests ported 1:1 from the donor apeireth-acp crate.

    #[test]
    fn envelope_roundtrips_through_validate() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({"hi": 1}));
        assert!(e.validate().is_ok());
    }

    #[test]
    fn empty_sender_is_rejected() {
        let e = AcpEnvelope::new("", "b", "ping", json!({}));
        assert!(e.validate().is_err());
    }

    #[test]
    fn empty_recipient_is_rejected() {
        let e = AcpEnvelope::new("a", " ", "ping", json!({}));
        assert!(e.validate().is_err());
    }

    #[test]
    fn checksum_deterministic() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({"x": 1}));
        let h1 = checksum(&e).unwrap();
        let h2 = checksum(&e).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16); // u64 hex = 16 chars
    }

    #[test]
    fn checksum_different_payload_changes_digest() {
        let e1 = AcpEnvelope::new("a", "b", "ping", json!({"x": 1}));
        let e2 = AcpEnvelope::new("a", "b", "ping", json!({"x": 2}));
        assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
    }

    #[test]
    fn checksum_kind_change_changes_digest() {
        let e1 = AcpEnvelope::new("a", "b", "ping", json!({"x": 1}));
        let e2 = AcpEnvelope::new("a", "b", "pong", json!({"x": 1}));
        assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
    }

    #[test]
    fn verify_matches_own_checksum() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({}));
        let h = checksum(&e).unwrap();
        assert!(verify(&e, &h).is_ok());
    }

    #[test]
    fn verify_rejects_tamper() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({}));
        let h = checksum(&e).unwrap();
        let tampered = AcpEnvelope::new("a", "b", "PING", json!({}));
        assert!(verify(&tampered, &h).is_err());
    }

    #[test]
    fn is_unicast_and_broadcast() {
        let u = AcpEnvelope::new("a", "b", "ping", json!({}));
        let b = AcpEnvelope::new("a", "*", "ping", json!({}));
        assert!(is_unicast(&u));
        assert!(!is_broadcast(&u));
        assert!(is_broadcast(&b));
        assert!(!is_unicast(&b));
    }

    #[test]
    fn sequence_number_monotonic_per_counter() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({}));
        let s1 = sequence_number(&e, 1).unwrap();
        let s2 = sequence_number(&e, 2).unwrap();
        // Not guaranteed to increment, but must differ per counter.
        assert_ne!(s1, s2);
    }

    #[test]
    fn sequence_number_is_stable_for_same_pair() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({}));
        assert_eq!(sequence_number(&e, 7).unwrap(), sequence_number(&e, 7).unwrap());
    }

    #[test]
    fn payload_equivalent_basic() {
        let a = AcpEnvelope::new("a", "x", "k", json!({"q": 1}));
        let b = AcpEnvelope::new("c", "y", "j", json!({"q": 1}));
        assert!(payload_equivalent(&a, &b));
    }

    #[test]
    fn matches_pair_basic() {
        let e = AcpEnvelope::new("alice", "bob", "request", json!({}));
        assert!(matches_pair(&e, "alice", "request"));
        assert!(!matches_pair(&e, "bob", "request"));
    }

    #[test]
    fn json_roundtrip() {
        let e = AcpEnvelope::new("a", "b", "ping", json!({"k": 42}));
        let s = to_json_string(&e).unwrap();
        let decoded = from_json_string(&s).unwrap();
        assert_eq!(decoded, e);
    }

    #[test]
    fn json_invalid_input_rejected() {
        assert!(from_json_string("not json").is_err());
    }

    #[test]
    fn json_envelope_with_empty_sender_rejected_on_decode() {
        // Wire contract: decode validates addressing immediately.
        let raw = r#"{"sender":"","recipient":"b","kind":"ping","payload":{}}"#;
        assert!(from_json_string(raw).is_err());
    }
}
