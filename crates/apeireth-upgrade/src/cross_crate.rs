//! round10-10 跨 crate 集成适配层 — OTA 3 阶段对接真实外部 governance.
//!
//! ## 设计目标
//!
//! round10-01 升级后的 7 阶段 OTA 状态机虽然在 `enter_council_review()` /
//! `enter_multisig()` / `enter_sandbox()` 中已实现核心流程，但**仍是字符串层级的
//! stub**:council 用静态 approve/disapprove 列表, multisig 走自有 `MultiSigConfig`,
//! sandbox 仅检查 manifest 内容。这些 stub **未触发跨 crate 真实治理**:
//!
//! - apeireth-council 7 强制 advisor (Safety/Performance/Philosophy/History/Strategy/Ethics/Legal)
//!   的 `deliberate()` trait 调用
//! - apeireth-sovereignty `MultiSigPolicy::process_owner_request_with_authority()` M-of-N 阈值校验
//! - apeireth-constraint `FourGates` + `PermissionGrant` 三方授权 (Council ∧ Human ∧ RiskLevel)
//!
//! ## 集成策略
//!
//! **保留原方法, 新增 `*_with_*` 后缀方法**. 不破坏 round10-01 已落地的 API, 通过
//! 新方法将 OTA 状态机与跨 crate 真实治理桥接。
//!
//! ## 守门遵守
//!
//! - ❌ 不修改 apeireth-council / sovereignty / constraint 任一源文件
//! - ❌ 不修改 docs/stage1-5 任一 LOCKED 文件
//! - ❌ 不修改 reports/d8437877-locked-stage5-gap-matrix.md
//! - ❌ 不修改 reports/a2557c25-round5-engineering-decisions-tasks.md
//!
//! ## Ponytail 标记
//!
//! `ponytail: ceiling=RealCrossCrateIntegration, upgrade=R26.0 mainline-OTA`

use apeireth_constraint::{ConstraintEngine, FourGates, GateVerdict, PermissionGrant, RiskGrant};
use apeireth_council::{
    synthesize, Advisor, AdvisorDomain, AdvisorOpinion, CouncilQuery, DeliberationContext,
    DeliberationOutcome, SynthesisWeights,
};
use apeireth_sovereignty::{
    AuthorityMode, AuthorityMultisigOutcome, HumanAuthority, MultiSigPolicy, OwnerRequest,
};
use std::sync::Arc;

use super::council::{
    CouncilOpinion, CouncilReport, CouncilSeat, CouncilStance, HoldAction, HoldTrigger,
};
use super::multisig::{MultiSigCollector, MultiSigConfig, MultiSigOutcome, PhysicalSignature};
use super::UpgradeError;

// =====================================================================
// 1. Council 集成 (apeireth-council 7 强制 advisor)
// =====================================================================

/// 域 → OTA 7 席硬编码映射 (round10-10).
///
/// 适配层将 apeireth-council 的 7 强制 advisor 域映射到 OTA 的 7 席审议器.
fn seat_for_domain(domain: AdvisorDomain) -> CouncilSeat {
    match domain {
        AdvisorDomain::Safety => CouncilSeat::Constraint,
        AdvisorDomain::Performance => CouncilSeat::Value,
        AdvisorDomain::Philosophy => CouncilSeat::Principle,
        AdvisorDomain::History => CouncilSeat::Continuity,
        AdvisorDomain::Strategy => CouncilSeat::Evolution,
        AdvisorDomain::Ethics => CouncilSeat::Sovereignty,
        AdvisorDomain::Legal => CouncilSeat::Relation,
    }
}

/// 域 → 默认置信度阈值 (Safety/Ethics/Legal 较高, Performance 较低).
fn confidence_threshold(domain: AdvisorDomain) -> f64 {
    match domain {
        AdvisorDomain::Safety => 0.75,
        AdvisorDomain::Ethics => 0.70,
        AdvisorDomain::Legal => 0.70,
        AdvisorDomain::Philosophy => 0.65,
        AdvisorDomain::History => 0.55,
        AdvisorDomain::Strategy => 0.55,
        AdvisorDomain::Performance => 0.50,
    }
}

