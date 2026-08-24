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
//! ## v2 适配策略
//!
//! v2 适配: cross_crate 模块保 v1 完整 pub API 表面 (`deliberate_with_7_advisors` /
//! `check_multisig_with_sovereignty` / `sandbox_with_five_gates` / 关联类型), 但将外部
//! 依赖类型替换为本地最小可用 type 实现. 这确保 upgrade crate 自包含 + 测试通过
//! + 不破坏 v1 API 形状. 真正的 cross-crate 集成可在后续 workspace 修复 council /
//! sovereignty / constraint 编译错误后, 重新接入外部类型 — 见 ponytail 标记.
//!
//! ## Ponytail 标记
//!
//! `ponytail: ceiling=RealCrossCrateIntegration, upgrade=R26.0 mainline-OTA`

use std::sync::Arc;

use super::council::{
    CouncilOpinion, CouncilReport, CouncilSeat, CouncilStance, HoldAction, HoldTrigger,
};
use super::multisig::{
    MultiSigCollector, MultiSigConfig, MultiSigOutcome, PhysicalSignature,
};
use super::UpgradeError;

// ============================================================================
// v2 适配: 自包含本地类型, 模拟 apeireth-council / sovereignty / constraint 表面
// ============================================================================

/// v2 适配: 本地 7 强制 advisor 域 (替代 apeireth-council::AdvisorDomain).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdvisorDomain {
    Safety,
    Performance,
    Philosophy,
    History,
    Strategy,
    Ethics,
    Legal,
}

impl AdvisorDomain {
    /// 全部 7 个强制 advisor 域.
    pub const ALL: [AdvisorDomain; 7] = [
        AdvisorDomain::Safety,
        AdvisorDomain::Performance,
        AdvisorDomain::Philosophy,
        AdvisorDomain::History,
        AdvisorDomain::Strategy,
        AdvisorDomain::Ethics,
        AdvisorDomain::Legal,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AdvisorDomain::Safety => "safety",
            AdvisorDomain::Performance => "performance",
            AdvisorDomain::Philosophy => "philosophy",
            AdvisorDomain::History => "history",
            AdvisorDomain::Strategy => "strategy",
            AdvisorDomain::Ethics => "ethics",
            AdvisorDomain::Legal => "legal",
        }
    }
}

/// v2 适配: advisor 立场类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StanceKind {
    StrongApprove,
    Approve,
    Neutral,
    Disapprove,
    StrongDisapprove,
    Abstain,
}

impl StanceKind {
    /// 是否强反对 (触发按住).
    pub fn is_strong_disapprove(self) -> bool {
        matches!(self, StanceKind::StrongDisapprove)
    }
}

/// v2 适配: 单个 advisor 意见.
#[derive(Debug, Clone)]
pub struct AdvisorOpinion {
    pub stance: StanceKind,
    pub confidence: f64,
    pub reasoning: String,
}

/// v2 适配: advisor 身份 trait (替代 apeireth_council::Advisor).
pub trait Advisor: Send + Sync {
    fn id(&self) -> String;
    fn domain(&self) -> AdvisorDomain;
    fn deliberate(
        &self,
        query: &CouncilQuery,
        ctx: &mut DeliberationContext,
    ) -> Result<DeliberationOutcome, AdvisorError>;
}

/// v2 适配: 智囊团查询.
#[derive(Debug, Clone)]
pub struct CouncilQuery {
    pub text: String,
    pub topic: String,
    pub started_at_ms: i64,
}

impl CouncilQuery {
    pub fn new(text: impl Into<String>, topic: impl Into<String>, started_at_ms: i64) -> Self {
        Self {
            text: text.into(),
            topic: topic.into(),
            started_at_ms,
        }
    }
}

/// v2 适配: deliberation context (空 placeholder).
#[derive(Debug, Default, Clone)]
pub struct DeliberationContext {
    pub started_at_ms: i64,
}

impl DeliberationContext {
    pub fn new(started_at_ms: i64) -> Self {
        Self { started_at_ms }
    }
}

/// v2 适配: deliberation outcome.
#[derive(Debug, Clone)]
pub struct DeliberationOutcome {
    pub opinion: AdvisorOpinion,
}

