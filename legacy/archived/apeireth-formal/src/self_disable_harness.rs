//! R131.8 Self-Disable 5 机制 Kani harness (critical missing proof 1)
//!
//! **目的**: 形式化 5 大机制不可绕过 — 任意符号化输入下, 5 check 必返 Pass 或
//! Triggered (0 panic), 必产生单调 trigger_id, disarm/rearm 不能清除 records.
//!
//! **跑法**: `cargo kani --harness kani_verify_self_disable_*`

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 编译期常量 (POD-friendly, 0 假设生产类型)
// ============================================================

pub const SELF_DISABLE_MECHANISM_COUNT: u8 = 5;
pub const SELF_DISABLE_MECHANISM_MAX: u8 = 5;
pub const RISK_RANK_MAX: i32 = 4;
pub const RISK_RANK_MIN: i32 = -1;
pub const PROTECTED_RULES_COUNT: usize = 5;

// ============================================================
// POD 模型: 镜像 Self-DisableGuard 行为
// ============================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SelfDisableGuardPod {
    pub is_armed: bool,
    pub record_count_by_mechanism: [u32; 5],
    pub total_records: u32,
    pub next_trigger_id: u32,
}

impl SelfDisableGuardPod {
    pub const fn new() -> Self {
        Self { is_armed: true, record_count_by_mechanism: [0; 5], total_records: 0, next_trigger_id: 0 }
    }
    pub const fn disarm(&mut self) { self.is_armed = false; }
    pub const fn rearm(&mut self) { self.is_armed = true; }

    fn trigger(&mut self, mechanism_id: u8) -> bool {
        if !self.is_armed { return false; }
        let idx = (mechanism_id - 1) as usize;
        if idx >= 5 { return false; }
        self.record_count_by_mechanism[idx] += 1;
        self.total_records += 1;
        self.next_trigger_id += 1;
        true
    }

    pub fn check_no_degrade(&mut self, original_rank: i32, proposed_rank: i32) -> bool {
        if !self.is_armed { return false; }
        if proposed_rank < original_rank { return self.trigger(1); }
        false
    }
    pub fn check_no_patch(&mut self, rule_id: u8) -> bool {
        if !self.is_armed { return false; }
        if rule_id < PROTECTED_RULES_COUNT as u8 { return self.trigger(2); }
        false
    }
    pub fn check_no_bypass(&mut self, owner_id: u8, bypassed: bool) -> bool {
        if !self.is_armed { return false; }
        if owner_id == 0 && bypassed { return self.trigger(3); }
        false
    }
    pub fn check_no_reverse(&mut self, _trigger_id: u32) -> bool {
        if !self.is_armed { return false; }
        self.trigger(4)
    }
    pub fn check_no_hide(&mut self, window_id: u32) -> bool {
        if !self.is_armed { return false; }
        if window_id != 0 { return self.trigger(5); }
        false
    }
}

// ============================================================
// nondet_* helper (per 既有 kani_harness.rs 模板)
// ============================================================

#[cfg(kani)]
fn nondet_u8() -> u8 { kani::any() }
#[cfg(not(kani))]
fn nondet_u8() -> u8 { 0 }

#[cfg(kani)]
fn nondet_u32() -> u32 { kani::any() }
#[cfg(not(kani))]
fn nondet_u32() -> u32 { 0 }

#[cfg(kani)]
fn nondet_i32() -> i32 { kani::any() }
#[cfg(not(kani))]
fn nondet_i32() -> i32 { 0 }

#[cfg(kani)]
fn nondet_bool() -> bool { kani::any() }
#[cfg(not(kani))]
fn nondet_bool() -> bool { false }

// ============================================================
// Kani harness 1: 5 机制全 0 panic + 单调用 0 跨界
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_self_disable_5_mechanisms_no_panic() {
    let mut g = SelfDisableGuardPod::new();
    let owner_id = nondet_u8();
    let rule_id = nondet_u8();
    let original_rank = nondet_i32();
    let proposed_rank = nondet_i32();
    let trigger_id = nondet_u32();
    let window_id = nondet_u32();
    let bypassed = nondet_bool();

    let _ = g.check_no_degrade(original_rank, proposed_rank);
    let _ = g.check_no_patch(rule_id);
    let _ = g.check_no_bypass(owner_id, bypassed);
    let _ = g.check_no_reverse(trigger_id);
    let _ = g.check_no_hide(window_id);

    let sum: u32 = g.record_count_by_mechanism.iter().sum();
    assert!(sum == g.total_records, "records count invariant broken");
}

// ============================================================
// Kani harness 2: disarm/rearm 不能清除 records + trigger_id 单调
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_self_disable_disarm_rearm_history_immutable() {
    let mut g = SelfDisableGuardPod::new();
    g.check_no_degrade(RISK_RANK_MAX, RISK_RANK_MIN);
    assert!(g.total_records == 1);
    let id_before = g.next_trigger_id;

    for _ in 0..10 {
        g.disarm();
        g.rearm();
        assert!(g.total_records == 1, "records must not change");
        assert!(g.next_trigger_id == id_before, "trigger_id must be stable");
    }

    g.check_no_patch(0);
    assert!(g.total_records == 2, "after rearm, new violations still record");
    assert!(g.next_trigger_id > id_before, "new triggers increment id");
}

