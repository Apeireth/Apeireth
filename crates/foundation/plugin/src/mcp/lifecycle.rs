//! MCP initialize handshake, capability metadata, and client session state.
//!
//! Engine: `legacy/canonical/apeireth-mcp/src/{initialize.rs,lib.rs}` (ServerInfo /
//! ClientInfo / protocol version negotiation).
//!
//! The production client in `apeireth-tools::mcp` sends a hardcoded
//! `2024-11-05` initialize and does not keep a session state machine. This
//! module recovers:
//! - protocol version negotiation (same-year MCP dating convention)
//! - richer capability flags (`tools`, `resources`, `prompts`, `logging`)
//! - a four-state client session (New → Initializing → Ready → Closed)
//!
//! Default-off: no transport is opened here.

#![allow(non_snake_case)] // MCP wire fields are camelCase.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::mcp::jsonrpc::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// Protocol versions this library knows how to speak.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-03-26", "2024-11-05"];

/// Default protocol version claimed by this library.
pub const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// Server-defined error when a client version is unusable.
pub const PROTOCOL_VERSION_MISMATCH: i32 = -32002;

/// Same-year MCP dating convention (spec §Versioning).
///
/// Exact match always succeeds. `"YYYY-MM-DD"` strings whose year agrees
/// are treated as compatible. Anything else is not.
pub fn protocol_versions_compatible(client_v: &str, server_v: &str) -> bool {
    if client_v == server_v {
        return true;
    }
    let client_year = client_v.split('-').next().unwrap_or("");
    let server_year = server_v.split('-').next().unwrap_or("");
    if client_year.is_empty() || server_year.is_empty() {
        return false;
    }
    client_year.chars().all(|c| c.is_ascii_digit())
        && server_year.chars().all(|c| c.is_ascii_digit())
        && client_year == server_year
}

/// Prefer the client's version when compatible, otherwise the server's.
pub fn negotiate_protocol_version(client_v: &str, server_v: &str) -> String {
    if protocol_versions_compatible(client_v, server_v) {
        client_v.to_string()
    } else {
        server_v.to_string()
    }
}

/// Client identity (spec §InitializeParams.clientInfo).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

impl ClientInfo {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// Client capability root (spec §InitializeParams.capabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClientCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RootsCapability {
    #[serde(default)]
    pub listChanged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SamplingCapability {}

/// Tools capability sub-object (spec §ServerCapabilities.tools).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ToolsCapability {
    #[serde(default)]
    pub listChanged: bool,
}

/// Resources capability sub-object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: bool,
    #[serde(default)]
    pub listChanged: bool,
}

/// Prompts capability sub-object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PromptsCapability {
    #[serde(default)]
    pub listChanged: bool,
}

/// Logging capability marker (presence = supported).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LoggingCapability {}

/// Server identity (spec §InitializeResult.serverInfo).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerIdentity {
    pub name: String,
    pub version: String,
}

/// Server capability declaration. Engine only advertised `tools`; this
/// recovery also models resources/prompts/logging flags so a host can
/// advertise them without inventing a second registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

impl ServerCapabilities {
    /// Tools-only advertisement (matches the canonical default).
    pub fn tools_only() -> Self {
        Self {
            tools: Some(ToolsCapability { listChanged: false }),
            ..Self::default()
        }
    }

    /// Advertise tools + resources + prompts. Still default-off: the
    /// caller decides whether to actually serve those methods. Does
    /// not claim `tools/list` / `tools/call` ownership — those stay
    /// with the v2 tools client.
    pub fn with_resources_and_prompts(mut self) -> Self {
        self.resources = Some(ResourcesCapability {
            subscribe: true,
            listChanged: false,
        });
        self.prompts = Some(PromptsCapability { listChanged: false });
        self
    }

    pub fn supports_tools(&self) -> bool {
        self.tools.is_some()
    }

    pub fn supports_resources(&self) -> bool {
        self.resources.is_some()
    }

    pub fn supports_prompts(&self) -> bool {
        self.prompts.is_some()
    }

    pub fn supports_resource_subscribe(&self) -> bool {
        self.resources
            .as_ref()
            .map(|r| r.subscribe)
            .unwrap_or(false)
    }
}

/// Server initialize result (spec §InitializeResult).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerInfo {
    pub protocolVersion: String,
    pub serverInfo: ServerIdentity,
    pub capabilities: ServerCapabilities,
}

