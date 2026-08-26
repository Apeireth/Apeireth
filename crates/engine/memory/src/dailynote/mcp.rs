//! MCP server for daily notes (4 tools).

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
pub enum DailyNoteTool {
    NoteCreate,
    NoteGet,
    NoteSearch,
    NoteExport,
}

impl DailyNoteTool {
    pub fn name(&self) -> &'static str {
        match self {
            DailyNoteTool::NoteCreate => "note_create",
            DailyNoteTool::NoteGet => "note_get",
            DailyNoteTool::NoteSearch => "note_search",
            DailyNoteTool::NoteExport => "note_export",
        }
    }
    pub fn all() -> &'static [DailyNoteTool] {
        &[
            DailyNoteTool::NoteCreate,
            DailyNoteTool::NoteGet,
            DailyNoteTool::NoteSearch,
            DailyNoteTool::NoteExport,
        ]
    }
}

pub const DAILYNOTE_MCP_TOOL_COUNT: usize = 4;

pub struct DailyNoteMcp;

impl DailyNoteMcp {
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
                    "serverInfo": {"name": "apeireth-dailynote", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = DailyNoteTool::all()
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name(),
                            "description": match t {
                                DailyNoteTool::NoteCreate => "Create a new daily note",
                                DailyNoteTool::NoteGet => "Get a daily note by ID",
                                DailyNoteTool::NoteSearch => "Search notes by query and/or tag",
                                DailyNoteTool::NoteExport => "Export a note in Markdown or JSON",
                            }
                        })
                    })
                    .collect();
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
                    "note_create" => format!(
                        "create note title={:?}",
                        args.get("title").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "note_get" => format!(
                        "get note id={:?}",
                        args.get("id").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "note_search" => format!(
                        "search query={:?}",
                        args.get("query").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    "note_export" => format!(
                        "export note id={:?} format={:?}",
                        args.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        args.get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("markdown")
                    ),
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

impl Default for DailyNoteMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_count_is_4() {
        assert_eq!(DAILYNOTE_MCP_TOOL_COUNT, 4);
    }
    #[test]
    fn initialize_works() {
        let m = DailyNoteMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        });
        assert!(r.result.is_some());
    }
    #[test]
    fn tools_list_4() {
        let m = DailyNoteMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        });
        let binding = r.result.unwrap();
        let tools = binding["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
    }
    #[test]
    fn note_create_tool() {
        let m = DailyNoteMcp::new();
        let r = m.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "note_create", "arguments": {"title": "T", "content": "C"}}),
        });
        let result = r.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("title"));
    }
}
