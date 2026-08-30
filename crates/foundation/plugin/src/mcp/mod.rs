//! MCP protocol library primitives.
//!
//! Recovered from donor `apeireth-mcp` as **types and algorithms**, not as a
//! production MCP host. This module is **default-off**: nothing here is wired
//! into [`crate::PluginManager`], [`crate::CapabilityRegistry`], or the
//! runtime dispatch loop. Tool ownership stays with Module-exposed
//! [`crate::ToolCapability`] implementations.
//!
//! Recovered surface:
//! - JSON-RPC 2.0 envelopes (untagged id, notifications, batch)
//! - initialize lifecycle + capability metadata
//! - primitive method dispatch table (library handlers only)
//!
//! Explicitly **not** recovered here:
//! - reqwest SSE / HTTP-streamable / stdio transports
//! - `McpServer::from_registry` over the old `ToolRegistry`
//! - File/Organ/Convention I/O resource hosts
//! - a second tool registry or parallel MCP host
//!
//! The v2 client in `apeireth-tools::mcp` remains the production tools/list+call
//! bridge. Integrators that need ACP envelopes should reuse `protocol::acp`
//! (salvage-16), not a copy in this module.

pub mod discovery;
pub mod jsonrpc;
pub mod lifecycle;

pub use discovery::{
    dispatch_by_method, Primitive, PrimitiveDispatch, PRIMITIVE_COUNT, SUPPORTED_METHODS,
};
pub use jsonrpc::{
    looks_like_batch, Id, JsonRpcBatch, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    JSON_RPC_VERSION,
};
pub use lifecycle::{
    build_initialize_params, handle_initialize, negotiate_protocol_version,
    protocol_versions_compatible, ClientCapabilities, ClientInfo, ClientSession, InitializeRequest,
    LoggingCapability, PromptsCapability, ResourcesCapability, RootsCapability, SamplingCapability,
    ServerCapabilities, ServerIdentity, ServerInfo, SessionState, ToolsCapability,
    MCP_PROTOCOL_VERSION, PROTOCOL_VERSION_MISMATCH, SUPPORTED_PROTOCOL_VERSIONS,
};
