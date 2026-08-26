//! The agent loop: the runtime's single semantic entry point.
//!
//! # The chain
//!
//! ```text
//!   user request
//!        v
//!   normalized request  <-- session transcript + active tool declarations
//!        v
//!   governance (completion)
//!        v
//!   provider router --> provider
//!        v
//!   tool calls?  -- no --> final response
//!        | yes
//!        v
//!   governance (dispatch) --> capability lookup --> plugin dispatch
//!        v
//!   tool result -> transcript -> provider again
//! ```
//!
//! Everything above happens here and nowhere else. A gateway or CLI that runs
//! its own version of this chain is a second runtime, and the two will diverge
//! at the first behaviour change.
//!
//! # Approval is not an error
//!
//! `Runtime::execute_outcome` returns a [`TurnOutcome`]: either a completed
//! turn or a pending approval. `Runtime::execute` is a compatibility wrapper
//! that maps a pending approval back to the old `RuntimeError::ApprovalRequired`
//! for callers that have not adopted the canonical outcome model yet. New
//! callers should use `execute_outcome` and `resolve_approval`.
//!
//! # Why a tool failure is not a turn failure
//!
//! Three things can go wrong with a tool call, and all three produce a
//! [`ToolResult`] handed back to the model rather than an aborted turn: the
//! model named a tool that does not exist, governance refused the call, or the
//! tool itself failed. In each case the model is told what happened and gets to
//! respond — which is the entire point of a loop. Aborting would discard a turn
//! the model could have recovered from, and hide the reason from the only party
//! able to act on it.
//!
//! Two things do abort: governance denying the *completion* itself, and the
//! round limit. Neither is something the model can recover from by trying again.

use apeireth_core::kernel::{ApprovalId, CapabilityId, RequestId, SessionId, Timestamp, TraceId};
use apeireth_governance::{Action, Decision, GovernanceRequest};
use apeireth_protocol::canonical::{
    NormalizedMessage, NormalizedRequest, NormalizedResponse, NormalizedTool, NormalizedUsage,
    ToolCall, ToolResult,
};

use super::approval::{
    operation_fingerprint_with_invocation, ApprovalDecision, ApprovalStatus,
    FrozenTurnContinuation, PendingApproval, PendingApprovalView,
};
use super::error::{RuntimeError, RuntimeResult};
use super::runtime::Runtime;
use super::session::{Session, SessionEventKind};
use super::trace::{ExecutionTrace, TraceEvent};

/// One turn's input.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    /// The conversation to continue. A session that does not exist is created.
    pub session: SessionId,
    /// What the user said.
    pub input: String,
    /// Model to use, or the runtime's default when absent.
    pub model: Option<String>,
    /// System instruction, applied only when the transcript is empty.
    ///
    /// Applied once rather than per turn so that a resumed session does not
    /// accumulate duplicate system messages, which quietly change behaviour and
    /// cost tokens on every subsequent request.
    pub system: Option<String>,
}

impl TurnRequest {
    /// A turn against `session` saying `input`.
    pub fn new(session: SessionId, input: impl Into<String>) -> Self {
        Self {
            session,
            input: input.into(),
            model: None,
            system: None,
        }
    }

    /// Use a specific model for this turn.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Seed a new session with a system instruction.
    #[must_use]
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }
}

/// One turn's outcome.
///
/// Note the absence of any raw-reasoning field. What the turn *did* is in
/// [`TurnResponse::trace`]; see [`super::trace`].
#[derive(Debug, Clone)]
pub struct TurnResponse {
    /// The conversation this belongs to.
    pub session: SessionId,
    /// This turn's request id.
    pub request: RequestId,
    /// The assistant's final text.
    pub text: String,
    /// The provider that produced the final response.
    pub served_by: CapabilityId,
    /// Token accounting for the final response.
    pub usage: NormalizedUsage,
    /// How many provider round-trips the turn took.
    pub rounds: u32,
    /// Everything the runtime did, in order.
    pub trace: ExecutionTrace,
}

/// The result of running one turn: it either completed or paused for human
/// approval.
#[derive(Debug, Clone)]
pub enum TurnOutcome {
    /// The turn reached a final assistant response.
    Completed(TurnResponse),
    /// The turn is suspended until a human resolves the returned approval.
    PendingApproval(PendingApprovalView),
}

