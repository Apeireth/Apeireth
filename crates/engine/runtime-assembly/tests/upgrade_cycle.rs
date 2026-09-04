//! Stage 5 集成测试: L0-L5 UpgradeCycle driver 完整跑通 happy path.

use apeireth_runtime_assembly as apeireth_runtime;

use std::sync::Arc;

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::Clock;
use apeireth_core::kernel::SessionId;
use apeireth_governance::{
    AllowAll, Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict,
};
use apeireth_orchestration::{Council, CouncilInvoker, Proposal};
use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::self_assessment::{SelfAssessment, SelfAssessmentStore};
use apeireth_runtime::canonical::orchestrator::{
    LocalOrchestratorRelationship, LocalSovereignty, MockCouncilDecision, MockCouncilInvoker,
    OrchestratorBoundaries, OrchestratorLoopConfig, OrganOrchestrator, SovereigntyGate,
};
use apeireth_runtime::canonical::upgrade_cycle::{
    CycleStep, DefaultTagSuggester, TagSuggester, UpgradeCycle,
};
use async_trait::async_trait;
use chrono::TimeZone;

/// Mock SelfAssessmentStore — 返 Ok(空 vec) (Stage 5 简化: 无 self_assessment 历史 → 默认 Approved).
struct EmptySelfAssessments;

impl SelfAssessmentStore for EmptySelfAssessments {
    fn record(
        &self,
        _sa: &SelfAssessment,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<()> {
        Ok(())
    }
    fn recent_for_task(
        &self,
        _task_id: &str,
        _limit: u32,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<Vec<SelfAssessment>> {
        Ok(Vec::new())
    }
    fn latest_alignment(
        &self,
        _task_id: &str,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<Option<f64>> {
        Ok(None)
    }
}

/// Mock SelfAssessmentStore — 返 Ok(单条低 alignment 0.4 < 0.6 → Rejected).
struct LowAlignmentAssessments;

impl SelfAssessmentStore for LowAlignmentAssessments {
    fn record(
        &self,
        _sa: &SelfAssessment,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<()> {
        Ok(())
    }
    fn recent_for_task(
        &self,
        _task_id: &str,
        _limit: u32,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<Vec<SelfAssessment>> {
        Ok(vec![SelfAssessment {
            id: "sa-1".into(),
            task_id: "test".into(),
            round: 1,
            session_id: SessionId::default(),
            alignment: 0.4, // < 0.6 → Rejected
            quality: 0.5,
            deviations: serde_json::json!([]),
            assessed_at: 0,
            reviewer_id: "test-reviewer".into(),
        }])
    }
    fn latest_alignment(
        &self,
        _task_id: &str,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<Option<f64>> {
        Ok(Some(0.4))
    }
}

/// Mock Organ trait — 返 NotImplemented (Stage 5 简化: 不真接 9 organ)
struct MockOrgan {
    kind: OrganKind,
}

#[async_trait]
impl OrganTrait for MockOrgan {
    fn name(&self) -> &'static str {
        "MockOrgan"
    }
    fn organ_id(&self) -> OrganKind {
        self.kind
    }
    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        Ok(OrganOutput::NotImplemented {
            organ: self.kind,
            note: "mock organ (upgrade_cycle integration test)".to_string(),
        })
    }
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .unwrap(),
    ))
}

fn build_orchestrator() -> OrganOrchestrator<LocalOrchestratorRelationship> {
    let organ_e4: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::E4,
    });
    let organ_f1: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::F1,
    });
    let organ_f4: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::F4,
    });
    let organ_f6: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::F6,
    });
    let organ_w1: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::W1,
    });
    let organ_w2: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::W2,
    });
    let organ_w3: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::W3,
    });
    let organ_e7: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::E7,
    });
    let organ_memory: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::Memory,
    });

    let council = Arc::new(Council::default_allow());
    let council_invoker: Arc<dyn CouncilInvoker> = Arc::new(MockCouncilInvoker::allow_all());
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    let rel = LocalOrchestratorRelationship::new(0.5);

    OrganOrchestrator::new(
        organ_e4,
        organ_f1,
        organ_f4,
        organ_f6,
        organ_w1,
        organ_w2,
        organ_w3,
        organ_e7,
        organ_memory,
        council,
        council_invoker,
        sovereignty,
        rel,
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        clock(),
    )
}

fn build_upgrade_cycle(
    self_assessments: Arc<dyn SelfAssessmentStore>,
) -> UpgradeCycle<LocalOrchestratorRelationship> {
    UpgradeCycle::new(
        Arc::new(build_orchestrator()),
        Arc::new(AllowAll),
        self_assessments,
        Arc::new(DefaultTagSuggester),
        "1.2.0", // current workspace.version (LOCKED 不改, 这是建议来源)
    )
}

