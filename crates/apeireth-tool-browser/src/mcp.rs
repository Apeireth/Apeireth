//! MCP JSON-RPC 2.0 server interface for browser.
//!
//! Per v2 plan §9.4: dual mode (CLI + MCP). Coding agents prefer CLI (token
//! efficiency); long-running agents use MCP (standard protocol).
//!
//! We implement the MCP `initialize` / `tools/list` / `tools/call` flow
//! over stdin/stdout (newline-delimited JSON-RPC 2.0).
//!
//! Tools exposed:
//! - `browser_navigate` — params: { url: string }
//! - `browser_snapshot` — params: { kind?: "full"|"text"|"refs" }
//! - `browser_extract`  — params: {}

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
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        Self
    }

    /// Handle a single MCP request and return a response.
    pub fn handle(req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "apeireth-browser",
                        "version": "1.2.0"
                    },
                    "capabilities": {
                        "tools": {}
                    }
                })),
                error: None,
            },
            "tools/list" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: Some(json!({
                    "tools": [
                        {
                            "name": "browser_navigate",
                            "description": "Navigate to a URL and return page snapshot",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "url": { "type": "string" }
                                },
                                "required": ["url"]
                            }
                        },
                        {
                            "name": "browser_snapshot",
                            "description": "Get current page snapshot (full / text / refs)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "kind": {
                                        "type": "string",
                                        "enum": ["full", "text", "refs"]
                                    }
                                }
                            }
                        },
                        {
                            "name": "browser_extract",
                            "description": "Extract text content for LLM context",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                })),
                error: None,
            },
            "tools/call" => {
                let tool_name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
                match tool_name {
                    "browser_navigate" => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(json!({
                            "content": [{"type": "text", "text": format!(
                                "navigate called with url={}", arguments.get("url").and_then(|v| v.as_str()).unwrap_or("")
                            )}],
                            "isError": false
                        })),
                        error: None,
                    },
                    "browser_snapshot" => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(json!({
                            "content": [{"type": "text", "text": "(stub) snapshot"}],
                            "isError": false
                        })),
                        error: None,
                    },
                    "browser_extract" => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(json!({
                            "content": [{"type": "text", "text": "(stub) extract"}],
                            "isError": false
                        })),
                        error: None,
                    },
                    other => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(McpError {
                            code: -32602,
                            message: format!("unknown tool: {}", other),
                            data: None,
                        }),
                    },
                }
            }
            "ping" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({})),
                error: None,
            },
            other => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("method not found: {}", other),
                    data: None,
                }),
            },
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a JSON-RPC request from a string.
pub fn parse_request(s: &str) -> Result<McpRequest, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_protocol_version() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = McpServer::handle(req);
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "apeireth-browser");
    }

    #[test]
    fn tools_list_returns_3_tools() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = McpServer::handle(req);
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"browser_navigate"));
        assert!(names.contains(&"browser_snapshot"));
        assert!(names.contains(&"browser_extract"));
    }

    #[test]
    fn tools_call_navigate() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "browser_navigate", "arguments": {"url": "https://example.com"}}),
        };
        let resp = McpServer::handle(req);
        let result = resp.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("example.com"));
    }

    #[test]
    fn tools_call_unknown_tool_errors() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({"name": "nope", "arguments": {}}),
        };
        let resp = McpServer::handle(req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32602);
    }

    #[test]
    fn method_not_found() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "wat".to_string(),
            params: json!({}),
        };
        let resp = McpServer::handle(req);
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn ping_responds() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "ping".to_string(),
            params: json!({}),
        };
        let resp = McpServer::handle(req);
        assert!(resp.result.is_some());
    }

    #[test]
    fn parse_request_valid() {
        let s = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let r = parse_request(s).unwrap();
        assert_eq!(r.method, "initialize");
    }
}