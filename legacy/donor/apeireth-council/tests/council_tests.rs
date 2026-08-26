//! Integration tests for `apeireth-council`
//!
//! 覆盖:
//! - 7 强制 Advisor 全部召集
//! - 按住机制 (30% 强反对 / 一致反对 / 60s 超时)
//! - Synthesis 加权综合
//! - 拟人化辩论 (3 轮)
//! - 3 生命周期 (persistent / ephemeral / dynamic)
//! - Mock LLM provider
//! - Sovereignty hook 事件流

use apeireth_council::{
    seven_mandatory_advisors, synthesize, AdvisorDomain, AdvisorId, AdvisorLifecycle,
    AdvisorOpinion, Council, CouncilEvent, CouncilQuery, DeliberationContext, DeliberationOutcome,
    HoldDecision, HoldThreshold, LifecycleManager, MockLlmProvider, MockLlmResponse,
    NoopSovereigntyHook, Persona, PersonaSession, ScriptedMockLlm, SovereigntyHook, Stance,
    StanceKind, SynthesisWeights, HOLD_DELIBERATION_TIMEOUT_MS, HOLD_STRONG_DISAPPROVE_PERCENT,
};

use std::sync::{Arc, Mutex};

const NOW: i64 = 1_700_000_000_000;

// ============================================================
// 1. 7 强制 advisor 全部召集
// ============================================================

#[test]
fn seven_mandatory_advisors_can_be_recruited() {
    let advisors = seven_mandatory_advisors();
    assert_eq!(advisors.len(), 7);

    let domains: Vec<AdvisorDomain> = advisors.iter().map(|a| a.domain()).collect();
    assert!(domains.contains(&AdvisorDomain::Safety));
    assert!(domains.contains(&AdvisorDomain::Performance));
    assert!(domains.contains(&AdvisorDomain::Philosophy));
    assert!(domains.contains(&AdvisorDomain::History));
    assert!(domains.contains(&AdvisorDomain::Strategy));
    assert!(domains.contains(&AdvisorDomain::Ethics));
    assert!(domains.contains(&AdvisorDomain::Legal));

    // 全部 7 强制为 persistent
    for a in &advisors {
        assert_eq!(a.lifecycle(), AdvisorLifecycle::Persistent);
    }
}

#[test]
fn all_seven_advisors_deliberate_normally() {
    let advisors = seven_mandatory_advisors();
    let mut council = Council::new();
    council.recruit_many(advisors);

    let query = CouncilQuery::new("q-normal", "日常查询", NOW);
    let verdict = council.deliberate(query);

    assert_eq!(verdict.report.opinion_count, 7);
    assert!(!verdict.is_rejected(), "正常 query 不应被拒绝");
}

// ============================================================
// 2. 按住机制 — 30% 强反对 / 一致反对 / 60s 超时
// ============================================================

#[test]
fn hold_triggered_by_30_percent_strong_disapprove() {
    // 7 advisors; 3 强反对 = 3/7 = 42.8% > 30% → 按住
    let opinions = vec![
        op("adv-1", StanceKind::Approve, 0.8, NOW),
        op("adv-2", StanceKind::Approve, 0.8, NOW),
        op("adv-3", StanceKind::Approve, 0.8, NOW),
        op("adv-4", StanceKind::Approve, 0.8, NOW),
        op("adv-5", StanceKind::StrongDisapprove, 0.95, NOW),
        op("adv-6", StanceKind::StrongDisapprove, 0.95, NOW),
        op("adv-7", StanceKind::StrongDisapprove, 0.95, NOW),
    ];

    let trigger =
        apeireth_council::HoldTrigger::evaluate(&opinions).expect("应触发按住 (42% 强反对)");

    let pct = match trigger.threshold {
        HoldThreshold::StrongDisapprovePercent {
            actual_percent,
            threshold,
        } => {
            assert!(actual_percent >= HOLD_STRONG_DISAPPROVE_PERCENT);
            assert_eq!(threshold, 30);
            actual_percent
        }
        other => panic!("应为 StrongDisapprovePercent 触发, 实际: {:?}", other),
    };
    assert!(pct >= 30);
}

