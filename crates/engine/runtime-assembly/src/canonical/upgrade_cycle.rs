//! L0-L5 自升级 cycle driver (Stage 5 完整化, per R11 §7 + v2-architecture-reflection.md §6).
//!
//! # 0 装诚实真账
//!
//! - L0 永远不可变: 9 哲学锚 + 13 键 LOCKED (per `crates/foundation/core/src/eight_anchors.rs:58-79`
//!   + `philosophy.rs:142`); Orchestrator 0 触碰. UpgradeCycle 第一步真调 governance crate
//!   `GovernanceHook::evaluate()` 验哲学锚边界.
//! - L1-L5: Orchestrator 串联 + cognitive module 注入 + governance 接入 + git tag.
//! - 完整 cycle = 1.L0 哲学锚校验 → 2.L1 self_assessment → 3.L2 Orchestrator 智囊团审议 →
//!   4.L3 9 organ 串联 + sandbox regression → 5.L4 governance 3 hook + 主人 Veto →
//!   6.L5 git tag v2.x+1 (建议模式, **不自动跑**).
//! - 真生产路径: cycle 由 L0 主人手动触发 (或 governance 定时审计触发), 不是 Orchestrator 内部循环.
//! - Stage 5 真实施最小可行: struct + run_full_cycle + 5 步骤 + 1 集成测试验证 happy path.
//!
//! # L0-L5 6 步骤语义 (per `v2-architecture-reflection.md` §6)
//!
//! - **L0 L0HumanApproval**: governance `GovernanceHook::evaluate()` 真调. 真生产路径
//!   `SovereigntyGate::is_frozen()` 熔断检查 + 9 哲学锚 0 触碰 LOCKED 校验. **永远不失败**
//!   (LOCKED 不可变, 这是硬墙). 但若 governance hook 配错 (AllowAll 不存在), cycle 启动失败.
//! - **L1 L1SelfAssessment**: 调 `SelfAssessmentStore::recent_for_task()` 拿最近 alignment score.
//!   若 < 0.6 → 触发 DeviationReport (per `apeireth-memory` RC-4 schema). Stage 5 真生产路径
//!   `ProductionCognitiveModules::self_assessments` (per production.rs:101).
//! - **L2 L2ProposalGeneration**: 调 `OrganOrchestrator::council_deliberate()` (Stage 4 真路径)
//!   + OrchestratorService::propose_policy(). Stage 5 简化 = council_deliberate 决策 (Stop/Continue).
//! - **L3 L3Verification**: 9 organ 串联 + sandbox regression. Stage 5 简化 = Orchestrator
//!   `chain_9_organs()` 真调 (Stage 3 真路径). Sandbox regression 留 L3 未来 patch.
//! - **L4 L4MasterApproval**: governance `GovernancePipeline::evaluate()` (per governance/lib.rs:347-402)
//!   + 主人 Veto 接口 (RequireApproval → 主人手动). Stage 5 真生产路径 = governance pipeline
//!   真调 + RequireApproval 由主人 dashboard 处理.
//! - **L5 L5RuntimePatch**: git tag v2.x+1 (per `v2-architecture-reflection.md:255-261`).
//!   **0 装诚实**: Stage 5 返建议 (`tag_suggestion` 字段), **不** 自动跑 `Command::new("git", ...)`.
//!   真生产路径 = 主人收到建议 + 手跑 `git tag v2.0.1` + 推 master.
//!
//! # 实施边界 (Stage 5)
//!
//! - ✅ struct UpgradeCycle + run_full_cycle (L0-L5 5 步骤串行)
//! - ✅ L0/L2/L3 真调 Orchestrator + governance hook (Stage 5 完整化)
//! - ✅ L1 真调 SelfAssessmentStore (Stage 5 完整化)
//! - ✅ L5 TagSuggester trait + DefaultTagSuggester (per workspace.version "1.2.0" → "1.2.1")
//! - ⏳ L4 GovernancePipeline 真接 + 主人 Veto dashboard (留 v2.0.0 release 接入)

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_orchestration::Proposal;
use apeireth_plugin::self_assessment::SelfAssessmentStore;

