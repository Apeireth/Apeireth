//! Fixed V1 plugin hook port. Requests are data, never permission grants.

use std::sync::Arc;

use apeireth_core::kernel::{RequestId, SessionId, TaskId, TraceId};
use apeireth_protocol::canonical::{
    ModuleHookV1, NormalizedMessage, NormalizedResponse, ToolCall, ToolResult,
};

/// Immutable snapshot; no session writer, model request or dispatcher handle.
#[derive(Debug, Clone)]
pub struct HookContextV1 {
    pub hook: ModuleHookV1,
    pub session: SessionId,
    pub request: RequestId,
    pub trace: TraceId,
    pub messages: Arc<[NormalizedMessage]>,
    pub candidate: Option<NormalizedResponse>,
    pub tool_call: Option<ToolCall>,
    pub tool_result: Option<ToolResult>,
}

/// Stable execution id makes retries of a callback idempotent within a session.
#[derive(Debug, Clone, PartialEq)]
pub struct HookToolRequest {
    pub execution: TaskId,
    pub call: ToolCall,
}
