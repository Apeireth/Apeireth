use super::*;
use apeireth_core::kernel::system_clock;
use apeireth_protocol::canonical::NormalizedMessage;

pub(super) const HELLO_HASH: &str =
    "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

pub(super) fn append_turn(
    session: &mut Session,
    input: &str,
    text: Option<&str>,
) -> (RequestId, TraceId) {
    let clock = system_clock();
    let request = RequestId::new();
    let trace = TraceId::new();
    session.append(NormalizedMessage::user(input), clock.as_ref());
    session.record(
        request,
        trace,
        SessionEventKind::TurnStarted,
        clock.as_ref(),
    );
    if let Some(text) = text {
        session.append(NormalizedMessage::assistant(text), clock.as_ref());
        session.record(
            request,
            trace,
            SessionEventKind::TurnCompleted { rounds: 1 },
            clock.as_ref(),
        );
    }
    (request, trace)
}

pub(super) fn fixture() -> (Session, RequestId, TraceId) {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let (request, trace) = append_turn(&mut session, "hello", Some("真实结果 😀\n保留末尾空格 "));
    (session, request, trace)
}

fn selector(request: Option<RequestId>, trace: Option<TraceId>) -> Selector {
    Selector {
        digest: HELLO_HASH.into(),
        request,
        trace,
    }
}

#[test]
fn completed_text_is_exact_and_historical_usage_stays_unknown() {
    assert_eq!(digest("hello"), HELLO_HASH);
    let (session, request, trace) = fixture();
    let result = project(&session, &selector(None, None)).unwrap();
    assert_eq!(result.schema, 1);
    assert_eq!(result.source, "session_log");
    assert_eq!(result.state, "completed");
    assert_eq!(result.session, session.id);
    assert_eq!(result.request, request);
    assert_eq!(result.trace_id, trace);
    assert_eq!(result.text.as_deref(), Some("真实结果 😀\n保留末尾空格 "));
    assert_eq!(result.usage, None);
    assert!(result.recorded_at.is_some());
}

#[test]
fn repeated_identical_inputs_require_the_original_request_not_latest() {
    let (mut session, first, trace) = fixture();
    let (second, second_trace) = append_turn(&mut session, "hello", Some("second"));
    assert_eq!(
        project(&session, &selector(None, None)).unwrap_err(),
        ProjectionError::Conflict
    );
    assert_eq!(
        project(&session, &selector(None, Some(second_trace))).unwrap_err(),
        ProjectionError::Conflict
    );
    assert_eq!(
        project(&session, &selector(Some(first), Some(trace)))
            .unwrap()
            .text
            .as_deref(),
        Some("真实结果 😀\n保留末尾空格 ")
    );
    assert_eq!(
        project(&session, &selector(Some(second), None))
            .unwrap()
            .text
            .as_deref(),
        Some("second")
    );
}

#[test]
fn known_identities_and_raw_input_cannot_be_rebound() {
    let (mut session, first, trace) = fixture();
    let (_, other_trace) = append_turn(&mut session, "other input", Some("other output"));
    assert_eq!(
        project(&session, &selector(Some(first), Some(other_trace))).unwrap_err(),
        ProjectionError::Conflict
    );
    assert_eq!(
        project(&session, &selector(Some(RequestId::new()), None)).unwrap_err(),
        ProjectionError::Conflict
    );
    let changed = Selector {
        digest: digest("hello\n"),
        request: Some(first),
        trace: Some(trace),
    };
    assert_eq!(
        project(&session, &changed).unwrap_err(),
        ProjectionError::Conflict
    );
    let absent = Selector {
        digest: digest("different"),
        request: None,
        trace: None,
    };
    assert_eq!(
        project(&session, &absent).unwrap_err(),
        ProjectionError::NotFound
    );
}

#[test]
fn a_start_without_completion_is_unconfirmed_even_with_a_later_assistant() {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let (request, trace) = append_turn(&mut session, "hello", None);
    session.append(
        NormalizedMessage::assistant("no completion fact"),
        system_clock().as_ref(),
    );
    let result = project(&session, &selector(None, None)).unwrap();
    assert_eq!(result.state, "unconfirmed");
    assert_eq!((result.request, result.trace_id), (request, trace));
    assert_eq!(result.text, None);
    assert_eq!(result.recorded_at, None);
}

