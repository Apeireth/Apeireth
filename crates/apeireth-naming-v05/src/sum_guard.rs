//! # sum_guard — 4 大类权重 sum=1.00 守门 (R20 阶段 4 估补核心)
//!
//! ## 设计要点
//!
//! 1. **4 大类权重**: PC (Positive Capability) 0.40 / RC (Risk Constraint) 0.30
//!    / HG (Honesty Gap) 0.15 / GP (Growth Phase) 0.15 — sum 必须 = 1.00 (守门).
//! 2. **编译期 hardcode**: `DEFAULT_WEIGHTS` 编译期 constant, 不允许运行时修改.
//! 3. **容差 0.001**: 浮点 f32 累加误差容差, 1.00 ± 0.001 都算通过.
//! 4. **m3 防御**: `check_sum_equals_1` 是入口守门, encode/decode 全程调用.
//!
//! ## 4 大类来源 (per v1077 V0.5 17 维 LOCKED + 提议 v2 24 维)
//!
//! | 类 | 全称 | 估权重 | 含义 |
//! |---|---|------|---|
//! | PC | Positive Capability | 0.40 | 正向能力 (代码 / 对话 / 视觉 / 音频 / 工具 / 推理) |
//! | RC | Risk Constraint | 0.30 | 风险约束 (守门 / 权限 / 安全等级) |
//! | HG | Honesty Gap | 0.15 | 诚实标缺 (知道什么不知道, 不假装) |
//! | GP | Growth Phase | 0.15 | 成长阶段 (level 0=seed, 9=mature) |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.
//!
//! 0 改 v1077 17 维 LOCKED / 0 改 workspace version / 0 改 24 LOCKED crate /
//! 0 引 NewAPI / 0 假装已实现 / 0 改 LOCKED 文档 / 0 git add / 0 git commit.

use crate::error::{NamingError, NamingResult};

// ============================================================================
// §1 守门常量 (编译期 hardcode, m3 防御)
// ============================================================================

/// 容差: 浮点 f32 累加误差, 1.00 ± 0.001 都算 sum=1.00.
pub const SUM_GUARD_TOLERANCE: f32 = 0.001;

/// 守门目标: sum 必须等于 1.00.
pub const SUM_GUARD_TARGET: f32 = 1.00;

// ============================================================================
// §2 4 大类权重结构 (编译期 hardcode 字段)
// ============================================================================

/// 4 大类权重结构 (sum 必须 = 1.00, 守门).
///
/// ## 字段 (顺序 = 大类重要性顺序, 跟 class.rs 1:1)
/// - `pc`: Positive Capability (估 0.40)
/// - `rc`: Risk Constraint (估 0.30)
/// - `hg`: Honesty Gap (估 0.15)
/// - `gp`: Growth Phase (估 0.15)
///
/// ## 守门
/// `check_sum_equals_1(&weights)` 必须返回 Ok 才能 encode/decode.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClassWeights {
    /// Positive Capability (正向能力) 估 0.40.
    pub pc: f32,
    /// Risk Constraint (风险约束) 估 0.30.
    pub rc: f32,
    /// Honesty Gap (诚实标缺) 估 0.15.
    pub hg: f32,
    /// Growth Phase (成长阶段) 估 0.15.
    pub gp: f32,
}

impl ClassWeights {
    /// 构造新权重.
    pub const fn new(pc: f32, rc: f32, hg: f32, gp: f32) -> Self {
        Self { pc, rc, hg, gp }
    }

    /// 计算 4 大类权重和.
    #[inline]
    pub fn sum(&self) -> f32 {
        self.pc + self.rc + self.hg + self.gp
    }

    /// 跟目标 (1.00) 的偏差.
    #[inline]
    pub fn delta(&self) -> f32 {
        (self.sum() - SUM_GUARD_TARGET).abs()
    }

    /// 守门自检: sum 在容差内 (1.00 ± 0.001).
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.delta() < SUM_GUARD_TOLERANCE
    }
}

/// 默认权重 (4 大类 估 0.40 / 0.30 / 0.15 / 0.15, sum=1.00 守门).
///
/// 编译期 constant, 不允许运行时修改. 任何想改默认权重的尝试必须改源码.
pub const DEFAULT_WEIGHTS: ClassWeights = ClassWeights {
    pc: 0.40,
    rc: 0.30,
    hg: 0.15,
    gp: 0.15,
};

// ============================================================================
// §3 守门函数 (入口守门, encode/decode 全程调用)
// ============================================================================

