//! ACP - Agent Communication Protocol (从 v1.0 apeireth-acp 1.1K LOC 收敛)
//!
//! 0 装 PASS: 简化版 JSON-RPC 2.0 (request/response/notification), 
//! 完整 v1.0 era 不做 (transport 层如 WebSocket / Unix domain socket)

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: u64,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Value, id: u64) -> Self {
        Self { jsonrpc: "2.0".into(), method: method.into(), params, id }
    }
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

impl JsonRpcResponse {
    /// 0 装 PASS: 真实成功响应 (result 不带 error, 0 装 PASS 不假装)
    pub fn success(id: u64, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), result: Some(result), error: None, id }
    }

    /// 0 装 PASS: 真实错误响应 (-32601 Method not found 等)
    pub fn error(id: u64, code: i32, message: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0".into(), result: None, error: Some(JsonRpcError { code, message: message.into(), data: None }), id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_request_serialization() {
        let req = JsonRpcRequest::new("echo", serde_json::json!({"x": 1}), 42);
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"method\":\"echo\""));
        assert!(s.contains("\"id\":42"));
    }
    #[test] fn test_success_response() {
        let r = JsonRpcResponse::success(1, serde_json::json!("ok"));
        assert!(r.result.is_some());
        assert!(r.error.is_none());
    }
    #[test] fn test_error_response() {
        let r = JsonRpcResponse::error(1, -32601, "Method not found");
        assert!(r.result.is_none());
        assert!(r.error.is_some());
        assert_eq!(r.error.unwrap().code, -32601);
    }
}
