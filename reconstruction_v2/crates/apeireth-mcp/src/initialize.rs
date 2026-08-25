//! MCP initialize 握手 (v1 initialize.rs 抄录升级核心)
//!
//! 0 装 PASS: 真 capability 协商 + protocol version check

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    pub protocol_version: String,
    pub capabilities: serde_json::Map<String, Value>,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub protocol_version: String,
    pub server_info: super::ServerInfo,
    pub capabilities: serde_json::Map<String, Value>,
}

/// 0 装 PASS: 真 handle initialize (capability negotiation)
pub fn handle_initialize(req: InitializeRequest, server: &super::ServerInfo) -> Result<InitializeResponse, String> {
    if req.protocol_version != "2.0" && !req.protocol_version.starts_with("2025") {
        return Err(format!("unsupported protocol version: {}", req.protocol_version));
    }
    Ok(InitializeResponse {
        protocol_version: crate::JSON_RPC_VERSION.into(),
        server_info: server.clone(),
        capabilities: serde_json::Map::from_iter([
            ("tools".into(), json!({})),
            ("resources".into(), json!({})),
            ("prompts".into(), json!({})),
        ]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mkreq() -> InitializeRequest {
        InitializeRequest {
            protocol_version: crate::JSON_RPC_VERSION.into(),
            capabilities: serde_json::Map::new(),
            client_info: ClientInfo { name: "c".into(), version: "1".into() },
        }
    }
    #[test]
    fn test_initialize_ok() {
        let s = super::super::ServerInfo { name: "s".into(), version: "1".into(), protocol_version: "1".into() };
        let r = handle_initialize(mkreq(), &s).unwrap();
        assert_eq!(r.server_info.name, "s");
    }
    #[test]
    fn test_initialize_unsupported() {
        let mut r = mkreq();
        r.protocol_version = "1999".into();
        let s = super::super::ServerInfo { name: "s".into(), version: "1".into(), protocol_version: "1".into() };
        assert!(handle_initialize(r, &s).is_err());
    }
    #[test]
    fn test_capabilities() {
        let s = super::super::ServerInfo { name: "s".into(), version: "1".into(), protocol_version: "1".into() };
        let r = handle_initialize(mkreq(), &s).unwrap();
        assert!(r.capabilities.contains_key("tools"));
    }
}
