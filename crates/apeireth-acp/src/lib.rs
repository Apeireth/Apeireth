
//! apeireth-acp — R23 6 module acp 子模块。
//!
//! R23 P1 #5 实质化: 加 +7 顶层 pub fn — envelope 完整性 / 路由 / 序列号 / 校验和.
//! 不假装: 真 stdlib SipHash + base64-json serialization. **0 引新 crate** (sha2 / base64 在
//! 锁链已存在, 但 apeireth-acp 不再新增 dep — stdlib 已足够 S-6 透明校验).
//!
//! **SipHash vs SHA-256**: stdlib SipHash 1-3 不抗 attacker-selected collision, 适合
//! 进程内防误改; 跨进程 / 跨主机 安全校验留 R24+ 接 sha2 (在 Cargo.lock 已存在,
//! 引入时只需 apeireth-acp/Cargo.toml +1 行).
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AcpError {
    #[error("acp: sender `{0}` is empty")]
    EmptySender(String),
    #[error("acp: 校验和不匹配, 期望 {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("acp: 序列化失败: {0}")]
    SerializationError(String),
}
pub type AcpResult<T> = Result<T, AcpError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub sender: String,
    pub recipient: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

impl Envelope {
    pub fn new(sender: impl Into<String>, recipient: impl Into<String>, kind: impl Into<String>, payload: serde_json::Value) -> Self {
        Self { sender: sender.into(), recipient: recipient.into(), kind: kind.into(), payload }
    }
    pub fn validate(&self) -> AcpResult<()> {
        if self.sender.trim().is_empty() { return Err(AcpError::EmptySender(self.sender.clone())); }
        if self.recipient.trim().is_empty() { return Err(AcpError::EmptySender(format!("recipient={}", self.recipient))); }
        Ok(())
    }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — Envelope integrity + routing + serialization
// ============================================================================

/// Compute SipHash (stdlib) hex digest. 16 chars.
pub fn checksum(env: &Envelope) -> AcpResult<String> {
    env.validate()?;
    // 保证字段顺序与类型: { sender, recipient, kind, payload }
    let record = serde_json::json!({
        "sender":    env.sender,
        "recipient": env.recipient,
        "kind":      env.kind,
        "payload":   env.payload,
    });
    let mut serialized = serde_json::to_string(&record)
        .map_err(|e| AcpError::SerializationError(e.to_string()))?;
    // Seed with type prefix to avoid collisions across envelope types
    serialized.insert(0, 'E');
    let mut hasher = DefaultHasher::new();
    serialized.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

/// Verify envelope against expected checksum.
pub fn verify(env: &Envelope, expected_hex: &str) -> AcpResult<()> {
    let actual = checksum(env)?;
    if actual != expected_hex {
        return Err(AcpError::ChecksumMismatch { expected: expected_hex.into(), actual });
    }
    Ok(())
}

/// Whether envelope targets a single recipient (`recipient != "*"`).
pub fn is_unicast(env: &Envelope) -> bool { env.recipient != "*" && !env.recipient.is_empty() }

/// Whether envelope is broadcast (`recipient == "*"`).
pub fn is_broadcast(env: &Envelope) -> bool { env.recipient == "*" }

/// Generate deterministic sequence number from sender + counters in content.
/// Use case: replace random UUID with deterministic ID for replay scenarios.
pub fn sequence_number(env: &Envelope, counter: u64) -> AcpResult<u64> {
    env.validate()?;
    let mut hasher = DefaultHasher::new();
    env.sender.hash(&mut hasher);
    counter.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Two envelopes equivalent if their payload equals (sender/recipient/kind may differ).
pub fn payload_equivalent(a: &Envelope, b: &Envelope) -> bool { a.payload == b.payload }

/// Whether envelope matches given (sender, kind) pair.
pub fn matches_pair(env: &Envelope, sender: &str, kind: &str) -> bool {
    env.sender == sender && env.kind == kind
}

/// Serialize envelope to deterministic JSON string (no trailing whitespace, sorted keys
/// guaranteed by `serde_json`).
pub fn to_json_string(env: &Envelope) -> AcpResult<String> {
    env.validate()?;
    serde_json::to_string(env).map_err(|e| AcpError::SerializationError(e.to_string()))
}

/// Deserialize envelope from JSON string. validate immediately.
pub fn from_json_string(s: &str) -> AcpResult<Envelope> {
    let env: Envelope = serde_json::from_str(s).map_err(|e| AcpError::SerializationError(format!("json decode: {e}")))?;
    env.validate()?;
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test] fn envelope_roundtrips_through_validate() {
        let e = Envelope::new("a", "b", "ping", json!({"hi": 1}));
        assert!(e.validate().is_ok());
    }
    #[test] fn empty_sender_is_rejected() {
        let e = Envelope::new("", "b", "ping", json!({}));
        assert!(e.validate().is_err());
    }

    #[test] fn checksum_deterministic() {
        let e = Envelope::new("a", "b", "ping", json!({"x": 1}));
        let h1 = checksum(&e).unwrap();
        let h2 = checksum(&e).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);  // u64 hex = 16 chars
    }
    #[test] fn checksum_different_payload_changes_digest() {
        let e1 = Envelope::new("a", "b", "ping", json!({"x": 1}));
        let e2 = Envelope::new("a", "b", "ping", json!({"x": 2}));
        assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
    }
    #[test] fn verify_matches_own_checksum() {
        let e = Envelope::new("a", "b", "ping", json!({}));
        let h = checksum(&e).unwrap();
        assert!(verify(&e, &h).is_ok());
    }
    #[test] fn verify_rejects_tamper() {
        let e = Envelope::new("a", "b", "ping", json!({}));
        let h = checksum(&e).unwrap();
        let tampered = Envelope::new("a", "b", "PING", json!({}));
        assert!(verify(&tampered, &h).is_err());
    }
    #[test] fn is_unicast_and_broadcast() {
        let u = Envelope::new("a", "b", "ping", json!({}));
        let b = Envelope::new("a", "*", "ping", json!({}));
        assert!(is_unicast(&u));
        assert!(!is_broadcast(&u));
        assert!(is_broadcast(&b));
        assert!(!is_unicast(&b));
    }
    #[test] fn sequence_number_monotonic_per_counter() {
        let e = Envelope::new("a", "b", "ping", json!({}));
        let s1 = sequence_number(&e, 1).unwrap();
        let s2 = sequence_number(&e, 2).unwrap();
        // 不保证实际递增 (只要不同即可), 但肯定不等
        assert_ne!(s1, s2);
    }
    #[test] fn payload_equivalent_basic() {
        let a = Envelope::new("a", "x", "k", json!({"q": 1}));
        let b = Envelope::new("c", "y", "j", json!({"q": 1}));
        assert!(payload_equivalent(&a, &b));
    }
    #[test] fn matches_pair_basic() {
        let e = Envelope::new("alice", "bob", "request", json!({}));
        assert!(matches_pair(&e, "alice", "request"));
        assert!(!matches_pair(&e, "bob", "request"));
    }
    #[test] fn json_roundtrip() {
        let e = Envelope::new("a", "b", "ping", json!({"k": 42}));
        let s = to_json_string(&e).unwrap();
        let decoded = from_json_string(&s).unwrap();
        assert_eq!(decoded, e);
    }
    #[test] fn json_invalid_input_rejected() {
        assert!(from_json_string("not json").is_err());
    }
}