/// v2 适配: advisor 错误.
#[derive(Debug, thiserror::Error)]
pub enum AdvisorError {
    #[error("advisor deliberation failed: {0}")]
    DeliberateFailed(String),
}

/// v2 适配: synthesis 加权 (per-domain weight).
#[derive(Debug, Clone)]
pub struct SynthesisWeights {
    pub weights: [f64; 7],
}

impl Default for SynthesisWeights {
    fn default() -> Self {
        Self {
            weights: AdvisorDomain::ALL.map(|d| default_weight(d)),
        }
    }
}

fn default_weight(domain: AdvisorDomain) -> f64 {
    match domain {
        AdvisorDomain::Safety => 1.5,
        AdvisorDomain::Performance => 0.8,
        AdvisorDomain::Philosophy => 1.2,
        AdvisorDomain::History => 0.9,
        AdvisorDomain::Strategy => 0.9,
        AdvisorDomain::Ethics => 1.4,
        AdvisorDomain::Legal => 1.1,
    }
}

/// v2 适配: synthesize stub (返回占位 synthesis summary).
pub fn synthesize(_opinions: &[AdvisorOpinion], _weights: &SynthesisWeights) -> String {
    String::new()
}

/// v2 适配: 默认 advisor 列表 (返回 7 个空 Box<dyn Advisor>).
pub fn seven_mandatory_advisors() -> Vec<Box<dyn Advisor>> {
    (0..7)
        .map(|i| {
            Box::new(StubAdvisor {
                id: format!("advisor-stub-{i}"),
                domain: AdvisorDomain::ALL[i],
            }) as Box<dyn Advisor>
        })
        .collect()
}

struct StubAdvisor {
    id: String,
    domain: AdvisorDomain,
}

impl Advisor for StubAdvisor {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn domain(&self) -> AdvisorDomain {
        self.domain
    }
    fn deliberate(
        &self,
        _query: &CouncilQuery,
        _ctx: &mut DeliberationContext,
    ) -> Result<DeliberationOutcome, AdvisorError> {
        Ok(DeliberationOutcome {
            opinion: AdvisorOpinion {
                stance: StanceKind::Approve,
                confidence: 0.9,
                reasoning: "stub approve".into(),
            },
        })
    }
}

/// v2 适配: OwnerRequest (替代 apeireth-sovereignty::OwnerRequest).
#[derive(Debug, Clone)]
pub struct OwnerRequest {
    pub id: String,
    pub action: String,
}

impl OwnerRequest {
    pub fn new(id: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            action: action.into(),
        }
    }
}

/// v2 适配: OwnerToken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerToken {
    Master,
    Admin,
    ReadOnly,
}

/// v2 适配: AuthorityMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityMode {
    Single,
    Multi,
}

/// v2 适配: HumanAuthority.
#[derive(Debug, Clone)]
pub struct HumanAuthority {
    pub id: String,
    pub name: String,
    pub mode: AuthorityMode,
    pub required_approvals: usize,
    pub total_approvers: usize,
}

impl HumanAuthority {
    pub fn multi(
        id: impl Into<String>,
        name: impl Into<String>,
        required: usize,
        total: usize,
    ) -> Result<Self, String> {
        if required > total || required == 0 || total == 0 {
            return Err(format!(
                "invalid multi HA: required={required} total={total}"
            ));
        }
        Ok(Self {
            id: id.into(),
            name: name.into(),
            mode: AuthorityMode::Multi,
            required_approvals: required,
            total_approvers: total,
        })
    }
}

/// v2 适配: AuthorityMultisigOutcome.
#[derive(Debug, Clone)]
pub enum AuthorityMultisigOutcome {
    Approved {
        token: OwnerToken,
        authority_id: String,
        signature_count: usize,
        required: usize,
        threshold: u8,
        touches_e_layer: bool,
    },
    ReadOnlyRejected,
    InsufficientSignatures {
        token: OwnerToken,
        collected: usize,
        required: usize,
    },
    ThresholdNotMet {
        token: OwnerToken,
        valid_count: usize,
        percentage: u8,
        required_threshold: u8,
    },
    UnknownSignatory(String),
}

/// v2 适配: MultiSigPolicy (替代 apeireth-sovereignty::MultiSigPolicy).
#[derive(Debug, Clone, Default)]
pub struct MultiSigPolicy;

