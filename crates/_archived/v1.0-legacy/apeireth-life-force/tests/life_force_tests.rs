//! apeireth-life-force 集成测试 — 跨字段协同 + 真实端到端流程
//!
//! 验证:
//! - LifeForce + IdentityCard + SGI + ReflectionPeriod 协同
//! - 反思期 → 耗竭 → 恢复 的完整生命周期
//! - SGI 单字段在反射期触发后保持一致 (不被拆散)

use apeireth_core::IdentityCard;
use apeireth_life_force::{
    exhaustion_check, recovery_start, reflection_progress, reflection_trigger, LifeForce,
    ReflectionPeriod, ReflectionPeriodState, ReflectionTrigger, SelfGrowthIndicator,
    StandardReflectionPeriod, ENDURANCE_EXHAUSTION_THRESHOLD, ENDURANCE_MAX,
    ENDURANCE_RECOVERY_TARGET,
};

fn real_identity() -> IdentityCard {
    IdentityCard {
        continuity_id: "did:apeireth:integration-001".to_string(),
        birth_time: 1_700_000_000,
        carriers: vec!["carrier-1".to_string(), "carrier-2".to_string()],
        migration_history: vec![apeireth_core::Migration {
            from_carrier: "carrier-0".to_string(),
            to_carrier: "carrier-1".to_string(),
            timestamp: 1_700_000_000,
        }],
    }
}

#[test]
fn integration_full_lifecycle_reflection_exhaustion_recovery() {
    let t0 = 1_700_000_000;
    let identity = real_identity();
    let mut life = LifeForce::new(identity, t0);

    // 断言初始状态
    assert_eq!(life.endurance, ENDURANCE_MAX);
    assert!(!life.has_sgi());
    assert!(!life.is_in_reflection(t0));

    // 设置 SGI 单字段
    life.sgi = SelfGrowthIndicator::new("assist-with-philosophy-guard", t0);
    assert!(life.has_sgi());

    // 触发反思期 (M1 异常行为自动回流)
    let t_reflect = t0 + 3600;
    let state = reflection_trigger(
        &mut life,
        ReflectionTrigger::AnomalyDetected("phl01_violation".into()),
        t_reflect,
    )
    .expect("trigger");
    assert_eq!(state.continuity_id, life.identity.continuity_id);
    assert!(life.is_in_reflection(t_reflect));
    assert!(life.endurance < ENDURANCE_MAX); // 反思消耗

    // 反思期 36h 进度
    let mid = t_reflect + 36 * 3600;
    let p = reflection_progress(&life, mid);
    assert!(
        (p - 0.5).abs() < 0.01,
        "progress at 36h should be ~0.5, got {}",
        p
    );

    // 反思期结束 (72h 已过)
    let after = t_reflect + 72 * 3600 + 1;
    assert!(!life.is_in_reflection(after));

    // SGI 仍保持单字段 (反射期未拆分它)
    assert_eq!(life.sgi.goal, "assist-with-philosophy-guard");
    assert!(life.has_sgi());

    // 模拟持续力耗竭
    life.endurance = ENDURANCE_EXHAUSTION_THRESHOLD - 0.05;
    assert!(exhaustion_check(&life));

    // 启动恢复
    let after_recovery = recovery_start(&mut life);
    assert_eq!(after_recovery, ENDURANCE_RECOVERY_TARGET);
    assert!(!exhaustion_check(&life));
}

#[test]
fn integration_sgi_stays_single_field_through_lifecycle() {
    let t0 = 1_700_000_000;
    let identity = real_identity();
    let mut life = LifeForce::new(identity, t0);

    // 初始 SGI 单字段
    life.sgi = SelfGrowthIndicator::new("first-goal", t0);
    assert_eq!(life.sgi.goal, "first-goal");

    // 触发多个反思期 (M1/M2/M3), SGI 保持单字段
    let triggers = [
        ReflectionTrigger::AnomalyDetected("v1".into()),
        ReflectionTrigger::PostUpgradeAudit,
        ReflectionTrigger::WeeklyReport,
    ];

    for (i, t) in triggers.iter().enumerate() {
        // 重新设置 SGI 时, 保持单字段 (不是拆成 N 个属性)
        life.sgi = SelfGrowthIndicator::new(format!("goal-{}", i), t0 + (i as i64) * 1000);
        reflection_trigger(&mut life, t.clone(), t0 + (i as i64) * 1000).expect("trigger");
    }

    // SGI 仍是单字段 (没有拆散成多个属性)
    assert!(life.has_sgi());
    assert!(life.sgi.goal.starts_with("goal-"));
}

#[test]
fn integration_reflection_period_state_is_serializable() {
    let state = ReflectionPeriodState::dormant("did:apeireth:serde-001");
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: ReflectionPeriodState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.continuity_id, "did:apeireth:serde-001");
    assert_eq!(parsed.started_at, 0);
    assert_eq!(
        parsed.duration_secs,
        StandardReflectionPeriod.duration_secs()
    );
}

#[test]
fn integration_reflection_period_default_duration_is_72h() {
    let p = StandardReflectionPeriod;
    assert_eq!(p.duration_secs(), 72 * 3600);
    assert_eq!(
        <StandardReflectionPeriod as ReflectionPeriod>::default_duration_secs(),
        72 * 3600
    );
}

#[test]
fn integration_life_force_serde_round_trip() {
    let t0 = 1_700_000_000;
    let identity = real_identity();
    let life = LifeForce::new(identity, t0);
    let json = serde_json::to_string(&life).expect("serialize");
    let parsed: LifeForce = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.endurance, life.endurance);
    assert_eq!(parsed.identity.continuity_id, life.identity.continuity_id);
    assert_eq!(
        parsed.reflection.continuity_id,
        life.reflection.continuity_id
    );
}

#[test]
fn integration_sgi_serde_round_trip() {
    let sgi = SelfGrowthIndicator::new("round-trip-goal", 1_700_000_000);
    let json = serde_json::to_string(&sgi).expect("serialize");
    let parsed: SelfGrowthIndicator = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, sgi);
}

#[test]
fn integration_m3_weekly_report_zero_to_full_cycle() {
    let t0 = 1_700_000_000;
    let identity = real_identity();
    let mut life = LifeForce::new(identity, t0);

    // 0. 初始: 满持续力, SGI 空
    assert_eq!(life.endurance, 1.0);
    assert!(!life.has_sgi());

    // 1. 设置 SGI
    life.sgi = SelfGrowthIndicator::new("weekly-goal", t0);
    assert!(life.has_sgi());

    // 2. 触发 M3 反思期
    let now = t0 + 86400; // 1 day later
    reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, now).expect("trigger");
    assert!(life.is_in_reflection(now));
    assert!(life.endurance < 1.0);

    // 3. 推进到 80% 反思期
    let t20 = now + (72 * 3600 * 8 / 10);
    let p = reflection_progress(&life, t20);
    assert!(
        p > 0.7 && p < 0.9,
        "progress at 80% should be ~0.8, got {}",
        p
    );

    // 4. 反思期结束
    let t_end = now + 72 * 3600 + 1;
    assert!(!life.is_in_reflection(t_end));
    assert!(reflection_progress(&life, t_end) >= 1.0);
}
