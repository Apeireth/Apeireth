//! C1 规范流的权威注册表 (后端合同 C 批 · C1 段 · 裁决 16/17)。
//!
//! 钉的是权威侧的**不变量**: 序号从 1 单调递增、权威回填帧身份、终帧只发一次、
//! 环形缓冲**有界**、慢订阅者**丢帧而不阻塞发布方**、落后于窗口 ⇒ 显式
//! `ResyncRequired` 事实 (不是错误)。

use apeireth_protocol::canonical_stream::{C1FinalFrame, C1StreamState, C1_FINAL_EVENT};
use apeireth_runtime::canonical::canonical_stream::{
    global_registry, ReplayOutcome, StreamError, StreamRegistry, STREAM_BUFFER_CAPACITY,
    STREAM_SUBSCRIBER_QUEUE_CAPACITY,
};

#[test]
fn sequences_start_at_one_and_increase_monotonically() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-1", "request-1")
        .expect("register");

    assert_eq!(
        registry.publish_delta(&handle, None, "a").expect("delta 1"),
        1
    );
    assert_eq!(
        registry.publish_delta(&handle, None, "b").expect("delta 2"),
        2
    );
    assert_eq!(
        registry
            .publish_delta(&handle, Some("trace-1"), "c")
            .expect("delta 3"),
        3
    );
}

#[test]
fn the_final_frame_identity_and_sequence_are_filled_in_by_the_authoritative_side() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-2", "request-2")
        .expect("register");
    registry.publish_delta(&handle, None, "a").expect("delta");

    let mut frame = C1FinalFrame::new(
        "wrong-session",
        "wrong-request",
        None,
        C1StreamState::Completed,
    );
    registry.publish_final(&handle, &mut frame).expect("final");

    assert_eq!(frame.event, C1_FINAL_EVENT);
    assert_eq!(frame.session_id, "session-2");
    assert_eq!(frame.request_id, "request-2");
    assert_eq!(frame.sequence, 2, "终帧序号由权威侧接着分配");
}

#[test]
fn a_final_frame_can_only_be_published_once() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-3", "request-3")
        .expect("register");

    let mut first = C1FinalFrame::new("session-3", "request-3", None, C1StreamState::Completed);
    registry
        .publish_final(&handle, &mut first)
        .expect("first final");

    let mut second = C1FinalFrame::new("session-3", "request-3", None, C1StreamState::Failed);
    let error = registry
        .publish_final(&handle, &mut second)
        .expect_err("第二次发布终帧必须被拒");
    assert!(
        matches!(error, StreamError::FinalAlreadyPublished { .. }),
        "实际错误: {error:?}"
    );
    assert_eq!(second.sequence, 0, "被拒的终帧不得被回填序号");
}

#[test]
fn a_finalized_stream_refuses_further_deltas() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-4", "request-4")
        .expect("register");

    let mut frame = C1FinalFrame::new("session-4", "request-4", None, C1StreamState::Completed);
    registry.publish_final(&handle, &mut frame).expect("final");

    let error = registry
        .publish_delta(&handle, None, "late")
        .expect_err("终态之后不得再发增量");
    assert!(
        matches!(error, StreamError::StreamFinalized { .. }),
        "实际错误: {error:?}"
    );
}

#[test]
fn replay_returns_only_the_frames_after_the_asked_sequence() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-5", "request-5")
        .expect("register");
    for text in ["a", "b", "c"] {
        registry.publish_delta(&handle, None, text).expect("delta");
    }

    let outcome = registry.replay(&handle, 1).expect("replay");

    match outcome {
        ReplayOutcome::Window(frames) => {
            assert_eq!(frames.len(), 2, "只回放序号严格大于 1 的帧");
            assert_eq!(frames[0].sequence(), 2);
            assert_eq!(frames[1].sequence(), 3);
            assert!(!frames[0].is_final());
        }
        other => panic!("期望窗口, 实际 {other:?}"),
    }
}

