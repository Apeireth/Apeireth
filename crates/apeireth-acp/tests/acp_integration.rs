//! Integration tests for apeireth-acp (post-1.0.0)
//!
//! src/lib.rs 已有 12 #[test] + organ_kani_proofs 10. 这里 (tests/) 加跨函数集成 + 边界.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_acp::{
    checksum, from_json_string, is_broadcast, is_unicast, matches_pair, payload_equivalent,
    sequence_number, to_json_string, verify, AcpError, Envelope,
};
use serde_json::json;

// =============================================================================
// Envelope validate
// =============================================================================

#[test]
fn envelope_new_fields() {
    let e = Envelope::new("alice", "bob", "ping", json!({"x": 1}));
    assert_eq!(e.sender, "alice");
    assert_eq!(e.recipient, "bob");
    assert_eq!(e.kind, "ping");
    assert_eq!(e.payload, json!({"x": 1}));
}

#[test]
fn envelope_validate_empty_sender_rejected() {
    let e = Envelope::new("", "bob", "ping", json!({}));
    assert!(e.validate().is_err());
}

#[test]
fn envelope_validate_whitespace_sender_rejected() {
    let e = Envelope::new("   ", "bob", "ping", json!({}));
    assert!(e.validate().is_err(), "whitespace-only sender 应拒");
}

#[test]
fn envelope_validate_empty_recipient_rejected() {
    let e = Envelope::new("alice", "", "ping", json!({}));
    assert!(e.validate().is_err(), "空 recipient 应拒");
}

#[test]
fn envelope_validate_whitespace_recipient_rejected() {
    let e = Envelope::new("alice", "   ", "ping", json!({}));
    assert!(e.validate().is_err());
}

#[test]
fn envelope_validate_empty_kind_allowed() {
    // kind 不参与 validate (per src code)
    let e = Envelope::new("alice", "bob", "", json!({}));
    assert!(e.validate().is_ok(), "kind 允许空");
}

#[test]
fn envelope_clone_eq() {
    let a = Envelope::new("alice", "bob", "ping", json!({"k": 1}));
    let b = a.clone();
    assert_eq!(a, b);
}

// =============================================================================
// checksum
// =============================================================================

#[test]
fn checksum_is_16_hex() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let h = checksum(&e).unwrap();
    assert_eq!(h.len(), 16);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn checksum_changes_with_sender() {
    let e1 = Envelope::new("a", "b", "k", json!({}));
    let e2 = Envelope::new("x", "b", "k", json!({}));
    assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
}

#[test]
fn checksum_changes_with_recipient() {
    let e1 = Envelope::new("a", "b", "k", json!({}));
    let e2 = Envelope::new("a", "c", "k", json!({}));
    assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
}

#[test]
fn checksum_changes_with_kind() {
    let e1 = Envelope::new("a", "b", "ping", json!({}));
    let e2 = Envelope::new("a", "b", "PONG", json!({}));
    assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
}

#[test]
fn checksum_changes_with_payload_field() {
    let e1 = Envelope::new("a", "b", "k", json!({"x": 1}));
    let e2 = Envelope::new("a", "b", "k", json!({"x": 2}));
    assert_ne!(checksum(&e1).unwrap(), checksum(&e2).unwrap());
}

#[test]
fn checksum_empty_payload_deterministic() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let h1 = checksum(&e).unwrap();
    let h2 = checksum(&e).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn checksum_invalid_envelope_errors() {
    let e = Envelope::new("", "b", "k", json!({}));
    assert!(checksum(&e).is_err());
}

// =============================================================================
// verify
// =============================================================================

#[test]
fn verify_ok_when_unchanged() {
    let e = Envelope::new("a", "b", "k", json!({"v": 1}));
    let h = checksum(&e).unwrap();
    assert!(verify(&e, &h).is_ok());
}

#[test]
fn verify_fails_after_payload_change() {
    let e = Envelope::new("a", "b", "k", json!({"v": 1}));
    let h = checksum(&e).unwrap();
    let tampered = Envelope::new("a", "b", "k", json!({"v": 2}));
    assert!(verify(&tampered, &h).is_err());
}

#[test]
fn verify_fails_after_sender_change() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let h = checksum(&e).unwrap();
    let tampered = Envelope::new("X", "b", "k", json!({}));
    assert!(verify(&tampered, &h).is_err());
}

