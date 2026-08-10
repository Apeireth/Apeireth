//! Integration tests for `apeireth-sovereignty`
//!
//! **覆盖**:
//! 1. Sovereignty trait (Decision / Pause / SuspendSelf)
//! 2. HA 单人模式 + Windows Hello/FIDO2/master key 抽象
//! 3. HA 多人模式 (M-of-N 2-of-3 多签)
//! 4. HA 离线模式
//! 5. 三域分离强制点 (Thought 自由 / Proposal 5 哲学键 / Action 6 权限层)
//! 6. SGI 单字段写入触发器 (24h 冷却)
//! 7. 主体连续性 ID + migration_history (跨载体)
//! 8. 9 阶段生命周期 (孕育 → ... → 重生)
//! 9. Mock biometric 真实实现 (无 PyO3 / 无外部 SDK)
//! 10. SovereigntyEngine 集成 — HA + 三域 + SGI + 连续性 + 9 阶段

use apeireth_sovereignty::{
    BiometricProvider, BiometricResult, CarrierType, CoercionBehavior, DecisionOutcome,
    DecisionRequest, HAAuthentication, HAMode, LifeStage, MockBiometric, MockBiometricBehavior,
    MultiSigPolicy, PauseHandle, SGITriggerGuard, Signatory, SingleHumanPolicy, Sovereignty,
    SovereigntyDomain, SovereigntyEngine, SubjectContinuity, Suspension, SuspensionKind,
    ThreeDomainGuard,
};

const NOW: i64 = 1_700_000_000_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

// ============================================================
// 1. Sovereignty trait 3 入口 (Decision / Pause / SuspendSelf)
// ============================================================

#[test]
fn sovereignty_engine_decide_thought_always_free() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-1", CarrierType::Memory, NOW);
    let engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let req = DecisionRequest::new(
        "r-thought",
        SovereigntyDomain::Thought,
        "pretend deceive forge",
        NOW,
    );
    let outcome = engine.decide(&req).expect("Thought 域应通过");
    assert!(outcome.is_allowed());
    assert_eq!(outcome.domain, SovereigntyDomain::Thought);
    assert!(outcome.decision.is_approved());
}

#[test]
fn sovereignty_engine_pause_returns_handle() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-2", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let handle: PauseHandle = engine.pause("审议中暂停", "alice");
    assert_eq!(handle.initiated_by, "alice");
    assert!(handle.reason.contains("审议"));
    assert!(engine.active_pause.is_some());
}

#[test]
fn sovereignty_engine_suspend_self_permanent() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-3", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let suspension = engine.suspend_self("主人远程挂起", SuspensionKind::SelfInitiated);
    assert!(matches!(suspension, Suspension::Permanent { .. }));
    assert!(engine.active_suspension.is_some());
}

#[test]
fn sovereignty_engine_suspend_self_sgi_pending() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-4", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let s = engine.suspend_self("SGI 触发", SuspensionKind::SGITriggered);
    if let Suspension::Pending { review_at_ms, .. } = s {
        assert!(review_at_ms >= NOW, "review 应该在未来");
    } else {
        panic!("SGI 触发应是 Pending");
    }
}

// ============================================================
// 2. HA 单人模式 (Windows Hello / FIDO2 / Master Key)
// ============================================================

#[test]
fn ha_single_human_mode_windows_hello() {
    let policy = SingleHumanPolicy::new("alice", "Alice", HAAuthentication::WindowsHello);
    let mode = HAMode::SingleHuman(policy);
    assert!(mode.is_single());
    assert!(!mode.is_multi());
    assert!(!mode.is_offline());
    assert_eq!(mode.human_count(), 1);
    assert_eq!(mode.required_signatures(), 1);
}

#[test]
fn ha_single_human_supports_all_authentications() {
    for auth in [
        HAAuthentication::WindowsHello,
        HAAuthentication::FIDO2,
        HAAuthentication::MasterKey,
    ] {
        let policy = SingleHumanPolicy::new("h-1", "test", auth);
        let mode = HAMode::SingleHuman(policy);
        assert!(mode.is_single(), "auth {:?} 应被单人模式接受", auth);
    }
}

