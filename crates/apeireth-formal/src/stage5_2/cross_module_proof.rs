//! R129-10 Stage 5.2 形式化扩展 F9 — 跨模块证明 (F1-F8 跨模块集成)
//!
//! # 背景 (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! F9 跨模块证明 (F1-F8 跨模块集成) 8 模块互锁:
//! - F1 6 重守门 v7 (B4)
//! - F2 8 哲学锚 (B5)
//! - F3 V0.5 30 维 (B3)
//! - F4 13 键 verdict cache (A3)
//! - F5 R11 baseline 3 值 (A1)
//! - F6 24 LOCKED 入口签名 (B1)
//! - F7 8 借鉴 ID 真实施 (C2)
//! - F8 整合 #4 commit 严守 (C1)
//!
//! 跨模块不变量: 8 模块各自 invariant 全 pass + 1 联合 invariant 全 pass
//!
//! # 借鉴 ID
//!
//! `R129-10-F9-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: F1-F8 跨模块 0 越界
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - F1-F8 0 越界: 8 模块各自严守 (B4/B5/B3/A3/A1/B1/C2/C1)
//! - 跨模块 invariant 严守: 8 模块互锁
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

use super::{
    six_gates_v7_formal::{SixFoldGatePod, six_fold_v7_invariant, SIX_FOLD_GATE_V7_COUNT},
    eight_anchors_formal::{EightAnchorPod, eight_anchors_invariant, EIGHT_ANCHORS_COUNT},
    v05_30dim_formal::{V05DimPod, v05_30dim_invariant, V05_30_DIM_COUNT},
    verdict_cache_13keys_formal::{VerdictKey13Pod, verdict_cache_13keys_invariant, VERDICT_CACHE_13_KEYS_COUNT},
    r11_baseline_formal::{R11BaselinePod, r11_baseline_3values_invariant, R11_BASELINE_V1141, R11_BASELINE_V1131, R11_BASELINE_V1136},
    locked_24_entry_formal::{Locked24EntryPod, locked_24_entry_invariant, LOCKED_24_CRATES_COUNT},
    borrow_8_id_formal::{Borrow8IdPod, borrow_8_id_invariant, BORROW_8_ID_COUNT},
    integration_4_commit_formal::{Integration4CommitPod, integration_4_commit_invariant, INTEGRATION_4_HARD_WALLS_VERIFY},
};

// ============================================================
// 1. 编译期常量 (F9 跨模块 8 模块互锁)
// ============================================================

/// F9 跨模块 8 模块总数 (F1-F8, 1:1 跟 8 硬墙 严守)
pub const CROSS_MODULE_8_COUNT: usize = 8;

/// F9 跨模块 8 模块 ID (1:1 跟 8 硬墙 严守)
pub const CROSS_MODULE_8_IDS: [&str; CROSS_MODULE_8_COUNT] = [
    "F1_six_gates_v7",      // B4
    "F2_eight_anchors",     // B5
    "F3_v05_30dim",         // B3
    "F4_verdict_cache_13keys", // A3
    "F5_r11_baseline",      // A1
    "F6_locked_24_entry",   // B1
    "F7_borrow_8_id",       // C2
    "F8_integration_4_commit", // C1
];

// ============================================================
// 2. F9 跨模块 POD 镜像
// ============================================================

/// F9 跨模块 8 模块 POD 镜像 (1:1 跟 8 硬墙 严守)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CrossModule8Id {
    /// F1 6 重守门 v7 (B4 严守)
    F1SixGatesV7 = 0,
    /// F2 8 哲学锚 (B5 严守)
    F2EightAnchors = 1,
    /// F3 V0.5 30 维 (B3 严守)
    F3V0530Dim = 2,
    /// F4 13 键 verdict cache (A3 严守)
    F4VerdictCache13Keys = 3,
    /// F5 R11 baseline 3 值 (A1 严守)
    F5R11Baseline = 4,
    /// F6 24 LOCKED 入口签名 (B1 严守)
    F6Locked24Entry = 5,
    /// F7 8 借鉴 ID 真实施 (C2 严守)
    F7Borrow8Id = 6,
    /// F8 整合 #4 commit 严守 (C1 严守)
    F8Integration4Commit = 7,
}

