use super::*;
use apeireth_core::kernel::system_clock;
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{CommittedInputOrigin, SessionEvent};

fn append_committed_turn(
    session: &mut Session,
    input: &str,
    answer: Option<&str>,
) -> (RequestId, TraceId) {
    let clock = system_clock();
    let request = RequestId::new();
    let trace = TraceId::new();
    let start_seq = session.messages.len();
    session.append(NormalizedMessage::user(input), clock.as_ref());
    session.record(
        request,
        trace,
        SessionEventKind::InputCommitted {
            version: 1,
            origin: CommittedInputOrigin::DirectUser,
            start_seq,
            end_seq: start_seq + 1,
        },
        clock.as_ref(),
    );
    session.record(
        request,
        trace,
        SessionEventKind::TurnStarted,
        clock.as_ref(),
    );
    if let Some(answer) = answer {
        let assistant_seq = session.messages.len();
        session.append(NormalizedMessage::assistant(answer), clock.as_ref());
        session.record(
            request,
            trace,
            SessionEventKind::TurnResponseCommitted {
                version: 1,
                response_id: "response-identity".into(),
                assistant_seq,
            },
            clock.as_ref(),
        );
        session.record(
            request,
            trace,
            SessionEventKind::TurnCompleted { rounds: 1 },
            clock.as_ref(),
        );
    }
    (request, trace)
}

fn fixture() -> (Session, Selector) {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let (request, trace) = append_committed_turn(&mut session, "hello", Some("exact 😀\nanswer "));
    (
        session,
        Selector {
            digest: digest("hello"),
            request: Some(request),
            trace: Some(trace),
        },
    )
}

