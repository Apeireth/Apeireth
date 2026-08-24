//! trait fail-6 — 6 类失败路径的统一出口
//!
//! **fail-6 路径**:
//! 1. ReflectionFailure        → Retired (反思失败自动放弃)
//! 2. CouncilRejectFailure     → Retired (智囊团拒绝)
//! 3. CouncilHoldFailure       → Draft (多轮 retry, 回到起草)
//! 4. ActivationTimeoutFailure → Retired (激活超时)
//! 5. OutOfReflectionWindowFailure → Retired (反思期外 abort)
//! 6. IntegrityCheckFailure    → Retired (完整性校验失败)
//!
//! **统一出口**: 失败最终都进入 `Retired`, 但 `CouncilHoldFailure` 是特例: 回到
//! `Draft` 触发新一轮起草, 走 retry 循环 (由 `EvolutionEngine` 计数 retry 次数).

use crate::state::{EvolutionState, EvolutionStateMachine, TransitionReason};
use crate::{EvolutionError, EvolutionResult};
use serde::{Deserialize, Serialize};

/// 6 类失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailKind {
    /// 反思失败 (内部状态不一致 / 学习步失败)
    ReflectionFailure,
    /// 智囊团拒绝 (council verdict hold/reject)
    CouncilRejectFailure,
    /// 智囊团按住 (CouncilHold — retry 回到 Draft)
    CouncilHoldFailure,
    /// 激活超时 (Ratified 后未在窗口内激活)
    ActivationTimeoutFailure,
    /// 反思期外 abort (进入 retry 时已超过 reflection_window_ms)
    OutOfReflectionWindowFailure,
    /// 完整性校验失败 (patch 描述空白 / 风险缺失)
    IntegrityCheckFailure,
}

impl FailKind {
    /// 全部 6 类 (编译时 hardcode)。
    pub const ALL: [FailKind; 6] = [
        Self::ReflectionFailure,
        Self::CouncilRejectFailure,
        Self::CouncilHoldFailure,
        Self::ActivationTimeoutFailure,
        Self::OutOfReflectionWindowFailure,
        Self::IntegrityCheckFailure,
    ];

    /// 是否 retry-able (仅 CouncilHoldFailure 走 retry, 其余 → Retired 直接终态)。
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::CouncilHoldFailure)
    }

    /// 映射到状态机 TransitionReason。
    pub fn transition_reason(self) -> TransitionReason {
        match self {
            Self::ReflectionFailure => TransitionReason::Failure("reflection".into()),
            Self::CouncilRejectFailure => TransitionReason::CouncilReject,
            Self::CouncilHoldFailure => TransitionReason::CouncilHold,
            Self::ActivationTimeoutFailure => TransitionReason::ActivationTimeout,
            Self::OutOfReflectionWindowFailure => TransitionReason::ReflectionWindowExpired,
            Self::IntegrityCheckFailure => TransitionReason::IntegrityFailure,
        }
    }

    /// 映射到目标状态。
    ///
    /// **规则**:
    /// - CouncilHoldFailure → Draft (retry)
    /// - 其余 → Retired
    pub const fn target_state(self) -> EvolutionState {
        match self {
            Self::CouncilHoldFailure => EvolutionState::Draft,
            _ => EvolutionState::Retired,
        }
    }
}

/// 失败处理结果 (失败后状态机到达的状态 + 记录)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailOutcome {
    /// 已转 Retired
    Retired,
    /// 已回 Draft (用于 retry)
    RetriedToDraft {
        /// 重试次数 (从 1 起)
        attempt: u32,
    },
}

/// 失败记录 (审计字段)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailRecord {
    /// 失败类型
    pub kind: FailKind,
    /// 失败描述
    pub description: String,
    /// 时间戳 (epoch ms)
    pub at_ms: i64,
    /// 重试次数 (0 = 首次, 1 = 第 2 次, ...)
    pub retry_attempt: u32,
}

impl FailRecord {
    /// 构造失败记录。
    pub fn new(kind: FailKind, description: impl Into<String>, at_ms: i64) -> Self {
        Self {
            kind,
            description: description.into(),
            at_ms,
            retry_attempt: 0,
        }
    }

    /// 标记重试次数。
    pub fn with_retry(mut self, attempt: u32) -> Self {
        self.retry_attempt = attempt;
        self
    }
}