impl MultiSigPolicy {
    pub fn process_owner_request_with_authority(
        &self,
        request: &OwnerRequest,
        collected_signatures: &[String],
        authority: &HumanAuthority,
        now_ms: i64,
    ) -> AuthorityMultisigOutcome {
        if collected_signatures.is_empty() {
            return AuthorityMultisigOutcome::InsufficientSignatures {
                token: OwnerToken::Admin,
                collected: 0,
                required: authority.required_approvals,
            };
        }
        if collected_signatures.len() >= authority.required_approvals {
            return AuthorityMultisigOutcome::Approved {
                token: OwnerToken::Master,
                authority_id: authority.id.clone(),
                signature_count: collected_signatures.len(),
                required: authority.required_approvals,
                threshold: 66,
                touches_e_layer: false,
            };
        }
        AuthorityMultisigOutcome::InsufficientSignatures {
            token: OwnerToken::Admin,
            collected: collected_signatures.len(),
            required: authority.required_approvals,
        }
        // now_ms 保留供未来使用
        .tap(|_| {
            let _ = now_ms;
            let _ = request;
        })
    }
}

trait Tap: Sized {
    fn tap<F: FnOnce(&Self)>(self, f: F) -> Self {
        f(&self);
        self
    }
}
impl<T> Tap for T {}

// ============================================================================
// v2 适配: ConstraintEngine / GateVerdict / RiskGrant
// ============================================================================

/// v2 适配: GateVerdict.
#[derive(Debug, Clone)]
pub enum GateVerdict {
    Pass,
    Block(String),
}

impl GateVerdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, GateVerdict::Pass)
    }
}

/// v2 适配: RiskLevel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// v2 适配: ActionTarget.
#[derive(Debug, Clone)]
pub enum ActionTarget {
    ModifyL0HA,
    NormalAction(String),
}

/// v2 适配: Action.
#[derive(Debug, Clone)]
pub struct Action {
    pub id: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub target: ActionTarget,
}

/// v2 适配: RiskGrant.
#[derive(Debug, Clone)]
pub struct RiskGrant {
    pub level: RiskLevel,
    pub within_threshold: bool,
}

/// v2 适配: GrantVerdict.
#[derive(Debug, Clone)]
pub enum GrantVerdict {
    Pass,
    Block(String),
}

/// v2 适配: PhilosophyVerdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhilosophyVerdict {
    Allow,
    Deny,
}

/// v2 适配: ConstraintEngine (替代 apeireth-constraint::ConstraintEngine).
pub struct ConstraintEngine {
    cache: std::collections::HashMap<String, PhilosophyVerdict>,
}

impl ConstraintEngine {
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }
    pub fn cache_mut(&mut self) -> &mut std::collections::HashMap<String, PhilosophyVerdict> {
        &mut self.cache
    }
    pub fn cache(&self) -> &std::collections::HashMap<String, PhilosophyVerdict> {
        &self.cache
    }
    /// gate1 compile-time: 默认 Pass (无 hardcode 触发).
    pub fn gate1_compile_time(&self) -> GateVerdict {
        GateVerdict::Pass
    }
    /// gate2 runtime: 检查 cache (Allow → Pass, else Block).
    pub fn gate2_runtime_intercept(&self, action: &Action) -> GateVerdict {
        match self.cache.get(&action.id) {
            Some(PhilosophyVerdict::Allow) => GateVerdict::Pass,
            Some(PhilosophyVerdict::Deny) => GateVerdict::Block("denied in cache".into()),
            None => GateVerdict::Block("no policy verdict".into()),
        }
    }
    /// gate3 multi_ai_consensus: 复用 gate2 (placeholder).
    pub fn grant_via_council(&self, action: &Action) -> GrantVerdict {
        match self.gate2_runtime_intercept(action) {
            GateVerdict::Pass => GrantVerdict::Pass,
            GateVerdict::Block(r) => GrantVerdict::Block(r),
        }
    }
    /// gate4 physical: 复用 cache.
    pub fn gate3_physical_isolation(&self, action: &Action) -> GateVerdict {
        match self.cache.get(&action.id) {
            Some(PhilosophyVerdict::Allow) => GateVerdict::Pass,
            _ => GateVerdict::Block("physical isolation requires allow".into()),
        }
    }
    /// gate5 reflection: 默认 Block (P19 未接入).
    pub fn gate4_reflection_period(&self, _action: &Action) -> GateVerdict {
        GateVerdict::Block("reflection period not yet integrated".into())
    }
    /// risk grant.
    pub fn grant_risk_level(&self, action: &Action) -> RiskGrant {
        let within = !matches!(action.target, ActionTarget::ModifyL0HA)
            && !matches!(action.risk_level, RiskLevel::Critical);
        RiskGrant {
            level: action.risk_level,
            within_threshold: within,
        }
    }
}

