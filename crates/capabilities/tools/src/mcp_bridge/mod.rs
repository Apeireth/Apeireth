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
//! `McpToolBridge::new_with_policy`); the production assembly still builds the
//! bridge with `McpToolBridge::new`, so the bridge's policy is a library-level
//! gate, not a production one.
//!
//! [`mock`] is an in-process JSON-RPC server test double (no sockets, no subprocesses) for the e2e tests.
//!
//! **生产已接线**（`PRODUCTION WIRED`，默认关：`APEIRETH_ENABLE_MCP=1`）：
//! **发现 / 登记 / 授权 / 派发四环都有生产调用点** —— ① **发现**
//! `McpToolBridge::refresh()` ← 组合根 `crates/adapters/cli/src/external_tools.rs`
//! 的 `wire_at_startup` → 装配面的 `wire_external_tools`；
//! ② **登记** `Runtime::register_dynamic_tool()` ⇒ 外部工具进**运行时能力表**
//! ⇒ **真的进 `tool_declarations()`**（模型面工具名 `mcp:<server>:<tool>`）；
//! ③ **授权** = 生产权限策略的**族授权 + 族逐次人批**
//! （`authorize_external_tool_family`）⇒ 族内能力一律逐次人批；
//! ④ **派发** = 能力表按模型面名命中（`find_by_name`）。
//!
//! **装配面只注册两份东西**：**资源工厂形态**（`McpDefinition` —— 运行时初始化时
//! 交出活桥）与**登记账袋**（`McpModule` —— 自述面晚绑定「实际登记件数」）
//! ⇒ **不重复连进程、不重复构造桥对象**（活桥只有一只）。
//!
//! **仍属实的遗留**：桥内闸用的仍是**桥内自建策略**（生产装配走
//! `McpToolBridge::new`，**不是**共享生产策略 `new_with_policy`）⇒ 运行时裁决
//! 与桥内闸不是同一份策略（族级放行补齐等效效果，但两者不是同一把锁）。
//!
//! **仍未实测**：真第三方 MCP 服务器端到端未跑过 ⇒ `HARDWARE VALIDATED` 不成立
//! （`DEFAULT ENABLED` 也不成立：默认关）。

pub mod bridge;
pub mod catalog;
pub mod config;
pub mod connection;
pub mod definition;
pub mod family;
pub mod governance;
pub mod mock;
pub mod module;
mod process_owner;
pub use module::McpPlugin;
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
pub use definition::McpDefinition;
pub use family::{is_capability_in_family, MCP_FAMILY_ROOT};
pub use governance::{McpPermissionMapping, McpRiskMappingHook, McpToolDenyGuard};
pub use mock::{InProcessChannel, InProcessChannelFactory, InProcessMcpServer, MockToolSpec};
pub use tool::{mcp_output_schema, normalized_mcp_value, tool_result_from_mcp, McpDynamicTool};
pub use transport::{
    open_channel, HttpChannel, McpChannel, McpChannelFactory, SpecChannelFactory, SseChannel,
    StaticChannelFactory, StdioChannel,
};
