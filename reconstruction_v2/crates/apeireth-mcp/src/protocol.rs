//! JSON-RPC 2.0 基础类型 (从 v1 era apeireth-mcp/protocol.rs 抄录升级)

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JSON_RPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcBatch { Single(JsonRpcRequest), Batch(Vec<JsonRpcRequest>) }

pub type Id = u64;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_request_serde() {
        let r = JsonRpcRequest { jsonrpc: "2.0".into(), method: "x".into(), params: serde_json::json!({}), id: 1 };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains(""method":"x""));
    }
    #[test]
    fn test_error_serde() {
        let e = JsonRpcError { code: -32601, message: "not found".into(), data: None };
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("-32601"));
    }
    #[test]
    fn test_batch() {
        let b = JsonRpcBatch::Batch(vec![]);
        let _ = serde_json::to_string(&b).unwrap();
    }
}