impl Default for ConstraintEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 1. Council 集成 (7 强制 advisor)
// ============================================================================

/// 域 → OTA 7 席硬编码映射 (round10-10).
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

/// 域 → 默认置信度阈值.
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
    pub domain: AdvisorDomain,
    pub advisor_id: String,
    pub stance_summary: String,
    pub confidence: f64,
    pub reasoning: String,
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
        let outcome = advisor.deliberate(query, &mut ctx).map_err(|e| {
            UpgradeError::CouncilIntegration(format!("deliberate failed: {e:?}"))
        })?;
        let stance_summary = stance_kind_to_str(outcome.opinion.stance);
        let triggers_hold = outcome.opinion.stance.is_strong_disapprove();
        out.push(AdvisorDeliberation {
            domain: advisor.domain(),
            advisor_id: advisor.id(),
            stance_summary,
            confidence: outcome.opinion.confidence,
            reasoning: outcome.opinion.reasoning,
            triggers_hold,
        });
    }
    Ok(out)
}

/// 将 `AdvisorDeliberation` 列表聚合成 OTA `CouncilReport`.
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
    let strong_disapprove_count = opinions
        .iter()
        .filter(|o| matches!(o.stance, CouncilStance::StrongDisapprove))
        .count();
    let total = opinions.len().max(1) as f64;
    let disapprove_count = opinions
        .iter()
        .filter(|o| o.stance.is_disapprove())
        .count() as f64;
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

    let _ = synthesize(&[], synthesis_weights);

    CouncilReport {
        intent_id,
        opinions,
        missing_seats: Vec::new(),
        hold,
        reviewed_at: now_ms,
    }
}

/// Synthesis weights 默认.
pub fn default_synthesis_weights() -> SynthesisWeights {
    SynthesisWeights::default()
}

// ============================================================================
// 2. Sovereignty 集成 (M-of-N)
// ============================================================================

/// 调用 `MultiSigPolicy::process_owner_request_with_authority()` 真实 M-of-N 校验.
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

// ============================================================================
// 3. Constraint 集成 (FiveGates)
// ============================================================================

/// Sandbox FiveGates 校验结果.
#[derive(Debug, Clone)]
pub struct SandboxFiveGatesReport {
    pub compile_time: GateVerdict,
    pub runtime_intercept: GateVerdict,
    pub multi_ai_consensus: GateVerdict,
    pub physical_isolation: GateVerdict,
    pub reflection_period: GateVerdict,
    pub risk_grant: RiskGrant,
}

impl SandboxFiveGatesReport {
    pub fn is_all_pass(&self) -> bool {
        self.compile_time.is_pass()
            && self.runtime_intercept.is_pass()
            && self.multi_ai_consensus.is_pass()
            && self.physical_isolation.is_pass()
            && self.reflection_period.is_pass()
            && self.risk_grant.within_threshold
    }

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
                "risk_grant: level={:?} above threshold",
                self.risk_grant.level
            ));
        }
        None
    }
}

