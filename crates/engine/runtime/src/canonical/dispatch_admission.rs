//! Canonical resolve/validate/governance/frozen-approval admission and invocation.
//! Extracted from execute without changing the approval state machine.
use super::*;
use std::future::Future;
use std::pin::Pin;

/// Execution stage of one admitted call: the invocation, nothing else.
///
/// The frozen-invocation and plain paths collapse into the session-scoped
/// entry: `frozen: None` routes to `invoke` by the trait's own contract, so
/// the observable behavior of both branches is unchanged — only the session
/// scope now travels with the call.
async fn run_admitted_call(admitted: &AdmittedCall) -> ToolResult {
    admitted
        .tool
        .invoke_in_session(
            &admitted.call,
            admitted.frozen_invocation.as_ref(),
            Some(admitted.session),
        )
        .await
}

/// Batch runner for one dispatch window: slot index to invocation future.
pub(super) struct WindowRunner {
    pub(super) entries: Vec<WindowEntry>,
}

impl CallRunner<ToolResult> for WindowRunner {
    fn start(
        &self,
        index: usize,
        _call: &ScheduledCall,
    ) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> {
        match &self.entries[index].execution {
            WindowExecution::Resolved(result) => Box::pin(std::future::ready(result.clone())),
            WindowExecution::Ready { tool, .. } => {
                let tool = Arc::clone(tool);
                let call = self.entries[index].call.clone();
                let session = self.entries[index].session;
                Box::pin(async move { tool.invoke_in_session(&call, None, Some(session)).await })
            }
        }
    }
}