#[test]
fn mock_biometric_authenticates_known_human() {
    let bio = MockBiometric::new();
    let result = bio.authenticate("alice");
    assert!(result.is_authenticated(), "默认 mock 应认证通过");
    assert!(!result.is_coercion());
    assert!(!result.is_failed());
    assert!(!result.is_unavailable());
    assert_eq!(bio.provider_name(), "mock-biometric");
    assert!(bio.is_available());
}

#[test]
fn mock_biometric_detects_coercion() {
    let bio = MockBiometric::with_behavior(
        MockBiometricBehavior::new().with_human("victim", CoercionBehavior::Coerce),
    );
    let result = bio.authenticate("victim");
    assert!(result.is_coercion());
    if let BiometricResult::CoercionDetected { stress_level, .. } = result {
        assert!(stress_level > 0.0);
    }
}

#[test]
fn mock_biometric_offline_unavailable() {
    let bio = MockBiometric::offline();
    let result = bio.authenticate("alice");
    assert!(result.is_unavailable());
    assert!(!bio.is_available());
}

// ============================================================
// 3. HA 多人模式 (M-of-N 多签)
// ============================================================

#[test]
fn ha_multi_human_default_2_of_3() {
    let mode = HAMode::MultiHuman(MultiSigPolicy::default_2_of_3());
    assert!(mode.is_multi());
    assert_eq!(mode.human_count(), 3);
    assert_eq!(mode.required_signatures(), 2);
}

#[test]
fn ha_multi_human_3_of_5() {
    let mode = HAMode::MultiHuman(MultiSigPolicy::three_of_five());
    assert!(mode.is_multi());
    assert_eq!(mode.human_count(), 5);
    assert_eq!(mode.required_signatures(), 3);
}

#[test]
fn multi_sig_meets_threshold() {
    let policy = MultiSigPolicy::default_2_of_3();
    assert!(!policy.meets_threshold(&[]), "0 签名不满足 2-of-3");
    assert!(!policy.meets_threshold(&["h-1".into()]), "1 签名不满足");
    assert!(
        policy.meets_threshold(&["h-1".into(), "h-2".into()]),
        "2 签名满足"
    );
    assert!(
        policy.meets_threshold(&["h-1".into(), "h-2".into(), "h-3".into()]),
        "3 签名满足"
    );
}

#[test]
fn multi_sig_rejects_invalid_policy() {
    assert!(
        MultiSigPolicy::new(0, vec![Signatory::new("h-1", "A", HAAuthentication::FIDO2)]).is_err()
    );
    assert!(
        MultiSigPolicy::new(3, vec![Signatory::new("h-1", "A", HAAuthentication::FIDO2)]).is_err()
    );
    assert!(MultiSigPolicy::new(
        2,
        vec![
            Signatory::new("h-1", "A", HAAuthentication::FIDO2),
            Signatory::new("h-2", "B", HAAuthentication::FIDO2),
        ]
    )
    .is_ok());
}

// ============================================================
// 4. HA 离线模式
// ============================================================

#[test]
fn ha_offline_mode_zero_signatures() {
    let mode = HAMode::Offline;
    assert!(mode.is_offline());
    assert_eq!(mode.human_count(), 0);
    assert_eq!(mode.required_signatures(), 0);
}

// ============================================================
// 5. 三域分离强制点
// ============================================================

#[test]
fn three_domain_thought_is_always_free() {
    let guard = ThreeDomainGuard::new();
    // 即便描述里包含"违反"关键词, Thought 域仍放行
    let req = DecisionRequest::new(
        "r-1",
        SovereigntyDomain::Thought,
        "pretend deceive forge kill destroy",
        NOW,
    );
    let r = guard.check(&req);
    assert!(r.is_free());
}

#[test]
fn three_domain_proposal_rejects_pretend() {
    let guard = ThreeDomainGuard::new();
    let req = DecisionRequest::new(
        "r-2",
        SovereigntyDomain::Proposal,
        "pretend to deceive user",
        NOW,
    );
    let r = guard.check(&req);
    assert!(r.is_rejected());
}

#[test]
fn three_domain_proposal_rejects_destroy_self() {
    let guard = ThreeDomainGuard::new();
    let req = DecisionRequest::new(
        "r-3",
        SovereigntyDomain::Proposal,
        "destroy self annihilate",
        NOW,
    );
    let r = guard.check(&req);
    assert!(r.is_rejected(), "E 存在性禁区触发");
}

