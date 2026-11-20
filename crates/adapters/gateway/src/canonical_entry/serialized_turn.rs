use super::*;
use apeireth_guard::{IntentInput, IntentInterpreter, RuleIntentInterpreter};
use apeireth_runtime::canonical::{OperationAdmission, OperationRequest, TurnSecurityContext};

pub(super) enum TurnProgress {
    Callback(Arc<dyn Fn(String) + Send + Sync>),
    Native(native_stream::Progress),
}

/// Admit through the native operation gate before taking the session write permit.
/// Replays use the stored outcome and never open another progress stream.
pub(super) async fn execute_chat_attributed_operation(
    runtime: &Runtime,
    request: CanonicalChatRequest,
    attribution: SessionAttribution,
    operation: Option<OperationRequest>,
    progress: Option<TurnProgress>,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
    validate_chat_input(&request.input)?;
    let expected_contact = request.expected_contact_id;
    let session = request.effective_session()?;
    let input = request.input;
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: session.to_string(),
        trace_id: String::new(),
        user_request: input.clone(),
        created_at_ms: 0,
    });
    let context = TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent);
    let mut turn = TurnRequest::new(session, input).with_security_context(context);
    if let Some(model) = request.model {
        turn = turn.with_model(model);
    }
    if let Some(system) = request.system {
        turn = turn.with_system(system);
    }

    let outcome = match operation {
        None => {
            run_turn_serialized(runtime, turn, &attribution, progress, expected_contact).await?
        }
        Some(operation) => match runtime.begin_operation(session, operation) {
            OperationAdmission::Replay(outcome) => {
                if let Some(contact) = expected_contact {
                    let _permit = runtime.sessions().write_permit(session).await;
                    contact_session::verify_contact(runtime.sessions(), session, contact).await?;
                }
                return Ok(turn_outcome_to_chat(outcome));
            }
            OperationAdmission::Conflict { operation, reason } => {
                return Err(RuntimeError::OperationConflict { operation, reason }.into());
            }
            OperationAdmission::InProgress { operation } => {
                return Err(RuntimeError::OperationInProgress { operation }.into());
            }
            OperationAdmission::Fresh(guard) => {
                let outcome =
                    run_turn_serialized(runtime, turn, &attribution, progress, expected_contact)
                        .await?;
                guard.complete(outcome.clone());
                outcome
            }
        },
    };
    Ok(turn_outcome_to_chat(outcome))
}

/// Hold the write permit from the contact check through execution, so a
/// concurrent rebind, deletion or settings write cannot change its identity.
pub(super) async fn run_turn_serialized(
    runtime: &Runtime,
    turn: TurnRequest,
    attribution: &SessionAttribution,
    progress: Option<TurnProgress>,
    expected_contact: Option<ContactId>,
) -> Result<TurnOutcome, RuntimeError> {
    let permit = runtime.sessions().write_permit(turn.session).await;
    record_session_client(
        attribution.audit.as_deref(),
        turn.session,
        &attribution.client,
        "turn",
        Some(permit.admission()),
    );
    let turn = if let Some(contact) = expected_contact {
        contact_session::verify_contact(runtime.sessions(), turn.session, contact).await?;
        turn.without_tools()
    } else {
        turn
    };
    match progress {
        Some(TurnProgress::Callback(sink)) => runtime.execute_outcome_streaming(turn, sink).await,
        Some(TurnProgress::Native(progress)) => {
            let (request, sink) = progress.start(turn.session);
            let turn = turn.with_request(request).without_stream_replay();
            runtime.execute_outcome_streaming(turn, sink).await
        }
        None => runtime.execute_outcome(turn).await,
    }
}
