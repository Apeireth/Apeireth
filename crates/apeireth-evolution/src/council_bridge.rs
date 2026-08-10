//! 与 `apeireth-council` 的集成桥 — 接收 `CouncilEvent` + 翻译成演化动作
//!
//! **设计**: `CouncilAdapter` 接收 `CouncilEvent`, 翻译成对 `EvolutionEngine` 的调用;
//! 集成测试无需运行真实 council, 通过手工构造 `AdvisorOpinion` 数组 + `HoldTrigger`
//! + 模拟 verdict 来覆盖 fail-6 完整路径.

use crate::engine::{EvolutionEngine, EvolutionStep};
use crate::fail::{FailKind, FailOutcome};
use crate::{EvolutionError, EvolutionResult};
use apeireth_council::{
    synthesis::SynthesisReport, AdvisorDomain, AdvisorId, AdvisorOpinion, CouncilEvent,
    CouncilVerdict, HoldOutcome, HoldThreshold, HoldTrigger, Stance, StanceKind,
};
use serde::{Deserialize, Serialize};

/// 默认最大 retry 轮次 (与 council MAX_PERSONA_DEBATE_ROUNDS=3 留余量)。
pub const DEFAULT_MAX_RETRY_ROUNDS: u32 = 3;

/// 默认反思窗口 (与 council HOLD_DELIBERATION_TIMEOUT_MS=60_000 一致)。
pub const DEFAULT_REFLECTION_WINDOW_MS: u64 = 60_000;

/// 演化提案描述 (提交给 council 前的载体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionProposal {
    /// 提案 ID
    pub proposal_id: String,
    /// 描述
    pub description: String,
    /// 目标层
    pub target_layer: String,
    /// 风险等级
    pub risk: String,
}

impl EvolutionProposal {
    /// 便利构造。
    pub fn new(
        proposal_id: impl Into<String>,
        description: impl Into<String>,
        target_layer: impl Into<String>,
        risk: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            description: description.into(),
            target_layer: target_layer.into(),
            risk: risk.into(),
        }
    }

    /// 是否触及 L0 (硬件锚定层)。
    pub fn targets_l0(&self) -> bool {
        self.target_layer.eq_ignore_ascii_case("L0")
    }
}

/// CouncilAdapter 集成配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouncilIntegrationConfig {
    /// 最大 retry 轮次
    pub max_retry: u32,
    /// 反思窗口 (ms)
    pub reflection_window_ms: u64,
}

impl Default for CouncilIntegrationConfig {
    fn default() -> Self {
        Self {
            max_retry: DEFAULT_MAX_RETRY_ROUNDS,
            reflection_window_ms: DEFAULT_REFLECTION_WINDOW_MS,
        }
    }
}

/// 演化裁决产出 (CouncilAdapter 翻译完 verdict 后返回)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionOutcome {
    /// Council 通过 (Proposed → Ratified)
    Ratified,
    /// Council 按住 → retry 一次 (Proposed → Draft)
    Retried {
        /// retry 计数
        attempt: u32,
    },
    /// Council 拒绝 (Proposed → Retired)
    Rejected {
        /// 原因
        reason: String,
    },
    /// 主权仲裁完成 (按住解除)
    SovereigntyAdjudicated {
        /// 是否放行
        released: bool,
    },
    /// L0 防护触发的直接终态
    L0Guard,
    /// 其他 council event (DeliberationStarted / OpinionIssued / DeliberationCompleted) — 忽略.
    Ignored,
}

/// CouncilAdapter — 把 CouncilEvent 翻译为 EvolutionEngine 操作。
pub struct CouncilAdapter<'a> {
    engine: &'a mut EvolutionEngine,
    config: CouncilIntegrationConfig,
}

impl<'a> CouncilAdapter<'a> {
    /// 创建 adapter (借用 engine)。
    pub fn new(engine: &'a mut EvolutionEngine, config: CouncilIntegrationConfig) -> Self {
        Self { engine, config }
    }

    /// 借用 config (只读)。
    pub fn config(&self) -> &CouncilIntegrationConfig {
        &self.config
    }