fn sample_proposal() -> Proposal {
    Proposal {
        id: "upgrade-cycle-test-1".into(),
        proposer: "apeireth-test".into(),
        payload: serde_json::json!({"action": "upgrade_cycle_test"}),
        submitted_at: chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .unwrap()
            .timestamp(),
        session_id: SessionId::default(),
    }
}

// ============================================
// 测试 1: upgrade_cycle_happy_path_all_pass
// ============================================

/// Happy path: L0-L5 全通过 → final_step = Tagged + tag_suggestion = "1.2.1".
///
/// 验证:
/// - L0 哲学锚 (AllowAll governance) → Approved
/// - L1 self_assessment (EmptySelfAssessments → 无历史 → 默认 Approved)
/// - L2 Orchestrator council (allow_all MockCouncilInvoker) → Approved
/// - L3 9 organ chain (MockOrgan NotImplemented) → Approved (all_present)
/// - L4 governance (AllowAll) → Approved
/// - L5 → Tagged + "1.2.1" tag 建议
#[tokio::test]
async fn upgrade_cycle_happy_path_all_pass() {
    let cycle = build_upgrade_cycle(Arc::new(EmptySelfAssessments));
    let result = cycle.run_full_cycle(sample_proposal()).await;

    assert_eq!(result.layer_outcomes.len(), 6, "L0-L5 6 步骤");
    // 5 steps Approved + L5 Tagged
    assert!(matches!(result.layer_outcomes[0].1, CycleStep::Approved));
    assert!(matches!(result.layer_outcomes[1].1, CycleStep::Approved));
    assert!(matches!(result.layer_outcomes[2].1, CycleStep::Approved));
    assert!(matches!(result.layer_outcomes[3].1, CycleStep::Approved));
    assert!(matches!(result.layer_outcomes[4].1, CycleStep::Approved));
    assert!(matches!(result.layer_outcomes[5].1, CycleStep::Tagged));
    assert_eq!(result.final_step, CycleStep::Tagged, "happy path → Tagged");
    assert_eq!(result.tag_suggestion, "1.2.1", "tag suggestion bumps patch");
    assert!(result.all_passed());
}

// ============================================
// 测试 2: upgrade_cycle_l1_low_alignment_rejected
// ============================================

/// L1 self_assessment alignment < 0.6 → cycle 在 L1 拦下, L2-L5 不跑.
#[tokio::test]
async fn upgrade_cycle_l1_low_alignment_rejected() {
    let cycle = build_upgrade_cycle(Arc::new(LowAlignmentAssessments));
    let result = cycle.run_full_cycle(sample_proposal()).await;

    assert_eq!(result.layer_outcomes.len(), 2, "L0 + L1 only (early stop)");
    assert!(matches!(result.layer_outcomes[0].1, CycleStep::Approved));
    assert!(matches!(result.layer_outcomes[1].1, CycleStep::Rejected));
    assert_eq!(result.final_step, CycleStep::Rejected);
    assert!(!result.all_passed());
    // Rejected tag 建议: "1.2.0-NOT-READY"
    assert!(
        result.tag_suggestion.contains("NOT-READY"),
        "Rejected → NOT-READY, got {}",
        result.tag_suggestion
    );
}

// ============================================
// 测试 3: upgrade_cycle_l2_council_stop_rejected
// ============================================

/// L2 Orchestrator council = Stop → cycle 在 L2 拦下.
#[tokio::test]
async fn upgrade_cycle_l2_council_stop_rejected() {
    let orch = Arc::new(build_orchestrator_with_stop_council());
    let cycle = UpgradeCycle::new(
        orch,
        Arc::new(AllowAll),
        Arc::new(EmptySelfAssessments),
        Arc::new(DefaultTagSuggester),
        "1.2.0",
    );
    let result = cycle.run_full_cycle(sample_proposal()).await;

    assert_eq!(result.layer_outcomes.len(), 3, "L0 + L1 + L2 (L2 stops)");
    assert!(matches!(result.layer_outcomes[2].1, CycleStep::Rejected));
    assert_eq!(result.final_step, CycleStep::Rejected);
}

/// Helper: orchestrator with stop_all MockCouncilInvoker (L2 CouncilDecision::Stop)
fn build_orchestrator_with_stop_council() -> OrganOrchestrator<LocalOrchestratorRelationship> {
    let organ_e4: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::E4,
    });
    let organ_f1: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::F1,
    });
    let organ_f4: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::F4,
    });
    let organ_f6: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::F6,
    });
    let organ_w1: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::W1,
    });
    let organ_w2: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::W2,
    });
    let organ_w3: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::W3,
    });
    let organ_e7: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::E7,
    });
    let organ_memory: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
        kind: OrganKind::Memory,
    });

    let council = Arc::new(Council::default_allow());
    let council_invoker: Arc<dyn CouncilInvoker> = Arc::new(MockCouncilInvoker {
        decision: MockCouncilDecision::StopAll,
    });
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    let rel = LocalOrchestratorRelationship::new(0.5);

    OrganOrchestrator::new(
        organ_e4,
        organ_f1,
        organ_f4,
        organ_f6,
        organ_w1,
        organ_w2,
        organ_w3,
        organ_e7,
        organ_memory,
        council,
        council_invoker,
        sovereignty,
        rel,
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        clock(),
    )
}

