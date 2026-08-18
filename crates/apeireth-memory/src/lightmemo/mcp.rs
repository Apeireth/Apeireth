//! MCP server for LightMemo (5 tools).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightMemoTool {
    MemoryAdd,
    MemoryGet,
    MemorySearch,
    DecayCheck,
    DreamCycle,
}

impl LightMemoTool {
    pub fn name(&self) -> &'static str {
        match self {
            LightMemoTool::MemoryAdd => "memory_add",
            LightMemoTool::MemoryGet => "memory_get",
            LightMemoTool::MemorySearch => "memory_search",
            LightMemoTool::DecayCheck => "decay_check",
            LightMemoTool::DreamCycle => "dream_cycle",
        }
    }
    pub fn all() -> &'static [LightMemoTool] {
        &[
            LightMemoTool::MemoryAdd,
            LightMemoTool::MemoryGet,
            LightMemoTool::MemorySearch,
            LightMemoTool::DecayCheck,
            LightMemoTool::DreamCycle,
        ]
    }
}

pub const LIGHTMEMO_MCP_TOOL_COUNT: usize = 5;

pub struct LightMemoMcp;

impl LightMemoMcp {
    pub fn new() -> Self {
        Self
    }
    pub fn handle(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "apeireth-lightmemo", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = LightMemoTool::all().iter().map(|t| json!({
                    "name": t.name(),
                    "description": match t {
                        LightMemoTool::MemoryAdd => "Add a memory item across all 4 layers",
                        LightMemoTool::MemoryGet => "Get a memory item by ID",
                        LightMemoTool::MemorySearch => "Search via multi-pipe (keyword + vector + tag)",
                        LightMemoTool::DecayCheck => "Check decay strength of an item",
                        LightMemoTool::DreamCycle => "Run a dream consolidation cycle",
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
                let tool = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
                let summary = match tool {
                    "memory_add" => format!(
                        "add content={:?}",
                        args.get("content").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "memory_get" => format!(
                        "get id={:?}",
                        args.get("id").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "memory_search" => format!(
                        "search query={:?}",
                        args.get("query").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "decay_check" => format!(
                        "decay id={:?}",
                        args.get("id").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "dream_cycle" => "dream cycle".to_string(),
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
                }),
            },
        }
    }
}

impl Default for LightMemoMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_count() {
        assert_eq!(LIGHTMEMO_MCP_TOOL_COUNT, 5);
    }
    #[test]
    fn initialize() {
        let m = LightMemoMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        });
        assert!(r.result.is_some());
    }
    #[test]
    fn tools_list_5() {
        let m = LightMemoMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        });
        let binding = r.result.unwrap();
        let tools = binding["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
    }
    #[test]
    fn memory_add_tool() {
        let m = LightMemoMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "memory_add", "arguments": {"content": "hello"}}),
        });
        let result = r.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("hello"));
    }
}