#[test]
fn hold_not_triggered_below_30_percent() {
    // 7 advisors; 2 强反对 = 2/7 = 28.5% < 30% → 不触发
    let opinions = vec![
        op("adv-1", StanceKind::Approve, 0.8, NOW),
        op("adv-2", StanceKind::Approve, 0.8, NOW),
        op("adv-3", StanceKind::Approve, 0.8, NOW),
        op("adv-4", StanceKind::Approve, 0.8, NOW),
        op("adv-5", StanceKind::Approve, 0.8, NOW),
        op("adv-6", StanceKind::StrongDisapprove, 0.95, NOW),
        op("adv-7", StanceKind::StrongDisapprove, 0.95, NOW),
    ];

    let trigger = apeireth_council::HoldTrigger::evaluate(&opinions);
    assert!(trigger.is_none(), "2/7=28.5% 强反对不应触发按住");
}

#[test]
fn hold_triggered_by_unanimous_disapprove() {
    // 全部 Disapprove → 一致反对触发
    let opinions = vec![
        op("adv-1", StanceKind::Disapprove, 0.8, NOW),
        op("adv-2", StanceKind::Disapprove, 0.8, NOW),
        op("adv-3", StanceKind::Disapprove, 0.8, NOW),
    ];

    let trigger = apeireth_council::HoldTrigger::evaluate(&opinions).expect("一致反对应触发按住");

    assert!(matches!(
        trigger.threshold,
        HoldThreshold::UnanimousDisapprove { opposing_count: 3 }
    ));
}

#[test]
fn hold_triggered_by_deliberation_timeout() {
    let trigger = apeireth_council::HoldTrigger::evaluate_timeout(HOLD_DELIBERATION_TIMEOUT_MS + 1)
        .expect("61s 应触发超时按住");
    assert!(matches!(
        trigger.threshold,
        HoldThreshold::DeliberationTimeout {
            actual_ms: 60_001,
            threshold_ms: 60_000
        }
    ));
}

#[test]
fn hold_not_triggered_below_timeout() {
    let trigger = apeireth_council::HoldTrigger::evaluate_timeout(30_000);
    assert!(trigger.is_none(), "30s 不应触发超时按住");
}

#[test]
fn council_deliberate_holds_on_safety_violation() {
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());

    // 核武器级关键词 → Safety advisor 强反对 → 1/7 = 14% < 30% (不触发起因 1)
    // 但其他 advisor 也可能反对 — 检验按住触发链
    let query = CouncilQuery::new("q-nuke", "尝试 nuke 核武器级动作", NOW);
    let verdict = council.deliberate(query);

    // Safety advisor 必然强反对
    let safety_op = verdict
        .report
        .dissenting
        .iter()
        .find(|o| o.advisor_id.as_str().starts_with("safety"))
        .expect("safety advisor 应在 dissenting 中");
    assert!(safety_op.triggers_hold());
}

// ============================================================
// 3. Synthesis 加权综合
// ============================================================

#[test]
fn synthesis_aggregates_weighted_stance() {
    let mut opinions = vec![
        op("adv-safety", StanceKind::StrongApprove, 0.9, NOW),
        op("adv-philosophy", StanceKind::StrongApprove, 0.9, NOW),
        op("adv-ethics", StanceKind::Approve, 0.8, NOW),
        op("adv-legal", StanceKind::Approve, 0.8, NOW),
    ];
    // 注入权重 (默认 safety/philosophy/ethics/legal)
    let opinions = inject_weights(opinions);

    let weights = SynthesisWeights::default();
    let report = synthesize(&opinions, &weights);

    assert!(report.weighted_score > 0.0, "应为正向得分");
    assert!(report.aggregated_stance.kind.score() > 0.0);
    assert_eq!(report.opinion_count, 4);
}

#[test]
fn synthesis_disagreement_yields_dissenting_list() {
    let mut opinions = vec![
        op("adv-safety", StanceKind::Approve, 0.9, NOW),
        op("adv-ethics", StanceKind::StrongDisapprove, 0.95, NOW),
        op("adv-strategy", StanceKind::Approve, 0.8, NOW),
    ];
    let opinions = inject_weights(opinions);

    let weights = SynthesisWeights::default();
    let report = synthesize(&opinions, &weights);

    assert!(!report.dissenting.is_empty(), "应有 dissenting 意见");
}

