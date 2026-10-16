//! Checkpoint events survive the durable session store: the existing session
//! table stores the session as JSON, so the new event kinds ride along with no
//! schema change — and a reload replays the same derived view.

use apeireth_core::kernel::{system_clock, RequestId, SessionId, TraceId};
use apeireth_orchestration::compaction_checkpoint::SummarySections;
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::canonical::{Session, SessionEventKind, SessionStore};
use apeireth_runtime_assembly::SqliteSessionStore;

fn small_summary(keys: &str) -> String {
    SummarySections {
        keys: keys.into(),
        ..SummarySections::default()
    }
    .render()
}

#[tokio::test]
async fn checkpoint_events_round_trip_through_the_sqlite_session_store() {
    let store = SqliteSessionStore::in_memory().await.unwrap();
    let clock = system_clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    for text in ["first", "second", "third"] {
        session.append(NormalizedMessage::user(text), clock.as_ref());
    }

    let (request, trace) = (RequestId::new(), TraceId::new());
    let marker = "compaction/durable-1".to_string();
    session.record(
        request,
        trace,
        SessionEventKind::CompactionStarted {
            marker: marker.clone(),
        },
        clock.as_ref(),
    );
    session.record(
        request,
        trace,
        SessionEventKind::CompactionCheckpoint {
            start_seq: 0,
            end_seq: 2,
            summary: small_summary("kept"),
            marker: marker.clone(),
        },
        clock.as_ref(),
    );
    session.record(
        request,
        trace,
        SessionEventKind::CompactionClosed {
            marker: marker.clone(),
        },
        clock.as_ref(),
    );
    let view_before = session.provider_view();
    store.save(&session).await.unwrap();

    let reloaded = store.load(&session.id).await.unwrap().unwrap();
    assert_eq!(reloaded.messages.len(), 3, "转写原文完整落盘");
    assert_eq!(reloaded.events.len(), 3, "标记对 + 检查点完整落盘");
    assert_eq!(
        reloaded.compaction_log().len(),
        3,
        "落盘往返后折叠所需的日志完整"
    );
    assert_eq!(
        reloaded.provider_view(),
        view_before,
        "落盘往返后重放出同一份压缩视图"
    );
    assert_eq!(reloaded.provider_view().len(), 2, "摘要 + 尾部");
}
