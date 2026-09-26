//! The session event log as the sole authority, end to end against the real
//! `Session`: durable round trips, the legacy migration, and session forking.
//!
//! The rules under test:
//!
//! - the log is append-only; a mask or a surface change never deletes an
//!   original — the transcript and the log keep it forever;
//! - what a provider sees is derived by one pure fold, so a persisted session
//!   replays to exactly the view it had before the write;
//! - a fork copies the exact prefix up to its cut, closes the tool calls the
//!   prefix left open with a placeholder result, and leaves children evidence
//!   on the parent;
//! - a session that uses none of this is byte-for-byte the session it was.

use std::sync::Arc;

use apeireth_core::kernel::{Clock, RequestId, SessionId, Timestamp, TraceId, VirtualClock};
use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedMessage, ToolCall};
use apeireth_runtime::canonical::{
    open_tool_calls, ForkRecord, LogEntry, Session, SessionEventKind, SurfaceOp,
};

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn text(message: &NormalizedMessage) -> String {
    ContentPart::join_text(&message.content)
}

fn texts(messages: &[NormalizedMessage]) -> Vec<String> {
    messages.iter().map(text).collect()
}

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "shell".into(),
        arguments: serde_json::json!({}),
    }
}

#[test]
fn a_persisted_session_replays_the_same_log_and_view() {
    let clock = clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.append(NormalizedMessage::user("hello"), clock.as_ref());
    session.append(NormalizedMessage::user("secret"), clock.as_ref());
    session.append(NormalizedMessage::assistant("hi"), clock.as_ref());
    session
        .mask_message(1, "sensitive", clock.as_ref())
        .unwrap();
    session
        .replace_surface(
            SurfaceOp::replace(2, 3),
            vec![NormalizedMessage::assistant("hey")],
            "rewrite",
            clock.as_ref(),
        )
        .unwrap();

    let (request, trace) = (RequestId::new(), TraceId::new());
    session.record(
        request,
        trace,
        SessionEventKind::TurnStarted,
        clock.as_ref(),
    );
    let marker = "compaction/rt-1".to_string();
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
            end_seq: 1,
            summary: "opening".into(),
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

    // 摘要顶掉 [0,1), 遮蔽藏起 1, 改写顶掉 [2,3): 原文三段全部留档。
    assert_eq!(
        texts(&session.provider_view()),
        vec!["opening", "hey"],
        "{:?}",
        session.surface_view().rejected
    );

    let json = serde_json::to_string(&session).unwrap();
    let reloaded: Session = serde_json::from_str(&json).unwrap();

    assert_eq!(reloaded.log, session.log, "权威日志逐条落盘往返一致");
    assert_eq!(reloaded.messages, session.messages, "转写原文完整往返");
    assert_eq!(reloaded.events, session.events, "事实流完整往返");
    assert_eq!(reloaded.surface_view(), session.surface_view());
    assert_eq!(reloaded.provider_view(), session.provider_view());
    assert!(
        reloaded.messages.iter().any(|m| text(m) == "secret"),
        "被遮蔽的原文在往返后仍然可查"
    );
    assert!(
        reloaded.messages.iter().any(|m| text(m) == "hello"),
        "被替换的原文在往返后仍然可查"
    );
}

#[test]
fn a_session_persisted_before_the_log_rebuilds_it_on_load() {
    // 旧落盘形状没有 log 字段: 加载边界从转写 + 事实重建一次, 此后日志即完整权威。
    let clock = clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.append(NormalizedMessage::user("first"), clock.as_ref());
    session.append(NormalizedMessage::user("second"), clock.as_ref());
    session.append(NormalizedMessage::assistant("done"), clock.as_ref());

    let (request, trace) = (RequestId::new(), TraceId::new());
    let marker = "compaction/rt-2".to_string();
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
            summary: "opening".into(),
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

    let mut value = serde_json::to_value(&session).unwrap();
    value.as_object_mut().unwrap().remove("log");
    let migrated: Session = serde_json::from_value(value).unwrap();

    assert_eq!(
        migrated.log.len(),
        session.messages.len() + session.events.len(),
        "重建的日志覆盖全部追加与事实"
    );
    assert_eq!(
        migrated.provider_view(),
        session.provider_view(),
        "重建后的日志重放出迁移前的同一份视图"
    );
}

#[test]
fn forking_inherits_exactly_the_prefix_and_nothing_after_the_cut() {
    let clock = clock();
    let mut parent = Session::new(SessionId::new(), clock.as_ref());
    parent.append(NormalizedMessage::user("one"), clock.as_ref());
    let (request, trace) = (RequestId::new(), TraceId::new());
    parent.record(
        request,
        trace,
        SessionEventKind::TurnStarted,
        clock.as_ref(),
    );
    parent.append(NormalizedMessage::user("two"), clock.as_ref());
    parent.append(NormalizedMessage::assistant("three"), clock.as_ref());
    assert_eq!(parent.log.len(), 4);

    let cut = 3;
    let prefix: Vec<LogEntry> = parent.log[..cut].to_vec();
    let child = parent.fork_session(cut, clock.as_ref()).unwrap();

    assert_eq!(child.inherited_event_count, Some(cut), "切点记录精确");
    assert_eq!(&child.log[..cut], &prefix[..], "前缀逐条精确复制");
    assert_eq!(child.log.len(), cut, "切点之后一条不带");
    assert_eq!(
        texts(&child.messages),
        vec!["one", "two"],
        "转写只有前缀里的追加"
    );
    assert_eq!(child.events.len(), 1, "事实只有前缀里带来源的记录");
    assert_eq!(child.events[0].request, request);
    assert_eq!(child.events[0].trace, trace);

    assert_eq!(parent.log.len(), cut + 2, "父会话原有 4 条 + 分叉证据 1 条");
    assert_eq!(parent.children()[0].prefix_end_seq, cut);
}

