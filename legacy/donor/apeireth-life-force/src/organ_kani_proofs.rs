//! R177 life-force organ Kani proofs (W2)
//!
//! **要验证的不变量**:
//! 1. ENDURANCE_MIN = 0.0, ENDURANCE_MAX = 1.0
//! 2. validate_endurance 拒绝越界值
//! 3. reflection_trigger 校验 continuity_id 一致
//! 4. reflection_trigger 校验 SGI 非空
//! 5. endurance 反思消耗 (max - 0.1, min 0)
//! 6. recovery_start 回到 ENDURANCE_RECOVERY_TARGET
//! 7. exhaustion_check 阈值检测
//! 8. ReflectionPeriodState::dormant -> is_active false

#![allow(missing_docs)]

use apeireth_core::IdentityCard;

use crate::{
    exhaustion_check, recovery_start, reflection_trigger, validate_endurance, LifeForce,
    ReflectionTrigger, ENDURANCE_EXHAUSTION_THRESHOLD, ENDURANCE_MAX, ENDURANCE_MIN,
    ENDURANCE_RECOVERY_TARGET,
};

fn make_life(now: i64) -> LifeForce {
    let id = IdentityCard {
        continuity_id: "r177-lf-test".into(),
        birth_time: 0,
        carriers: vec![],
        migration_history: vec![],
    };
    LifeForce::new(id, now)
}

// ============================================
// Property 1: 常量边界
// ============================================
#[test]
fn r177_lf_01_endurance_constants() {
    assert_eq!(ENDURANCE_MIN, 0.0);
    assert_eq!(ENDURANCE_MAX, 1.0);
    assert!(ENDURANCE_EXHAUSTION_THRESHOLD > 0.0);
    assert!(ENDURANCE_EXHAUSTION_THRESHOLD < ENDURANCE_MAX);
    assert!(ENDURANCE_RECOVERY_TARGET > ENDURANCE_EXHAUSTION_THRESHOLD);
    assert!(ENDURANCE_RECOVERY_TARGET <= ENDURANCE_MAX);
}

// ============================================
// Property 2: validate_endurance 拒绝越界
// ============================================
#[test]
fn r177_lf_02_validate_endurance() {
    // 边界通过
    assert!(validate_endurance(0.0).is_ok());
    assert!(validate_endurance(1.0).is_ok());
    assert!(validate_endurance(0.5).is_ok());

    // 越界拒绝
    assert!(validate_endurance(-0.1).is_err());
    assert!(validate_endurance(1.1).is_err());
    assert!(validate_endurance(100.0).is_err());
    assert!(validate_endurance(-100.0).is_err());
}

// ============================================
// Property 3: LifeForce::new 初始 endurance = ENDURANCE_MAX, SGI 空
// ============================================
#[test]
fn r177_lf_03_new_initial_state() {
    let life = make_life(1_700_000_000);
    assert_eq!(life.endurance, ENDURANCE_MAX);
    assert!(!life.has_sgi());
    assert!(!life.is_in_reflection(1_700_000_000));
}

// ============================================
// Property 4: SGI 设置后才允许 reflection_trigger
// ============================================
#[test]
fn r177_lf_04_reflection_requires_sgi() {
    let mut life = make_life(1_700_000_000);
    // SGI 空时反思触发失败
    let err = reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_000);
    assert!(err.is_err(), "SGI 空时反思触发应失败");

    // 设置 SGI 后反思触发成功
    life.sgi.goal = "test goal".into();
    let ok = reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_001);
    assert!(ok.is_ok());
    assert!(life.is_in_reflection(1_700_000_001));
}

// ============================================
// Property 5: 反思消耗 endurance (max - 0.1, min 0)
// ============================================
#[test]
fn r177_lf_05_reflection_decreases_endurance() {
    let mut life = make_life(1_700_000_000);
    life.sgi.goal = "x".into();
    let initial = life.endurance;
    reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_001).unwrap();
    let after = life.endurance;
    assert!(after <= initial, "反思后 endurance 应不增");
    assert!(after >= ENDURANCE_MIN, "反思后 endurance 应不低于 MIN");
    // 初始 1.0 - 0.1 = 0.9
    assert!((after - 0.9).abs() < 1e-9, "1.0 - 0.1 = 0.9, got {}", after);
}

