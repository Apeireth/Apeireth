//! C1 规范流帧 schema (后端合同 C 批 · C1 段)。
//!
//! 钉的是**线上字节形态**: 事件名、snake_case 字段名、可选位的省略口径、
//! `resync_required` 事实位的恒存在, 以及「帧里没有宿主侧标签、没有 `[DONE]`」。

use apeireth_protocol::canonical_stream::{
    C1Approval, C1DeltaFrame, C1FinalFrame, C1Frame, C1StreamState, C1Usage, C1_DELTA_EVENT,
    C1_FINAL_EVENT,
};
use serde_json::json;

#[test]
fn a_delta_frame_carries_the_c1_delta_event_name_and_snake_case_fields() {
    let frame = C1DeltaFrame::new("session-1", "request-1", Some("trace-1"), 7, "你好");

    let value = serde_json::to_value(&frame).expect("delta frame serializes");

    assert_eq!(
        value,
        json!({
            "event": "apeireth.c1.delta.v1",
            "session_id": "session-1",
            "request_id": "request-1",
            "trace_id": "trace-1",
            "sequence": 7,
            "delta": "你好",
            "state": "running",
        })
    );
}

#[test]
fn a_final_frame_carries_the_c1_final_event_name_with_its_content_slots() {
    let mut frame = C1FinalFrame::new(
        "session-1",
        "request-1",
        Some("trace-1"),
        C1StreamState::Approval,
    );
    frame.sequence = 9;
    frame.text = Some("partial answer".to_string());
    frame.usage = Some(C1Usage::new(10, 5));
    frame.approvals = Some(vec![C1Approval {
        approval_id: "approval-1".to_string(),
        action_binding_digest: Some("digest-1".to_string()),
        expires_at_ms: Some(1_700_000_000_000),
    }]);

    let value = serde_json::to_value(&frame).expect("final frame serializes");

    assert_eq!(
        value,
        json!({
            "event": "apeireth.c1.final.v1",
            "session_id": "session-1",
            "request_id": "request-1",
            "trace_id": "trace-1",
            "sequence": 9,
            "state": "approval",
            "text": "partial answer",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
            },
            "approvals": [{
                "approval_id": "approval-1",
                "action_binding_digest": "digest-1",
                "expires_at_ms": 1_700_000_000_000i64,
            }],
            "resync_required": false,
        })
    );
}

#[test]
fn absent_optional_slots_are_omitted_but_the_resync_fact_flag_is_always_present() {
    let frame = C1FinalFrame::new("session-1", "request-1", None, C1StreamState::Unknown);

    let value = serde_json::to_value(&frame).expect("final frame serializes");

    for omitted in ["trace_id", "text", "usage", "approvals", "error"] {
        assert!(
            value.get(omitted).is_none(),
            "可选位 {omitted} 应省略, 实际 {value}"
        );
    }
    assert_eq!(
        value["resync_required"],
        json!(false),
        "resync_required 恒存在 (客户端可依赖该字段): {value}"
    );
}

#[test]
fn the_resync_fact_flag_survives_a_json_round_trip() {
    let mut frame = C1FinalFrame::new("session-1", "request-1", None, C1StreamState::Unknown);
    frame.resync_required = true;

    let encoded = serde_json::to_string(&frame).expect("serialize");
    let decoded: C1FinalFrame = serde_json::from_str(&encoded).expect("deserialize");

    assert!(decoded.resync_required);
    assert_eq!(decoded, frame);
}

#[test]
fn the_state_vocabulary_is_the_contract_vocabulary() {
    for (state, wire) in [
        (C1StreamState::Running, "running"),
        (C1StreamState::Completed, "completed"),
        (C1StreamState::Approval, "approval"),
        (C1StreamState::Failed, "failed"),
        (C1StreamState::Unknown, "unknown"),
    ] {
        assert_eq!(
            serde_json::to_value(state).expect("state serializes"),
            json!(wire)
        );
        assert_eq!(state.as_str(), wire);
    }
}

#[test]
fn only_the_running_state_is_a_non_terminal_state() {
    assert!(!C1StreamState::Running.is_terminal());
    for terminal in [
        C1StreamState::Completed,
        C1StreamState::Approval,
        C1StreamState::Failed,
        C1StreamState::Unknown,
    ] {
        assert!(terminal.is_terminal(), "{terminal:?} 必须是终态");
    }
}

#[test]
fn the_usage_receipt_reports_the_total_as_the_sum_of_its_two_parts() {
    let usage = C1Usage::new(12, 30);
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 30);
    assert_eq!(usage.total_tokens, 42);
}

#[test]
fn the_frame_container_is_wire_transparent() {
    let delta = C1DeltaFrame::new("session-1", "request-1", None, 1, "x");
    let final_frame = C1FinalFrame::new("session-1", "request-1", None, C1StreamState::Completed);

    let delta_container = C1Frame::Delta(delta.clone());
    let final_container = C1Frame::Final(final_frame.clone());

    assert_eq!(
        serde_json::to_string(&delta_container).expect("serialize"),
        serde_json::to_string(&delta).expect("serialize"),
        "容器序列化必须原样透传内层帧 (线上不多一层包装)"
    );
    assert_eq!(
        serde_json::to_string(&final_container).expect("serialize"),
        serde_json::to_string(&final_frame).expect("serialize")
    );
    assert_eq!(delta_container.event_name(), C1_DELTA_EVENT);
    assert_eq!(final_container.event_name(), C1_FINAL_EVENT);
    assert_eq!(delta_container.sequence(), 1);
    assert!(!delta_container.is_final());
    assert!(final_container.is_final());
    assert_eq!(delta_container.session_id(), "session-1");
    assert_eq!(final_container.request_id(), "request-1");
}

#[test]
fn the_frames_carry_no_done_sentinel_and_no_host_side_labels() {
    let delta = C1DeltaFrame::new("session-1", "request-1", None, 1, "x");
    let mut final_frame =
        C1FinalFrame::new("session-1", "request-1", None, C1StreamState::Completed);
    final_frame.sequence = 2;

    for encoded in [
        serde_json::to_string(&delta).expect("serialize"),
        serde_json::to_string(&final_frame).expect("serialize"),
    ] {
        assert!(
            !encoded.contains("[DONE]"),
            "帧里不得出现 [DONE]: {encoded}"
        );
        assert!(
            !encoded.contains("operation"),
            "帧里不得自造 operation: {encoded}"
        );
        assert!(
            !encoded.contains("incarnation"),
            "帧里不得自造 incarnation: {encoded}"
        );
        assert!(
            !encoded.contains("context_key"),
            "帧里不得自造 context_key: {encoded}"
        );
    }
}