    /// 处理一次 council verdict (核心入口)。
    ///
    /// **流程**:
    /// 1. verdict.report.is_held() == true → 按住 → CouncilHold → retry
    /// 2. verdict.is_allowed() == true → 通过 → Ratified
    /// 3. verdict.is_rejected() == true → 拒绝 → Rejected → Retired
    pub fn handle_council_verdict(
        &mut self,
        verdict: &CouncilVerdict,
        at_ms: i64,
    ) -> EvolutionResult<EvolutionOutcome> {
        // 先检查 L0 防护 (若 proposal 描述触 L0)
        if self.engine.l0_anchor() != "L0-HARDWARE-ANCHOR" {
            return Err(EvolutionError::L0ModificationRejected);
        }

        if verdict.report.is_held() {
            // 按住 → retry
            let outcome = self.engine.apply_fail(
                FailKind::CouncilHoldFailure,
                format!("council hold (verdict held=true)"),
                at_ms,
            )?;
            match outcome {
                FailOutcome::RetriedToDraft { attempt } => {
                    Ok(EvolutionOutcome::Retried { attempt })
                }
                FailOutcome::Retired => Ok(EvolutionOutcome::Retried {
                    attempt: self.engine.retry_count(),
                }),
            }
        } else if verdict.is_allowed() {
            self.engine.mark_ratified(at_ms)?;
            Ok(EvolutionOutcome::Ratified)
        } else if verdict.is_rejected() {
            self.engine
                .apply_fail(FailKind::CouncilRejectFailure, "council reject", at_ms)?;
            Ok(EvolutionOutcome::Rejected {
                reason: format!(
                    "weighted={:.2}, held={}",
                    verdict.report.weighted_score, verdict.held
                ),
            })
        } else {
            // 中性 (neutral) → 拒绝 (默认保守)
            self.engine.apply_fail(
                FailKind::CouncilRejectFailure,
                "council neutral → reject",
                at_ms,
            )?;
            Ok(EvolutionOutcome::Rejected {
                reason: "neutral verdict treated as reject".into(),
            })
        }
    }

    /// 处理任意 CouncilEvent (含 verdict 之外的语义事件)。
    pub fn handle_event(
        &mut self,
        event: &CouncilEvent,
        at_ms: i64,
    ) -> EvolutionResult<EvolutionOutcome> {
        match event {
            CouncilEvent::DeliberationStarted { .. } => Ok(EvolutionOutcome::Ignored),
            CouncilEvent::OpinionIssued { .. } => Ok(EvolutionOutcome::Ignored),
            CouncilEvent::DeliberationCompleted { report, .. } => {
                // 把 synthesis report 翻译成伪 verdict (走 handle_council_verdict 路径)
                let pseudo = build_verdict_from_report(event, report);
                self.handle_council_verdict(&pseudo, at_ms)
            }
            CouncilEvent::HoldTriggered { .. } => {
                // 按住事件 → 直接走 hold failure
                let outcome = self.engine.apply_fail(
                    FailKind::CouncilHoldFailure,
                    "hold triggered event",
                    at_ms,
                )?;
                Ok(match outcome {
                    FailOutcome::RetriedToDraft { attempt } => {
                        EvolutionOutcome::Retried { attempt }
                    }
                    FailOutcome::Retired => EvolutionOutcome::Rejected {
                        reason: "hold retries exhausted".into(),
                    },
                })
            }
            CouncilEvent::SovereigntyAdjudicated {
                released,
                rationale,
                ..
            } => {
                if *released {
                    Ok(EvolutionOutcome::SovereigntyAdjudicated { released: true })
                } else {
                    self.engine.apply_fail(
                        FailKind::CouncilRejectFailure,
                        rationale.clone(),
                        at_ms,
                    )?;
                    Ok(EvolutionOutcome::SovereigntyAdjudicated { released: false })
                }
            }
        }
    }