/// 失败策略 (trait fail-6 入口)。
///
/// **设计**: trait 接受状态机 + 当前 epoch ms, 决定如何 fail。
pub trait FailPolicy {
    /// 应用一次失败路径。
    fn apply_fail(
        &self,
        machine: &mut EvolutionStateMachine,
        kind: FailKind,
        record: &FailRecord,
        at_ms: i64,
    ) -> EvolutionResult<FailOutcome>;
}

/// 默认失败策略 — 严格按 `FailKind` 矩阵转换。
#[derive(Debug, Default, Clone, Copy)]
pub struct StrictFailPolicy;

impl FailPolicy for StrictFailPolicy {
    fn apply_fail(
        &self,
        machine: &mut EvolutionStateMachine,
        kind: FailKind,
        _record: &FailRecord,
        at_ms: i64,
    ) -> EvolutionResult<FailOutcome> {
        let reason = kind.transition_reason();
        let target = kind.target_state();

        // 终态保持
        if machine.is_terminal() {
            let _ = machine.transition(EvolutionState::Retired, reason, at_ms);
            return Ok(FailOutcome::Retired);
        }

        let outcome = if matches!(kind, FailKind::CouncilHoldFailure) {
            // retry: 仅从 Proposed 状态合法回到 Draft;
            // Idle/Draft 状态触发 CouncilHold 视为逻辑错误, 强制 Retired.
            if machine.current != EvolutionState::Proposed {
                machine
                    .transition(EvolutionState::Retired, reason, at_ms)
                    .map_err(|e| match e {
                        EvolutionError::IllegalTransition { .. } => {
                            EvolutionError::IllegalTransition {
                                from: machine.current,
                                to: EvolutionState::Retired,
                                reason: "retry from non-Proposed is invalid".into(),
                            }
                        }
                        other => other,
                    })?;
                FailOutcome::Retired
            } else {
                machine.transition(EvolutionState::Draft, reason, at_ms)?;
                FailOutcome::RetriedToDraft {
                    attempt: _record.retry_attempt,
                }
            }
        } else {
            machine
                .transition(target, reason, at_ms)
                .map_err(|e| match e {
                    EvolutionError::IllegalTransition { .. } => EvolutionError::IllegalTransition {
                        from: machine.current,
                        to: target,
                        reason: format!("{:?}", kind),
                    },
                    other => other,
                })?;
            FailOutcome::Retired
        };

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TransitionReason;

    fn at_ms() -> i64 {
        crate::current_time_ms()
    }

    fn to_proposed(m: &mut EvolutionStateMachine) {
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
    }

    #[test]
    fn fail_kind_all_has_six_entries() {
        assert_eq!(FailKind::ALL.len(), 6);
    }

    #[test]
    fn fail_kind_is_retryable_only_council_hold() {
        assert!(FailKind::CouncilHoldFailure.is_retryable());
        for k in [
            FailKind::ReflectionFailure,
            FailKind::CouncilRejectFailure,
            FailKind::ActivationTimeoutFailure,
            FailKind::OutOfReflectionWindowFailure,
            FailKind::IntegrityCheckFailure,
        ] {
            assert!(!k.is_retryable(), "{:?} should not be retryable", k);
        }
    }

    #[test]
    fn fail_kind_target_state_correct() {
        assert_eq!(
            FailKind::CouncilHoldFailure.target_state(),
            EvolutionState::Draft
        );
        for k in [
            FailKind::ReflectionFailure,
            FailKind::CouncilRejectFailure,
            FailKind::ActivationTimeoutFailure,
            FailKind::OutOfReflectionWindowFailure,
            FailKind::IntegrityCheckFailure,
        ] {
            assert_eq!(k.target_state(), EvolutionState::Retired, "{:?}", k);
        }
    }

    #[test]
    fn fail_kind_transition_reason_is_failure() {
        // 6 fail kind 中, 仅 CouncilHold 映射的 CouncilHold 本身不算 is_failure
        // (CouncilHold 是 retry 类, 非真正失败), 其余 5 个都映射到失败类 reason.
        for k in FailKind::ALL {
            if matches!(k, FailKind::CouncilHoldFailure) {
                // CouncilHold 的 reason 是 CouncilHold, 非 is_failure
                assert!(!k.transition_reason().is_failure(), "{:?}", k);
            } else {
                assert!(k.transition_reason().is_failure(), "{:?}", k);
            }
        }
    }

    #[test]
    fn strict_policy_retires_on_reflection_failure() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(FailKind::ReflectionFailure, "internal", at_ms());
        let outcome = policy
            .apply_fail(&mut m, FailKind::ReflectionFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert!(m.is_terminal());
    }

    #[test]
    fn strict_policy_retires_on_council_reject() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(FailKind::CouncilRejectFailure, "council reject", at_ms());
        let outcome = policy
            .apply_fail(&mut m, FailKind::CouncilRejectFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert!(m.is_terminal());
    }

    #[test]
    fn strict_policy_retries_on_council_hold_from_proposed() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        let policy = StrictFailPolicy;
        let rec =
            FailRecord::new(FailKind::CouncilHoldFailure, "council hold", at_ms()).with_retry(1);
        let outcome = policy
            .apply_fail(&mut m, FailKind::CouncilHoldFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::RetriedToDraft { attempt: 1 });
        assert_eq!(m.current, EvolutionState::Draft);
    }