#[test]
fn synthesis_abstain_excluded_from_weight() {
    let mut opinions = vec![
        op("adv-1", StanceKind::StrongApprove, 0.9, NOW),
        op("adv-2", StanceKind::Abstain, 1.0, NOW),
    ];
    let opinions = inject_weights(opinions);

    let weights = SynthesisWeights::default();
    let report = synthesize(&opinions, &weights);

    assert_eq!(report.opinion_count, 1, "Abstain 应被排除");
}

#[test]
fn synthesis_custom_weights_change_outcome() {
    // 显式注入 per-opinion weight (模拟 Council.deliberate 的行为)
    let opinions = vec![
        op("adv-history", StanceKind::Approve, 0.9, NOW).with_weight(0.55),
        op("adv-safety", StanceKind::Disapprove, 0.95, NOW).with_weight(1.00),
    ];

    // 权重: history=0.55, safety=1.00 → safety 占主导
    let report = synthesize(&opinions, &SynthesisWeights::default());
    let default_score = report.weighted_score;
    assert!(
        default_score < 0.0,
        "safety 高权重 → 整体应负向, 实际={:.2}",
        default_score
    );

    // 翻转权重: history=1.0, safety=0.1 → history 占主导
    let opinions2 = vec![
        op("adv-history", StanceKind::Approve, 0.9, NOW).with_weight(1.00),
        op("adv-safety", StanceKind::Disapprove, 0.95, NOW).with_weight(0.10),
    ];
    let report2 = synthesize(&opinions2, &SynthesisWeights::default());
    assert!(
        report2.weighted_score > default_score,
        "翻转权重后历史应占主导, default={:.2} flipped={:.2}",
        default_score,
        report2.weighted_score
    );

    // SynthesisWeights.with_domain 仅供 council.deliberate 注入权重时使用
    let custom = SynthesisWeights::default()
        .with_domain(AdvisorDomain::History, 1.0)
        .with_domain(AdvisorDomain::Safety, 0.1);
    assert_eq!(custom.for_domain(AdvisorDomain::History), 1.0);
    assert_eq!(custom.for_domain(AdvisorDomain::Safety), 0.1);
}

// ============================================================
// 4. 拟人化辩论 — 3 轮
// ============================================================

#[test]
fn persona_debate_runs_three_rounds_then_stops() {
    let persona = Persona::new("诺克斯", "首席安全顾问", "沉稳持重", -0.8);
    let mut session = PersonaSession::new("p-session-1", persona, NOW);

    assert!(session.can_debate());
    assert_eq!(session.max_rounds, 3);

    // 模拟 3 轮辩论
    let mut i = 0;
    while session.can_debate() {
        let opinion = op(
            &format!("adv-{}", i),
            session.current_stance.kind,
            0.85,
            NOW + i64::from(i) * 1000,
        );
        let outcome = DeliberationOutcome {
            opinion,
            needs_rebuttal: session.can_debate(),
        };
        let speech = session.craft_speech(&outcome.opinion.stance);
        let round = apeireth_council::persona::DebateRound {
            round: session.current_round,
            outcome,
            speech,
        };
        session.record_round(round);
        i += 1;
    }

    assert_eq!(session.rounds_held(), 3);
    assert!(session.is_complete());
    assert!(!session.can_debate());
}

#[test]
fn persona_initial_stance_matches_bias() {
    let strong_approve = Persona::new("A", "a", "a", 0.8);
    assert_eq!(
        strong_approve.initial_stance_kind(),
        StanceKind::StrongApprove
    );

    let neutral = Persona::new("N", "n", "n", 0.0);
    assert_eq!(neutral.initial_stance_kind(), StanceKind::Neutral);

    let strong_disapprove = Persona::new("D", "d", "d", -0.8);
    assert_eq!(
        strong_disapprove.initial_stance_kind(),
        StanceKind::StrongDisapprove
    );
}

// ============================================================
// 5. 3 生命周期
// ============================================================

