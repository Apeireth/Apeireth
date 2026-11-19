//! Strict read-only compatibility with Runtime's versioned transcript markers.
use super::*;
use apeireth_runtime::CommittedInputOrigin;

struct ActiveTurn {
    request: Option<RequestId>,
    trace: Option<TraceId>,
    input_end: Option<usize>,
    response_seen: bool,
    completed: bool,
}

/// A marker is a contract, not an ignorable record between two messages.
pub(super) fn validate_markers(record: &Session, prefix: usize) -> Result<(), ProjectionError> {
    let mut active: Option<ActiveTurn> = None;
    for (index, entry) in record.log.iter().enumerate() {
        match &entry.event {
            SessionEventKind::InputCommitted {
                version,
                origin,
                start_seq,
                end_seq,
            } => {
                let previous = index.checked_sub(1).and_then(|i| record.log.get(i));
                let next = record.log.get(index + 1);
                if *version != 1
                    || *origin != CommittedInputOrigin::DirectUser
                    || start_seq.checked_add(1) != Some(*end_seq)
                    || *end_seq > record.messages.len()
                    || index >= prefix && index.saturating_sub(1) < prefix
                    || !previous.is_some_and(|previous| {
                        matches!(previous.event, SessionEventKind::MessageAppended { seq, .. }
                            if seq == *start_seq)
                            && plain_text(previous, MessageRole::User).is_some()
                    })
                    || !next.is_some_and(|next| {
                        matches!(next.event, SessionEventKind::TurnStarted)
                            && entry.request.is_some()
                            && entry.trace.is_some()
                            && next.request == entry.request
                            && next.trace == entry.trace
                    })
                {
                    return Err(ProjectionError::Unverifiable);
                }
            }
            SessionEventKind::TurnStarted => {
                let input_end = index
                    .checked_sub(1)
                    .and_then(|i| match record.log[i].event {
                        SessionEventKind::InputCommitted { end_seq, .. } => Some(end_seq),
                        _ => None,
                    });
                if input_end.is_some() && index >= prefix && index.saturating_sub(1) < prefix {
                    return Err(ProjectionError::Unverifiable);
                }
                active = Some(ActiveTurn {
                    request: entry.request,
                    trace: entry.trace,
                    input_end,
                    response_seen: false,
                    completed: false,
                });
            }
            SessionEventKind::TurnResponseCommitted {
                version,
                response_id,
                assistant_seq,
            } => {
                let turn = active.as_mut().ok_or(ProjectionError::Unverifiable)?;
                let previous = index.checked_sub(1).and_then(|i| record.log.get(i));
                let next = record.log.get(index + 1);
                if *version != 1
                    || response_id.trim().is_empty()
                    || response_id.chars().any(char::is_control)
                    || turn.input_end.is_none_or(|end| *assistant_seq < end)
                    || turn.response_seen
                    || turn.completed
                    || entry.request != turn.request
                    || entry.trace != turn.trace
                    || index >= prefix && index.saturating_sub(1) < prefix
                    || !previous.is_some_and(|previous| {
                        matches!(previous.event, SessionEventKind::MessageAppended { seq, .. }
                            if seq == *assistant_seq)
                            && plain_text(previous, MessageRole::Assistant).is_some()
                    })
                    || !next.is_some_and(|next| {
                        matches!(next.event, SessionEventKind::TurnCompleted { .. })
                            && next.request == entry.request
                            && next.trace == entry.trace
                    })
                {
                    return Err(ProjectionError::Unverifiable);
                }
                turn.response_seen = true;
            }
            SessionEventKind::TurnCompleted { .. } => {
                if let Some(turn) = active.as_mut().filter(|turn| turn.input_end.is_some()) {
                    if turn.completed
                        || !turn.response_seen
                        || entry.request != turn.request
                        || entry.trace != turn.trace
                        || index >= prefix && index.saturating_sub(1) < prefix
                        || !index.checked_sub(1).is_some_and(|i| {
                            matches!(
                                record.log[i].event,
                                SessionEventKind::TurnResponseCommitted { .. }
                            )
                        })
                    {
                        return Err(ProjectionError::Unverifiable);
                    }
                    turn.completed = true;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Mirrors are checked before this lookup. Never search for a nearby role.
pub(super) fn end_before_next_start(record: &Session, next_start: usize) -> usize {
    // Neither the next committed input nor its original user append belongs
    // to the preceding turn. validate_markers has established both adjacent
    // records and the marker's exact sequence and request/trace beforehand.
    match next_start.checked_sub(1) {
        Some(index)
            if matches!(
                record.log[index].event,
                SessionEventKind::InputCommitted { .. }
            ) =>
        {
            index - 1
        }
        _ => next_start,
    }
}

pub(super) fn input_text(record: &Session, start: usize) -> Option<&str> {
    let previous = start.checked_sub(1)?;
    let index = if matches!(
        record.log[previous].event,
        SessionEventKind::InputCommitted { .. }
    ) {
        previous.checked_sub(1)?
    } else {
        previous
    };
    plain_text(record.log.get(index)?, MessageRole::User)
}

pub(super) fn response_text(record: &Session, completed: usize) -> Option<&str> {
    let previous = completed.checked_sub(1)?;
    let index = if matches!(
        record.log[previous].event,
        SessionEventKind::TurnResponseCommitted { .. }
    ) {
        previous.checked_sub(1)?
    } else {
        previous
    };
    plain_text(record.log.get(index)?, MessageRole::Assistant)
}
