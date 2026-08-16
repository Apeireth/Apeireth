//! MCP conformance tests
//!
//! **R18 第 2 阶段第 9 项**: 9 conformance tests for MCP 2025-03-26 spec

use apeireth_mcp::{
    protocol::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse},
    ServerCapabilities, ServerIdentity, ServerInfo, ToolsCapability,
};
use serde_json::{json, Value};

// =====================================================================
// JSON-RPC 2.0 消息结构
// =====================================================================

#[test]
fn jsonrpc_request_id_can_be_number() {
    let req = JsonRpcRequest::new("tools/list", Some(json!({})), Id::Num(42));
    let serialized = serde_json::to_value(&req).unwrap();
    assert_eq!(serialized["id"], 42);
    assert_eq!(serialized["jsonrpc"], "2.0");
    assert_eq!(serialized["method"], "tools/list");
}

#[test]
fn jsonrpc_request_id_can_be_string() {
    let req = JsonRpcRequest::new("initialize", Some(json!({})), Id::Str("req-1".to_string()));
    let serialized = serde_json::to_value(&req).unwrap();
    assert_eq!(serialized["id"], "req-1");
}

#[test]
fn jsonrpc_request_can_be_notification() {
    let req = JsonRpcRequest::notification("notifications/initialized", Some(json!({})));
    let serialized = serde_json::to_value(&req).unwrap();
    assert!(serialized.get("id").is_none(), "notification has no id");
    assert_eq!(serialized["method"], "notifications/initialized");
}

#[test]
fn jsonrpc_response_ok() {
    let resp = JsonRpcResponse::ok(Some(Id::Num(1)), json!({"status": "ok"}));
    let serialized = serde_json::to_value(&resp).unwrap();
    assert_eq!(serialized["id"], 1);
    assert_eq!(serialized["result"]["status"], "ok");
    assert!(serialized.get("error").is_none());
}

#[test]
fn jsonrpc_response_err() {
    let err = JsonRpcError::new(-32601, "Method not found");
    let resp = JsonRpcResponse::err(Some(Id::Num(1)), err);
    let serialized = serde_json::to_value(&resp).unwrap();
    assert_eq!(serialized["id"], 1);
    assert_eq!(serialized["error"]["code"], -32601);
    assert_eq!(serialized["error"]["message"], "Method not found");
    assert!(serialized.get("result").is_none());
}

#[test]
fn jsonrpc_error_with_data() {
    let err = JsonRpcError::new(-32602, "Invalid params").with_data(json!({"field": "name"}));
    let serialized = serde_json::to_value(&err).unwrap();
    assert_eq!(serialized["data"]["field"], "name");
}

#[test]
fn jsonrpc_error_into_result() {
    let err = JsonRpcError::new(-1, "fail");
    let resp = JsonRpcResponse::err(Some(Id::Num(1)), err);
    let r = resp.into_result();
    assert!(r.is_err());
}

// =====================================================================
// MCP 2025-03-26 InitializeResponse 形状
// =====================================================================

#[test]
fn server_info_serialization() {
    let info = ServerInfo {
        protocolVersion: "2025-03-26".to_string(),
        serverInfo: ServerIdentity {
            name: "test-server".to_string(),
            version: "0.1.0".to_string(),
        },
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { listChanged: false }),
        },
    };
    let serialized = serde_json::to_value(&info).unwrap();
    assert_eq!(serialized["protocolVersion"], "2025-03-26");
    assert_eq!(serialized["serverInfo"]["name"], "test-server");
    assert_eq!(serialized["serverInfo"]["version"], "0.1.0");
    assert_eq!(serialized["capabilities"]["tools"]["listChanged"], false);
}

#[test]
fn server_capabilities_without_tools() {
    let caps = ServerCapabilities { tools: None };
    let serialized = serde_json::to_value(&caps).unwrap();
    assert!(
        serialized.get("tools").is_none(),
        "tools field should be skipped when None"
    );
}
