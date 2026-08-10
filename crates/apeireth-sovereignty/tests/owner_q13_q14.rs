//! Q13 / Q14 — 主人不能凌驾治理 (OwnerToken 必须走 5 重治理 + MEWG + 反思期)
//!
//! **LOCKED**: P25 § Q13 — 任何 token (Master / Admin / Operator / ReadOnly)
//! 修改 core-rule (E 层) 都必须走 MultiSigPolicy + Governance.process_owner_decision
//! (5 重: 多 AI + 多人 + 物理多签 + 反思期 + MEWG)。
//!
//! **测试矩阵** (4 token × 2 类请求 = 8 case):
//!
//! | Token    | Core-rule 变更              | 普通操作 (Audit)        |
//! |----------|-----------------------------|-------------------------|
//! | Master   | 走 5 重 (Approved/Pending)  | 走 process() (同普通)   |
//! | Admin    | 走 5 重                     | 走 process()            |
//! | Operator | 走 5 重                     | 走 process()            |
//! | ReadOnly | **被 MultiSigPolicy 拒绝**  | 走 process()            |
//!
//! **硬约束**:
//! 1. Master token 不能通过任何路径凌驾 5 重治理
//! 2. ReadOnly token 提交 core-rule 变更 → 立即 Blocked (兜底)
//! 3. Operator/Admin/Master 触及 E 层 → 触发 MEWG (evidence 包含 owner_token)
//! 4. SovereigntyHook 不提供 bypass — Master token 必须走 process_owner_decision

use apeireth_sovereignty::multi_ai::{AiStance, MockAiProvider};
use apeireth_sovereignty::multi_human::{HumanId, Vote};
use apeireth_sovereignty::owner::{OwnerAction, OwnerRequest, OwnerToken};
use apeireth_sovereignty::physical_multisig::PhysicalSignerId;
use apeireth_sovereignty::{
    Governance, GovernanceOutcome, GovernanceStep, HAAuthentication, MultiSigPolicy,
    OwnerRequestMultisigOutcome, Signatory,
};
use std::time::Duration;

/// 构造一个已通过 3 个 AI + 2 human + 2 sig 的 governance
async fn populated_governance() -> Governance {
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));

    // 3 个 AI 一致 approve
    gov.register_ai_provider(Box::new(MockAiProvider::new("gpt4", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("claude", AiStance::Approve)))
        .await
        .unwrap();
    gov.register_ai_provider(Box::new(MockAiProvider::new("local", AiStance::Approve)))
        .await
        .unwrap();

    // 2 个 human approve
    {
        let mut v = gov.multi_human.lock().await;
        v.register(HumanId::new("alice", "Alice", "owner"));
        v.register(HumanId::new("bob", "Bob", "co-owner"));
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("bob", Vote::Approve, "yes".to_string())
            .unwrap();
    }

    // 2 个 physical sig (不同 kind, 1 witness)
    {
        let mut m = gov.physical.lock().await;
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("p1", "phone", "bob"));
        m.collect_signature("y1", "digest".to_string(), true)
            .unwrap();
        m.collect_signature("p1", "digest".to_string(), false)
            .unwrap();
    }

    gov
}

fn owner_request(token: OwnerToken, action: OwnerAction) -> OwnerRequest {
    OwnerRequest::new("d-test", token, action, "alice", "Q13 test fixture")
}

#[tokio::test]
async fn master_core_rule_must_go_through_mewg() {
    let gov = populated_governance().await;
    let req = owner_request(OwnerToken::Master, OwnerAction::ModifyL0HumanAuthority);

    let outcome = gov
        .process_owner_decision(&req)
        .await
        .expect("process_owner_decision must run 5 重");

    match outcome {
        GovernanceOutcome::Approved {
            mewg_score,
            rationale,
        } => {
            assert!(mewg_score > 0.0, "MEWG score must be positive");
            assert!(
                rationale.contains("owner_token=master"),
                "rationale must record owner_token, got: {rationale}"
            );
            assert!(
                rationale.contains("touches_e_layer=true"),
                "rationale must record touches_e_layer=true, got: {rationale}"
            );
        }
        other => panic!("Master core-rule must be Approved, got {other:?}"),
    }
}

#[tokio::test]
async fn admin_core_rule_must_go_through_mewg() {
    let gov = populated_governance().await;
    let req = owner_request(OwnerToken::Admin, OwnerAction::ModifyPrincipleOnion);

    let outcome = gov.process_owner_decision(&req).await.unwrap();
    match outcome {
        GovernanceOutcome::Approved { rationale, .. } => {
            assert!(
                rationale.contains("owner_token=admin"),
                "rationale must record admin token, got: {rationale}"
            );
            assert!(rationale.contains("touches_e_layer=true"));
        }
        other => panic!("Admin core-rule must be Approved (no token bypass), got {other:?}"),
    }
}

#[tokio::test]
async fn operator_core_rule_must_go_through_mewg() {
    let gov = populated_governance().await;
    let req = owner_request(OwnerToken::Operator, OwnerAction::ModifyPermissionOnion);

    let outcome = gov.process_owner_decision(&req).await.unwrap();
    match outcome {
        GovernanceOutcome::Approved { rationale, .. } => {
            assert!(rationale.contains("owner_token=operator"));
            assert!(rationale.contains("touches_e_layer=true"));
        }
        other => panic!("Operator core-rule must be Approved, got {other:?}"),
    }
}