impl TurnOutcome {
    /// The completed response, when the turn completed.
    pub fn completed(self) -> Option<TurnResponse> {
        match self {
            Self::Completed(response) => Some(response),
            Self::PendingApproval(_) => None,
        }
    }
}

/// The result of resolving one pending approval.
#[derive(Debug, Clone)]
pub enum ApprovalResolution {
    /// The resolution was accepted and the turn resumed. This may complete the
    /// turn or pause again on a later tool call.
    Resumed(TurnOutcome),
    /// The approval had already reached a terminal state.
    AlreadyResolved { status: ApprovalStatus },
    /// The approval expired before it was resolved.
    Expired,
    /// The approval id is unknown for this session.
    NotFound,
}

enum ToolDispatch {
    Result(ToolResult),
    Pending {
        capability_id: CapabilityId,
        tool_name: String,
        tool_call: ToolCall,
        effective_invocation: Option<serde_json::Value>,
        governance_hook: String,
        governance_reason: String,
    },
}

impl Runtime {
    /// Run one turn to completion or pending approval.
    ///
    /// This is the canonical outcome-model entry point. CLI, gateway, desktop
    /// and tests should migrate to this; [`Runtime::execute`] is the
    /// compatibility wrapper.
    pub async fn execute_outcome(&self, request: TurnRequest) -> RuntimeResult<TurnOutcome> {
        let lock = self.session_locks.acquire(request.session).await;
        let _guard = lock.lock().await;

        self.execute_outcome_locked(request).await
    }

    async fn execute_outcome_locked(&self, request: TurnRequest) -> RuntimeResult<TurnOutcome> {
        let trace_id = TraceId::new();
        let request_id = RequestId::new();
        let mut trace = ExecutionTrace::new(trace_id, request.session, request_id);

        let clock = self.clock.as_ref();
        let mut session = self.sessions.load_or_create(request.session).await?;

        if let Some(active) = session.active_approval_id {
            return Err(RuntimeError::SessionApprovalPending {
                session: request.session,
                approval: active,
            });
        }

        if session.is_empty() {
            if let Some(system) = &request.system {
                session.append(NormalizedMessage::system(system.clone()), clock);
            }
        }
        session.append(NormalizedMessage::user(request.input.clone()), clock);
        session.record(request_id, trace_id, SessionEventKind::TurnStarted, clock);
        self.sessions.save(&session).await?;

        let model = match request
            .model
            .clone()
            .or_else(|| self.config.default_model.clone())
        {
            Some(model) => model,
            None => {
                let error = RuntimeError::misconfigured(
                    "no model: the turn named none and the runtime has no default_model",
                );
                session.record(
                    request_id,
                    trace_id,
                    SessionEventKind::ExecutionFailed {
                        phase: "model_selection".into(),
                        error: error.to_string(),
                    },
                    clock,
                );
                self.sessions.save(&session).await?;
                return Err(error);
            }
        };

        let tools = self.plugins.tool_declarations();
        let continuation =
            FrozenTurnContinuation::start_of_round(request_id, trace_id, model.clone(), 1);

        self.advance(
            session,
            trace,
            request.session,
            request_id,
            trace_id,
            tools,
            continuation,
        )
        .await
    }

    /// Compatibility wrapper: run one turn and return the completed response.
    ///
    /// A pending approval is mapped to [`RuntimeError::ApprovalRequired`] so
    /// callers that have not adopted [`Runtime::execute_outcome`] keep their
    /// old behaviour. The semantic engine is the same.
    pub async fn execute(&self, request: TurnRequest) -> RuntimeResult<TurnResponse> {
        match self.execute_outcome(request).await? {
            TurnOutcome::Completed(response) => Ok(response),
            TurnOutcome::PendingApproval(view) => Err(RuntimeError::ApprovalRequired {
                hook: view.governance_hook,
                reason: view.governance_reason,
            }),
        }
    }