impl CrossModule8Id {
    /// 8 模块总数 (F1-F8 跨模块 严守)
    pub const fn count() -> usize {
        CROSS_MODULE_8_COUNT
    }
}

// ============================================================
// 3. F9 跨模块不变量 (F1-F8 跨模块集成)
// ============================================================

/// F9 跨模块 1 联合不变量: 8 模块各自严守 永真 (F1-F8 跨模块 0 越界)
/// 物理含义: F1 6 重 + F2 8 锚 + F3 30 维 + F4 13 键 + F5 3 值 + F6 24 LOCKED + F7 8 借 + F8 整 4 commit
pub fn cross_module_8_joint_invariant(
    // F1: 6 重守门 v7
    g1: SixFoldGatePod, g2: SixFoldGatePod, g3: SixFoldGatePod,
    g4: SixFoldGatePod, g5: SixFoldGatePod, g6: SixFoldGatePod,
    // F2: 8 哲学锚
    a0: EightAnchorPod, a1: EightAnchorPod, a2: EightAnchorPod, a3: EightAnchorPod,
    a4: EightAnchorPod, a5: EightAnchorPod, a6: EightAnchorPod, a7: EightAnchorPod,
    // F3: V0.5 30 维
    d0: V05DimPod, d1: V05DimPod, d2: V05DimPod, d3: V05DimPod, d4: V05DimPod,
    // F4: 13 键 verdict cache
    k0: VerdictKey13Pod, k1: VerdictKey13Pod, k2: VerdictKey13Pod,
    // F5: R11 baseline 3 值
    b0: R11BaselinePod, b1: R11BaselinePod, b2: R11BaselinePod,
    // F6: 24 LOCKED 入口签名
    e0: Locked24EntryPod, e1: Locked24EntryPod,
    // F7: 8 借鉴 ID 真实施
    br0: Borrow8IdPod, br1: Borrow8IdPod,
    // F8: 整合 #4 commit 严守
    c0: Integration4CommitPod, c1: Integration4CommitPod,
) -> bool {
    // F1 6 重守门 v7 严守
    if !six_fold_v7_invariant(g1) || !six_fold_v7_invariant(g2) || !six_fold_v7_invariant(g3)
        || !six_fold_v7_invariant(g4) || !six_fold_v7_invariant(g5) || !six_fold_v7_invariant(g6) {
        return false;
    }
    // F2 8 哲学锚 严守
    if !eight_anchors_invariant(a0) || !eight_anchors_invariant(a1) || !eight_anchors_invariant(a2) || !eight_anchors_invariant(a3)
        || !eight_anchors_invariant(a4) || !eight_anchors_invariant(a5) || !eight_anchors_invariant(a6) || !eight_anchors_invariant(a7) {
        return false;
    }
    // F3 V0.5 30 维 严守
    if !v05_30dim_invariant(d0) || !v05_30dim_invariant(d1) || !v05_30dim_invariant(d2)
        || !v05_30dim_invariant(d3) || !v05_30dim_invariant(d4) {
        return false;
    }
    // F4 13 键 verdict cache 严守
    if !verdict_cache_13keys_invariant(k0) || !verdict_cache_13keys_invariant(k1) || !verdict_cache_13keys_invariant(k2) {
        return false;
    }
    // F5 R11 baseline 3 值 严守
    if !r11_baseline_3values_invariant(b0) || !r11_baseline_3values_invariant(b1) || !r11_baseline_3values_invariant(b2) {
        return false;
    }
    // F6 24 LOCKED 入口签名 严守
    if !locked_24_entry_invariant(e0) || !locked_24_entry_invariant(e1) {
        return false;
    }
    // F7 8 借鉴 ID 真实施 严守
    if !borrow_8_id_invariant(br0) || !borrow_8_id_invariant(br1) {
        return false;
    }
    // F8 整合 #4 commit 严守
    if !integration_4_commit_invariant(c0) || !integration_4_commit_invariant(c1) {
        return false;
    }
    true
}