// ============================================
// 测试 4: upgrade_cycle_l0_governance_deny_rejected
// ============================================

/// L0 governance Deny → cycle 在 L0 立即拦下 (L0 是硬墙, 但 governance 可配 RequireApproval/Deny).
struct DenyAllGovernance;

#[async_trait]
impl GovernanceHook for DenyAllGovernance {
    fn name(&self) -> &str {
        "deny_all_test"
    }
    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::deny("test deny all")
    }
    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        GovernanceVerdict::new(self.name(), self.evaluate(request).await)
    }
}

#[tokio::test]
async fn upgrade_cycle_l0_governance_deny_rejected() {
    let cycle = UpgradeCycle::new(
        Arc::new(build_orchestrator()),
        Arc::new(DenyAllGovernance),
        Arc::new(EmptySelfAssessments),
        Arc::new(DefaultTagSuggester),
        "1.2.0",
    );
    let result = cycle.run_full_cycle(sample_proposal()).await;

    assert_eq!(result.layer_outcomes.len(), 1, "L0 only (early stop)");
    assert!(matches!(result.layer_outcomes[0].1, CycleStep::Rejected));
    assert_eq!(result.final_step, CycleStep::Rejected);
}

// ============================================
// 测试 5: upgrade_cycle_layer_outcomes_order
// ============================================

/// 验证 layer_outcomes 顺序 = L0 → L5 (per R11 §7).
#[tokio::test]
async fn upgrade_cycle_layer_outcomes_order() {
    use apeireth_runtime::canonical::orchestrator::UpgradeLayer;
    let cycle = build_upgrade_cycle(Arc::new(EmptySelfAssessments));
    let result = cycle.run_full_cycle(sample_proposal()).await;

    assert_eq!(result.layer_outcomes[0].0, UpgradeLayer::L0HumanApproval);
    assert_eq!(result.layer_outcomes[1].0, UpgradeLayer::L1SelfAssessment);
    assert_eq!(
        result.layer_outcomes[2].0,
        UpgradeLayer::L2ProposalGeneration
    );
    assert_eq!(result.layer_outcomes[3].0, UpgradeLayer::L3Verification);
    assert_eq!(result.layer_outcomes[4].0, UpgradeLayer::L4MasterApproval);
    assert_eq!(result.layer_outcomes[5].0, UpgradeLayer::L5RuntimePatch);
}

// ============================================
// 测试 6: upgrade_cycle_default_tag_suggester
// ============================================

/// DefaultTagSuggester 真实行为: bump patch.
#[test]
fn upgrade_cycle_default_tag_suggester() {
    let sug = DefaultTagSuggester;
    assert_eq!(sug.suggest_next_tag("1.2.0", CycleStep::Tagged), "1.2.1");
    assert_eq!(sug.suggest_next_tag("1.2.5", CycleStep::Tagged), "1.2.6");
    assert_eq!(sug.suggest_next_tag("0.0.1", CycleStep::Tagged), "0.0.2");
    // Rejected → NOT-READY suffix
    assert_eq!(
        sug.suggest_next_tag("1.2.0", CycleStep::Rejected),
        "1.2.0-NOT-READY"
    );
    // 不规则 version → fallback
    assert_eq!(sug.suggest_next_tag("abc", CycleStep::Tagged), "abc-next");
}

/// TagSuggester trait object 用 (Stage 5 验证 trait 可对象化).
struct StubTagSuggester;
impl TagSuggester for StubTagSuggester {
    fn suggest_next_tag(&self, _current: &str, _step: CycleStep) -> String {
        "stub-1.0.0".into()
    }
}

#[tokio::test]
async fn upgrade_cycle_tag_suggester_trait_object() {
    let cycle = UpgradeCycle::new(
        Arc::new(build_orchestrator()),
        Arc::new(AllowAll),
        Arc::new(EmptySelfAssessments),
        Arc::new(StubTagSuggester),
        "1.2.0",
    );
    let result = cycle.run_full_cycle(sample_proposal()).await;
    assert_eq!(result.final_step, CycleStep::Tagged);
    assert_eq!(
        result.tag_suggestion, "stub-1.0.0",
        "TagSuggester trait object 工作 (Arc<dyn TagSuggester>)"
    );
}

// unused Episode import workaround
#[allow(dead_code)]
fn _unused_episode_marker() {}