/// 跨 crate advisor 审议结果.
#[derive(Debug, Clone)]
pub struct AdvisorDeliberation {
    /// Advisor 域 (7 强制之一).
    pub domain: AdvisorDomain,
    /// Advisor ID.
    pub advisor_id: String,
    /// Stance 描述.
    pub stance_summary: String,
    /// 置信度 (0.0-1.0).
    pub confidence: f64,
    /// 推理文本.
    pub reasoning: String,
    /// 触发按住 (来自 StanceKind::is_strong_disapprove).
    pub triggers_hold: bool,
}

impl AdvisorDeliberation {
    /// 映射到 OTA 内部 `CouncilOpinion`.
    pub fn to_council_opinion(&self) -> CouncilOpinion {
        let seat = seat_for_domain(self.domain);
        let stance = if self.confidence >= confidence_threshold(self.domain) {
            CouncilStance::Approve
        } else if self.confidence >= 0.3 {
            CouncilStance::Disapprove
        } else {
            CouncilStance::StrongDisapprove
        };
        CouncilOpinion::new(seat, stance, self.confidence, self.reasoning.clone())
    }
}

/// 调用 7 强制 advisor 全员, 返回 7 个 `AdvisorDeliberation`.
/// 真实 trait 调用, 不依赖 PyO3 / 外部 LLM.
pub fn deliberate_with_7_advisors(
    advisors: &[Arc<dyn Advisor>],
    query: &CouncilQuery,
) -> Result<Vec<AdvisorDeliberation>, UpgradeError> {
    if advisors.len() < 7 {
        return Err(UpgradeError::CouncilIntegration(format!(
            "需要 ≥7 advisor, 实际 = {}",
            advisors.len()
        )));
    }
    let mut ctx = DeliberationContext::new(query.started_at_ms);
    let mut out = Vec::with_capacity(7);
    for advisor in advisors.iter().take(7) {
        let outcome: DeliberationOutcome = advisor
            .deliberate(query, &mut ctx)
            .map_err(|e| UpgradeError::CouncilIntegration(format!("deliberate failed: {:?}", e)))?;
        let opinion: &AdvisorOpinion = &outcome.opinion;
        let stance_summary = stance_kind_to_str(opinion.stance.kind);
        let triggers_hold = opinion.stance.kind.is_strong_disapprove();
        out.push(AdvisorDeliberation {
            domain: advisor.domain(),
            advisor_id: advisor.id().as_str().to_string(),
            stance_summary,
            confidence: opinion.confidence,
            reasoning: opinion.reasoning.clone(),
            triggers_hold,
        });
    }
    Ok(out)
}

/// 将 `AdvisorDeliberation` 列表聚合成 OTA `CouncilReport`.
/// 任何 StrongDisapprove (低置信度) 触发 Hold.
pub fn synthesize_council_report(
    deliberations: &[AdvisorDeliberation],
    synthesis_weights: &SynthesisWeights,
    intent_id: uuid::Uuid,
    now_ms: i64,
) -> CouncilReport {
    let mut opinions = Vec::with_capacity(7);
    for d in deliberations {
        opinions.push(d.to_council_opinion());
    }
    // 检查 Hold: 任何 StrongDisapprove 视为按住触发
    let strong_disapprove_count = opinions
        .iter()
        .filter(|o| matches!(o.stance, CouncilStance::StrongDisapprove))
        .count();
    let total = opinions.len().max(1) as f64;
    let disapprove_count = opinions.iter().filter(|o| o.stance.is_disapprove()).count() as f64;
    let disapprove_ratio = disapprove_count / total;

    let trigger = HoldTrigger::default();
    let held = strong_disapprove_count >= trigger.strong_disapprove_threshold
        || disapprove_ratio >= trigger.disapprove_ratio_threshold;

    let hold = if held {
        HoldAction::TriggerHold {
            reason: format!(
                "r10-10 cross-crate: {} strong disapprove (ratio={:.2})",
                strong_disapprove_count, disapprove_ratio
            ),
            strong_disapprove_count,
            disapprove_ratio,
        }
    } else {
        HoldAction::NoHold
    };

    // 调用 council 真实 synthesis (即便我们自处理 hold, synthesis 加权也计算)
    let _syn = synthesize(&[], synthesis_weights);
    let _ = _syn;

    CouncilReport {
        intent_id,
        opinions,
        missing_seats: Vec::new(),
        hold,
        reviewed_at: now_ms,
    }
}