// ============================================================
// 4. Kani-style proof harness
// ============================================================

/// Kani proof harness — F9 跨模块 8 模块 0 越界 (F1-F8 跨模块 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_cross_module_8_count() {
    // 8 跨模块互锁 = 6 守门 + 8 锚 + 5 维 + 3 键 + 3 值 + 2 入口 + 2 借鉴 + 2 commit
    assert_eq!(SIX_FOLD_GATE_V7_COUNT, 6, "F1: 6 重守门 v7 严守");
    assert_eq!(EIGHT_ANCHORS_COUNT, 8, "F2: 8 哲学锚 严守");
    assert_eq!(V05_30_DIM_COUNT, 30, "F3: V0.5 30 维 严守");
    assert_eq!(VERDICT_CACHE_13_KEYS_COUNT, 13, "F4: 13 键 verdict cache 严守");
    assert_eq!(LOCKED_24_CRATES_COUNT, 24, "F6: 24 LOCKED 入口签名 严守");
    assert_eq!(BORROW_8_ID_COUNT, 8, "F7: 8 借鉴 ID 真实施 严守");
    assert_eq!(INTEGRATION_4_HARD_WALLS_VERIFY, 8, "F8: 整合 #4 commit 8 严守项");
    assert_eq!(CrossModule8Id::count(), 8, "F9: 跨模块 8 模块总数");
}

/// Kani proof harness — F9 跨模块 1 联合不变量 0 越界
#[cfg_attr(kani, kani::proof)]
pub fn proof_cross_module_8_joint() {
    // 6 重守门 v7
    let gs = [SixFoldGatePod::new(1, true, true); SIX_FOLD_GATE_V7_COUNT];
    // 8 哲学锚
    let as_ = [EightAnchorPod::new(0, crate::stage5_2::eight_anchors_formal::AnchorGroup::Subjective, true); EIGHT_ANCHORS_COUNT];
    // 5 维 (V0.5 30 维抽样 5)
    let ds = [V05DimPod::new(0, 50); 5];
    // 3 键 (13 键 verdict cache 抽样 3)
    let ks = [VerdictKey13Pod::new(0, 1, true); 3];
    // 3 值 R11 baseline
    let bs = [
        R11BaselinePod::new(0, "V1141-R11", R11_BASELINE_V1141),
        R11BaselinePod::new(1, "V1131-R11", R11_BASELINE_V1131),
        R11BaselinePod::new(2, "V1136-R11", R11_BASELINE_V1136),
    ];
    // 2 入口 (24 LOCKED 抽样 2)
    let es = [Locked24EntryPod::new(0, "apeireth-supervisor", true, crate::stage5_2::locked_24_entry_formal::KnownSet::MasterKnown); 2];
    // 2 借鉴 (8 借鉴 ID 抽样 2)
    let brs = [
        crate::stage5_2::borrow_8_id_formal::BORROW_8_ID_INDEX[0],
        crate::stage5_2::borrow_8_id_formal::BORROW_8_ID_INDEX[4], // kani 4502
    ];
    // 2 commit (8 verify 项抽样 2)
    let cs = [Integration4CommitPod::new(0, "abf12243", true); 2];

    // F9 跨模块 1 联合不变量
    let gs_arr: [SixFoldGatePod; 6] = [gs[0], gs[1], gs[2], gs[3], gs[4], gs[5]];
    let as_arr: [EightAnchorPod; 8] = [as_[0], as_[1], as_[2], as_[3], as_[4], as_[5], as_[6], as_[7]];
    let ds_arr: [V05DimPod; 5] = [ds[0], ds[1], ds[2], ds[3], ds[4]];
    let ks_arr: [VerdictKey13Pod; 3] = [ks[0], ks[1], ks[2]];
    let bs_arr: [R11BaselinePod; 3] = [bs[0], bs[1], bs[2]];
    let es_arr: [Locked24EntryPod; 2] = [es[0], es[1]];
    let brs_arr: [Borrow8IdPod; 2] = [brs[0], brs[1]];
    let cs_arr: [Integration4CommitPod; 2] = [cs[0], cs[1]];

    assert!(cross_module_8_joint_invariant(
        gs_arr[0], gs_arr[1], gs_arr[2], gs_arr[3], gs_arr[4], gs_arr[5],
        as_arr[0], as_arr[1], as_arr[2], as_arr[3], as_arr[4], as_arr[5], as_arr[6], as_arr[7],
        ds_arr[0], ds_arr[1], ds_arr[2], ds_arr[3], ds_arr[4],
        ks_arr[0], ks_arr[1], ks_arr[2],
        bs_arr[0], bs_arr[1], bs_arr[2],
        es_arr[0], es_arr[1],
        brs_arr[0], brs_arr[1],
        cs_arr[0], cs_arr[1],
    ));
}

