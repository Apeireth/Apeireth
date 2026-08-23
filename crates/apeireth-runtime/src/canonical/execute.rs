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

use apeireth_core::kernel::{RequestId, SessionId, Timestamp, TraceId};
use apeireth_governance::{Action, Decision, GovernanceRequest};
use apeireth_protocol::canonical::{
    NormalizedMessage, NormalizedRequest, NormalizedResponse, NormalizedUsage, ToolCall, ToolResult,
};

use super::error::{RuntimeError, RuntimeResult};
use super::runtime::Runtime;
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
    pub served_by: apeireth_core::kernel::CapabilityId,
    /// Token accounting for the final response.
    pub usage: NormalizedUsage,
    /// How many provider round-trips the turn took.
    pub rounds: u32,
    /// Everything the runtime did, in order.
    pub trace: ExecutionTrace,
}

impl Runtime {
    /// Run one turn to completion.
    ///
    /// This is the runtime's entry point. CLI, gateway, desktop and tests all
    /// call it; none of them reimplements any part of what it does.
    pub async fn execute(&self, request: TurnRequest) -> RuntimeResult<TurnResponse> {
        let trace_id = TraceId::new();
        let request_id = RequestId::new();
        let mut trace = ExecutionTrace::new(trace_id, request.session, request_id);

        let model = request
            .model
            .clone()
            .or_else(|| self.config.default_model.clone())
            .ok_or_else(|| {
                RuntimeError::misconfigured(
                    "no model: the turn named none and the runtime has no default_model",
                )
            })?;

        let clock = self.clock.as_ref();
        let mut session = self.sessions.load_or_create(request.session).await?;

        if session.is_empty() {
            if let Some(system) = &request.system {
                session.append(NormalizedMessage::system(system.clone()), clock);
            }
        }
        session.append(NormalizedMessage::user(request.input.clone()), clock);

        let tools = self.plugins.tool_declarations();

        for round in 1..=self.config.max_rounds {
            self.authorize_completion(
                &mut trace,
                &request.session,
                trace_id,
                &model,
                &session,
                round,
            )
            .await?;

            let mut provider_request =
                NormalizedRequest::new(model.clone(), session.messages.clone());
            provider_request.tools = tools.clone();

            let routed = self.providers.complete(&provider_request).await;

            let routed = match routed {
                Ok(routed) => routed,
                Err(e) => {
                    // The turn is over, but the session keeps the user's message
                    // so a retry continues the conversation rather than losing it.
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
                        model: model.clone(),
                        round,
                    },
                );
                trace.record(
                    at,
                    TraceEvent::ProviderFailed {
                        provider: provider.clone(),
                        round,
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
                    model: model.clone(),
                    round,
                },
            );
            trace.record(
                Timestamp::from_clock(clock),
                TraceEvent::ProviderSucceeded {
                    provider: served_by.clone(),
                    round,
                    finish_reason: response.finish_reason,
                    usage: response.usage.clone(),
                },
            );

            if response.tool_calls.is_empty() {
                return self
                    .finish_turn(session, trace, request_id, served_by, response, round)
                    .await;
            }

            // The assistant's tool-call message must reach the transcript before
            // the results, or the provider sees answers to questions it never
            // asked.
            session.append(
                NormalizedMessage::assistant_with_tool_calls(
                    response.content.clone(),
                    response.tool_calls.clone(),
                ),
                clock,
            );

            for call in &response.tool_calls {
                let result = self
                    .dispatch_tool_call(&mut trace, &request.session, trace_id, call, round)
                    .await?;
                session.append(result.into_message(), clock);
            }

            self.sessions.save(&session).await?;
        }

        self.sessions.save(&session).await?;
        Err(RuntimeError::RoundLimitExceeded {
            limit: self.config.max_rounds,
        })
    }

    /// Ask governance whether this round's completion may proceed.
    async fn authorize_completion(
        &self,
        trace: &mut ExecutionTrace,
        session_id: &SessionId,
        trace_id: TraceId,
        model: &str,
        session: &super::session::Session,
        round: u32,
    ) -> RuntimeResult<()> {
        let action = Action::Completion {
            model,
            message_count: session.len(),
        };
        let label = action.label();
        let decision = self
            .governance
            .evaluate(&GovernanceRequest::new(
                action,
                *session_id,
                trace_id,
                round,
            ))
            .await;

        trace.record(
            Timestamp::from_clock(self.clock.as_ref()),
            TraceEvent::GovernanceEvaluated {
                hook: self.governance.name().to_string(),
                action: label.to_string(),
                decision: decision.to_string(),
                round,
            },
        );

        match decision {
            Decision::Allow => Ok(()),
            Decision::Deny { reason } => Err(RuntimeError::Denied {
                hook: self.governance.name().to_string(),
                reason,
            }),
            Decision::RequireApproval { reason } => Err(RuntimeError::ApprovalRequired {
                hook: self.governance.name().to_string(),
                reason,
            }),
        }
    }

    /// Resolve, authorize, and run one tool call.
    ///
    /// Returns a [`ToolResult`] in every ordinary case, including refusal and
    /// failure, because the model is the party that needs to know.
    async fn dispatch_tool_call(
        &self,
        trace: &mut ExecutionTrace,
        session_id: &SessionId,
        trace_id: TraceId,
        call: &ToolCall,
        round: u32,
    ) -> RuntimeResult<ToolResult> {
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
            return Ok(ToolResult::permanent_error(&call.id, reason).with_name(&call.name));
        };

        let capability = tool.id().clone();
        let action = Action::CapabilityDispatch {
            capability: &capability,
            arguments: &call.arguments,
        };
        let label = action.label();
        let decision = self
            .governance
            .evaluate(&GovernanceRequest::new(
                action,
                *session_id,
                trace_id,
                round,
            ))
            .await;

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::GovernanceEvaluated {
                hook: self.governance.name().to_string(),
                action: label.to_string(),
                decision: decision.to_string(),
                round,
            },
        );

        match decision {
            Decision::Allow => {}
            Decision::Deny { reason } => {
                // The model is told, and the turn continues. It may well have a
                // permitted way to answer.
                return Ok(ToolResult::permanent_error(
                    &call.id,
                    format!("refused by governance: {reason}"),
                )
                .with_name(&call.name));
            }
            Decision::RequireApproval { reason } => {
                // A human has to decide, so the turn genuinely cannot continue.
                return Err(RuntimeError::ApprovalRequired {
                    hook: self.governance.name().to_string(),
                    reason,
                });
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

        trace.record(
            Timestamp::from_clock(clock),
            TraceEvent::CapabilityCompleted {
                capability,
                tool_call_id: call.id.clone(),
                succeeded: result.is_ok(),
                round,
            },
        );

        Ok(result)
    }

    /// Record the assistant's answer, persist the session, and close the trace.
    async fn finish_turn(
        &self,
        mut session: super::session::Session,
        mut trace: ExecutionTrace,
        request_id: RequestId,
        served_by: apeireth_core::kernel::CapabilityId,
        response: NormalizedResponse,
        rounds: u32,
    ) -> RuntimeResult<TurnResponse> {
        let clock = self.clock.as_ref();
        session.append(
            NormalizedMessage::assistant(response.content.clone()),
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