impl Runtime {
    /// Admission stage of one tool call: resolve the tool, authorize the
    /// dispatch, and record the dispatch boundary. It never invokes anything —
    /// a [`ToolAdmission::Ready`] verdict hands the invocation to the execution
    /// stage, which the dispatch surface may overlap for parallel windows.
    ///
    /// When `preapproved` is true the tool has already passed human approval
    /// and must be dispatched without a second governance evaluation. The
    /// approved dispatch must execute the exact stored frozen invocation.
    pub(super) async fn admit_one_tool(
        &self,
        trace: &mut ExecutionTrace,
        session: &mut Session,
        session_id: &SessionId,
        request_id: RequestId,
        trace_id: TraceId,
        call: &ToolCall,
        round: u32,
        security_context: Option<&TurnSecurityContext>,
        preapproved: bool,
        approved_approval: Option<&PendingApproval>,
        automatic: bool,
    ) -> RuntimeResult<ToolAdmission> {
        let clock = self.clock.as_ref();

        // Internal control commands are registered surfaces, not business-name
        // branches. They are runtime-owned state requests: no capability lookup,
        // no governance verdict, and no human approval over a pure state flip.
        // The registry guarantees unique names; if a tool somehow appears under
        // the same name later, the dispatch fails closed instead of picking one.
        if let Some(control) = self.capabilities.find_control_by_name(&call.name) {
            if !self.dispatcher.allows(*session_id, control.id()) {
                return Ok(ToolAdmission::Resolved(
                    crate::canonical::dispatcher::failure(
                        call,
                        crate::canonical::dispatcher::DispatchErrorCode::ScopeDenied,
                        "control is outside this session's tool scope",
                    ),
                ));
            }
            let tool_shadowed = self.capabilities.find_by_name(&call.name).is_some();
            if tool_shadowed {
                return Ok(ToolAdmission::Resolved(
                    ToolResult::permanent_error(
                        &call.id,
                        format!(
                            "internal control {:?} collides with a registered tool of the same name",
                            call.name
                        ),
                    )
                    .with_name(&call.name),
                ));
            }
            let mut ctx = ControlContext::new(session, request_id, trace_id, clock);
            return Ok(ToolAdmission::Resolved(control.invoke(call, &mut ctx)));
        }

        let Some(tool) = self.resolve_tool(&call.name) else {
            let available = self
                .tool_declarations()
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let reason = if available.is_empty() {
                format!("no tool named {:?}; no tools are available", call.name)
            } else {
                format!(
                    "no tool named {:?}; available tools: {available}",
                    call.name
                )
            };
            trace.record(
                Timestamp::from_clock(clock),
                TraceEvent::CapabilityUnavailable {
                    requested: call.name.clone(),
                    tool_call_id: call.id.clone(),
                    reason: reason.clone(),
                    round,
                },
            );
            session.record(
                request_id,
                trace_id,
                SessionEventKind::ToolFailed {
                    capability: None,
                    tool_call_id: call.id.clone(),
                    error: reason.clone(),
                    round,
                },
                clock,
            );
            return Ok(ToolAdmission::Resolved(
                crate::canonical::dispatcher::failure(
                    call,
                    crate::canonical::dispatcher::DispatchErrorCode::Unregistered,
                    reason,
                ),
            ));
        };

        if !automatic && (!tool.model_visible() || !tool.module_contract().model_visible()) {
            return Ok(ToolAdmission::Resolved(
                crate::canonical::dispatcher::failure(
                    call,
                    crate::canonical::dispatcher::DispatchErrorCode::PermissionDenied,
                    "tool is reserved for authenticated module calls",
                ),
            ));
        }
        if let Err(result) = self.dispatcher.validate(
            Some(*session_id),
            tool.as_ref(),
            call,
            self.tool_is_legacy(tool.id()),
        ) {
            return Ok(ToolAdmission::Resolved(result));
        }
        let capability = tool.id().clone();

        if preapproved {
            let Some(approval) = approved_approval else {
                let reason = "approved dispatch is missing its frozen approval".to_string();
                session.record(
                    request_id,
                    trace_id,
                    SessionEventKind::ToolFailed {
                        capability: Some(capability.clone()),
                        tool_call_id: call.id.clone(),
                        error: reason.clone(),
                        round,
                    },
                    clock,
                );
                return Ok(ToolAdmission::Resolved(
                    ToolResult::permanent_error(&call.id, reason).with_name(&call.name),
                ));
            };

            if let Err(refusal) = self.verify_frozen_approval_binding(approval, call, tool.as_ref())
            {
                // Fail closed: the stored approval does not match the live
                // capability, its fingerprint, or its action binding. Do not
                // invoke (G2.5 拒绝即帧: 载荷被改就地拒绝并落审计行).
                if let Some(audit) = refusal.audit() {
                    session.record(
                        request_id,
                        trace_id,
                        SessionEventKind::ApprovalBindingMismatch {
                            approval_id: approval.approval_id,
                            tool_call_id: call.id.clone(),
                            drift: audit.drift.label().to_string(),
                            expected_digest: audit.expected_digest.clone(),
                            actual_digest: audit.actual_digest.clone(),
                            round,
                        },
                        clock,
                    );
                }
                let reason = refusal.message();
                session.record(
                    request_id,
                    trace_id,
                    SessionEventKind::ToolFailed {
                        capability: Some(capability.clone()),
                        tool_call_id: call.id.clone(),
                        error: reason.clone(),
                        round,
                    },
                    clock,
                );
                return Ok(ToolAdmission::Resolved(
                    ToolResult::permanent_error(&call.id, reason).with_name(&call.name),
                ));
            }

            // Observation layer at the dispatch boundary: the side-effect
            // audit event is checked before the dispatch is recorded and the
            // invocation runs.
            self.observe_side_effect(session, request_id, trace_id, round, call, &capability)?;

            trace.record(
                Timestamp::from_clock(clock),
                TraceEvent::CapabilityDispatched {
                    capability: capability.clone(),
                    tool_call_id: call.id.clone(),
                    round,
                },
            );

            let tool = match self.bind_admitted_tool(
                tool,
                call,
                *session_id,
                request_id,
                trace_id,
                security_context,
                true,
                automatic,
            ) {
                Ok(tool) => tool,
                Err(result) => return Ok(ToolAdmission::Resolved(result)),
            };
            return Ok(ToolAdmission::Ready(AdmittedCall {
                capability,
                tool,
                call: call.clone(),
                frozen_invocation: approval.effective_invocation.clone(),
                session: *session_id,
            }));
        }

        let action = Action::CapabilityDispatch {
            capability: &capability,
            arguments: &call.arguments,
        };
        let label = action.label();
        let verdict = self
            .governance
            .evaluate_verbose(
                &GovernanceRequest::new(action, *session_id, trace_id, round)
                    .with_action_id(&call.id)
                    .with_optional_security_context(security_context),
            )
            .await;
        let (hook, owner, decision) = if matches!(verdict.decision, Decision::Allow)
            && tool.execution_policy().permission
                == apeireth_plugin::ToolPermissionPolicy::RequireApproval
        {
            (
                "tool.execution_policy".to_string(),
                self.capabilities
                    .registration(tool.id())
                    .and_then(|entry| entry.module),
                Decision::RequireApproval {
                    reason: "registered tool policy requires explicit approval".into(),
                },
            )
        } else {
            (verdict.hook, verdict.owner, verdict.decision)
        };

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::GovernanceEvaluated {
                hook: hook.clone(),
                owner,
                action: label.to_string(),
                decision: decision.label().to_string(),
                reason: decision.reason().map(str::to_owned),
                round,
            },
        );