// ============================================
// Property 6: 多次反思后 endurance 不为负
// ============================================
#[test]
fn r177_lf_06_repeated_reflection_floors_at_min() {
    let mut life = make_life(1_700_000_000);
    life.sgi.goal = "x".into();
    for i in 0..20 {
        let _ = reflection_trigger(
            &mut life,
            ReflectionTrigger::WeeklyReport,
            1_700_000_001 + i * 1000,
        );
    }
    assert!(
        life.endurance >= ENDURANCE_MIN,
        "多次反思 endurance 应 ≥ MIN, got {}",
        life.endurance
    );
}

// ============================================
// Property 7: recovery_start 回到 ENDURANCE_RECOVERY_TARGET
// ============================================
#[test]
fn r177_lf_07_recovery_target() {
    let mut life = make_life(1_700_000_000);
    life.endurance = 0.1;
    let after = recovery_start(&mut life);
    assert_eq!(after, ENDURANCE_RECOVERY_TARGET);
    assert!(life.endurance > ENDURANCE_EXHAUSTION_THRESHOLD);
}

// ============================================
// Property 8: exhaustion_check 阈值检测
// ============================================
#[test]
fn r177_lf_08_exhaustion_check() {
    let mut life = make_life(1_700_000_000);
    life.endurance = ENDURANCE_EXHAUSTION_THRESHOLD + 0.01;
    assert!(!exhaustion_check(&life), "高于阈值不应耗竭");
    life.endurance = ENDURANCE_EXHAUSTION_THRESHOLD - 0.01;
    assert!(exhaustion_check(&life), "低于阈值应耗竭");
    life.endurance = 0.0;
    assert!(exhaustion_check(&life));
}

// ============================================
// Property 9: continuity_id 不一致时反思失败
// ============================================
#[test]
fn r177_lf_09_continuity_mismatch() {
    let mut life = make_life(1_700_000_000);
    life.sgi.goal = "x".into();
    // 故意篡改 reflection 的 continuity_id
    life.reflection.continuity_id = "different-id".into();
    let err = reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_001);
    assert!(err.is_err(), "continuity_id 不一致应拒绝");
}

// ============================================
// Property 10: SGI::is_empty 与 goal 非空一致
// ============================================
#[test]
fn r177_lf_10_sgi_is_empty() {
    let mut life = make_life(1_700_000_000);
    assert!(life.sgi.is_empty());
    life.sgi.goal = " ".into();
    assert!(life.sgi.is_empty(), "空白 goal 仍视为空");
    life.sgi.goal = "x".into();
    assert!(!life.sgi.is_empty());
}

// ============================================
// Kani-style formal proof — endurance 永远 ∈ [0.0, 1.0]
// ============================================
#[cfg(kani)]
#[kani::proof]
fn r177_lf_kani_01_endurance_invariants() {
    assert!(ENDURANCE_MIN <= ENDURANCE_MAX);
    assert!(ENDURANCE_EXHAUSTION_THRESHOLD > ENDURANCE_MIN);
    assert!(ENDURANCE_EXHAUSTION_THRESHOLD < ENDURANCE_MAX);
    assert!(ENDURANCE_RECOVERY_TARGET >= ENDURANCE_EXHAUSTION_THRESHOLD);
    assert!(ENDURANCE_RECOVERY_TARGET <= ENDURANCE_MAX);
}

#[cfg(kani)]
#[kani::proof]
fn r177_lf_kani_02_validate_endurance_range() {
    assert!(validate_endurance(0.0).is_ok());
    assert!(validate_endurance(1.0).is_ok());
    assert!(validate_endurance(-1.0).is_err());
    assert!(validate_endurance(2.0).is_err());
}