    /// L0 防护: 把 proposal 提交前先 guard。
    pub fn guard_proposal(
        &mut self,
        proposal: &EvolutionProposal,
        at_ms: i64,
    ) -> EvolutionResult<bool> {
        if proposal.targets_l0() {
            self.engine.guard_l0(proposal.target_layer.clone(), at_ms)?;
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

/// 由 DeliberationCompleted + SynthesisReport 构造伪 CouncilVerdict
/// (避免在 adapter 内部重新走 deliberation.rs 私有路径)。
///
/// **公开**: 供 tests/ 与 examples/ 复用, 不依赖 Council 内部实现细节。
pub fn build_verdict_from_report(event: &CouncilEvent, report: &SynthesisReport) -> CouncilVerdict {
    // 取 session_id 与 elapsed_ms (DeliberationCompleted 才有 elapsed_ms)
    let (session_id, query_id, elapsed_ms) = match event {
        CouncilEvent::DeliberationCompleted {
            session_id,
            report: _,
            elapsed_ms,
        } => (session_id.clone(), String::new(), *elapsed_ms),
        _ => (String::new(), String::new(), 0),
    };

    CouncilVerdict {
        query_id,
        session_id,
        report: report.clone(),
        elapsed_ms,
        held: report.is_held(),
        hold_outcome: if report.is_held() {
            Some(HoldOutcome::ReflectionStarted {
                reason: "adjudicated by synthesis".into(),
                started_at_ms: 0,
            })
        } else {
            None
        },
    }
}

// ============================================================
// Test helpers — 构造 mock opinion/verdict (公开, 供 tests/ 复用)
// ============================================================

/// 构造单一 opinion (test 辅助)。
pub fn opinion(
    domain: AdvisorDomain,
    stance_kind: StanceKind,
    confidence: f64,
    description: impl Into<String>,
    timestamp_ms: i64,
) -> AdvisorOpinion {
    let mut op = AdvisorOpinion::new(
        AdvisorId::new(format!("{}-v1", domain)),
        Stance::new(stance_kind, description),
        confidence,
        "",
        timestamp_ms,
    );
    op.weight = domain.default_weight();
    op
}

/// 构造 7 强制 advisor 集合 (全 approve) 的合成报告。
pub fn all_approve_report(timestamp_ms: i64) -> SynthesisReport {
    synthesize_with(
        &AdvisorDomain::ALL
            .map(|d| opinion(d, StanceKind::StrongApprove, 0.9, "approve", timestamp_ms)),
        timestamp_ms,
    )
}

/// 构造含按住的合成报告 (1 个 strong disapprove, 其余 approve)。
pub fn held_report(timestamp_ms: i64) -> SynthesisReport {
    let opinions: Vec<AdvisorOpinion> = AdvisorDomain::ALL
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let kind = if i < 3 {
                StanceKind::StrongDisapprove
            } else {
                StanceKind::StrongApprove
            };
            opinion(*d, kind, 0.9, "test", timestamp_ms)
        })
        .collect();
    synthesize_with(&opinions, timestamp_ms)
}

/// 构造 reject 报告 (净加权为负, 但不触发按住 — 用 4 disapprove + 3 approve,
/// 强反对占比 = 4/7 ≈ 57% ≥ 30% 但 adapter 仍按 is_allowed==false 处理时
/// 走 held 路径, 这里改为不调用 HoldTrigger 入口的纯 synthesis).
pub fn reject_report(timestamp_ms: i64) -> SynthesisReport {
    // 4 个 Disapprove (非 Strong) + 3 个 Approve → non-unanimous, score < 0
    let opinions: Vec<AdvisorOpinion> = AdvisorDomain::ALL
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let kind = if i < 4 {
                StanceKind::Disapprove
            } else {
                StanceKind::Approve
            };
            opinion(*d, kind, 0.9, "reject", timestamp_ms)
        })
        .collect();
    synthesize_with(&opinions, timestamp_ms)
}

/// 直接合成报告 (test 辅助)。
pub fn synthesize_with(opinions: &[AdvisorOpinion], timestamp_ms: i64) -> SynthesisReport {
    use apeireth_council::{synthesize, SynthesisWeights};
    // 注入权重
    let weighted: Vec<AdvisorOpinion> = opinions
        .iter()
        .map(|op| {
            let mut w = op.clone();
            if w.weight <= 0.0 {
                w.weight = w
                    .advisor_id
                    .as_str()
                    .split('-')
                    .next()
                    .and_then(|name| match name {
                        "safety" => Some(AdvisorDomain::Safety.default_weight()),
                        "performance" => Some(AdvisorDomain::Performance.default_weight()),
                        "philosophy" => Some(AdvisorDomain::Philosophy.default_weight()),
                        "history" => Some(AdvisorDomain::History.default_weight()),
                        "strategy" => Some(AdvisorDomain::Strategy.default_weight()),
                        "ethics" => Some(AdvisorDomain::Ethics.default_weight()),
                        "legal" => Some(AdvisorDomain::Legal.default_weight()),
                        _ => None,
                    })
                    .unwrap_or(1.0);
            }
            w
        })
        .collect();
    let _ = timestamp_ms; // 留位, 真实 synthesize 不依赖时间
    synthesize(&weighted, &SynthesisWeights::default())
}

/// 触发超时按住 (test 辅助)。
pub fn timeout_trigger(actual_ms: u64) -> HoldTrigger {
    HoldTrigger::evaluate_timeout(actual_ms).expect("should trigger timeout")
}

/// 30% 强反对触发按住 (test 辅助)。
pub fn strong_disapprove_trigger(opinions: &[AdvisorOpinion]) -> HoldTrigger {
    HoldTrigger::evaluate(opinions).expect("should trigger hold")
}

