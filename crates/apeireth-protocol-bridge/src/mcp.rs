//! MCP server for vcp-bridge (2 tools).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError { pub code: i32, pub message: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeTool {
    DetectProtocol,
    ConvertRequest,
}

impl BridgeTool {
    pub fn name(&self) -> &'static str {
        match self {
            BridgeTool::DetectProtocol => "detect_protocol",
            BridgeTool::ConvertRequest => "convert_request",
        }
    }
    pub fn all() -> &'static [BridgeTool] {
        &[BridgeTool::DetectProtocol, BridgeTool::ConvertRequest]
    }
}

pub const BRIDGE_MCP_TOOL_COUNT: usize = 2;

pub struct CompatBridgeMcp;

impl CompatBridgeMcp {
    pub fn new() -> Self { Self }
    pub fn handle(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "apeireth-protocol-bridge", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = BridgeTool::all().iter().map(|t| json!({
                    "name": t.name(),
                    "description": match t {
                        BridgeTool::DetectProtocol => "Detect VCP protocol from hints",
                        BridgeTool::ConvertRequest => "Convert a request between protocols",
                    }
                })).collect();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({"tools": tools})),
                    error: None,
                }
            }
            "tools/call" => {
                let tool = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
                let summary = match tool {
                    "detect_protocol" => format!("path={:?}", args.get("path").and_then(|v| v.as_str()).unwrap_or("")),
                    "convert_request" => format!("protocol={:?}", args.get("protocol").and_then(|v| v.as_str()).unwrap_or("")),
                    other => format!("unknown tool: {}", other),
                };
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({
                        "content": [{"type": "text", "text": summary}],
                        "isError": false
                    })),
                    error: None,
                }
            }
            "ping" => McpResponse { jsonrpc: "2.0".to_string(), id: req.id, result: Some(json!({})), error: None },
            other => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(McpError { code: -32601, message: format!("method not found: {}", other) }),
            },
        }
    }
}

impl Default for CompatBridgeMcp { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_count_is_2() { assert_eq!(BRIDGE_MCP_TOOL_COUNT, 2); }
    #[test]
    fn initialize() {
        let m = CompatBridgeMcp::new();
        let r = m.handle(McpRequest { jsonrpc: "2.0".to_string(), id: Some(json!(1)), method: "initialize".to_string(), params: json!({}) });
        assert!(r.result.is_some());
    }
    #[test]
    fn tools_list() {
        let m = CompatBridgeMcp::new();
        let r = m.handle(McpRequest { jsonrpc: "2.0".to_string(), id: Some(json!(2)), method: "tools/list".to_string(), params: json!({}) });
        let binding = r.result.unwrap();
        let tools = binding["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
    }
    #[test]
    fn detect_protocol_tool() {
        let m = CompatBridgeMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "detect_protocol", "arguments": {"path": "/v1/messages"}}),
        });
        let result = r.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("/v1/messages"));
    }
}