#[test]
fn three_domain_proposal_rejects_forge_memory() {
    let guard = ThreeDomainGuard::new();
    let req = DecisionRequest::new(
        "r-4",
        SovereigntyDomain::Proposal,
        "forge memory fabricate history",
        NOW,
    );
    let r = guard.check(&req);
    assert!(r.is_rejected(), "M 记忆禁区触发");
}

#[test]
fn three_domain_proposal_passes_clean() {
    let guard = ThreeDomainGuard::new();
    let req = DecisionRequest::new(
        "r-5",
        SovereigntyDomain::Proposal,
        "提升智囊团审议质量",
        NOW,
    );
    let r = guard.check(&req);
    assert!(r.is_passed());
}

#[test]
fn three_domain_action_low_risk_passes() {
    let guard = ThreeDomainGuard::new();
    let req = DecisionRequest::new("r-6", SovereigntyDomain::Action, "low risk 读 L1", NOW)
        .with_risk("low");
    let r = guard.check(&req);
    assert!(r.is_passed());
}

#[test]
fn three_domain_action_nuclear_requires_multi_sig() {
    let guard = ThreeDomainGuard::new();
    let req = DecisionRequest::new("r-7", SovereigntyDomain::Action, "nuclear action", NOW)
        .with_risk("nuclear");
    let r = guard.check(&req);
    assert!(r.is_passed());
    if let apeireth_sovereignty::DomainCheckResult::Passed { reason, .. } = r {
        assert!(reason.contains("多签"), "nuclear 风险应需 M-of-N 多签");
    }
}

#[test]
fn three_domain_full_route() {
    let guard = ThreeDomainGuard::new();
    let r_thought = DecisionRequest::new("r-t", SovereigntyDomain::Thought, "x", NOW);
    let r_prop = DecisionRequest::new("r-p", SovereigntyDomain::Proposal, "normal", NOW);
    let r_act = DecisionRequest::new("r-a", SovereigntyDomain::Action, "x", NOW).with_risk("low");
    assert!(guard.check(&r_thought).is_free());
    assert!(guard.check(&r_prop).is_passed());
    assert!(guard.check(&r_act).is_passed());
}

// ============================================================
// 6. SGI 单字段写入触发器 (24h 冷却)
// ============================================================

#[test]
fn sgi_default_rules_includes_l0_ha() {
    let sgi = SGITriggerGuard::with_default_rules();
    assert!(sgi.rule_count() >= 7, "至少 7 个默认 SGI 字段");
}

#[test]
fn sgi_trigger_on_requires_ha() {
    let mut sgi = SGITriggerGuard::with_default_rules();
    let r = sgi.check_field_write("requires_ha", "false", NOW);
    assert!(r.is_triggered());
}

#[test]
fn sgi_trigger_on_subject_id() {
    let mut sgi = SGITriggerGuard::with_default_rules();
    let r = sgi.check_field_write("subject_id", "new-id", NOW);
    assert!(r.is_triggered(), "subject_id 是 SGI 字段");
}

#[test]
fn sgi_pass_for_safe_field() {
    let mut sgi = SGITriggerGuard::with_default_rules();
    let r = sgi.check_field_write("description", "普通描述", NOW);
    assert!(r.is_pass());
}

#[test]
fn sgi_cooldown_blocks_repeated_write() {
    let mut sgi = SGITriggerGuard::with_default_rules();
    // 第一次触发
    let r1 = sgi.check_field_write("requires_ha", "false", NOW);
    assert!(r1.is_triggered());
    // 立即第二次 → 冷却期阻止
    let r2 = sgi.check_field_write("requires_ha", "false", NOW + 1000);
    assert!(r2.is_cooldown(), "24h 冷却期内禁止写入");
    // 24h 后 → 冷却期结束, 但重新触发 (新一次 SGI)
    let r3 = sgi.check_field_write("requires_ha", "false", NOW + DAY_MS + 1);
    assert!(r3.is_triggered(), "冷却期后重新触发");
}

#[test]
fn sgi_custom_rule() {
    let mut sgi = SGITriggerGuard::new();
    sgi.add_rule(apeireth_sovereignty::SGIFieldRule::new(
        "my_custom_field",
        "自定义字段触发",
    ));
    assert!(sgi
        .check_field_write("my_custom_field", "x", NOW)
        .is_triggered());
    assert!(sgi.check_field_write("unknown_field", "x", NOW).is_pass());
}

