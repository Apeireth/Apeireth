//! MCP server for image-process.

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::router::{ImageRouter, ProcessOp};

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
pub enum ImageProcessTool {
    ImageHash,
    ImageExif,
    ImageOcr,
    ImageThumbnail,
}

impl ImageProcessTool {
    pub fn name(&self) -> &'static str {
        match self {
            ImageProcessTool::ImageHash => "image_hash",
            ImageProcessTool::ImageExif => "image_exif",
            ImageProcessTool::ImageOcr => "image_ocr",
            ImageProcessTool::ImageThumbnail => "image_thumbnail",
        }
    }
    pub fn all() -> &'static [ImageProcessTool] {
        &[ImageProcessTool::ImageHash, ImageProcessTool::ImageExif, ImageProcessTool::ImageOcr, ImageProcessTool::ImageThumbnail]
    }
}

pub const IMAGE_PROC_MCP_TOOL_COUNT: usize = 4;

pub struct ImageProcessMcp;

impl ImageProcessMcp {
    pub fn new() -> Self { Self }
    pub fn handle(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "apeireth-image-process", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = ImageProcessTool::all().iter().map(|t| json!({
                    "name": t.name(),
                    "description": match t {
                        ImageProcessTool::ImageHash => "Compute perceptual hash",
                        ImageProcessTool::ImageExif => "Extract EXIF data (stub)",
                        ImageProcessTool::ImageOcr => "OCR extract (stub)",
                        ImageProcessTool::ImageThumbnail => "Generate thumbnail metadata (stub)",
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
                let data_b64 = args.get("data").and_then(|v| v.as_str()).unwrap_or("");
                let data = base64_decode(data_b64);
                let lang = args.get("language").and_then(|v| v.as_str());
                let router = ImageRouter::new();
                let result_text = match tool {
                    "image_hash" => router.dispatch(ProcessOp::Hash, &data, lang).unwrap_or_else(|e| format!("error: {}", e)),
                    "image_exif" => router.dispatch(ProcessOp::Exif, &data, lang).unwrap_or_else(|e| format!("error: {}", e)),
                    "image_ocr" => router.dispatch(ProcessOp::Ocr, &data, lang).unwrap_or_else(|e| format!("error: {}", e)),
                    "image_thumbnail" => router.dispatch(ProcessOp::Thumbnail, &data, lang).unwrap_or_else(|e| format!("error: {}", e)),
                    other => format!("unknown tool: {}", other),
                };
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({
                        "content": [{"type": "text", "text": result_text}],
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
                error: Some(McpError { code: -32601, message: format!("method not found: {}", other) }),
            },
        }
    }
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).unwrap_or_default()
}

impl Default for ImageProcessMcp {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_count_is_4() {
        assert_eq!(IMAGE_PROC_MCP_TOOL_COUNT, 4);
    }

    #[test]
    fn initialize_works() {
        let mcp = ImageProcessMcp::new();
        let r = mcp.handle(McpRequest { jsonrpc: "2.0".to_string(), id: Some(json!(1)), method: "initialize".to_string(), params: json!({}) });
        assert!(r.result.is_some());
    }

    #[test]
    fn tools_list_4() {
        let mcp = ImageProcessMcp::new();
        let r = mcp.handle(McpRequest { jsonrpc: "2.0".to_string(), id: Some(json!(2)), method: "tools/list".to_string(), params: json!({}) });
        let binding = r.result.unwrap();
        let tools = binding["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
    }

    #[test]
    fn image_hash_tool() {
        let mcp = ImageProcessMcp::new();
        let r = mcp.handle(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "image_hash", "arguments": {"data": "aGVsbG8="}}),
        });
        let result = r.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("hash="));
    }
}