#[test]
fn verify_fails_with_wrong_expected() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let wrong = "0000000000000000";
    assert!(verify(&e, wrong).is_err());
}

#[test]
fn verify_invalid_envelope_errors() {
    let e = Envelope::new("", "b", "k", json!({}));
    assert!(verify(&e, "anything").is_err());
}

// =============================================================================
// is_unicast / is_broadcast
// =============================================================================

#[test]
fn unicast_specific_recipient() {
    let e = Envelope::new("a", "bob", "k", json!({}));
    assert!(is_unicast(&e));
    assert!(!is_broadcast(&e));
}

#[test]
fn broadcast_star_recipient() {
    let e = Envelope::new("a", "*", "k", json!({}));
    assert!(is_broadcast(&e));
    assert!(!is_unicast(&e));
}

#[test]
fn unicast_empty_recipient_neither() {
    let e = Envelope::new("a", "", "k", json!({}));
    assert!(!is_unicast(&e), "空 recipient ≠ unicast");
    assert!(!is_broadcast(&e), "空 recipient ≠ broadcast");
}

// =============================================================================
// sequence_number
// =============================================================================

#[test]
fn sequence_number_deterministic() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let s1 = sequence_number(&e, 5).unwrap();
    let s2 = sequence_number(&e, 5).unwrap();
    assert_eq!(s1, s2);
}

#[test]
fn sequence_number_changes_with_counter() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let s1 = sequence_number(&e, 1).unwrap();
    let s2 = sequence_number(&e, 2).unwrap();
    assert_ne!(s1, s2);
}

#[test]
fn sequence_number_changes_with_sender() {
    let e1 = Envelope::new("a", "b", "k", json!({}));
    let e2 = Envelope::new("X", "b", "k", json!({}));
    let s1 = sequence_number(&e1, 5).unwrap();
    let s2 = sequence_number(&e2, 5).unwrap();
    assert_ne!(s1, s2);
}

#[test]
fn sequence_number_zero_counter_ok() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let s = sequence_number(&e, 0).unwrap();
    let _ = s; // just shouldn't panic
}

#[test]
fn sequence_number_invalid_envelope_errors() {
    let e = Envelope::new("", "b", "k", json!({}));
    assert!(sequence_number(&e, 0).is_err());
}

// =============================================================================
// payload_equivalent
// =============================================================================

#[test]
fn payload_equivalent_same_payload() {
    let a = Envelope::new("alice", "bob", "ping", json!({"x": 1}));
    let b = Envelope::new("carol", "dave", "PONG", json!({"x": 1}));
    assert!(payload_equivalent(&a, &b));
}

#[test]
fn payload_equivalent_different_payload_false() {
    let a = Envelope::new("alice", "bob", "k", json!({"x": 1}));
    let b = Envelope::new("alice", "bob", "k", json!({"x": 2}));
    assert!(!payload_equivalent(&a, &b));
}

#[test]
fn payload_equivalent_empty_payloads() {
    let a = Envelope::new("a", "b", "k", json!({}));
    let b = Envelope::new("x", "y", "j", json!({}));
    assert!(payload_equivalent(&a, &b), "空 payload 等价");
}

#[test]
fn payload_equivalent_complex_payload() {
    let p = json!({
        "nested": {"a": [1, 2, 3], "b": "x"},
        "ok": true,
        "n": null
    });
    let a = Envelope::new("a", "b", "k", p.clone());
    let b = Envelope::new("x", "y", "j", p);
    assert!(payload_equivalent(&a, &b));
}

// =============================================================================
// matches_pair
// =============================================================================

#[test]
fn matches_pair_exact() {
    let e = Envelope::new("alice", "bob", "request", json!({}));
    assert!(matches_pair(&e, "alice", "request"));
}

#[test]
fn matches_pair_sender_mismatch() {
    let e = Envelope::new("alice", "bob", "request", json!({}));
    assert!(!matches_pair(&e, "bob", "request"));
}

#[test]
fn matches_pair_kind_mismatch() {
    let e = Envelope::new("alice", "bob", "request", json!({}));
    assert!(!matches_pair(&e, "alice", "response"));
}

#[test]
fn matches_pair_case_sensitive() {
    let e = Envelope::new("alice", "bob", "request", json!({}));
    assert!(!matches_pair(&e, "Alice", "request"), "应大小写敏感");
}

// =============================================================================
// JSON serialization
// =============================================================================