#[test]
fn forking_closes_open_tool_calls_with_a_placeholder_result() {
    let clock = clock();
    let mut parent = Session::new(SessionId::new(), clock.as_ref());
    parent.append(NormalizedMessage::user("run"), clock.as_ref());
    parent.append(
        NormalizedMessage::assistant_with_tool_calls("", vec![call("call-1")]),
        clock.as_ref(),
    );
    assert_eq!(
        open_tool_calls(&parent.messages),
        vec!["call-1".to_string()],
        "前置条件: 调用开放中"
    );

    let child = parent
        .fork_session(parent.log.len(), clock.as_ref())
        .unwrap();

    assert_eq!(child.inherited_event_count, Some(2));
    assert_eq!(child.messages.len(), 3, "两条前缀 + 一条占位结果");
    let placeholder = child.messages.last().unwrap();
    assert_eq!(placeholder.role, MessageRole::Tool);
    assert_eq!(placeholder.tool_call_id.as_deref(), Some("call-1"));
    assert!(
        text(placeholder).contains("已分叉"),
        "占位结果说明调用止于分叉, 不编造结局: {}",
        text(placeholder)
    );
    assert!(
        open_tool_calls(&child.messages).is_empty(),
        "占位结果把开放调用收口"
    );
    assert!(
        matches!(
            &child.log.last().unwrap().event,
            SessionEventKind::MessageAppended { seq: 2, .. }
        ),
        "占位结果以普通追加记录进子会话日志"
    );
}

#[test]
fn forked_sessions_then_evolve_independently() {
    let clock = clock();
    let mut parent = Session::new(SessionId::new(), clock.as_ref());
    parent.append(NormalizedMessage::user("one"), clock.as_ref());
    parent.append(NormalizedMessage::user("two"), clock.as_ref());

    let mut child = parent.fork_session(1, clock.as_ref()).unwrap();
    child.append(NormalizedMessage::user("only in child"), clock.as_ref());
    parent.append(NormalizedMessage::user("only in parent"), clock.as_ref());
    parent.mask_message(0, "hidden", clock.as_ref()).unwrap();

    assert_eq!(
        texts(&child.provider_view()),
        vec!["one", "only in child"],
        "父会话的遮蔽不波及子会话"
    );
    assert_eq!(
        texts(&parent.provider_view()),
        vec!["two", "only in parent"],
        "子会话的追加不波及父会话"
    );
    assert_eq!(child.log.len(), 2, "子会话日志: 前缀 1 条 + 自有追加 1 条");
    assert_eq!(
        parent.log.len(),
        5,
        "父会话日志: 追加 2 条 + 分叉证据 + 追加 + 遮蔽"
    );
}

#[test]
fn the_parent_keeps_children_evidence_for_every_fork() {
    let clock = clock();
    let mut parent = Session::new(SessionId::new(), clock.as_ref());
    parent.append(NormalizedMessage::user("one"), clock.as_ref());
    parent.append(NormalizedMessage::user("two"), clock.as_ref());

    let child_a = parent.fork_session(1, clock.as_ref()).unwrap();
    let child_b = parent.fork_session(2, clock.as_ref()).unwrap();

    let children = parent.children();
    assert_eq!(
        children,
        vec![
            ForkRecord {
                child_id: child_a.id,
                prefix_end_seq: 1,
            },
            ForkRecord {
                child_id: child_b.id,
                prefix_end_seq: 2,
            },
        ],
        "分叉顺序 + 各自切点都在父会话留证"
    );

    let json = serde_json::to_string(&parent).unwrap();
    let reloaded: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(
        reloaded.children(),
        children,
        "children 证据随日志落盘往返一致"
    );
}

#[test]
fn a_plain_session_is_untouched_by_the_authority_layer() {
    let clock = clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.append(NormalizedMessage::user("a"), clock.as_ref());
    session.append(NormalizedMessage::assistant("b"), clock.as_ref());

    assert!(session.events.is_empty(), "追加不写事实流");
    assert_eq!(session.log.len(), 2, "追加各记一条日志");
    assert_eq!(session.provider_view(), session.messages);

    let (request, trace) = (RequestId::new(), TraceId::new());
    session.record(
        request,
        trace,
        SessionEventKind::TurnCompleted { rounds: 1 },
        clock.as_ref(),
    );

    assert_eq!(session.messages.len(), 2, "事实记录不进转写");
    assert_eq!(session.events.len(), 1);
    assert_eq!(session.events[0].request, request);
    assert_eq!(session.events[0].trace, trace);
    assert_eq!(
        session.provider_view(),
        session.messages,
        "零表层操作时派生视图就是转写原文"
    );
}
