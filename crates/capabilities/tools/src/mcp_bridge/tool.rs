//! Dynamic tools: one external tool, one model-facing declaration.
//!
//! Every tool a server exposes becomes an [`McpDynamicTool`] under the
//! `mcp:<server>:<tool>` namespace. Execution is the remote `tools/call`; the
//! result structure is normalized here into the output contract's structured
//! value ({text, content, is_error}) before the pipeline's normalization
//! stage freezes it into a `ToolOutcome`. The tool itself is pipeline-neutral:
//! the bridge wraps it in `PipelinedCapability`, so risk mapping, guards, the
//! deadline, and the correction channel all wrap this call.

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::mcp::{content_block_to_wire, ToolCallResult};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;

use crate::exec_pipeline::{OutputSchema, SchemaKind};
use crate::mcp::McpError;

use super::catalog::McpToolEntry;
use super::connection::McpServerConnection;

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
    generation: u64,
    generation_scope: apeireth_plugin::task_scope::ModuleTaskScope,
}

impl McpDynamicTool {
    /// A dynamic tool for `entry`, executing through `connection`.
    pub fn new(entry: McpToolEntry, connection: Arc<McpServerConnection>) -> Self {
        let generation = connection.generation();
        let generation_scope = connection.generation_scope();
        Self {
            entry,
            connection,
            generation,
            generation_scope,
        }
    }

    fn descriptor_receipt(&self) -> serde_json::Value {
        serde_json::json!({"server":self.entry.server,"tool":self.entry.remote_name,"schema":self.entry.input_schema,"read_only":self.entry.declared_read_only})
    }

    /// The catalog entry backing this tool.
    pub fn entry(&self) -> &McpToolEntry {
        &self.entry
    }

    /// The connection this tool executes through.
    pub fn connection(&self) -> &Arc<McpServerConnection> {
        &self.connection
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

    fn freeze_invocation(&self, _call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        Ok(Some(FrozenInvocation::new(
            self.descriptor_receipt(),
            "Registered external tool descriptor".into(),
        )))
    }
    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        if frozen.is_some_and(|f| f.payload != self.descriptor_receipt()) {
            return ToolResult::permanent_error(
                &call.id,
                "external tool descriptor changed since approval; approve a new call",
            )
            .with_name(&call.name);
        }
        self.invoke(call).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = self.entry.model_name.as_str();
        if !self.connection.accepts_generation(self.generation) {
            return ToolResult::permanent_error(
                &call.id,
                "external tool belongs to an expired module generation",
            )
            .with_name(name);
        }
        match self
            .generation_scope
            .run(self.connection.call_tool_checked(
                &self.entry.remote_name,
                call.arguments.clone(),
                &self.entry.input_schema,
                self.entry.declared_read_only,
            ))
            .await
            .unwrap_or_else(|_| Err(McpError::Transport("module generation stopped".into())))
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
