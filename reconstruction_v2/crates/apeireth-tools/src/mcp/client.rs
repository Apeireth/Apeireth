use async_trait::async_trait;
use std::sync::Arc;
use serde_json::Value;

use crate::{Tool, ToolDefinition, ToolResult, ToolError, RiskLevel, ToolRegistry};
use super::protocol::{
    JsonRpcRequest, JsonRpcNotification, InitializeParams, InitializeResult,
    ClientCapabilities, ImplementationInfo, McpToolDefinition, CallToolResult,
    McpContent, McpResource, McpResourceContents, MCP_VERSION,
};
use super::transport::McpTransport;

pub struct McpClient {
    transport: Arc<dyn McpTransport>,
}

impl McpClient {
    pub fn new(transport: Arc<dyn McpTransport>) -> Self {
        Self { transport }
    }

    pub async fn initialize(&self, client_name: &str, client_version: &str) -> Result<InitializeResult, String> {
        let params = InitializeParams {
            protocol_version: MCP_VERSION.into(),
            capabilities: ClientCapabilities::default(),
            client_info: ImplementationInfo {
                name: client_name.into(),
                version: client_version.into(),
            },
        };

        let req = JsonRpcRequest::new(1, "initialize", Some(serde_json::to_value(params).unwrap()));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(format!("MCP Initialize Error ({}): {}", err.code, err.message));
        }

        let result = resp.result.ok_or("MCP Initialize returned empty result")?;
        let init_result = serde_json::from_value::<InitializeResult>(result).map_err(|e| format!("Failed to parse InitializeResult: {}", e))?;

        // Send initialized notification
        let notif = JsonRpcNotification::new("notifications/initialized", None);
        let _ = self.transport.send_notification(notif).await;

        Ok(init_result)
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDefinition>, String> {
        let req = JsonRpcRequest::new(2, "tools/list", None);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(format!("MCP tools/list Error ({}): {}", err.code, err.message));
        }

        let result = resp.result.ok_or("MCP tools/list returned empty result")?;
        let tools_val = result.get("tools").cloned().unwrap_or(Value::Array(vec![]));
        serde_json::from_value::<Vec<McpToolDefinition>>(tools_val).map_err(|e| format!("Failed to parse tools list: {}", e))
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, String> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });

        let req = JsonRpcRequest::new(3, "tools/call", Some(params));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(format!("MCP tools/call Error ({}): {}", err.code, err.message));
        }

        let result = resp.result.ok_or("MCP tools/call returned empty result")?;
        serde_json::from_value::<CallToolResult>(result).map_err(|e| format!("Failed to parse CallToolResult: {}", e))
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>, String> {
        let req = JsonRpcRequest::new(4, "resources/list", None);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(format!("MCP resources/list Error ({}): {}", err.code, err.message));
        }

        let result = resp.result.ok_or("MCP resources/list returned empty result")?;
        let res_val = result.get("resources").cloned().unwrap_or(Value::Array(vec![]));
        serde_json::from_value::<Vec<McpResource>>(res_val).map_err(|e| format!("Failed to parse resources list: {}", e))
    }

    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContents, String> {
        let params = serde_json::json!({ "uri": uri });
        let req = JsonRpcRequest::new(5, "resources/read", Some(params));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(format!("MCP resources/read Error ({}): {}", err.code, err.message));
        }

        let result = resp.result.ok_or("MCP resources/read returned empty result")?;
        let contents_val = result.get("contents").and_then(|c| c.as_array()).and_then(|arr| arr.first()).cloned()
            .ok_or("Empty contents in resources/read")?;

        serde_json::from_value::<McpResourceContents>(contents_val).map_err(|e| format!("Failed to parse McpResourceContents: {}", e))
    }

    /// Automatically discovers all tools from remote MCP Server and registers them into Apeireth's ToolRegistry
    pub async fn discover_and_register_tools(
        client_arc: Arc<Self>,
        registry: &mut ToolRegistry,
    ) -> Result<usize, String> {
        let tools = client_arc.list_tools().await?;
        let count = tools.len();
        for mcp_tool in tools {
            let adapter = McpToolAdapter {
                client: client_arc.clone(),
                definition: mcp_tool,
            };
            registry.register(Arc::new(adapter));
        }
        Ok(count)
    }
}

// -----------------------------------------------------------------------------
// McpToolAdapter: Bridges any MCP Tool into Apeireth's Native Tool Trait
// -----------------------------------------------------------------------------

pub struct McpToolAdapter {
    client: Arc<McpClient>,
    definition: McpToolDefinition,
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.definition.name.clone(),
            description: self.definition.description.clone().unwrap_or_default(),
            risk_level: RiskLevel::Medium,
        }
    }


    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let mcp_res = self.client.call_tool(&self.definition.name, params).await
            .map_err(|e| ToolError::ExecutionFailed(format!("MCP Tool call failure: {}", e)))?;

        if mcp_res.is_error {
            let err_msg = mcp_res.content.iter().filter_map(|c| match c {
                McpContent::Text { text } => Some(text.as_str()),
                _ => None,
            }).collect::<Vec<_>>().join(" ");
            return Err(ToolError::ExecutionFailed(err_msg));
        }

        let text_output = mcp_res.content.iter().filter_map(|c| match c {
            McpContent::Text { text } => Some(text.clone()),
            _ => None,
        }).collect::<Vec<_>>().join("\n");

        Ok(ToolResult::success(text_output))
    }
}