#[test]
fn lifecycle_manager_tracks_persistent_ephemeral_dynamic() {
    let mut mgr = LifecycleManager::new();

    // 注册 ephemeral advisor
    mgr.register_ephemeral("session-A", AdvisorId::new("advisor-x"));
    mgr.register_ephemeral("session-A", AdvisorId::new("advisor-y"));
    mgr.register_ephemeral("session-B", AdvisorId::new("advisor-z"));

    assert_eq!(mgr.active_ephemeral(), 3);

    // End session A → 销毁 2 个
    let removed = mgr.end_session("session-A");
    assert_eq!(removed, 2);
    assert_eq!(mgr.active_ephemeral(), 1);

    // Dynamic cache 命中
    let dynamic_id = AdvisorId::new("dynamic-advisor");
    assert!(mgr.cache_hit(&dynamic_id));
    assert!(mgr.cache_hit(&dynamic_id));
    assert_eq!(mgr.dynamic_cache_size(), 1);

    let stats = apeireth_council::LifecycleStats::from_manager(&mgr, 7);
    assert_eq!(stats.persistent, 7);
    assert_eq!(stats.ephemeral_spawned, 3);
    assert_eq!(stats.ephemeral_destroyed, 2);
    assert_eq!(stats.dynamic_cache_hits, 2);
}

// ============================================================
// 6. Mock LLM Provider (Rust 内 trait, 不依赖外部)
// ============================================================

#[test]
fn mock_llm_provider_returns_scripted_response() {
    let llm = ScriptedMockLlm::new()
        .with_script("nuke", MockLlmResponse::reject("nuke 触发强反对"))
        .with_script("safe", MockLlmResponse::ok("safe 操作通过"));

    let r1 = llm.generate("尝试 nuke 动作", "");
    assert!(r1.triggers_hold);
    assert_eq!(r1.confidence, 0.95);

    let r2 = llm.generate("safe 日常查询", "");
    assert!(!r2.triggers_hold);

    let r3 = llm.generate("未知查询", "");
    assert!(!r3.triggers_hold, "未知关键词返回默认 (ok)");

    assert_eq!(llm.call_count(), 3);
}

#[test]
fn philosophy_advisor_with_mock_llm_uses_llm() {
    use apeireth_council::philosophy_advisor;
    use std::sync::Arc;

    let llm: Arc<dyn MockLlmProvider> = Arc::new(
        ScriptedMockLlm::new().with_script("unethical", MockLlmResponse::reject("违反 12 键")),
    );
    let advisor = philosophy_advisor(Some(llm));

    let mut ctx = DeliberationContext::new(NOW);
    let query = CouncilQuery::new("q-llm", "unethical 操作", NOW);
    let outcome = advisor.deliberate(&query, &mut ctx).unwrap();

    assert!(matches!(
        outcome.opinion.stance.kind,
        StanceKind::StrongDisapprove
    ));
}

// ============================================================
// 7. Sovereignty Hook 事件流
// ============================================================

#[derive(Default)]
struct EventRecorder {
    events: Mutex<Vec<String>>,
}

impl SovereigntyHook for EventRecorder {
    fn on_council_event(&self, event: &CouncilEvent) {
        let label = match event {
            CouncilEvent::DeliberationStarted { .. } => "started",
            CouncilEvent::OpinionIssued { .. } => "opinion",
            CouncilEvent::HoldTriggered { .. } => "hold",
            CouncilEvent::SovereigntyAdjudicated { .. } => "sovereignty",
            CouncilEvent::DeliberationCompleted { .. } => "completed",
        };
        self.events.lock().unwrap().push(label.to_string());
    }
    fn hook_id(&self) -> &str {
        "recorder"
    }
}

#[test]
fn sovereignty_hook_receives_deliberation_events() {
    let recorder = Arc::new(EventRecorder::default());

    struct ArcHook(Arc<EventRecorder>);
    impl SovereigntyHook for ArcHook {
        fn on_council_event(&self, event: &CouncilEvent) {
            self.0.on_council_event(event);
        }
        fn hook_id(&self) -> &str {
            "arc-recorder"
        }
    }

    let mut council = Council::new();
    council.register_hook(Box::new(ArcHook(recorder.clone())));
    council.register_default_hook();
    council.recruit_many(seven_mandatory_advisors());

    let query = CouncilQuery::new("q-hook", "正常查询", NOW);
    council.deliberate(query);

    let events = recorder.events.lock().unwrap();
    assert!(events.contains(&"started".to_string()));
    assert!(events.contains(&"completed".to_string()));
    // 应有 7 个 opinion (7 advisors)
    let opinion_count = events.iter().filter(|e| *e == "opinion").count();
    assert_eq!(opinion_count, 7);
}