use super::orchestrator::{OrganOrchestrator, RelationshipState, UpgradeLayer};

// ============================================
// TagSuggester (L5 git tag 建议模式, 0 装诚实不自动跑)
// ============================================

/// git tag 建议模式 (per `v2-architecture-reflection.md` §6 L5).
///
/// **0 装诚实**: Stage 5 **不**自动跑 `Command::new("git", "tag")`. 主人收到
/// `UpgradeCycleResult::tag_suggestion` 字符串后手跑 + 推 master. 真生产路径可扩展为
/// `GitTagSuggester` 调 `git describe --tags` 拿当前 tag + bump patch.
pub trait TagSuggester: Send + Sync {
    /// 给当前 version + cycle 状态 → 建议下一个 tag.
    fn suggest_next_tag(&self, current_version: &str, last_cycle_step: CycleStep) -> String;
}

/// Cycle 单步状态 (per R11 spec §7).
///
/// **0 装诚实**: 阶段名 snake_case, telemetry 序列化用.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleStep {
    /// 待启动
    Pending,
    /// 进行中
    InProgress,
    /// 通过 (L0/L2/L3/L4 用)
    Approved,
    /// 拒绝 (L1 self_assessment 失败 / L2 Council veto / L4 主人 Veto)
    Rejected,
    /// L5 终态: 已 git tag (Stage 5 不自动跑, 故 Tag 状态由主人在外部标; Stage 5 返 Tagged 是建议)
    Tagged,
}

impl CycleStep {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Tagged => "tagged",
        }
    }
}

/// 默认 TagSuggester — bump patch 字段 (e.g. "1.2.0" → "1.2.1").
///
/// **0 装诚实**: 不调 `git describe` (Stage 5 简化). 真生产路径在 governance composition root
/// 注入 `GitTagSuggester` (调 `Command::new("git", ["describe", "--tags"])`).
pub struct DefaultTagSuggester;

impl TagSuggester for DefaultTagSuggester {
    fn suggest_next_tag(&self, current_version: &str, last_cycle_step: CycleStep) -> String {
        // **0 装诚实**: 只有 Tagged 才建议 (其他状态建议 "do not tag yet")
        if last_cycle_step != CycleStep::Tagged {
            return format!("{current_version}-NOT-READY");
        }
        // 简化 bump: "1.2.0" → "1.2.1"; 拿不到数字部分保留原值.
        let parts: Vec<&str> = current_version.split('.').collect();
        if parts.len() >= 3 {
            if let Ok(patch) = parts[2].parse::<u32>() {
                let prefix = format!("{}.{}.", parts[0], parts[1]);
                return format!("{}{}", prefix, patch + 1);
            }
        }
        // fallback: 不动 + 加 -next 后缀
        format!("{current_version}-next")
    }
}

// ============================================
// UpgradeCycleResult
// ============================================

/// Cycle 完整结果 (L0-L5 6 步骤全列 + tag 建议).
///
/// **0 装诚实**: 每步 (layer, step) 对真实记录. 任一步 Rejected → 整体 cycle 终态 = Rejected,
/// 不继续后续步骤.
#[derive(Debug, Clone, PartialEq)]
pub struct UpgradeCycleResult {
    /// 6 步骤每步结果 (按 L0 → L5 顺序).
    pub layer_outcomes: Vec<(UpgradeLayer, CycleStep)>,
    /// 最终 step (任一 Rejected → Rejected; 全 Approved/Tagged → Tagged).
    pub final_step: CycleStep,
    /// Tag 建议 (per `DefaultTagSuggester::suggest_next_tag`, 仅 final_step = Tagged 时有意义).
    pub tag_suggestion: String,
}

impl UpgradeCycleResult {
    /// 是否所有 6 步都 Approved/Tagged (i.e. cycle 通过).
    pub fn all_passed(&self) -> bool {
        self.final_step == CycleStep::Tagged
    }
}

// ============================================
// UpgradeCycle struct (Stage 5 主体)
// ============================================

