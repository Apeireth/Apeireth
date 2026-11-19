use super::*;
use crate::turn_results::tests::{append_turn, fixture, HELLO_HASH};
use apeireth_core::kernel::system_clock;
use apeireth_protocol::canonical::NormalizedMessage;

#[test]
fn pages_use_immutable_start_cursors_and_keep_identical_inputs_distinct() {
    let (mut session, first, _) = fixture();
    let (second, _) = append_turn(&mut session, "hello", Some("second"));
    append_turn(&mut session, "third", Some("last"));
    let first_page = page(&session, 0, 1).unwrap();
    assert_eq!(first_page.turns.len(), 1);
    assert_eq!(first_page.turns[0].request, first);
    assert_eq!(first_page.turns[0].input_sha256, HELLO_HASH);
    assert_eq!(first_page.turns[0].input, "hello");
    assert_eq!(first_page.turns[0].text, Some("真实结果 😀\n保留末尾空格 "));
    assert_eq!(first_page.next_cursor, 1);
    assert!(first_page.has_more);
    let next = page(&session, first_page.next_cursor, 2).unwrap();
    assert_eq!(next.turns.len(), 2);
    assert_eq!(next.turns[0].cursor, 5);
    assert_eq!(next.turns[0].request, second);
    assert_eq!(next.turns[1].cursor, 9);
    assert_eq!(next.next_cursor, 9);
    assert!(!next.has_more);
    let empty = page(&session, next.next_cursor, 8).unwrap();
    assert!(empty.turns.is_empty());
    assert_eq!(empty.next_cursor, next.next_cursor);
    assert!(!empty.has_more);
    assert_eq!(empty.revision, session.revision);
}

#[test]
fn unconfirmed_turn_refreshes_at_the_same_cursor_without_fabricated_text() {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let (request, trace) = append_turn(&mut session, "unfinished", None);
    session.append(
        NormalizedMessage::assistant("not yet final"),
        system_clock().as_ref(),
    );
    let pending = page(&session, 0, 8).unwrap();
    assert_eq!(pending.turns[0].state, "unconfirmed");
    assert_eq!(pending.turns[0].text, None);
    assert_eq!(pending.turns[0].recorded_at, None);
    assert_eq!(pending.turns[0].usage, None);
    let cursor = pending.next_cursor;
    session.record(
        request,
        trace,
        SessionEventKind::TurnCompleted { rounds: 1 },
        system_clock().as_ref(),
    );
    let refreshed = page(&session, 0, 8).unwrap();
    assert_eq!(refreshed.next_cursor, cursor);
    assert_eq!(refreshed.turns[0].state, "completed");
    assert_eq!(refreshed.turns[0].text, Some("not yet final"));
    assert!(page(&session, cursor, 8).unwrap().turns.is_empty());
}

#[test]
fn empty_sessions_and_inherited_fork_prefixes_do_not_invent_child_turns() {
    let empty = Session::new(SessionId::new(), system_clock().as_ref());
    assert!(page(&empty, 0, 8).unwrap().turns.is_empty());
    assert_eq!(page(&empty, 1, 8).unwrap_err(), HistoryError::InvalidCursor);
    let (mut child, _, _) = fixture();
    child.id = SessionId::new();
    child.inherited_event_count = Some(child.log.len());
    assert!(page(&child, 0, 8).unwrap().turns.is_empty());
    append_turn(&mut child, "child input", Some("child result"));
    let own = page(&child, 0, 8).unwrap();
    assert_eq!(own.turns.len(), 1);
    assert_eq!(own.turns[0].input, "child input");
    assert_eq!(own.turns[0].cursor, 5);
    child.inherited_event_count = Some(child.log.len() + 1);
    assert_eq!(page(&child, 0, 8).unwrap_err(), HistoryError::Unverifiable);
}

#[test]
fn broken_mirrors_identity_and_unsupported_local_input_are_explicit_conflicts() {
    let (mut broken, _, _) = fixture();
    broken.messages[0] = NormalizedMessage::user("altered");
    assert_eq!(page(&broken, 0, 8).unwrap_err(), HistoryError::Unverifiable);
    let (mut unsupported, _, _) = fixture();
    if let SessionEventKind::MessageAppended { message, .. } = &mut unsupported.log[0].event {
        message.role = MessageRole::System;
        unsupported.messages[0].role = MessageRole::System;
    }
    assert_eq!(
        page(&unsupported, 0, 8).unwrap_err(),
        HistoryError::Unverifiable
    );
    let (mut ambiguous, request, trace) = fixture();
    ambiguous.record(
        request,
        trace,
        SessionEventKind::TurnCompleted { rounds: 1 },
        system_clock().as_ref(),
    );
    assert_eq!(
        page(&ambiguous, 0, 8).unwrap_err(),
        HistoryError::Unverifiable
    );
    let (mut crossed, _, _) = fixture();
    crossed.log[3].trace = Some(TraceId::new());
    crossed.events[1].trace = crossed.log[3].trace.unwrap();
    assert_eq!(
        page(&crossed, 0, 8).unwrap_err(),
        HistoryError::Unverifiable
    );
}

#[test]
fn pages_bound_encoded_bytes_without_truncating_or_skipping_a_turn() {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    let large = "x".repeat(MAX_PAGE_BYTES / 2 + 20);
    append_turn(&mut session, "first", Some(&large));
    append_turn(&mut session, "second", Some(&large));
    let one = page(&session, 0, 8).unwrap();
    assert_eq!(one.turns.len(), 1);
    assert_eq!(one.turns[0].text.unwrap().len(), large.len());
    assert!(one.has_more);
    assert!(serde_json::to_vec(&one).unwrap().len() <= MAX_PAGE_BYTES);
    let two = page(&session, one.next_cursor, 8).unwrap();
    assert_eq!(two.turns.len(), 1);
    assert_eq!(two.turns[0].input, "second");
    assert_eq!(two.turns[0].text, Some(large.as_str()));
    assert!(!two.has_more);
    let mut escaped = Session::new(SessionId::new(), system_clock().as_ref());
    append_turn(
        &mut escaped,
        "small",
        Some(&"\"".repeat(MAX_PAGE_BYTES / 2)),
    );
    assert_eq!(page(&escaped, 0, 8).unwrap_err(), HistoryError::TooLarge);
}

#[test]
fn history_exposes_only_original_user_and_final_assistant_text() {
    let mut session = Session::new(SessionId::new(), system_clock().as_ref());
    session.append(
        NormalizedMessage::system("private system instructions"),
        system_clock().as_ref(),
    );
    append_turn(&mut session, "original user", Some("original final"));
    session.append(
        NormalizedMessage::assistant("unrelated later message"),
        system_clock().as_ref(),
    );
    let body = serde_json::to_string(&page(&session, 0, 8).unwrap()).unwrap();
    assert!(body.contains("original user"));
    assert!(body.contains("original final"));
    assert!(!body.contains("private system instructions"));
    assert!(!body.contains("unrelated later message"));
}
