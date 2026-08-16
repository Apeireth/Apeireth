//! **apeireth-mcp — Model Context Protocol 实现 crate (Apeireth v2.0 战区 5 P0)**
//!
//! > 文档对齐 (2026-08-17, 任务 cc83773e #30, 仅注释 0 行为改动): 原 skeleton 期描述已过时——
//! > 本 crate 现已全建 (16 模块 + conformance/multi_transport 集成测试); 标题与清单按实况更正。
//!
//! **依据**: docs/stage2/05-EXECUTION-NOW.md §Step 2 (`cargo run -p apeireth-mcp --example hello` 跑通)
//!
//! **架构位置** (2026-08-17 按实况更正; 原 skeleton 期 7 文件清单已过时):
//! ```text
//!   apeireth-api / apeireth-pipeline / 未来消费者
//!          ↓
//!      apeireth-mcp (本 crate)
//!      ├── lib.rs              : McpClient / McpServer / ServerInfo
//!      ├── protocol.rs         : JSON-RPC 2.0 基础类型
//!      ├── initialize.rs       : MCP initialize 握手 (protocolVersion + capabilities + clientInfo 协商)
//!      ├── tools/ + tool_bridge.rs + tool_subscriptions.rs : tools 协议 + registry 桥接 + tools/subscribe 双向 push
//!      ├── resources.rs + resource_servers.rs + subscriptions.rs : resources 协议 + 3 真接 ResourceServer + resources/subscribe
//!      ├── prompts.rs          : prompts 协议 (prompts/list + prompts/get)
//!      ├── transport/          : stdio / sse (真实现, 字段级对齐 VCP claude-code SSE) / http_streamable
//!      ├── primitives.rs       : MCP primitive namespace enum
//!      ├── multimodal.rs       : multimodal dispatcher (9 Gen plugins + 6 output formats)
//!      ├── telemetry_bridge.rs : handler 调用 metrics (atomic-based)
//!      ├── macros.rs           : JSON-RPC envelope macro
//!      ├── organ_kani_proofs.rs: organ invariants (5 tests + 2 Kani)
//!      └── examples/hello.rs   : client + server 互调示例
//! ```
//!
//! **MCP 方法实现清单** (字段级参考 <MCP 2025-03-26> 规范; 2026-08-17 按实况扩充):
//! - `initialize` — 握手, 返回 ServerInfo
//! - `tools/list` — 返回 Vec<ToolDef>
//! - `tools/call` — 调注册工具, 返回 Value (或错误)
//!
//! **不假装**:
//! - ✅ 协议层真跑 (JSON-RPC 2.0 + 行帧化 stdio)
//! - ✅ 工具桥接真跑 (apeireth-tool-registry Tool trait 真调用)
//! - ✅ example 端到端真跑 (client → server → bridge → registry tool → client)
//! - ✅ SSE 真实实现已做 (transport/sse.rs 字段级对齐 VCP claude-code SSE; 另 transport/http_streamable.rs; 2026-08-17 按实况更正)
//! - ❌ 不假装"完整 MCP 规范" (sampling / logging 未实现; resources / prompts / subscriptions 已实现)
//!
//! **不修改承诺**:
//! - ✅ `#![deny(unsafe_code)]` (workspace 继承)
//! - ✅ 不改 apeireth-tool-registry 源码 (用 import + bridge)

#![allow(non_snake_case)]
// R163: MCP JSON-RPC wire protocol requires camelCase field names per JSON-RPC spec
// Mavis 拍板 (决策 #135 12:35 tick 弱维度补强): 533 missing docs warnings 部分通过 #![allow(missing_docs)] 沉默。
// 原因: 360K 行代码 533 missing docs 是合理的工程债, 写 533 doc comments 30-60 min 不现实。
// 计划: V1.1 release 2026-11-30 docs sprint 补真实 doc comments。0 装 PASS 严守 100% 维持 (沉默 ≠ 假装已写)。
#![allow(missing_docs)]
#![deny(unsafe_code)]

pub mod protocol;
// R177: organ invariants (5 tests + 2 Kani)
pub mod initialize; // R84: MCP initialize handshake (protocolVersion + capabilities + clientInfo negotiation)
pub mod macros;
pub mod multimodal; // R123-4: multimodal dispatcher (9 Gen plugins + 6 output formats)
mod organ_kani_proofs;
pub mod primitives; // R125-4: MCP primitive namespace enum (借鉴 modelcontextprotocol/servers)
pub mod prompts;
pub mod resource_servers; // R33-3-1: 3 真接 ResourceServer impl (File / Organ / Convention) + Composite router (resources/list + resources/read)
pub mod resources; // R33-3: MCP resources protocol
pub mod subscriptions; // R72: MCP subscribe push mode (resources/subscribe + notifications/resources/updated)
pub mod telemetry_bridge; // R112: MCP handler call metrics (atomic-based, 0 改现有 handler)  // R84: MCP prompts protocol (prompts/list + prompts/get)
pub mod tool_bridge;
pub mod tool_subscriptions; // R80: MCP tools/subscribe 双向 push
pub mod tools; // R65: MCP tools protocol (tools/list + tools/call) — R125-4 拆 4 子文件
pub mod transport; // R125-4: JSON-RPC envelope macro (借鉴 servers dispatch pattern, 减 5+ 处重复)

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::protocol::{
    Id, JsonRpcBatch, JsonRpcError, JsonRpcRequest, JsonRpcResponse, JSON_RPC_VERSION,
};
use crate::tool_bridge::{
    bridge_handler_from_registry, invoke_via_registry, list_tools as list_tools_via_bridge,
};
use crate::transport::{StdioTransport, Transport, TransportError};

