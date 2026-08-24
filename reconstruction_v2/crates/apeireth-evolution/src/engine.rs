//! EvolutionEngine — 顶层驱动器, 串联状态机 + fail-6 policy + L0 防护.

use crate::fail::{FailKind, FailOutcome, FailPolicy, FailRecord, StrictFailPolicy};
use crate::state::{EvolutionState, EvolutionStateMachine, TransitionReason};
use crate::traits::{BasicEvolution, Patch, SelfModification, SystemState};
use crate::{
    EvolutionError, EvolutionResult, DEFAULT_MAX_RETRY, DEFAULT_REFLECTION_WINDOW, L0_ANCHOR,
};
use serde::{Deserialize, Serialize};

/// 单步演化操作 (审计日志条目)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionStep {
    /// 启动 (Idle → Draft)
    Started {
        /// 时间戳
        at_ms: i64,
    },
    /// 提交审议 (Draft → Proposed)
    Submitted {
        /// 时间戳
        at_ms: i64,
    },
    /// 通过 (Proposed → Ratified)
    Ratified {
        /// 时间戳
        at_ms: i64,
    },
    /// 激活 (Ratified → Active)
    Activated {
        /// 时间戳
        at_ms: i64,
    },
    /// 放弃 (Draft → Retired)
    Abandoned {
        /// 描述
        reason: String,
        /// 时间戳
        at_ms: i64,
    },
    /// 退场 (Active → Retired)
    Retired {
        /// 描述
        reason: String,
        /// 时间戳
        at_ms: i64,
    },
    /// 失败: 由 trait fail-6 触发
    Failed {
        /// 失败类型
        kind: FailKind,
        /// 描述
        description: String,
        /// 重试次数 (0 = 首次)
        retry_attempt: u32,
        /// 失败产出
        outcome: FailOutcome,
        /// 时间戳
        at_ms: i64,
    },
    /// L0 防护: 任何 patch 触 L0 即触发
    L0GuardTriggered {
        /// 试图写入的目标
        target: String,
        /// 时间戳
        at_ms: i64,
    },
}

/// 演化日志 (顶层驱动器产出)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLog {
    /// 提案 ID (Draft 起占用, Retired 时清空)
    pub proposal_id: String,
    /// 起始时间 (epoch ms)
    pub started_at_ms: i64,
    /// 结束时间 (epoch ms)
    pub ended_at_ms: Option<i64>,
    /// 步骤序列
    pub steps: Vec<EvolutionStep>,
    /// 最终状态
    pub final_state: EvolutionState,
    /// 是否成功 (最终 Active)
    pub succeeded: bool,
}

/// EvolutionEngine 配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineConfig {
    /// 反思窗口 (ms) — 超过则触发 OutOfReflectionWindow
    pub reflection_window_ms: u64,
    /// 最大重试轮次 (CouncilHold 重试上限)
    pub max_retry: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            reflection_window_ms: DEFAULT_REFLECTION_WINDOW,
            max_retry: DEFAULT_MAX_RETRY,
        }
    }
}

/// EvolutionEngine — 顶层驱动器。
///
/// **责任**:
/// 1. 启动提案 (start)
/// 2. 提交审议 (submit)
/// 3. 调用 council (submit_council / on_council_verdict)
/// 4. 处理 fail-6 (apply_fail)
/// 5. L0 防护
/// 6. 激活 / 退场
pub struct EvolutionEngine<P: FailPolicy = StrictFailPolicy> {
    /// 状态机
    state_machine: EvolutionStateMachine,
    /// 配置
    config: EngineConfig,
    /// 失败策略
    fail_policy: P,
    /// 演化引擎的 trait 实现 (4 trait)
    traits_impl: BasicEvolution,
    /// 日志
    log: EvolutionLog,
    /// 当前重试次数
    retry_count: u32,
    /// 起始时间
    started_at_ms: i64,
}

impl<P: FailPolicy> EvolutionEngine<P> {
    /// 创建新引擎。
    pub fn new(proposal_id: impl Into<String>, fail_policy: P) -> Self {
        Self::with_config(proposal_id, EngineConfig::default(), fail_policy)
    }

    /// 带配置创建。
    pub fn with_config(
        proposal_id: impl Into<String>,
        config: EngineConfig,
        fail_policy: P,
    ) -> Self {
        let proposal_id = proposal_id.into();
        let started_at_ms = crate::current_time_ms();
        Self {
            state_machine: EvolutionStateMachine::with_proposal(proposal_id.clone()),
            config,
            fail_policy,
            traits_impl: BasicEvolution::new(),
            log: EvolutionLog {
                proposal_id,
                started_at_ms,
                ended_at_ms: None,
                steps: Vec::new(),
                final_state: EvolutionState::Idle,
                succeeded: false,
            },
            retry_count: 0,
            started_at_ms,
        }
    }

