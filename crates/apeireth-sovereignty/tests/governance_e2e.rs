//! Integration tests — MEWG 5 重治理端到端流程
//!
//! **覆盖清单**:
//! 1. `test_governance_full_approval_path` — 5 重治理全通过 → Approved
//! 2. `test_governance_e_layer_hard_gate_blocks` — E 层修改无 MultiHuman 证据 → Blocked
//! 3. `test_governance_partial_ai_consensus_blocks` — 多 AI 部分一致 → Blocked
//! 4. `test_governance_reflection_pending_blocks` — 反思期未结束 → PendingReview
//! 5. `test_governance_human_reject_blocks` — 1 人 reject → Blocked
//! 6. `test_governance_physical_multisig_rejects_same_kind` — 同 kind 物理设备 → Blocked
//! 7. `test_governance_council_synthesis_collaboration` — council synthesis 协同
//! 8. `test_governance_reflection_default_seven_days` — 默认反思期 7 天

use std::time::Duration;

use apeireth_council::synthesis::SynthesisReport;
use apeireth_council::SovereigntyHook;
use apeireth_sovereignty::{
    AiStance, Decision, Governance, GovernanceOutcome, GovernanceStep, HumanId, MockAiProvider,
    PhysicalSignerId, Vote,
};

fn decision(id: &str, touches_e: bool) -> Decision {
    Decision {
        id: id.into(),
        title: format!("test decision {id}"),
        description: format!("integration test decision {id}"),
        touches_e_layer: touches_e,
        tags: vec!["test".into()],
        submitted_at: 0,
        metadata: None,
    }
}

/// 完整成功路径 helper — 注册 3 AI approve + 2 human approve + 2 physical signature
async fn setup_full_path(gov: &Governance) {
    gov.register_ai_provider(Box::new(MockAiProvider::new(
        "gpt4-mock",
        AiStance::Approve,
    )))
    .await
    .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new(
        "claude-mock",
        AiStance::Approve,
    )))
    .await
    .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new(
        "local-mock",
        AiStance::Approve,
    )))
    .await
    .unwrap();
    {
        let mut v = gov.multi_human.lock().await;
        v.register(HumanId::new("alice", "Alice", "owner"));
        v.register(HumanId::new("bob", "Bob", "co-owner"));
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("bob", Vote::Approve, "yes".to_string())
            .unwrap();
    }
    {
        let mut m = gov.physical.lock().await;
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("p1", "phone", "bob"));
        m.collect_signature("y1", "digest".to_string(), true)
            .unwrap();
        m.collect_signature("p1", "digest".to_string(), false)
            .unwrap();
    }
}

// =====================================================
// §7.1  完整通过路径
// =====================================================

#[tokio::test]
async fn test_governance_full_approval_path() {
    // 反思期长度 = 0 → process() 立即进入 AwaitingResolution
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    setup_full_path(&gov).await;

    let outcome = gov.process(&decision("d1", false)).await.unwrap();
    match outcome {
        GovernanceOutcome::Approved { mewg_score, .. } => {
            assert!(
                mewg_score > 0.0,
                "mewg_score should be positive: {mewg_score}"
            );
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

// =====================================================
// §7.2  E 层硬门槛 (MEWG §8.3)
// =====================================================

#[tokio::test]
async fn test_governance_e_layer_no_human_vote_returns_pending_review() {
    // E 层修改, 但没人投票 → orchestrator 在 MultiHuman 步骤返回 PendingReview
    // (MEWG E 层硬门槛的 unit 测试在 test_mewg_authority_alone_with_e_layer_modification)
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    // AI 一致通过
    gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve)))
        .await
        .unwrap();
    // **不注册人类投票**
    {
        let mut m = gov.physical.lock().await;
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("p1", "phone", "bob"));
        m.collect_signature("y1", "digest".to_string(), true)
            .unwrap();
        m.collect_signature("p1", "digest".to_string(), false)
            .unwrap();
    }

    let outcome = gov.process(&decision("e-mod", true)).await.unwrap();
    match outcome {
        GovernanceOutcome::PendingReview { waiting_at, state } => {
            assert_eq!(waiting_at, GovernanceStep::MultiHuman);
            assert!(state.contains("0/2"));
        }
        other => panic!("expected PendingReview at MultiHuman, got {other:?}"),
    }
}

// =====================================================
// §7.3  多 AI 部分一致
// =====================================================