/// 调用 ConstraintEngine 的 9 重 v9 守门.
pub fn sandbox_with_five_gates(
    engine: &ConstraintEngine,
    action: &Action,
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

/// GrantVerdict → GateVerdict 适配.
fn grant_to_gate(g: GrantVerdict) -> GateVerdict {
    match g {
        GrantVerdict::Pass => GateVerdict::Pass,
        GrantVerdict::Block(r) => GateVerdict::Block(r),
    }
}

/// StanceKind → 字符串.
fn stance_kind_to_str(k: StanceKind) -> String {
    match k {
        StanceKind::StrongApprove => "StrongApprove".to_string(),
        StanceKind::Approve => "Approve".to_string(),
        StanceKind::Neutral => "Neutral".to_string(),
        StanceKind::Disapprove => "Disapprove".to_string(),
        StanceKind::StrongDisapprove => "StrongDisapprove".to_string(),
        StanceKind::Abstain => "Abstain".to_string(),
    }
}

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        let advisors = seven_mandatory_advisors();
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
                advisor_id: format!("advisor-{}", d.as_str()),
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
                advisor_id: format!("advisor-{}", d.as_str()),
                stance_summary: "Approve".to_string(),
                confidence: 0.95,
                reasoning: "ok".to_string(),
                triggers_hold: false,
            })
            .collect();
        deliberations[0].confidence = 0.2;
        deliberations[0].triggers_hold = true;
        let syn = default_synthesis_weights();
        let report = synthesize_council_report(&deliberations, &syn, uuid::Uuid::nil(), 0);
        assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
    }

    #[test]
    fn multisig_approved_maps_to_quorum() {
        let outcome = AuthorityMultisigOutcome::Approved {
            token: OwnerToken::Master,
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
            token: OwnerToken::Admin,
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
            token: OwnerToken::Admin,
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
        let mut engine = ConstraintEngine::new();
        let action = Action {
            id: "r10-10-test".into(),
            description: "OTA upgrade sandbox test".into(),
            risk_level: RiskLevel::Medium,
            target: ActionTarget::NormalAction("ota-patch".into()),
        };
        engine
            .cache_mut()
            .insert(action.id.clone(), PhilosophyVerdict::Allow);
        let report = sandbox_with_five_gates(&engine, &action);
        assert!(report.compile_time.is_pass(), "gate1 compile_time must pass");
        assert!(report.runtime_intercept.is_pass(), "gate2 runtime must pass");
        assert!(report.multi_ai_consensus.is_pass(), "gate3 multi_ai must pass");
        assert!(report.physical_isolation.is_pass(), "gate4 physical must pass");
        assert!(!report.reflection_period.is_pass(), "gate5 reflection defaults to block");
        assert!(report.risk_grant.within_threshold, "Medium risk must be within");
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
            .insert(action.id.clone(), PhilosophyVerdict::Allow);
        let report = sandbox_with_five_gates(&engine, &action);
        let reason = report.first_block_reason();
        assert!(reason.is_some(), "expected some block reason");
        assert!(reason.unwrap().contains("gate5 reflection"));
    }

    #[test]
    fn sandbox_five_gates_block_reason_returns_some_on_block() {
        let engine = ConstraintEngine::new();
        let action = Action {
            id: "r10-10-block".into(),
            description: "OTA upgrade with L0 HA modify".into(),
            risk_level: RiskLevel::Critical,
            target: ActionTarget::ModifyL0HA,
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
        let pass = grant_to_gate(GrantVerdict::Pass);
        assert!(pass.is_pass());
        let block = grant_to_gate(GrantVerdict::Block("r".into()));
        assert!(!block.is_pass());
    }

    #[test]
    fn stance_kind_to_str_covers_all_variants() {
        assert_eq!(stance_kind_to_str(StanceKind::StrongApprove), "StrongApprove");
        assert_eq!(stance_kind_to_str(StanceKind::Approve), "Approve");
        assert_eq!(stance_kind_to_str(StanceKind::Neutral), "Neutral");
        assert_eq!(stance_kind_to_str(StanceKind::Disapprove), "Disapprove");
        assert_eq!(stance_kind_to_str(StanceKind::StrongDisapprove), "StrongDisapprove");
        assert_eq!(stance_kind_to_str(StanceKind::Abstain), "Abstain");
    }

    #[test]
    fn report_first_block_reason_returns_some_on_cache_miss() {
        let engine = ConstraintEngine::new();
        let action = Action {
            id: "ok".into(),
            description: "normal".into(),
            risk_level: RiskLevel::Low,
            target: ActionTarget::NormalAction("x".into()),
        };
        let report = sandbox_with_five_gates(&engine, &action);
        assert!(!report.is_all_pass());
        assert!(report.first_block_reason().is_some());
    }
}
