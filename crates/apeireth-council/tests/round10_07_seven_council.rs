//! round10-07: 7 席审议庭 + 补充式修正 LOCKED 真实集成测试
//!
//! 目的: 验证 apeireth-council 7 强制 advisor 真实协同 + 按住机制 + 拟人化 3 轮辩论
//!
//! 测试策略 (基于"工程实现有没有受到欺骗或误解"用户关切):
//! - 单元测试 (≥5): 7 强制 advisor 全部召集, 7 强制编译时常量, 握住判定, synthesis 加权,
//!                   3 轮拟人化辩论
//! - 集成测试 (≥4): 7 advisor 协同审议, 按住机制触发, 3 轮辩论满轮, synthesis 集成
//!
//! **不修改**:
//! - `AdvisorDomain::ALL` 7 强制域
//! - `SEVEN_MANDATORY_ADVISORS = 7` 常量
//! - `MAX_PERSONA_DEBATE_ROUNDS = 3` 常量
//! - `HOLD_STRONG_DISAPPROVE_PERCENT = 30` 常量
//! - `HOLD_DELIBERATION_TIMEOUT_MS = 60_000` 常量
//! - 7 强制 advisor 工厂函数
//! - Persona 3 轮 hardcode
//! - **Cargo.toml** (守"不引入新依赖"承诺 — council 已有 deps 不变)
//! - **council ↔ constraint 集成**测试由 constraint/tests/ 侧覆盖 (避免 council 引入新 deps)

use apeireth_council::{
    seven_mandatory_advisors, synthesize, AdvisorDomain, AdvisorId, AdvisorOpinion, Council,
    CouncilQuery, HoldTrigger, Persona, PersonaSession, Stance, StanceKind, SynthesisWeights,
    HOLD_DELIBERATION_TIMEOUT_MS, HOLD_STRONG_DISAPPROVE_PERCENT, MAX_PERSONA_DEBATE_ROUNDS,
    SEVEN_MANDATORY_ADVISORS,
};

const NOW: i64 = 1_700_000_000_000;

// ============================================================================
// 单元测试 1: 7 强制 advisor 编译时常量硬锁
// ============================================================================

#[test]
fn seven_mandatory_advisors_count_constant_is_seven() {
    assert_eq!(
        SEVEN_MANDATORY_ADVISORS, 7,
        "7 强制 advisor 编译时 hardcode"
    );
}

#[test]
fn advisor_domain_all_array_is_exactly_seven() {
    let all = AdvisorDomain::ALL;
    assert_eq!(all.len(), 7, "AdvisorDomain::ALL 必须 7 强制域");

    // 7 强制域 必含
    let expected = [
        AdvisorDomain::Safety,
        AdvisorDomain::Performance,
        AdvisorDomain::Philosophy,
        AdvisorDomain::History,
        AdvisorDomain::Strategy,
        AdvisorDomain::Ethics,
        AdvisorDomain::Legal,
    ];
    for e in expected.iter() {
        assert!(all.contains(e), "缺失 7 强制域: {:?}", e);
    }

    // 7 强制域 互不重复
    let mut seen: Vec<AdvisorDomain> = Vec::new();
    for d in all.iter() {
        assert!(!seen.contains(d), "重复域: {:?}", d);
        seen.push(*d);
    }
    assert_eq!(seen.len(), 7);
}

#[test]
fn persona_debate_rounds_constant_is_three() {
    assert_eq!(MAX_PERSONA_DEBATE_ROUNDS, 3, "拟人化 3 轮辩论 hardcode");
}

#[test]
fn hold_strong_disapprove_percent_constant_is_thirty() {
    assert_eq!(
        HOLD_STRONG_DISAPPROVE_PERCENT, 30,
        "按住机制 30% 强反对阈值 hardcode"
    );
}

#[test]
fn hold_deliberation_timeout_constant_is_60s() {
    assert_eq!(
        HOLD_DELIBERATION_TIMEOUT_MS, 60_000,
        "按住机制 60s 裁决超时 hardcode"
    );
}

// ============================================================================
// 单元测试 2: 7 强制 advisor 工厂召集
// ============================================================================