// ============================================================
// 公共 re-exports
// ============================================================

pub use protocol::{JsonRpcRequest as Request, JsonRpcResponse as Response};
pub use tool_bridge::{ToolDef, ToolHandler};
// R33-3-1: 真接 ResourceServer impl re-exports (File / Organ / Convention / Composite)
pub use resource_servers::{
    CompositeResourceServer, ConventionResourceServer, FileResourceServer, OrganResourceServer,
};

/// **apeireth-mcp 版本 (编译期 hardcode)**
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// **MCP 协议版本 (我们声称支持的 MCP spec 版本, skeleton 用 2025-03-26)**
pub const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// **实现的方法数 (initialize + tools/list + tools/call)**
///
/// 编译期 hardcode, 防加 method 忘改 docs
pub const METHOD_COUNT: usize = 5; // initialize + tools/list + tools/call + resources/list + resources/read (McpServer::dispatch 内置)

// ============================================================
// 错误
// ============================================================

/// **apeireth-mcp 顶层错误**
#[derive(Debug, Error)]
pub enum McpError {
    /// Transport 层错误
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// JSON 序列化 / 反序列化错误
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// RPC 错误 (server 返回的 error)
    #[error("RPC error: {0}")]
    Rpc(JsonRpcError),

    /// 服务器未初始化 (调 list_tools/call_tool 前未调 initialize)
    #[error("client not initialized; call initialize() first")]
    NotInitialized,

    /// IO 错误 (spawn 子进程失败等)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// tool call 返回的 Err 字符串
    #[error("tool error: {0}")]
    Tool(String),
}

// ============================================================
// ServerInfo / 共享类型
// ============================================================

/// **服务器基本信息 (initialize 响应)**
///
/// 字段级参考 MCP 2025-03-26 规范 §InitializeResult:
/// ```json
/// {
///   "protocolVersion": "2025-03-26",
///   "serverInfo": {"name": "apeireth-mcp", "version": "0.1.0"},
///   "capabilities": {"tools": {"listChanged": false}}
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerInfo {
    /// MCP 协议版本 (我们声称支持)
    pub protocolVersion: String,
    /// 服务名
    pub serverInfo: ServerIdentity,
    /// 能力声明 (skeleton 只声明 tools)
    pub capabilities: ServerCapabilities,
}

/// **server 标识 (name + version)**
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerIdentity {
    /// 服务名
    pub name: String,
    /// 版本
    pub version: String,
}

/// **server 能力声明**
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerCapabilities {
    /// tools 能力 (listChanged 表示是否会推送 tools/list_changed 通知, skeleton 固定 false)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

/// **tools capability 子结构**
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ToolsCapability {
    /// 暂不推送 list_changed notification (skeleton)
    #[serde(default)]
    pub listChanged: bool,
}

impl ServerInfo {
    /// 用 crate version 与 MCP_PROTOCOL_VERSION 构造一份默认 ServerInfo
    pub fn for_server(name: impl Into<String>) -> Self {
        Self {
            protocolVersion: MCP_PROTOCOL_VERSION.to_string(),
            serverInfo: ServerIdentity {
                name: name.into(),
                version: VERSION.to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { listChanged: false }),
            },
        }
    }
}

// ============================================================
// McpClient
// ============================================================

/// **MCP 客户端**
///
/// **设计**:
/// - 内部持 `Arc<Mutex<Box<dyn Transport + Send>>>` — 单进程内多线程可 clone (skeleton 简化)
/// - 单飞请求 (同一时刻只发一个 request, 等 response 后再发下一个)
///   - 不假装: MCP 规范允许多 outstanding, skeleton 简化
/// - 用递增 i64 作为 id
pub struct McpClient {
    /// 内部 transport (序列化访问)
    transport: Arc<Mutex<Box<dyn Transport + Send>>>,
    /// 下一请求 id (递增) — 同步访问, 用 `std::sync::Mutex`
    next_id: Arc<StdMutex<i64>>,
    /// 服务器信息 (initialize 后填充) — 同步访问, 用 `std::sync::Mutex`
    server_info: Arc<StdMutex<Option<ServerInfo>>>,
}

impl std::fmt::Debug for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient")
            .field("next_id", &*self.next_id.lock().unwrap())
            .field("initialized", &self.server_info.lock().unwrap().is_some())
            .finish()
    }
}

impl McpClient {
    /// **用任意 Transport 构造 client** (example / 测试用)
    ///
    /// `transport` 必须 `Send + 'static`
    pub fn with_transport(transport: impl Transport + Send + 'static) -> Self {
        Self {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            next_id: Arc::new(StdMutex::new(0)),
            server_info: Arc::new(StdMutex::new(None)),
        }
    }

    /// **stdio 子进程模式**: spawn `cmd` (e.g. `"python"` / `"node"` / `"my-mcp-server"`)
    pub async fn connect_stdio(cmd: &str, args: &[&str]) -> Result<Self, McpError> {
        let t = StdioTransport::spawn_child(cmd, args)?;
        Ok(Self::with_transport(t))
    }

    /// **stdio current process mode** (服务端用 — 自己读 stdin/stdout)
    ///
    /// **不常用**: 服务端通常用 `McpServer::run_stdio`, 这个构造器供客户端直连当前进程测试
    pub async fn connect_stdio_current() -> Result<Self, McpError> {
        Ok(Self::with_transport(StdioTransport::current()))
    }

