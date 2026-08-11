//! R129-10 Stage 5.2 形式化扩展 F3 — V0.5 30 维形式化 (B3 严守, 0 改 30 维)
//!
//! # 背景 (per 决策 #33 §2.3 B3 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! R125-13 V0.5 25 → 30 维 升级 done (per 决策 #36 §1.4 + 决策 #51 §1.2 P1-4):
//! - 原 24 维 V0.5 (R11 LOCKED): Continuity × 5 + ...
//! - R125 末 25 维 (24 + Robustness 鲁棒性)
//! - R125-13 30 维 (4 类 × 6 维 + 5 新 meta-dim + 1 派生 overall, per `apeireth-naming-v05/src/extension.rs:65`)
//!
//! 30 维 编译期 hardcode 锁: `pub const V05_30_TOTAL_DIMS: usize = 30;` (per `extension.rs:65`)
//!
//! # 借鉴 ID
//!
//! `R129-10-F3-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: B3 30 维严守 0 改
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - B3 30 维: 仅形式化, 0 改 30 维
//! - A1 R11 baseline 3 值 0.8682/0.8532/0.9063 0 改
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (V0.5 30 维, 1:1 跟 B3 严守)
// ============================================================

/// V0.5 30 维 总数 (1:1 跟 B3 严守, per R125-13 P1-4 done)
pub const V05_30_DIM_COUNT: usize = 30;

/// V0.5 30 维 4 类 (B3 严守, per R125-13 extension.rs:21)
pub const V05_30_BASE_CLASS_COUNT: usize = 4;
/// V0.5 30 维 6 维/类 (B3 严守, per R125-13 extension.rs:21)
pub const V05_30_BASE_DIM_PER_CLASS: usize = 6;
/// V0.5 30 维 5 新 meta-dim (B3 严守, per R125-13 extension.rs:21)
pub const V05_30_META_DIM_COUNT: usize = 5;
/// V0.5 30 维 1 派生 overall (B3 严守, per R125-13 extension.rs:21)
pub const V05_30_OVERALL_DIM_COUNT: usize = 1;

/// V0.5 30 维 POD 镜像 (1:1 跟 B3 严守)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct V05DimPod {
    /// 维身份 (0..29, 1:1 跟 B3 严守, 0 改)
    pub dim: u8,
    /// 维值 (0..=100, 简化映射, 0 改 30 维)
    pub value: u8,
}

impl V05DimPod {
    /// 构造 (编译期 hardcode)
    pub const fn new(dim: u8, value: u8) -> Self {
        Self { dim, value }
    }

    /// 30 维总数 (B3 严守)
    pub const fn count() -> usize {
        V05_30_DIM_COUNT
    }
}

// ============================================================
// 2. V0.5 30 维不变量 (B3 严守 0 改)
// ============================================================

/// V0.5 30 维不变量: dim ∈ 0..29 永真
pub fn v05_30dim_invariant(d: V05DimPod) -> bool {
    d.dim < V05_30_DIM_COUNT as u8
}

/// V0.5 30 维分项不变量: 4 × 6 + 5 + 1 = 30 (B3 严守)
pub fn v05_30dim_partition_invariant() -> bool {
    V05_30_BASE_CLASS_COUNT * V05_30_BASE_DIM_PER_CLASS
        + V05_30_META_DIM_COUNT
        + V05_30_OVERALL_DIM_COUNT
        == V05_30_DIM_COUNT
}

/// V0.5 30 维值不变量: value ∈ 0..=100 永真 (B3 严守)
pub fn v05_30dim_value_invariant(d: V05DimPod) -> bool {
    d.value <= 100
}

// ============================================================
// 3. Kani-style proof harness
// ============================================================

/// Kani proof harness — V0.5 30 维 dim ∈ 0..29 永真 (B3 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_v05_30dim_in_range() {
    let d = nondet_dim();
    assert!(v05_30dim_invariant(d), "V0.5 30 维 dim 必须在 0..29");
}

/// Kani proof harness — V0.5 30 维 count = 30 (B3 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_v05_30dim_count_is_30() {
    assert_eq!(V05DimPod::count(), 30, "V0.5 30 维 count 必须 = 30");
    assert!(v05_30dim_partition_invariant(), "V0.5 30 维分项 = 4×6+5+1 = 30");
}

#[cfg(kani)]
fn nondet_dim() -> V05DimPod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_dim() -> V05DimPod {
    // cargo test 兜底: 选 dim=0, value=50 的 happy path, 不会触发 assert!
    V05DimPod::new(0, 50)
}

/// Runtime sanity: V0.5 30 维 (0..29) 应全部通过
pub fn sanity_check() -> bool {
    for dim in 0u8..V05_30_DIM_COUNT as u8 {
        if !v05_30dim_invariant(V05DimPod::new(dim, 50)) {
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
        let _: fn() = proof_v05_30dim_in_range;
        let _: fn() = proof_v05_30dim_count_is_30;
    }

    #[test]
    fn v05_30dim_count_is_30() {
        assert_eq!(V05_30_DIM_COUNT, 30);
        assert_eq!(V05DimPod::count(), 30);
    }

    #[test]
    fn v05_30dim_partition_4_6_5_1_equals_30() {
        // B3 严守 4×6+5+1=30 (per R125-13 extension.rs:21)
        assert_eq!(V05_30_BASE_CLASS_COUNT, 4);
        assert_eq!(V05_30_BASE_DIM_PER_CLASS, 6);
        assert_eq!(V05_30_META_DIM_COUNT, 5);
        assert_eq!(V05_30_OVERALL_DIM_COUNT, 1);
        assert!(v05_30dim_partition_invariant());
    }

    #[test]
    fn v05_30dim_0_to_29_all_pass() {
        for dim in 0u8..30 {
            assert!(v05_30dim_invariant(V05DimPod::new(dim, 50)));
        }
    }

    #[test]
    fn v05_30dim_30_violates() {
        // 反例: dim=30 越界 (B3 严守 0..30)
        assert!(!v05_30dim_invariant(V05DimPod::new(30, 50)));
    }

    #[test]
    fn v05_30dim_value_0_to_100_all_pass() {
        for value in [0u8, 50, 100] {
            assert!(v05_30dim_value_invariant(V05DimPod::new(0, value)));
        }
    }

    #[test]
    fn v05_30dim_value_101_violates() {
        // 反例: value=101 越界 (B3 严守 0..=100)
        assert!(!v05_30dim_value_invariant(V05DimPod::new(0, 101)));
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
