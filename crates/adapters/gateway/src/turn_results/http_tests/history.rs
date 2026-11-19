use super::*;
use crate::turn_results::tests::append_turn;

fn history_uri(session: SessionId) -> String {
    format!("/v1/sessions/{session}/turns")
}

#[tokio::test]
async fn history_route_is_read_only_paged_and_protected_outside_the_token_gate() {
    let state = state().await;
    let (mut session, first, _) = fixture();
    for number in 2..=10 {
        append_turn(
            &mut session,
            &format!("input {number}"),
            Some(&format!("output {number}")),
        );
    }
    state.runtime.sessions().save(&session).await.unwrap();
    let before =
        serde_json::to_value(state.runtime.sessions().load(&session.id).await.unwrap()).unwrap();
    let target = history_uri(session.id);
    assert_eq!(
        get(&state, "GET", &target, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&state, "GET", &target, Some("wrong")).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&state, "POST", &target, Some(TOKEN)).await.0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    let (status, body) = get(&state, "GET", &target, Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["schema"], 1);
    assert_eq!(body["source"], "session_log");
    assert_eq!(body["session"], session.id.to_string());
    assert_eq!(body["revision"], session.revision);
    assert_eq!(body["turns"].as_array().unwrap().len(), 8);
    assert_eq!(body["turns"][0]["request"], first.to_string());
    assert_eq!(body["turns"][0]["input"], "hello");
    assert_eq!(body["next_cursor"], 29);
    assert_eq!(body["has_more"], true);
    let (status, tail) = get(
        &state,
        "GET",
        &format!("{target}?after=29&limit=16"),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tail}");
    assert_eq!(tail["turns"].as_array().unwrap().len(), 2);
    assert_eq!(tail["next_cursor"], 37);
    assert_eq!(tail["has_more"], false);
    let after =
        serde_json::to_value(state.runtime.sessions().load(&session.id).await.unwrap()).unwrap();
    assert_eq!(
        before, after,
        "history reads do not mutate the durable snapshot"
    );
}

#[tokio::test]
async fn history_rejects_malformed_selectors_and_reports_empty_broken_and_oversize() {
    let state = state().await;
    let (session, _, _) = fixture();
    state.runtime.sessions().save(&session).await.unwrap();
    let target = history_uri(session.id);
    for query in [
        "after=-1",
        "after=no",
        "after=18446744073709551616",
        "after=999",
        "limit=0",
        "limit=17",
        "limit=256",
        "limit=no",
        "limit=1&limit=2",
        "after=0&after=1",
        "unknown=1",
    ] {
        assert_eq!(
            get(&state, "GET", &format!("{target}?{query}"), Some(TOKEN))
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{query}"
        );
    }
    assert_eq!(
        get(&state, "GET", "/v1/sessions/not-a-uuid/turns", Some(TOKEN))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let absent = get(&state, "GET", &history_uri(SessionId::new()), Some(TOKEN)).await;
    assert_eq!(absent.0, StatusCode::NOT_FOUND);
    let empty = Session::new(SessionId::new(), system_clock().as_ref());
    state.runtime.sessions().save(&empty).await.unwrap();
    let (status, body) = get(&state, "GET", &history_uri(empty.id), Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["turns"], serde_json::json!([]));
    assert_eq!(body["next_cursor"], 0);
    assert_eq!(body["has_more"], false);
    let mut broken = session;
    broken.messages[0] = apeireth_protocol::canonical::NormalizedMessage::user("broken mirror");
    state.runtime.sessions().save(&broken).await.unwrap();
    assert_eq!(
        get(&state, "GET", &target, Some(TOKEN)).await.0,
        StatusCode::CONFLICT
    );
    let mut oversized = Session::new(SessionId::new(), system_clock().as_ref());
    append_turn(
        &mut oversized,
        "oversized",
        Some(&"x".repeat(4 * 1024 * 1024)),
    );
    state.runtime.sessions().save(&oversized).await.unwrap();
    assert_eq!(
        get(&state, "GET", &history_uri(oversized.id), Some(TOKEN))
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn history_reads_do_not_wait_for_an_executing_turn_write_permit() {
    let state = state().await;
    let (session, _, _) = fixture();
    state.runtime.sessions().save(&session).await.unwrap();
    let _permit = state.runtime.sessions().write_permit(session.id).await;
    let result = tokio::time::timeout(
        Duration::from_millis(250),
        get(&state, "GET", &history_uri(session.id), Some(TOKEN)),
    )
    .await;
    assert_eq!(
        result.expect("read must not queue behind execution").0,
        StatusCode::OK
    );
}