    /// Resolve a pending approval for one session.
    ///
    /// The resolver supplies only a decision and an optional human reason. It
    /// never supplies replacement tool arguments, cwd, script text, or process
    /// configuration. The frozen operation is executed exactly as stored.
    pub async fn resolve_approval(
        &self,
        session_id: SessionId,
        approval_id: ApprovalId,
        decision: ApprovalDecision,
    ) -> RuntimeResult<ApprovalResolution> {
        let lock = self.session_locks.acquire(session_id).await;
        let _guard = lock.lock().await;

        self.resolve_approval_locked(session_id, approval_id, decision)
            .await
    }

    async fn resolve_approval_locked(
        &self,
        session_id: SessionId,
        approval_id: ApprovalId,
        decision: ApprovalDecision,
    ) -> RuntimeResult<ApprovalResolution> {
        let mut session = match self.sessions.load(&session_id).await? {
            Some(session) => session,
            None => return Ok(ApprovalResolution::NotFound),
        };

        let Some(approval) = session.approvals.get(&approval_id).cloned() else {
            return Ok(ApprovalResolution::NotFound);
        };

        if approval.status != ApprovalStatus::Pending {
            return Ok(ApprovalResolution::AlreadyResolved {
                status: approval.status,
            });
        }

        let now = Timestamp::from_clock(self.clock.as_ref());
        if approval.is_expired(now) {
            let expired = {
                let mut expired = approval.clone();
                expired.status = ApprovalStatus::Expired;
                expired.human_reason = None;
                expired
            };
            session.approvals.insert(approval_id, expired);
            session.active_approval_id = None;
            session.record(
                approval.request_id,
                approval.trace_id,
                SessionEventKind::ApprovalResolved {
                    approval_id,
                    decision: "expired".into(),
                    round: approval.round,
                    human_reason: None,
                },
                self.clock.as_ref(),
            );
            self.sessions.save(&session).await?;
            return Ok(ApprovalResolution::Expired);
        }

        match decision {
            ApprovalDecision::Reject { reason } => {
                let rejected = {
                    let mut rejected = approval.clone();
                    rejected.status = ApprovalStatus::Rejected;
                    rejected.human_reason = reason.clone();
                    rejected
                };
                session.approvals.insert(approval_id, rejected);
                session.active_approval_id = None;
                session.record(
                    approval.request_id,
                    approval.trace_id,
                    SessionEventKind::ApprovalResolved {
                        approval_id,
                        decision: "rejected".into(),
                        round: approval.round,
                        human_reason: reason,
                    },
                    self.clock.as_ref(),
                );

                // The model gets a canonical rejection result and may recover.
                let rejection = ToolResult::permanent_error(
                    &approval.tool_call.id,
                    "operation rejected by user",
                )
                .with_name(&approval.tool_call.name);
                session.append(rejection.into_message(), self.clock.as_ref());

                let mut continuation = approval.continuation.clone();
                continuation.next_tool_index = continuation.next_tool_index.saturating_add(1);
                continuation.approved_tool_index = None;
                continuation.approved_approval_id = None;

                let tools = self.plugins.tool_declarations();
                let mut trace =
                    ExecutionTrace::new(approval.trace_id, session_id, approval.request_id);
                trace.record(
                    now,
                    TraceEvent::ApprovalResolved {
                        approval_id,
                        decision: "rejected".into(),
                        round: approval.round,
                    },
                );

                let outcome = self
                    .advance(
                        session,
                        trace,
                        session_id,
                        approval.request_id,
                        approval.trace_id,
                        tools,
                        continuation,
                    )
                    .await?;
                Ok(ApprovalResolution::Resumed(outcome))
            }
            ApprovalDecision::Approve => {
                let claimed = {
                    let mut claimed = approval.clone();
                    claimed.status = ApprovalStatus::Approved;
                    claimed.human_reason = None;
                    claimed
                };
                session.approvals.insert(approval_id, claimed);
                session.active_approval_id = None;
                session.record(
                    approval.request_id,
                    approval.trace_id,
                    SessionEventKind::ApprovalResolved {
                        approval_id,
                        decision: "approved".into(),
                        round: approval.round,
                        human_reason: None,
                    },
                    self.clock.as_ref(),
                );

                let mut continuation = approval.continuation.clone();
                continuation.approved_tool_index = Some(continuation.next_tool_index);
                continuation.approved_approval_id = Some(approval_id);

                let tools = self.plugins.tool_declarations();
                let mut trace =
                    ExecutionTrace::new(approval.trace_id, session_id, approval.request_id);
                trace.record(
                    now,
                    TraceEvent::ApprovalResolved {
                        approval_id,
                        decision: "approved".into(),
                        round: approval.round,
                    },
                );

                let outcome = self
                    .advance(
                        session,
                        trace,
                        session_id,
                        approval.request_id,
                        approval.trace_id,
                        tools,
                        continuation,
                    )
                    .await?;
                Ok(ApprovalResolution::Resumed(outcome))
            }
        }
    }