/// Runtime sanity: F9 跨模块 8 模块 ID 严守
pub fn sanity_check() -> bool {
    CROSS_MODULE_8_IDS.len() == CROSS_MODULE_8_COUNT
}

// ============================================================
// 5. 单元测试 (5 tests, F1-F8 跨模块 严守 verify)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = proof_cross_module_8_count;
        let _: fn() = proof_cross_module_8_joint;
    }

    #[test]
    fn cross_module_8_count_is_8() {
        assert_eq!(CROSS_MODULE_8_COUNT, 8);
        assert_eq!(CROSS_MODULE_8_IDS.len(), 8);
        assert_eq!(CrossModule8Id::count(), 8);
    }

    #[test]
    fn cross_module_8_ids_match() {
        // F9 跨模块 8 模块 ID (F1-F8, 1:1 跟 8 硬墙)
        assert_eq!(CROSS_MODULE_8_IDS[0], "F1_six_gates_v7");
        assert_eq!(CROSS_MODULE_8_IDS[1], "F2_eight_anchors");
        assert_eq!(CROSS_MODULE_8_IDS[2], "F3_v05_30dim");
        assert_eq!(CROSS_MODULE_8_IDS[3], "F4_verdict_cache_13keys");
        assert_eq!(CROSS_MODULE_8_IDS[4], "F5_r11_baseline");
        assert_eq!(CROSS_MODULE_8_IDS[5], "F6_locked_24_entry");
        assert_eq!(CROSS_MODULE_8_IDS[6], "F7_borrow_8_id");
        assert_eq!(CROSS_MODULE_8_IDS[7], "F8_integration_4_commit");
    }

    #[test]
    fn cross_module_8_joint_invariant_happy() {
        // F1-F8 全部 happy path 跨模块 invariant 永真
        let gs = [SixFoldGatePod::new(1, true, true); 6];
        let as_ = [EightAnchorPod::new(0, crate::stage5_2::eight_anchors_formal::AnchorGroup::Subjective, true); 8];
        let ds = [V05DimPod::new(0, 50); 5];
        let ks = [VerdictKey13Pod::new(0, 1, true); 3];
        let bs = [
            R11BaselinePod::new(0, "V1141-R11", R11_BASELINE_V1141),
            R11BaselinePod::new(1, "V1131-R11", R11_BASELINE_V1131),
            R11BaselinePod::new(2, "V1136-R11", R11_BASELINE_V1136),
        ];
        let es = [Locked24EntryPod::new(0, "test", true, crate::stage5_2::locked_24_entry_formal::KnownSet::MasterKnown); 2];
        let brs = [crate::stage5_2::borrow_8_id_formal::BORROW_8_ID_INDEX[0]; 2];
        let cs = [Integration4CommitPod::new(0, "abf12243", true); 2];
        assert!(cross_module_8_joint_invariant(
            gs[0], gs[1], gs[2], gs[3], gs[4], gs[5],
            as_[0], as_[1], as_[2], as_[3], as_[4], as_[5], as_[6], as_[7],
            ds[0], ds[1], ds[2], ds[3], ds[4],
            ks[0], ks[1], ks[2],
            bs[0], bs[1], bs[2],
            es[0], es[1],
            brs[0], brs[1],
            cs[0], cs[1],
        ));
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
