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
//! - schema / wire-key normalization (`inputSchema`, `mimeType`, `isError`)
//! - resources/list+read and prompts/list+get models
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
pub mod prompt;
pub mod resource;
pub mod schema;

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
pub use prompt::{
    dispatch as dispatch_prompts, handle_prompts_get, handle_prompts_list, GetPromptResult, Prompt,
    PromptArgument, PromptContent, PromptMessage, PromptRole, PromptServer, StaticPromptServer,
    PROMPT_INVALID_ARGS, PROMPT_NOT_FOUND, PROMPT_RENDER_FAILED,
};
pub use resource::{
    dispatch as dispatch_resources, handle_resources_list, handle_resources_read,
    CompositeResourceServer, Resource, ResourceContent, ResourceServer, StaticResourceServer,
    RESOURCE_INVALID_URI, RESOURCE_NOT_FOUND, RESOURCE_READ_FAILED,
};
pub use schema::{
    content_block_from_wire, content_block_to_wire, is_valid_mcp_name, normalize_mcp_result,
    normalize_wire_object, ContentBlock, McpTool, ToolCallResult, TOOL_CALL_FAILED, TOOL_INTERNAL,
    TOOL_INVALID_ARGS, TOOL_NOT_FOUND,
};