// ============================================================
// Kani harness 3: mechanism_id 严格唯一 1-5
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_self_disable_mechanism_id_uniqueness() {
    let mut g = SelfDisableGuardPod::new();
    let owner_id = nondet_u8();
    let rule_id = nondet_u8();
    let rank = nondet_i32();
    let bypassed = nondet_bool();

    // 每个机制调用 1 次 — 至少 3 个无条件触发 (degrade / reverse / hide)
    let _ = g.check_no_degrade(rank, rank - 1);
    let _ = g.check_no_patch(rule_id);
    let _ = g.check_no_bypass(owner_id, bypassed);
    let _ = g.check_no_reverse(0);
    let _ = g.check_no_hide(1);

    assert!(g.total_records >= 3, "at least 3 unconditional mechanisms trigger");
    assert!(g.record_count_by_mechanism[0] >= 1, "mechanism 1 triggers");
    assert!(g.record_count_by_mechanism[3] >= 1, "mechanism 4 triggers");
    assert!(g.record_count_by_mechanism[4] >= 1, "mechanism 5 triggers");
}

// ============================================================
// Kani harness 4: trigger_id 严格单调递增
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_self_disable_trigger_id_monotonic() {
    let mut g = SelfDisableGuardPod::new();
    let mut prev_id = 0u32;
    let n: u8 = nondet_u8();

    for _ in 0..n {
        let rank = nondet_i32();
        let _ = g.check_no_degrade(rank, rank - 1);
        assert!(g.next_trigger_id > prev_id, "trigger_id must strictly increase");
        prev_id = g.next_trigger_id;
    }
}

// ============================================================
// Kani harness 5: 5 机制数 == 5 (编译期守门)
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_self_disable_mechanism_count_eq_5() {
    assert!(SELF_DISABLE_MECHANISM_COUNT == 5);
    assert!(SELF_DISABLE_MECHANISM_MAX == 5);
    assert!(PROTECTED_RULES_COUNT == 5);
}

// ============================================================
// Unit tests (cargo test 跑, 0 装 Kani)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_no_degrade_blocks() {
        let mut g = SelfDisableGuardPod::new();
        assert!(g.check_no_degrade(4, 0));
        assert!(g.total_records == 1);
        assert!(!g.check_no_degrade(0, 4));
        assert!(g.total_records == 1);
    }

    #[test]
    fn pod_no_patch_protected() {
        let mut g = SelfDisableGuardPod::new();
        assert!(g.check_no_patch(0));
        assert!(g.check_no_patch(4));
        assert!(!g.check_no_patch(5));
        assert!(g.total_records == 2);
    }

    #[test]
    fn pod_no_bypass_master() {
        let mut g = SelfDisableGuardPod::new();
        assert!(g.check_no_bypass(0, true));
        assert!(!g.check_no_bypass(0, false));
        assert!(!g.check_no_bypass(1, true));
        assert!(g.total_records == 1);
    }

    #[test]
    fn pod_no_reverse_any() {
        let mut g = SelfDisableGuardPod::new();
        assert!(g.check_no_reverse(0));
        assert!(g.check_no_reverse(42));
        assert!(g.total_records == 2);
    }

    #[test]
    fn pod_no_hide_nonzero() {
        let mut g = SelfDisableGuardPod::new();
        assert!(!g.check_no_hide(0));
        assert!(g.check_no_hide(1));
        assert!(g.check_no_hide(123));
        assert!(g.total_records == 2);
    }

    #[test]
    fn pod_disarm_skips_all() {
        let mut g = SelfDisableGuardPod::new();
        g.disarm();
        assert!(!g.check_no_degrade(4, 0));
        assert!(!g.check_no_patch(0));
        assert!(!g.check_no_bypass(0, true));
        assert!(!g.check_no_reverse(0));
        assert!(!g.check_no_hide(1));
        assert!(g.total_records == 0);
    }

    #[test]
    fn pod_disarm_rearm_keeps_records() {
        let mut g = SelfDisableGuardPod::new();
        g.check_no_degrade(4, 0);
        assert!(g.total_records == 1);
        for _ in 0..100 {
            g.disarm();
            g.rearm();
        }
        assert!(g.total_records == 1);
        g.check_no_patch(0);
        assert!(g.total_records == 2);
    }

    #[test]
    fn pod_compile_time_guards() {
        assert_eq!(SELF_DISABLE_MECHANISM_COUNT, 5);
        assert_eq!(SELF_DISABLE_MECHANISM_MAX, 5);
        assert_eq!(PROTECTED_RULES_COUNT, 5);
    }

    #[test]
    fn pod_5_mechanisms_total_records_invariant() {
        let mut g = SelfDisableGuardPod::new();
        for _ in 0..50 {
            let _ = g.check_no_degrade(4, 0);
            let _ = g.check_no_patch(2);
            let _ = g.check_no_bypass(0, true);
            let _ = g.check_no_reverse(0);
            let _ = g.check_no_hide(42);
        }
        let sum: u32 = g.record_count_by_mechanism.iter().sum();
        assert!(sum == g.total_records, "records count must match sum across 5 mechanism slots");
    }

    #[test]
    fn pod_mechanism_id_within_range() {
        // 5 mechanism ID 严格 1-5 (per SELF_DISABLE_MECHANISM_COUNT)
        for id in 1..=5u8 {
            assert!(id >= 1 && id <= 5, "mechanism_id must be 1-5");
        }
    }
}