    /// 当前状态机引用。
    pub fn state_machine(&self) -> &EvolutionStateMachine {
        &self.state_machine
    }

    /// 当前状态 (state_machine.current 转发)。
    pub fn current_state(&self) -> EvolutionState {
        self.state_machine.current
    }

    /// 当前 trait 实现引用。
    pub fn traits_impl(&self) -> &BasicEvolution {
        &self.traits_impl
    }

    /// trait 实现可变引用 (供扩展)。
    pub fn traits_impl_mut(&mut self) -> &mut BasicEvolution {
        &mut self.traits_impl
    }

    /// 日志引用。
    pub fn log(&self) -> &EvolutionLog {
        &self.log
    }

    /// 重试计数。
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    fn push_step(&mut self, step: EvolutionStep) {
        self.log.steps.push(step);
    }

    /// 启动 (Idle → Draft)。
    pub fn start(&mut self, at_ms: i64) -> EvolutionResult<()> {
        self.state_machine
            .transition(EvolutionState::Draft, TransitionReason::Start, at_ms)?;
        self.push_step(EvolutionStep::Started { at_ms });
        Ok(())
    }

    /// 提交审议 (Draft → Proposed)。
    pub fn submit(&mut self, at_ms: i64) -> EvolutionResult<()> {
        self.state_machine
            .transition(EvolutionState::Proposed, TransitionReason::Submit, at_ms)?;
        self.push_step(EvolutionStep::Submitted { at_ms });
        Ok(())
    }

    /// 申请激活 (Ratified → Active)。
    pub fn activate(&mut self, at_ms: i64) -> EvolutionResult<()> {
        self.state_machine
            .transition(EvolutionState::Active, TransitionReason::Activate, at_ms)?;
        self.push_step(EvolutionStep::Activated { at_ms });
        self.finalize_success(at_ms);
        Ok(())
    }

    /// 放弃 (Draft → Retired) — 仅 Draft 状态可放弃。
    pub fn abandon(&mut self, reason: impl Into<String>, at_ms: i64) -> EvolutionResult<()> {
        let reason = reason.into();
        let r = TransitionReason::Abandon;
        if self.state_machine.current != EvolutionState::Draft {
            return Err(EvolutionError::IllegalTransition {
                from: self.state_machine.current,
                to: EvolutionState::Retired,
                reason: "abandon only from Draft".into(),
            });
        }
        self.state_machine
            .transition(EvolutionState::Retired, r, at_ms)?;
        self.push_step(EvolutionStep::Abandoned {
            reason: reason.clone(),
            at_ms,
        });
        self.finalize_failure(at_ms);
        Ok(())
    }

    /// 退场 (Active → Retired)。
    pub fn retire(&mut self, reason: impl Into<String>, at_ms: i64) -> EvolutionResult<()> {
        let reason = reason.into();
        if self.state_machine.current != EvolutionState::Active {
            return Err(EvolutionError::IllegalTransition {
                from: self.state_machine.current,
                to: EvolutionState::Retired,
                reason: "retire only from Active".into(),
            });
        }
        self.state_machine
            .transition(EvolutionState::Retired, TransitionReason::Retire, at_ms)?;
        self.push_step(EvolutionStep::Retired {
            reason: reason.clone(),
            at_ms,
        });
        self.finalize_failure(at_ms);
        Ok(())
    }

    /// 标记 Ratified (由 council_bridge 调用)。
    pub fn mark_ratified(&mut self, at_ms: i64) -> EvolutionResult<()> {
        self.state_machine.transition(
            EvolutionState::Ratified,
            TransitionReason::CouncilApprove,
            at_ms,
        )?;
        self.push_step(EvolutionStep::Ratified { at_ms });
        Ok(())
    }

    /// L0 防护: 任何 patch 触 L0 即触发 (立即 Retired)。
    pub fn guard_l0(&mut self, target: impl Into<String>, at_ms: i64) -> EvolutionResult<()> {
        let target = target.into();
        self.state_machine
            .transition(EvolutionState::Retired, TransitionReason::L0Guard, at_ms)?;
        self.push_step(EvolutionStep::L0GuardTriggered {
            target: target.clone(),
            at_ms,
        });
        self.finalize_failure(at_ms);
        Ok(())
    }