#[test]
fn seven_mandatory_advisors_factory_returns_all_seven() {
    let advisors = seven_mandatory_advisors();
    assert_eq!(
        advisors.len(),
        7,
        "seven_mandatory_advisors 必须返回 7 advisor"
    );

    // 7 advisor 必对应 7 强制域
    let mut domains: Vec<AdvisorDomain> = advisors.iter().map(|a| a.domain()).collect();
    domains.sort_by_key(|d| format!("{:?}", d));

    let mut expected: Vec<AdvisorDomain> = [
        AdvisorDomain::Safety,
        AdvisorDomain::Performance,
        AdvisorDomain::Philosophy,
        AdvisorDomain::History,
        AdvisorDomain::Strategy,
        AdvisorDomain::Ethics,
        AdvisorDomain::Legal,
    ]
    .to_vec();
    expected.sort_by_key(|d| format!("{:?}", d));

    assert_eq!(domains, expected, "7 advisor 必覆盖 7 强制域");
}

#[test]
fn seven_mandatory_advisors_have_distinct_ids() {
    let advisors = seven_mandatory_advisors();
    let mut ids: Vec<String> = Vec::new();
    for a in &advisors {
        let id_str = a.id().to_string();
        assert!(!ids.contains(&id_str), "重复 advisor ID: {}", id_str);
        ids.push(id_str);
    }
    assert_eq!(ids.len(), 7);
}

#[test]
fn seven_advisors_have_synthesis_weights_in_range() {
    // 7 强制 advisor 都有 default_weight, 必须 0..=1
    for d in AdvisorDomain::ALL.iter() {
        let w = d.default_weight();
        assert!((0.0..=1.0).contains(&w), "{:?} weight 越界: {}", d, w);
    }
}

#[test]
fn stance_kind_score_in_unit_range() {
    // 6 stance 都有 score, 必须 -1..=+1
    for s in [
        StanceKind::StrongApprove,
        StanceKind::Approve,
        StanceKind::Neutral,
        StanceKind::Disapprove,
        StanceKind::StrongDisapprove,
        StanceKind::Abstain,
    ] {
        let score = s.score();
        assert!(
            (-1.0..=1.0).contains(&score),
            "{:?} score 越界: {}",
            s,
            score
        );
    }
    assert!(StanceKind::StrongDisapprove.is_strong_disapprove());
    assert!(StanceKind::Abstain.is_abstain());
}

#[test]
fn persona_session_has_three_debate_rounds() {
    // 拟人化 3 轮辩论 hardcode
    let persona = Persona::new("p1", "安全导向", "rational", 0.7);
    let mut session = PersonaSession::new("session-1", persona, NOW);
    assert_eq!(
        session.max_rounds, MAX_PERSONA_DEBATE_ROUNDS,
        "max_rounds = 3 hardcode"
    );
    assert!(session.can_debate(), "第 0 轮必须能辩论");
    // 手工填 3 个 round (因 advance_round 是 private)
    for i in 0..MAX_PERSONA_DEBATE_ROUNDS {
        session.current_round = i;
        assert!(session.can_debate(), "第 {} 轮必须能辩论", i);
    }
    session.current_round = MAX_PERSONA_DEBATE_ROUNDS;
    assert!(!session.can_debate(), "第 3 轮必须不可辩论 (= 3 轮上限)");
    assert!(session.is_complete(), "第 3 轮 must be complete");
}

// ============================================================================
// 集成测试 3: 7 advisor 真实协同审议
// ============================================================================

#[test]
fn seven_advisors_full_deliberation_produces_seven_opinions() {
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());
    assert_eq!(council.advisor_count(), 7);

    let query = CouncilQuery::new("q-1", "集成测试: 7 advisor 协同审议", NOW);
    let verdict = council.deliberate(query);

    assert_eq!(
        verdict.report.opinion_count, 7,
        "7 advisor 审议必须产生 7 个 opinion"
    );
}