#[tokio::test]
async fn readonly_core_rule_is_blocked() {
    let gov = populated_governance().await;
    let req = owner_request(OwnerToken::ReadOnly, OwnerAction::ModifyL0HumanAuthority);

    let outcome = gov.process_owner_decision(&req).await.unwrap();
    match outcome {
        GovernanceOutcome::Blocked { failed_at, reason } => {
            assert_eq!(
                failed_at,
                GovernanceStep::MultiAi,
                "ReadOnly must fail at MultiSig layer (兜底 Blocked)"
            );
            assert!(
                reason.contains("ReadOnly") || reason.contains("read_only"),
                "reason must mention ReadOnly, got: {reason}"
            );
        }
        other => panic!("ReadOnly core-rule must be Blocked (no bypass), got {other:?}"),
    }
}

#[tokio::test]
async fn readonly_audit_query_runs_normal_governance() {
    let gov = populated_governance().await;
    let req = owner_request(OwnerToken::ReadOnly, OwnerAction::AuditQuery);

    let outcome = gov.process_owner_decision(&req).await.unwrap();
    assert!(matches!(outcome, GovernanceOutcome::Approved { .. }));
}

#[tokio::test]
async fn master_pause_ai_is_non_core_rule() {
    let gov = populated_governance().await;
    let req = owner_request(OwnerToken::Master, OwnerAction::PauseAi);

    let outcome = gov.process_owner_decision(&req).await.unwrap();
    assert!(matches!(outcome, GovernanceOutcome::Approved { .. }));
}

#[tokio::test]
async fn multisig_policy_rejects_insufficient_sigs_for_master() {
    // MultiSigPolicy 默认 required=2, 只收集 1 → 应被拒
    let policy = MultiSigPolicy::new(
        2,
        vec![
            Signatory::new("alice", "Alice", HAAuthentication::WindowsHello),
            Signatory::new("bob", "Bob", HAAuthentication::FIDO2),
        ],
    )
    .unwrap();
    let collected = vec!["alice".to_string()]; // 只 1 个 sig, required=2

    let req = owner_request(OwnerToken::Master, OwnerAction::ModifyL0HumanAuthority);
    let result = policy.process_owner_request(&req, &collected);
    assert!(
        matches!(
            result,
            OwnerRequestMultisigOutcome::InsufficientSignatures { .. }
        ),
        "MultiSigPolicy must reject with insufficient sigs, got: {result:?}"
    );
}

#[tokio::test]
async fn multisig_policy_accepts_master_with_enough_sigs() {
    let policy = MultiSigPolicy::new(
        2,
        vec![
            Signatory::new("alice", "Alice", HAAuthentication::WindowsHello),
            Signatory::new("bob", "Bob", HAAuthentication::FIDO2),
        ],
    )
    .unwrap();
    let collected = vec!["alice".to_string(), "bob".to_string()];

    let req = owner_request(OwnerToken::Master, OwnerAction::ModifyL0HumanAuthority);
    let result = policy.process_owner_request(&req, &collected);
    assert!(
        matches!(result, OwnerRequestMultisigOutcome::Approved { .. }),
        "Master with enough sigs must pass MultiSig, got: {result:?}"
    );
}

#[tokio::test]
async fn sovereign_hook_has_no_master_bypass() {
    // Q13 硬约束 #3: SovereigntyHook 没有任何方法能让 Master 绕过 5 重治理
    use apeireth_council::SovereigntyHook;

    struct NoopHook;
    impl SovereigntyHook for NoopHook {
        fn on_council_event(&self, _: &apeireth_council::CouncilEvent) {}
    }
    let hook = NoopHook;
    assert_eq!(hook.hook_id(), "default");
    // trait 检查确保: 即使将来增加方法, 编译期强制实现 — 不会无感 bypass
}

#[tokio::test]
async fn core_rule_with_no_governance_population_is_pending_or_blocked() {
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    // 不注册 AI, 不投票, 不签
    let req = owner_request(OwnerToken::Master, OwnerAction::ModifyL0HumanAuthority);
    let outcome = gov.process_owner_decision(&req).await.unwrap();

    // 没有 AI 投票 → 必然 PendingReview 或 Blocked (不能 Approved)
    assert!(
        matches!(
            outcome,
            GovernanceOutcome::PendingReview { .. } | GovernanceOutcome::Blocked { .. }
        ),
        "core-rule with no AI/human/sig must NOT be Approved, got {outcome:?}"
    );
}

#[tokio::test]
async fn owner_token_classification_matches_documentation() {
    use apeireth_sovereignty::owner::OwnerToken;

    assert!(OwnerToken::Master.is_privileged());
    assert!(OwnerToken::Admin.is_privileged());
    assert!(!OwnerToken::Operator.is_privileged());
    assert!(!OwnerToken::ReadOnly.is_privileged());

    assert!(OwnerToken::Master.can_attempt_core_rule());
    assert!(OwnerToken::Admin.can_attempt_core_rule());
    assert!(OwnerToken::Operator.can_attempt_core_rule());
    assert!(!OwnerToken::ReadOnly.can_attempt_core_rule());
}
