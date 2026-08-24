//! apeireth-life-force v2 — Emergence + ReflectionCycle 跨模块集成测试.
//!
//! 验证 v1 emergence.rs + reflection_cycle.rs 模块的 API 表面在 v2 中端到端协同:
//! 1. EmergenceDetector 记录跨域洞察信号
//! 2. ReflectionCycleScheduler 推进 4 阶段
//! 3. 两者通过 continuity_id 协同 (跨载体同 ID)

use apeireth_core::IdentityCard;
use apeireth_life_force::emergence::{
    EmergenceDetector, EmergenceSignal, EmergenceSignalType,
};
use apeireth_life_force::reflection_cycle::{
    ReflectionCycleScheduler, ReflectionPhase,
};
use apeireth_life_force::{
    exhaustion_check, recovery_start, reflection_trigger, LifeForce, ReflectionTrigger,
    SelfGrowthIndicator,
};

fn make_continuity_id() -> String {
    "did:apeireth:emergence-reflection-001".to_string()
}

fn make_identity() -> IdentityCard {
    IdentityCard {
        continuity_id: make_continuity_id(),
        birth_time: 1_700_000_000,
        carriers: vec!["emergence-carrier".to_string()],
        migration_history: vec![],
        ..Default::default()
    }
}

#[test]
fn v2_emergence_detector_records_signals_and_filters() {
    let mut detector = EmergenceDetector::new(&make_continuity_id());

    let signal = EmergenceSignal {
        signal_type: EmergenceSignalType::CrossDomainInsight,
        confidence: 0.85,
        evidence: vec!["domain-A-mapping".to_string(), "domain-B-validated".to_string()],
        ts: 1_700_001_000,
        continuity_id: make_continuity_id(),
    };
    detector.record(signal.clone()).expect("record");
    assert_eq!(detector.len(), 1);
    // total_recorded 字段为 private, 通过 snapshot 总数间接验证

    let report = detector.snapshot(1_700_001_500);
    assert_eq!(report.total_signals_recorded, 1);
    assert_eq!(report.signals_above_threshold.len(), 1);
    assert_eq!(report.threshold, apeireth_life_force::emergence::DEFAULT_EMERGENCE_THRESHOLD);
}

#[test]
fn v2_reflection_cycle_full_4_phase_cycle() {
    let cid = make_continuity_id();
    let mut scheduler = ReflectionCycleScheduler::new(&cid, 1_700_000_000);

    // 初始 = Triggered
    assert_eq!(scheduler.current, ReflectionPhase::Triggered);
    assert_eq!(scheduler.cycles_completed, 0);

    // Triggered → Reflecting
    scheduler.advance(ReflectionPhase::Reflecting, 1_700_000_100).unwrap();
    assert_eq!(scheduler.current, ReflectionPhase::Reflecting);

    // Reflecting → Consolidating
    scheduler.advance(ReflectionPhase::Consolidating, 1_700_000_200).unwrap();
    assert_eq!(scheduler.current, ReflectionPhase::Consolidating);

    // Consolidating → Concluded (auto retrigger → Triggered)
    scheduler.advance(ReflectionPhase::Concluded, 1_700_000_300).unwrap();
    assert_eq!(scheduler.current, ReflectionPhase::Triggered);
    assert_eq!(scheduler.cycles_completed, 1);
}

#[test]
fn v2_reflection_cycle_rejects_invalid_transition() {
    let mut scheduler = ReflectionCycleScheduler::new(&make_continuity_id(), 1_700_000_000);
    // Triggered → Consolidating 是非法 (必须经过 Reflecting)
    let res = scheduler.advance(ReflectionPhase::Consolidating, 1_700_000_001);
    assert!(res.is_err());
    assert_eq!(scheduler.current, ReflectionPhase::Triggered);
}