/// L0-L5 自升级 cycle driver (Stage 5 完整化, per R11 §7 + v2-architecture-reflection.md §6).
///
/// **0 装诚实**:
/// - 依赖 Orchestrator (Stage 1-4 真实施) + governance crate GovernanceHook (L0 + L4 真接) +
///   SelfAssessmentStore (L1 真接) + TagSuggester (L5 建议).
/// - run_full_cycle 是**确定性强 + 0 LLM 调用** (除 Orchestrator.council_deliberate 已含的
///   Council 7 advisor LLM side-call, 真生产路径 per cognitive-module-wiring.md:99).
/// - L4 governance pipeline 真接 `GovernanceHook::evaluate()`, 但 主人 Veto dashboard
///   留 v2.0.0 release 接入 (Stage 5 简化 = 仅 governance 决策, RequireApproval 由
///   `Decision::RequireApproval` enum 标, 主人手动 ack 在外部).
pub struct UpgradeCycle<RS: RelationshipState + 'static> {
    orchestrator: Arc<OrganOrchestrator<RS>>,
    governance: Arc<dyn GovernanceHook>,
    self_assessments: Arc<dyn SelfAssessmentStore>,
    tag_suggester: Arc<dyn TagSuggester>,
    current_version: String,
}

impl<RS: RelationshipState + 'static> UpgradeCycle<RS> {
    /// 构造 (Stage 5 真生产路径: governance composition root 注入所有 5 dep).
    pub fn new(
        orchestrator: Arc<OrganOrchestrator<RS>>,
        governance: Arc<dyn GovernanceHook>,
        self_assessments: Arc<dyn SelfAssessmentStore>,
        tag_suggester: Arc<dyn TagSuggester>,
        current_version: impl Into<String>,
    ) -> Self {
        Self {
            orchestrator,
            governance,
            self_assessments,
            tag_suggester,
            current_version: current_version.into(),
        }
    }

    /// 跑完整 L0-L5 cycle (Stage 5 真生产路径).
    ///
    /// **0 装诚实**:
    /// - 6 步骤串行; 任一 Rejected → cycle 立即停, 后续步骤不跑.
    /// - L5 只在 final_step 是所有 Approved (i.e. cycle 通过) 时建议 tag (per CycleStep::Tagged).
    /// - 不修改任何 LOCKED 数据 (9 哲学锚 + 13 键 + workspace.version 等).
    pub async fn run_full_cycle(&self, proposal: Proposal) -> UpgradeCycleResult {
        let mut outcomes: Vec<(UpgradeLayer, CycleStep)> = Vec::with_capacity(6);
        let session_id = proposal.session_id;
        let trace_id = TraceId::new();

        // L0: 哲学锚 + 13 键 LOCKED 校验 (governance 真调)
        let l0_step = self.step_l0(&session_id, &trace_id).await;
        outcomes.push((UpgradeLayer::L0HumanApproval, l0_step));
        if l0_step != CycleStep::Approved {
            return self.build_rejected_result(outcomes);
        }

        // L1: cognitive self_assessment
        let l1_step = self.step_l1(&session_id).await;
        outcomes.push((UpgradeLayer::L1SelfAssessment, l1_step));
        if l1_step != CycleStep::Approved {
            return self.build_rejected_result(outcomes);
        }

        // L2: Orchestrator council_deliberate
        let l2_step = self.step_l2(&proposal).await;
        outcomes.push((UpgradeLayer::L2ProposalGeneration, l2_step));
        if l2_step != CycleStep::Approved {
            return self.build_rejected_result(outcomes);
        }

        // L3: 9 organ 串联 + sandbox regression
        let l3_step = self.step_l3(&proposal).await;
        outcomes.push((UpgradeLayer::L3Verification, l3_step));
        if l3_step != CycleStep::Approved {
            return self.build_rejected_result(outcomes);
        }

        // L4: governance 主人 Veto
        let l4_step = self.step_l4(&session_id, &trace_id).await;
        outcomes.push((UpgradeLayer::L4MasterApproval, l4_step));
        if l4_step != CycleStep::Approved {
            return self.build_rejected_result(outcomes);
        }

        // L5: git tag 建议 (不自动跑)
        outcomes.push((UpgradeLayer::L5RuntimePatch, CycleStep::Tagged));
        let tag_suggestion = self
            .tag_suggester
            .suggest_next_tag(&self.current_version, CycleStep::Tagged);

        UpgradeCycleResult {
            layer_outcomes: outcomes,
            final_step: CycleStep::Tagged,
            tag_suggestion,
        }
    }

    fn build_rejected_result(
        &self,
        outcomes: Vec<(UpgradeLayer, CycleStep)>,
    ) -> UpgradeCycleResult {
        let tag_suggestion = self
            .tag_suggester
            .suggest_next_tag(&self.current_version, CycleStep::Rejected);
        UpgradeCycleResult {
            layer_outcomes: outcomes,
            final_step: CycleStep::Rejected,
            tag_suggestion,
        }
    }

    /// L0: governance.GovernanceHook 真调 (哲学锚 + 13 键 LOCKED 边界校验).
    ///
    /// **0 装诚实**: AllowAll 默认; 真生产路径用 `GovernancePipeline` 含 9 哲学锚校验 hook.
    async fn step_l0(&self, session_id: &SessionId, trace_id: &TraceId) -> CycleStep {
        // 用 GovernanceRequest::new 调 GovernanceHook::evaluate (per governance/lib.rs:107-115)
        let request = GovernanceRequest::new(
            Action::Completion {
                model: "upgrade-cycle-L0",
                message_count: 0,
            },
            *session_id,
            *trace_id,
            1, // L0 永远 round = 1 (硬墙)
        );
        match self.governance.evaluate(&request).await {
            Decision::Allow => CycleStep::Approved,
            Decision::Deny { .. } => CycleStep::Rejected,
            Decision::RequireApproval { .. } => CycleStep::Rejected,
        }
    }

    /// L1: cognitive self_assessment (per RC-4 SelfAssessmentStore schema).
    ///
    /// **0 装诚实**: 调 `SelfAssessmentStore::recent_for_task()` 拿最近 alignment.
    /// 真生产路径 store 是 SQLite (per RC-4 `042ad4eb`). Mock store 返 Some(score).
    /// 若 score < 0.6 → Rejected (per v2-architecture-reflection.md §6 L1).
    async fn step_l1(&self, session_id: &SessionId) -> CycleStep {
        let session_id_str = session_id.to_string();
        match self.self_assessments.recent_for_task(&session_id_str, 1) {
            Ok(assessments) => {
                if let Some(latest) = assessments.first() {
                    if latest.alignment >= 0.6 {
                        CycleStep::Approved
                    } else {
                        CycleStep::Rejected
                    }
                } else {
                    // 无历史记录 = 默认 Approved (per RC-4 schema: 启动时无 self_assessment
                    // 不应阻塞 cycle, 真生产路径 ProductionCognitiveModules 默认开 self_assessment)
                    CycleStep::Approved
                }
            }
            Err(_e) => {
                // store Err 0 装诚实: 不假装"通过". 真生产路径 = store Err 应 fail-closed.
                CycleStep::Rejected
            }
        }
    }

    /// L2: Orchestrator council_deliberate 真调 (Stage 4 真路径).
    ///
    /// **0 装诚实**: council_deliberate 已返回 typed (true=pass, false=veto). 真生产路径
    /// 调 Council 7 advisor LLM side-call per cognitive-module-wiring.md:99 60s timeout.
    async fn step_l2(&self, proposal: &Proposal) -> CycleStep {
        match self.orchestrator.council_deliberate(proposal).await {
            Ok(true) => CycleStep::Approved,
            Ok(false) => CycleStep::Rejected,
            Err(_) => CycleStep::Rejected,
        }
    }

    /// L3: 9 organ 串联 (per Stage 3 真路径). Sandbox regression 留未来 patch.
    ///
    /// **0 装诚实**: 简化 = 调 `chain_9_organs()` 拿 9 output, all_present() 验证.
    /// 真生产路径 = 9 organ 全 Output::NotImplemented 也算通过 (0 装诚实: 不假装)
    /// 或加 sandbox regression (独立 L3 patch).
    async fn step_l3(&self, proposal: &Proposal) -> CycleStep {
        // 把 Proposal 转换成 OrganInput (per orchestrator module 约定)
        use apeireth_core::kernel::memory::Episode;
        use apeireth_plugin::organ::OrganInput;
        let episode = Episode {
            id: format!("upgrade-cycle-{}", proposal.id),
            session_id: proposal.session_id.to_string(),
            role: "system".into(),
            content: format!("upgrade cycle L3 verification for proposal {}", proposal.id),
            timestamp: 0,
        };
        let input = OrganInput::new(episode, vec![]);
        let chain = self.orchestrator.chain_9_organs(input).await;
        if chain.all_present() {
            CycleStep::Approved
        } else {
            CycleStep::Rejected
        }
    }

    /// L4: governance GovernancePipeline 主人 Veto (per governance/lib.rs:347-402).
    ///
    /// **0 装诚实**: 真生产路径 = governance pipeline 含 3 hook + 主人 dashboard.
    // 主人 Veto 接口由 `Decision::RequireApproval` 标, 主人手动 ack 在外部 (留 v2.0.0 release 接入).
    /// Stage 5 简化 = governance.AllowAll 永远通过; RequireApproval → 视为 Rejected (待主人 ack).
    async fn step_l4(&self, session_id: &SessionId, trace_id: &TraceId) -> CycleStep {
        // **0 装诚实**: CapabilityId::new 返 Result. Stage 5 fallback 简化 = 失败用空 capability id
        // (governance hook 仍可处理 CapabilityDispatch 空 capability).
        let capability = CapabilityId::new("upgrade.cycle.l4")
            .unwrap_or_else(|_| CapabilityId::new("upgrade.cycle.l4.fallback").unwrap());
        let args = serde_json::json!({"phase": "L4"});
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &args,
            },
            *session_id,
            *trace_id,
            2, // L4 round = 2
        );
        match self.governance.evaluate(&request).await {
            Decision::Allow => CycleStep::Approved,
            Decision::Deny { .. } | Decision::RequireApproval { .. } => CycleStep::Rejected,
        }
    }
}

