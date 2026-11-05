//! Dynamic tools: one external tool, one model-facing declaration.
//!
//! Every tool a server exposes becomes an [`McpDynamicTool`] under the
//! `mcp:<server>:<tool>` namespace. Execution is the remote `tools/call`; the
//! result structure is normalized here into the output contract's structured
//! value ({text, content, is_error}) before the pipeline's normalization
//! stage freezes it into a `ToolOutcome`. The tool itself is pipeline-neutral:
//! the bridge wraps it in `PipelinedCapability`, so risk mapping, guards, the
//! deadline, and the correction channel all wrap this call.
//!
//! # Frozen invocations (the approval card's payload)
//!
//! The tool freezes its execution inputs ([`McpDynamicTool::freeze_invocation`])
//! so the runtime's approval card shows **what is about to run**, and resumes
//! the approved call from that frozen payload
//! ([`McpDynamicTool::invoke_frozen`]) instead of from the ambient
//! environment. The payload carries the server, the remote tool name, and the
//! arguments; the display carries the same call's *shape* only — the
//! arguments' values can carry credentials, so the card shows their key
//! names, never their values.

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::mcp::{content_block_to_wire, ToolCallResult};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::exec_pipeline::{OutputSchema, SchemaKind};
use crate::mcp::McpError;

use super::catalog::McpToolEntry;
use super::connection::McpServerConnection;

/// The frozen payload version of the external-tool invocation.
///
/// Bumped whenever the payload shape changes; a resumed call carrying another
/// version is refused instead of executed with a half-understood payload.
const MCP_FROZEN_VERSION: u32 = 1;

/// The exact, versioned execution inputs frozen at approval time.
///
/// This is the bridge's own payload schema. The runtime treats it as an
/// opaque `serde_json::Value`; the bridge owns deserialization and must
/// execute these fields — and only these fields — when resuming an approved
/// operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpFrozenInvocation {
    version: u32,
    /// The server the approved call runs against.
    server: String,
    /// The server-side tool name.
    remote_name: String,
    /// The arguments the approved call carries.
    arguments: serde_json::Value,
}

/// The declared output shape every MCP result must normalize into.
pub fn mcp_output_schema() -> OutputSchema {
    OutputSchema::new()
        .require("text", SchemaKind::String)
        .require("content", SchemaKind::Array)
}

/// Normalize one MCP result structure into the contract's structured value:
/// joined text, the wire content blocks, and the server's error flag.
pub fn normalized_mcp_value(result: &ToolCallResult) -> serde_json::Value {
    let content: Vec<serde_json::Value> =
        result.content.iter().map(content_block_to_wire).collect();
    serde_json::json!({
        "text": result.extract_text(),
        "content": content,
        "is_error": result.is_error,
    })
}

/// Freeze one MCP result into a model-facing tool result.
///
/// A server-side error (`isError`) becomes a permanent tool error carrying
/// the server's own text; a success becomes the normalized structure the
/// output contract validates.
pub fn tool_result_from_mcp(call_id: &str, tool_name: &str, result: ToolCallResult) -> ToolResult {
    let text = result.extract_text();
    if result.is_error {
        let message = if text.is_empty() {
            "external tool reported an error without detail".to_string()
        } else {
            text
        };
        ToolResult::permanent_error(call_id, message).with_name(tool_name)
    } else {
        ToolResult::ok(call_id, normalized_mcp_value(&result)).with_name(tool_name)
    }
}

/// One external tool, callable like any builtin tool.
pub struct McpDynamicTool {
    entry: McpToolEntry,
    connection: Arc<McpServerConnection>,
}

impl McpDynamicTool {
    /// A dynamic tool for `entry`, executing through `connection`.
    pub fn new(entry: McpToolEntry, connection: Arc<McpServerConnection>) -> Self {
        Self { entry, connection }
    }

    /// The catalog entry backing this tool.
    pub fn entry(&self) -> &McpToolEntry {
        &self.entry
    }

    /// The connection this tool executes through.
    pub fn connection(&self) -> &Arc<McpServerConnection> {
        &self.connection
    }