    /// The single turn state machine.
    ///
    /// It starts a new round when `continuation.tool_calls` is empty, and
    /// resumes mid-round when a pending approval froze the original tool-call
    /// batch. The original provider tool call is never regenerated.
    async fn advance(
        &self,
        mut session: Session,
        mut trace: ExecutionTrace,
        session_id: SessionId,
        request_id: RequestId,
        trace_id: TraceId,
        tools: Vec<NormalizedTool>,
        mut continuation: FrozenTurnContinuation,
    ) -> RuntimeResult<TurnOutcome> {
        let clock = self.clock.as_ref();

        loop {
            if continuation.tool_calls.is_empty() {
                if continuation.round > self.config.max_rounds {
                    let error = RuntimeError::RoundLimitExceeded {
                        limit: self.config.max_rounds,
                    };
                    session.record(
                        request_id,
                        trace_id,
                        SessionEventKind::ExecutionFailed {
                            phase: "round_limit".into(),
                            error: error.to_string(),
                        },
                        clock,
                    );
                    self.sessions.save(&session).await?;
                    return Err(error);
                }
                if let Err(error) = self
                    .authorize_completion(
                        &mut trace,
                        &session_id,
                        request_id,
                        trace_id,
                        &continuation.model,
                        &mut session,
                        continuation.round,
                    )
                    .await
                {
                    self.sessions.save(&session).await?;
                    return Err(error);
                }

                let provider_request =
                    NormalizedRequest::new(continuation.model.clone(), session.messages.clone());

                let routed = self
                    .providers
                    .complete_with_tools(&provider_request, &tools)
                    .await;

                let routed = match routed {
                    Ok(routed) => routed,
                    Err(e) => {
                        session.record(
                            request_id,
                            trace_id,
                            SessionEventKind::ProviderFailed {
                                error: e.to_string(),
                                round: continuation.round,
                            },
                            clock,
                        );
                        self.sessions.save(&session).await?;
                        return Err(e);
                    }
                };

                for (provider, error) in &routed.failed_attempts {
                    let at = Timestamp::from_clock(clock);
                    trace.record(
                        at,
                        TraceEvent::ProviderInvoked {
                            provider: provider.clone(),
                            model: continuation.model.clone(),
                            round: continuation.round,
                        },
                    );
                    trace.record(
                        at,
                        TraceEvent::ProviderFailed {
                            provider: provider.clone(),
                            round: continuation.round,
                            error: error.to_string(),
                            retryable: error.is_retryable(),
                        },
                    );
                }

                let response = routed.response;
                let served_by = routed.served_by;
                trace.record(
                    Timestamp::from_clock(clock),
                    TraceEvent::ProviderInvoked {
                        provider: served_by.clone(),
                        model: continuation.model.clone(),
                        round: continuation.round,
                    },
                );
                trace.record(
                    Timestamp::from_clock(clock),
                    TraceEvent::ProviderSucceeded {
                        provider: served_by.clone(),
                        round: continuation.round,
                        finish_reason: response.finish_reason,
                        usage: response.usage.clone(),
                    },
                );

                if response.tool_calls.is_empty() {
                    return self
                        .finish_turn(
                            session,
                            trace,
                            request_id,
                            served_by,
                            response,
                            continuation.round,
                        )
                        .await
                        .map(TurnOutcome::Completed);
                }

                // The assistant's tool-call message must reach the transcript
                // before the results, or the provider sees answers to questions
                // it never asked.
                session.append(
                    NormalizedMessage::assistant_with_tool_calls(
                        response.content.clone(),
                        response.tool_calls.clone(),
                    ),
                    clock,
                );

                continuation.tool_calls = response.tool_calls;
                continuation.next_tool_index = 0;
                continuation.approved_tool_index = None;
                continuation.approved_approval_id = None;
            }

            while continuation.next_tool_index < continuation.tool_calls.len() {
                let index = continuation.next_tool_index;
                let call = continuation.tool_calls[index].clone();
                let is_preapproved = continuation.approved_tool_index == Some(index);

                match self
                    .dispatch_one_tool(
                        &mut trace,
                        &mut session,
                        &session_id,
                        request_id,
                        trace_id,
                        &call,
                        continuation.round,
                        is_preapproved,
                    )
                    .await?
                {
                    ToolDispatch::Result(result) => {
                        session.append(result.into_message(), clock);
                    }
                    ToolDispatch::Pending {
                        capability_id,
                        tool_name,
                        tool_call,
                        effective_invocation,
                        governance_hook,
                        governance_reason,
                    } => {
                        let approval_id = ApprovalId::new();
                        let created_at = Timestamp::from_clock(clock);
                        let expires_at = Timestamp::from_epoch_millis(
                            created_at
                                .epoch_millis()
                                .saturating_add(self.config.approval_ttl_ms as i64),
                        )
                        .unwrap_or(created_at);
                        let fingerprint = operation_fingerprint_with_invocation(
                            "capability_dispatch",
                            &capability_id,
                            &tool_name,
                            &tool_call.id,
                            &tool_call.arguments,
                            effective_invocation.as_ref(),
                            session_id,
                            request_id,
                            continuation.round,
                        );
                        let frozen = FrozenTurnContinuation {
                            request_id,
                            trace_id,
                            model: continuation.model.clone(),
                            round: continuation.round,
                            tool_calls: continuation.tool_calls.clone(),
                            next_tool_index: index,
                            approved_tool_index: None,
                            approved_approval_id: None,
                        };
                        let pending = PendingApproval {
                            approval_id,
                            session_id,
                            request_id,
                            trace_id,
                            round: continuation.round,
                            capability_id,
                            tool_name,
                            tool_call,
                            effective_invocation,
                            governance_hook: governance_hook.clone(),
                            governance_reason,
                            operation_fingerprint: fingerprint,
                            created_at,
                            expires_at,
                            status: ApprovalStatus::Pending,
                            continuation: frozen,
                            human_reason: None,
                        };

                        session.record(
                            request_id,
                            trace_id,
                            SessionEventKind::ApprovalRequired {
                                hook: governance_hook,
                                action: "capability_dispatch".into(),
                                reason: pending.governance_reason.clone(),
                                round: continuation.round,
                                approval_id,
                            },
                            clock,
                        );
                        trace.record(
                            Timestamp::from_clock(clock),
                            TraceEvent::ApprovalRequested {
                                approval_id,
                                capability: pending.capability_id.clone(),
                                tool_call_id: pending.tool_call.id.clone(),
                                round: continuation.round,
                            },
                        );

                        session.approvals.insert(approval_id, pending.clone());
                        session.active_approval_id = Some(approval_id);
                        self.sessions.save(&session).await?;

                        return Ok(TurnOutcome::PendingApproval(PendingApprovalView::from(
                            &pending,
                        )));
                    }
                }

                if let Some(approved_approval_id) = continuation.approved_approval_id {
                    if let Some(approval) = session.approvals.get_mut(&approved_approval_id) {
                        approval.status = ApprovalStatus::Consumed;
                    }
                }

                continuation.next_tool_index += 1;
                continuation.approved_tool_index = None;
                continuation.approved_approval_id = None;
            }

            // Every tool call in this round has a result in the transcript.
            self.sessions.save(&session).await?;

            continuation.round += 1;
            continuation.tool_calls.clear();
            continuation.next_tool_index = 0;
            continuation.approved_tool_index = None;
            continuation.approved_approval_id = None;
        }
    }