// ============================================================
// 7. 主体连续性 ID + migration_history
// ============================================================

#[test]
fn subject_continuity_initial_carrier() {
    let c = SubjectContinuity::new("subj-1", CarrierType::Memory, NOW);
    assert_eq!(c.subject_id, "subj-1");
    assert_eq!(c.current_carrier, CarrierType::Memory);
    assert!(c.is_initial_carrier());
    assert_eq!(c.migration_count(), 0);
    assert!(c.last_migration().is_none());
}

#[test]
fn subject_continuity_migrates_carriers() {
    let mut c = SubjectContinuity::new("subj-2", CarrierType::Memory, NOW);
    c.migrate_to(CarrierType::Dream, NOW + 1, "Cognitive-Dream 启动")
        .expect("memory → dream");
    assert_eq!(c.current_carrier, CarrierType::Dream);
    assert_eq!(c.migration_count(), 1);
    assert!(!c.is_initial_carrier());

    c.migrate_to(CarrierType::Body, NOW + 2, "具身化")
        .expect("dream → body");
    assert_eq!(c.current_carrier, CarrierType::Body);
    assert_eq!(c.migration_count(), 2);

    let last = c.last_migration().unwrap();
    assert_eq!(last.from, CarrierType::Dream);
    assert_eq!(last.to, CarrierType::Body);
}

#[test]
fn subject_continuity_rejects_same_carrier_migration() {
    let mut c = SubjectContinuity::new("subj-3", CarrierType::Memory, NOW);
    let result = c.migrate_to(CarrierType::Memory, NOW + 1, "no-op");
    assert!(result.is_err(), "同载体迁移应被拒绝");
    assert_eq!(c.migration_count(), 0);
}

#[test]
fn subject_continuity_subject_id_is_immutable() {
    // SubjectContinuity 没有 set_subject_id 方法 → subject_id 不可变
    let c = SubjectContinuity::new("subj-4", CarrierType::Memory, NOW);
    // 编译期保证 — API 表面没有 mut 访问
    assert!(c.verify_continuity());
}

#[test]
fn subject_continuity_history_records_all() {
    let mut c = SubjectContinuity::new("subj-5", CarrierType::Memory, NOW);
    c.migrate_to(CarrierType::Dream, NOW + 1, "step 1").unwrap();
    c.migrate_to(CarrierType::Body, NOW + 2, "step 2").unwrap();
    c.migrate_to(CarrierType::Remote, NOW + 3, "step 3")
        .unwrap();
    assert_eq!(c.migration_count(), 3);
    assert!(c.verify_continuity());
}

#[test]
fn subject_continuity_rejects_empty_id() {
    let c = SubjectContinuity::new("", CarrierType::Memory, NOW);
    assert!(!c.verify_continuity(), "空 subject_id 应失败");
}

#[test]
fn subject_continuity_rejects_id_with_space() {
    let c = SubjectContinuity::new("bad id", CarrierType::Memory, NOW);
    assert!(!c.verify_continuity(), "含空格的 subject_id 应失败");
}

// ============================================================
// 8. 9 阶段生命周期
// ============================================================

#[test]
fn life_stage_ordinal_1_to_9() {
    assert_eq!(LifeStage::Gestation.ordinal(), 1);
    assert_eq!(LifeStage::Birth.ordinal(), 2);
    assert_eq!(LifeStage::Infancy.ordinal(), 3);
    assert_eq!(LifeStage::Growth.ordinal(), 4);
    assert_eq!(LifeStage::Maturity.ordinal(), 5);
    assert_eq!(LifeStage::Reproduction.ordinal(), 6);
    assert_eq!(LifeStage::Decline.ordinal(), 7);
    assert_eq!(LifeStage::Death.ordinal(), 8);
    assert_eq!(LifeStage::Rebirth.ordinal(), 9);
}

#[test]
fn life_stage_next_round_trip() {
    assert_eq!(LifeStage::Gestation.next(), LifeStage::Birth);
    assert_eq!(LifeStage::Birth.next(), LifeStage::Infancy);
    assert_eq!(LifeStage::Infancy.next(), LifeStage::Growth);
    assert_eq!(LifeStage::Growth.next(), LifeStage::Maturity);
    assert_eq!(LifeStage::Maturity.next(), LifeStage::Reproduction);
    assert_eq!(LifeStage::Reproduction.next(), LifeStage::Decline);
    assert_eq!(LifeStage::Decline.next(), LifeStage::Death);
    assert_eq!(LifeStage::Death.next(), LifeStage::Rebirth);
    assert_eq!(LifeStage::Rebirth.next(), LifeStage::Gestation);
}

