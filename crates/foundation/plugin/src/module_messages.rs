//! Typed single-process module messages. Delivery does not grant authority.
use crate::{agents::AgentPrincipal, ToolCancellation};
use apeireth_core::kernel::{CapabilityId, ModuleInstanceId, PluginId, SessionId, TaskId, TraceId};
use apeireth_protocol::canonical::ToolResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleMessageKind {
    Command,
    Query,
    Event,
    Reply,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageVisibility {
    ModulePrivate,
    PublicStatus,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageReplyMode {
    Result,
    Acknowledgement,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthoritySource {
    CallerDelegation,
    ModuleStanding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageModuleIdentity {
    pub definition: PluginId,
    pub instance: ModuleInstanceId,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleMessageEnvelope {
    pub protocol_version: u16,
    pub message_id: TaskId,
    pub correlation_id: Option<TaskId>,
    pub causation_id: Option<TaskId>,
    pub trace_id: TraceId,
    pub source: MessageModuleIdentity,
    pub target_module: PluginId,
    pub target_capability: Option<CapabilityId>,
    pub principal: Option<AgentPrincipal>,
    pub authorization_ref: String,
    pub authority_source: MessageAuthoritySource,
    pub kind: ModuleMessageKind,
    pub payload_schema_version: u16,
    pub payload: serde_json::Value,
    pub deadline_unix_ms: u64,
    pub reply_mode: MessageReplyMode,
    pub visibility: MessageVisibility,
}
/// Runtime-created causal route, never read from a tool/model payload.
#[derive(Debug, Clone)]
pub struct ModuleMessageRoute {
    pub visited: Vec<MessageModuleIdentity>,
    pub depth: usize,
    pub trace: TraceId,
    pub message: TaskId,
}
/// Sender identity/principal/authorization are deliberately absent from this input.
#[derive(Clone)]
pub struct ModuleMessageRequest {
    pub target_module: PluginId,
    pub capability: CapabilityId,
    pub kind: ModuleMessageKind,
    pub schema_version: u16,
    pub payload: serde_json::Value,
    pub timeout: Duration,
    pub cancellation: ToolCancellation,
    pub visibility: MessageVisibility,
    pub idempotency_key: Option<String>,
}
impl ModuleMessageRequest {
    pub fn command(
        target_module: PluginId,
        capability: CapabilityId,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            target_module,
            capability,
            kind: ModuleMessageKind::Command,
            schema_version: 1,
            payload,
            timeout: Duration::from_secs(5),
            cancellation: Default::default(),
            visibility: MessageVisibility::ModulePrivate,
            idempotency_key: None,
        }
    }
}
#[derive(Clone)]
pub struct InternalToolRequest {
    pub capability: CapabilityId,
    pub arguments: serde_json::Value,
    pub timeout: Duration,
    pub cancellation: ToolCancellation,
    pub idempotency_key: Option<String>,
}
impl InternalToolRequest {
    pub fn new(capability: CapabilityId, arguments: serde_json::Value) -> Self {
        Self {
            capability,
            arguments,
            timeout: Duration::from_secs(5),
            cancellation: Default::default(),
            idempotency_key: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMessageReply {
    pub envelope: ModuleMessageEnvelope,
    /// Events acknowledge delivery without projecting the receiver's business data.
    pub result: Option<ToolResult>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleMessageErrorCode {
    Disabled,
    InvalidEnvelope,
    Unauthorized,
    SourceStopped,
    TargetUnavailable,
    PrivateTarget,
    Backpressure,
    Timeout,
    Cancelled,
    Loop,
    Duplicate,
    IdempotencyConflict,
    ResultLimit,
    Execution,
}
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{code:?}: {message}")]
pub struct ModuleMessageError {
    pub code: ModuleMessageErrorCode,
    pub message: String,
    pub original_message: Option<TaskId>,
    pub may_have_executed: bool,
}
impl ModuleMessageError {
    pub fn new(code: ModuleMessageErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            original_message: None,
            may_have_executed: false,
        }
    }
}
pub type ModuleMessageResult = Result<ModuleMessageReply, ModuleMessageError>;
#[async_trait]
pub trait ModuleMessages: Send + Sync {
    async fn send(&self, request: ModuleMessageRequest) -> ModuleMessageResult;
    /// Only this bound owner's explicitly granted Internal capabilities are reachable.
    async fn invoke_internal(&self, request: InternalToolRequest) -> ModuleMessageResult;
}