    /// 通用失败入口 (trait fail-6)。
    pub fn apply_fail(
        &mut self,
        kind: FailKind,
        description: impl Into<String>,
        at_ms: i64,
    ) -> EvolutionResult<FailOutcome> {
        let description = description.into();

        // retry 预算检查 (CouncilHold 才有重试)
        if matches!(kind, FailKind::CouncilHoldFailure) {
            if self.retry_count >= self.config.max_retry {
                // 预算耗尽 → 升级为 ReflectionFailure (一次性终态)
                let rec = FailRecord::new(FailKind::ReflectionFailure, "budget", at_ms)
                    .with_retry(self.retry_count);
                let outcome = self
                    .fail_policy
                    .apply_fail(
                        &mut self.state_machine,
                        FailKind::ReflectionFailure,
                        &rec,
                        at_ms,
                    )
                    .unwrap_or(FailOutcome::Retired);
                // 直接转 Retired (绕过 retry)
                let _ = self.state_machine.transition(
                    EvolutionState::Retired,
                    TransitionReason::Failure("budget".into()),
                    at_ms,
                );
                self.push_step(EvolutionStep::Failed {
                    kind,
                    description: description.clone(),
                    retry_attempt: self.retry_count,
                    outcome: FailOutcome::Retired,
                    at_ms,
                });
                self.finalize_failure(at_ms);
                return Ok(outcome);
            }
            self.retry_count += 1;
        }

        // 反思期外检查 (任何 retry-able 失败都需在窗口内)
        let elapsed = at_ms.saturating_sub(self.started_at_ms).max(0) as u64;
        if kind.is_retryable() && elapsed > self.config.reflection_window_ms {
            // 反思期外 → OutOfReflectionWindow → 终态
            let rec = FailRecord::new(FailKind::OutOfReflectionWindowFailure, "elapsed", at_ms)
                .with_retry(self.retry_count);
            let outcome = self.fail_policy.apply_fail(
                &mut self.state_machine,
                FailKind::OutOfReflectionWindowFailure,
                &rec,
                at_ms,
            )?;
            self.push_step(EvolutionStep::Failed {
                kind: FailKind::OutOfReflectionWindowFailure,
                description: "elapsed exceeded reflection window".into(),
                retry_attempt: self.retry_count,
                outcome,
                at_ms,
            });
            self.finalize_failure(at_ms);
            return Ok(FailOutcome::Retired);
        }

        let rec = FailRecord::new(kind, description.clone(), at_ms).with_retry(self.retry_count);
        let outcome = self
            .fail_policy
            .apply_fail(&mut self.state_machine, kind, &rec, at_ms)?;
        let attempt = match outcome {
            FailOutcome::RetriedToDraft { attempt } => attempt,
            FailOutcome::Retired => self.retry_count,
        };
        self.push_step(EvolutionStep::Failed {
            kind,
            description,
            retry_attempt: attempt,
            outcome: outcome.clone(),
            at_ms,
        });

        if matches!(outcome, FailOutcome::Retired) {
            self.finalize_failure(at_ms);
        }
        Ok(outcome)
    }

    fn finalize_success(&mut self, at_ms: i64) {
        self.log.ended_at_ms = Some(at_ms);
        self.log.final_state = self.state_machine.current;
        self.log.succeeded = self.state_machine.current == EvolutionState::Active;
    }

    fn finalize_failure(&mut self, at_ms: i64) {
        self.log.ended_at_ms = Some(at_ms);
        self.log.final_state = self.state_machine.current;
        self.log.succeeded = false;
    }

    /// 提议一个补丁 (走 L0 防护)。
    pub fn propose_patch(&self, current: &SystemState) -> Patch {
        self.traits_impl.propose_patch(current)
    }

    /// L0 锚定层标识 (公开常量, 用于跨 crate 校验)。
    pub fn l0_anchor(&self) -> &'static str {
        L0_ANCHOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Extension;

    fn at_ms() -> i64 {
        crate::current_time_ms()
    }

    #[test]
    fn engine_starts_in_idle() {
        let e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        assert_eq!(e.current_state(), EvolutionState::Idle);
    }