impl ServerInfo {
    /// Default tools-only advertisement using this crate's version string.
    pub fn for_server(name: impl Into<String>) -> Self {
        Self {
            protocolVersion: MCP_PROTOCOL_VERSION.to_string(),
            serverInfo: ServerIdentity {
                name: name.into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: ServerCapabilities::tools_only(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Client initialize params (spec §InitializeParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitializeRequest {
    pub protocolVersion: String,
    pub capabilities: ClientCapabilities,
    pub clientInfo: ClientInfo,
}

impl InitializeRequest {
    pub fn from_params(params: &Value) -> Result<Self, JsonRpcError> {
        serde_json::from_value(params.clone()).map_err(|e| {
            JsonRpcError::new(
                JsonRpcError::CODE_INVALID_PARAMS,
                format!("initialize params invalid: {e}"),
            )
        })
    }

    /// Negotiate a [`ServerInfo`] from this request and a server default.
    pub fn negotiate(&self, default_server_info: ServerInfo) -> ServerInfo {
        let negotiated =
            negotiate_protocol_version(&self.protocolVersion, &default_server_info.protocolVersion);
        let mut info = default_server_info;
        info.protocolVersion = negotiated;
        info
    }
}

/// Build client-side initialize params.
pub fn build_initialize_params(
    protocol_version: &str,
    client_info: &ClientInfo,
    capabilities: &ClientCapabilities,
) -> Value {
    json!({
        "protocolVersion": protocol_version,
        "clientInfo": client_info,
        "capabilities": capabilities,
    })
}

/// Handle an `initialize` JSON-RPC request against a default [`ServerInfo`].
pub fn handle_initialize(req: &JsonRpcRequest, default_server_info: ServerInfo) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(
                JsonRpcError::CODE_INVALID_PARAMS,
                "initialize requires params {protocolVersion, capabilities, clientInfo}",
            ),
        );
    };
    let init_req = match InitializeRequest::from_params(params) {
        Ok(r) => r,
        Err(e) => return JsonRpcResponse::err(req.id.clone(), e),
    };
    let result = init_req.negotiate(default_server_info);
    let value = serde_json::to_value(&result)
        .unwrap_or_else(|e| json!({ "error": format!("serialize ServerInfo failed: {e}") }));
    JsonRpcResponse::ok(req.id.clone(), value)
}

/// Client-side session state. New → Initializing → Ready, or Closed.
///
/// Engine `McpClient` only had a boolean `server_info.is_some()`. A reconnect
/// policy needs the extra states so `tools/list` cannot race initialize and
/// so a closed transport cannot be reused silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Constructed, no handshake yet.
    New,
    /// `initialize` sent, waiting for result.
    Initializing,
    /// Handshake complete; methods other than initialize are legal.
    Ready,
    /// Transport closed or session torn down. Terminal.
    Closed,
}

/// In-memory client session tracker. No I/O.
#[derive(Debug, Clone)]
pub struct ClientSession {
    state: SessionState,
    next_id: i64,
    server_info: Option<ServerInfo>,
    client_info: ClientInfo,
}

impl ClientSession {
    pub fn new(client_info: ClientInfo) -> Self {
        Self {
            state: SessionState::New,
            next_id: 0,
            server_info: None,
            client_info,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    pub fn client_info(&self) -> &ClientInfo {
        &self.client_info
    }

    /// Allocate the next numeric request id.
    pub fn next_id(&mut self) -> Id {
        self.next_id += 1;
        Id::Num(self.next_id)
    }

    /// Build the `initialize` request and move New → Initializing.
    pub fn begin_initialize(
        &mut self,
        capabilities: &ClientCapabilities,
    ) -> Result<JsonRpcRequest, JsonRpcError> {
        if self.state != SessionState::New {
            return Err(JsonRpcError::new(
                JsonRpcError::CODE_INVALID_REQUEST,
                format!("initialize illegal from {:?}", self.state),
            ));
        }
        let id = self.next_id();
        let params = build_initialize_params(MCP_PROTOCOL_VERSION, &self.client_info, capabilities);
        self.state = SessionState::Initializing;
        Ok(JsonRpcRequest::new("initialize", Some(params), id))
    }

    /// Apply a successful initialize result. Initializing → Ready.
    pub fn complete_initialize(&mut self, info: ServerInfo) -> Result<(), JsonRpcError> {
        if self.state != SessionState::Initializing {
            return Err(JsonRpcError::new(
                JsonRpcError::CODE_INVALID_REQUEST,
                format!("complete_initialize illegal from {:?}", self.state),
            ));
        }
        self.server_info = Some(info);
        self.state = SessionState::Ready;
        Ok(())
    }

    /// Fail a handshake. Initializing → Closed.
    pub fn fail_initialize(&mut self) {
        if self.state == SessionState::Initializing {
            self.state = SessionState::Closed;
        }
    }

    /// Methods other than initialize require Ready.
    pub fn ensure_ready(&self) -> Result<(), JsonRpcError> {
        if self.state == SessionState::Ready {
            Ok(())
        } else {
            Err(JsonRpcError::new(
                JsonRpcError::CODE_INVALID_REQUEST,
                format!("session not ready (state {:?})", self.state),
            ))
        }
    }

    /// Tear down. Any non-Closed state → Closed.
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
    }

    /// Re-open after a reconnect. Closed → New, dropping cached server info.
    ///
    /// The caller is expected to run `begin_initialize` again. Id counter is
    /// kept so outstanding ids from the previous connection cannot collide
    /// with the next handshake if a late response arrives.
    pub fn reset_for_reconnect(&mut self) -> Result<(), JsonRpcError> {
        if self.state != SessionState::Closed {
            return Err(JsonRpcError::new(
                JsonRpcError::CODE_INVALID_REQUEST,
                format!("reset_for_reconnect illegal from {:?}", self.state),
            ));
        }
        self.server_info = None;
        self.state = SessionState::New;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_versions_compatible_exact() {
        assert!(protocol_versions_compatible("2025-03-26", "2025-03-26"));
    }

    #[test]
    fn protocol_versions_compatible_same_year() {
        assert!(protocol_versions_compatible("2025-03-26", "2025-06-18"));
    }

    #[test]
    fn protocol_versions_incompatible_different_year() {
        assert!(!protocol_versions_compatible("2024-11-05", "2025-03-26"));
    }

    #[test]
    fn protocol_versions_incompatible_empty_or_garbage() {
        assert!(!protocol_versions_compatible("", "2025-03-26"));
        assert!(!protocol_versions_compatible("garbage", "2025-03-26"));
    }

    #[test]
    fn negotiate_uses_client_version_when_compatible() {
        assert_eq!(
            negotiate_protocol_version("2025-03-26", "2025-03-26"),
            "2025-03-26"
        );
    }

    #[test]
    fn negotiate_falls_back_to_server_version() {
        assert_eq!(
            negotiate_protocol_version("2099-12-31", "2025-03-26"),
            "2025-03-26"
        );
    }

    #[test]
    fn client_info_serde_round_trip() {
        let info = ClientInfo::new("test-client", "0.1.0");
        let v = serde_json::to_value(&info).unwrap();
        assert_eq!(v["name"], "test-client");
        assert_eq!(v["version"], "0.1.0");
        let back: ClientInfo = serde_json::from_value(v).unwrap();
        assert_eq!(back, info);
    }

    #[test]
    fn client_capabilities_default_is_empty() {
        let caps = ClientCapabilities::default();
        let v = serde_json::to_value(&caps).unwrap();
        assert_eq!(v, json!({}));
    }

    #[test]
    fn client_capabilities_with_roots_list_changed() {
        let mut caps = ClientCapabilities::default();
        caps.roots = Some(RootsCapability { listChanged: true });
        let v = serde_json::to_value(&caps).unwrap();
        assert_eq!(v["roots"]["listChanged"], true);
    }

    #[test]
    fn initialize_request_from_valid_params() {
        let params = json!({
            "protocolVersion": "2025-03-26",
            "clientInfo": {"name": "test", "version": "1.0.0"},
            "capabilities": {}
        });
        let req = InitializeRequest::from_params(&params).unwrap();
        assert_eq!(req.protocolVersion, "2025-03-26");
        assert_eq!(req.clientInfo.name, "test");
    }

    #[test]
    fn initialize_request_from_invalid_params_errors() {
        let params = json!({"bad": "shape"});
        let err = InitializeRequest::from_params(&params).unwrap_err();
        assert_eq!(err.code, JsonRpcError::CODE_INVALID_PARAMS);
    }

    #[test]
    fn initialize_result_build_negotiates_version() {
        let client_req = InitializeRequest {
            protocolVersion: "2025-03-26".to_string(),
            capabilities: ClientCapabilities::default(),
            clientInfo: ClientInfo::new("c", "0.1.0"),
        };
        let server_info = ServerInfo::for_server("test-server");
        let result = client_req.negotiate(server_info);
        assert_eq!(result.protocolVersion, "2025-03-26");
        assert_eq!(result.serverInfo.name, "test-server");
    }

    #[test]
    fn initialize_result_build_falls_back_on_mismatch() {
        let client_req = InitializeRequest {
            protocolVersion: "1999-01-01".to_string(),
            capabilities: ClientCapabilities::default(),
            clientInfo: ClientInfo::new("c", "0.1.0"),
        };
        let result = client_req.negotiate(ServerInfo::for_server("test-server"));
        assert_eq!(result.protocolVersion, "2025-03-26");
    }

    #[test]
    fn handle_initialize_returns_ok_with_server_info() {
        let params = build_initialize_params(
            "2025-03-26",
            &ClientInfo::new("test-client", "0.1.0"),
            &ClientCapabilities::default(),
        );
        let req = JsonRpcRequest::new("initialize", Some(params), Id::Num(1));
        let resp = handle_initialize(&req, ServerInfo::for_server("test-server"));
        assert!(resp.error.is_none());
        let result = resp.into_result().unwrap();
        assert_eq!(result["protocolVersion"], "2025-03-26");
        assert_eq!(result["serverInfo"]["name"], "test-server");
    }

    #[test]
    fn handle_initialize_missing_params_errors() {
        let req = JsonRpcRequest::new("initialize", None, Id::Num(2));
        let resp = handle_initialize(&req, ServerInfo::for_server("x"));
        let err = resp.error.unwrap();
        assert_eq!(err.code, JsonRpcError::CODE_INVALID_PARAMS);
    }

    #[test]
    fn handle_initialize_invalid_params_errors() {
        let req = JsonRpcRequest::new("initialize", Some(json!({"wrong": "shape"})), Id::Num(3));
        let resp = handle_initialize(&req, ServerInfo::for_server("x"));
        let err = resp.error.unwrap();
        assert_eq!(err.code, JsonRpcError::CODE_INVALID_PARAMS);
    }

    #[test]
    fn server_capabilities_flags() {
        let caps = ServerCapabilities::tools_only().with_resources_and_prompts();
        assert!(caps.supports_tools());
        assert!(caps.supports_resources());
        assert!(caps.supports_prompts());
        assert!(caps.supports_resource_subscribe());
        let v = serde_json::to_value(&caps).unwrap();
        assert_eq!(v["resources"]["subscribe"], true);
        assert!(v.get("logging").is_none());
    }

    #[test]
    fn session_happy_path() {
        let mut s = ClientSession::new(ClientInfo::new("c", "1"));
        assert_eq!(s.state(), SessionState::New);
        let req = s.begin_initialize(&ClientCapabilities::default()).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(s.state(), SessionState::Initializing);
        s.complete_initialize(ServerInfo::for_server("srv"))
            .unwrap();
        assert_eq!(s.state(), SessionState::Ready);
        s.ensure_ready().unwrap();
        s.close();
        assert_eq!(s.state(), SessionState::Closed);
        s.reset_for_reconnect().unwrap();
        assert_eq!(s.state(), SessionState::New);
        assert!(s.server_info().is_none());
    }

    #[test]
    fn session_rejects_list_before_ready() {
        let s = ClientSession::new(ClientInfo::new("c", "1"));
        let err = s.ensure_ready().unwrap_err();
        assert_eq!(err.code, JsonRpcError::CODE_INVALID_REQUEST);
    }

    #[test]
    fn session_cannot_initialize_twice() {
        let mut s = ClientSession::new(ClientInfo::new("c", "1"));
        s.begin_initialize(&ClientCapabilities::default()).unwrap();
        assert!(s.begin_initialize(&ClientCapabilities::default()).is_err());
    }

    #[test]
    fn session_fail_initialize_closes() {
        let mut s = ClientSession::new(ClientInfo::new("c", "1"));
        s.begin_initialize(&ClientCapabilities::default()).unwrap();
        s.fail_initialize();
        assert_eq!(s.state(), SessionState::Closed);
    }

    #[test]
    fn session_reset_only_from_closed() {
        let mut s = ClientSession::new(ClientInfo::new("c", "1"));
        assert!(s.reset_for_reconnect().is_err());
    }

    #[test]
    fn session_id_monotonic_across_reconnect() {
        let mut s = ClientSession::new(ClientInfo::new("c", "1"));
        s.begin_initialize(&ClientCapabilities::default()).unwrap();
        s.fail_initialize();
        s.reset_for_reconnect().unwrap();
        let req = s.begin_initialize(&ClientCapabilities::default()).unwrap();
        assert_eq!(req.id, Some(Id::Num(2)));
    }

    #[test]
    fn supported_versions_contains_both_known() {
        assert!(SUPPORTED_PROTOCOL_VERSIONS.contains(&"2025-03-26"));
        assert!(SUPPORTED_PROTOCOL_VERSIONS.contains(&"2024-11-05"));
    }

    #[test]
    fn notifications_initialized_is_a_notification() {
        let n = JsonRpcRequest::notification("notifications/initialized", None);
        assert!(n.is_notification());
    }
}
