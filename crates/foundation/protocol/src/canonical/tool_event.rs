//! Versioned, owned tool facts and bounded projection into closed conversations.
//!
//! This layer translates data only. It does not dispatch tools, authorize work,
//! schedule hooks, or mutate an in-flight request. A host supplies an execution
//! identity, distinct from the provider's opaque call id, and applies a batch at
//! a closed conversation boundary. Existing transcripts are never reconstructed
//! from these records.

use std::collections::BTreeMap;
use std::io::{self, Write};

use apeireth_core::kernel::{PluginId, TaskId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use super::{MessageRole, NormalizedMessage, ToolCall, ToolResult};

/// Fixed V1 hook vocabulary. Defining a point does not schedule or emit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleHookV1 {
    SessionStart,
    BeforeModelCall,
    AfterModelCall,
    BeforeToolCall,
    AfterToolCall,
    OnIdle,
    BeforeCompaction,
    SessionEnd,
    TurnStart,
    BeforeFinalCommit,
    OnError,
}

/// Who produced the fact. Module ownership uses the existing extension id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolEventSourceV1 {
    /// A model requested the call; ownership may be resolved by the dispatcher.
    Model { module_id: Option<PluginId> },
    /// A private direct module request. New readers retain old variants unchanged.
    ModuleMessage {
        module_id: PluginId,
        module_instance_id: apeireth_core::kernel::ModuleInstanceId,
        target_module_id: PluginId,
        target_module_instance_id: apeireth_core::kernel::ModuleInstanceId,
        message_id: TaskId,
    },
    /// A module produced a result at a fixed hook boundary.
    Module {
        module_id: PluginId,
        hook: ModuleHookV1,
    },
}

/// A correlated result. Private fields keep constructed records validated.
///
/// `call_id` is a host execution identity. `call.id` is the independent provider
/// identity and is preserved verbatim; upstream ids may recur in later closed
/// rounds. A module-generated wire id must not impersonate an outstanding call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ToolEventRepr")]
pub struct ToolEventV1 {
    version: u16,
    call_id: TaskId,
    source: ToolEventSourceV1,
    call: ToolCall,
    result: ToolResult,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolEventRepr {
    version: u16,
    call_id: TaskId,
    source: ToolEventSourceV1,
    call: ToolCall,
    result: ToolResult,
}

impl TryFrom<ToolEventRepr> for ToolEventV1 {
    type Error = ToolEventError;

    fn try_from(value: ToolEventRepr) -> Result<Self, Self::Error> {
        if value.version != 1 {
            return Err(ToolEventError::UnsupportedVersion(value.version));
        }
        Self::new(value.call_id, value.source, value.call, value.result)
    }
}

impl ToolEventV1 {
    /// Validate both identities, correlation, and optional name echo.
    ///
    /// Parameter-schema and permission validation belong to the dispatcher;
    /// this constructor does not grant authority or attest that an effect ran.
    pub fn new(
        call_id: TaskId,
        source: ToolEventSourceV1,
        call: ToolCall,
        mut result: ToolResult,
    ) -> Result<Self, ToolEventError> {
        valid_label(&call.id, "provider call id")?;
        valid_label(&call.name, "tool name")?;
        if !call.arguments.is_object() {
            return Err(ToolEventError::InvalidArguments);
        }
        if call.id != result.tool_call_id {
            return Err(ToolEventError::Correlation);
        }
        if result.name.as_deref().is_some_and(|name| name != call.name) {
            return Err(ToolEventError::ToolName);
        }
        let owner = match &source {
            ToolEventSourceV1::Model { module_id } => module_id.as_ref(),
            ToolEventSourceV1::Module { module_id, .. }
            | ToolEventSourceV1::ModuleMessage { module_id, .. } => Some(module_id),
        };
        if let Some(owner) = owner {
            valid_label(owner.as_str(), "module id")?;
        }
        if let ToolEventSourceV1::ModuleMessage {
            message_id,
            target_module_id,
            ..
        } = &source
        {
            if *message_id != call_id {
                return Err(ToolEventError::Correlation);
            }
            valid_label(target_module_id.as_str(), "target module id")?;
        }
        result.name = Some(call.name.clone());
        Ok(Self {
            version: 1,
            call_id,
            source,
            call,
            result,
        })
    }