#[test]
fn v2_emergence_and_life_force_share_continuity_id() {
    // v2: continuity_id 派生自 identity.name (per continuity_id_from_identity)
    let cid = make_continuity_id();
    let mut identity = make_identity();
    identity.name = cid.clone(); // v2: continuity_id = identity.name

    let mut life = LifeForce::new(identity, 1_700_000_000);
    life.sgi = SelfGrowthIndicator::new("cross-domain-research", 1_700_000_000);

    // 触发反思期
    let trigger = reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_500);
    assert!(trigger.is_ok(), "反思期应成功触发");

    // emergence detector 共享同一 continuity_id
    let mut detector = EmergenceDetector::new(&cid);
    let signal = EmergenceSignal {
        signal_type: EmergenceSignalType::CrossDomainInsight,
        confidence: 0.9,
        evidence: vec!["evidence-1".to_string(), "evidence-2".to_string()],
        ts: 1_700_000_600,
        continuity_id: cid.clone(),
    };
    assert!(detector.record(signal).is_ok());
    assert_eq!(life.reflection.continuity_id, detector.continuity_id);
}

#[test]
fn v2_emergence_detector_continuity_mismatch_rejected() {
    let mut detector = EmergenceDetector::new("did:apeireth:owner");
    let bad_signal = EmergenceSignal {
        signal_type: EmergenceSignalType::CrossDomainInsight,
        confidence: 0.8,
        evidence: vec!["a".to_string(), "b".to_string()],
        ts: 0,
        continuity_id: "did:apeireth:other".to_string(),
    };
    let res = detector.record(bad_signal);
    assert!(res.is_err(), "continuity_id 不匹配应被拒绝");
}

#[test]
fn v2_emergence_threshold_clamps_to_unit_range() {
    let d = EmergenceDetector::with_threshold("did:test", 1.5);
    assert_eq!(d.threshold, 1.0, "1.5 应 clamp 到 1.0");
    let d2 = EmergenceDetector::with_threshold("did:test", -0.5);
    assert_eq!(d2.threshold, 0.0, "-0.5 应 clamp 到 0.0");
}

#[test]
fn v2_life_force_exhaustion_recovery_via_emergence_signal() {
    // 完整生命周期: 反思期 → 持续力下降 → 耗竭检查 → 恢复
    let identity = make_identity();
    let mut life = LifeForce::new(identity, 1_700_000_000);
    life.sgi = SelfGrowthIndicator::new("guard-philosophy", 1_700_000_000);

    // 多次反思触发 → endurance 下降
    for i in 0..5 {
        let _ = reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_000 + i * 100);
    }

    // 强制设为耗竭
    life.endurance = 0.1;
    assert!(exhaustion_check(&life), "endurance=0.1 应耗竭");

    // 恢复启动
    let after = recovery_start(&mut life);
    assert!(!exhaustion_check(&life));
    assert!(after >= 0.8);

    // emergence signal 记录此恢复 (作为 cross-domain insight)
    // v2: continuity_id 派生自 identity.name
    let mut detector = EmergenceDetector::new(&life.identity.name);
    let signal = EmergenceSignal {
        signal_type: EmergenceSignalType::RecursiveImprovement,
        confidence: 0.85,
        evidence: vec!["recovery-cycle".to_string(), "endurance-restored".to_string()],
        ts: 1_700_001_000,
        continuity_id: life.identity.name.clone(),
    };
    detector.record(signal).expect("record emergence");
    let report = detector.snapshot(1_700_001_500);
    assert_eq!(report.signals_above_threshold.len(), 1);
}

#[test]
fn v2_reflection_cycle_event_history_lifo() {
    let mut scheduler = ReflectionCycleScheduler::new(&make_continuity_id(), 1_700_000_000);
    scheduler.advance(ReflectionPhase::Reflecting, 1_700_000_100).unwrap();
    scheduler.advance(ReflectionPhase::Consolidating, 1_700_000_200).unwrap();

    let events = scheduler.recent_events(2);
    assert_eq!(events.len(), 2);
    // LIFO: 最新在前
    assert_eq!(events[0].phase, ReflectionPhase::Consolidating);
    assert_eq!(events[1].phase, ReflectionPhase::Reflecting);
}

#[test]
fn v2_reflection_cycle_validate_continuity_pass() {
    let scheduler = ReflectionCycleScheduler::new(&make_continuity_id(), 0);
    assert!(scheduler.validate_continuity(&make_continuity_id()).is_ok());
    assert!(scheduler.validate_continuity("did:apeireth:other").is_err());
}
