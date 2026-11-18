//! C2-3: C1 终帧映射 (仅纯函数)。
//!
//! 本模块把运行时真实结果 (`RuntimeResult<TurnOutcome>`) 映射为 C1 终帧:
//! - 显式停止优先于结果 (停止摘要冻结后, 即使回合随后返回 Completed, 终态仍是
//!   `stopped`);
//! - `Completed` ⇒ `completed` (完整文本 + 用量如实映射);
//! - `PendingApproval` ⇒ `approval` (携带本次待批清单);
//! - `Err` ⇒ `failed` (错误文本如实给出)。
//!
//! 不做什么 (诚实边界): 不改 `TurnOutcome` 形状, 不生成停止摘要, 不做恢复后
//! 续流; 映射只描述运行时已有事实。

use apeireth_core::kernel::{RequestId, SessionId, TraceId};
use apeireth_protocol::canonical::NormalizedUsage;
use apeireth_protocol::canonical_stream::{C1Approval, C1FinalFrame, C1StreamState, C1Usage};

use super::approval::PendingApprovalView;
use super::error::RuntimeResult;
use super::execute::TurnOutcome;
use super::turn_cancellation::StopSummary;

/// 把一个回合结果映射为 C1 终帧 (帧序号由权威寄存器回填)。
///
/// 停止优先: 只要该回合有冻结的停止摘要, 终态就是 `stopped`, 不让后续正常
/// 完成覆盖停止事实。
pub(super) fn final_frame(
    session: SessionId,
    request: RequestId,
    trace: TraceId,
    result: &RuntimeResult<TurnOutcome>,
    stopped: Option<StopSummary>,
) -> C1FinalFrame {
    let state = if stopped.is_some() {
        C1StreamState::Stopped
    } else {
        match result {
            Ok(TurnOutcome::Completed(_)) => C1StreamState::Completed,
            Ok(TurnOutcome::PendingApproval(_)) => C1StreamState::Approval,
            Err(_) => C1StreamState::Failed,
        }
    };
    let mut frame = C1FinalFrame::new(session.to_string(), request.to_string(), None, state);
    frame.trace_id = Some(trace.to_string());
    if stopped.is_some() {
        return frame;
    }
    match result {
        Ok(TurnOutcome::Completed(response)) => {
            frame.text = Some(response.text.clone());
            frame.usage = Some(usage_from(&response.usage));
        }
        Ok(TurnOutcome::PendingApproval(view)) => {
            frame.approvals = Some(vec![approval_from(view)]);
            if !view.generated_text.is_empty() {
                frame.text = Some(view.generated_text.clone());
            }
        }
        Err(error) => {
            frame.error = Some(error.to_string());
        }
    }
    frame
}

/// 用量映射: 输入/输出如实搬运, 总量由 C1Usage::new 相加得出。
pub(super) fn usage_from(usage: &NormalizedUsage) -> C1Usage {
    C1Usage::new(
        u64::from(usage.prompt_tokens),
        u64::from(usage.completion_tokens),
    )
}