    /// The execution inputs of one call, frozen.
    ///
    /// The freeze takes the **current** entry (server, remote tool name) and
    /// the call's arguments — this is the shape a not-yet-approved call
    /// executes in. The resumed call uses the frozen payload instead.
    fn build_frozen(&self, call: &ToolCall) -> McpFrozenInvocation {
        McpFrozenInvocation {
            version: MCP_FROZEN_VERSION,
            server: self.entry.server.clone(),
            remote_name: self.entry.remote_name.clone(),
            arguments: call.arguments.clone(),
        }
    }

    /// The human-facing approval card face of one frozen call.
    ///
    /// The card shows the **shape** of the call, never its values: a remote
    /// tool's arguments can carry credentials, and the remote
    /// `input_schema` can describe sensitive fields — so neither the values
    /// nor the schema belong here. The payload carries the real arguments.
    fn display_invocation(frozen: &McpFrozenInvocation) -> serde_json::Value {
        let (argument_keys, argument_shape) = match frozen.arguments.as_object() {
            Some(arguments) => (arguments.keys().cloned().collect::<Vec<String>>(), "object"),
            None => (Vec::new(), "non_object"),
        };
        serde_json::json!({
            "version": frozen.version,
            "server": frozen.server,
            "tool": frozen.remote_name,
            "argument_shape": argument_shape,
            "argument_keys": argument_keys,
            "argument_count": argument_keys.len(),
        })
    }

    /// Execute one frozen call: the frozen server, the frozen remote tool
    /// name, the frozen arguments.
    async fn execute_frozen(&self, call: &ToolCall, frozen: &McpFrozenInvocation) -> ToolResult {
        let name = self.entry.model_name.as_str();
        match self
            .connection
            .call_tool(&frozen.remote_name, frozen.arguments.clone())
            .await
        {
            Ok(result) => tool_result_from_mcp(&call.id, name, result),
            Err(McpError::Transport(reason)) => ToolResult::retryable_error(
                &call.id,
                format!("mcp transport failure for {name}: {reason}"),
            )
            .with_name(name),
            Err(McpError::JsonRpc { code, message }) => ToolResult::permanent_error(
                &call.id,
                format!("mcp error for {name} (code {code}): {message}"),
            )
            .with_name(name),
            Err(other) => ToolResult::permanent_error(
                &call.id,
                format!("mcp call failed for {name}: {other}"),
            )
            .with_name(name),
        }
    }
}

#[async_trait]
impl ToolCapability for McpDynamicTool {
    fn id(&self) -> &CapabilityId {
        &self.entry.capability_id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        if let Some(schema) = &self.entry.input_schema {
            if let Some(object) = schema.as_object() {
                parameters = object.clone();
            }
        }
        NormalizedTool {
            name: self.entry.model_name.clone(),
            description: self.entry.description.clone(),
            parameters,
            strict: false,
        }
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call);
        let payload = serde_json::to_value(&frozen).map_err(|error| {
            ToolResult::permanent_error(
                &call.id,
                format!("failed to serialize frozen external invocation: {error}"),
            )
            .with_name(&self.entry.model_name)
        })?;
        let display = Self::display_invocation(&frozen);
        Ok(Some(FrozenInvocation::new(payload, display)))
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let Some(frozen) = frozen else {
            return self.invoke(call).await;
        };

        let mcp_frozen: McpFrozenInvocation = match serde_json::from_value(frozen.payload.clone()) {
            Ok(mcp_frozen) => mcp_frozen,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("frozen external invocation is invalid: {error}"),
                )
                .with_name(&self.entry.model_name)
            }
        };

        if mcp_frozen.version != MCP_FROZEN_VERSION {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen external invocation version {} (expected {})",
                    mcp_frozen.version, MCP_FROZEN_VERSION
                ),
            )
            .with_name(&self.entry.model_name);
        }

        // The frozen server must be the one this tool executes through: a
        // payload naming another server is not this tool's call (fail-closed).
        if mcp_frozen.server != self.entry.server {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "frozen external invocation names server {:?}, but this tool executes through {:?}",
                    mcp_frozen.server, self.entry.server
                ),
            )
            .with_name(&self.entry.model_name);
        }

        self.execute_frozen(call, &mcp_frozen).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let frozen = self.build_frozen(call);
        self.execute_frozen(call, &frozen).await
    }
}
