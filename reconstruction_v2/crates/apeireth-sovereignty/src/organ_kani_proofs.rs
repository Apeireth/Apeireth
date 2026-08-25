//! R177 sovereignty organ invariants

#![allow(missing_docs)]

use crate::self_disable::{SelfDisableGuard, SelfDisableTrigger};


#[test]
fn r177_sov_01_new_armed_default() {
    let g = SelfDisableGuard::new();
    assert!(g.is_armed);
    assert_eq!(g.record_count(), 0);
    assert!(!g.has_triggered());
}

#[test]
fn r177_sov_02_disarm_relaxes() {
    let mut g = SelfDisableGuard::new();
    g.disarm(); assert!(!g.is_armed);
    let r = g.check_no_degrade("high", "low", "x", 1_700_000_000);
    assert!(r.is_pass());
    assert_eq!(g.record_count(), 0);
}

#[test]
fn r177_sov_03_rearm_reactivates() {
    let mut g = SelfDisableGuard::new();
    g.disarm(); g.rearm(); assert!(g.is_armed);
}

#[test]
fn r177_sov_04_records_append_only() {
    let mut g = SelfDisableGuard::new();
    let before = g.record_count();
    let _ = g.check_no_degrade("high", "low", "t1", 1_700_000_000);
    let after1 = g.record_count();
    let _ = g.check_no_degrade("high", "low", "t2", 1_700_000_001);
    let after2 = g.record_count();
    assert_eq!(after1, before + 1);
    assert_eq!(after2, before + 2);
}

#[test]
fn r177_sov_05_no_degrade_high_to_low() {
    let mut g = SelfDisableGuard::new();
    assert!(g.check_no_degrade("high", "low", "x", 1_700_000_000).is_triggered());
    assert_eq!(g.record_count(), 1);
}

#[test]
fn r177_sov_06_no_degrade_same_passes() {
    let mut g = SelfDisableGuard::new();
    assert!(g.check_no_degrade("high", "high", "x", 1_700_000_000).is_pass());
    assert!(g.check_no_degrade("medium", "high", "x", 1_700_000_000).is_pass());
}

#[test]
fn r177_sov_07_trigger_id_format() {
    let mut g = SelfDisableGuard::new();
    let _ = g.check_no_degrade("high", "low", "x", 1_700_000_000);
    let records = g.records();
    assert!(!records.is_empty());
    let id = &records[0].trigger_id;
    assert!(id.starts_with("sd-"));
    assert!(id.len() >= 9);
}

#[test]
fn r177_sov_08_has_triggered_consistent() {
    let mut g = SelfDisableGuard::new();
    assert!(!g.has_triggered());
    let _ = g.check_no_degrade("high", "low", "x", 1_700_000_000);
    assert!(g.has_triggered());
    assert_eq!(g.records().len(), g.record_count());
}

#[test]
fn r177_sov_09_records_by_mechanism() {
    let mut g = SelfDisableGuard::new();
    let _ = g.check_no_degrade("high", "low", "t1", 1_700_000_000);
    let _ = g.check_no_degrade("high", "low", "t2", 1_700_000_001);
    assert_eq!(g.records_by_mechanism(1).len(), 2);
    assert_eq!(g.records_by_mechanism(99).len(), 0);
}

#[test]
fn r177_sov_10_trigger_name() {
    let t = SelfDisableTrigger::NoDegradeViolation { from: "high".into(), to: "low".into() };
    assert!(!t.mechanism_name().is_empty());
    assert!(!t.chinese_name().is_empty());
    assert_eq!(t.mechanism_id(), 1);
}