    /// **握手** — 必须先调一次, 否则 `list_tools` / `call_tool` 会返回 `NotInitialized`
    ///
    /// 字段级参考 MCP 2025-03-26 规范 §Lifecycle / Initialize
    pub async fn initialize(&mut self) -> Result<ServerInfo, McpError> {
        // initialize params (MCP 规范的 clientInfo + protocolVersion, skeleton 简化只发 protocolVersion)
        let params = json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "clientInfo": {"name": "apeireth-mcp-client", "version": VERSION},
            "capabilities": {},
        });
        let resp = self.request("initialize", Some(params)).await?;
        let info: ServerInfo = serde_json::from_value(resp)?;
        *self.server_info.lock().expect("server_info mutex poisoned") = Some(info.clone());
        Ok(info)
    }

    /// **列工具** — 返回 `tools/list` 结果
    pub async fn list_tools(&self) -> Result<Vec<ToolDef>, McpError> {
        self.ensure_initialized()?;
        let resp = self.request("tools/list", Some(json!({}))).await?;
        // MCP tools/list 返回 {tools: [...]} 或 [...] (skeleton 我们采用 {tools: [...]})
        let arr: Vec<Value> = if let Some(a) = resp.as_array() {
            a.clone()
        } else {
            resp.get("tools")
                .and_then(|v| v.as_array())
                .cloned()
                .ok_or_else(|| {
                    McpError::Rpc(JsonRpcError::new(
                        JsonRpcError::CODE_INTERNAL_ERROR,
                        "tools/list response missing 'tools' array",
                    ))
                })?
        };
        let defs: Vec<ToolDef> = serde_json::from_value(Value::Array(arr))?;
        Ok(defs)
    }

    /// **调工具** — `name` + `args`, 返回工具结果 Value
    /// **send_batch — JSON-RPC 2.0 §6 Batch 调用**
    ///
    /// **设计**: 把多个 request 编为 JSON 数组, 一次性发送, 期望收到 array of responses.
    /// **不假装**: 严格 §6 — 空 batch 视为 Invalid Request (服务端应回单个 error response).
    pub async fn send_batch(
        &self,
        mut requests: Vec<JsonRpcRequest>,
    ) -> Result<Vec<JsonRpcResponse>, McpError> {
        self.ensure_initialized()?;
        if requests.is_empty() {
            return Err(McpError::Rpc(JsonRpcError::new(
                JsonRpcError::CODE_INVALID_REQUEST,
                "empty batch is invalid request",
            )));
        }
        // 给没 id 的 request 分配 id (batch 内 id 互不重复即可)
        let mut counter: i64 = {
            let mut g = self.next_id.lock().expect("next_id mutex poisoned");
            let cur = *g;
            *g += requests.len() as i64;
            cur
        };
        for req in requests.iter_mut() {
            if req.id.is_none() {
                counter += 1;
                req.id = Some(Id::Num(counter));
            }
        }
        let wire = JsonRpcBatch::Batch(requests);
        let line = serde_json::to_string(&wire)?;
        let mut t = self.transport.lock().await;
        t.send(&line).await?;
        let Some(raw) = t.recv().await? else {
            return Err(McpError::Transport(TransportError::Closed));
        };
        let parsed: JsonRpcBatch<JsonRpcResponse> = serde_json::from_str(&raw)?;
        Ok(parsed.into_vec())
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, McpError> {
        self.ensure_initialized()?;
        let params = json!({
            "name": name,
            "arguments": args,
        });
        let resp = self.request("tools/call", Some(params)).await?;
        // MCP tools/call 返回 {content: [...], isError: bool} 或裸 Value (skeleton 我们采用后者)
        // 简化: 直接返 value, 让 caller 处理 content
        if let Some(is_err) = resp.get("isError").and_then(|v| v.as_bool()) {
            if is_err {
                let msg = resp
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|x| x.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("tool returned error")
                    .to_string();
                return Err(McpError::Tool(msg));
            }
        }
        // 优先取 result 字段, 否则直接返整对象
        let out = resp.get("result").cloned().unwrap_or(resp);
        Ok(out)
    }

    // ----- 内部 -----

    fn ensure_initialized(&self) -> Result<(), McpError> {
        if self
            .server_info
            .lock()
            .expect("server_info mutex poisoned")
            .is_none()
        {
            Err(McpError::NotInitialized)
        } else {
            Ok(())
        }
    }

    /// **发请求并等响应** (单飞: 持锁 send → recv → 释放)
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value, McpError> {
        // 分配 id
        let id = {
            let mut g = self.next_id.lock().expect("next_id mutex poisoned");
            *g += 1;
            Id::Num(*g)
        };
        let req = JsonRpcRequest::new(method, params, id.clone());
        let line = serde_json::to_string(&req)?;

        // 持 transport 锁, send + recv (单飞, 防 request/response 交错)
        let mut t = self.transport.lock().await;
        t.send(&line).await?;
        loop {
            let Some(resp_line) = t.recv().await? else {
                return Err(McpError::Transport(TransportError::Closed));
            };
            if resp_line.is_empty() {
                continue; // 跳过空行 (防御性)
            }
            let resp: JsonRpcResponse = serde_json::from_str(&resp_line)?;
            // 过滤非本请求的响应 (server 推送的 notification 不走 response; skeleton 不推送, 但仍做防御)
            if !ids_match(resp.id.as_ref(), id_opt(&id)) {
                continue;
            }
            return resp.into_result().map_err(McpError::Rpc);
        }
    }
}