#[tokio::test]
async fn test_governance_partial_ai_consensus_blocks() {
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    // 2 reject + 1 approve
    gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Reject)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Reject)))
        .await
        .unwrap();

    let outcome = gov.process(&decision("d1", false)).await.unwrap();
    match outcome {
        GovernanceOutcome::Blocked { failed_at, .. } => {
            assert_eq!(failed_at, GovernanceStep::MultiAi);
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
}

// =====================================================
// §7.4  反思期未结束
// =====================================================

#[tokio::test]
async fn test_governance_reflection_pending_blocks() {
    // 反思期 = 7 天 → process() 立即返回 PendingReview
    let gov = Governance::default(); // 默认 7 天
    setup_full_path(&gov).await;

    let outcome = gov.process(&decision("d1", false)).await.unwrap();
    match outcome {
        GovernanceOutcome::PendingReview { waiting_at, state } => {
            assert_eq!(waiting_at, GovernanceStep::Reflection);
            assert!(state.contains("反思期"), "state: {state}");
        }
        other => panic!("expected PendingReview, got {other:?}"),
    }
}

// =====================================================
// §7.5  1 人 reject → Blocked
// =====================================================

#[tokio::test]
async fn test_governance_human_reject_blocks() {
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve)))
        .await
        .unwrap();
    {
        let mut v = gov.multi_human.lock().await;
        v.register(HumanId::new("alice", "Alice", "owner"));
        v.register(HumanId::new("bob", "Bob", "co-owner"));
        v.register(HumanId::new("carol", "Carol", "witness"));
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("bob", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("carol", Vote::Reject, "no".to_string())
            .unwrap();
    }
    {
        let mut m = gov.physical.lock().await;
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("p1", "phone", "bob"));
        m.collect_signature("y1", "digest".to_string(), true)
            .unwrap();
        m.collect_signature("p1", "digest".to_string(), false)
            .unwrap();
    }

    let outcome = gov.process(&decision("d1", false)).await.unwrap();
    match outcome {
        GovernanceOutcome::Blocked { failed_at, reason } => {
            assert_eq!(failed_at, GovernanceStep::MultiHuman);
            assert!(reason.contains("反对"));
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
}

// =====================================================
// §7.6  物理多签同 kind 拒绝
// =====================================================

#[tokio::test]
async fn test_governance_physical_multisig_rejects_same_kind() {
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve)))
        .await
        .unwrap();
    {
        let mut v = gov.multi_human.lock().await;
        v.register(HumanId::new("alice", "Alice", "owner"));
        v.register(HumanId::new("bob", "Bob", "co-owner"));
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("bob", Vote::Approve, "yes".to_string())
            .unwrap();
    }
    // 2 个 yubikey (同 kind)
    {
        let mut m = gov.physical.lock().await;
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("y2", "yubikey", "alice"));
        m.collect_signature("y1", "digest".to_string(), true)
            .unwrap();
        m.collect_signature("y2", "digest".to_string(), true)
            .unwrap();
    }

    let outcome = gov.process(&decision("d1", false)).await.unwrap();
    match outcome {
        GovernanceOutcome::Blocked { failed_at, reason } => {
            assert_eq!(failed_at, GovernanceStep::PhysicalMultisig);
            assert!(reason.contains("kind"));
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
}

// =====================================================
// §7.7  Council synthesis 协同
// =====================================================

#[tokio::test]
async fn test_governance_council_synthesis_collaboration() {
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));

    // 注册 council hook
    let hook = gov.council_hook();
    let event = apeireth_council::CouncilEvent::DeliberationStarted {
        session_id: "s1".into(),
        query_id: "q1".into(),
        started_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    hook.on_council_event(&event);

    // 验证 hook 收到了事件
    let events = gov.council_event_sink.lock().await;
    assert_eq!(events.len(), 1, "hook should have received 1 event");

    // 验证 synthesis_to_evidence 转换 (调用一个合成报告)
    let fake_report = SynthesisReport {
        weighted_score: 0.85,
        aggregated_stance: apeireth_council::advisor::Stance::new(
            apeireth_council::advisor::StanceKind::StrongApprove,
            "test synthesis",
        ),
        confidence: 0.9,
        dissenting: vec![],
        hold_decision: apeireth_council::hold::HoldDecision::released(),
        opinion_count: 7,
    };
    let evidence = gov.synthesis_to_evidence(&fake_report);
    assert_eq!(
        evidence.source,
        apeireth_sovereignty::EvidenceSource::MultiAi
    );
    assert!(evidence.score > 0.0);
    assert!(evidence.weight > 0.0);
}

// =====================================================
// §7.8  反思期默认 7 天
// =====================================================

#[tokio::test]
async fn test_governance_reflection_default_seven_days() {
    use apeireth_sovereignty::reflection::DEFAULT_REFLECTION_PERIOD;
    assert_eq!(
        DEFAULT_REFLECTION_PERIOD,
        Duration::from_secs(7 * 24 * 60 * 60)
    );
    // governance 默认用 7 天
    let gov = Governance::default();
    assert_eq!(gov.reflection_period, DEFAULT_REFLECTION_PERIOD);
}

// =====================================================
// §7.9  MEWG 自身单独测试 (覆盖 5 重治理拆分验证)
// =====================================================

#[tokio::test]
async fn test_mewg_authority_alone_with_e_layer_modification() {
    use apeireth_sovereignty::mewg::{
        Decision, DefaultMewgAuthority, EvidenceSource, MewgAuthority, MewgEvidence,
    };

    let auth = DefaultMewgAuthority::new();
    let d = decision("e-mod", true);

    // 仅 AI 证据 (无 MultiHuman) → E 层硬门槛应 Blocked
    let ai_only = vec![MewgEvidence::new(
        "ai",
        EvidenceSource::MultiAi,
        0.9,
        0.5,
        "ai only".to_string(),
    )
    .unwrap()];
    let verdict = auth.evaluate(&d, &ai_only).unwrap();
    assert!(matches!(
        verdict,
        apeireth_sovereignty::mewg::MewgVerdict::Blocked { .. }
    ));

    // 加上 MultiHuman 证据 → Approved
    let full = vec![
        MewgEvidence::new("ai", EvidenceSource::MultiAi, 0.9, 0.5, "ai".to_string()).unwrap(),
        MewgEvidence::new(
            "h",
            EvidenceSource::MultiHuman,
            0.8,
            0.5,
            "human".to_string(),
        )
        .unwrap(),
    ];
    let verdict = auth.evaluate(&d, &full).unwrap();
    assert!(matches!(
        verdict,
        apeireth_sovereignty::mewg::MewgVerdict::Approved { .. }
    ));
}