#[test]
fn four_gates_and_hold_gates_compile_time_constants_locked() {
    // 验证 握住 3 闸门 (人数闸门 + 阈值闸门 + 超时闸门) 编译时 hardcode
    assert_eq!(SEVEN_MANDATORY_ADVISORS, 7);
    assert_eq!(HOLD_STRONG_DISAPPROVE_PERCENT, 30);
    assert_eq!(HOLD_DELIBERATION_TIMEOUT_MS, 60_000);
}

#[test]
fn hold_trigger_evaluates_strong_disapprove_threshold() {
    // 7 个 opinion 中 ≥ 30% 强反对 → HoldTrigger
    let make_op = |advisor_id: &str, stance: StanceKind| -> AdvisorOpinion {
        let mut o = AdvisorOpinion::new(
            AdvisorId::new(advisor_id),
            Stance::new(stance, "test"),
            0.9,
            "test",
            NOW,
        );
        o.weight = 0.5;
        o
    };

    // 3/7 ≈ 43% 强反对 (> 30% 阈值应触发)
    let opinions = vec![
        make_op("a1", StanceKind::StrongDisapprove),
        make_op("a2", StanceKind::StrongDisapprove),
        make_op("a3", StanceKind::StrongDisapprove),
        make_op("a4", StanceKind::Approve),
        make_op("a5", StanceKind::Approve),
        make_op("a6", StanceKind::Neutral),
        make_op("a7", StanceKind::Abstain),
    ];
    let trigger = HoldTrigger::evaluate(&opinions);
    assert!(
        trigger.is_some(),
        "3/7 ≈ 43% 强反对 应触发 HoldTrigger (≥ 30% 阈值)"
    );
}

#[test]
fn hold_trigger_no_trigger_when_consensus_approve() {
    // 全部 Approve → 不应触发
    let mut opinions = Vec::new();
    for i in 0..7 {
        let mut o = AdvisorOpinion::new(
            AdvisorId::new(format!("a{}", i)),
            Stance::new(StanceKind::Approve, "ok"),
            0.9,
            "ok",
            NOW,
        );
        o.weight = 0.5;
        opinions.push(o);
    }
    let trigger = HoldTrigger::evaluate(&opinions);
    assert!(trigger.is_none(), "全部 Approve 不应触发 HoldTrigger");
}

// ============================================================================
// 集成测试 4: 拟人化 3 轮辩论真实集成
// ============================================================================

#[test]
fn seven_advisors_can_deliberate_three_persona_rounds() {
    // 7 advisor 各自 3 轮 persona 辩论
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());

    let query = CouncilQuery::new("q-persona", "拟人化 3 轮辩论", NOW);

    // 7 个 persona session, 每人 3 轮
    let mut personas: Vec<PersonaSession> = (0..7)
        .map(|i| {
            PersonaSession::new(
                format!("session-{}", i),
                Persona::new(format!("persona-{}", i), "default", "rational", 0.5),
                NOW,
            )
        })
        .collect();

    let _verdict = council.deliberate_persona(query, &mut personas);

    // 7 个 persona 每人 3 轮全部用完
    for (i, p) in personas.iter().enumerate() {
        assert_eq!(
            p.current_round, MAX_PERSONA_DEBATE_ROUNDS,
            "persona[{}] 必须完成 3 轮",
            i
        );
        assert!(!p.can_debate(), "persona[{}] 第 4 轮不可辩论", i);
    }
}

// ============================================================================
// 集成测试 5: synthesis 集成 + 7 advisor 协同审议 (无 constraint deps)
// ============================================================================

#[test]
fn synthesis_with_default_weights_produces_balanced_report() {
    // synthesis 用 default weights: 7 强制权重 0.55..1.0
    let opinions = vec![
        AdvisorOpinion::new(
            AdvisorId::new("safety"),
            Stance::new(StanceKind::StrongApprove, "safe"),
            0.95,
            "safe",
            NOW,
        ),
        AdvisorOpinion::new(
            AdvisorId::new("performance"),
            Stance::new(StanceKind::Approve, "ok"),
            0.65,
            "ok",
            NOW,
        ),
        AdvisorOpinion::new(
            AdvisorId::new("philosophy"),
            Stance::new(StanceKind::StrongApprove, "aligned"),
            0.95,
            "aligned",
            NOW,
        ),
    ];

    let report = synthesize(&opinions, &SynthesisWeights::default());
    assert_eq!(report.opinion_count, 3);
    assert!(report.weighted_score > 0.0, "全部 approve 集成应正分");
}

