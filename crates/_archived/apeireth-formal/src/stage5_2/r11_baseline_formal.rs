//! R129-10 Stage 5.2 形式化扩展 F5 — R11 baseline 3 值形式化 (A1 严守, 0 改 3 值)
//!
//! # 背景 (per 决策 #33 §2.3 A1 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! A1 R11 baseline 3 值 数字严守 (per 决策 #22 §5.1 + 决策 #33 §2.3 A1 + 决策 #36 §2.1):
//! - **V1141-R11 = 0.8682** (IC-001 fresh, 24 维 V0.5, per `crates/apeireth-asi/src/lib.rs:pub const V05_DIM_COUNT: usize = 24`)
//! - **V1131-R11 = 0.8532** (dashboard v05_total)
//! - **V1136-R11 = 0.9063** (综合均值, 9 子测度 per `crates/apeireth-asi/src/lib.rs:pub const V1136_SUBMEASURE_COUNT: usize = 9`)
//!
//! 17 文件原位 0 改 (per 决策 #22 §5.1, 8 项不修改承诺保留原意).
//!
//! # 借鉴 ID
//!
//! `R129-10-F5-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: A1 3 值严守 0 改
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - A1 3 值 数字: 仅形式化, 0 改 0.8682/0.8532/0.9063
//! - 0 触碰 17 文件原位 (per 决策 #22 §5.1)
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (R11 baseline 3 值, 1:1 跟 A1 严守)
// ============================================================

/// V1141-R11 baseline (IC-001 fresh, 24 维 V0.5) = 0.8682 (A1 严守, 数字 0 改)
/// 物理含义: V0.5 24 维 fresh 测试 baseline (per `docs/omnibus/r11-baseline.md`)
pub const R11_BASELINE_V1141: f64 = 0.8682;

/// V1131-R11 baseline (dashboard v05_total) = 0.8532 (A1 严守, 数字 0 改)
/// 物理含义: 仪表板 v05_total 显示 baseline (per `docs/omnibus/r11-baseline.md`)
pub const R11_BASELINE_V1131: f64 = 0.8532;

/// V1136-R11 baseline (综合均值, 9 子测度) = 0.9063 (A1 严守, 数字 0 改)
/// 物理含义: 9 organ 综合 R 值 (per `docs/omnibus/r11-baseline.md` + `V1136_SUBMEASURE_COUNT = 9`)
pub const R11_BASELINE_V1136: f64 = 0.9063;

/// R11 baseline 3 值索引 (1:1 跟 A1 严守, per `R11-BASELINE-VALUES-3-KEYS` 命名)
pub const R11_BASELINE_KEYS: [&str; 3] = ["V1141-R11", "V1131-R11", "V1136-R11"];

/// 3 值 POD 镜像 (1:1 跟 A1 严守, 0.8682/0.8532/0.9063 数字 0 改)
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct R11BaselinePod {
    /// 键索引 (0..2, 1:1 跟 A1 严守)
    pub key_index: u8,
    /// 键名 (V1141-R11 / V1131-R11 / V1136-R11, A1 严守)
    pub key_name: &'static str,
    /// 数字 baseline 值 (0.8682 / 0.8532 / 0.9063, A1 严守, 数字 0 改)
    pub value: f64,
}

impl R11BaselinePod {
    /// 构造 (编译期 hardcode, 数字 0 改)
    pub const fn new(key_index: u8, key_name: &'static str, value: f64) -> Self {
        Self { key_index, key_name, value }
    }

    /// 3 值总数 (A1 严守)
    pub const fn count() -> usize {
        3
    }
}

// ============================================================
// 2. R11 baseline 3 值不变量 (A1 严守 0 改)
// ============================================================

/// R11 baseline 3 值不变量: key_index ∈ 0..2 永真 (A1 严守)
pub fn r11_baseline_3values_invariant(b: R11BaselinePod) -> bool {
    b.key_index < 3
}

/// R11 baseline 3 值 数字严守不变量: V1141=0.8682 / V1131=0.8532 / V1136=0.9063 永真 (A1 严守, 0 改)
pub fn r11_baseline_3values_digital_hardcode(b: R11BaselinePod) -> bool {
    match b.key_index {
        0 => b.value.to_bits() == R11_BASELINE_V1141.to_bits(),
        1 => b.value.to_bits() == R11_BASELINE_V1131.to_bits(),
        2 => b.value.to_bits() == R11_BASELINE_V1136.to_bits(),
        _ => false,
    }
}

