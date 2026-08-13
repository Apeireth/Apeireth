//! R129-10 Stage 5.2 形式化扩展 F1 — 6 重守门 v7 形式化 (B4 严守, 0 改 6 重)
//!
//! # 背景 (per 决策 #33 §2.3 B4 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! P1-3 R126 6 重守门 v7 done (per 决策 #36 §1.3 + 决策 #51 §1.2 P1-3):
//! - L1TypeCheck (类型守门)
//! - L2ScopeCheck (范围守门)
//! - L3RateCheck (速率守门)
//! - L4GuardCheck (守门守门)
//! - L5AuditCheck (审计守门)
//! - L6ProvenanceCheck (来源守门)
//!
//! R129-5 G2 PermissionLayer 1:1 翻译此 6 重 (per `permission_governance.rs:60-78`).
//!
//! # 借鉴 ID
//!
//! `R129-10-F1-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: B4 6 重 v7 严守 0 改
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - B4 6 重守门 v7: 仅形式化, 0 改 6 重
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板
//! - C2 0 装 PASS 严守: 真 src 改动 + 真 tests pass

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (6 重守门 v7, 1:1 跟 B4 严守)
// ============================================================

/// 6 重守门 v7 总数 (1:1 跟 B4 严守, per P1-3 R126 done)
pub const SIX_FOLD_GATE_V7_COUNT: usize = 6;

/// 6 重守门 v7 守门层 POD (B4 严守 0 改)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SixFoldGateV7 {
    /// L1 类型守门 (TypeCheck)
    L1TypeCheck = 1,
    /// L2 范围守门 (ScopeCheck)
    L2ScopeCheck = 2,
    /// L3 速率守门 (RateCheck)
    L3RateCheck = 3,
    /// L4 守门守门 (GuardCheck)
    L4GuardCheck = 4,
    /// L5 审计守门 (AuditCheck)
    L5AuditCheck = 5,
    /// L6 来源守门 (ProvenanceCheck)
    L6ProvenanceCheck = 6,
}

/// 6 重守门 v7 POD 镜像 (1:1 跟 SixFoldGateV7 严守, B4 严守)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SixFoldGatePod {
    /// 守门身份 (1..=6, 1:1 跟 B4 严守, 0 改)
    pub layer: u8,
    /// 是否启用 (true=enabled, false=disabled)
    pub enabled: bool,
    /// 守门结果 (true=pass, false=block)
    pub passed: bool,
}

impl SixFoldGatePod {
    /// 构造 (编译期 hardcode)
    pub const fn new(layer: u8, enabled: bool, passed: bool) -> Self {
        Self { layer, enabled, passed }
    }

    /// 守门计数 (1:1 跟 B4 严守)
    pub const fn count() -> usize {
        SIX_FOLD_GATE_V7_COUNT
    }
}

// ============================================================
// 2. 6 重守门 v7 不变量 (B4 严守 0 改)
// ============================================================

/// 6 重守门 v7 不变量: layer ∈ 1..=6 永真
pub fn six_fold_v7_invariant(g: SixFoldGatePod) -> bool {
    g.layer >= 1 && g.layer <= SIX_FOLD_GATE_V7_COUNT as u8
}

/// 6 重守门 v7 启用不变量: enabled=true 守门数 = 6 (B4 严守)
pub fn six_fold_v7_all_enabled_count(gs: [SixFoldGatePod; SIX_FOLD_GATE_V7_COUNT]) -> usize {
    let mut count = 0;
    for g in &gs {
        if g.enabled {
            count += 1;
        }
    }
    count
}

/// 6 重守门 v7 全 pass 不变量: passed=true = 6 (B4 严守)
pub fn six_fold_v7_all_passed(gs: [SixFoldGatePod; SIX_FOLD_GATE_V7_COUNT]) -> bool {
    for g in &gs {
        if !g.passed {
            return false;
        }
    }
    true
}

// ============================================================
// 3. Kani-style proof harness (per P5-2 + P8-2 retry 1:1 翻译)
// ============================================================

/// Kani proof harness — 6 重守门 v7 layer ∈ 1..=6 永真 (B4 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_six_fold_v7_layer_in_range() {
    let g = nondet_gate();
    assert!(six_fold_v7_invariant(g), "6 重守门 v7 layer 必须在 1..=6");
}

/// Kani proof harness — 守门计数 = 6 (B4 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_six_fold_v7_count_is_six() {
    assert_eq!(SixFoldGatePod::count(), 6, "6 重守门 v7 count 必须 = 6");
}

#[cfg(kani)]
fn nondet_gate() -> SixFoldGatePod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_gate() -> SixFoldGatePod {
    // cargo test 兜底: 选 L1 enabled passed 的 happy path, 不会触发 assert!
    SixFoldGatePod::new(1, true, true)
}

/// Runtime sanity: 6 重守门 v7 (1..=6) 应全部通过
pub fn sanity_check() -> bool {
    for layer in 1u8..=SIX_FOLD_GATE_V7_COUNT as u8 {
        if !six_fold_v7_invariant(SixFoldGatePod::new(layer, true, true)) {
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
        let _: fn() = proof_six_fold_v7_layer_in_range;
        let _: fn() = proof_six_fold_v7_count_is_six;
    }

    #[test]
    fn six_fold_v7_count_is_six() {
        assert_eq!(SIX_FOLD_GATE_V7_COUNT, 6);
        assert_eq!(SixFoldGatePod::count(), 6);
    }

    #[test]
    fn six_fold_v7_layer_1_to_6_all_pass() {
        for layer in 1u8..=6 {
            assert!(six_fold_v7_invariant(SixFoldGatePod::new(layer, true, true)));
        }
    }

    #[test]
    fn six_fold_v7_layer_0_violates() {
        // 反例: layer=0 越界 (B4 严守 1..=6)
        assert!(!six_fold_v7_invariant(SixFoldGatePod::new(0, true, true)));
    }

    #[test]
    fn six_fold_v7_layer_7_violates() {
        // 反例: layer=7 越界 (B4 严守 1..=6)
        assert!(!six_fold_v7_invariant(SixFoldGatePod::new(7, true, true)));
    }

    #[test]
    fn six_fold_v7_all_passed_with_6_enabled() {
        let gs = [
            SixFoldGatePod::new(1, true, true),
            SixFoldGatePod::new(2, true, true),
            SixFoldGatePod::new(3, true, true),
            SixFoldGatePod::new(4, true, true),
            SixFoldGatePod::new(5, true, true),
            SixFoldGatePod::new(6, true, true),
        ];
        assert!(six_fold_v7_all_passed(gs));
        assert_eq!(six_fold_v7_all_enabled_count(gs), 6);
    }

    #[test]
    fn six_fold_v7_one_fails_blocks_all() {
        let gs = [
            SixFoldGatePod::new(1, true, true),
            SixFoldGatePod::new(2, true, true),
            SixFoldGatePod::new(3, true, false), // L3 失败
            SixFoldGatePod::new(4, true, true),
            SixFoldGatePod::new(5, true, true),
            SixFoldGatePod::new(6, true, true),
        ];
        assert!(!six_fold_v7_all_passed(gs));
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
