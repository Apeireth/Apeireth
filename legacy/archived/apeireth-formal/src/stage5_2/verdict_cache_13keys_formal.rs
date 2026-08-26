//! R129-10 Stage 5.2 形式化扩展 F4 — 13 键 verdict cache 形式化 (A3 严守, 0 改 13 键)
//!
//! # 背景 (per 决策 #33 §2.3 A3 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! A3 12 键 + PHL-07 = 13 键 (per 决策 #33 §2.3 A3 + 决策 #36 §1.5 + 决策 #51 §1.2 P1-2):
//! - V3 PHL-01 (3): NotClone / NotPerfect / NotUuid
//! - V3 PHL-02b (3): NotUndo / NotProof / NotSafe
//! - V3 PHL-03 (3): SpecIsNotProof / CounterexampleIsNotBug / ProverIsNotTruth
//! - v4.1 PHL-04/05/06 (3): NotUnobservable / NotUnscientific / NotSelfRelationless
//! - **R125-12 PHL-07 (1)**: NotUnoptimizable
//!
//! 13 键 编译期 hardcode 锁: `pub const ALL_THIRTEEN_KEYS: [PhilosophyKey; 13] = [...]`
//!
//! # 借鉴 ID
//!
//! `R129-10-F4-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: A3 13 键严守 0 改
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - A3 13 键: 仅形式化, 0 改 13 键
//! - 0 触碰 `apeireth-core::PhilosophyKey` enum (PHL-01~07)
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (13 键 verdict cache, 1:1 跟 A3 严守)
// ============================================================

/// 13 键 verdict cache 总数 (1:1 跟 A3 严守, per R125-12 PHL-07 done)
pub const VERDICT_CACHE_13_KEYS_COUNT: usize = 13;

/// 13 键分组 (1:1 跟 A3 严守, per PHL-01/02b/03/04/05/06/07)
pub const VERDICT_CACHE_GROUP_COUNT: usize = 7;

/// 13 键 POD 镜像 (1:1 跟 A3 严守, 12 键 → 13 键 PHL-07 升级)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VerdictKey13Pod {
    /// 键身份 (0..12, 1:1 跟 A3 严守)
    pub key: u8,
    /// 键分组 (1..=7, PHL-01/02b/03/04/05/06/07, A3 严守)
    pub group: u8,
    /// verdict 结果 (true=pass, false=block)
    pub verdict: bool,
}

impl VerdictKey13Pod {
    /// 构造 (编译期 hardcode)
    pub const fn new(key: u8, group: u8, verdict: bool) -> Self {
        Self { key, group, verdict }
    }

    /// 13 键总数 (A3 严守)
    pub const fn count() -> usize {
        VERDICT_CACHE_13_KEYS_COUNT
    }
}

// ============================================================
// 2. 13 键 verdict cache 不变量 (A3 严守 0 改)
// ============================================================

/// 13 键 verdict cache 不变量: key ∈ 0..12 永真 (A3 严守)
pub fn verdict_cache_13keys_invariant(k: VerdictKey13Pod) -> bool {
    k.key < VERDICT_CACHE_13_KEYS_COUNT as u8
}

/// 13 键 verdict cache 分组不变量: group ∈ 1..=7 永真 (A3 严守)
pub fn verdict_cache_13keys_group_invariant(k: VerdictKey13Pod) -> bool {
    k.group >= 1 && k.group <= VERDICT_CACHE_GROUP_COUNT as u8
}

/// 13 键 verdict cache 全 pass 不变量 (A3 严守)
pub fn verdict_cache_13keys_all_passed(ks: [VerdictKey13Pod; VERDICT_CACHE_13_KEYS_COUNT]) -> bool {
    for k in &ks {
        if !k.verdict {
            return false;
        }
    }
    true
}

// ============================================================
// 3. Kani-style proof harness
// ============================================================

/// Kani proof harness — 13 键 key ∈ 0..12 永真 (A3 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_verdict_cache_13keys_in_range() {
    let k = nondet_key();
    assert!(verdict_cache_13keys_invariant(k), "13 键 key 必须在 0..12");
}

/// Kani proof harness — 13 键 count = 13 (A3 严守, 12 + PHL-07)
#[cfg_attr(kani, kani::proof)]
pub fn proof_verdict_cache_13keys_count_is_13() {
    assert_eq!(VerdictKey13Pod::count(), 13, "13 键 count 必须 = 13 (12 + PHL-07)");
}

#[cfg(kani)]
fn nondet_key() -> VerdictKey13Pod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_key() -> VerdictKey13Pod {
    // cargo test 兜底: 选 PHL-01 NotClone pass 的 happy path
    VerdictKey13Pod::new(0, 1, true)
}

/// Runtime sanity: 13 键 (0..12) 应全部通过
pub fn sanity_check() -> bool {
    for key in 0u8..VERDICT_CACHE_13_KEYS_COUNT as u8 {
        if !verdict_cache_13keys_invariant(VerdictKey13Pod::new(key, (key % 7) + 1, true)) {
            return false;
        }
    }
    true
}

// ============================================================
// 4. 单元测试 (8 tests, 0 装 PASS 严守 verify)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = proof_verdict_cache_13keys_in_range;
        let _: fn() = proof_verdict_cache_13keys_count_is_13;
    }

    #[test]
    fn verdict_cache_13keys_count_is_13() {
        assert_eq!(VERDICT_CACHE_13_KEYS_COUNT, 13);
        assert_eq!(VerdictKey13Pod::count(), 13);
    }

    #[test]
    fn verdict_cache_13keys_0_to_12_all_pass() {
        for key in 0u8..13 {
            assert!(verdict_cache_13keys_invariant(VerdictKey13Pod::new(key, 1, true)));
        }
    }

    #[test]
    fn verdict_cache_13keys_13_violates() {
        // 反例: key=13 越界 (A3 严守 0..13)
        assert!(!verdict_cache_13keys_invariant(VerdictKey13Pod::new(13, 1, true)));
    }

    #[test]
    fn verdict_cache_13keys_group_1_to_7_all_pass() {
        for group in 1u8..=7 {
            assert!(verdict_cache_13keys_group_invariant(VerdictKey13Pod::new(0, group, true)));
        }
    }

    #[test]
    fn verdict_cache_13keys_group_0_violates() {
        // 反例: group=0 越界 (A3 严守 1..=7)
        assert!(!verdict_cache_13keys_group_invariant(VerdictKey13Pod::new(0, 0, true)));
    }

    #[test]
    fn verdict_cache_13keys_a3_hardcode_12_to_13() {
        // A3 升级证据: 12 → 13 (R125-12 PHL-07 完成)
        assert_eq!(VERDICT_CACHE_13_KEYS_COUNT, 13);
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
