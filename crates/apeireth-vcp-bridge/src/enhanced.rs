//! EnhancedVcpBridge composed entry.

use crate::bridge::VcpBridge;
use crate::mcp::{VcpBridgeMcp, McpRequest, McpResponse};
use crate::protocol::{ProtocolHints, VcpProtocol};

pub struct EnhancedVcpBridge {
    bridge: VcpBridge,
    mcp: VcpBridgeMcp,
}

impl EnhancedVcpBridge {
    pub fn new() -> Self {
        Self { bridge: VcpBridge::new(), mcp: VcpBridgeMcp::new() }
    }
    pub fn detect(&self, hints: &ProtocolHints) -> VcpProtocol {
        self.bridge.detect(hints)
    }
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        self.mcp.handle(req)
    }
}

impl Default for EnhancedVcpBridge { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detect_via_path() {
        let e = EnhancedVcpBridge::new();
        let p = e.detect(&ProtocolHints { path: Some("/v1/chat/completions".into()), ..Default::default() });
        assert_eq!(p, VcpProtocol::OpenAIChatCompletions);
    }

    #[test]
    fn dispatch_mcp() {
        let e = EnhancedVcpBridge::new();
        let r = e.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        });
        assert!(r.result.is_some());
    }
}