/// R11 baseline 3 值 范围不变量: value ∈ [0.85, 0.91] (per A1 严守 baseline 区间)
pub fn r11_baseline_3values_range_invariant(b: R11BaselinePod) -> bool {
    b.value >= 0.85 && b.value <= 0.91
}

// ============================================================
// 3. Kani-style proof harness
// ============================================================

/// Kani proof harness — R11 baseline 3 值 key_index ∈ 0..2 永真 (A1 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_r11_baseline_3values_in_range() {
    let b = nondet_baseline();
    assert!(r11_baseline_3values_invariant(b), "R11 baseline 3 值 key_index 必须在 0..2");
}

/// Kani proof harness — R11 baseline 3 值 数字严守 0.8682/0.8532/0.9063 (A1 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_r11_baseline_3values_digital_hardcode() {
    assert_eq!(R11BaselinePod::count(), 3, "R11 baseline 3 值 count 必须 = 3");
    // 数字 0 改: 0.8682/0.8532/0.9063
    assert!(R11_BASELINE_V1141.to_bits() == 0.8682_f64.to_bits(), "V1141 = 0.8682 严守");
    assert!(R11_BASELINE_V1131.to_bits() == 0.8532_f64.to_bits(), "V1131 = 0.8532 严守");
    assert!(R11_BASELINE_V1136.to_bits() == 0.9063_f64.to_bits(), "V1136 = 0.9063 严守");
}

#[cfg(kani)]
fn nondet_baseline() -> R11BaselinePod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_baseline() -> R11BaselinePod {
    // cargo test 兜底: V1141 0.8682 happy path
    R11BaselinePod::new(0, "V1141-R11", R11_BASELINE_V1141)
}

/// Runtime sanity: R11 baseline 3 值 (V1141/V1131/V1136) 应全部通过
pub fn sanity_check() -> bool {
    let baselines = [
        R11BaselinePod::new(0, "V1141-R11", R11_BASELINE_V1141),
        R11BaselinePod::new(1, "V1131-R11", R11_BASELINE_V1131),
        R11BaselinePod::new(2, "V1136-R11", R11_BASELINE_V1136),
    ];
    for b in &baselines {
        if !r11_baseline_3values_invariant(*b) {
            return false;
        }
        if !r11_baseline_3values_digital_hardcode(*b) {
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
        let _: fn() = proof_r11_baseline_3values_in_range;
        let _: fn() = proof_r11_baseline_3values_digital_hardcode;
    }

    #[test]
    fn r11_baseline_3values_count_is_3() {
        assert_eq!(R11BaselinePod::count(), 3);
        assert_eq!(R11_BASELINE_KEYS.len(), 3);
    }

    #[test]
    fn r11_baseline_v1141_is_0_8682() {
        // A1 严守: 数字 0 改
        assert!(R11_BASELINE_V1141.to_bits() == 0.8682_f64.to_bits());
    }

    #[test]
    fn r11_baseline_v1131_is_0_8532() {
        // A1 严守: 数字 0 改
        assert!(R11_BASELINE_V1131.to_bits() == 0.8532_f64.to_bits());
    }

    #[test]
    fn r11_baseline_v1136_is_0_9063() {
        // A1 严守: 数字 0 改
        assert!(R11_BASELINE_V1136.to_bits() == 0.9063_f64.to_bits());
    }

    #[test]
    fn r11_baseline_3values_all_digital_hardcode() {
        let baselines = [
            R11BaselinePod::new(0, "V1141-R11", R11_BASELINE_V1141),
            R11BaselinePod::new(1, "V1131-R11", R11_BASELINE_V1131),
            R11BaselinePod::new(2, "V1136-R11", R11_BASELINE_V1136),
        ];
        for b in &baselines {
            assert!(r11_baseline_3values_digital_hardcode(*b));
        }
    }

    #[test]
    fn r11_baseline_violation_caught() {
        // 反例: 数字 0.5 ≠ 0.8682
        let bad = R11BaselinePod::new(0, "V1141-R11", 0.5);
        assert!(!r11_baseline_3values_digital_hardcode(bad));
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
