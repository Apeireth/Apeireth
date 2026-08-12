//! Bridge: orchestrates detect + convert + audit.

use chrono::Utc;
use serde_json::Value;
use thiserror::Error;

use crate::audit::{AuditDirection, AuditEntry, AuditLog};
use crate::convert::{convert_request, convert_response};
use crate::detect::detect_protocol;
use crate::protocol::{ProtocolHints, CompatProtocol};

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("unknown protocol (no hints matched)")]
    UnknownProtocol,
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
}

pub struct CompatBridge {
    audit: AuditLog,
}

impl CompatBridge {
    pub fn new() -> Self {
        Self { audit: AuditLog::default() }
    }
    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }
    /// Detect protocol from hints.
    pub fn detect(&self, hints: &ProtocolHints) -> CompatProtocol {
        detect_protocol(hints)
    }
    /// Convert incoming request + record audit entry.
    pub fn incoming(&mut self, hints: &ProtocolHints, body: Value) -> Result<(CompatProtocol, Value), BridgeError> {
        let proto = self.detect(hints);
        if proto == CompatProtocol::Unknown {
            return Err(BridgeError::UnknownProtocol);
        }
        let converted = convert_request(proto, body);
        self.audit.record(AuditEntry {
            timestamp: Utc::now(),
            protocol: proto,
            direction: AuditDirection::Request,
            status: 200,
        });
        Ok((proto, converted))
    }
    /// Convert outgoing response + record audit entry.
    pub fn outgoing(&mut self, proto: CompatProtocol, internal: Value) -> Value {
        let resp = convert_response(proto, internal);
        self.audit.record(AuditEntry {
            timestamp: Utc::now(),
            protocol: proto,
            direction: AuditDirection::Response,
            status: 200,
        });
        resp
    }
}

impl Default for CompatBridge { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detect_and_convert() {
        let mut b = CompatBridge::new();
        let hints = ProtocolHints { path: Some("/v1/messages".into()), ..Default::default() };
        let (proto, body) = b.incoming(&hints, json!({"model": "c", "messages": []})).unwrap();
        assert_eq!(proto, CompatProtocol::AnthropicMessages);
        assert_eq!(body["max_tokens"], 4096);
        assert_eq!(b.audit().count(), 1);
    }

    #[test]
    fn unknown_protocol_errors() {
        let mut b = CompatBridge::new();
        let hints = ProtocolHints { path: Some("/random".into()), ..Default::default() };
        let r = b.incoming(&hints, json!({}));
        assert!(matches!(r, Err(BridgeError::UnknownProtocol)));
    }

    #[test]
    fn outgoing_audit() {
        let mut b = CompatBridge::new();
        let r = b.outgoing(CompatProtocol::OpenAIChatCompletions, json!({"x": 1}));
        assert_eq!(r["x"], 1);
        assert_eq!(b.audit().count(), 1);
    }
}