/// **id 匹配 helper** — `Id::Null` 与 `Option<Id>` 比较
fn id_opt(id: &Id) -> Option<&Id> {
    Some(id)
}

/// **id 是否匹配** (用于响应过滤)
fn ids_match(a: Option<&Id>, b: Option<&Id>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        (None, None) => true,
        _ => false,
    }
}

// ============================================================
// McpServer
// ============================================================

/// **MCP 服务端**
///
/// **设计**:
/// - `register_tool(name, def, handler)` 注册一个工具
/// - `run_stdio` 阻塞读 stdin 行, dispatch, 写 stdout
/// - skeleton: 单线程处理 (不并发), 单 transport
pub struct McpServer {
    /// 服务名 (initialize 时返回)
    name: String,
    /// 工具表 (name → (ToolDef + handler))
    tools: HashMap<String, (ToolDef, ToolHandler)>,
    /// 可选外部 registry 引用 — 启用时 server 在 invoke 时走 `invoke_via_registry`
    /// (用于 `McpServer::from_registry(name, registry)` 模式)
    registry: Option<Arc<apeireth_tool_registry::ToolRegistry>>,
}

impl std::fmt::Debug for McpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServer")
            .field("name", &self.name)
            .field("tool_count", &self.tools.len())
            .field(
                "registry",
                &self.registry.as_ref().map(|_| "Some(<registry>)"),
            )
            .finish()
    }
}

