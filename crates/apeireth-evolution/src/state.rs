//! 6 状态机 — Idle / Draft / Proposed / Ratified / Active / Retired
//!
//! **合法转换矩阵** (编译时 hardcode):
//! ```text
//!   Idle ──start──> Draft
//!   Draft ──submit──> Proposed
//!   Draft ──abandon──> Retired
//!   Proposed ──council_approve──> Ratified
//!   Proposed ──council_hold──> Draft   (retry)
//!   Proposed ──council_reject──> Retired
//!   Ratified ──activate──> Active
//!   Ratified ──timeout──> Retired
//!   Active ──retire──> Retired
//! ```
//!
//! **不变量**:
//! - `Retired` 是终态, 无 outgoing transition
//! - `Idle` 是初态, 无 incoming transition
//! - L0 修改请求 (目标层 == "L0") → 任何状态都立即 Retired + L0ModificationRejected

use crate::{EvolutionError, EvolutionResult};
use serde::{Deserialize, Serialize};

/// 6 状态枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvolutionState {
    /// 初态 — 等待新提案
    Idle,
    /// 起草中 — 内部设计
    Draft,
    /// 提交智囊团审议
    Proposed,
    /// 智囊团通过, 等待激活
    Ratified,
    /// 已激活 — 正在生效
    Active,
    /// 终态 — 永久退出
    Retired,
}

impl EvolutionState {
    /// 所有 6 状态 (编译时 hardcode 兜底)。
    pub const ALL: [EvolutionState; 6] = [
        Self::Idle,
        Self::Draft,
        Self::Proposed,
        Self::Ratified,
        Self::Active,
        Self::Retired,
    ];

    /// 是否终态 (Retired)。
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Retired)
    }

    /// 是否活跃态 (Active + Ratified)。
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active | Self::Ratified)
    }
}

/// 转换原因 (审计字段)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransitionReason {
    /// start (Idle → Draft)
    Start,
    /// submit (Draft → Proposed)
    Submit,
    /// abandon (Draft → Retired)
    Abandon,
    /// council_approve (Proposed → Ratified)
    CouncilApprove,
    /// council_hold (Proposed → Draft, retry)
    CouncilHold,
    /// council_reject (Proposed → Retired)
    CouncilReject,
    /// activate (Ratified → Active)
    Activate,
    /// timeout (Ratified → Retired)
    ActivationTimeout,
    /// retire (Active → Retired)
    Retire,
    /// L0 防护: 任何 → Retired
    L0Guard,
    /// 完整性校验失败
    IntegrityFailure,
    /// 反思期外 abort
    ReflectionWindowExpired,
    /// 通用失败
    Failure(String),
}

impl TransitionReason {
    /// 是否失败类原因 (用于审计归类)。
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::CouncilReject
                | Self::ActivationTimeout
                | Self::L0Guard
                | Self::IntegrityFailure
                | Self::ReflectionWindowExpired
                | Self::Failure(_)
                | Self::Abandon
        )
    }
}

/// 单次状态转换记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    /// 源状态
    pub from: EvolutionState,
    /// 目标状态
    pub to: EvolutionState,
    /// 原因
    pub reason: TransitionReason,
    /// 时间戳 (epoch ms)
    pub at_ms: i64,
}

impl StateTransition {
    /// 构造一条转换记录。
    pub fn new(
        from: EvolutionState,
        to: EvolutionState,
        reason: TransitionReason,
        at_ms: i64,
    ) -> Self {
        Self {
            from,
            to,
            reason,
            at_ms,
        }
    }
}

/// 6 状态机本体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionStateMachine {
    /// 当前状态
    pub current: EvolutionState,
    /// 转换历史 (审计轨迹)
    pub history: Vec<StateTransition>,
    /// 当前提案 ID (Draft 起占用, Retired 时清空)
    pub active_proposal_id: Option<String>,
}

impl Default for EvolutionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl EvolutionStateMachine {
    /// 创建新状态机 (Idle 初态)。
    pub fn new() -> Self {
        Self {
            current: EvolutionState::Idle,
            history: Vec::new(),
            active_proposal_id: None,
        }
    }