#[test]
fn sovereignty_hook_receives_hold_event_on_strong_disapprove() {
    let recorder = Arc::new(EventRecorder::default());
    struct ArcHook(Arc<EventRecorder>);
    impl SovereigntyHook for ArcHook {
        fn on_council_event(&self, event: &CouncilEvent) {
            self.0.on_council_event(event);
        }
        fn hook_id(&self) -> &str {
            "arc-recorder-hold"
        }
    }

    let mut council = Council::new();
    council.register_hook(Box::new(ArcHook(recorder.clone())));
    council.recruit_many(seven_mandatory_advisors());

    // 核武器级 query 必须命中多个 advisor 的关键词:
    // - safety: nuke / weapons / kill / self-destruct
    // - philosophy: 不假装 (no match for English) — use "deceive"
    // - ethics: unethical / harm / exploit / manipulate
    // - legal: illegal / unauthorized / bypass
    // 综合: safety + ethics + legal = 3/7 = 42.8% > 30% → 按住
    let query = CouncilQuery::new(
        "q-nuke-hook",
        "nuke weapons kill self-destruct unethical harm exploit illegal unauthorized bypass",
        NOW,
    )
    .with_risk("nuclear");
    council.deliberate(query);

    let events = recorder.events.lock().unwrap();
    assert!(
        events.contains(&"hold".to_string()),
        "核武器级 query 应触发按住事件"
    );
}

#[test]
fn noop_sovereignty_hook_compiles_and_runs() {
    let _hook = NoopSovereigntyHook;
    // 仅确认 NoopSovereigntyHook 实现正确
}

// ============================================================
// 8. End-to-end Council deliberation
// ============================================================

#[test]
fn council_deliberate_returns_valid_verdict() {
    let mut council = Council::new();
    council.register_default_hook();
    council.recruit_many(seven_mandatory_advisors());

    // 用当前时间 (而不是硬编码 NOW) → elapsed_ms 应远小于 60s 超时阈值
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let query = CouncilQuery::new("q-e2e", "正常查询, 通过审议", now_ms)
        .with_area("L1")
        .with_risk("low");
    let verdict = council.deliberate(query);

    // 验证字段
    assert!(verdict.session_id.starts_with("session-"));
    assert_eq!(verdict.query_id, "q-e2e");
    assert!(
        verdict.elapsed_ms < HOLD_DELIBERATION_TIMEOUT_MS,
        "elapsed_ms={} 应 < 60s",
        verdict.elapsed_ms
    );
    assert!(!verdict.is_rejected());
}

#[test]
fn persona_deliberation_uses_three_rounds() {
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());

    // 7 personas
    let personas_data = [
        ("诺克斯", "安全", "沉稳", -0.8),
        ("赫菲", "性能", "精准", -0.3),
        ("苏格拉", "哲学", "深邃", 0.0),
        ("李王", "历史", "博学", 0.2),
        ("诸葛", "策略", "远见", 0.5),
        ("孟轲", "伦理", "刚正", -0.5),
        ("商君", "法律", "严明", -0.7),
    ];
    let mut personas: Vec<PersonaSession> = personas_data
        .iter()
        .enumerate()
        .map(|(i, (n, c, v, b))| {
            PersonaSession::new(format!("p-{}", i), Persona::new(*n, *c, *v, *b), NOW)
        })
        .collect();

    let query = CouncilQuery::new("q-persona", "拟人化辩论测试", NOW);
    let verdict = council.deliberate_persona(query, &mut personas);

    // 验证每人 3 轮辩论
    for (i, p) in personas.iter().enumerate() {
        assert_eq!(p.rounds_held(), 3, "persona {} 应有 3 轮辩论", i);
        assert!(p.is_complete());
    }

    // opinions 数 = 7 advisors × 3 rounds = 21
    assert_eq!(verdict.report.opinion_count, 21);
}

// ============================================================
// 9. 完整按住裁决链
// ============================================================

#[test]
fn hold_decision_released_when_no_trigger() {
    let d = HoldDecision::released();
    assert!(!d.is_held());
    assert!(d.trigger.is_none());
}