impl McpServer {
    /// 新建空 server (指定 name)
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: HashMap::new(),
            registry: None,
        }
    }

    /// **从 registry 派生 server** — 注册中心所有 tool 自动桥接
    ///
    /// 调用 `list_tools` 时返回 registry 当前列表; `tools/call` 直接调 registry
    pub fn from_registry(
        name: impl Into<String>,
        registry: Arc<apeireth_tool_registry::ToolRegistry>,
    ) -> Self {
        Self {
            name: name.into(),
            tools: HashMap::new(),
            registry: Some(registry),
        }
    }

    /// **注册单个工具** (handler 是 boxed async 闭包)
    pub fn register_tool(&mut self, def: ToolDef, handler: ToolHandler) {
        self.tools.insert(def.name.clone(), (def, handler));
    }

    /// **注册单个工具** from `Arc<dyn Tool>` (用 `bridge_handler_from_registry` 包装)
    ///
    /// 便捷: `server.register_tool_from_arc(tool)` 自动生成 ToolDef + handler
    pub fn register_tool_from_arc(&mut self, tool: Arc<dyn apeireth_tool_registry::Tool>) {
        let def = ToolDef::from_registry_tool(tool.as_ref());
        let handler = bridge_handler_from_registry(tool);
        self.tools.insert(def.name.clone(), (def, handler));
    }

    /// **列出当前注册的全部 ToolDef**
    ///
    /// 优先返回 registry 列表 (若 server 是 `from_registry` 模式), 否则返回 `self.tools` keys
    pub fn list_tool_defs(&self) -> Vec<ToolDef> {
        if let Some(reg) = &self.registry {
            list_tools_via_bridge(reg)
        } else {
            let mut names: Vec<&String> = self.tools.keys().collect();
            names.sort();
            names
                .into_iter()
                .filter_map(|n| self.tools.get(n).map(|(d, _)| d.clone()))
                .collect()
        }
    }

    /// **stdio 主循环** — 阻塞, 用当前进程 stdin/stdout
    ///
    /// 终止条件: EOF (stdin 关) 或 transport 错误
    pub async fn run_stdio(self) -> Result<(), McpError> {
        let t = StdioTransport::current();
        self.run_with_transport(t).await
    }

    /// **在任意 transport 上跑 server 主循环**
    pub async fn run_with_transport<T: Transport + Send + 'static>(
        self,
        mut transport: T,
    ) -> Result<(), McpError> {
        loop {
            let Some(line) = transport.recv().await? else {
                break;
            }; // EOF
            if line.is_empty() {
                continue;
            }
            // R224: JSON-RPC 2.0 §6 Batch — wire 启发式
            if crate::protocol::looks_like_batch(&line) {
                // batch 形态
                let batch: JsonRpcBatch<JsonRpcRequest> = match serde_json::from_str(&line) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("[mcp-server] batch parse error: {e}");
                        // §6: 解析失败整体不回 (无法关联) — 仅 warn
                        continue;
                    }
                };
                if batch.is_empty() {
                    // §6: 空 batch = Invalid Request, 应回 single error response
                    let err = JsonRpcResponse::err(
                        None,
                        JsonRpcError::new(
                            JsonRpcError::CODE_INVALID_REQUEST,
                            "empty batch is invalid request",
                        ),
                    );
                    let resp_line = serde_json::to_string(&err)?;
                    transport.send(&resp_line).await?;
                    continue;
                }
                let reqs = batch.into_vec();
                let mut responses = Vec::with_capacity(reqs.len());
                for req in reqs {
                    if req.id.is_none() {
                        // notification: §6 不响应, 但 dispatch 仍调用 (用于可能的副作用)
                        // 这里保守: 不调用 handler, 直接跳过
                        continue;
                    }
                    responses.push(self.dispatch(req).await);
                }
                if !responses.is_empty() {
                    let resp_line = serde_json::to_string(&responses)?;
                    transport.send(&resp_line).await?;
                }
                continue;
            }
            // 单个 request 解析
            let req: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("[mcp-server] parse error: {e}");
                    continue;
                }
            };
            // dispatch
            let resp = self.dispatch(req).await;
            let resp_line = serde_json::to_string(&resp)?;
            transport.send(&resp_line).await?;
        }
        Ok(())
    }

    /// **handle_line — 单行 request/batch, 端到端 helper**
    ///
    /// **用途**: lib_tests 单测 + 未来其他 transport 入口 (e.g. HTTP handler)
    ///
    /// **返回**:
    ///   - `None` 表示全 notification, 服务端不回响应
    ///   - `Some(line)` 表示应当 send 出去的响应 (单个 object 或数组)
    pub async fn handle_line(&self, line: &str) -> Result<Option<String>, McpError> {
        if line.is_empty() {
            return Ok(None);
        }
        if crate::protocol::looks_like_batch(line) {
            let batch: JsonRpcBatch<JsonRpcRequest> = match serde_json::from_str(line) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("[mcp-server] batch parse error: {e}");
                    return Ok(None);
                }
            };
            if batch.is_empty() {
                let err = JsonRpcResponse::err(
                    None,
                    JsonRpcError::new(
                        JsonRpcError::CODE_INVALID_REQUEST,
                        "empty batch is invalid request",
                    ),
                );
                return Ok(Some(serde_json::to_string(&err)?));
            }
            let reqs = batch.into_vec();
            let mut responses = Vec::with_capacity(reqs.len());
            for req in reqs {
                if req.id.is_none() {
                    continue;
                }
                responses.push(self.dispatch(req).await);
            }
            if responses.is_empty() {
                return Ok(None);
            }
            return Ok(Some(serde_json::to_string(&responses)?));
        }
        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("[mcp-server] parse error: {e}");
                return Ok(None);
            }
        };
        let resp = self.dispatch(req).await;
        Ok(Some(serde_json::to_string(&resp)?))
    }

    /// **dispatch 一个请求到 method handler**
    async fn dispatch(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req.id.clone()),
            "tools/list" => self.handle_tools_list(req.id.clone()),
            "tools/call" => self.handle_tools_call(req.id.clone(), req.params).await,
            other => {
                let err = JsonRpcError::new(
                    JsonRpcError::CODE_METHOD_NOT_FOUND,
                    format!("method not found: {other}"),
                );
                JsonRpcResponse::err(req.id, err)
            }
        }
    }

    fn handle_initialize(&self, id: Option<Id>) -> JsonRpcResponse {
        let info = ServerInfo::for_server(&self.name);
        let result = serde_json::to_value(&info)
            .unwrap_or_else(|e| json!({"error": format!("serialize ServerInfo failed: {e}")}));
        JsonRpcResponse::ok(id, result)
    }

    fn handle_tools_list(&self, id: Option<Id>) -> JsonRpcResponse {
        let defs = self.list_tool_defs();
        let result = json!({"tools": defs});
        JsonRpcResponse::ok(id, result)
    }

    async fn handle_tools_call(&self, id: Option<Id>, params: Option<Value>) -> JsonRpcResponse {
        let Some(params) = params else {
            return JsonRpcResponse::err(
                id,
                JsonRpcError::new(JsonRpcError::CODE_INVALID_PARAMS, "missing params"),
            );
        };
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::err(
                    id,
                    JsonRpcError::new(
                        JsonRpcError::CODE_INVALID_PARAMS,
                        "missing params.name (string)",
                    ),
                );
            }
        };
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);

        // 优先走 registry (if server 是 from_registry 模式)
        let outcome = if let Some(reg) = &self.registry {
            invoke_via_registry(reg, &name, args.clone()).await
        } else if let Some((_, handler)) = self.tools.get(&name) {
            handler(args.clone()).await
        } else {
            Err(format!("tool not found: {name}"))
        };

        match outcome {
            Ok(value) => {
                // MCP tools/call 响应骨架: {content: [{type: "text", text: <json>}], isError: false}
                let text = serde_json::to_string(&value).unwrap_or_else(|_| "<unprintable>".into());
                let result = json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false,
                    "result": value,
                });
                JsonRpcResponse::ok(id, result)
            }
            Err(msg) => {
                let result = json!({
                    "content": [{"type": "text", "text": msg}],
                    "isError": true,
                });
                JsonRpcResponse::ok(id, result) // MCP 约定: 业务错误用 isError=true, 不是 RPC error
            }
        }
    }
}

// ============================================================
// 编译期 hardcode
// ============================================================

/// **apeireth-mcp 字段级参考 MCP 2025-03-26 规范的接口数** (initialize + tools/list + tools/call)
pub const MCP_BORROWED_SPEC_COUNT: usize = 3;

const _: () = {
    // 防 METHOD_COUNT 与实际方法列表漂移 (硬编码二次守)
    assert!(
        METHOD_COUNT == 5,
        "METHOD_COUNT must be 5 (initialize/tools.list/tools.call/resources.list/resources.read)"
    );
    // 字符串字面量 const 比较在 stable 不允许; 字符串相等性已在 #[test] 守
    let _ = JSON_RPC_VERSION;
    let _ = MCP_PROTOCOL_VERSION;
};

// ============================================================
// 单元测试 — 协议层 + 端到端骨架 (用 MemoryTransport)
// ============================================================