    /// Ask governance whether this round's completion may proceed.
    async fn authorize_completion(
        &self,
        trace: &mut ExecutionTrace,
        session_id: &SessionId,
        request_id: RequestId,
        trace_id: TraceId,
        model: &str,
        session: &mut Session,
        round: u32,
    ) -> RuntimeResult<()> {
        let action = Action::Completion {
            model,
            message_count: session.len(),
        };
        let label = action.label();
        let verdict = self
            .governance
            .evaluate_verbose(&GovernanceRequest::new(
                action,
                *session_id,
                trace_id,
                round,
            ))
            .await;
        let hook = verdict.hook;
        let owner = verdict.owner;
        let decision = verdict.decision;

        trace.record(
            Timestamp::from_clock(self.clock.as_ref()),
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
            Decision::Allow => Ok(()),
            Decision::Deny { reason } => {
                session.record(
                    request_id,
                    trace_id,
                    SessionEventKind::GovernanceDenied {
                        hook: hook.clone(),
                        action: label.to_string(),
                        reason: reason.clone(),
                        round,
                    },
                    self.clock.as_ref(),
                );
                Err(RuntimeError::Denied { hook, reason })
            }
            Decision::RequireApproval { reason } => {
                session.record(
                    request_id,
                    trace_id,
                    SessionEventKind::ApprovalRequired {
                        hook: hook.clone(),
                        action: label.to_string(),
                        reason: reason.clone(),
                        round,
                        approval_id: ApprovalId::new(),
                    },
                    self.clock.as_ref(),
                );
                Err(RuntimeError::ApprovalRequired { hook, reason })
            }
        }
    }