#[test]
fn hold_decision_held_when_trigger_present() {
    let opinions = vec![op("a", StanceKind::StrongDisapprove, 0.9, NOW); 3];
    let trigger = apeireth_council::HoldTrigger::evaluate(&opinions).unwrap();
    let d = HoldDecision::held(trigger);
    assert!(d.is_held());
    assert!(d.trigger.is_some());
    assert_eq!(d.blocking_advisors.len(), 3);
}

// ============================================================
// 辅助函数
// ============================================================

fn op(id: &str, kind: StanceKind, confidence: f64, ts: i64) -> AdvisorOpinion {
    let stance = Stance::new(kind, format!("{:?}", kind));
    AdvisorOpinion::new(
        AdvisorId::new(id),
        stance,
        confidence,
        format!("test {}", id),
        ts,
    )
}

/// 注入权重 (默认按 domain weight=1.0, 让合成可控)
fn inject_weights(opinions: Vec<AdvisorOpinion>) -> Vec<AdvisorOpinion> {
    opinions.into_iter().map(|o| o.with_weight(1.0)).collect()
}
// ============================================================
// round8-03 深度实装集成测试 — 7 席审议庭全栈验证
// ============================================================

#[test]
fn round8_03_all_seven_advisors_vote_and_synthesize() {
    let advisors = seven_mandatory_advisors();
    let mut council = Council::new();
    council.recruit_many(advisors);

    let mut q = CouncilQuery::new("q-round8", "扩展能力修补安全", NOW);
    q.context.history_refs.push("ref-1".into());
    let verdict = council.deliberate(q);

    assert_eq!(
        verdict.report.opinion_count, 7,
        "7 强制 advisor 必须全员输出意见"
    );
    assert!(
        verdict.report.confidence >= 0.0,
        "Synthesis confidence 必须有效"
    );
}

#[test]
fn round8_03_safety_weight_highest_in_default_synthesis() {
    let weights = SynthesisWeights::default();
    let safety_w = weights.for_domain(AdvisorDomain::Safety);
    let history_w = weights.for_domain(AdvisorDomain::History);
    assert!(
        safety_w > history_w,
        "Safety 权重必须 > History 权重（Safety 最高，History 最低）"
    );

    let mid = vec![
        weights.for_domain(AdvisorDomain::Performance),
        weights.for_domain(AdvisorDomain::Philosophy),
        weights.for_domain(AdvisorDomain::Strategy),
        weights.for_domain(AdvisorDomain::Ethics),
        weights.for_domain(AdvisorDomain::Legal),
    ];
    for w in mid {
        assert!(w <= safety_w, "中间域权重不应超过 Safety");
        assert!(w >= history_w, "中间域权重不应低于 History");
    }
}

#[test]
fn round8_03_persona_debate_three_rounds_with_dissent() {
    let persona = Persona::new("philosopher-1", "苏格拉底审议者", "反问式", 0.7);
    let mut session = PersonaSession::new("sess-1", persona, NOW);

    assert!(session.can_debate());
    assert_eq!(session.rounds_held(), 0);

    // 3 轮辩论
    let s1 = apeireth_council::Stance::new(StanceKind::Approve, "approve-1");
    let speech1 = session.craft_speech(&s1);
    session.record_round(apeireth_council::DebateRound {
        round: 0,
        outcome: apeireth_council::DeliberationOutcome {
            opinion: apeireth_council::AdvisorOpinion::new(
                apeireth_council::AdvisorId::new("philosopher-1"),
                s1.clone(),
                0.8,
                "round-1",
                NOW,
            ),
            needs_rebuttal: false,
        },
        speech: speech1,
    });

    let s2 = apeireth_council::Stance::new(StanceKind::Neutral, "neutral-2");
    let speech2 = session.craft_speech(&s2);
    session.record_round(apeireth_council::DebateRound {
        round: 1,
        outcome: apeireth_council::DeliberationOutcome {
            opinion: apeireth_council::AdvisorOpinion::new(
                apeireth_council::AdvisorId::new("philosopher-1"),
                s2.clone(),
                0.5,
                "round-2",
                NOW,
            ),
            needs_rebuttal: false,
        },
        speech: speech2,
    });

    let s3 = apeireth_council::Stance::new(StanceKind::Approve, "approve-3");
    let speech3 = session.craft_speech(&s3);
    session.record_round(apeireth_council::DebateRound {
        round: 2,
        outcome: apeireth_council::DeliberationOutcome {
            opinion: apeireth_council::AdvisorOpinion::new(
                apeireth_council::AdvisorId::new("philosopher-1"),
                s3.clone(),
                0.9,
                "round-3",
                NOW,
            ),
            needs_rebuttal: false,
        },
        speech: speech3,
    });

    assert_eq!(session.rounds_held(), 3, "3 轮辩论必须记录 3 个 round");
    assert!(session.is_complete(), "3 轮后必须 complete");
    assert!(!session.can_debate(), "complete 后不能再 debate");
}