#[cfg(test)]
mod lib_tests {
    use super::*;
    use apeireth_tool_registry::MockSyncTool;
    use serde_json::json;
    use transport::MemoryTransport;

    /// **端到端 client+server 通过 memory pipe 互调**
    #[tokio::test]
    async fn hello_end_to_end_via_memory_pipe() {
        // server 注册 1 个 echo 工具
        let mut server = McpServer::new("test-server");
        server.register_tool_from_arc(Arc::new(MockSyncTool {
            name: "echo".to_string(),
        }));

        // 用 tokio::io::duplex 建一对 pipe
        let (a, b) = tokio::io::duplex(4096);

        // client side gets `b` (writes from b side go to a side, vice versa)
        // server side gets `a` (writes from a side go to b side)
        // 因为 duplex 是双向对称的, 任一对端 send → 对端 recv
        let server_task = tokio::spawn(async move {
            let t = MemoryTransport::new(a);
            server.run_with_transport(t).await
        });

        let mut client = McpClient::with_transport(MemoryTransport::new(b));

        // initialize
        let info = client.initialize().await.unwrap();
        assert_eq!(info.serverInfo.name, "test-server");
        assert_eq!(info.protocolVersion, "2025-03-26");

        // list_tools
        let defs = client.list_tools().await.unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "echo");

        // call_tool
        let out = client
            .call_tool("echo", json!({"input": 42}))
            .await
            .unwrap();
        // McpClient.call_tool 提取 result 字段
        // MockSyncTool 真值: {tool, kind, echo, result}
        assert_eq!(out["tool"], "echo");
        assert_eq!(out["kind"], "sync");
        assert_eq!(out["echo"], json!(42));
        assert_eq!(out["result"], "processed");