    /// Resolve, authorize, and run one tool call.
    ///
    /// When `preapproved` is true the tool has already passed human approval
    /// and must be dispatched without a second governance evaluation.
    async fn dispatch_one_tool(
        &self,
        trace: &mut ExecutionTrace,
        session: &mut Session,
        session_id: &SessionId,
        request_id: RequestId,
        trace_id: TraceId,
        call: &ToolCall,
        round: u32,
        preapproved: bool,
    ) -> RuntimeResult<ToolDispatch> {
        let clock = self.clock.as_ref();

        let Some(tool) = self.plugins.tool_by_name(&call.name) else {
            let available = self
                .plugins
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
            return Ok(ToolDispatch::Result(
                ToolResult::permanent_error(&call.id, reason).with_name(&call.name),
            ));
        };

        let capability = tool.id().clone();

        if !preapproved {
            let action = Action::CapabilityDispatch {
                capability: &capability,
                arguments: &call.arguments,
            };
            let label = action.label();
            let verdict = self
                .governance
                .evaluate_verbose(&GovernanceRequest::new(
                    action,
                    *session_id,
                    trace_id,
                    round,
                ))
                .await;
            let hook = verdict.hook;
            let owner = verdict.owner;
            let decision = verdict.decision;

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
                    return Ok(ToolDispatch::Result(
                        ToolResult::permanent_error(
                            &call.id,
                            format!("refused by governance: {reason}"),
                        )
                        .with_name(&call.name),
                    ));
                }
                Decision::RequireApproval { reason } => {
                    return Ok(ToolDispatch::Pending {
                        capability_id: capability,
                        tool_name: call.name.clone(),
                        tool_call: call.clone(),
                        effective_invocation: tool.freeze_invocation(call),
                        governance_hook: hook,
                        governance_reason: reason,
                    });
                }
            }
        }

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::CapabilityDispatched {
                capability: capability.clone(),
                tool_call_id: call.id.clone(),
                round,
            },
        );

        let result = tool.invoke(call).await;

        if !result.is_ok() {
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
        }

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::CapabilityCompleted {
                capability,
                tool_call_id: call.id.clone(),
                succeeded: result.is_ok(),
                round,
            },
        );

        Ok(ToolDispatch::Result(result))
    }

    /// Record the assistant's answer, persist the session, and close the trace.
    async fn finish_turn(
        &self,
        mut session: Session,
        mut trace: ExecutionTrace,
        request_id: RequestId,
        served_by: CapabilityId,
        response: NormalizedResponse,
        rounds: u32,
    ) -> RuntimeResult<TurnResponse> {
        let clock = self.clock.as_ref();
        session.append(
            NormalizedMessage::assistant(response.content.clone()),
            clock,
        );
        session.record(
            request_id,
            trace.trace,
            SessionEventKind::TurnCompleted { rounds },
            clock,
        );
        self.sessions.save(&session).await?;

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::TurnCompleted { rounds },
        );

        Ok(TurnResponse {
            session: session.id,
            request: request_id,
            text: response.content,
            served_by,
            usage: response.usage,
            rounds,
            trace,
        })
    }
}