#[test]
fn json_roundtrip_basic() {
    let e = Envelope::new("alice", "bob", "ping", json!({"k": 42}));
    let s = to_json_string(&e).unwrap();
    let back = from_json_string(&s).unwrap();
    assert_eq!(e, back);
}

#[test]
fn json_roundtrip_complex_payload() {
    let p = json!({
        "string": "中文",
        "array": [1, 2, 3],
        "nested": {"a": true, "b": null}
    });
    let e = Envelope::new("alice", "bob", "ping", p);
    let s = to_json_string(&e).unwrap();
    let back = from_json_string(&s).unwrap();
    assert_eq!(e, back);
}

#[test]
fn json_invalid_input_returns_err() {
    let r = from_json_string("not valid json");
    assert!(r.is_err());
}

#[test]
fn json_invalid_envelope_serialized_input_rejected() {
    // 序列化时已 validate, 但 deserialize 后还会 validate
    // 构造一个 envelope 手动 JSON 化后改掉 sender
    let valid = Envelope::new("alice", "bob", "k", json!({}));
    let mut s = to_json_string(&valid).unwrap();
    s = s.replace("\"alice\"", "\"\"");
    let r = from_json_string(&s);
    assert!(r.is_err(), "空 sender 应被 validate 拒");
}

#[test]
fn json_no_whitespace_trailing() {
    let e = Envelope::new("a", "b", "k", json!({}));
    let s = to_json_string(&e).unwrap();
    assert!(!s.contains("  "), "应无多余空格");
    assert!(!s.ends_with('\n'), "应无 trailing newline");
}

// =============================================================================
// AcpError display
// =============================================================================

#[test]
fn error_empty_sender_display() {
    let e = AcpError::EmptySender("foo".into());
    let s = e.to_string();
    assert!(s.contains("foo"));
    assert!(s.contains("empty"));
}

#[test]
fn error_checksum_mismatch_display() {
    let e = AcpError::ChecksumMismatch {
        expected: "aaa".into(),
        actual: "bbb".into(),
    };
    let s = e.to_string();
    assert!(s.contains("aaa"));
    assert!(s.contains("bbb"));
}

#[test]
fn error_serialization_display() {
    let e = AcpError::SerializationError("bad json".into());
    let s = e.to_string();
    assert!(s.contains("bad json"));
}

// =============================================================================
// Cross-API integration
// =============================================================================

#[test]
fn integration_checksum_verify_json_roundtrip() {
    let e = Envelope::new(
        "alice",
        "bob",
        "request",
        json!({"action": "deploy", "version": 3}),
    );
    let h = checksum(&e).unwrap();
    let s = to_json_string(&e).unwrap();
    let back = from_json_string(&s).unwrap();
    assert!(verify(&back, &h).is_ok(), "roundtrip 后 checksum 仍 OK");
}

#[test]
fn integration_payload_equiv_with_routing() {
    let payload = json!({"event": "approval_required"});
    let unicast = Envelope::new("alice", "bob", "approval", payload.clone());
    let broadcast = Envelope::new("alice", "*", "approval", payload.clone());
    assert!(payload_equivalent(&unicast, &broadcast));
    assert!(is_unicast(&unicast));
    assert!(is_broadcast(&broadcast));
    assert!(matches_pair(&unicast, "alice", "approval"));
}

#[test]
fn integration_broadcast_checksum_unique() {
    let e1 = Envelope::new("a", "*", "k", json!({}));
    let e2 = Envelope::new("a", "*", "k", json!({}));
    let h1 = checksum(&e1).unwrap();
    let h2 = checksum(&e2).unwrap();
    assert_eq!(h1, h2, "相同 broadcast 应同 hash");
}

#[test]
fn integration_sequence_per_sender() {
    // 不同 sender 不同 sequence
    let e_a = Envelope::new("alice", "bob", "k", json!({}));
    let e_b = Envelope::new("bob", "alice", "k", json!({}));
    let s_a = sequence_number(&e_a, 5).unwrap();
    let s_b = sequence_number(&e_b, 5).unwrap();
    assert_ne!(s_a, s_b, "不同 sender 不同 seq");
}

#[test]
fn integration_match_pair_after_deserialize() {
    let e = Envelope::new("alice", "bob", "ping", json!({"v": 1}));
    let s = to_json_string(&e).unwrap();
    let back = from_json_string(&s).unwrap();
    assert!(matches_pair(&back, "alice", "ping"));
    assert!(!matches_pair(&back, "alice", "pong"));
}
