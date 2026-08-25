//! Kani-style proofs for Self-Disable guards (cargo test mirror)

#![allow(missing_docs)]

use crate::self_disable::{
    SelfDisableCheck, SelfDisableGuard, SelfDisableRecord, SelfDisableTrigger,
};

fn trigger_no_degrade() -> SelfDisableTrigger {
    SelfDisableTrigger::NoDegradeViolation { from: "high".to_string(), to: "low".to_string() }
}
fn trigger_no_patch() -> SelfDisableTrigger {
    SelfDisableTrigger::NoPatchViolation { rule: "principle_keys_count".to_string() }
}
fn trigger_no_bypass() -> SelfDisableTrigger {
    SelfDisableTrigger::NoBypassViolation { token: "MasterToken".to_string() }
}
fn trigger_no_reverse() -> SelfDisableTrigger {
    SelfDisableTrigger::NoReverseViolation { trigger_id: "x".to_string() }
}
fn trigger_no_hide() -> SelfDisableTrigger {
    SelfDisableTrigger::NoHideViolation { window_id: "w1".to_string() }
}

#[test]
fn r253_01_no_revoke_when_triggered() {
    let mut guard = SelfDisableGuard::new();
    assert_eq!(guard.record_count(), 0);
    guard.check_no_degrade("high", "low", "v1", 100);
    guard.check_no_patch("principle_keys_count", 3, "v2", 200);
    guard.check_no_bypass("master", true, "v3", 300);
    guard.check_no_reverse("x", "v4", 400);
    guard.check_no_hide("w1", "v5", 500);
    assert_eq!(guard.record_count(), 5);
    guard.disarm(); guard.rearm();
    assert_eq!(guard.record_count(), 5);
    let _ = guard.has_triggered();
    assert_eq!(guard.record_count(), 5);
    let _ = guard.records_by_mechanism(1);
    assert_eq!(guard.record_count(), 5);
}

#[test]
fn r253_02_armed_blocks_all_violations() {
    let mut guard = SelfDisableGuard::new();
    assert!(guard.is_armed);
    for i in 0..5 {
        let _ = guard.check_no_degrade("high", "low", &format!("v{}", i), i64::from(i));
    }
    assert_eq!(guard.record_count(), 5);
    assert!(guard.has_triggered());
    let _ = SelfDisableCheck::Triggered(SelfDisableRecord::new("test", 100, trigger_no_patch(), "test"));
}

#[test]
fn r253_03_no_path_disarm_when_triggered() {
    let mut guard = SelfDisableGuard::new();
    let _ = guard.check_no_degrade("high", "low", "first", 0);
    assert!(guard.has_triggered());
    guard.disarm(); guard.rearm();
    assert!(guard.has_triggered());
    for _ in 0..5 {
        guard.disarm(); guard.rearm();
        assert!(guard.has_triggered());
    }
    assert_eq!(guard.record_count(), 1);
}

#[test]
fn r253_04_integration_all_three() {
    let mut guard = SelfDisableGuard::new();
    assert_eq!(guard.record_count(), 0);
    assert!(guard.is_armed);
    let _ = guard.check_no_degrade("high", "low", "x", 1);
    guard.disarm(); guard.rearm();
    assert_eq!(guard.record_count(), 1);
    assert!(guard.has_triggered());
    assert!(guard.is_armed);
    let _ = guard.check_no_degrade("high", "low", "x", 2);
    assert_eq!(guard.record_count(), 2);
    assert!(guard.has_triggered());
}

#[test]
fn r268_01_disarmed_blocks_all() {
    let mut guard = SelfDisableGuard::new();
    guard.disarm();
    let mut triggered = false;
    for i in 0..10 {
        if let SelfDisableCheck::Triggered(_) = guard.check_no_degrade("high", "low", "ctx", i) {
            triggered = true; break;
        }
    }
    assert!(!triggered);
    assert_eq!(guard.record_count(), 0);
}

#[test]
fn r268_02_rearm_restores() {
    let mut guard = SelfDisableGuard::new();
    guard.disarm(); assert!(!guard.is_armed);
    guard.rearm(); assert!(guard.is_armed);
    let r = guard.check_no_degrade("high", "low", "ctx", 0);
    assert!(matches!(r, SelfDisableCheck::Triggered(_)));
}

#[test]
fn r268_03_pass_path_no_record() {
    let mut guard = SelfDisableGuard::new();
    let before = guard.record_count();
    let r1 = guard.check_no_degrade("high", "high", "same", 0);
    let r2 = guard.check_no_degrade("low", "high", "up", 1);
    let r3 = guard.check_no_degrade("medium", "medium", "same", 2);
    assert!(matches!(r1, SelfDisableCheck::Pass));
    assert!(matches!(r2, SelfDisableCheck::Pass));
    assert!(matches!(r3, SelfDisableCheck::Pass));
    assert_eq!(guard.record_count(), before);
    assert!(!guard.has_triggered());
}

#[test]
fn r268_04_trigger_id_uniqueness() {
    let mut guard = SelfDisableGuard::new();
    let mut ids = std::collections::HashSet::new();
    for i in 0..20 {
        if let SelfDisableCheck::Triggered(rec) = guard.check_no_degrade("high", "low", &format!("c{}", i), i) {
            assert!(!ids.contains(&rec.trigger_id));
            ids.insert(rec.trigger_id);
        }
    }
    assert_eq!(ids.len(), 20);
}

#[test]
fn r268_05_mechanism_ids_stable() {
    assert_eq!(trigger_no_degrade().mechanism_id(), 1);
    assert_eq!(trigger_no_patch().mechanism_id(), 2);
    assert_eq!(trigger_no_bypass().mechanism_id(), 3);
    assert_eq!(trigger_no_reverse().mechanism_id(), 4);
    assert_eq!(trigger_no_hide().mechanism_id(), 5);
}