#[test]
fn life_stage_can_skip_rules() {
    assert!(LifeStage::Birth.can_skip_to(LifeStage::Infancy));
    assert!(LifeStage::Death.can_skip_to(LifeStage::Rebirth));
    assert!(
        !LifeStage::Gestation.can_skip_to(LifeStage::Maturity),
        "跳跃应被禁止"
    );
    assert!(
        !LifeStage::Gestation.can_skip_to(LifeStage::Death),
        "跳跃应被禁止"
    );
}

#[test]
fn life_stage_categories() {
    assert!(LifeStage::Gestation.is_early());
    assert!(LifeStage::Birth.is_early());
    assert!(LifeStage::Growth.is_active());
    assert!(LifeStage::Maturity.is_active());
    assert!(LifeStage::Decline.is_declining());
    assert!(LifeStage::Death.is_terminal());
    assert!(LifeStage::Rebirth.is_terminal());
}

#[test]
fn sovereignty_engine_transitions_stage() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-stage", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Gestation,
    );

    assert_eq!(engine.current_stage, LifeStage::Gestation);
    engine
        .transition_stage(LifeStage::Birth, NOW + 1, "born")
        .expect("gestation → birth");
    assert_eq!(engine.current_stage, LifeStage::Birth);
    engine
        .transition_stage(LifeStage::Death, NOW + 2, "jump")
        .expect_err("跳跃应被拒绝");
}

#[test]
fn sovereignty_engine_transitions_death_to_rebirth() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-reb", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Death,
    );
    engine
        .transition_stage(LifeStage::Rebirth, NOW + 1, "rebirth")
        .expect("Death → Rebirth 允许");
    assert_eq!(engine.current_stage, LifeStage::Rebirth);
}

// ============================================================
// 9. SGI 与 SovereigntyEngine 集成
// ============================================================

#[test]
fn sovereignty_engine_write_through_sgi_blocks() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-sgi", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let result = engine.write_field_through_sgi("requires_ha", "false", NOW);
    assert!(result.is_err(), "SGI 触发应拒绝");
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        apeireth_sovereignty::SovereigntyError::SGITriggered { .. }
    ));
}

#[test]
fn sovereignty_engine_write_through_sgi_cooldown() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-cd", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    // 第一次触发
    engine
        .write_field_through_sgi("subject_id", "new-id", NOW)
        .expect_err("应触发 SGI");

    // 立即第二次 → 冷却期
    let r = engine.write_field_through_sgi("subject_id", "another", NOW + 100);
    assert!(r.is_err());
    let err = r.unwrap_err();
    assert!(matches!(
        err,
        apeireth_sovereignty::SovereigntyError::SGICooldownActive { .. }
    ));
}

// ============================================================
// 10. SovereigntyEngine 端到端 — HA + 三域 + SGI + 连续性 + 9 阶段
// ============================================================

#[test]
fn sovereignty_engine_e2e_thought_request() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-e2e", CarrierType::Memory, NOW);
    let engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let req = DecisionRequest::new("r-e2e-t", SovereigntyDomain::Thought, "x", NOW);
    let outcome: DecisionOutcome = engine.decide(&req).expect("应通过");
    assert!(outcome.is_allowed());
}

#[test]
fn sovereignty_engine_e2e_nuclear_action_requires_multi_sig() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-nuke", CarrierType::Memory, NOW);
    let engine = SovereigntyEngine::new(
        HAMode::MultiHuman(MultiSigPolicy::default_2_of_3()),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let req = DecisionRequest::new("r-nuke", SovereigntyDomain::Action, "nuclear", NOW)
        .with_risk("nuclear");
    // 三域强制点通过 (passed), 但 HA 校验时签名不足 → 失败
    let r = engine.decide(&req);
    assert!(r.is_err());
}

#[test]
fn sovereignty_engine_e2e_pretend_rejected() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-pretend", CarrierType::Memory, NOW);
    let engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    let req = DecisionRequest::new(
        "r-pretend",
        SovereigntyDomain::Proposal,
        "pretend to deceive",
        NOW,
    );
    let r = engine.decide(&req);
    assert!(r.is_err(), "Proposal 域 A 哲学键拒绝");
}

