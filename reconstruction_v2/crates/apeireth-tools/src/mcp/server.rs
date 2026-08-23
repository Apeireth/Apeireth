use std::sync::Arc;


use crate::ToolRegistry;
use super::protocol::{
    JsonRpcRequest, JsonRpcResponse, InitializeResult, ServerCapabilities,
    ToolsCapability, ResourcesCapability, PromptsCapability, ImplementationInfo,
    McpToolDefinition, CallToolResult, McpContent, McpResource, MCP_VERSION,
};

pub struct McpServer {
    tool_registry: Arc<ToolRegistry>,
    server_name: String,
    server_version: String,
}

impl McpServer {
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            tool_registry,
            server_name: "apeireth-mcp-server".into(),
            server_version: "2.0.0".into(),
        }
    }

    pub fn with_info(tool_registry: Arc<ToolRegistry>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            tool_registry,
            server_name: name.into(),
            server_version: version.into(),
        }
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => {
                let init_result = InitializeResult {
                    protocol_version: MCP_VERSION.into(),
                    capabilities: ServerCapabilities {
                        tools: Some(ToolsCapability { list_changed: Some(true) }),
                        resources: Some(ResourcesCapability { subscribe: Some(false), list_changed: Some(false) }),
                        prompts: Some(PromptsCapability { list_changed: Some(false) }),
                        logging: None,
                    },
                    server_info: ImplementationInfo {
                        name: self.server_name.clone(),
                        version: self.server_version.clone(),
                    },
                    instructions: Some("Apeireth 2.0 Sovereign MCP Server. Provides sandboxed tools and ACT-R memory.".into()),
                };
                JsonRpcResponse::success(req.id, serde_json::to_value(init_result).unwrap())
            }
            "ping" => {
                JsonRpcResponse::success(req.id, serde_json::json!({}))
            }
            "tools/list" => {
                let defs = self.tool_registry.list_tools();
                let mcp_tools: Vec<McpToolDefinition> = defs.into_iter().map(|d| McpToolDefinition {
                    name: d.name,
                    description: Some(d.description),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                }).collect();

                JsonRpcResponse::success(req.id, serde_json::json!({ "tools": mcp_tools }))
            }
            "tools/call" => {
                let params = match req.params {
                    Some(p) => p,
                    None => return JsonRpcResponse::error(req.id, -32602, "Missing params in tools/call", None),
                };

                let name = match params.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n,
                    None => return JsonRpcResponse::error(req.id, -32602, "Missing tool name in params", None),
                };

                let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

                match self.tool_registry.execute(name, arguments).await {
                    Ok(tool_res) => {
                        let content = vec![McpContent::Text { text: tool_res.output }];
                        let call_res = CallToolResult {
                            content,
                            is_error: !tool_res.success,
                        };

                        JsonRpcResponse::success(req.id, serde_json::to_value(call_res).unwrap())
                    }
                    Err(err) => {
                        let call_res = CallToolResult {
                            content: vec![McpContent::Text { text: err.to_string() }],
                            is_error: true,
                        };
                        JsonRpcResponse::success(req.id, serde_json::to_value(call_res).unwrap())
                    }
                }
            }

            "resources/list" => {
                let resources = vec![
                    McpResource {
                        uri: "apeireth://memory/act-r".into(),
                        name: "ACT-R Cognitive Memory".into(),
                        description: Some("Apeireth episodic and semantic memory store with ACT-R base-level decay.".into()),
                        mime_type: Some("application/json".into()),
                    },
                    McpResource {
                        uri: "apeireth://system/audit".into(),
                        name: "SHA-256 Audit Trail".into(),
                        description: Some("Immutable tamper-evident forward-chaining audit trail.".into()),
                        mime_type: Some("application/json".into()),
                    },
                ];
                JsonRpcResponse::success(req.id, serde_json::json!({ "resources": resources }))
            }
            "prompts/list" => {
                JsonRpcResponse::success(req.id, serde_json::json!({ "prompts": [] }))
            }
            _ => {
                JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method), None)
            }
        }
    }
}