#[test]
fn corrupt_message_and_fact_mirrors_cannot_confirm_a_result() {
    let (session, _, _) = fixture();
    let mut altered = session.clone();
    altered.messages[1] = NormalizedMessage::assistant("changed projection");
    assert_eq!(
        project(&altered, &selector(None, None)).unwrap_err(),
        ProjectionError::Unverifiable
    );
    let mut altered = session.clone();
    altered.events.clear();
    assert_eq!(
        project(&altered, &selector(None, None)).unwrap_err(),
        ProjectionError::Unverifiable
    );
    let mut altered = session.clone();
    if let SessionEventKind::MessageAppended { seq, .. } = &mut altered.log[2].event {
        *seq = 5;
    }
    assert_eq!(
        project(&altered, &selector(None, None)).unwrap_err(),
        ProjectionError::Unverifiable
    );
    let mut altered = session;
    altered.log[3].request = Some(RequestId::new());
    altered.events[1].request = altered.log[3].request.unwrap();
    assert_eq!(
        project(&altered, &selector(None, None)).unwrap_err(),
        ProjectionError::Unverifiable
    );
}

#[test]
fn completion_must_immediately_follow_plain_assistant_text() {
    let (mut session, request, trace) = fixture();
    session.log.remove(2);
    session.messages.pop();
    assert_eq!(
        project(&session, &selector(Some(request), Some(trace))).unwrap_err(),
        ProjectionError::Unverifiable
    );
    let (mut session, _, _) = fixture();
    session.append(
        NormalizedMessage::assistant("unrelated last message"),
        system_clock().as_ref(),
    );
    assert_eq!(
        project(&session, &selector(None, None))
            .unwrap()
            .text
            .as_deref(),
        Some("真实结果 😀\n保留末尾空格 ")
    );
}

#[test]
fn forked_inherited_results_are_not_child_executions() {
    let (mut child, _, _) = fixture();
    child.id = SessionId::new();
    child.inherited_event_count = Some(child.log.len());
    assert_eq!(
        project(&child, &selector(None, None)).unwrap_err(),
        ProjectionError::NotFound
    );
    let (own_request, own_trace) = append_turn(&mut child, "hello", Some("child own result"));
    assert_eq!(
        project(&child, &selector(Some(own_request), Some(own_trace)))
            .unwrap()
            .text
            .as_deref(),
        Some("child own result")
    );
    child.inherited_event_count = Some(child.log.len() + 1);
    assert_eq!(
        project(&child, &selector(None, None)).unwrap_err(),
        ProjectionError::Unverifiable
    );
}

#[test]
fn legacy_transcript_migration_does_not_supply_missing_turn_adjacency() {
    let (session, _, _) = fixture();
    let mut stored = serde_json::to_value(&session).unwrap();
    stored.as_object_mut().unwrap().remove("log");
    let old: Session = serde_json::from_value(stored).unwrap();
    assert!(!old.log.is_empty(), "legacy load rebuilds a log");
    assert_eq!(
        project(&old, &selector(None, None)).unwrap_err(),
        ProjectionError::NotFound
    );
}

#[test]
fn selector_rejects_noncanonical_digests_and_bad_known_ids() {
    for hash in ["", "123", &HELLO_HASH.to_uppercase(), &"g".repeat(64)] {
        assert!(Selector::try_from(TurnResultQuery {
            input_sha256: hash.into(),
            request: None,
            trace_id: None
        })
        .is_err());
    }
    for (request, trace_id) in [(Some("bad".into()), None), (None, Some("bad".into()))] {
        assert!(Selector::try_from(TurnResultQuery {
            input_sha256: HELLO_HASH.into(),
            request,
            trace_id
        })
        .is_err());
    }
}

#[test]
fn derived_masks_and_replacements_never_become_the_original_completion() {
    use apeireth_runtime::canonical::SurfaceOp;
    let (mut session, _, _) = fixture();
    session
        .mask_message(1, "provider context", system_clock().as_ref())
        .unwrap();
    assert_eq!(session.provider_view().len(), 1);
    assert_eq!(
        project(&session, &selector(None, None))
            .unwrap()
            .text
            .as_deref(),
        Some("真实结果 😀\n保留末尾空格 ")
    );
    let (mut session, _, _) = fixture();
    session
        .replace_surface(
            SurfaceOp::replace(0, 2),
            vec![
                NormalizedMessage::user("derived input"),
                NormalizedMessage::assistant("derived answer"),
            ],
            "context rewrite",
            system_clock().as_ref(),
        )
        .unwrap();
    assert_eq!(
        session.provider_view()[1],
        NormalizedMessage::assistant("derived answer")
    );
    assert_eq!(
        project(&session, &selector(None, None))
            .unwrap()
            .text
            .as_deref(),
        Some("真实结果 😀\n保留末尾空格 ")
    );
    let derived = Selector {
        digest: digest("derived input"),
        request: None,
        trace: None,
    };
    assert_eq!(
        project(&session, &derived).unwrap_err(),
        ProjectionError::NotFound
    );
}
