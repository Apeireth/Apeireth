//! R174 stage 6: Privacy Guard bridge at the gateway boundary.
//!
//! **Wire path**:
//! ```text
//!   Node (TUI/CLI/...) outbound
//!     -> GatewayGuard::redact_inbound_into (detect + redact → in-tree payload)
//!     -> runtime / bus / ...
//!     -> GatewayGuard::redact_outbound_from (redact before-send-to-Node)
//!     -> OutFrame -> Node
//! ```
//!
//! **What this module owns**:
//! - `GatewayGuard` — thin wrapper around `apeireth_guard::PrivacyGuard` that applies
//!   the redact/audit pipeline to `InFrame` / `OutFrame` JSON payloads.
//! - `AuditSummary` — small read-only snapshot of audit counters for diagnostics.
//!
//! **0 drift**:
//! - 0 monkey-patches of `apeireth-guard` (we use its public surface only).
//! - 0 unsafe, 0 IO.

#![deny(unsafe_code)]

use crate::transport::{InFrame, OutFrame};
use apeireth_guard::PrivacyGuard;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardSide {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone)]
pub struct GuardedFrame {
    pub frame: Value,
    pub matches: usize,
    pub redacted: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AuditSummary {
    pub frames_in: u64,
    pub frames_out: u64,
    pub pii_matches: u64,
    pub redactions: u64,
}

/// GatewayGuard — wraps PrivacyGuard and applies the pipeline to gateway frames.
pub struct GatewayGuard {
    inner: PrivacyGuard,
    summary: AuditSummary,
}

impl Default for GatewayGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayGuard {
    pub fn new() -> Self {
        Self {
            inner: PrivacyGuard::new(),
            summary: AuditSummary::default(),
        }
    }

    pub fn with_audit_capacity(capacity: usize) -> Self {
        Self {
            inner: PrivacyGuard::new().with_audit_capacity(capacity),
            summary: AuditSummary::default(),
        }
    }

    /// Redact the payload of an inbound frame. The frame's `payload` field is
    /// expected to be a JSON object containing a string `text` field; if absent,
    /// the payload is returned untouched.
    pub fn redact_inbound(&mut self, frame: &InFrame, now: i64) -> GuardedFrame {
        self.summary.frames_in += 1;
        let payload = frame.payload.clone();
        let (new_payload, matches, redacted) = self.redact_payload(payload, now);
        if matches > 0 {
            self.summary.pii_matches += matches as u64;
        }
        if redacted {
            self.summary.redactions += 1;
        }
        GuardedFrame {
            frame: new_payload,
            matches,
            redacted,
        }
    }

    /// Redact the payload of an outbound frame (same convention as inbound).
    pub fn redact_outbound(&mut self, frame: &OutFrame, now: i64) -> GuardedFrame {
        self.summary.frames_out += 1;
        let payload = frame.payload.clone();
        let (new_payload, matches, redacted) = self.redact_payload(payload, now);
        if matches > 0 {
            self.summary.pii_matches += matches as u64;
        }
        if redacted {
            self.summary.redactions += 1;
        }
        GuardedFrame {
            frame: new_payload,
            matches,
            redacted,
        }
    }

    pub fn summary(&self) -> &AuditSummary {
        &self.summary
    }

    pub fn audit_log_len(&self) -> usize {
        self.inner.audit().len()
    }