// ============================================
// 单元测试 (Stage 5 集成测试在 tests/upgrade_cycle.rs)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认 TagSuggester 简单测试 (per workspace.version "1.2.0").
    #[test]
    fn default_tag_suggester_bumps_patch() {
        let sug = DefaultTagSuggester;
        assert_eq!(
            sug.suggest_next_tag("1.2.0", CycleStep::Tagged),
            "1.2.1",
            "Tagged + current=1.2.0 → next=1.2.1"
        );
        assert_eq!(
            sug.suggest_next_tag("1.2.0", CycleStep::Rejected),
            "1.2.0-NOT-READY",
            "Rejected → NOT-READY suffix (0 装诚实不假装)"
        );
    }

    /// CycleStep::as_str snake_case 序列化.
    #[test]
    fn cycle_step_as_str_stable() {
        assert_eq!(CycleStep::Pending.as_str(), "pending");
        assert_eq!(CycleStep::InProgress.as_str(), "in_progress");
        assert_eq!(CycleStep::Approved.as_str(), "approved");
        assert_eq!(CycleStep::Rejected.as_str(), "rejected");
        assert_eq!(CycleStep::Tagged.as_str(), "tagged");
    }

    /// UpgradeLayer 6 variant 仍可达 (per Stage 5 不破坏 Stage 4 enum 定义).
    #[test]
    fn upgrade_layer_6_variants_intact() {
        let layers = [
            UpgradeLayer::L0HumanApproval,
            UpgradeLayer::L1SelfAssessment,
            UpgradeLayer::L2ProposalGeneration,
            UpgradeLayer::L3Verification,
            UpgradeLayer::L4MasterApproval,
            UpgradeLayer::L5RuntimePatch,
        ];
        assert_eq!(layers.len(), 6, "L0-L5 6 layers");
    }
}