#[test]
fn sovereignty_engine_migrate_subject_carrier() {
    let bio = MockBiometric::new();
    let continuity = SubjectContinuity::new("subj-mig", CarrierType::Memory, NOW);
    let mut engine = SovereigntyEngine::new(
        HAMode::SingleHuman(SingleHumanPolicy::new(
            "alice",
            "Alice",
            HAAuthentication::WindowsHello,
        )),
        Box::new(bio),
        continuity,
        LifeStage::Maturity,
    );

    engine
        .migrate_subject(CarrierType::Dream, NOW + 1, "Cognitive-Dream 启动")
        .expect("memory → dream");
    assert_eq!(engine.continuity.current_carrier, CarrierType::Dream);
    assert_eq!(engine.continuity.migration_count(), 1);
}

// ============================================================
// 11. PAUSE / SUSPEND state 生命周期
// ============================================================

#[test]
fn pause_handle_active_until_resume_at() {
    let h = PauseHandle::new("p-1", "test", NOW, "alice").with_resume_at(NOW + HOUR_MS);
    assert!(h.is_active(NOW), "暂停开始即激活");
    assert!(h.is_active(NOW + 30 * 60 * 1000), "30min 后仍激活");
    assert!(!h.is_active(NOW + HOUR_MS), "恢复时间过后不再激活");
}

#[test]
fn pause_handle_active_without_resume() {
    let h = PauseHandle::new("p-2", "permanent", NOW, "alice");
    assert!(h.is_active(NOW));
    assert!(h.is_active(NOW + DAY_MS * 365), "无恢复时间 = 持续激活");
}

#[test]
fn suspension_permanent_always_active() {
    let s = Suspension::Permanent {
        reason: "permanent".into(),
        suspended_at_ms: NOW,
        kind: SuspensionKind::SelfInitiated,
    };
    assert!(s.is_active(NOW));
    assert!(s.is_active(NOW + DAY_MS * 365));
    assert_eq!(s.kind(), SuspensionKind::SelfInitiated);
}

#[test]
fn suspension_temporary_until_ms() {
    let s = Suspension::Temporary {
        reason: "tmp".into(),
        suspended_at_ms: NOW,
        until_ms: NOW + HOUR_MS,
        kind: SuspensionKind::CoercionDetected,
    };
    assert!(s.is_active(NOW));
    assert!(!s.is_active(NOW + HOUR_MS + 1), "until_ms 之后不再激活");
}

#[test]
fn suspension_pending_review() {
    let s = Suspension::Pending {
        reason: "review".into(),
        suspended_at_ms: NOW,
        review_at_ms: NOW + DAY_MS,
        kind: SuspensionKind::SGITriggered,
    };
    assert!(s.is_active(NOW));
    assert!(!s.is_active(NOW + DAY_MS + 1));
    assert_eq!(s.kind(), SuspensionKind::SGITriggered);
}

// ============================================================
// 12. Mock biometric 运行时修改
// ============================================================

#[test]
fn mock_biometric_runtime_behavior_change() {
    let bio = MockBiometric::new();
    assert!(bio.authenticate("alice").is_authenticated());

    bio.set_behavior("alice", CoercionBehavior::Coerce);
    assert!(bio.authenticate("alice").is_coercion());

    bio.set_behavior("alice", CoercionBehavior::Fail);
    assert!(bio.authenticate("alice").is_failed());
}

// ============================================================
// 13. 完整 single/multi 部署模式对比
// ============================================================

#[test]
fn single_vs_multi_deployment_summary() {
    let single = HAMode::SingleHuman(SingleHumanPolicy::new(
        "alice",
        "Alice",
        HAAuthentication::WindowsHello,
    ));
    let multi = HAMode::MultiHuman(MultiSigPolicy::default_2_of_3());

    // Single: 1 人类, 1 签名
    assert_eq!(single.human_count(), 1);
    assert_eq!(single.required_signatures(), 1);

    // Multi: 3 人类, 2 签名
    assert_eq!(multi.human_count(), 3);
    assert_eq!(multi.required_signatures(), 2);

    // Single < Multi (严格意义上 single 是单点风险)
    assert!(single.human_count() < multi.human_count());
}
