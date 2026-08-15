//! R177 sovereignty organ Kani proofs (W5+)
//!
//! **要验证的不变量 (VCP 模式 8/8 + 找 bug)**:
//! 1. SelfDisableGuard::new 默认 armed=true
//! 2. disarm 后 check 应 Pass (机制外)
//! 3. rearm 后 armed=true
//! 4. records 单调增 (append-only)
//! 5. NoDegrade 机制 — 高 → 低风险触发
//! 6. NoDegrade 机制 — 同级或升高 Pass
//! 7. next_id 单调递增
//! 8. has_triggered 与 records 一致
//! 9. records_by_mechanism 过滤正确
//! 10. trigger_id 格式 sd-NNNNNN

#![allow(missing_docs)]

use crate::self_disable::{SelfDisableGuard, SelfDisableTrigger};

#[test]
fn r177_sov_01_new_armed_by_default() {
    let g = SelfDisableGuard::new();
    assert!(g.is_armed, "默认 armed=true");
    assert_eq!(g.record_count(), 0);
    assert!(!g.has_triggered());
}

#[test]
fn r177_sov_02_disarm_relaxes_checks() {
    let mut g = SelfDisableGuard::new();
    g.disarm();
    assert!(!g.is_armed);
    let r = g.check_no_degrade("high", "low", "test", 1_700_000_000);
    assert!(r.is_pass(), "disarm 后应 Pass (机制外)");
    assert_eq!(g.record_count(), 0);
}

#[test]
fn r177_sov_03_rearm_reactivates() {
    let mut g = SelfDisableGuard::new();
    g.disarm();
    g.rearm();
    assert!(g.is_armed);
}

#[test]
fn r177_sov_04_records_append_only() {
    let mut g = SelfDisableGuard::new();
    let before = g.record_count();
    let _ = g.check_no_degrade("high", "low", "test1", 1_700_000_000);
    let after1 = g.record_count();
    let _ = g.check_no_degrade("high", "low", "test2", 1_700_000_001);
    let after2 = g.record_count();
    assert_eq!(after1, before + 1);
    assert_eq!(after2, before + 2);
}

#[test]
fn r177_sov_05_no_degrade_high_to_low_triggers() {
    let mut g = SelfDisableGuard::new();
    let r = g.check_no_degrade("high", "low", "test", 1_700_000_000);
    assert!(r.is_triggered(), "high → low 应触发");
    assert_eq!(g.record_count(), 1);
}

#[test]
fn r177_sov_06_no_degrade_same_level_passes() {
    let mut g = SelfDisableGuard::new();
    let r = g.check_no_degrade("high", "high", "test", 1_700_000_000);
    assert!(r.is_pass(), "high → high 应 Pass");
    let r2 = g.check_no_degrade("medium", "high", "test", 1_700_000_000);
    assert!(r2.is_pass(), "medium → high 应 Pass (升级)");
}

#[test]
fn r177_sov_07_trigger_id_format() {
    let mut g = SelfDisableGuard::new();
    let _ = g.check_no_degrade("high", "low", "test", 1_700_000_000);
    let records = g.records();
    assert!(!records.is_empty());
    let id = &records[0].trigger_id;
    assert!(id.starts_with("sd-"), "trigger_id 应以 sd- 开头, got {}", id);
    assert!(id.len() >= 9, "trigger_id 长度应 ≥ 9 (sd-NNNNNN), got {}", id);
}

#[test]
fn r177_sov_08_has_triggered_consistent_with_records() {
    let mut g = SelfDisableGuard::new();
    assert!(!g.has_triggered());
    let _ = g.check_no_degrade("high", "low", "test", 1_700_000_000);
    assert!(g.has_triggered());
    assert_eq!(g.records().len(), g.record_count());
}

#[test]
fn r177_sov_09_records_by_mechanism() {
    let mut g = SelfDisableGuard::new();
    let _ = g.check_no_degrade("high", "low", "t1", 1_700_000_000);
    let _ = g.check_no_degrade("high", "low", "t2", 1_700_000_001);
    let m_records = g.records_by_mechanism(1);
    assert_eq!(m_records.len(), 2, "NoDegrade = mechanism 1");
    let m_other = g.records_by_mechanism(99);
    assert_eq!(m_other.len(), 0);
}

#[test]
fn r177_sov_10_trigger_name_non_empty() {
    let t = SelfDisableTrigger::NoDegradeViolation {
        from: "high".into(),
        to: "low".into(),
    };
    assert!(!t.mechanism_name().is_empty());
    assert!(!t.chinese_name().is_empty());
    assert_eq!(t.mechanism_id(), 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_sov_kani_01_armed_invariants() {
    let g = SelfDisableGuard::new();
    assert!(g.is_armed);
    assert_eq!(g.record_count(), 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_sov_kani_02_no_degrade_same_or_up() {
    let mut g = SelfDisableGuard::new();
    let r = g.check_no_degrade("high", "high", "k", 1_700_000_000);
    assert!(r.is_pass());
    let r2 = g.check_no_degrade("low", "high", "k", 1_700_000_000);
    assert!(r2.is_pass());
}