#[test]
fn a_client_behind_the_window_gets_an_explicit_resync_fact_instead_of_an_error() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-6", "request-6")
        .expect("register");
    for index in 0..(STREAM_BUFFER_CAPACITY + 10) {
        registry
            .publish_delta(&handle, None, &index.to_string())
            .expect("delta");
    }

    // 落后于窗口 ⇒ 显式事实 (Ok, 不是 Err)。
    let outcome = registry.replay(&handle, 0).expect("replay 不得失败");
    match outcome {
        ReplayOutcome::ResyncRequired {
            oldest_available_sequence,
        } => assert_eq!(
            oldest_available_sequence, 11,
            "窗口保留最后 256 帧 ⇒ 最早可用序号是 11"
        ),
        other => panic!("期望 resync_required, 实际 {other:?}"),
    }

    // 锚点落在窗口内 ⇒ 正常窗口, 且第一帧就是最早可用帧。
    let outcome = registry.replay(&handle, 10).expect("replay");
    match outcome {
        ReplayOutcome::Window(frames) => {
            assert_eq!(frames.len(), STREAM_BUFFER_CAPACITY);
            assert_eq!(frames[0].sequence(), 11);
            assert_eq!(frames[frames.len() - 1].sequence(), 266);
        }
        other => panic!("期望窗口, 实际 {other:?}"),
    }
}

#[test]
fn the_ring_buffer_is_bounded() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-7", "request-7")
        .expect("register");
    for index in 0..(STREAM_BUFFER_CAPACITY + 10) {
        registry
            .publish_delta(&handle, None, &index.to_string())
            .expect("delta");
    }

    let snapshot = registry.stream_snapshot(&handle).expect("snapshot");
    assert_eq!(
        snapshot.buffered_frames, STREAM_BUFFER_CAPACITY,
        "权威缓冲必须是有界的"
    );
    assert_eq!(snapshot.oldest_available_sequence, 11);
    assert_eq!(snapshot.next_sequence, 267);
    assert!(!snapshot.finalized);
}

#[tokio::test]
async fn a_slow_subscriber_loses_frames_instead_of_blocking_the_publisher() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-8", "request-8")
        .expect("register");
    let mut subscription = registry.subscribe(&handle).expect("subscribe");

    // 发布方一次发满队列 + 10 帧 ⇒ 队列满 ⇒ 丢帧, 但发布方**不得被阻塞**。
    for index in 0..(STREAM_SUBSCRIBER_QUEUE_CAPACITY + 10) {
        registry
            .publish_delta(&handle, None, &index.to_string())
            .expect("发布不得被慢订阅者阻塞");
    }

    assert_eq!(subscription.dropped_frames(), 10, "慢订阅者丢帧必须被计数");
    let first = subscription.recv().await.expect("队列里应有最早的帧");
    assert_eq!(first.sequence(), 1);
}

#[tokio::test]
async fn a_subscriber_of_a_finalized_stream_receives_the_final_frame_and_then_the_channel_closes() {
    let registry = StreamRegistry::new();
    let handle = registry
        .register("session-9", "request-9")
        .expect("register");

    let mut frame = C1FinalFrame::new("session-9", "request-9", None, C1StreamState::Completed);
    registry.publish_final(&handle, &mut frame).expect("final");

    let mut subscription = registry.subscribe(&handle).expect("subscribe");
    let received = subscription.recv().await.expect("终帧");
    assert!(received.is_final());
    assert_eq!(received.sequence(), 1);
    assert!(subscription.recv().await.is_none(), "终帧之后通道必须关闭");
}

#[test]
fn registering_the_same_stream_twice_is_refused_and_unknown_streams_are_reported() {
    let registry = StreamRegistry::new();
    registry
        .register("session-10", "request-10")
        .expect("register");

    let error = registry
        .register("session-10", "request-10")
        .expect_err("同一条流不得重复注册");
    assert!(
        matches!(error, StreamError::AlreadyRegistered { .. }),
        "实际错误: {error:?}"
    );

    let ghost = registry
        .subscribe(
            &apeireth_runtime::canonical::canonical_stream::StreamHandle::new(
                "session-ghost",
                "request-ghost",
            ),
        )
        .expect_err("未注册的流不得可订阅");
    assert!(
        matches!(ghost, StreamError::UnknownStream { .. }),
        "实际错误: {ghost:?}"
    );
    assert_eq!(registry.stream_count(), 1);
}

#[test]
fn the_process_wide_registry_is_a_single_instance() {
    let first = global_registry();
    let second = global_registry();
    assert!(
        std::ptr::eq(first, second),
        "进程内权威缓冲必须只有一个实例 (网关只投影这一份)"
    );
}
