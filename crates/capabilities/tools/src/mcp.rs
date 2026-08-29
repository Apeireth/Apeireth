//! `apeireth-tools-canonical::mcp` — Model Context Protocol (MCP) 标准外部工具桥接.
//!
//! ## 核心哲学 (O-2 站在前人肩膀上 + S-3 质量工程化)
//! MCP 是大模型工具互联的标准开放协议 (JSON-RPC 2.0)。
//! 本模块实现纯 Safe Rust 的 MCP 客户端桥接器 (`McpClient`)：
//! - **协议握手 (`initialize`)**: 标准协议版本协商与能力清单同步；
//! - **动态工具发现 (`tools/list`)**: 将外部 MCP Server 工具转换为 Apeireth 统一工具契约；
//! - **安全隔离调用 (`tools/call`)**: 结构化参数封装与执行结果归一化；
//! - 纯 Safe Rust (`#![deny(unsafe_code)]`)，0 未定义行为。

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// MCP 通信与执行错误.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum McpError {
    #[error("JSON-RPC 协议错误: code={code}, message={message}")]
    JsonRpc { code: i64, message: String },
    #[error("MCP 工具未找到: {0}")]
    ToolNotFound(String),
    #[error("载荷序列化/反序列化失败: {0}")]
    Serialization(String),
    #[error("传输层 IO 失败: {0}")]
    Transport(String),
    #[error("MCP 握手失败: {0}")]
    HandshakeFailed(String),
}

/// JSON-RPC 2.0 请求格式.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 响应格式.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

/// JSON-RPC 错误对象.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP 工具元数据描述符.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP 调用响应内容块.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { uri: String, text: Option<String> },
}

/// MCP 工具执行结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

impl McpToolResult {
    /// 提取纯文本输出.
    pub fn extract_text(&self) -> String {
        let mut out = String::new();
        for c in &self.content {
            if let McpContent::Text { text } = c {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(text);
            }
        }
        out
    }
}

/// MCP 底层传输抽象 Trait.
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;
}

/// MCP 客户端桥接器.
pub struct McpClient {
    transport: Arc<dyn McpTransport>,
    server_name: String,
    cached_tools: HashMap<String, McpToolDescriptor>,
}

impl McpClient {
    pub fn new(server_name: impl Into<String>, transport: Arc<dyn McpTransport>) -> Self {
        Self {
            transport,
            server_name: server_name.into(),
            cached_tools: HashMap::new(),
        }
    }

    /// 执行 MCP 握手并同步能力.
    pub async fn initialize(&mut self) -> Result<(), McpError> {
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "apeireth-mcp-bridge",
                "version": "2.0.0"
            }
        });

        let req = JsonRpcRequest::new(1, "initialize", Some(init_params));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(McpError::HandshakeFailed(format!(
                "code={}: {}",
                err.code, err.message
            )));
        }

        // 握手后立即同步工具列表
        self.refresh_tools().await?;
        Ok(())
    }

    /// 刷新并缓存远端工具列表.
    pub async fn refresh_tools(&mut self) -> Result<Vec<McpToolDescriptor>, McpError> {
        let req = JsonRpcRequest::new(2, "tools/list", None);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(McpError::JsonRpc {
                code: err.code,
                message: err.message,
            });
        }

        let result = resp.result.ok_or_else(|| {
            McpError::Serialization("tools/list 返回空 result".to_string())
        })?;

        #[derive(Deserialize)]
        struct ToolsListResponse {
            tools: Vec<McpToolDescriptor>,
        }

        let parsed: ToolsListResponse = serde_json::from_value(result)
            .map_err(|e| McpError::Serialization(e.to_string()))?;

        self.cached_tools.clear();
        for t in &parsed.tools {
            self.cached_tools.insert(t.name.clone(), t.clone());
        }

        Ok(parsed.tools)
    }

    /// 获取已缓存的工具描述符.
    pub fn get_cached_tool(&self, name: &str) -> Option<&McpToolDescriptor> {
        self.cached_tools.get(name)
    }

    /// 调用远端 MCP 工具.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        if !self.cached_tools.contains_key(name) {
            return Err(McpError::ToolNotFound(name.to_string()));
        }

        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });

        let req = JsonRpcRequest::new(3, "tools/call", Some(params));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(McpError::JsonRpc {
                code: err.code,
                message: err.message,
            });
        }

        let result = resp.result.ok_or_else(|| {
            McpError::Serialization("tools/call 返回空 result".to_string())
        })?;

        let tool_result: McpToolResult = serde_json::from_value(result)
            .map_err(|e| McpError::Serialization(e.to_string()))?;

        Ok(tool_result)
    }

    /// MCP Server 名称.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockMcpTransport {
        requests: Arc<Mutex<Vec<JsonRpcRequest>>>,
    }

    #[async_trait]
    impl McpTransport for MockMcpTransport {
        async fn send_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
            self.requests.lock().unwrap().push(req.clone());

            match req.method.as_str() {
                "initialize" => Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": { "name": "mock-server", "version": "1.0" }
                    })),
                    error: None,
                }),
                "tools/list" => Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({
                        "tools": [
                            {
                                "name": "sqlite_query",
                                "description": "Execute a SQL query",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "query": { "type": "string" }
                                    }
                                }
                            }
                        ]
                    })),
                    error: None,
                }),
                "tools/call" => Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({
                        "content": [
                            {
                                "type": "text",
                                "text": "{\"rows\":[{\"id\":1,\"val\":\"ok\"}]}"
                            }
                        ],
                        "isError": false
                    })),
                    error: None,
                }),
                _ => Err(McpError::JsonRpc {
                    code: -32601,
                    message: "Method not found".to_string(),
                }),
            }
        }
    }

    #[tokio::test]
    async fn test_mcp_client_lifecycle_and_call() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = Arc::new(MockMcpTransport {
            requests: requests.clone(),
        });

        let mut client = McpClient::new("sqlite-mcp", transport);
        assert_eq!(client.server_name(), "sqlite-mcp");

        // 1. 握手并自动同步工具
        client.initialize().await.unwrap();

        let tool = client.get_cached_tool("sqlite_query");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().description, "Execute a SQL query");

        // 2. 调用已知工具
        let res = client
            .call_tool(
                "sqlite_query",
                serde_json::json!({ "query": "SELECT * FROM users;" }),
            )
            .await
            .unwrap();

        assert!(!res.is_error);
        assert!(res.extract_text().contains("rows"));

        // 3. 调用未知工具拦截
        let err = client
            .call_tool("unknown_tool", serde_json::json!({}))
            .await
            .unwrap_err();

        assert_eq!(err, McpError::ToolNotFound("unknown_tool".to_string()));
    }
}