    fn redact_payload(&self, payload: Value, now: i64) -> (Value, usize, bool) {
        let mut redacted = false;
        let mut matches_total = 0usize;
        let new_payload = match payload {
            Value::Object(mut map) => {
                if let Some(Value::String(text)) = map.get("text").cloned() {
                    let r = self.inner.check_and_redact(&text, now);
                    matches_total = r.matches.len();
                    if r.matches.len() > 0 {
                        redacted = true;
                    }
                    map.insert("text".into(), Value::String(r.redacted_text));
                }
                Value::Object(map)
            }
            Value::String(text) => {
                let r = self.inner.check_and_redact(&text, now);
                matches_total = r.matches.len();
                if r.matches.len() > 0 {
                    redacted = true;
                }
                Value::String(r.redacted_text)
            }
            other => other,
        };
        (new_payload, matches_total, redacted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn guard_constructs_and_has_zero_summary() {
        let g = GatewayGuard::new();
        let s = g.summary();
        assert_eq!(s.frames_in, 0);
        assert_eq!(s.frames_out, 0);
        assert_eq!(s.pii_matches, 0);
        assert_eq!(s.redactions, 0);
    }

    #[test]
    fn guard_redacts_email_in_outbound() {
        let mut g = GatewayGuard::new();
        let frame = OutFrame::new(
            "loopback",
            json!({"text": "contact me at alice@example.com"}),
        );
        let out = g.redact_outbound(&frame, 100);
        assert!(out.redacted);
        assert!(out.matches >= 1);
        let redacted_text = out.frame["text"].as_str().unwrap();
        assert!(!redacted_text.contains("alice@example.com"));
    }

    #[test]
    fn guard_redacts_string_payload() {
        let mut g = GatewayGuard::new();
        let frame = OutFrame::new("loopback", Value::String("Phone: 138-1234-5678".into()));
        let out = g.redact_outbound(&frame, 0);
        assert!(out.redacted);
    }

    #[test]
    fn guard_redacts_inbound() {
        let mut g = GatewayGuard::new();
        let frame = InFrame::new(
            Uuid::new_v4(),
            "loopback",
            json!({"text": "SSN: 123-45-6789"}),
        );
        let out = g.redact_inbound(&frame, 0);
        assert!(out.redacted);
        assert!(out.matches >= 1);
    }

    #[test]
    fn guard_passthrough_when_no_pii() {
        let mut g = GatewayGuard::new();
        let frame = OutFrame::new("loopback", json!({"text": "hello world"}));
        let out = g.redact_outbound(&frame, 0);
        assert!(!out.redacted);
        assert_eq!(out.matches, 0);
        assert_eq!(out.frame, frame.payload);
    }

    #[test]
    fn guard_passthrough_when_no_text_field() {
        let mut g = GatewayGuard::new();
        let frame = OutFrame::new("loopback", json!({"data": 12345}));
        let out = g.redact_outbound(&frame, 0);
        assert!(!out.redacted);
        assert_eq!(out.frame, frame.payload);
    }

    #[test]
    fn guard_summary_increments() {
        let mut g = GatewayGuard::new();
        let out = OutFrame::new("loopback", json!({"text": "send to bob@example.com"}));
        let _ = g.redact_outbound(&out, 0);
        let _ = g.redact_outbound(&out, 0);
        let s = g.summary();
        assert_eq!(s.frames_out, 2);
        assert!(s.pii_matches >= 2);
        assert_eq!(s.redactions, 2);
    }

    #[test]
    fn guard_inbound_outbound_separate_counts() {
        let mut g = GatewayGuard::new();
        let inbound = InFrame::new(
            Uuid::new_v4(),
            "loopback",
            json!({"text": "alice@example.com"}),
        );
        let outbound = OutFrame::new("loopback", json!({"text": "bob@example.com"}));
        let _ = g.redact_inbound(&inbound, 0);
        let _ = g.redact_outbound(&outbound, 0);
        let s = g.summary();
        assert_eq!(s.frames_in, 1);
        assert_eq!(s.frames_out, 1);
    }

    #[test]
    fn guard_audit_log_records_each_match() {
        let mut g = GatewayGuard::with_audit_capacity(100);
        let frame = OutFrame::new(
            "loopback",
            json!({"text": "alice@example.com and bob@example.com"}),
        );
        let _ = g.redact_outbound(&frame, 0);
        // Two emails -> two audit events.
        assert!(g.audit_log_len() >= 2);
    }

    #[test]
    fn guard_default_works() {
        let g = GatewayGuard::default();
        assert_eq!(g.summary().frames_in, 0);
    }
}