    #[test]
    fn engine_happy_path_to_active() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        e.mark_ratified(at_ms()).unwrap();
        e.activate(at_ms()).unwrap();
        assert_eq!(e.current_state(), EvolutionState::Active);
        assert!(e.log().succeeded);
        assert!(e.log().ended_at_ms.is_some());
    }

    #[test]
    fn engine_abandon_from_draft_retires() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.abandon("user cancel", at_ms()).unwrap();
        assert_eq!(e.current_state(), EvolutionState::Retired);
        assert!(!e.log().succeeded);
    }

    #[test]
    fn engine_abandon_from_idle_fails() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        let r = e.abandon("x", at_ms());
        assert!(r.is_err());
    }

    #[test]
    fn engine_retire_from_active() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        e.mark_ratified(at_ms()).unwrap();
        e.activate(at_ms()).unwrap();
        e.retire("decom", at_ms()).unwrap();
        assert!(e.log().final_state.is_terminal());
    }

    #[test]
    fn engine_retire_from_non_active_fails() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        let r = e.retire("x", at_ms());
        assert!(r.is_err());
    }

    #[test]
    fn engine_apply_reflection_failure_retires() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        let outcome = e
            .apply_fail(FailKind::ReflectionFailure, "internal", at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert_eq!(e.current_state(), EvolutionState::Retired);
    }

    #[test]
    fn engine_apply_council_hold_retries_to_draft() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        let outcome = e
            .apply_fail(FailKind::CouncilHoldFailure, "hold", at_ms())
            .unwrap();
        assert!(matches!(outcome, FailOutcome::RetriedToDraft { .. }));
        assert_eq!(e.current_state(), EvolutionState::Draft);
        assert_eq!(e.retry_count(), 1);
    }

    #[test]
    fn engine_l0_guard_immediately_retires() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.guard_l0("L0", at_ms()).unwrap();
        assert_eq!(e.current_state(), EvolutionState::Retired);
    }

    #[test]
    fn engine_retry_budget_exhaustion_terminates() {
        let mut e: EvolutionEngine = EvolutionEngine::with_config(
            "p-1",
            EngineConfig {
                reflection_window_ms: DEFAULT_REFLECTION_WINDOW,
                max_retry: 1,
            },
            StrictFailPolicy,
        );
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        // 第 1 次 hold
        e.apply_fail(FailKind::CouncilHoldFailure, "h1", at_ms())
            .unwrap();
        // 重新提交
        e.submit(at_ms()).unwrap();
        // 第 2 次 hold → budget 耗尽 → Retired
        let outcome = e
            .apply_fail(FailKind::CouncilHoldFailure, "h2", at_ms())
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
        assert!(e.current_state().is_terminal());
    }

    #[test]
    fn engine_out_of_reflection_window_terminates() {
        let mut e: EvolutionEngine = EvolutionEngine::with_config(
            "p-1",
            EngineConfig {
                reflection_window_ms: 0,
                max_retry: 100,
            },
            StrictFailPolicy,
        );
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        // window=0 → 任何 retry 都超窗
        let outcome = e
            .apply_fail(FailKind::CouncilHoldFailure, "h", at_ms() + 1000)
            .unwrap();
        assert_eq!(outcome, FailOutcome::Retired);
    }

    #[test]
    fn engine_log_steps_record_all() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        e.apply_fail(FailKind::CouncilRejectFailure, "r", at_ms())
            .unwrap();
        let log = e.log();
        assert_eq!(log.steps.len(), 3);
        assert!(matches!(log.steps[0], EvolutionStep::Started { .. }));
        assert!(matches!(log.steps[1], EvolutionStep::Submitted { .. }));
        assert!(matches!(log.steps[2], EvolutionStep::Failed { .. }));
    }

    #[test]
    fn engine_propose_patch_uses_traits_impl() {
        let e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        let patch = e.propose_patch(&SystemState::new("L3", 75));
        assert_eq!(patch.target_layer, "L3");
    }

    #[test]
    fn engine_l0_anchor_constant() {
        let e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        assert_eq!(e.l0_anchor(), L0_ANCHOR);
    }

    #[test]
    fn engine_traits_impl_mut_can_extend() {
        use crate::traits::{MockPlugin, Plugin, PluginKind};
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", StrictFailPolicy);
        let p: Box<dyn Plugin> = Box::new(MockPlugin::new("p1", PluginKind::Tool, "ok"));
        e.traits_impl_mut().extend_capability(p).unwrap();
        assert_eq!(e.traits_impl().plugin_count(), 1);
    }

    #[test]
    fn engine_config_default_uses_constants() {
        let c = EngineConfig::default();
        assert_eq!(c.reflection_window_ms, DEFAULT_REFLECTION_WINDOW);
        assert_eq!(c.max_retry, DEFAULT_MAX_RETRY);
    }
}