/// Keep both durable mirrors coherent so a malformed marker is the tested defect.
fn sync_facts(session: &mut Session) {
    session.events = session
        .log
        .iter()
        .filter_map(|entry| {
            Some(SessionEvent {
                at: entry.at,
                request: entry.request?,
                trace: entry.trace?,
                event: entry.event.clone(),
            })
        })
        .collect();
    let messages: Vec<_> = session
        .log
        .iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::MessageAppended { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(messages, session.messages);
}

#[test]
fn committed_results_and_history_read_exact_original_text_without_mutation() {
    let (mut session, selector) = fixture();
    append_committed_turn(&mut session, "hello", Some("second response"));
    let before = serde_json::to_vec(&session).unwrap();
    let result = project(&session, &selector).unwrap();
    assert_eq!(result.state, "completed");
    assert_eq!(result.text.as_deref(), Some("exact 😀\nanswer "));
    assert_eq!(result.usage, None);
    let first = super::page(&session, 0, 1).unwrap();
    assert_eq!(first.turns.len(), 1);
    assert_eq!(first.turns[0].request, selector.request.unwrap());
    assert_eq!(first.turns[0].text, Some("exact 😀\nanswer "));
    assert!(first.has_more);
    let second = super::page(&session, first.next_cursor, 1).unwrap();
    assert_eq!(second.turns[0].input, "hello");
    assert_eq!(second.turns[0].text, Some("second response"));
    assert_ne!(second.turns[0].request, first.turns[0].request);
    assert!(!second.has_more);
    assert_eq!(serde_json::to_vec(&session).unwrap(), before);
}

#[test]
fn legacy_and_committed_turns_can_coexist_without_widening_legacy_adjacency() {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let (old, _) = super::super::tests::append_turn(&mut session, "old input", Some("old answer"));
    let (new, _) = append_committed_turn(&mut session, "new input", Some("new answer"));
    let page = super::page(&session, 0, 8).unwrap();
    assert_eq!(page.turns.len(), 2);
    assert_eq!(page.turns[0].request, old);
    assert_eq!(page.turns[0].text, Some("old answer"));
    assert_eq!(page.turns[1].request, new);
    assert_eq!(page.turns[1].text, Some("new answer"));
}

#[test]
fn committed_input_without_a_completion_remains_unconfirmed() {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let (request, trace) = append_committed_turn(&mut session, "hello", None);
    session.append(
        NormalizedMessage::assistant("not committed"),
        system_clock().as_ref(),
    );
    let selector = Selector {
        digest: digest("hello"),
        request: Some(request),
        trace: Some(trace),
    };
    let result = project(&session, &selector).unwrap();
    assert_eq!(result.state, "unconfirmed");
    assert_eq!(result.text, None);
    let page = super::page(&session, 0, 8).unwrap();
    assert_eq!(page.turns[0].state, "unconfirmed");
    assert_eq!(page.turns[0].text, None);
}

#[test]
fn malformed_committed_input_cannot_downgrade_to_a_legacy_turn() {
    for mode in 0..11 {
        let (mut session, selector) = fixture();
        match mode {
            0 => {
                if let SessionEventKind::InputCommitted { version, .. } = &mut session.log[1].event
                {
                    *version = 2;
                }
            }
            1 => {
                if let SessionEventKind::InputCommitted { origin, .. } = &mut session.log[1].event {
                    *origin = CommittedInputOrigin::AgentTask;
                }
            }
            2 => {
                if let SessionEventKind::InputCommitted { end_seq, .. } = &mut session.log[1].event
                {
                    *end_seq = 0;
                }
            }
            3 => {
                if let SessionEventKind::InputCommitted { end_seq, .. } = &mut session.log[1].event
                {
                    *end_seq = 2;
                }
            }
            4 => {
                if let SessionEventKind::InputCommitted {
                    start_seq, end_seq, ..
                } = &mut session.log[1].event
                {
                    *start_seq = 1;
                    *end_seq = 2;
                }
            }
            5 => {
                if let SessionEventKind::InputCommitted { end_seq, .. } = &mut session.log[1].event
                {
                    *end_seq = usize::MAX;
                }
            }
            6 => session.log[1].request = Some(RequestId::new()),
            7 => session.log[1].trace = Some(TraceId::new()),
            8 => {
                session.log.remove(1);
            }
            9 => session.log.insert(1, session.log[1].clone()),
            _ => session.log.swap(1, 2),
        }
        sync_facts(&mut session);
        assert_eq!(
            project(&session, &selector).unwrap_err(),
            ProjectionError::Unverifiable,
            "input mode {mode}"
        );
        assert_eq!(
            super::page(&session, 0, 8).unwrap_err(),
            super::HistoryError::Unverifiable,
            "input mode {mode}"
        );
    }
}

#[test]
fn malformed_committed_response_cannot_confirm_any_assistant_text() {
    for mode in 0..10 {
        let (mut session, selector) = fixture();
        match mode {
            0 => {
                session.log.remove(4);
            }
            1 => session.log.insert(4, session.log[4].clone()),
            2 => {
                if let SessionEventKind::TurnResponseCommitted { version, .. } =
                    &mut session.log[4].event
                {
                    *version = 2;
                }
            }
            3 => {
                if let SessionEventKind::TurnResponseCommitted { response_id, .. } =
                    &mut session.log[4].event
                {
                    response_id.clear();
                }
            }
            4 => {
                if let SessionEventKind::TurnResponseCommitted { response_id, .. } =
                    &mut session.log[4].event
                {
                    *response_id = " \n".into();
                }
            }
            5 => {
                if let SessionEventKind::TurnResponseCommitted { assistant_seq, .. } =
                    &mut session.log[4].event
                {
                    *assistant_seq = 0;
                }
            }
            6 => {
                if let SessionEventKind::TurnResponseCommitted { assistant_seq, .. } =
                    &mut session.log[4].event
                {
                    *assistant_seq = 99;
                }
            }
            7 => session.log[4].request = Some(RequestId::new()),
            8 => session.log[4].trace = Some(TraceId::new()),
            _ => session.log.swap(4, 5),
        }
        sync_facts(&mut session);
        assert_eq!(
            project(&session, &selector).unwrap_err(),
            ProjectionError::Unverifiable,
            "response mode {mode}"
        );
        assert_eq!(
            super::page(&session, 0, 8).unwrap_err(),
            super::HistoryError::Unverifiable,
            "response mode {mode}"
        );
    }
}

#[test]
fn markers_cannot_borrow_another_turns_request_trace_or_original_message() {
    for mode in 0..4 {
        let (mut session, selector) = fixture();
        let (other_request, other_trace) =
            append_committed_turn(&mut session, "other input", Some("other answer"));
        match mode {
            0 => session.log[1].request = Some(other_request),
            1 => session.log[1].trace = Some(other_trace),
            2 => session.log[4].request = Some(other_request),
            _ => session.log[4].trace = Some(other_trace),
        }
        sync_facts(&mut session);
        assert_eq!(
            project(&session, &selector).unwrap_err(),
            ProjectionError::Unverifiable
        );
        assert_eq!(
            super::page(&session, 0, 8).unwrap_err(),
            super::HistoryError::Unverifiable
        );
    }
}

#[test]
fn another_turns_fact_inside_the_current_completion_window_stays_unverifiable() {
    let (mut session, selector) = fixture();
    let (other_request, other_trace) =
        append_committed_turn(&mut session, "other input", Some("other answer"));
    let mut foreign = session.log[1].clone();
    foreign.request = Some(other_request);
    foreign.trace = Some(other_trace);
    foreign.event = SessionEventKind::SessionEnded;
    session.log.insert(3, foreign);
    sync_facts(&mut session);
    // Every commit marker and both mirrors remain well formed. The unchanged
    // completion identity check must reject this unrelated correlated fact.
    assert_eq!(
        project(&session, &selector).unwrap_err(),
        ProjectionError::Unverifiable
    );
    assert_eq!(
        super::page(&session, 0, 8).unwrap_err(),
        super::HistoryError::Unverifiable
    );
}

#[test]
fn inherited_markers_and_partial_marker_boundaries_cannot_supply_a_local_turn() {
    let (mut session, selector) = fixture();
    session.inherited_event_count = Some(session.log.len());
    assert_eq!(
        project(&session, &selector).unwrap_err(),
        ProjectionError::NotFound
    );
    assert!(super::page(&session, 0, 8).unwrap().turns.is_empty());
    append_committed_turn(&mut session, "child input", Some("child answer"));
    let page = super::page(&session, 0, 8).unwrap();
    assert_eq!(page.turns.len(), 1);
    assert_eq!(page.turns[0].input, "child input");
    for prefix in [1, 2, 4, 5] {
        let (mut session, selector) = fixture();
        session.inherited_event_count = Some(prefix);
        assert_eq!(
            project(&session, &selector).unwrap_err(),
            ProjectionError::Unverifiable,
            "prefix {prefix}"
        );
        assert_eq!(
            super::page(&session, 0, 8).unwrap_err(),
            super::HistoryError::Unverifiable,
            "prefix {prefix}"
        );
    }
}
