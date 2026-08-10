//! R33-3: MCP resources protocol (MCP 真协议 §resources/list + resources/read)
//!
//! **MCP 协议 (per modelcontextprotocol/specification 2025-03-26)**:
//! - `resources/list` — 客户端列 server 端 resources (URI + metadata)
//! - `resources/read` — 客户端按 URI 读 resource 内容
//!
//! **Apeireth 真接 (本 module)**:
//! - `Resource` struct — URI / name / description / mimeType (MCP §resources/list item 1:1)
//! - `ResourceContent` — uri / mimeType / text (MCP §resources/read contents[0] 1:1)
//! - `ResourceServer` trait — `list() -> Vec<Resource>` + `read(uri) -> Result<ResourceContent, _>`
//! - `handle_resources_list(req) -> JsonRpcResponse` — 调 server.list() 包成 JSON-RPC 响应
//! - `handle_resources_read(req) -> JsonRpcResponse` — 调 server.read(uri) 包成 JSON-RPC 响应
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 JSON-RPC 2.0 基础 (protocol.rs 0 触碰)
//! - 0 引入 I/O / 网络 (server 注入, 0 真接)
//! - 0 业务耦合 (apeireth-mcp 0 依赖 tui/api, 任意 server impl 都能挂)

use crate::protocol::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse, JSON_RPC_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// MCP 错误码 (per MCP spec, -32000 ~ -32099 范围 server-define)
pub const RESOURCE_NOT_FOUND: i32 = -32001;
pub const RESOURCE_INVALID_URI: i32 = -32002;
pub const RESOURCE_READ_FAILED: i32 = -32003;

/// MCP Resource (per spec §resources/list item)
///
/// ```json
/// { "uri": "file:///x.rs", "name": "x.rs", "description": "...", "mimeType": "text/x-rust" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Resource {
    /// 唯一 URI (e.g. "file:///path/to/x.rs" or "apeireth://organ/memory")
    pub uri: String,
    /// 人类可读名 (e.g. "x.rs")
    pub name: String,
    /// 描述 (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type (e.g. "text/x-rust", "application/json")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl Resource {
    /// 便捷构造 (必填字段, description / mime_type 选填)
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    pub fn with_mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }
}

/// MCP ResourceContents (per spec §resources/read contents[0])
///
/// ```json
/// { "uri": "file:///x.rs", "mimeType": "text/x-rust", "text": "fn main() { ... }" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceContent {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 文本内容 (e.g. 源码)
    pub text: String,
}

impl ResourceContent {
    pub fn new(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: None,
            text: text.into(),
        }
    }
    pub fn with_mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }
}

/// MCP ResourceServer trait (server 端抽象)
///
/// 任意 server impl (e.g. `FileResourceServer` / `OrganResourceServer` / `AiderResourceServer`)
/// 都能挂到 mcp handler.
pub trait ResourceServer: Send + Sync {
    /// 列所有 resources
    fn list(&self) -> Vec<Resource>;
    /// 按 URI 读 resource
    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError>;
}

/// 处理 `resources/list` 请求 → JSON-RPC 响应
pub fn handle_resources_list(
    req: &JsonRpcRequest,
    server: &dyn ResourceServer,
) -> JsonRpcResponse {
    let resources = server.list();
    JsonRpcResponse::ok(
        req.id.clone(),
        json!({ "resources": resources }),
    )
}

/// 处理 `resources/read` 请求 → JSON-RPC 响应
pub fn handle_resources_read(
    req: &JsonRpcRequest,
    server: &dyn ResourceServer,
) -> JsonRpcResponse {
    // params 必填含 uri
    let uri = match req
        .params
        .as_ref()
        .and_then(|p| p.get("uri"))
        .and_then(|v| v.as_str())
    {
        Some(u) => u.to_string(),
        None => {
            return JsonRpcResponse::err(
                req.id.clone(),
                JsonRpcError::new(RESOURCE_INVALID_URI, "params.uri missing or not string"),
            );
        }
    };
    match server.read(&uri) {
        Ok(content) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({ "contents": [content] }),
        ),
        Err(e) => JsonRpcResponse::err(req.id.clone(), e),
    }
}

/// dispatch helper: 给定 method 路由到对应 handler
///
/// 已知 method 走对应 handler, 未知 method 返 Method not found (-32601 per JSON-RPC 2.0 spec)
pub fn dispatch(
    req: &JsonRpcRequest,
    server: &dyn ResourceServer,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "resources/list" => handle_resources_list(req, server),
        "resources/read" => handle_resources_read(req, server),
        _ => JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(-32601, format!("Method not found: {}", req.method)),
        ),
    }
}

// ============================================================
// 内置测试用 server: StaticResourceServer (固定 resources, 0 网络)
// ============================================================

/// 静态 ResourceServer (test / 演示用, 0 网络)
#[derive(Debug, Clone, Default)]
pub struct StaticResourceServer {
    resources: Vec<Resource>,
}

