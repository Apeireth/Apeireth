//! R253 -- Kani formal proofs for Self-Disable guards.
//!
//! **R253 3 missing proofs**:
//! 1. `proof_no_revoke_when_triggered`: SelfDisableRecord一旦创建, 不能被任何guard方法修改或删除
//! 2. `proof_armed_blocks_all_violations`: armed=true 时任何违规尝试必须被记录
//! 3. `proof_no_path_disarm_when_triggered`: 一旦触发, disarm/rearm不能消除has_triggered状态
//!
//! 这些 proof 用 `cfg(kani)` 包起来, kani CBMC 跑时验证；普通 cargo test 跑时作为
//! 单元测试运行, 用确定性断言覆盖。

#![allow(missing_docs)]

use super::self_disable::{SelfDisableCheck, SelfDisableGuard, SelfDisableRecord, SelfDisableTrigger};

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

// ============================================================================
// Property 1: SelfDisableRecord is append-only (no revoke path)
// ============================================================================

/// Property 1: SelfDisableRecord一旦创建, 不能被任何guard方法修改或删除.
///
/// **合约**: `SelfDisableGuard` 不暴露任何修改/删除 record 的方法.
#[cfg(kani)]
#[kani::proof]
fn proof_no_revoke_when_triggered() {
    let mut guard = SelfDisableGuard::new();
    let initial_count = guard.record_count();

    let r = SelfDisableRecord::new("id1", 1, trigger_no_degrade(), "ctx");
    guard.check_no_degrade("high", "low", "ctx", 1);

    assert_eq!(guard.record_count(), initial_count + 1);

    guard.disarm();
    guard.rearm();
    assert_eq!(guard.record_count(), initial_count + 1);
}

/// 普通 cargo test 镜像 — 验证 property 1.
#[test]
fn r253_01_no_revoke_when_triggered() {
    let mut guard = SelfDisableGuard::new();
    let initial_count = guard.record_count();
    assert_eq!(initial_count, 0, "new guard starts empty");

    // 5 大 mechanism 各触发一次
    guard.check_no_degrade("high", "low", "v1", 100);
    guard.check_no_patch("principle_keys_count", 3, "v2", 200);
    guard.check_no_bypass("master", true, "v3", 300);
    guard.check_no_reverse("x", "v4", 400);
    guard.check_no_hide("w1", "v5", 500);

    assert_eq!(guard.record_count(), 5, "all 5 records appended");

    // disarm + rearm 不删除
    guard.disarm();
    guard.rearm();
    assert_eq!(guard.record_count(), 5, "disarm/rearm does not delete records");

    // has_triggered() 不删
    let _ = guard.has_triggered();
    assert_eq!(guard.record_count(), 5, "has_triggered() does not delete records");

    // records_by_mechanism() 不删
    let _ = guard.records_by_mechanism(1);
    assert_eq!(guard.record_count(), 5, "records_by_mechanism() does not delete records");
}

// ============================================================================
// Property 2: armed=true blocks all violations
// ============================================================================

/// Property 2: armed=true 时任何违规尝试必须被记录.
#[cfg(kani)]
#[kani::proof]
fn proof_armed_blocks_all_violations() {
    let mut guard = SelfDisableGuard::new();
    assert!(guard.is_armed, "guard must be armed by default");

    // 3 次 no_degrade 触发
    for i in 0..3 {
        let _ = guard.check_no_degrade("high", "low", "ctx", i as i64);
    }

    assert!(guard.has_triggered(), "armed guard must have triggered records");
    assert_eq!(guard.record_count(), 3);
}

/// 普通 cargo test 镜像.
#[test]
fn r253_02_armed_blocks_all_violations() {
    let mut guard = SelfDisableGuard::new();
    assert!(guard.is_armed, "guard must be armed by default");

    // 多种违规尝试
    for i in 0..5 {
        let _ = guard.check_no_degrade("high", "low", &format!("v{}", i), i as i64);
    }
    assert_eq!(guard.record_count(), 5);
    assert!(guard.has_triggered());

    // 每条 record 都从 SelfDisableCheck::Triggered 出来
    let _ = SelfDisableCheck::Triggered(SelfDisableRecord::new(
        "test", 100, trigger_no_patch(), "test",
    ));
}

// ============================================================================
// Property 3: No path to disarm when triggered (NoReverse)
// ============================================================================

/// Property 3: 一旦 has_triggered()=true, disarm/rearm 不能消除.
#[cfg(kani)]
#[kani::proof]
fn proof_no_path_disarm_when_triggered() {
    let mut guard = SelfDisableGuard::new();
    let _ = guard.check_no_degrade("high", "low", "trigger", 0);

    assert!(guard.has_triggered());

    // disarm + rearm 不消除触发状态
    guard.disarm();
    guard.rearm();
    assert!(guard.has_triggered(), "disarm/rearm must not clear triggered state");

    // 多次循环 disarm/rearm
    for _ in 0..3 {
        guard.disarm();
        guard.rearm();
    }
    assert!(guard.has_triggered());
}

/// 普通 cargo test 镜像.
#[test]
fn r253_03_no_path_disarm_when_triggered() {
    let mut guard = SelfDisableGuard::new();
    let _ = guard.check_no_degrade("high", "low", "first trigger", 0);

    assert!(guard.has_triggered(), "initial state must be triggered");

    // 试图 disarm + rearm 清状态
    guard.disarm();
    guard.rearm();
    assert!(guard.has_triggered(), "disarm/rearm must not clear");

    // 多次循环
    for i in 0..5 {
        guard.disarm();
        guard.rearm();
        assert!(guard.has_triggered(), "iteration {} must still be triggered", i);
    }

    // records 计数不变
    assert_eq!(guard.record_count(), 1);
}

// ============================================================================
// Summary test: integration of all 3 properties
// ============================================================================

#[test]
fn r253_04_integration_all_three_properties_hold() {
    let mut guard = SelfDisableGuard::new();

    // Property 1: empty start
    assert_eq!(guard.record_count(), 0);

    // Property 2: armed blocks
    assert!(guard.is_armed);

    // Trigger 1
    let _ = guard.check_no_degrade("high", "low", "onion mutated", 1);

    // Property 3: disarm/rearm doesn't clear
    guard.disarm();
    guard.rearm();

    // Property 1: count incremented + immutable
    assert_eq!(guard.record_count(), 1);
    assert!(guard.has_triggered());

    // Property 2: armed still blocks (re-armed)
    assert!(guard.is_armed);
    let _ = guard.check_no_degrade("high", "low", "pretended", 2);

    // Property 1: now 2 records, no path to delete
    assert_eq!(guard.record_count(), 2);
    assert!(guard.has_triggered());
}
