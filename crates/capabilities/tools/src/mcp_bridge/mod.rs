//! The MCP tool bridge: external tools become dynamic tools of this runtime.
//!
//! This module is the last-mile wiring between the MCP protocol library in
//! `apeireth_plugin::mcp` (JSON-RPC envelopes, initialize lifecycle, wire
//! schema, reconnect policy — consumed, never copied) and the canonical tool
//! execution chain (`crate::exec_pipeline`):
//!
//! 1. **Configuration surface** ([`config`]): `APEIRETH_MCP_SERVERS` (JSON
//!    array) is the primary entry, the data directory's `mcp-servers.json`
//!    (opened with the stored-document fail-closed semantics) is the
//!    secondary one. A broken configuration refuses to open; it never
//!    silently degrades to defaults. Startup logs are redacted: URLs may be
//!    recorded, token/secret parameters never are.
//! 2. **Connections** ([`connection`]): one connection per configured server,
//!    with an initialize lifecycle state machine and disconnect-reconnect
//!    that re-discovers the tool list after every reconnect.
//! 3. **Dynamic tools** ([`tool`] + [`bridge`]): each tool a server exposes
//!    through `tools/list` is registered under the `mcp:<server>:<tool>`
//!    namespace and executes through the five-stage pipeline — pre-execute
//!    risk mapping (external tools default to the require-approval level),
//!    monotonic guards, an around deadline (MCP call timeouts close as
//!    `timeout.*`), the post-execute correction channel, and the output
//!    normalization contract that freezes MCP result structures into
//!    [`ToolOutcome`](crate::exec_pipeline::ToolOutcome).
//! 4. **Governance mapping** ([`governance`]): an external tool's permission
//!    declaration maps to explicit [`PermissionPolicy`] grants; with no
//!    explicit authorization the call goes to human approval, and
//!    `APEIRETH_MCP_READONLY_PRESET=1` releases server-declared read-only
//!    tools (still through the full pipeline).
//!
//! [`mock`] is an in-process JSON-RPC server test double (no sockets, no
//! subprocesses) used by the end-to-end tests.

pub mod bridge;
pub mod catalog;
pub mod config;
pub mod connection;
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
pub use governance::{McpPermissionMapping, McpRiskMappingHook, McpToolDenyGuard};
pub use mock::{InProcessChannel, InProcessChannelFactory, InProcessMcpServer, MockToolSpec};
pub use tool::{mcp_output_schema, normalized_mcp_value, tool_result_from_mcp, McpDynamicTool};
pub use transport::{
    open_channel, HttpChannel, McpChannel, McpChannelFactory, SpecChannelFactory, SseChannel,
    StaticChannelFactory, StdioChannel,
};