impl StaticResourceServer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_resource(mut self, r: Resource) -> Self {
        self.resources.push(r);
        self
    }
}

impl ResourceServer for StaticResourceServer {
    fn list(&self) -> Vec<Resource> {
        self.resources.clone()
    }
    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        // 找 matching resource, 返 mock 内容
        for r in &self.resources {
            if r.uri == uri {
                return Ok(ResourceContent::new(&r.uri, format!("content of {}", r.name))
                    .with_mime_type(r.mime_type.clone().unwrap_or_else(|| "text/plain".to_string())));
            }
        }
        Err(JsonRpcError::new(
            RESOURCE_NOT_FOUND,
            format!("resource not found: {uri}"),
        ))
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod resources_tests {
    use super::*;

    fn test_server() -> StaticResourceServer {
        StaticResourceServer::new()
            .with_resource(
                Resource::new("file:///x.rs", "x.rs")
                    .with_description("main entry")
                    .with_mime_type("text/x-rust"),
            )
            .with_resource(
                Resource::new("apeireth://organ/memory", "memory")
                    .with_description("9 organ: memory"),
            )
    }

    #[test]
    fn resource_new_and_with() {
        let r = Resource::new("uri", "name")
            .with_description("desc")
            .with_mime_type("text/plain");
        assert_eq!(r.uri, "uri");
        assert_eq!(r.name, "name");
        assert_eq!(r.description.as_deref(), Some("desc"));
        assert_eq!(r.mime_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn resource_content_with_mime() {
        let c = ResourceContent::new("uri", "text").with_mime_type("text/plain");
        assert_eq!(c.uri, "uri");
        assert_eq!(c.text, "text");
        assert_eq!(c.mime_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn static_server_list_returns_resources() {
        let s = test_server();
        let list = s.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].uri, "file:///x.rs");
        assert_eq!(list[1].uri, "apeireth://organ/memory");
    }

    #[test]
    fn static_server_read_existing_uri() {
        let s = test_server();
        let c = s.read("file:///x.rs").unwrap();
        assert_eq!(c.uri, "file:///x.rs");
        assert!(c.text.contains("x.rs"));
        assert_eq!(c.mime_type.as_deref(), Some("text/x-rust"));
    }

    #[test]
    fn static_server_read_missing_uri_errors() {
        let s = test_server();
        let e = s.read("not-found").unwrap_err();
        assert_eq!(e.code, RESOURCE_NOT_FOUND);
        assert!(e.message.contains("not-found"));
    }

    #[test]
    fn handle_resources_list_returns_json_rpc_ok() {
        let req = JsonRpcRequest::new("resources/list", None, Id::Num(1));
        let s = test_server();
        let resp = handle_resources_list(&req, &s);
        assert_eq!(resp.jsonrpc, JSON_RPC_VERSION);
        assert_eq!(resp.id, Some(Id::Num(1)));
        let result = resp.into_result().unwrap();
        let resources = result.get("resources").and_then(|v| v.as_array()).unwrap();
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn handle_resources_read_with_uri_returns_content() {
        let params = json!({ "uri": "file:///x.rs" });
        let req = JsonRpcRequest::new("resources/read", Some(params), Id::Str("r1".to_string()));
        let s = test_server();
        let resp = handle_resources_read(&req, &s);
        let result = resp.into_result().unwrap();
        let contents = result.get("contents").and_then(|v| v.as_array()).unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].get("uri").and_then(|v| v.as_str()), Some("file:///x.rs"));
    }

    #[test]
    fn handle_resources_read_missing_uri_returns_error() {
        let params = json!({ "uri": "not-found" });
        let req = JsonRpcRequest::new("resources/read", Some(params), Id::Num(2));
        let s = test_server();
        let resp = handle_resources_read(&req, &s);
        let err = resp.error.unwrap();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn handle_resources_read_no_uri_param_errors() {
        let req = JsonRpcRequest::new("resources/read", None, Id::Num(3));
        let s = test_server();
        let resp = handle_resources_read(&req, &s);
        let err = resp.error.unwrap();
        assert_eq!(err.code, RESOURCE_INVALID_URI);
    }

    #[test]
    fn dispatch_known_method_routes() {
        let req = JsonRpcRequest::new("resources/list", None, Id::Num(4));
        let s = test_server();
        let resp = dispatch(&req, &s);
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn dispatch_unknown_method_returns_method_not_found() {
        let req = JsonRpcRequest::new("resources/foo", None, Id::Num(5));
        let s = test_server();
        let resp = dispatch(&req, &s);
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);  // JSON-RPC 2.0 standard
        assert!(err.message.contains("resources/foo"));
    }

    #[test]
    fn resource_serde_round_trip() {
        let r = Resource::new("uri", "name").with_mime_type("text/plain");
        let json = serde_json::to_string(&r).unwrap();
        let back: Resource = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn resource_content_serde_round_trip() {
        let c = ResourceContent::new("uri", "text").with_mime_type("text/plain");
        let json = serde_json::to_string(&c).unwrap();
        let back: ResourceContent = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
