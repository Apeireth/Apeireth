use super::*;

/// Hold the write permit from the contact check through execution, so a
/// concurrent rebind, deletion or settings write cannot change its identity.
pub(super) async fn run_turn_serialized(
    runtime: &Runtime,
    turn: TurnRequest,
    attribution: &SessionAttribution,
    on_delta: Option<Arc<dyn Fn(String) + Send + Sync>>,
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
    match on_delta {
        Some(sink) => runtime.execute_outcome_streaming(turn, sink).await,
        None => runtime.execute_outcome(turn).await,
    }
}