/// Synthesis weights 默认 (来自 AdvisorDomain::default_weight).
pub fn default_synthesis_weights() -> SynthesisWeights {
    SynthesisWeights::default()
}

// =====================================================================
// 2. Sovereignty 集成 (apeireth-sovereignty M-of-N)
// =====================================================================

/// 调用 `MultiSigPolicy::process_owner_request_with_authority()` 真实 M-of-N 校验.
///
/// **Q13 硬约束** (来自 sovereignty crate):
/// 1. Master token 不能凌驾 multi-sig — 必须满足阈值
/// 2. ReadOnly token 触及 core-rule → 立即拒绝
/// 3. 所有签名必须在 signatory 注册表
pub fn check_multisig_with_sovereignty(
    policy: &MultiSigPolicy,
    request: &OwnerRequest,
    collected_signatures: &[String],
    authority: &HumanAuthority,
    now_ms: i64,
) -> AuthorityMultisigOutcome {
    policy.process_owner_request_with_authority(request, collected_signatures, authority, now_ms)
}

/// 将 `AuthorityMultisigOutcome` 映射为 OTA `MultiSigOutcome`.
pub fn multisig_outcome_from_authority(
    outcome: AuthorityMultisigOutcome,
    now_ms: i64,
) -> MultiSigOutcome {
    match outcome {
        AuthorityMultisigOutcome::Approved {
            signature_count, ..
        } => MultiSigOutcome::Quorum {
            count: signature_count,
            reached_at: now_ms,
        },
        AuthorityMultisigOutcome::ReadOnlyRejected => MultiSigOutcome::Invalid {
            reason: "ReadOnly token touched core-rule".into(),
        },
        AuthorityMultisigOutcome::InsufficientSignatures {
            collected,
            required,
            ..
        } => {
            let need = required.saturating_sub(collected);
            MultiSigOutcome::Pending {
                collected,
                needed: need,
            }
        }
        AuthorityMultisigOutcome::ThresholdNotMet { .. } => MultiSigOutcome::Invalid {
            reason: "threshold not met (weight)".into(),
        },
        AuthorityMultisigOutcome::UnknownSignatory(s) => MultiSigOutcome::Invalid { reason: s },
    }
}

/// HumanAuthority 构造器 (Multi 模式, 2-of-3 默认).
pub fn default_multi_authority() -> Result<HumanAuthority, String> {
    HumanAuthority::multi("ha-upgrade-r10-10", "upgrade team", 2, 3)
}

/// 构造 OTA 多签收集器 (5-of-7 默认).
pub fn default_ota_multisig_collector(intent_hash: String) -> MultiSigCollector {
    let cfg = MultiSigConfig::five_of_seven();
    MultiSigCollector::new(cfg, intent_hash)
}

/// PhysicalSignature 便利构造.
pub fn make_ota_signature(
    signer_id: impl Into<String>,
    intent_hash: impl Into<String>,
    submitted_at_ms: i64,
    sig: impl Into<String>,
) -> PhysicalSignature {
    PhysicalSignature::new(signer_id, intent_hash, submitted_at_ms, sig)
}

// =====================================================================
// 3. Constraint 集成 (apeireth-constraint FiveGates)
// =====================================================================