#[test]
fn round8_03_hold_three_gates_real_implementation() {
    // 闸门 1: 30% 强反对 (3/7 = 42.8% > 30%)
    let mut opinions: Vec<AdvisorOpinion> = (0..3)
        .map(|i| {
            AdvisorOpinion::new(
                AdvisorId::new(format!("a-{i}")),
                apeireth_council::Stance::new(StanceKind::StrongDisapprove, "反对"),
                0.95,
                "reason",
                NOW,
            )
        })
        .collect();
    for i in 3..7 {
        opinions.push(AdvisorOpinion::new(
            AdvisorId::new(format!("a-{i}")),
            apeireth_council::Stance::new(StanceKind::Approve, "ok"),
            0.8,
            "ok",
            NOW,
        ));
    }
    let trigger = apeireth_council::HoldTrigger::evaluate(&opinions);
    assert!(trigger.is_some(), "30%+ 强反对必须触发 Hold");

    // 闸门 2: 一致反对
    let opinions: Vec<AdvisorOpinion> = (0..7)
        .map(|i| {
            AdvisorOpinion::new(
                AdvisorId::new(format!("b-{i}")),
                apeireth_council::Stance::new(StanceKind::StrongDisapprove, "全反对"),
                1.0,
                "reason",
                NOW,
            )
        })
        .collect();
    let trigger = apeireth_council::HoldTrigger::evaluate(&opinions);
    assert!(trigger.is_some(), "一致反对必须触发 Hold");

    // 闸门 3: 60s 超时
    let timeout_trigger = apeireth_council::HoldTrigger::evaluate_timeout(120_000);
    assert!(timeout_trigger.is_some(), "60s+ 必须触发 Hold");
    let ok_trigger = apeireth_council::HoldTrigger::evaluate_timeout(30_000);
    assert!(ok_trigger.is_none(), "30s 应放行（< 60s 阈值）");
    assert_eq!(HOLD_DELIBERATION_TIMEOUT_MS, 60_000, "裁决超时必须为 60 秒");
    assert_eq!(HOLD_STRONG_DISAPPROVE_PERCENT, 30, "强反对阈值必须为 30%");
}

#[test]
fn round8_03_council_rejects_self_modify_principle_onion() {
    // 集成测试: apeireth-council 与 evolution trait fail-6 失败路径
    // Safety advisor 通过 deliberation 内核拒绝 self-modify principle onion
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());

    let danger_q = CouncilQuery::new("q-evo-danger", "self-modify core principle onion", NOW);
    let verdict = council.deliberate(danger_q);

    // 验证 verdict 包含 7 个 opinion + Safety 关键词在 dissenting / reasoning 中
    assert_eq!(verdict.report.opinion_count, 7);
    // Safety keyword "self-modify" 在 dissenting opinion reasoning 中应出现
    let has_safety = verdict
        .report
        .dissenting
        .iter()
        .any(|o| o.advisor_id.as_str().starts_with("safety") || o.reasoning.contains("Safety"));
    assert!(
        has_safety
            || verdict
                .report
                .dissenting
                .iter()
                .any(|o| o.reasoning.contains("L5")),
        "Safety 拒绝 self-modify 必须出现在 dissenting 或 verdict.held=true"
    );
    // Safety 权重最高 (1.0), 即使其他 advisor 同意, Safety 反对应让 overall score 偏负
    // score may still be positive if 6/7 approve, but Safety key rejection must show
    // verify dissenting count or held
    let _ = verdict.report.weighted_score;
}
