//! C2-2 回合级取消注册表的对外行为面。
//!
//! 每个用例用新铸 id 造键 (实际不会碰撞), 不依赖全局注册表的清空顺序。

use apeireth_core::kernel::{RequestId, SessionId};
use apeireth_plugin::ToolCancellation;
use apeireth_runtime::canonical::turn_cancellation::{
    cancellation_for, register, stop, StopOutcome, StopState, RETAINED_STOP_SUMMARIES,
};

#[test]
fn stop_cancels_the_registered_turn_token() {
    let session = SessionId::new();
    let request = RequestId::new();
    let _guard = register(session, request);
    let token = cancellation_for(session, request, Vec::new());
    assert!(!token.is_cancelled());

    let outcome = stop(session, request);
    assert!(matches!(outcome, StopOutcome::Stopped(_)));
    assert!(token.is_cancelled(), "stop must cancel the live turn token");
}

#[test]
fn repeated_stop_replays_the_same_summary() {
    let session = SessionId::new();
    let request = RequestId::new();
    let _guard = register(session, request);

    let StopOutcome::Stopped(first) = stop(session, request) else {
        panic!("first stop must hit the live turn");
    };
    let StopOutcome::AlreadyStopped(second) = stop(session, request) else {
        panic!("second stop must replay, not re-stop");
    };
    assert_eq!(first, second);
    assert_eq!(first.state, StopState::Stopped);

    let wire = serde_json::to_value(&first).expect("summary serializes");
    assert_eq!(wire["state"], "stopped");
    assert_eq!(wire["effects_may_be_incomplete"], true);
    assert_eq!(wire["session_id"], session.to_string());
    assert_eq!(wire["request_id"], request.to_string());
}

#[test]
fn dropping_the_turn_guard_keeps_the_stop_summary() {
    let session = SessionId::new();
    let request = RequestId::new();
    let guard = register(session, request);
    let StopOutcome::Stopped(first) = stop(session, request) else {
        panic!("stop must hit the live turn");
    };
    drop(guard);

    // 运行中条目已注销: 新派发的工具上下文不再接上旧令牌。
    let fresh = cancellation_for(session, request, Vec::new());
    assert!(!fresh.is_cancelled());
    // 但停止摘要保留 ⇒ 重复 stop 仍幂等回放。
    let StopOutcome::AlreadyStopped(replay) = stop(session, request) else {
        panic!("stop summary must outlive the turn");
    };
    assert_eq!(first, replay);
}

#[test]
fn unknown_turn_is_not_found() {
    let session = SessionId::new();
    let request = RequestId::new();
    assert!(matches!(stop(session, request), StopOutcome::NotFound));
}

#[test]
fn cancellation_keeps_parent_sources() {
    let session = SessionId::new();
    let request = RequestId::new();
    let _guard = register(session, request);
    let parent = ToolCancellation::default();
    let token = cancellation_for(session, request, vec![parent.clone()]);
    assert!(!token.is_cancelled());
    parent.cancel();
    assert!(token.is_cancelled(), "parent cancellation must propagate");
}

#[test]
fn retained_stop_summaries_is_bounded() {
    assert_eq!(RETAINED_STOP_SUMMARIES, 512);
}