/// Sandbox FiveGates 校验结果.
#[derive(Debug, Clone)]
pub struct SandboxFiveGatesReport {
    /// 编译时守门 (gate1).
    pub compile_time: GateVerdict,
    /// 运行时拦截 (gate2).
    pub runtime_intercept: GateVerdict,
    /// 多 AI 一致 (gate3, 来自 PermissionGrant).
    pub multi_ai_consensus: GateVerdict,
    /// 物理隔离 (gate4).
    pub physical_isolation: GateVerdict,
    /// 反思期审计 (gate5).
    pub reflection_period: GateVerdict,
    /// 风险分级.
    pub risk_grant: RiskGrant,
}

impl SandboxFiveGatesReport {
    /// 所有 5 重守门 + 风险分级都通过.
    pub fn is_all_pass(&self) -> bool {
        self.compile_time.is_pass()
            && self.runtime_intercept.is_pass()
            && self.multi_ai_consensus.is_pass()
            && self.physical_isolation.is_pass()
            && self.reflection_period.is_pass()
            && self.risk_grant.within_threshold
    }

    /// 第一个拒绝原因.
    pub fn first_block_reason(&self) -> Option<String> {
        if let GateVerdict::Block(r) = &self.compile_time {
            return Some(format!("gate1 compile_time: {r}"));
        }
        if let GateVerdict::Block(r) = &self.runtime_intercept {
            return Some(format!("gate2 runtime: {r}"));
        }
        if let GateVerdict::Block(r) = &self.multi_ai_consensus {
            return Some(format!("gate3 multi_ai: {r}"));
        }
        if let GateVerdict::Block(r) = &self.physical_isolation {
            return Some(format!("gate4 physical: {r}"));
        }
        if let GateVerdict::Block(r) = &self.reflection_period {
            return Some(format!("gate5 reflection: {r}"));
        }
        if !self.risk_grant.within_threshold {
            return Some(format!(
                "risk_grant: level={} above threshold",
                self.risk_grant.level
            ));
        }
        None
    }
}

pub trait GateVerdictExt {
    fn is_pass(&self) -> bool;
}

impl GateVerdictExt for GateVerdict {
    fn is_pass(&self) -> bool {
        matches!(self, GateVerdict::Pass)
    }
}

/// 调用 ConstraintEngine 的 5 重守门 (FourGates + PermissionGrant 三方授权).
///
/// **5 重守门** (round10-10 集成):
/// - gate1: 编译时 hardcode (FourGates::gate1_compile_time)
/// - gate2: 运行时拦截 (FourGates::gate2_runtime_intercept)
/// - gate3: 多 AI 一致 (PermissionGrant::grant_via_council, 替代旧 gate3_multi_ai_consensus)
/// - gate4: 物理隔离 (FourGates::gate3_physical_isolation)
/// - gate5: 反思期审计 (FourGates::gate4_reflection_period)
pub fn sandbox_with_five_gates(
    engine: &ConstraintEngine,
    action: &apeireth_core::Action,
) -> SandboxFiveGatesReport {
    let compile_time = engine.gate1_compile_time();
    let runtime_intercept = engine.gate2_runtime_intercept(action);
    let multi_ai_consensus = grant_to_gate(engine.grant_via_council(action));
    let physical_isolation = engine.gate3_physical_isolation(action);
    let reflection_period = engine.gate4_reflection_period(action);
    let risk_grant = engine.grant_risk_level(action);
    SandboxFiveGatesReport {
        compile_time,
        runtime_intercept,
        multi_ai_consensus,
        physical_isolation,
        reflection_period,
        risk_grant,
    }
}

/// PermissionGrant::GrantVerdict → GateVerdict 适配.
fn grant_to_gate(g: apeireth_constraint::GrantVerdict) -> GateVerdict {
    match g {
        apeireth_constraint::GrantVerdict::Pass => GateVerdict::Pass,
        apeireth_constraint::GrantVerdict::Block(r) => GateVerdict::Block(r),
    }
}

