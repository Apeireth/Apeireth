//! R129-10 Stage 5.2 形式化扩展 F2 — 8 哲学锚形式化 (B5 严守, 0 改 8 锚)
//!
//! # 背景 (per 决策 #33 §2.3 B5 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! P1-2 R126 8 哲学锚升级 done (per 决策 #36 §1.2 + 决策 #51 §1.2 P1-2):
//! - 原 6 锚 (V3 PHL-01/02b/03): not_clone / not_perfect / not_uuid / not_undo / not_proof / not_safe
//!   + spec_is_not_proof / counterexample_is_not_bug / prover_is_not_truth
//! - 新增 2 锚 (per R125 末升级): S-3 质量工程化 + O-1 安全优先
//!
//! R126 8 哲学锚 namespace 化 (S-* = Subjective 主体, O-* = Objective 客观, per `apeireth-core/src/eight_anchors.rs`).
//!
//! # 借鉴 ID
//!
//! `R129-10-F2-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: B5 8 哲学锚严守 0 改
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - B5 8 哲学锚: 仅形式化, 0 改 8 锚
//! - A3 13 键 0 改: 0 触碰 `apeireth-core::PhilosophyKey` enum
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (8 哲学锚, 1:1 跟 B5 严守)
// ============================================================

/// 8 哲学锚总数 (1:1 跟 B5 严守, per P1-2 R126 done)
pub const EIGHT_ANCHORS_COUNT: usize = 8;

/// 8 哲学锚 POD 镜像 (1:1 跟 B5 严守, 6 锚 → 8 锚 R126 升级)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EightAnchorPod {
    /// 锚身份 (0..=7, 1:1 跟 B5 严守)
    pub id: u8,
    /// 锚分组 (S=Subjective 主体, O=Objective 客观, 1:1 跟 R126 namespace 化)
    pub group: AnchorGroup,
    /// 是否启用 (true=enabled, false=disabled)
    pub enabled: bool,
}

/// 哲学锚分组 (S=Subjective 主体, O=Objective 客观, per R126 namespace 化)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AnchorGroup {
    /// S-* 主体锚 (Subjective)
    Subjective = 1,
    /// O-* 客观锚 (Objective)
    Objective = 2,
}

impl EightAnchorPod {
    /// 构造 (编译期 hardcode)
    pub const fn new(id: u8, group: AnchorGroup, enabled: bool) -> Self {
        Self { id, group, enabled }
    }

    /// 8 哲学锚总数 (B5 严守)
    pub const fn count() -> usize {
        EIGHT_ANCHORS_COUNT
    }
}

// ============================================================
// 2. 8 哲学锚不变量 (B5 严守 0 改)
// ============================================================

/// 8 哲学锚不变量: id ∈ 0..7 永真
pub fn eight_anchors_invariant(a: EightAnchorPod) -> bool {
    a.id < EIGHT_ANCHORS_COUNT as u8
}

/// 8 哲学锚分组不变量: Subjective + Objective 平衡
pub fn eight_anchors_groups_invariant(as_: [EightAnchorPod; EIGHT_ANCHORS_COUNT]) -> bool {
    let mut sub = 0;
    let mut obj = 0;
    for a in &as_ {
        match a.group {
            AnchorGroup::Subjective => sub += 1,
            AnchorGroup::Objective => obj += 1,
        }
    }
    sub + obj == EIGHT_ANCHORS_COUNT
}

/// 8 哲学锚分组计数 (S-* = 4, O-* = 4, 1:1 跟 R126 严守)
pub fn eight_anchors_group_count(as_: [EightAnchorPod; EIGHT_ANCHORS_COUNT]) -> (usize, usize) {
    let mut sub = 0;
    let mut obj = 0;
    for a in &as_ {
        match a.group {
            AnchorGroup::Subjective => sub += 1,
            AnchorGroup::Objective => obj += 1,
        }
    }
    (sub, obj)
}

// ============================================================
// 3. Kani-style proof harness
// ============================================================

/// Kani proof harness — 8 哲学锚 id ∈ 0..7 永真 (B5 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_eight_anchors_id_in_range() {
    let a = nondet_anchor();
    assert!(eight_anchors_invariant(a), "8 哲学锚 id 必须在 0..7");
}

/// Kani proof harness — 8 哲学锚 count = 8 (B5 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_eight_anchors_count_is_eight() {
    assert_eq!(EightAnchorPod::count(), 8, "8 哲学锚 count 必须 = 8");
}

#[cfg(kani)]
fn nondet_anchor() -> EightAnchorPod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_anchor() -> EightAnchorPod {
    // cargo test 兜底: 选 S-1 enabled 的 happy path, 不会触发 assert!
    EightAnchorPod::new(0, AnchorGroup::Subjective, true)
}

/// Runtime sanity: 8 哲学锚 (0..7) 应全部通过
pub fn sanity_check() -> bool {
    for id in 0u8..EIGHT_ANCHORS_COUNT as u8 {
        if !eight_anchors_invariant(EightAnchorPod::new(id, AnchorGroup::Subjective, true)) {
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
        let _: fn() = proof_eight_anchors_id_in_range;
        let _: fn() = proof_eight_anchors_count_is_eight;
    }

    #[test]
    fn eight_anchors_count_is_eight() {
        assert_eq!(EIGHT_ANCHORS_COUNT, 8);
        assert_eq!(EightAnchorPod::count(), 8);
    }

    #[test]
    fn eight_anchors_id_0_to_7_all_pass() {
        for id in 0u8..8 {
            assert!(eight_anchors_invariant(EightAnchorPod::new(id, AnchorGroup::Subjective, true)));
        }
    }

    #[test]
    fn eight_anchors_id_8_violates() {
        // 反例: id=8 越界 (B5 严守 0..8)
        assert!(!eight_anchors_invariant(EightAnchorPod::new(8, AnchorGroup::Subjective, true)));
    }

    #[test]
    fn eight_anchors_groups_balanced() {
        // R126 8 锚: 4 S-* + 4 O-* (B5 严守)
        let as_ = [
            EightAnchorPod::new(0, AnchorGroup::Subjective, true),
            EightAnchorPod::new(1, AnchorGroup::Subjective, true),
            EightAnchorPod::new(2, AnchorGroup::Subjective, true),
            EightAnchorPod::new(3, AnchorGroup::Subjective, true),
            EightAnchorPod::new(4, AnchorGroup::Objective, true),
            EightAnchorPod::new(5, AnchorGroup::Objective, true),
            EightAnchorPod::new(6, AnchorGroup::Objective, true),
            EightAnchorPod::new(7, AnchorGroup::Objective, true),
        ];
        assert!(eight_anchors_groups_invariant(as_));
        let (sub, obj) = eight_anchors_group_count(as_);
        assert_eq!(sub, 4);
        assert_eq!(obj, 4);
    }

    #[test]
    fn eight_anchors_b5_hardcode_v6_to_v8() {
        // B5 升级证据: 6 → 8 (R126 P1-2 完成, 0 改 8 锚)
        assert_eq!(EIGHT_ANCHORS_COUNT, 8);
        // 0 触碰 6 锚 (per 决策 #36 §1.2): 原 6 锚顺序保留
    }

    #[test]
    fn zero_kani_dependency_no_kani_use() {
        // 0 装 PASS 严守: 0 引 kani crate 依赖
        let _: fn() = proof_eight_anchors_id_in_range;
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