    #[test]
    fn strict_policy_council_hold_from_idle_retires() {
        let mut m = EvolutionStateMachine::new();
        // idle → CouncilHold 不合法, 走 Retired
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(FailKind::CouncilHoldFailure, "from idle", at_ms());
        let outcome = policy
            .apply_fail(&mut m, FailKind::CouncilHoldFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert!(m.is_terminal());
    }

    #[test]
    fn strict_policy_retires_on_activation_timeout() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        m.transition(
            EvolutionState::Ratified,
            TransitionReason::CouncilApprove,
            at_ms(),
        )
        .unwrap();
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(FailKind::ActivationTimeoutFailure, "timeout", at_ms());
        let outcome = policy
            .apply_fail(&mut m, FailKind::ActivationTimeoutFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert!(m.is_terminal());
    }

    #[test]
    fn strict_policy_retires_on_out_of_reflection_window() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(
            FailKind::OutOfReflectionWindowFailure,
            "window expired",
            at_ms(),
        );
        let outcome = policy
            .apply_fail(
                &mut m,
                FailKind::OutOfReflectionWindowFailure,
                &rec,
                at_ms(),
            )
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
    }

    #[test]
    fn strict_policy_retires_on_integrity_failure() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(FailKind::IntegrityCheckFailure, "patch bad", at_ms());
        let outcome = policy
            .apply_fail(&mut m, FailKind::IntegrityCheckFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
    }

    #[test]
    fn strict_policy_from_terminal_is_idempotent() {
        let mut m = EvolutionStateMachine::new();
        m.abort(TransitionReason::Failure("init".into()), at_ms())
            .unwrap();
        let policy = StrictFailPolicy;
        let rec = FailRecord::new(FailKind::ReflectionFailure, "after terminal", at_ms());
        let outcome = policy
            .apply_fail(&mut m, FailKind::ReflectionFailure, &rec, at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert!(m.is_terminal());
    }

    #[test]
    fn fail_record_new_sets_retry_zero() {
        let r = FailRecord::new(FailKind::ReflectionFailure, "x", 0);
        assert_eq!(r.retry_attempt, 0);
        assert_eq!(r.kind, FailKind::ReflectionFailure);
    }

    #[test]
    fn fail_record_with_retry_updates_attempt() {
        let r = FailRecord::new(FailKind::CouncilHoldFailure, "x", 0).with_retry(3);
        assert_eq!(r.retry_attempt, 3);
    }

    #[test]
    fn strict_policy_retried_increments_attempt() {
        let mut m = EvolutionStateMachine::new();
        to_proposed(&mut m);
        let policy = StrictFailPolicy;
        // 模拟第 1 次 retry
        let rec = FailRecord::new(FailKind::CouncilHoldFailure, "h", at_ms()).with_retry(1);
        let outcome = policy
            .apply_fail(&mut m, FailKind::CouncilHoldFailure, &rec, at_ms())
            .unwrap();
        assert!(matches!(
            outcome,
            FailOutcome::RetriedToDraft { attempt: 1 }
        ));
        // 重新提交
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        // 模拟第 2 次 retry
        let rec2 = FailRecord::new(FailKind::CouncilHoldFailure, "h", at_ms()).with_retry(2);
        let outcome2 = policy
            .apply_fail(&mut m, FailKind::CouncilHoldFailure, &rec2, at_ms())
            .unwrap();
        assert!(matches!(
            outcome2,
            FailOutcome::RetriedToDraft { attempt: 2 }
        ));
    }
}