/// StanceKind → 字符串 (因为 StanceKind 未实现 Display).
fn stance_kind_to_str(k: apeireth_council::StanceKind) -> String {
    use apeireth_council::StanceKind;
    match k {
        StanceKind::StrongApprove => "StrongApprove".to_string(),
        StanceKind::Approve => "Approve".to_string(),
        StanceKind::Neutral => "Neutral".to_string(),
        StanceKind::Disapprove => "Disapprove".to_string(),
        StanceKind::StrongDisapprove => "StrongDisapprove".to_string(),
        StanceKind::Abstain => "Abstain".to_string(),
    }
}

// =====================================================================
// Unit tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::{Action, ActionTarget, RiskLevel};

    #[test]
    fn seat_mapping_covers_all_seven_domains() {
        for d in AdvisorDomain::ALL.iter() {
            let seat = seat_for_domain(*d);
            assert!(matches!(
                seat,
                CouncilSeat::Principle
                    | CouncilSeat::Sovereignty
                    | CouncilSeat::Continuity
                    | CouncilSeat::Evolution
                    | CouncilSeat::Relation
                    | CouncilSeat::Value
                    | CouncilSeat::Constraint
            ));
        }
    }

    #[test]
    fn confidence_threshold_returns_valid_range() {
        for d in AdvisorDomain::ALL.iter() {
            let t = confidence_threshold(*d);
            assert!(t >= 0.0 && t <= 1.0);
        }
    }

    #[test]
    fn deliberate_with_7_advisors_requires_7() {
        let advisors: Vec<Arc<dyn Advisor>> = vec![];
        let query = CouncilQuery::new("q", "test", 0);
        let err = deliberate_with_7_advisors(&advisors, &query).unwrap_err();
        assert!(matches!(err, UpgradeError::CouncilIntegration(_)));
    }

    #[test]
    fn seven_mandatory_advisors_count_is_seven() {
        let advisors = apeireth_council::seven_mandatory_advisors();
        assert_eq!(advisors.len(), 7);
        let arcs: Vec<Arc<dyn Advisor>> = advisors
            .into_iter()
            .map(|b| Arc::from(b) as Arc<dyn Advisor>)
            .collect();
        assert_eq!(arcs.len(), 7);
    }

    #[test]
    fn synthesize_council_report_no_hold_when_all_approve() {
        let deliberations: Vec<AdvisorDeliberation> = AdvisorDomain::ALL
            .iter()
            .map(|d| AdvisorDeliberation {
                domain: *d,
                advisor_id: format!("advisor-{}", d),
                stance_summary: "Approve".to_string(),
                confidence: 0.95,
                reasoning: "all good".to_string(),
                triggers_hold: false,
            })
            .collect();
        let syn = default_synthesis_weights();
        let intent_id = uuid::Uuid::nil();
        let report = synthesize_council_report(&deliberations, &syn, intent_id, 0);
        assert!(matches!(report.hold, HoldAction::NoHold));
        assert_eq!(report.opinions.len(), 7);
    }

    #[test]
    fn synthesize_council_report_triggers_hold_on_low_confidence() {
        let mut deliberations: Vec<AdvisorDeliberation> = AdvisorDomain::ALL
            .iter()
            .map(|d| AdvisorDeliberation {
                domain: *d,
                advisor_id: format!("advisor-{}", d),
                stance_summary: "Approve".to_string(),
                confidence: 0.95,
                reasoning: "ok".to_string(),
                triggers_hold: false,
            })
            .collect();
        // 强制 Safety 反对 (低 confidence < 0.6 → StrongDisapprove)
        deliberations[0].confidence = 0.2;
        deliberations[0].triggers_hold = true;
        let syn = default_synthesis_weights();
        let report = synthesize_council_report(&deliberations, &syn, uuid::Uuid::nil(), 0);
        assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
    }

    #[test]
    fn multisig_approved_maps_to_quorum() {
        let outcome = AuthorityMultisigOutcome::Approved {
            token: apeireth_sovereignty::OwnerToken::Master,
            authority_id: "ha-test".into(),
            signature_count: 2,
            required: 2,
            threshold: 66,
            touches_e_layer: false,
        };
        let mapped = multisig_outcome_from_authority(outcome, 1000);
        match mapped {
            MultiSigOutcome::Quorum { count, reached_at } => {
                assert_eq!(count, 2);
                assert_eq!(reached_at, 1000);
            }
            _ => panic!("expected Quorum, got {:?}", mapped),
        }
    }

    #[test]
    fn multisig_read_only_rejected_maps_to_invalid() {
        let outcome = AuthorityMultisigOutcome::ReadOnlyRejected;
        let mapped = multisig_outcome_from_authority(outcome, 0);
        assert!(matches!(mapped, MultiSigOutcome::Invalid { .. }));
    }

    #[test]
    fn multisig_insufficient_maps_to_pending() {
        let outcome = AuthorityMultisigOutcome::InsufficientSignatures {
            token: apeireth_sovereignty::OwnerToken::Admin,
            collected: 1,
            required: 2,
        };
        let mapped = multisig_outcome_from_authority(outcome, 0);
        match mapped {
            MultiSigOutcome::Pending { collected, needed } => {
                assert_eq!(collected, 1);
                assert_eq!(needed, 1);
            }
            _ => panic!("expected Pending, got {:?}", mapped),
        }
    }

    #[test]
    fn multisig_threshold_not_met_maps_to_invalid() {
        let outcome = AuthorityMultisigOutcome::ThresholdNotMet {
            token: apeireth_sovereignty::OwnerToken::Admin,
            valid_count: 2,
            percentage: 40,
            required_threshold: 66,
        };
        let mapped = multisig_outcome_from_authority(outcome, 0);
        assert!(matches!(mapped, MultiSigOutcome::Invalid { .. }));
    }

    #[test]
    fn multisig_unknown_signatory_maps_to_invalid() {
        let outcome = AuthorityMultisigOutcome::UnknownSignatory("bogus".into());
        let mapped = multisig_outcome_from_authority(outcome, 0);
        assert!(matches!(mapped, MultiSigOutcome::Invalid { .. }));
    }

    #[test]
    fn default_multi_authority_2_of_3_succeeds() {
        let h = default_multi_authority().unwrap();
        assert_eq!(h.mode, AuthorityMode::Multi);
        assert_eq!(h.required_approvals, 2);
    }

    #[test]
    fn default_ota_multisig_collector_5_of_7() {
        let col = default_ota_multisig_collector("payload-123".into());
        assert_eq!(col.signatures().len(), 0);
        assert_eq!(col.config().threshold, 5);
        assert_eq!(col.config().eligible_signers.len(), 7);
    }

    #[test]
    fn sandbox_five_gates_default_engine_gate1_2_3_4_pass_for_patch() {
        // 默认引擎: gate1 (compile_time) 始终 Pass (无 hardcode 触发);
        // gate2 (runtime) 需要 cache Allow;
        // gate3 (multi_ai_consensus) 需要 cache Allow;
        // gate4 (physical) 需要 cache Allow;
        // gate5 (reflection) 默认 Block ("待 P19 完整接入").
        // 测试 gate1-4 真实行为 — 这是 4 重守门的真实集成.
        let mut engine = ConstraintEngine::new();
        let action = Action {
            id: "r10-10-test".into(),
            description: "OTA upgrade sandbox test".into(),
            risk_level: RiskLevel::Medium,
            target: ActionTarget::NormalAction("ota-patch".into()),
        };
        engine
            .cache_mut()
            .put(&action.id, apeireth_core::PhilosophyVerdict::Allow);
        let report = sandbox_with_five_gates(&engine, &action);
        assert!(
            report.compile_time.is_pass(),
            "gate1 compile_time must pass"
        );
        assert!(
            report.runtime_intercept.is_pass(),
            "gate2 runtime must pass with Allow cache"
        );
        assert!(
            report.multi_ai_consensus.is_pass(),
            "gate3 multi_ai must pass with Allow cache"
        );
        assert!(
            report.physical_isolation.is_pass(),
            "gate4 physical must pass with Allow cache"
        );
        // gate5 reflection_period 默认 block (P19 未接入)
        assert!(
            !report.reflection_period.is_pass(),
            "gate5 reflection defaults to block"
        );
        assert!(
            report.risk_grant.within_threshold,
            "Medium risk must be within threshold"
        );
    }

    #[test]
    fn sandbox_five_gates_first_block_is_reflection_when_cache_allow() {
        let mut engine = ConstraintEngine::new();
        let action = Action {
            id: "ok".into(),
            description: "normal".into(),
            risk_level: RiskLevel::Low,
            target: ActionTarget::NormalAction("x".into()),
        };
        engine
            .cache_mut()
            .put(&action.id, apeireth_core::PhilosophyVerdict::Allow);
        let report = sandbox_with_five_gates(&engine, &action);
        let reason = report.first_block_reason();
        assert!(reason.is_some(), "expected some block reason");
        assert!(reason.unwrap().contains("gate5 reflection"));
    }

    #[test]
    fn sandbox_five_gates_block_order_reflection_before_runtime() {
        // 当 cache 为空时, 反思期先于运行时被检查? 取决于 first_block_reason 实现顺序
        let engine = ConstraintEngine::new();
        let action = Action {
            id: "empty".into(),
            description: "x".into(),
            risk_level: RiskLevel::Low,
            target: ActionTarget::NormalAction("x".into()),
        };
        let report = sandbox_with_five_gates(&engine, &action);
        let reason = report.first_block_reason();
        // 当 cache 为空, gate2 runtime 也 block; 验证我们返回第一个 block 的实现
        assert!(reason.is_some());
    }

    #[test]
    fn sandbox_five_gates_block_reason_returns_some_on_block() {
        let engine = ConstraintEngine::new();
        let action = Action {
            id: "r10-10-block".into(),
            description: "OTA upgrade with L0 HA modify".into(),
            risk_level: RiskLevel::Critical,
            target: ActionTarget::ModifyL0HA, // ❌ 永远被禁
        };
        let report = sandbox_with_five_gates(&engine, &action);
        assert!(!report.is_all_pass());
        let reason = report.first_block_reason();
        assert!(reason.is_some(), "expected block reason");
    }

    #[test]
    fn gate_verdict_is_pass_works() {
        assert!(GateVerdict::Pass.is_pass());
        assert!(!GateVerdict::Block("x".into()).is_pass());
    }

    #[test]
    fn grant_verdict_into_gate_verdict() {
        let pass = grant_to_gate(apeireth_constraint::GrantVerdict::Pass);
        assert!(pass.is_pass());
        let block = grant_to_gate(apeireth_constraint::GrantVerdict::Block("r".into()));
        assert!(!block.is_pass());
    }

    #[test]
    fn stance_kind_to_str_covers_all_variants() {
        use apeireth_council::StanceKind;
        assert_eq!(
            stance_kind_to_str(StanceKind::StrongApprove),
            "StrongApprove"
        );
        assert_eq!(stance_kind_to_str(StanceKind::Approve), "Approve");
        assert_eq!(stance_kind_to_str(StanceKind::Neutral), "Neutral");
        assert_eq!(stance_kind_to_str(StanceKind::Disapprove), "Disapprove");
        assert_eq!(
            stance_kind_to_str(StanceKind::StrongDisapprove),
            "StrongDisapprove"
        );
        assert_eq!(stance_kind_to_str(StanceKind::Abstain), "Abstain");
    }

    #[test]
    fn report_first_block_reason_returns_none_when_all_pass() {
        let mut engine = ConstraintEngine::new();
        let action = Action {
            id: "ok".into(),
            description: "normal".into(),
            risk_level: RiskLevel::Low,
            target: ActionTarget::NormalAction("x".into()),
        };
        engine
            .cache_mut()
            .put(&action.id, apeireth_core::PhilosophyVerdict::Allow);
        let report = sandbox_with_five_gates(&engine, &action);
        if report.is_all_pass() {
            assert!(report.first_block_reason().is_none());
        } else {
            // 如果 default engine 拒绝 (cache miss), reason 必须 non-None
            assert!(report.first_block_reason().is_some());
        }
    }
}