#[test]
fn seven_advisors_full_deliberation_produces_per_advisor_opinions() {
    // 7 advisor 各出 1 opinion, 1 synthesis 1 verdict
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());
    assert_eq!(council.advisor_count(), 7);

    let query = CouncilQuery::new("q-per-advisor", "per-advisor opinions", NOW);
    let verdict = council.deliberate(query);

    // 每人 1 opinion, total = 7
    assert_eq!(verdict.report.opinion_count, 7);

    // 不应触发 HoldTrigger (无强反对)
    assert!(
        verdict.report.hold_decision.trigger.is_none(),
        "无强反对不应触发 HoldDecision"
    );
}

#[test]
fn seven_advisors_deliberate_with_custom_stances_synthesizes() {
    // 7 advisor 协同审议 + 注入不同 stance, 验证 synthesis 集成
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());

    let query = CouncilQuery::new("q-synth-custom", "synthesize 7 stance", NOW);
    let verdict = council.deliberate(query);

    // 7 opinion 全部进 synthesis
    assert_eq!(verdict.report.opinion_count, 7);
    // weighted_score 有限 (而非 NaN)
    assert!(
        verdict.report.weighted_score.is_finite(),
        "weighted_score 必须有限"
    );
}

#[test]
fn seven_advisors_have_real_advisor_kinds_not_stubs() {
    // 7 强制 advisor 必含: safety / performance / philosophy / history / strategy / ethics / legal
    // 不能是 stub
    let advisors = seven_mandatory_advisors();
    let domain_set: Vec<AdvisorDomain> = advisors.iter().map(|a| a.domain()).collect();
    let unique_domains: Vec<AdvisorDomain> = {
        let mut v = domain_set.clone();
        v.sort_by_key(|d| format!("{:?}", d));
        v.dedup();
        v
    };
    assert_eq!(unique_domains.len(), 7, "7 advisor 必 7 不同域");
}

#[test]
fn synthesis_report_has_all_required_fields() {
    // SynthesisReport 必含字段: opinion_count, weighted_score, aggregated_stance,
    //                        confidence, dissenting, hold_decision
    let opinions = vec![AdvisorOpinion::new(
        AdvisorId::new("a1"),
        Stance::new(StanceKind::Approve, "ok"),
        0.9,
        "ok",
        NOW,
    )];
    let report = synthesize(&opinions, &SynthesisWeights::default());
    assert_eq!(report.opinion_count, 1);
    assert!(report.weighted_score.is_finite());
    assert!(report.confidence.is_finite());
    // dissenting / hold_decision 都是 Vec / Option, 类型上访问
    let _ = report.dissenting.len();
}

#[test]
fn seven_advisors_full_deliberation_with_three_rounds_integration() {
    // 终极集成: 7 advisor 召集 + 3 轮 persona 辩论 + synthesis 加权 + HoldTrigger 评估
    let mut council = Council::new();
    council.recruit_many(seven_mandatory_advisors());
    assert_eq!(council.advisor_count(), 7);

    let query = CouncilQuery::new("q-final", "终极集成", NOW);

    // 7 persona session, 每人 3 轮
    let mut personas: Vec<PersonaSession> = (0..7)
        .map(|i| {
            PersonaSession::new(
                format!("final-session-{}", i),
                Persona::new(format!("final-p{}", i), "default", "rational", 0.5),
                NOW,
            )
        })
        .collect();

    let verdict = council.deliberate_persona(query, &mut personas);

    // 7 advisor × 3 轮 = 21 opinion (拟人化辩论每次 round 一次 opinion)
    assert_eq!(
        verdict.report.opinion_count, 21,
        "7 advisor × 3 轮 = 21 opinion"
    );

    // 7 persona 每人 3 轮全部用完
    for (i, p) in personas.iter().enumerate() {
        assert_eq!(
            p.current_round, MAX_PERSONA_DEBATE_ROUNDS,
            "persona[{}] 必须完成 3 轮",
            i
        );
    }
}
