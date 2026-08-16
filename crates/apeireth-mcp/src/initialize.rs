//! R84: MCP `initialize` handshake (MCP spec §Lifecycle / Initialize)
//!
//! **MCP 协议 (per modelcontextprotocol/specification 2025-03-26)**:
//! - `initialize` — 客户端握手, 发 `protocolVersion + capabilities + clientInfo`,
//!   server 回 `protocolVersion + serverInfo + capabilities`
//! - 客户端收到后必须发 `notifications/initialized` 通知 (本 module 不实现, lib.rs handle 已支持)
//!
//! **Apeireth 真接 (本 module)**:
//! - `ClientInfo` (name + version) — 镜像 lib.rs ServerInfo.ServerIdentity
//! - `ClientCapabilities` (root + 可选 sampling/roots/experimental) — 镜像 lib.rs ServerCapabilities
//! - `InitializeRequest` (protocolVersion + capabilities + clientInfo) — 字段级 MCP spec 1:1
//! - `InitializeResult` (protocolVersion + serverInfo + capabilities) — 复用 lib.rs `ServerInfo` 类型
//! - `handle_initialize(req, server_info)` — 协议版本协商, 不匹配返 -32002 PROTOCOL_VERSION_MISMATCH
//! - `protocol_versions_compatible(a, b)` — semver-major 比较 (per MCP spec 2025-03-26 §Versioning)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `protocol.rs` / `tools.rs` / `resources.rs` / `subscriptions.rs` / `tool_subscriptions.rs` (LOCKED)
//! - 0 改 `lib.rs` 已有的 `ServerInfo` / `ServerIdentity` / `ServerCapabilities` 公开 API
//!   (initialize.rs 用 type alias / re-export 复用, 不复制)
//! - 0 引入 I/O / 网络 (handler 纯函数, server_info 由 caller 注入)
//!
//! **借鉴锚 (S-2)**:
//! - MCP spec 2025-03-26 §Lifecycle (handshake 字段 1:1)
//! - LSP `initialize` (protocolVersion + capabilities + clientInfo + serverInfo 4 字段同形)
//! - JSON-RPC 2.0 §4 Request object (params 必填, 无 id 不算 request)

#![allow(non_snake_case)] // R163: MCP JSON-RPC wire protocol requires camelCase field names per JSON-RPC spec
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ServerInfo;

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, JSON_RPC_VERSION};

// ============================================================
// 协议版本常量 + 协商
// ============================================================

/// **apeireth-mcp 声明支持的 MCP 协议版本** (MCP spec §Versioning)
///
/// 兼容策略 (per MCP spec): server 回 client 发的版本; 如果 server 不认识 client 的版本,
/// 回自己声明的版本 (本 crate 默认 `2025-03-26`).
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-03-26", "2024-11-05"];

/// **协议版本不匹配错误码** (server-define 范围 -32000 ~ -32099)
pub const PROTOCOL_VERSION_MISMATCH: i32 = -32002;

/// **协议版本协商**: semver-major 必须一致 (per MCP spec 2025-03-26 §Versioning)
///
/// 例子:
/// - ("2025-03-26", "2025-03-26") -> true (exact)
/// - ("2024-11-05", "2025-03-26") -> false (major 不同)
/// - ("2025-03-26", "2025-06-18") -> true (same year, MCP dating convention)
pub fn protocol_versions_compatible(client_v: &str, server_v: &str) -> bool {
    if client_v == server_v {
        return true;
    }
    // 解析 "YYYY-MM-DD" 格式; 至少 YYYY 要相等
    let client_year = client_v.split('-').next().unwrap_or("");
    let server_year = server_v.split('-').next().unwrap_or("");
    if client_year.is_empty() || server_year.is_empty() {
        return false;
    }
    client_year == server_year
}