/// 单条待批映射: 空动作摘要 ⇒ None (不给假摘要); 过期时刻如实给出。
pub(super) fn approval_from(view: &PendingApprovalView) -> C1Approval {
    let mut approval = C1Approval::new(view.approval_id.to_string());
    if !view.action_binding_digest.is_empty() {
        approval.action_binding_digest = Some(view.action_binding_digest.clone());
    }
    approval.expires_at_ms = Some(view.expires_at.epoch_millis());
    approval
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{ApprovalId, CapabilityId, Timestamp};
    use apeireth_protocol::canonical::ToolCall;

    fn usage(prompt: u32, completion: u32) -> NormalizedUsage {
        NormalizedUsage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            prompt_cache_hit_tokens: None,
        }
    }

    fn view(digest: &str) -> PendingApprovalView {
        let expires = Timestamp::from_epoch_millis(1_700_000_030_000).unwrap();
        PendingApprovalView {
            approval_id: ApprovalId::new(),
            session_id: SessionId::new(),
            request_id: RequestId::new(),
            trace_id: TraceId::new(),
            round: 1,
            capability_id: CapabilityId::new("tool.approval_b").unwrap(),
            tool_name: "approval_b".into(),
            tool_call: ToolCall {
                id: "call_b".into(),
                name: "approval_b".into(),
                arguments: serde_json::json!({}),
            },
            effective_invocation: None,
            governance_hook: "permission".into(),
            governance_reason: "approval required".into(),
            command_text: String::new(),
            arguments_summary: String::new(),
            authorization_families: Vec::new(),
            action_binding_digest: digest.into(),
            created_at: Timestamp::from_epoch_millis(1_700_000_000_000).unwrap(),
            expires_at: expires,
            operation_fingerprint: "fingerprint".into(),
            generated_text: "before approval".into(),
            rounds_used: 1,
        }
    }

    fn completed(text: &str, prompt: u32, completion: u32) -> TurnOutcome {
        TurnOutcome::Completed(super::super::execute::TurnResponse {
            session: SessionId::new(),
            request: RequestId::new(),
            text: text.into(),
            served_by: CapabilityId::new("provider.fake").unwrap(),
            usage: usage(prompt, completion),
            rounds: 1,
            trace: super::super::trace::ExecutionTrace::new(
                TraceId::new(),
                SessionId::new(),
                RequestId::new(),
            ),
        })
    }

    #[test]
    fn completed_carries_text_and_added_usage() {
        let frame = final_frame(
            SessionId::new(),
            RequestId::new(),
            TraceId::new(),
            &Ok(completed("hello", 7, 3)),
            None,
        );
        assert_eq!(frame.state, C1StreamState::Completed);
        assert_eq!(frame.text.as_deref(), Some("hello"));
        assert_eq!(
            frame.usage,
            Some(C1Usage {
                prompt_tokens: 7,
                completion_tokens: 3,
                total_tokens: 10,
            })
        );
    }

    #[test]
    fn pending_approval_carries_the_approval_view() {
        let view = view("");
        let frame = final_frame(
            view.session_id,
            view.request_id,
            view.trace_id,
            &Ok(TurnOutcome::PendingApproval(view.clone())),
            None,
        );
        assert_eq!(frame.state, C1StreamState::Approval);
        assert_eq!(frame.text.as_deref(), Some("before approval"));
        let approvals = frame.approvals.expect("approval list");
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals[0].approval_id, view.approval_id.to_string());
        assert_eq!(approvals[0].action_binding_digest, None);
        assert_eq!(
            approvals[0].expires_at_ms,
            Some(view.expires_at.epoch_millis())
        );
    }

    #[test]
    fn failed_carries_the_error_and_stopped_wins() {
        let failed = final_frame(
            SessionId::new(),
            RequestId::new(),
            TraceId::new(),
            &Err(super::super::error::RuntimeError::misconfigured("boom")),
            None,
        );
        assert_eq!(failed.state, C1StreamState::Failed);
        assert!(failed.error.as_deref().unwrap().contains("boom"));

        let summary = super::super::turn_cancellation::StopSummary {
            session_id: SessionId::new(),
            request_id: RequestId::new(),
            state: super::super::turn_cancellation::StopState::Stopped,
            effects_may_be_incomplete: true,
        };
        let stopped = final_frame(
            SessionId::new(),
            RequestId::new(),
            TraceId::new(),
            &Ok(completed("late", 1, 1)),
            Some(summary),
        );
        assert_eq!(stopped.state, C1StreamState::Stopped);
        assert_eq!(stopped.text, None);
    }

    #[test]
    fn usage_sums_prompt_and_completion() {
        let mapped = usage_from(&usage(11, 5));
        assert_eq!(mapped.total_tokens, 16);
    }
}