    /// 创建已占用提案的状态机 (用于从 ExternalId 恢复)。
    pub fn with_proposal(proposal_id: impl Into<String>) -> Self {
        Self {
            current: EvolutionState::Idle,
            history: Vec::new(),
            active_proposal_id: Some(proposal_id.into()),
        }
    }

    /// 当前是否终态。
    pub fn is_terminal(&self) -> bool {
        self.current.is_terminal()
    }

    /// 合法转换表 (compile-time hardcode)。
    ///
    /// 返回从 `from` 出发的合法 `to` 集合 (按 docstring 转换矩阵)。
    ///
    /// **特殊**: Idle → Retired 是 abort 路径 (合法), 因为 Idle 是初态可随时放弃。
    pub fn allowed_targets(from: EvolutionState) -> &'static [EvolutionState] {
        match from {
            EvolutionState::Idle => &[EvolutionState::Draft, EvolutionState::Retired],
            EvolutionState::Draft => &[EvolutionState::Proposed, EvolutionState::Retired],
            EvolutionState::Proposed => &[
                EvolutionState::Ratified,
                EvolutionState::Draft,
                EvolutionState::Retired,
            ],
            EvolutionState::Ratified => &[EvolutionState::Active, EvolutionState::Retired],
            EvolutionState::Active => &[EvolutionState::Retired],
            EvolutionState::Retired => &[],
        }
    }

    /// 给定 from + to, 是否在合法转换矩阵内。
    pub fn is_legal(from: EvolutionState, to: EvolutionState) -> bool {
        Self::allowed_targets(from).contains(&to)
    }

    /// 执行一次状态转换。
    ///
    /// **L0 防护**: 若 reason == L0Guard 且 to != Retired, 一律拒绝并返回错误。
    pub fn transition(
        &mut self,
        to: EvolutionState,
        reason: TransitionReason,
        at_ms: i64,
    ) -> EvolutionResult<StateTransition> {
        let from = self.current;

        // L0 guard 强制: 不允许任何中间态进入 L0 修改 (本状态机拒绝 L0 写入)
        if matches!(reason, TransitionReason::L0Guard) && to != EvolutionState::Retired {
            return Err(EvolutionError::IllegalTransition {
                from,
                to,
                reason: "L0Guard must route to Retired".into(),
            });
        }

        // Retired 是终态
        if from.is_terminal() && to != from {
            return Err(EvolutionError::IllegalTransition {
                from,
                to,
                reason: "Retired is terminal".into(),
            });
        }

        if !Self::is_legal(from, to) {
            return Err(EvolutionError::IllegalTransition {
                from,
                to,
                reason: format!("{:?}", reason),
            });
        }

        self.current = to;
        if to == EvolutionState::Retired {
            self.active_proposal_id = None;
        }
        let record = StateTransition::new(from, to, reason, at_ms);
        self.history.push(record.clone());
        Ok(record)
    }

    /// 强制转为 Retired (失败路径统一出口)。
    ///
    /// **用途**: fail-6 trait 触发时调用。
    pub fn abort(
        &mut self,
        reason: TransitionReason,
        at_ms: i64,
    ) -> EvolutionResult<StateTransition> {
        let from = self.current;
        // 终态保持
        if from.is_terminal() {
            let record = StateTransition::new(from, EvolutionState::Retired, reason, at_ms);
            self.history.push(record.clone());
            return Ok(record);
        }
        // 终态以外的合法转换: 任意 → Retired
        if !Self::is_legal(from, EvolutionState::Retired) {
            return Err(EvolutionError::IllegalTransition {
                from,
                to: EvolutionState::Retired,
                reason: format!("{:?}", reason),
            });
        }
        self.current = EvolutionState::Retired;
        self.active_proposal_id = None;
        let record = StateTransition::new(from, EvolutionState::Retired, reason, at_ms);
        self.history.push(record.clone());
        Ok(record)
    }

    /// 历史长度 (审计指标)。
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// 最近一次失败转换的索引 (用于 retry 决策)。
    pub fn last_failure_index(&self) -> Option<usize> {
        self.history.iter().rposition(|t| t.reason.is_failure())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_ms() -> i64 {
        crate::current_time_ms()
    }

    #[test]
    fn all_states_count_is_six() {
        assert_eq!(EvolutionState::ALL.len(), 6);
    }

    #[test]
    fn terminal_is_only_retired() {
        assert!(EvolutionState::Retired.is_terminal());
        for s in [
            EvolutionState::Idle,
            EvolutionState::Draft,
            EvolutionState::Proposed,
            EvolutionState::Ratified,
            EvolutionState::Active,
        ] {
            assert!(!s.is_terminal(), "{:?} should not be terminal", s);
        }
    }

    #[test]
    fn active_includes_active_and_ratified() {
        assert!(EvolutionState::Active.is_active());
        assert!(EvolutionState::Ratified.is_active());
        assert!(!EvolutionState::Idle.is_active());
    }

    #[test]
    fn allowed_targets_table_matches_spec() {
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Idle),
            &[EvolutionState::Draft, EvolutionState::Retired]
        );
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Draft),
            &[EvolutionState::Proposed, EvolutionState::Retired]
        );
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Proposed),
            &[
                EvolutionState::Ratified,
                EvolutionState::Draft,
                EvolutionState::Retired
            ]
        );
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Ratified),
            &[EvolutionState::Active, EvolutionState::Retired]
        );
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Active),
            &[EvolutionState::Retired]
        );
        assert!(EvolutionStateMachine::allowed_targets(EvolutionState::Retired).is_empty());
    }

    #[test]
    fn illegal_transition_is_rejected() {
        let mut m = EvolutionStateMachine::new();
        let r = m.transition(EvolutionState::Active, TransitionReason::Activate, at_ms());
        assert!(matches!(r, Err(EvolutionError::IllegalTransition { .. })));
    }

    #[test]
    fn happy_path_idle_to_active() {
        let mut m = EvolutionStateMachine::new();
        assert_eq!(m.current, EvolutionState::Idle);

        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Ratified,
            TransitionReason::CouncilApprove,
            at_ms(),
        )
        .unwrap();
        m.transition(EvolutionState::Active, TransitionReason::Activate, at_ms())
            .unwrap();

        assert_eq!(m.current, EvolutionState::Active);
        assert!(m.current.is_active());
        assert_eq!(m.history_len(), 4);
    }

    #[test]
    fn retired_is_terminal_no_outgoing() {
        let mut m = EvolutionStateMachine::new();
        m.abort(TransitionReason::Failure("test".into()), at_ms())
            .unwrap();
        assert!(m.is_terminal());

        let r = m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms());
        assert!(r.is_err());
    }

    #[test]
    fn proposed_hold_returns_to_draft_for_retry() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Draft,
            TransitionReason::CouncilHold,
            at_ms(),
        )
        .unwrap();
        assert_eq!(m.current, EvolutionState::Draft);
    }

    #[test]
    fn proposed_reject_retires() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Retired,
            TransitionReason::CouncilReject,
            at_ms(),
        )
        .unwrap();
        assert!(m.is_terminal());
    }

    #[test]
    fn abort_is_legal_from_any_non_terminal() {
        for from in [
            EvolutionState::Idle,
            EvolutionState::Draft,
            EvolutionState::Proposed,
            EvolutionState::Ratified,
            EvolutionState::Active,
        ] {
            let mut m = EvolutionStateMachine::new();
            m.current = from;
            let r = m.abort(TransitionReason::Failure("abort".into()), at_ms());
            assert!(r.is_ok(), "from {:?} should allow abort", from);
            assert!(m.is_terminal());
        }
    }

    #[test]
    fn abort_from_retired_is_idempotent() {
        let mut m = EvolutionStateMachine::new();
        m.abort(TransitionReason::Failure("first".into()), at_ms())
            .unwrap();
        let again = m
            .abort(TransitionReason::Failure("second".into()), at_ms())
            .unwrap();
        assert_eq!(again.from, EvolutionState::Retired);
        assert_eq!(again.to, EvolutionState::Retired);
        assert_eq!(m.history_len(), 2);
    }

    #[test]
    fn l0_guard_to_non_retired_is_rejected() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        let r = m.transition(
            EvolutionState::Draft, // 任意非 Retired
            TransitionReason::L0Guard,
            at_ms(),
        );
        assert!(r.is_err());
    }

    #[test]
    fn l0_guard_to_retired_is_allowed() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        let r = m.transition(EvolutionState::Retired, TransitionReason::L0Guard, at_ms());
        assert!(r.is_ok());
        assert!(m.is_terminal());
    }

    #[test]
    fn ratified_activation_timeout_retires() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Ratified,
            TransitionReason::CouncilApprove,
            at_ms(),
        )
        .unwrap();
        let r = m.transition(
            EvolutionState::Retired,
            TransitionReason::ActivationTimeout,
            at_ms(),
        );
        assert!(r.is_ok());
        assert!(m.is_terminal());
    }

    #[test]
    fn active_to_retired_is_legal() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Ratified,
            TransitionReason::CouncilApprove,
            at_ms(),
        )
        .unwrap();
        m.transition(EvolutionState::Active, TransitionReason::Activate, at_ms())
            .unwrap();
        let r = m.transition(EvolutionState::Retired, TransitionReason::Retire, at_ms());
        assert!(r.is_ok());
        assert!(m.is_terminal());
    }

    #[test]
    fn history_records_all_transitions() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Retired,
            TransitionReason::CouncilReject,
            at_ms(),
        )
        .unwrap();
        assert_eq!(m.history_len(), 3);
        assert_eq!(m.history[0].from, EvolutionState::Idle);
        assert_eq!(m.history[0].to, EvolutionState::Draft);
        assert_eq!(m.history[2].to, EvolutionState::Retired);
    }

    #[test]
    fn last_failure_index_finds_latest() {
        let mut m = EvolutionStateMachine::new();
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        assert_eq!(m.last_failure_index(), None);
        m.transition(
            EvolutionState::Draft,
            TransitionReason::CouncilHold,
            at_ms(),
        )
        .unwrap();
        // CouncilHold 不是 is_failure, 应仍 None
        assert_eq!(m.last_failure_index(), None);
        m.transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms())
            .unwrap();
        m.transition(
            EvolutionState::Retired,
            TransitionReason::CouncilReject,
            at_ms(),
        )
        .unwrap();
        // history[0]=Start, [1]=Submit, [2]=CouncilHold, [3]=Submit, [4]=CouncilReject
        assert_eq!(m.last_failure_index(), Some(4));
    }

    #[test]
    fn with_proposal_sets_id() {
        let m = EvolutionStateMachine::with_proposal("p-1");
        assert_eq!(m.active_proposal_id.as_deref(), Some("p-1"));
        assert_eq!(m.current, EvolutionState::Idle);
    }

    #[test]
    fn retired_clears_active_proposal_id() {
        let mut m = EvolutionStateMachine::with_proposal("p-1");
        m.transition(EvolutionState::Draft, TransitionReason::Start, at_ms())
            .unwrap();
        assert_eq!(m.active_proposal_id.as_deref(), Some("p-1"));
        m.abort(TransitionReason::Failure("test".into()), at_ms())
            .unwrap();
        assert!(m.active_proposal_id.is_none());
    }

    #[test]
    fn transition_reason_is_failure_predicate() {
        assert!(TransitionReason::CouncilReject.is_failure());
        assert!(TransitionReason::L0Guard.is_failure());
        assert!(TransitionReason::Failure("x".into()).is_failure());
        assert!(!TransitionReason::Start.is_failure());
        assert!(!TransitionReason::Submit.is_failure());
        assert!(!TransitionReason::CouncilHold.is_failure());
        assert!(!TransitionReason::Activate.is_failure());
    }

    #[test]
    fn state_display_matches_debug() {
        assert_eq!(format!("{}", EvolutionState::Idle), "Idle");
        assert_eq!(format!("{}", EvolutionState::Retired), "Retired");
    }

    #[test]
    fn history_record_new_keeps_at_ms() {
        let r = StateTransition::new(
            EvolutionState::Draft,
            EvolutionState::Proposed,
            TransitionReason::Submit,
            12345,
        );
        assert_eq!(r.at_ms, 12345);
        assert_eq!(r.from, EvolutionState::Draft);
        assert_eq!(r.to, EvolutionState::Proposed);
    }

    #[test]
    fn idle_target_table_allows_draft_and_retired() {
        // Idle 是初态, 允许直接 abort 到 Retired
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Idle).len(),
            2
        );
    }

    #[test]
    fn proposed_target_table_has_three() {
        assert_eq!(
            EvolutionStateMachine::allowed_targets(EvolutionState::Proposed).len(),
            3
        );
    }
}