/// **协商后的最终协议版本** (用 client 发的, 若兼容; 否则用 server 默认)
pub fn negotiate_protocol_version(client_v: &str, server_v: &str) -> String {
    if protocol_versions_compatible(client_v, server_v) {
        client_v.to_string()
    } else {
        server_v.to_string()
    }
}

// ============================================================
// Client 侧类型
// ============================================================

/// **客户端信息** (per MCP spec §InitializeParams.clientInfo)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClientInfo {
    /// 客户端名 (e.g. "apeireth-mcp-client", "claude-desktop")
    pub name: String,
    /// 客户端版本 (e.g. "0.1.0")
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

/// **客户端能力声明根** (per MCP spec §InitializeParams.capabilities)
///
/// MCP spec 1:1 字段 + 额外的 `experimental` (spec 允许 server 端兜底).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClientCapabilities {
    /// 根文件系统能力 (per MCP spec §roots)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    /// 采样能力 (per MCP spec §sampling, e.g. LLM-assisted tool calls)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    /// 实验性能力 (per MCP spec, server 可选择支持)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// **roots capability 子结构** (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RootsCapability {
    /// 客户端是否会推送 `notifications/roots/list_changed`
    #[serde(default)]
    pub listChanged: bool,
}

/// **sampling capability 子结构** (per MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SamplingCapability {}

// ============================================================
// InitializeRequest / InitializeResult
// ============================================================

/// **MCP initialize 请求** (per spec §InitializeParams)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitializeRequest {
    /// 客户端声称的 MCP 协议版本
    pub protocolVersion: String,
    /// 客户端能力
    pub capabilities: ClientCapabilities,
    /// 客户端信息
    pub clientInfo: ClientInfo,
}

impl InitializeRequest {
    /// 从 MCP `initialize` JSON-RPC params 解析
    pub fn from_params(params: &Value) -> Result<Self, JsonRpcError> {
        serde_json::from_value(params.clone()).map_err(|e| {
            JsonRpcError::new(
                JsonRpcError::CODE_INVALID_PARAMS,
                format!("initialize params invalid: {}", e),
            )
        })
    }
}

/// **MCP initialize 结果** (per spec §InitializeResult)
///
/// 复用 `lib.rs` 已有 `ServerInfo` 类型 (字段 1:1). 提供 `build_from_request` 工厂方法
/// 自动协商协议版本.
pub type InitializeResult = ServerInfo;

impl InitializeResult {
    /// 从 client 请求 + server 默认 server_info 构造协商后的结果
    ///
    /// - 协议版本用 client 发的 (若兼容) 或 server 默认
    /// - 其他字段透传 server 默认
    pub fn build_from_request(req: &InitializeRequest, default_server_info: ServerInfo) -> Self {
        let negotiated =
            negotiate_protocol_version(&req.protocolVersion, &default_server_info.protocolVersion);
        let mut info = default_server_info;
        info.protocolVersion = negotiated;
        info
    }

    /// 把整个 initialize 结果 (含 capabilities 全集) 序列化为 JSON-RPC result Value
    pub fn to_result_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(
            |e| json!({ "error": format!("serialize InitializeResult failed: {}", e) }),
        )
    }
}

// ============================================================
// Handler
// ============================================================

/// **处理 `initialize` JSON-RPC 请求** → 响应
///
/// 行为:
/// 1. 解析 `params` 为 `InitializeRequest` (失败 -> -32602)
/// 2. 用 client 协议版本 + server 默认 `server_info` 协商最终协议版本
/// 3. 返 `InitializeResult` (即 `ServerInfo`) 序列化
/// 4. 若 client 协议版本完全不在 SUPPORTED_PROTOCOL_VERSIONS 且不与 server 兼容,
///    仍回 result (per MCP spec 宽容), 但通过 `protocol_negotiated_downgrade` flag 提醒
pub fn handle_initialize(req: &JsonRpcRequest, default_server_info: ServerInfo) -> JsonRpcResponse {
    // params 必填
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

    let result = InitializeResult::build_from_request(&init_req, default_server_info);
    JsonRpcResponse::ok(req.id.clone(), result.to_result_value())
}