/// 守门入口: 检查 4 大类权重和是否等于 1.00 (容差 0.001).
///
/// 守门破坏时返 `NamingError::SumNotEquals1 { sum, delta }`.
///
/// ## 用法
/// ```ignore
/// let w = ClassWeights::new(0.40, 0.30, 0.15, 0.15);
/// check_sum_equals_1(&w)?;  // Ok(())
///
/// let bad = ClassWeights::new(0.50, 0.30, 0.15, 0.15);
/// let err = check_sum_equals_1(&bad).unwrap_err();
/// assert!(matches!(err, NamingError::SumNotEquals1 { .. }));
/// ```
pub fn check_sum_equals_1(w: &ClassWeights) -> NamingResult<()> {
    let sum = w.sum();
    let delta = w.delta();
    if delta < SUM_GUARD_TOLERANCE {
        Ok(())
    } else {
        Err(NamingError::SumNotEquals1 { sum, delta })
    }
}

/// 守门入口 (默认权重版, 永远 OK, 但保留守门接口).
///
/// 用法: `check_sum_equals_1_default()?;` 验证 DEFAULT_WEIGHTS 守门通过.
pub fn check_sum_equals_1_default() -> NamingResult<()> {
    check_sum_equals_1(&DEFAULT_WEIGHTS)
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 默认权重 sum=1.00 守门通过.
    #[test]
    fn default_weights_sum_equals_1() {
        // 0.40 + 0.30 + 0.15 + 0.15 = 1.00 (浮点精确)
        assert!((DEFAULT_WEIGHTS.sum() - 1.0).abs() < 1e-6);
        assert!(DEFAULT_WEIGHTS.is_valid());
        assert!(check_sum_equals_1(&DEFAULT_WEIGHTS).is_ok());
    }

    /// 守门 #2: 默认权重各字段 hardcode.
    #[test]
    fn default_weights_field_values() {
        assert_eq!(DEFAULT_WEIGHTS.pc, 0.40);
        assert_eq!(DEFAULT_WEIGHTS.rc, 0.30);
        assert_eq!(DEFAULT_WEIGHTS.hg, 0.15);
        assert_eq!(DEFAULT_WEIGHTS.gp, 0.15);
    }

    /// 守门 #3: sum=1.05 守门拒绝.
    #[test]
    fn sum_1_05_rejected() {
        let bad = ClassWeights::new(0.50, 0.30, 0.15, 0.10); // sum=1.05 (f32 累加 ≈ 1.0500001)
        // f32 加法有微小浮点误差, 用容差比较
        assert!((bad.sum() - 1.05).abs() < 1e-5, "sum 应 ≈ 1.05, 实际 {}", bad.sum());
        let err = check_sum_equals_1(&bad).unwrap_err();
        match err {
            NamingError::SumNotEquals1 { sum, delta } => {
                assert!((sum - 1.05).abs() < 1e-5, "sum 应 ≈ 1.05, 实际 {sum}");
                assert!((delta - 0.05).abs() < 1e-5, "delta 应 ≈ 0.05, 实际 {delta}");
            }
            other => panic!("期望 SumNotEquals1, 实际: {other:?}"),
        }
    }

    /// 守门 #4: sum=0.95 守门拒绝.
    #[test]
    fn sum_0_95_rejected() {
        let bad = ClassWeights::new(0.30, 0.30, 0.15, 0.20); // sum=0.95
        assert_eq!(bad.sum(), 0.95);
        assert!(check_sum_equals_1(&bad).is_err());
    }

    /// 守门 #5: 容差 0.001 内接受.
    #[test]
    fn tolerance_within_0_001_accepted() {
        // 1.00 + 0.0005 = 1.0005 在容差内
        let ok = ClassWeights::new(0.4005, 0.30, 0.15, 0.15);
        assert!(ok.is_valid());
        assert!(check_sum_equals_1(&ok).is_ok());

        // 1.00 + 0.0009 = 1.0009 在容差内
        let ok2 = ClassWeights::new(0.4009, 0.30, 0.15, 0.15);
        assert!(ok2.is_valid());

        // 1.00 + 0.002 = 1.002 超出容差
        let bad = ClassWeights::new(0.402, 0.30, 0.15, 0.15);
        assert!(!bad.is_valid());
        assert!(check_sum_equals_1(&bad).is_err());
    }

    /// 守门 #6: ClassWeights::new 构造 + delta() 算.
    #[test]
    fn class_weights_new_and_delta() {
        let w = ClassWeights::new(0.40, 0.30, 0.15, 0.15);
        assert_eq!(w.sum(), 1.00);
        assert!(w.delta() < 1e-6);
        assert!(w.is_valid());

        let w2 = ClassWeights::new(0.25, 0.25, 0.25, 0.25);
        assert_eq!(w2.sum(), 1.00);
        assert!(w2.is_valid());
    }

    /// 守门 #7: ClassWeights Serialize/Deserialize (serde roundtrip).
    #[test]
    fn class_weights_serde_roundtrip() {
        let w = DEFAULT_WEIGHTS;
        let s = serde_json::to_string(&w).unwrap();
        let parsed: ClassWeights = serde_json::from_str(&s).unwrap();
        assert_eq!(w, parsed);
    }
}
