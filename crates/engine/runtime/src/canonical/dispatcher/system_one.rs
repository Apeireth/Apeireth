//! Deterministic checks remain on both sides of asynchronous model review.
use super::*;
use apeireth_plugin::system_one::*;
use sha2::{Digest, Sha256};

impl ToolDispatcher {
    pub(super) async fn review_internal(
        &self,
        tool: &dyn ToolCapability,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &mut ToolExecutionContext,
        legacy: bool,
    ) -> Result<(), ToolResult> {
        self.validate(context.session, tool, call, legacy)?;
        if !self.context_current(context) {
            return Err(failure(
                call,
                DispatchErrorCode::PermissionDenied,
                "internal execution context is no longer authorized",
            ));
        }
        let gate = self.system_one.as_ref().ok_or_else(|| {
            failure(
                call,
                DispatchErrorCode::SystemOneUnavailable,
                "System One is required for internal execution; supervisor not configured",
            )
        })?;
        let instance = context.module_instance.ok_or_else(|| {
            failure(
                call,
                DispatchErrorCode::SystemOneUnavailable,
                "internal execution requires a registered module instance",
            )
        })?;
        let bytes = serde_json::to_vec(&(tool.id(), &call.arguments, frozen.map(|f| &f.payload)))
            .map_err(|_| {
            failure(
                call,
                DispatchErrorCode::InvalidArguments,
                "cannot digest internal action",
            )
        })?;
        let digest = format!("{:x}", Sha256::digest(bytes));
        let owner = context
            .session
            .and_then(|session| self.usage.as_ref()?.owner(session));
        let mut delegation_chain = match &context.caller {
            ToolCaller::Module(module) => vec![module.clone()],
            _ => vec![],
        };
        if let Some(apeireth_plugin::agents::AgentOwner::Module { definition, .. }) = &owner {
            if !delegation_chain.contains(definition) {
                delegation_chain.push(definition.clone());
            }
        }
        let request = SystemOneRequest {
            version: SYSTEM_ONE_PROTOCOL_VERSION,
            policy_version: SYSTEM_ONE_POLICY_VERSION,
            risk: if tool.execution_policy().permission
                == apeireth_plugin::ToolPermissionPolicy::RequireApproval
            {
                SystemOneRisk::ExplicitApprovalRequired
            } else {
                SystemOneRisk::Governed
            },
            review_id: TaskId::new(),
            module_definition_id: context.module.clone(),
            module_instance_id: instance,
            agent_id: context.agent,
            agent_owner: owner,
            session_id: context.session,
            turn_id: context.turn,
            caller_principal: context.principal.clone(),
            delegation_chain,
            capability_id: tool.id().clone(),
            action_arguments_digest: digest,
            scope: context.session.and_then(|id| self.scope_snapshot(id)),
            intent_id: context.authorization.intent_id.clone(),
            authorization_epoch: context.authorization.principal_epoch.clone(),
            approved: context.authorization.approved,
            deadline_remaining_ms: context
                .deadline
                .saturating_duration_since(Instant::now())
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            cancel_token: context.cancellation.clone(),
            trace_id: context.trace,
            call_id: context.call_id.clone(),
            arguments: call.arguments.clone(),
            frozen_display: frozen.map(|f| f.display.clone()),
        };
        let generation = gate.review(&request).await.map_err(|error| {
            use crate::canonical::system_one::ReviewFailure;
            let code = match error {
                ReviewFailure::Denied => DispatchErrorCode::SystemOneDenied,
                ReviewFailure::Escalated => DispatchErrorCode::SystemOneEscalated,
                ReviewFailure::Unavailable => DispatchErrorCode::SystemOneUnavailable,
                ReviewFailure::Timeout => DispatchErrorCode::Timeout,
                ReviewFailure::Audit => DispatchErrorCode::SystemOneAuditFailed,
                ReviewFailure::Cancelled | ReviewFailure::Revoked => DispatchErrorCode::Stopping,
                ReviewFailure::Invalid => DispatchErrorCode::SystemOneInvalidVerdict,
            };
            failure(call, code, "internal execution refused by mandatory review; no permission is granted by a model verdict")
        })?;
        self.validate(context.session, tool, call, legacy)?;
        if !gate.is_current(generation) || !self.context_current(context) {
            return Err(failure(
                call,
                DispatchErrorCode::PermissionDenied,
                "authority changed during internal execution review",
            ));
        }
        context.internal_review = Some(InternalReviewReceipt::from_audited_admission(
            tool.id().clone(),
            call,
            frozen,
            context,
        ));
        Ok(())
    }
    fn context_current(&self, context: &ToolExecutionContext) -> bool {
        if context.cancellation.is_cancelled()
            || !context.module_tasks.is_open()
            || Instant::now() >= context.deadline
        {
            return false;
        }
        if let Some(session) = context.session {
            if self.principal(session) != context.principal {
                return false;
            }
            if self
                .authorization_epochs
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .get(&session)
                != context.authorization.principal_epoch.as_ref()
            {
                return false;
            }
        }
        if let (Some(authority), Some(principal)) =
            (&self.principal_authorizations, &context.principal)
        {
            if authority.get(principal).is_none_or(|grant| {
                Some(grant.epoch()) != context.authorization.principal_epoch.as_deref()
            }) {
                return false;
            }
        }
        true
    }
}