    /// Stable internal execution identity, used for idempotency.
    pub fn call_id(&self) -> TaskId {
        self.call_id
    }

    /// Provenance, independent of the provider message role.
    pub fn source(&self) -> &ToolEventSourceV1 {
        &self.source
    }

    /// Original provider-facing call and its opaque wire identity.
    pub fn call(&self) -> &ToolCall {
        &self.call
    }

    /// Canonical success/error value, before provider projection.
    pub fn result(&self) -> &ToolResult {
        &self.result
    }

    /// A standalone, matched call/result pair for a closed history boundary.
    ///
    /// This must not be appended in the middle of an existing assistant batch;
    /// use [`ToolEventBatch::append_to_closed_context`] to check that boundary.
    /// Module provenance stays inside the tool result. Model-result content
    /// keeps the existing canonical renderer.
    pub fn provider_messages(&self) -> [NormalizedMessage; 2] {
        let content = match &self.source {
            ToolEventSourceV1::Model { .. } => self.result.render(),
            ToolEventSourceV1::ModuleMessage { .. } => json!({"version":1,"origin":"module_message","source":self.source,"call_id":self.call_id,"payload":self.result.outcome}).to_string(),
            ToolEventSourceV1::Module { module_id, hook } => json!({
                "version": 1,
                "origin": "module",
                "module_id": module_id.as_str(),
                "call_id": self.call_id,
                "tool_name": self.call.name,
                "hook": hook,
                "payload": self.result.outcome,
            })
            .to_string(),
        };
        [
            NormalizedMessage::assistant_with_tool_calls("", vec![self.call.clone()]),
            NormalizedMessage::tool_result(
                self.call.id.clone(),
                Some(self.call.name.clone()),
                content,
            ),
        ]
    }
}

fn valid_label(value: &str, field: &'static str) -> Result<(), ToolEventError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(ToolEventError::InvalidLabel(field));
    }
    Ok(())
}

/// Record budgets, measured on canonical UTF-8 event JSON, not input ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolEventLimits {
    /// Maximum retained execution records, including projected records.
    pub max_events: usize,
    /// Sum of serialized event bytes, distinct from provider/context budgets.
    pub max_bytes: usize,
}

impl Default for ToolEventLimits {
    fn default() -> Self {
        Self {
            max_events: 64,
            max_bytes: 256 * 1024,
        }
    }
}

/// Whether an insertion added a fact or recognized its identical retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEventInsertion {
    Inserted,
    Duplicate,
}

/// An instance/session-owned batch; insertion order is the projection order.
///
/// The host owns ordering across producers. There is no global state, lock,
/// scheduler, or unbounded asynchronous injection queue in this data type.
#[derive(Debug, Clone)]
pub struct ToolEventBatch {
    limits: ToolEventLimits,
    events: Vec<ToolEventV1>,
    identities: BTreeMap<TaskId, usize>,
    bytes: usize,
    projected: usize,
}

impl ToolEventBatch {
    pub fn new(limits: ToolEventLimits) -> Self {
        Self {
            limits,
            events: Vec::new(),
            identities: BTreeMap::new(),
            bytes: 0,
            projected: 0,
        }
    }

    /// Validated facts in deterministic insertion order.
    pub fn events(&self) -> &[ToolEventV1] {
        &self.events
    }

    /// Canonical event bytes charged so far. Retries are not charged twice.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Insert atomically, rejecting a conflicting reuse of an execution id.
    pub fn push(&mut self, event: ToolEventV1) -> Result<ToolEventInsertion, ToolEventError> {
        if let Some(&index) = self.identities.get(&event.call_id) {
            return if self.events[index] == event {
                Ok(ToolEventInsertion::Duplicate)
            } else {
                Err(ToolEventError::ConflictingExecution(event.call_id))
            };
        }
        if self.events.len() >= self.limits.max_events {
            return Err(ToolEventError::Budget("events"));
        }
        let mut counter = LimitedCounter {
            limit: self.limits.max_bytes - self.bytes,
            bytes: 0,
            exhausted: false,
        };
        if serde_json::to_writer(&mut counter, &event).is_err() {
            return Err(if counter.exhausted {
                ToolEventError::Budget("bytes")
            } else {
                ToolEventError::Serialization
            });
        }
        let size = counter.bytes;
        let bytes = self
            .bytes
            .checked_add(size)
            .ok_or(ToolEventError::Budget("bytes"))?;
        if bytes > self.limits.max_bytes {
            return Err(ToolEventError::Budget("bytes"));
        }
        self.identities.insert(event.call_id, self.events.len());
        self.events.push(event);
        self.bytes = bytes;
        Ok(ToolEventInsertion::Inserted)
    }

