//! Supervisor decisions add review; deterministic authority remains with the Host.
use crate::agents::{AgentOwner, AgentPrincipal};
use crate::{FrozenInvocation, ToolCancellation, ToolExecutionContext};
use apeireth_core::kernel::{
    AgentId, CapabilityId, ModuleInstanceId, PluginId, RequestId, SessionId, TaskId, TraceId,
};
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const SYSTEM_ONE_PROTOCOL_VERSION: u16 = 1;
pub const SYSTEM_ONE_POLICY_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemOneDecision {
    Allow,
    Deny,
    Escalate,
}

/// Registered permission class, not a model-generated severity estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemOneRisk {
    Governed,
    ExplicitApprovalRequired,
}

#[derive(Clone, Serialize)]
pub struct SystemOneRequest {
    pub version: u16,
    pub policy_version: u32,
    pub risk: SystemOneRisk,
    pub review_id: TaskId,
    pub module_definition_id: PluginId,
    pub module_instance_id: ModuleInstanceId,
    pub agent_id: AgentId,
    pub agent_owner: Option<AgentOwner>,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<RequestId>,
    pub caller_principal: Option<AgentPrincipal>,
    pub delegation_chain: Vec<PluginId>,
    pub capability_id: CapabilityId,
    pub action_arguments_digest: String,
    pub scope: Option<Vec<CapabilityId>>,
    pub intent_id: Option<String>,
    pub authorization_epoch: Option<String>,
    pub approved: bool,
    pub deadline_remaining_ms: u64,
    #[serde(skip)]
    pub cancel_token: ToolCancellation,
    pub trace_id: Option<TraceId>,
    pub call_id: String,
    pub arguments: serde_json::Value,
    /// Only the capability's safe frozen display is sent to the supervisor.
    pub frozen_display: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemOneVerdict {
    pub version: u16,
    pub policy_version: u32,
    pub review_id: TaskId,
    pub action_arguments_digest: String,
    pub decision: SystemOneDecision,
}
impl SystemOneVerdict {
    pub fn for_request(request: &SystemOneRequest, decision: SystemOneDecision) -> Self {
        Self {
            version: request.version,
            policy_version: request.policy_version,
            review_id: request.review_id,
            action_arguments_digest: request.action_arguments_digest.clone(),
            decision,
        }
    }
    pub fn matches(&self, request: &SystemOneRequest) -> bool {
        self.version == SYSTEM_ONE_PROTOCOL_VERSION
            && self.policy_version == SYSTEM_ONE_POLICY_VERSION
            && self.version == request.version
            && self.policy_version == request.policy_version
            && self.review_id == request.review_id
            && self.action_arguments_digest == request.action_arguments_digest
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SystemOneError {
    #[error("supervisor unavailable")]
    Unavailable,
    #[error("invalid supervisor response")]
    Malformed,
    #[error("supervisor deadline exceeded")]
    Timeout,
    #[error("review audit unavailable")]
    AuditFailed,
}
#[async_trait]
pub trait SystemOneSupervisor: Send + Sync {
    async fn review(&self, request: &SystemOneRequest) -> Result<SystemOneVerdict, SystemOneError>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemOneReviewStatus {
    Allowed,
    Denied,
    Escalated,
    Unavailable,
    Malformed,
    Timeout,
    Cancelled,
    Revoked,
}
/// Review facts contain identities and digests, never prompts, arguments or frozen secrets.
/// An Allowed review is not a claim that the native operation completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemOneAuditRecord {
    pub version: u16,
    pub policy_version: u32,
    pub risk: SystemOneRisk,
    pub review_id: TaskId,
    pub module_definition_id: PluginId,
    pub module_instance_id: ModuleInstanceId,
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<RequestId>,
    pub trace_id: Option<TraceId>,
    pub call_id: String,
    pub capability_id: CapabilityId,
    pub action_arguments_digest: String,
    pub status: SystemOneReviewStatus,
}
impl SystemOneAuditRecord {
    pub fn from_review(request: &SystemOneRequest, status: SystemOneReviewStatus) -> Self {
        Self {
            version: request.version,
            policy_version: request.policy_version,
            risk: request.risk,
            review_id: request.review_id,
            module_definition_id: request.module_definition_id.clone(),
            module_instance_id: request.module_instance_id,
            agent_id: request.agent_id,
            session_id: request.session_id,
            turn_id: request.turn_id,
            trace_id: request.trace_id,
            call_id: request.call_id.clone(),
            capability_id: request.capability_id.clone(),
            action_arguments_digest: request.action_arguments_digest.clone(),
            status,
        }
    }
}
#[async_trait]
pub trait SystemOneAudit: Send + Sync {
    /// Acknowledgement must mean the Host's chosen audit sink accepted the fact.
    /// Durable implementations must persist before returning Ok.
    async fn record(&self, record: &SystemOneAuditRecord) -> Result<(), SystemOneError>;
}

/// Non-serializable per-call receipt, issued by the Host execution seam only
/// after review, audit acknowledgement and deterministic revalidation.
#[derive(Clone)]
pub struct InternalReviewReceipt {
    capability: CapabilityId,
    call: ToolCall,
    frozen: Option<FrozenInvocation>,
    module: PluginId,
    instance: Option<ModuleInstanceId>,
    agent: AgentId,
    session: Option<SessionId>,
    principal: Option<AgentPrincipal>,
    authorization_epoch: Option<String>,
    approved: bool,
    intent: Option<String>,
    authorization_context: Option<String>,
    caller: crate::ToolCaller,
    turn: Option<RequestId>,
    trace: Option<TraceId>,
    cancellation: ToolCancellation,
    deadline: std::time::Instant,
}
impl InternalReviewReceipt {
    /// Trusted Host API; not a grant callable through any model or message payload.
    pub fn from_audited_admission(
        capability: CapabilityId,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> Self {
        Self {
            capability,
            call: call.clone(),
            frozen: frozen.cloned(),
            module: context.module.clone(),
            instance: context.module_instance,
            agent: context.agent,
            session: context.session,
            principal: context.principal.clone(),
            authorization_epoch: context.authorization.principal_epoch.clone(),
            approved: context.authorization.approved,
            intent: context.authorization.intent_id.clone(),
            authorization_context: context.authorization.authorization_context.clone(),
            caller: context.caller.clone(),
            turn: context.turn,
            trace: context.trace,
            cancellation: context.cancellation.clone(),
            deadline: context.deadline,
        }
    }
    pub fn matches(
        &self,
        capability: &CapabilityId,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> bool {
        &self.capability == capability
            && &self.call == call
            && self.frozen.as_ref() == frozen
            && self.module == context.module
            && self.instance == context.module_instance
            && self.agent == context.agent
            && self.session == context.session
            && self.principal == context.principal
            && self.authorization_epoch == context.authorization.principal_epoch
            && self.approved == context.authorization.approved
            && self.intent == context.authorization.intent_id
            && self.authorization_context == context.authorization.authorization_context
            && self.caller == context.caller
            && self.turn == context.turn
            && self.trace == context.trace
            && self.cancellation.same_call(&context.cancellation)
            && !self.cancellation.is_cancelled()
            && self.deadline == context.deadline
            && std::time::Instant::now() < self.deadline
            && context.module_tasks.is_open()
    }
}
pub fn requires_system_one(contract: &crate::module_contract::ModuleToolContract) -> bool {
    contract.surface == crate::module_contract::ToolSurface::Internal
        || contract.effect == crate::module_contract::EffectClass::Protected
}
pub fn require_internal_review(
    capability: &CapabilityId,
    call: &ToolCall,
    frozen: Option<&FrozenInvocation>,
    context: &ToolExecutionContext,
) -> Result<(), ToolResult> {
    if context
        .internal_review
        .as_ref()
        .is_some_and(|receipt| receipt.matches(capability, call, frozen, context))
    {
        Ok(())
    } else {
        Err(ToolResult::permanent_error(
            &call.id,
            "internal execution requires a current audited System One receipt",
        )
        .with_name(&call.name))
    }
}
