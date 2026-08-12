//! MCP server for image-gen — 2 tools.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::generators::{default_registry, MockProvider};
use crate::params::ImageGenParams;
use crate::provider::{ImageGenProvider, ProviderRegistry};

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
pub enum ImageMcpTool {
    ImageGenerate,
    ListProviders,
}

impl ImageMcpTool {
    pub fn name(&self) -> &'static str {
        match self {
            ImageMcpTool::ImageGenerate => "image_generate",
            ImageMcpTool::ListProviders => "list_providers",
        }
    }
    pub fn all() -> &'static [ImageMcpTool] {
        &[ImageMcpTool::ImageGenerate, ImageMcpTool::ListProviders]
    }
}

pub const IMAGE_MCP_TOOL_COUNT: usize = 2;

pub struct ImageGenMcp {
    registry: ProviderRegistry,
}

impl ImageGenMcp {
    pub fn new() -> Self {
        Self { registry: default_registry() }
    }
    pub fn with_registry(registry: ProviderRegistry) -> Self {
        Self { registry }
    }
    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }

    pub fn handle(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "apeireth-image-gen", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = ImageMcpTool::all().iter().map(|t| json!({
                    "name": t.name(),
                    "description": match t {
                        ImageMcpTool::ImageGenerate => "Generate image from prompt (uses mock provider by default)",
                        ImageMcpTool::ListProviders => "List available image generation providers",
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
                let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
                match tool_name {
                    "list_providers" => {
                        let names = self.registry.names();
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": format!("providers ({}): {}", names.len(), names.join(", "))}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "image_generate" => {
                        let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("untitled");
                        let provider_name = args.get("provider").and_then(|v| v.as_str()).unwrap_or("mock");
                        let params = ImageGenParams::new(prompt);
                        let result = if provider_name == "mock" {
                            // Synchronous mock call wrapped in tokio
                            tokio::task::block_in_place(|| {
                                let handle = tokio::runtime::Handle::current();
                                handle.block_on(async {
                                    MockProvider::new().generate(&params).await
                                })
                            })
                        } else {
                            // Other providers require API keys; report honestly
                            return McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(json!({
                                    "content": [{"type": "text", "text": format!("provider `{}` requires API key (not configured)", provider_name)}],
                                    "isError": true
                                })),
                                error: None,
                            };
                        };
                        match result {
                            Ok(r) => {
                                let summary = format!("provider={} model={} count={}", r.provider, r.model, r.images.len());
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
                            Err(e) => McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(json!({
                                    "content": [{"type": "text", "text": format!("error: {}", e)}],
                                    "isError": true
                                })),
                                error: None,
                            },
                        }
                    }
                    other => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(McpError { code: -32602, message: format!("unknown tool: {}", other) }),
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
                error: Some(McpError { code: -32601, message: format!("method not found: {}", other) }),
            },
        }
    }
}

impl Default for ImageGenMcp {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_count_is_2() {
        assert_eq!(IMAGE_MCP_TOOL_COUNT, 2);
        assert_eq!(ImageMcpTool::all().len(), 2);
    }

    #[test]
    fn initialize_works() {
        let mcp = ImageGenMcp::new();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let r = mcp.handle(req);
        assert!(r.result.is_some());
    }

    #[test]
    fn list_providers_via_mcp() {
        let mcp = ImageGenMcp::new();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/call".to_string(),
            params: json!({"name": "list_providers", "arguments": {}}),
        };
        let r = mcp.handle(req);
        let result = r.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("mock"));
        assert!(text.contains("openai-dalle"));
    }

    #[test]
    fn image_generate_mock_succeeds() {
        // mcp tool calls block_in_place which requires multi-thread runtime
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mcp = ImageGenMcp::new();
            let req = McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(3)),
                method: "tools/call".to_string(),
                params: json!({"name": "image_generate", "arguments": {"prompt": "a cat", "provider": "mock"}}),
            };
            let r = mcp.handle(req);
            let result = r.result.unwrap();
            let text = result["content"][0]["text"].as_str().unwrap();
            assert!(text.contains("mock"));
        });
    }

    #[test]
    fn image_generate_unknown_provider_errors() {
        let mcp = ImageGenMcp::new();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({"name": "image_generate", "arguments": {"prompt": "x", "provider": "openai-dalle"}}),
        };
        let r = mcp.handle(req);
        let result = r.result.unwrap();
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("requires API key"));
    }
}