/// HoldThreshold 序列化测试 (test 辅助)。
pub fn describe_threshold(t: &HoldThreshold) -> &'static str {
    match t {
        HoldThreshold::StrongDisapprovePercent { .. } => "StrongDisapprovePercent",
        HoldThreshold::UnanimousDisapprove { .. } => "UnanimousDisapprove",
        HoldThreshold::DeliberationTimeout { .. } => "DeliberationTimeout",
    }
}

/// 序列化以测试 EvolutionStep 字段。
pub fn step_label(step: &EvolutionStep) -> &'static str {
    match step {
        EvolutionStep::Started { .. } => "Started",
        EvolutionStep::Submitted { .. } => "Submitted",
        EvolutionStep::Ratified { .. } => "Ratified",
        EvolutionStep::Activated { .. } => "Activated",
        EvolutionStep::Abandoned { .. } => "Abandoned",
        EvolutionStep::Retired { .. } => "Retired",
        EvolutionStep::Failed { .. } => "Failed",
        EvolutionStep::L0GuardTriggered { .. } => "L0Guard",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_ms() -> i64 {
        crate::current_time_ms()
    }

    #[test]
    fn proposal_targets_l0_recognises() {
        assert!(EvolutionProposal::new("p", "d", "L0", "low").targets_l0());
        assert!(!EvolutionProposal::new("p", "d", "L3", "low").targets_l0());
    }

    #[test]
    fn proposal_default_fields() {
        let p = EvolutionProposal::new("p1", "evolve", "L3", "medium");
        assert_eq!(p.proposal_id, "p1");
        assert_eq!(p.target_layer, "L3");
        assert_eq!(p.risk, "medium");
        assert!(!p.targets_l0());
    }

    #[test]
    fn integration_config_default_uses_constants() {
        let c = CouncilIntegrationConfig::default();
        assert_eq!(c.max_retry, DEFAULT_MAX_RETRY_ROUNDS);
        assert_eq!(c.reflection_window_ms, DEFAULT_REFLECTION_WINDOW_MS);
    }

    #[test]
    fn opinion_helper_sets_weight() {
        let op = opinion(
            AdvisorDomain::Safety,
            StanceKind::StrongApprove,
            0.9,
            "ok",
            at_ms(),
        );
        assert_eq!(op.weight, AdvisorDomain::Safety.default_weight());
    }

    #[test]
    fn all_approve_report_is_allowed() {
        let r = all_approve_report(at_ms());
        assert!(!r.is_held());
        assert!(r.weighted_score > 0.0);
    }

    #[test]
    fn held_report_is_held() {
        let r = held_report(at_ms());
        assert!(r.is_held());
    }

    #[test]
    fn reject_report_is_negative() {
        let r = reject_report(at_ms());
        assert!(r.weighted_score < 0.0);
    }

    #[test]
    fn timeout_trigger_fires() {
        let t = timeout_trigger(70_000);
        assert!(matches!(
            t.threshold,
            HoldThreshold::DeliberationTimeout { .. }
        ));
    }

    #[test]
    fn strong_disapprove_trigger_fires() {
        let opinions: Vec<AdvisorOpinion> = AdvisorDomain::ALL
            .iter()
            .take(3)
            .map(|d| opinion(*d, StanceKind::StrongDisapprove, 0.9, "x", at_ms()))
            .collect();
        let t = strong_disapprove_trigger(&opinions);
        assert!(matches!(
            t.threshold,
            HoldThreshold::StrongDisapprovePercent { .. }
        ));
    }

    #[test]
    fn adapter_handle_allowed_verdict_ratifies() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();

        let report = all_approve_report(at_ms());
        let verdict = build_verdict_from_report(
            &CouncilEvent::DeliberationCompleted {
                session_id: "s-1".into(),
                report: report.clone(),
                elapsed_ms: 100,
            },
            &report,
        );

        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let outcome = adapter.handle_council_verdict(&verdict, at_ms()).unwrap();
        assert_eq!(outcome, EvolutionOutcome::Ratified);
        assert_eq!(e.current_state(), crate::state::EvolutionState::Ratified);
    }

    #[test]
    fn adapter_handle_held_verdict_retries() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();

        let report = held_report(at_ms());
        let verdict = build_verdict_from_report(
            &CouncilEvent::DeliberationCompleted {
                session_id: "s-1".into(),
                report: report.clone(),
                elapsed_ms: 100,
            },
            &report,
        );

        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let outcome = adapter.handle_council_verdict(&verdict, at_ms()).unwrap();
        assert!(matches!(outcome, EvolutionOutcome::Retried { .. }));
        assert_eq!(e.current_state(), crate::state::EvolutionState::Draft);
    }

    #[test]
    fn adapter_handle_rejected_verdict_retires() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();

        let report = reject_report(at_ms());
        let verdict = build_verdict_from_report(
            &CouncilEvent::DeliberationCompleted {
                session_id: "s-1".into(),
                report: report.clone(),
                elapsed_ms: 100,
            },
            &report,
        );

        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let outcome = adapter.handle_council_verdict(&verdict, at_ms()).unwrap();
        assert!(matches!(outcome, EvolutionOutcome::Rejected { .. }));
        assert!(e.current_state().is_terminal());
    }

    #[test]
    fn adapter_guard_proposal_rejects_l0() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        e.start(at_ms()).unwrap();
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());

        let bad = EvolutionProposal::new("p-1", "touch L0", "L0", "high");
        let ok = adapter.guard_proposal(&bad, at_ms()).unwrap();
        assert!(!ok);
        assert!(e.current_state().is_terminal());
    }

    #[test]
    fn adapter_guard_proposal_allows_l3() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        e.start(at_ms()).unwrap();
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());

        let good = EvolutionProposal::new("p-1", "L3 evolve", "L3", "low");
        let ok = adapter.guard_proposal(&good, at_ms()).unwrap();
        assert!(ok);
        assert_eq!(e.current_state(), crate::state::EvolutionState::Draft);
    }

    #[test]
    fn adapter_handle_event_started_is_ignored() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let event = CouncilEvent::DeliberationStarted {
            session_id: "s".into(),
            query_id: "q".into(),
            started_at_ms: 0,
        };
        let outcome = adapter.handle_event(&event, at_ms()).unwrap();
        assert_eq!(outcome, EvolutionOutcome::Ignored);
    }

    #[test]
    fn adapter_handle_event_sovereignty_released() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let event = CouncilEvent::SovereigntyAdjudicated {
            session_id: "s".into(),
            released: true,
            rationale: "released by owner".into(),
        };
        let outcome = adapter.handle_event(&event, at_ms()).unwrap();
        assert_eq!(
            outcome,
            EvolutionOutcome::SovereigntyAdjudicated { released: true }
        );
    }

    #[test]
    fn adapter_handle_event_sovereignty_not_released() {
        let mut e: EvolutionEngine = EvolutionEngine::new("p-1", crate::fail::StrictFailPolicy);
        e.start(at_ms()).unwrap();
        e.submit(at_ms()).unwrap();
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let event = CouncilEvent::SovereigntyAdjudicated {
            session_id: "s".into(),
            released: false,
            rationale: "denied".into(),
        };
        let outcome = adapter.handle_event(&event, at_ms()).unwrap();
        assert_eq!(
            outcome,
            EvolutionOutcome::SovereigntyAdjudicated { released: false }
        );
        assert!(e.current_state().is_terminal());
    }

    #[test]
    fn step_label_matches_all_variants() {
        let now = at_ms();
        let steps = vec![
            EvolutionStep::Started { at_ms: now },
            EvolutionStep::Submitted { at_ms: now },
            EvolutionStep::Ratified { at_ms: now },
            EvolutionStep::Activated { at_ms: now },
            EvolutionStep::Abandoned {
                reason: "x".into(),
                at_ms: now,
            },
            EvolutionStep::Retired {
                reason: "x".into(),
                at_ms: now,
            },
            EvolutionStep::Failed {
                kind: FailKind::ReflectionFailure,
                description: "x".into(),
                retry_attempt: 0,
                outcome: FailOutcome::Retired,
                at_ms: now,
            },
            EvolutionStep::L0GuardTriggered {
                target: "L0".into(),
                at_ms: now,
            },
        ];
        let labels: Vec<&str> = steps.iter().map(step_label).collect();
        assert_eq!(
            labels,
            vec![
                "Started",
                "Submitted",
                "Ratified",
                "Activated",
                "Abandoned",
                "Retired",
                "Failed",
                "L0Guard"
            ]
        );
    }

    #[test]
    fn threshold_describe_matches_all_variants() {
        let t1 = HoldThreshold::StrongDisapprovePercent {
            actual_percent: 50,
            threshold: 30,
        };
        let t2 = HoldThreshold::UnanimousDisapprove { opposing_count: 7 };
        let t3 = HoldThreshold::DeliberationTimeout {
            actual_ms: 60_000,
            threshold_ms: 60_000,
        };
        assert_eq!(describe_threshold(&t1), "StrongDisapprovePercent");
        assert_eq!(describe_threshold(&t2), "UnanimousDisapprove");
        assert_eq!(describe_threshold(&t3), "DeliberationTimeout");
    }
}
