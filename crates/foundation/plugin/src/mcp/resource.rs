//! MCP resources/list + resources/read models.
//!
//! Engine: `legacy/canonical/apeireth-mcp/src/resources.rs`.
//!
//! Library primitives only. [`ResourceServer`] is a trait a host *may*
//! implement; nothing here walks a real filesystem or talks to organs.
//! Path-containment lives in [`crate::mcp::uri`] so a later File host can
//! reuse it without copying the old `FileResourceServer` (which depended
//! on `apeireth-tools` conventions and would become a second I/O owner).

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::mcp::jsonrpc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// Resource not found.
pub const RESOURCE_NOT_FOUND: i32 = -32001;
/// `params.uri` missing or illegal.
pub const RESOURCE_INVALID_URI: i32 = -32002;
/// Read failed (I/O, size cap, …) — defined so hosts can reuse the code.
pub const RESOURCE_READ_FAILED: i32 = -32003;

/// MCP Resource (spec §resources/list item).
///
/// Wire: `mimeType` camelCase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "mimeType")]
    pub mime_type: Option<String>,
}

impl Resource {
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

/// MCP ResourceContents (spec §resources/read contents[0]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceContent {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "mimeType")]
    pub mime_type: Option<String>,
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

/// Server-side resource provider. Injected; this crate does no I/O.
pub trait ResourceServer: Send + Sync {
    fn list(&self) -> Vec<Resource>;
    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError>;
}

/// Handle `resources/list`.
pub fn handle_resources_list(req: &JsonRpcRequest, server: &dyn ResourceServer) -> JsonRpcResponse {
    let resources = server.list();
    JsonRpcResponse::ok(req.id.clone(), json!({ "resources": resources }))
}

/// Handle `resources/read`.
pub fn handle_resources_read(req: &JsonRpcRequest, server: &dyn ResourceServer) -> JsonRpcResponse {
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
        Ok(content) => JsonRpcResponse::ok(req.id.clone(), json!({ "contents": [content] })),
        Err(e) => JsonRpcResponse::err(req.id.clone(), e),
    }
}

/// Route `resources/list` / `resources/read`; unknown method → −32601.
pub fn dispatch(req: &JsonRpcRequest, server: &dyn ResourceServer) -> JsonRpcResponse {
    match req.method.as_str() {
        "resources/list" => handle_resources_list(req, server),
        "resources/read" => handle_resources_read(req, server),
        _ => JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(
                JsonRpcError::CODE_METHOD_NOT_FOUND,
                format!("Method not found: {}", req.method),
            ),
        ),
    }
}

/// In-memory resource table (tests / demos). No filesystem.
#[derive(Debug, Clone, Default)]
pub struct StaticResourceServer {
    resources: Vec<(Resource, String)>,
}

impl StaticResourceServer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a resource whose read body is `"content of {name}"`.
    pub fn with_resource(mut self, r: Resource) -> Self {
        let body = format!("content of {}", r.name);
        self.resources.push((r, body));
        self
    }

    /// Register a resource with an explicit body.
    pub fn with_body(mut self, r: Resource, body: impl Into<String>) -> Self {
        self.resources.push((r, body.into()));
        self
    }
}

impl ResourceServer for StaticResourceServer {
    fn list(&self) -> Vec<Resource> {
        self.resources.iter().map(|(r, _)| r.clone()).collect()
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        for (r, body) in &self.resources {
            if r.uri == uri {
                let mut c = ResourceContent::new(&r.uri, body.clone());
                if let Some(mime) = &r.mime_type {
                    c = c.with_mime_type(mime.clone());
                } else {
                    c = c.with_mime_type("text/plain");
                }
                return Ok(c);
            }
        }
        Err(JsonRpcError::new(
            RESOURCE_NOT_FOUND,
            format!("resource not found: {uri}"),
        ))
    }
}

/// First-match composite over several [`ResourceServer`]s.
///
/// `list` concatenates; `read` tries each server in order and returns the
/// first non-`RESOURCE_NOT_FOUND` outcome (success or a different error).
#[derive(Default)]
pub struct CompositeResourceServer {
    inner: Vec<Box<dyn ResourceServer>>,
}

impl std::fmt::Debug for CompositeResourceServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeResourceServer")
            .field("servers", &self.inner.len())
            .finish()
    }
}

impl CompositeResourceServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, server: Box<dyn ResourceServer>) {
        self.inner.push(server);
    }

    pub fn with_server(mut self, server: Box<dyn ResourceServer>) -> Self {
        self.push(server);
        self
    }
}

impl ResourceServer for CompositeResourceServer {
    fn list(&self) -> Vec<Resource> {
        let mut out = Vec::new();
        for s in &self.inner {
            out.extend(s.list());
        }
        out
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        let mut last_not_found: Option<JsonRpcError> = None;
        for s in &self.inner {
            match s.read(uri) {
                Ok(c) => return Ok(c),
                Err(e) if e.code == RESOURCE_NOT_FOUND => last_not_found = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last_not_found.unwrap_or_else(|| {
            JsonRpcError::new(RESOURCE_NOT_FOUND, format!("resource not found: {uri}"))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::jsonrpc::{Id, JSON_RPC_VERSION};

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
    fn resource_serde_uses_mime_type_camel_case() {
        let r = Resource::new("uri", "name").with_mime_type("text/plain");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["mimeType"], "text/plain");
        assert!(v.get("mime_type").is_none());
        let back: Resource = serde_json::from_value(v).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn resource_content_serde_uses_mime_type_camel_case() {
        let c = ResourceContent::new("uri", "text").with_mime_type("text/plain");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["mimeType"], "text/plain");
        let back: ResourceContent = serde_json::from_value(v).unwrap();
        assert_eq!(back, c);
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
        assert_eq!(resources[0]["mimeType"], "text/x-rust");
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
        assert_eq!(
            contents[0].get("uri").and_then(|v| v.as_str()),
            Some("file:///x.rs")
        );
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
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("resources/foo"));
    }

    #[test]
    fn composite_first_match_and_fallback() {
        let a = StaticResourceServer::new().with_body(
            Resource::new("mem://a", "a").with_mime_type("text/plain"),
            "from-a",
        );
        let b = StaticResourceServer::new().with_body(
            Resource::new("mem://b", "b").with_mime_type("text/plain"),
            "from-b",
        );
        let c = CompositeResourceServer::new()
            .with_server(Box::new(a))
            .with_server(Box::new(b));
        assert_eq!(c.list().len(), 2);
        assert_eq!(c.read("mem://b").unwrap().text, "from-b");
        assert_eq!(c.read("nope").unwrap_err().code, RESOURCE_NOT_FOUND);
    }
}