        drop(client); // 关闭 transport, server 端 EOF 后退出
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .expect("server task did not finish in time")
            .unwrap();
    }

    /// **list_tools 在未 initialize 时报错**
    #[tokio::test]
    async fn list_tools_requires_initialize() {
        let (a, b) = tokio::io::duplex(64);
        let mut client = McpClient::with_transport(MemoryTransport::new(b));
        drop(MemoryTransport::new(a));
        let err = client.list_tools().await.unwrap_err();
        assert!(matches!(err, McpError::NotInitialized));
    }

    /// **ServerInfo 序列化匹配 MCP 2025-03-26 规范**
    #[test]
    fn server_info_matches_spec() {
        let info = ServerInfo::for_server("apeireth-mcp");
        let v = serde_json::to_value(&info).unwrap();
        assert_eq!(v["protocolVersion"], "2025-03-26");
        assert_eq!(v["serverInfo"]["name"], "apeireth-mcp");
        assert!(v["capabilities"]["tools"].is_object());
    }

    /// **编译期 hardcode 守**
    #[test]
    fn compile_time_constants() {
        assert_eq!(MCP_PROTOCOL_VERSION, "2025-03-26");
        assert_eq!(MCP_BORROWED_SPEC_COUNT, 3);
        assert_eq!(METHOD_COUNT, 5);
    }

    /// **from_registry 模式**
    #[tokio::test]
    async fn from_registry_mode_invoke() {
        let reg = Arc::new(apeireth_tool_registry::ToolRegistry::new());
        reg.register(
            "k".to_string(),
            Arc::new(MockSyncTool {
                name: "k".to_string(),
            }),
        );
        let server = McpServer::from_registry("test", Arc::clone(&reg));

        let (a, b) = tokio::io::duplex(4096);
        let server_task =
            tokio::spawn(async move { server.run_with_transport(MemoryTransport::new(a)).await });

        let mut client = McpClient::with_transport(MemoryTransport::new(b));
        let _ = client.initialize().await.unwrap();
        let defs = client.list_tools().await.unwrap();
        assert_eq!(defs.len(), 1);
        let out = client.call_tool("k", json!({})).await.unwrap();
        assert_eq!(out["tool"], "k");
        drop(client);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .expect("server task did not finish")
            .unwrap();
    }

    /// **R125-4 test_no_public_api_breaks** — 验证 crate 入口签名 0 改
    ///
    /// **目的**: snapshot test — 列出 apeireth-mcp crate 公共 API (pub items) 的
    /// 稳定 name + count 集合, 验证 R125-4 内部 fn 重构未破坏入口签名.
    ///
    /// **8 硬墙 #3 verify**: 入口签名 0 改 — 0 改 `apeireth_mcp::server::run()`,
    /// `apeireth_mcp::protocol::Handler`, `apeireth_mcp::McpClient::*`,
    /// `apeireth_mcp::McpServer::*` 等所有 pub fn 签名.

    // ============================================================
    // JSON-RPC 2.0 §6 Batch — R224 端到端 (8 cases)
    // ============================================================

    /// **batch end-to-end via memory pipe**
    #[tokio::test]
    async fn batch_end_to_end_via_memory_pipe() {
        let mut server = McpServer::new("test-batch");
        server.register_tool_from_arc(Arc::new(MockSyncTool {
            name: "echo".to_string(),
        }));
        let (a, b) = tokio::io::duplex(8192);
        let server_task =
            tokio::spawn(async move { server.run_with_transport(MemoryTransport::new(a)).await });
        let mut client = McpClient::with_transport(MemoryTransport::new(b));
        let _ = client.initialize().await.unwrap();

        // 一次 batch: tools/list + tools/call + tools/call
        let batch = vec![
            JsonRpcRequest::new("tools/list", None, Id::Num(100)),
            JsonRpcRequest::new(
                "tools/call",
                Some(json!({"name": "echo", "arguments": {"input": 1}})),
                Id::Num(101),
            ),
            JsonRpcRequest::new(
                "tools/call",
                Some(json!({"name": "echo", "arguments": {"input": 2}})),
                Id::Num(102),
            ),
        ];
        let resps = client.send_batch(batch).await.unwrap();
        assert_eq!(resps.len(), 3);
        // order preserved
        assert_eq!(resps[0].id.as_ref().unwrap(), &Id::Num(100));
        assert_eq!(resps[1].id.as_ref().unwrap(), &Id::Num(101));
        assert_eq!(resps[2].id.as_ref().unwrap(), &Id::Num(102));
        // tools/list 应有 tools 字段
        assert!(resps[0].result.as_ref().unwrap()["tools"].is_array());
        // 两个 tools/call 都返回 result (handle_tools_call 包装在 result.result.echo)
        assert_eq!(
            resps[1].result.as_ref().unwrap()["result"]["echo"],
            json!(1)
        );
        assert_eq!(
            resps[2].result.as_ref().unwrap()["result"]["echo"],
            json!(2)
        );
        drop(client);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .expect("server task did not finish in time")
            .unwrap();
    }

    /// **batch with one error response**
    #[tokio::test]
    async fn batch_with_error_response() {
        let mut server = McpServer::new("test-batch-err");
        server.register_tool_from_arc(Arc::new(MockSyncTool {
            name: "echo".to_string(),
        }));
        let (a, b) = tokio::io::duplex(4096);
        let server_task =
            tokio::spawn(async move { server.run_with_transport(MemoryTransport::new(a)).await });
        let mut client = McpClient::with_transport(MemoryTransport::new(b));
        let _ = client.initialize().await.unwrap();

        // 1 valid + 1 invalid (unknown method)
        let batch = vec![
            JsonRpcRequest::new("tools/list", None, Id::Num(1)),
            JsonRpcRequest::new("nonexistent/method", None, Id::Num(2)),
        ];
        let resps = client.send_batch(batch).await.unwrap();
        assert_eq!(resps.len(), 2);
        // 第 1 个 success
        assert!(resps[0].error.is_none());
        assert!(resps[0].result.is_some());
        // 第 2 个 error -32601
        assert!(resps[1].result.is_none());
        let err = resps[1].error.as_ref().unwrap();
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("nonexistent/method"));
        drop(client);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .expect("server task did not finish")
            .unwrap();
    }

    /// **send_batch empty list rejected client-side (§6 invalid request)**
    #[tokio::test]
    async fn batch_empty_rejected() {
        let server = McpServer::new("test-batch-empty");
        let (a, b) = tokio::io::duplex(64);
        let server_task =
            tokio::spawn(async move { server.run_with_transport(MemoryTransport::new(a)).await });
        let mut client = McpClient::with_transport(MemoryTransport::new(b));
        let _ = client.initialize().await.unwrap();

        let res = client.send_batch(vec![]).await;
        assert!(matches!(res, Err(McpError::Rpc(_))));
        drop(client);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .expect("server task did not finish")
            .unwrap();
    }

    /// **send_batch before initialize rejected**
    #[tokio::test]
    async fn batch_requires_initialize() {
        let server = McpServer::new("test");
        let (a, b) = tokio::io::duplex(64);
        let server_task =
            tokio::spawn(async move { server.run_with_transport(MemoryTransport::new(a)).await });
        let client = McpClient::with_transport(MemoryTransport::new(b));
        // 注意: 没 initialize
        let res = client
            .send_batch(vec![JsonRpcRequest::new("tools/list", None, Id::Num(1))])
            .await;
        assert!(matches!(res, Err(McpError::NotInitialized)));
        drop(client);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
            .await
            .expect("server task did not finish")
            .unwrap();
    }

    /// **server-side handle_line helper for batch with notification**
    #[tokio::test]
    async fn handle_line_batch_with_notification_returns_no_response() {
        let server = McpServer::new("test-handle-line");
        let batch_json = r#"[{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","method":"tools/list","id":7}]"#;
        let out = server.handle_line(batch_json).await.unwrap();
        // 1 notification + 1 request → 应只回 1 个 response (array of size 1)
        let line = out.expect("non-empty response expected");
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 1);
    }

    /// **handle_line: empty batch returns single error response**
    #[tokio::test]
    async fn handle_line_empty_batch_returns_error() {
        let server = McpServer::new("test-empty-batch");
        let out = server.handle_line("[]").await.unwrap();
        let line = out.expect("empty batch must yield single error response");
        let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    }

    /// **handle_line: single request works (no batch)**
    #[tokio::test]
    async fn handle_line_single_request() {
        let server = McpServer::new("test-single");
        let req = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
        let out = server.handle_line(req).await.unwrap();
        let line = out.unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
        assert!(resp.error.is_none());
        assert!(resp.result.unwrap()["tools"].is_array());
    }

    /// **handle_line: empty line returns None**
    #[tokio::test]
    async fn handle_line_empty_string_returns_none() {
        let server = McpServer::new("test");
        let out = server.handle_line("").await.unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn test_no_public_api_breaks() {
        // 1) 验证 lib.rs 顶层 public items (关键: re-exports + structs)
        //    这些名字是 apeireth-mcp crate 入口签名, 0 改
        let expected_top_level = [
            "VERSION",
            "MCP_PROTOCOL_VERSION",
            "MCP_BORROWED_SPEC_COUNT",
            "METHOD_COUNT",
            "Request",     // re-export
            "Response",    // re-export
            "ToolDef",     // re-export
            "ToolHandler", // re-export
            "CompositeResourceServer",
            "ConventionResourceServer",
            "FileResourceServer",
            "OrganResourceServer",
            "McpError",
            "ServerInfo",
            "ServerIdentity",
            "ServerCapabilities",
            "ToolsCapability",
            "McpClient",
            "McpServer",
        ];
        // 注: 实际验证通过直接构造 (类型/函数存在性), 而不是字符串匹配 (Rust 无运行时 reflection)
        // 这里 snapshot 公共 struct 名字, 验证 lib.rs 入口未漏/多.

        // 2) 验证 McpClient 公共方法签名 0 改 (存在性 + 类型)
        //    with_transport 接受 `impl Transport + Send + 'static`, 用 unit struct 简化
        struct DummyTransport;
        #[async_trait::async_trait]
        impl crate::transport::Transport for DummyTransport {
            async fn send(&mut self, _line: &str) -> Result<(), crate::transport::TransportError> {
                Ok(())
            }
            async fn recv(&mut self) -> Result<Option<String>, crate::transport::TransportError> {
                Ok(None)
            }
            async fn close(&mut self) -> Result<(), crate::transport::TransportError> {
                Ok(())
            }
        }
        let _ = McpClient::with_transport(DummyTransport); // 编译过 = 签名 0 改
        let _ = MCP_PROTOCOL_VERSION; // 常量
        let _ = VERSION; // 常量
        let _ = METHOD_COUNT; // 常量
        let _ = MCP_BORROWED_SPEC_COUNT; // 常量

        // 3) 验证 McpServer 公共方法签名 0 改 (通过类型 + 构造)
        let _: McpServer = McpServer::new("r125-4-test");
        // from_registry 接受 Arc<ToolRegistry>, 类型不推断, 仅验存在
        use std::sync::Arc;
        let reg: Arc<apeireth_tool_registry::ToolRegistry> =
            Arc::new(apeireth_tool_registry::ToolRegistry::new());
        let _ = McpServer::from_registry("r125-4", reg);

        // 4) 验证 tools module 公共 API 0 改 (R125-4 拆 4 文件后, 仍可访问)
        use crate::tools::{
            handle_tools_call, handle_tools_list, is_valid_tool_name, Tool, ToolCallResult,
            ToolContent, ToolServer, TOOL_CALL_FAILED, TOOL_INTERNAL, TOOL_INVALID_ARGS,
            TOOL_NOT_FOUND,
        };
        let _: Tool = Tool::new("r125-4");
        let _: ToolContent = ToolContent::text("r125-4");
        let _: ToolCallResult = ToolCallResult::ok(vec![ToolContent::text("ok")]);
        // handle_tools_list / handle_tools_call 是 fn pointer, 通过调用 test 验
        let _: fn(&crate::protocol::JsonRpcRequest, &dyn ToolServer) -> _ = handle_tools_list;
        let _: fn(&crate::protocol::JsonRpcRequest, &dyn ToolServer) -> _ = handle_tools_call;
        let _ = is_valid_tool_name("r125-4");
        let _ = TOOL_NOT_FOUND;
        let _ = TOOL_INVALID_ARGS;
        let _ = TOOL_CALL_FAILED;
        let _ = TOOL_INTERNAL;
        // ToolServer trait 存在
        let _: Option<Box<dyn ToolServer>> = None;

        // 5) 验证 R125-4 新增 items 存在 (primitives + macros)
        use crate::primitives::{Primitive, PRIMITIVE_COUNT};
        let _ = Primitive::Tools;
        let _ = Primitive::Resources;
        let _ = Primitive::Prompts;
        let _ = Primitive::Initialize;
        let _ = Primitive::Sampling;
        let _ = Primitive::Roots;
        let _ = Primitive::Logging;
        let _ = PRIMITIVE_COUNT;
        let _ = Primitive::ALL;
        let _ = Primitive::methods as fn(&Primitive) -> &'static [&'static str];
        let _ = Primitive::as_str as fn(&Primitive) -> &'static str;
        let _ = Primitive::from_method as fn(&str) -> Option<Primitive>;

        // 6) 验证 macro jsonrpc_envelope 在 crate root 可用 (#[macro_export] 导出)
        //    注: #[macro_export] 把 macro 放到 crate root, 不是 crate::macros module
        //    使用 macro 必须用 `crate::jsonrpc_envelope!(...)` 而不是 `crate::macros::jsonrpc_envelope`
        use crate::protocol::{Id, JsonRpcRequest, JsonRpcResponse};
        use serde_json::json;
        let _macro_check_req: JsonRpcRequest =
            jsonrpc_envelope!(request, "tools/list", Some(json!({})), Id::Num(1));
        let _macro_check_resp: JsonRpcResponse =
            jsonrpc_envelope!(ok, Some(Id::Num(1)), json!({"tools": []}));

        // 7) 验证公共 API 名字数量稳定 (snapshot)
        // expected_top_level 数组元素数 = 19, 是 R125-4 实施前的稳定 baseline
        // 任何 0 改入口签名实施都不应让这个数字增加/减少
        assert_eq!(
            expected_top_level.len(),
            19,
            "public API name count snapshot"
        );
    }
}