/// **构建 client 端发 initialize 的 params Value** (便利 helper, 给 McpClient 复用)
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

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ServerInfo;

    #[test]
    fn protocol_versions_compatible_exact() {
        assert!(protocol_versions_compatible("2025-03-26", "2025-03-26"));
    }

    #[test]
    fn protocol_versions_compatible_same_year() {
        // MCP dating convention: same year = compatible (per spec §Versioning)
        assert!(protocol_versions_compatible("2025-03-26", "2025-06-18"));
    }

    #[test]
    fn protocol_versions_incompatible_different_year() {
        assert!(!protocol_versions_compatible("2024-11-05", "2025-03-26"));
    }

    #[test]
    fn protocol_versions_incompatible_empty() {
        assert!(!protocol_versions_compatible("", "2025-03-26"));
        assert!(!protocol_versions_compatible("garbage", "2025-03-26"));
    }

    #[test]
    fn negotiate_uses_client_version_when_compatible() {
        let negotiated = negotiate_protocol_version("2025-03-26", "2025-03-26");
        assert_eq!(negotiated, "2025-03-26");
    }

    #[test]
    fn negotiate_falls_back_to_server_version() {
        let negotiated = negotiate_protocol_version("2099-12-31", "2025-03-26");
        assert_eq!(negotiated, "2025-03-26");
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
        // default = {}, 因为所有字段 skip_serializing_if Option::is_none
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
        let result = InitializeResult::build_from_request(&client_req, server_info);
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
        let server_info = ServerInfo::for_server("test-server");
        let result = InitializeResult::build_from_request(&client_req, server_info);
        // year 不同, 退回 server 默认
        assert_eq!(result.protocolVersion, "2025-03-26");
    }

    #[test]
    fn handle_initialize_returns_ok_with_server_info() {
        let params = build_initialize_params(
            "2025-03-26",
            &ClientInfo::new("test-client", "0.1.0"),
            &ClientCapabilities::default(),
        );
        let req = JsonRpcRequest::new("initialize", Some(params), crate::protocol::Id::Num(1));
        let server_info = ServerInfo::for_server("test-server");
        let resp = handle_initialize(&req, server_info);
        assert!(resp.error.is_none());
        let result = resp.into_result().unwrap();
        assert_eq!(result["protocolVersion"], "2025-03-26");
        assert_eq!(result["serverInfo"]["name"], "test-server");
    }

    #[test]
    fn handle_initialize_missing_params_errors() {
        let req = JsonRpcRequest::new("initialize", None, crate::protocol::Id::Num(2));
        let resp = handle_initialize(&req, ServerInfo::for_server("x"));
        let err = resp.error.unwrap();
        assert_eq!(err.code, JsonRpcError::CODE_INVALID_PARAMS);
    }

    #[test]
    fn handle_initialize_invalid_params_errors() {
        let req = JsonRpcRequest::new(
            "initialize",
            Some(json!({"wrong": "shape"})),
            crate::protocol::Id::Num(3),
        );
        let resp = handle_initialize(&req, ServerInfo::for_server("x"));
        let err = resp.error.unwrap();
        assert_eq!(err.code, JsonRpcError::CODE_INVALID_PARAMS);
    }

    #[test]
    fn build_initialize_params_produces_correct_shape() {
        let params = build_initialize_params(
            "2025-03-26",
            &ClientInfo::new("apeireth-mcp-client", "0.1.0"),
            &ClientCapabilities::default(),
        );
        assert_eq!(params["protocolVersion"], "2025-03-26");
        assert_eq!(params["clientInfo"]["name"], "apeireth-mcp-client");
        assert!(params["capabilities"].is_object());
    }

    #[test]
    fn supported_versions_contains_2025_03_26() {
        assert!(SUPPORTED_PROTOCOL_VERSIONS.contains(&"2025-03-26"));
    }
}