        match decision {
            Decision::Allow => {}
            Decision::Deny { reason } => {
                session.record(
                    request_id,
                    trace_id,
                    SessionEventKind::GovernanceDenied {
                        hook,
                        action: label.to_string(),
                        reason: reason.clone(),
                        round,
                    },
                    clock,
                );
                return Ok(ToolAdmission::Resolved(
                    ToolResult::permanent_error(
                        &call.id,
                        format!("refused by governance: {reason}"),
                    )
                    .with_name(&call.name),
                ));
            }
            Decision::RequireApproval { reason } => {
                if self.isolated_task() {
                    return Ok(ToolAdmission::Resolved(ToolResult::permanent_error(
                        &call.id, format!("subloop capability requires interactive approval which is not permitted in subloops: {reason}")
                    ).with_name(&call.name)));
                }
                let effective_invocation = match tool.freeze_invocation(call) {
                    Ok(frozen) => frozen,
                    Err(result) => {
                        // A tool whose effective invocation cannot be prepared
                        // must not produce a pending approval: asking a human to
                        // approve an invalid operation is misleading.
                        session.record(
                            request_id,
                            trace_id,
                            SessionEventKind::ToolFailed {
                                capability: Some(capability.clone()),
                                tool_call_id: call.id.clone(),
                                error: result.render(),
                                round,
                            },
                            clock,
                        );
                        return Ok(ToolAdmission::Resolved(result));
                    }
                };

                return Ok(ToolAdmission::Pending(PendingDispatch {
                    capability_id: capability,
                    tool_name: call.name.clone(),
                    tool_call: call.clone(),
                    effective_invocation,
                    tool_schema_version: tool_schema_version_digest(tool.as_ref()),
                    governance_hook: hook,
                    governance_reason: reason,
                }));
            }
        }

        // Observation layer at the dispatch boundary: the side-effect audit
        // event is checked before the dispatch is recorded and the invocation
        // runs.
        self.observe_side_effect(session, request_id, trace_id, round, call, &capability)?;

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::CapabilityDispatched {
                capability: capability.clone(),
                tool_call_id: call.id.clone(),
                round,
            },
        );

        let tool = match self.bind_admitted_tool(
            tool,
            call,
            *session_id,
            request_id,
            trace_id,
            security_context,
            false,
            automatic,
        ) {
            Ok(tool) => tool,
            Err(result) => return Ok(ToolAdmission::Resolved(result)),
        };
        Ok(ToolAdmission::Ready(AdmittedCall {
            capability,
            tool,
            call: call.clone(),
            frozen_invocation: None,
            session: *session_id,
        }))
    }

    /// Resolve, authorize, and run one tool call along the serial path.
    ///
    /// Admission and execution stay separate ([`Self::admit_one_tool`] and the
    /// invocation) so the parallel window can overlap only the execution stage;
    /// here the two run back to back, exactly one call at a time.
    pub(in crate::canonical) async fn dispatch_one_tool(
        &self,
        trace: &mut ExecutionTrace,
        session: &mut Session,
        session_id: &SessionId,
        request_id: RequestId,
        trace_id: TraceId,
        call: &ToolCall,
        round: u32,
        security_context: Option<&TurnSecurityContext>,
        preapproved: bool,
        approved_approval: Option<&PendingApproval>,
        automatic: bool,
    ) -> RuntimeResult<ToolDispatch> {
        // 步数计量 (工具调用计数): 每进入派发的调用记一步; 恢复一个已批准
        // 调用是同一调用的续跑, 不重复记步 (首次派发已记)。
        if !preapproved {
            self.meter_tool_step(session).await?;
        }
        match self
            .admit_one_tool(
                trace,
                session,
                session_id,
                request_id,
                trace_id,
                call,
                round,
                security_context,
                preapproved,
                approved_approval,
                automatic,
            )
            .await?
        {
            ToolAdmission::Resolved(result) => {
                if !automatic {
                    self.record_model_tool_event(session, request_id, trace_id, call, &result);
                }
                Ok(ToolDispatch::Result(result))
            }
            ToolAdmission::Pending(pending) => Ok(ToolDispatch::Pending(pending)),
            ToolAdmission::Ready(admitted) => {
                let result = run_admitted_call(&admitted).await;
                if !automatic {
                    self.record_model_tool_event(session, request_id, trace_id, call, &result);
                }
                self.record_call_completed(
                    session,
                    trace,
                    request_id,
                    trace_id,
                    round,
                    &admitted.call,
                    &admitted.capability,
                    &result,
                );
                Ok(ToolDispatch::Result(result))
            }
        }
    }
}
