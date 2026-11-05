//! The MCP tool bridge: external tools become dynamic tools of this runtime.
//!
//! This module is the last-mile wiring between the MCP protocol library in
//! `apeireth_plugin::mcp` (JSON-RPC envelopes, initialize lifecycle, wire
//! schema, reconnect policy — consumed, never copied) and the canonical tool
//! execution chain (`crate::exec_pipeline`):
//!
//! 1. **Configuration surface** ([`config`]): `APEIRETH_MCP_SERVERS` (JSON
//!    array) is the primary entry, the data directory's `mcp-servers.json`
//!    (stored-document fail-closed semantics) is the secondary one. A broken
//!    configuration refuses to open; it never degrades to defaults. Startup
//!    logs are redacted: URLs may be recorded, token/secret parameters never.
//! 2. **Connections** ([`connection`]): one connection per configured server,
//!    with an initialize lifecycle state machine and disconnect-reconnect
//!    that re-discovers the tool list after every reconnect.
//! 3. **Dynamic tools** ([`tool`] + [`bridge`]): each tool a server exposes
//!    through `tools/list` is registered under the `mcp:<server>:<tool>`
//!    namespace and executes through the five-stage pipeline — a pre-execute
//!    **deny-only gate**, monotonic guards, an around deadline (MCP call
//!    timeouts close as `timeout.*`), the post-execute correction channel,
//!    and the output normalization contract that freezes MCP result
//!    structures into [`ToolOutcome`](crate::exec_pipeline::ToolOutcome).
//!    Each tool freezes its execution inputs ([`tool::McpDynamicTool`]), so
//!    the runtime's approval card shows what is about to run and the resumed
//!    call runs the approved payload, not the ambient environment.
//! 4. **Governance mapping** ([`governance`]): an external tool's permission
//!    declaration maps to explicit [`PermissionPolicy`] grants; with no
//!    explicit authorization the call is **refused** by the gate
//!    (`pipeline.pre_deny`), and `APEIRETH_MCP_READONLY_PRESET=1` releases
//!    read-only tools into the authorization surface (full pipeline).
//!
//! # Where the human approval happens
//!
//! The bridge's pre-execute gate is **deny-only**: it refuses what the
//! shared policy refuses and passes everything else through. It never asks —
//! an `ask` verdict closes as a permanent error (`pipeline.pre_ask`), not as
//! a suspended approval, and would break the runtime's approval-resume path
//! (the resumed call would ask a second time and die even though the human
//! had already approved). **The human approval right belongs to the runtime
//! governance chain (T1) alone**; the tool pipeline (T2) only refuses.
//! Layering: runtime governance chain first, tool pipeline second.
//!
//! The bridge's policy is meant to be the **shared production policy** (see
//! `McpToolBridge::new_with_policy`); until the wiring batch hands it over,
//! the bridge's policy is a library-level gate, not a production one.
//!
//! [`mock`] is an in-process JSON-RPC server test double (no sockets, no subprocesses) for the e2e tests.
//!
//! **生产未接线**：发现 / 登记 / 授权三环零生产调用点 ⇒ 生产只构造桥与模块袋，0 个外部工具进模型工具目录。

pub mod bridge;
pub mod catalog;
pub mod config;
pub mod connection;
pub mod family;
pub mod governance;
pub mod mock;
pub mod tool;
pub mod transport;

pub use bridge::{DiscoveryReport, McpBridgeError, McpToolBridge, McpToolRegistry};
pub use catalog::{McpCatalogError, McpToolCatalog, McpToolEntry};
pub use config::{
    doc_compat, readonly_preset_value, redact_args, redact_url, server_list_path, McpBridgeOptions,
    McpConfigError, McpServerConfig, McpServerSpec, McpTransportKind, DEFAULT_CALL_TIMEOUT_MS,
    MCP_READONLY_PRESET_ENV, MCP_SERVERS_ENV, MCP_SERVERS_FILE,
};
pub use connection::{parse_tool_list, DiscoveredTool, McpServerConnection};
pub use family::{is_capability_in_family, MCP_FAMILY_ROOT};
pub use governance::{McpPermissionMapping, McpRiskMappingHook, McpToolDenyGuard};
pub use mock::{InProcessChannel, InProcessChannelFactory, InProcessMcpServer, MockToolSpec};
pub use tool::{mcp_output_schema, normalized_mcp_value, tool_result_from_mcp, McpDynamicTool};
pub use transport::{
    open_channel, HttpChannel, McpChannel, McpChannelFactory, SpecChannelFactory, SseChannel,
    StaticChannelFactory, StdioChannel,
};