    /// Add a producer's whole batch or leave this batch byte-for-byte intact.
    pub fn extend_atomic(
        &mut self,
        events: impl IntoIterator<Item = ToolEventV1>,
    ) -> Result<usize, ToolEventError> {
        let mut staged = self.clone();
        let before = staged.events.len();
        for event in events {
            staged.push(event)?;
        }
        let added = staged.events.len() - before;
        *self = staged;
        Ok(added)
    }

    /// Append complete pairs only after every existing call has a result.
    ///
    /// Failure leaves the context and cursor unchanged. A successful append
    /// advances this batch's cursor; retrying it appends nothing twice. Durable
    /// checkpointing across process restarts remains the host's responsibility.
    /// Execution ids are never inferred from old wire ids.
    pub fn append_to_closed_context(
        &mut self,
        messages: &mut Vec<NormalizedMessage>,
    ) -> Result<(), ToolEventError> {
        validate_closed_tool_context(messages)?;
        messages.extend(
            self.events[self.projected..]
                .iter()
                .flat_map(ToolEventV1::provider_messages),
        );
        self.projected = self.events.len();
        Ok(())
    }
}

/// Validate a model-request boundary, allowing wire-id reuse in closed rounds.
/// Every result must answer one outstanding call, and every call must be closed
/// before another conversation message or request boundary is introduced.
pub fn validate_closed_tool_context(messages: &[NormalizedMessage]) -> Result<(), ToolEventError> {
    let mut pending = BTreeMap::new();
    for message in messages {
        if message.role == MessageRole::Tool {
            if !message.tool_calls.is_empty() {
                return Err(ToolEventError::MalformedContext);
            }
            let id = message
                .tool_call_id
                .as_deref()
                .ok_or(ToolEventError::Correlation)?;
            let name = pending.remove(id).ok_or(ToolEventError::Correlation)?;
            if message.name.as_deref().is_some_and(|echo| echo != name) {
                return Err(ToolEventError::ToolName);
            }
        } else {
            if !pending.is_empty() {
                return Err(ToolEventError::OpenCalls);
            }
            if message.tool_call_id.is_some()
                || (!message.tool_calls.is_empty() && message.role != MessageRole::Assistant)
            {
                return Err(ToolEventError::MalformedContext);
            }
            for call in &message.tool_calls {
                valid_label(&call.id, "provider call id")?;
                valid_label(&call.name, "tool name")?;
                if pending
                    .insert(call.id.as_str(), call.name.as_str())
                    .is_some()
                {
                    return Err(ToolEventError::Correlation);
                }
            }
        }
    }
    if !pending.is_empty() {
        return Err(ToolEventError::OpenCalls);
    }
    Ok(())
}

/// Count encoded bytes without allocating a copy of an oversized value.
struct LimitedCounter {
    limit: usize,
    bytes: usize,
    exhausted: bool,
}

impl Write for LimitedCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.bytes {
            self.exhausted = true;
            return Err(io::Error::other("event byte budget exceeded"));
        }
        self.bytes += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Protocol failures carry identities/dimensions, never tool payloads.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolEventError {
    #[error("unsupported tool event version {0}")]
    UnsupportedVersion(u16),
    #[error("invalid {0}")]
    InvalidLabel(&'static str),
    #[error("tool arguments must be an object")]
    InvalidArguments,
    #[error("tool result does not match an outstanding call")]
    Correlation,
    #[error("tool result name does not match its call")]
    ToolName,
    #[error("conflicting execution {0}")]
    ConflictingExecution(TaskId),
    #[error("tool event {0} budget exceeded")]
    Budget(&'static str),
    #[error("tool event serialization failed")]
    Serialization,
    #[error("conversation has unanswered tool calls")]
    OpenCalls,
    #[error("conversation has an invalid tool message shape")]
    MalformedContext,
}
