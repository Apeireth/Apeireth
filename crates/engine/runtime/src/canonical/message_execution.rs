//! Direct registered dispatch for private module mailboxes. No Provider execution loop.
use super::dispatcher::{failure, DispatchErrorCode};
use super::execute::ToolDispatch;
use super::module_messages::{cancelled, MessageInvocation};
use super::{Runtime, SessionEventKind};
use apeireth_core::kernel::{AgentId, RequestId, SessionId};
use apeireth_plugin::module_messages::{ModuleMessageError, ModuleMessageErrorCode};
use apeireth_protocol::canonical::{ToolCall, ToolEventSourceV1, ToolEventV1, ToolResult};
use std::sync::Arc;

fn execution_error(error: impl ToString, started: bool) -> ModuleMessageError {
    let mut result = ModuleMessageError::new(ModuleMessageErrorCode::Execution, error.to_string());
    result.may_have_executed = started;
    result
}
impl Runtime {
    pub(crate) async fn execute_message_tool(
        &self,
        session_id: SessionId,
        agent: AgentId,
        invocation: Arc<MessageInvocation>,
    ) -> Result<ToolResult, ModuleMessageError> {
        let _permit = self.sessions.write_permit(session_id).await;
        let grant = self
            .authorize_session(session_id)
            .await
            .map_err(|e| execution_error(e, false))?;
        let mut session = self
            .sessions
            .load_or_create(session_id)
            .await
            .map_err(|e| execution_error(e, false))?;
        if session.principal != invocation.envelope.principal
            || invocation.parent.as_ref().is_some_and(|p| {
                p.principal != session.principal
                    || p.authorization.principal_epoch != session.authorization_epoch
            })
        {
            return Err(ModuleMessageError::new(
                ModuleMessageErrorCode::Unauthorized,
                "mailbox principal/epoch does not match the delegation",
            ));
        }
        let capability = invocation
            .envelope
            .target_capability
            .as_ref()
            .expect("validated target capability");
        let registration = self.capabilities.registration(capability).ok_or_else(|| {
            ModuleMessageError::new(
                ModuleMessageErrorCode::TargetUnavailable,
                "target unregistered",
            )
        })?;
        if registration.module.as_ref() != Some(&invocation.receiver.definition)
            || !registration.scope.is_open()
            || self
                .plugins
                .instance_id(&invocation.receiver.definition)
                .ok()
                != Some(invocation.receiver.instance)
        {
            return Err(ModuleMessageError::new(
                ModuleMessageErrorCode::TargetUnavailable,
                "target generation changed",
            ));
        }
        let tool = self
            .capabilities
            .capabilities()
            .into_iter()
            .find(|t| t.id() == capability)
            .ok_or_else(|| {
                ModuleMessageError::new(
                    ModuleMessageErrorCode::TargetUnavailable,
                    "target unavailable",
                )
            })?;
        let call = ToolCall {
            id: format!("msg_{}", invocation.envelope.message_id),
            name: tool.declaration().name,
            arguments: invocation.envelope.payload.clone(),
        };
        let request = RequestId::new();
        let trace = invocation.envelope.trace_id;
        session.record(
            request,
            trace,
            SessionEventKind::ModuleMessageStarted {
                agent,
                envelope: invocation.envelope.clone(),
                call_id: call.id.clone(),
            },
            self.clock.as_ref(),
        );
        self.sessions
            .save(&session)
            .await
            .map_err(|e| execution_error(e, false))?;
        invocation
            .started
            .store(true, std::sync::atomic::Ordering::Release);
        let mut execution_trace = super::trace::ExecutionTrace::new(trace, session_id, request);
        let work = async {
            tokio::select! {
                biased;
                _=cancelled(&invocation.cancellation)=>Ok(failure(&call,DispatchErrorCode::Stopping,"message cancelled; execution may be incomplete")),
                result=tokio::time::timeout(invocation.deadline.saturating_duration_since(std::time::Instant::now()),self.dispatch_one_tool(&mut execution_trace,&mut session,&session_id,request,trace,&call,0,invocation.security.as_ref(),false,None,true))=>match result {
                    Ok(Ok(ToolDispatch::Result(result)))=>Ok(result),
                    Ok(Ok(ToolDispatch::Pending(_)))=>Ok(failure(&call,DispatchErrorCode::PermissionDenied,"message operation requires interactive approval; not executed")),
                    Ok(Err(error))=>Err(error),
                    Err(_)=>Ok(failure(&call,DispatchErrorCode::Timeout,"message deadline exceeded; execution may be incomplete")),
                }
            }
        };
        let result = if let Some(grant) = grant {
            grant.run(work).await
        } else {
            work.await
        }
        .map_err(|e| execution_error(e, true))?;
        if std::time::Instant::now() >= invocation.deadline {
            let mut error = ModuleMessageError::new(
                ModuleMessageErrorCode::Timeout,
                "message deadline exceeded; execution may be incomplete",
            );
            error.may_have_executed = true;
            return Err(error);
        }
        let event = ToolEventV1::new(
            invocation.envelope.message_id,
            ToolEventSourceV1::ModuleMessage {
                module_id: invocation.envelope.source.definition.clone(),
                module_instance_id: invocation.envelope.source.instance,
                target_module_id: invocation.receiver.definition.clone(),
                target_module_instance_id: invocation.receiver.instance,
                message_id: invocation.envelope.message_id,
            },
            call,
            result.clone(),
        )
        .map_err(|e| execution_error(e, true))?;
        session.record(
            request,
            trace,
            SessionEventKind::ToolEventRecorded { fact: event },
            self.clock.as_ref(),
        );
        self.sessions
            .save(&session)
            .await
            .map_err(|e| execution_error(e, true))?;
        Ok(result)
    }
}
