//! EnhancedCompatBridge composed entry.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use crate::bridge::CompatBridge;
use crate::mcp::{CompatBridgeMcp, McpRequest, McpResponse};
use crate::protocol::{ProtocolHints, CompatProtocol};

pub struct EnhancedCompatBridge {
    bridge: CompatBridge,
    mcp: CompatBridgeMcp,
}

impl EnhancedCompatBridge {
    pub fn new() -> Self {
        Self { bridge: CompatBridge::new(), mcp: CompatBridgeMcp::new() }
    }
    pub fn detect(&self, hints: &ProtocolHints) -> CompatProtocol {
        self.bridge.detect(hints)
    }
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        self.mcp.handle(req)
    }
}

impl Default for EnhancedCompatBridge { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detect_via_path() {
        let e = EnhancedCompatBridge::new();
        let p = e.detect(&ProtocolHints { path: Some("/v1/chat/completions".into()), ..Default::default() });
        assert_eq!(p, CompatProtocol::OpenAIChatCompletions);
    }

    #[test]
    fn dispatch_mcp() {
        let e = EnhancedCompatBridge::new();
        let r = e.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        });
        assert!(r.result.is_some());
    }
}