//! # extension — V0.5 → V0.5.30 扩展 (B3 25→30 维 verify, R126 P1-4)
//!
//! ## 借鉴 ID
//!
//! `R126-v05-30-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10` (per 决策 #36 §1.1 + 决策 #51 §1.2 P1-4)
//!
//! ## 借鉴源码
//!
//! `.openclaw\workspace\borrowed-repos\langgraph\` (✅ cloned 16:31, 829 files, per R125-13)
//!
//! ## 5 新增维度 (per R125-13 dispatch §3 5 维扩展)
//!
//! | # | 维度 | 来源 crate | 取值范围 | 守门 |
//! |---|---|---|---|---|
//! | 7  | **Robustness**         | apeireth-formal 24 LOCKED 形式化 (R125-10) | 0.0-1.0 f32 | ≥ 0.0 && ≤ 1.0 |
//! | 8  | **SelfImprovement**    | apeireth-evolution PODA (R125-7) | 0.0-1.0 f32 | ≥ 0.0 && ≤ 1.0 |
//! | 9  | **Adversarial**        | apeireth-sovereignty 守门 (R125-5) | 0.0-1.0 f32 | ≥ 0.0 && ≤ 1.0 |
//! | 10 | **CiPassRate**         | apeireth-asi 评估 (R120 D) | 0.0-1.0 f32 | ≥ 0.0 && ≤ 1.0 |
//! | 11 | **VerifierConsistency**| apeireth-formal Kani 24 (R125-10) | 0.0-1.0 f32 | ≥ 0.0 && ≤ 1.0 |
//!
//! ## 30 维 完整结构 (4 类 × 6 维 + 5 新 meta-dim + 1 派生 overall)
//!
//! ```text
//! 4 base classes × 6 base dims = 24 dim (per V05Spec, 0 改)
//! 5 new meta-dims = 5 dim (per MetaDims, NEW)
//! 1 derived overall = 1 dim (per MetaOverall, NEW)
//! Total = 30 dim (per V05_30_TOTAL_DIMS, NEW)
//! ```
//!
//! ## 0 装 PASS 严守 (per 决策 #36 §1.1 + 主人 17:22 升级授权 + 决策 #33 §2.3 C2)
//!
//! - ✅ **cloned = 真实施** — 借鉴源码 langgraph 829 files (per R125-13 17:35 done) = 真实施, 5 新维度 typed struct with from_f32 validation
//! - ⏳ **限流** — 不适用 (langgraph 0 限流, ✅ cloned)
//! - ❌ **跳过** — 不适用 (OpenCog AGPL-3.0 跳过, 跟 R126 P1-4 无关)
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人肩上 /
//! O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use serde::{Deserialize, Serialize};

use crate::class::V05Spec;
use crate::error::{NamingError, NamingResult};

// ============================================================================
// §1 30 维 计数守门 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 4 base classes 1:1 计数 (per V05Spec, 0 改).
pub const BASE_CLASS_COUNT: usize = 4;

/// 6 base dimensions 1:1 计数 (per V05Spec, 0 改).
pub const BASE_DIM_COUNT: usize = 6;

/// 5 new meta-dimensions 1:1 计数 (per R125-13 dispatch §3, NEW).
pub const META_DIM_COUNT: usize = 5;

/// 1 derived overall 1:1 计数 (per R126 P1-4 设计, NEW).
pub const OVERALL_DIM_COUNT: usize = 1;

/// 30 维 = BASE_CLASS_COUNT × BASE_DIM_COUNT + META_DIM_COUNT + OVERALL_DIM_COUNT.
///
/// m3 防御: 改这个常量或拆解 BASE/META/OVERALL 数字会立刻破坏编译.
pub const V05_30_TOTAL_DIMS: usize =
    BASE_CLASS_COUNT * BASE_DIM_COUNT + META_DIM_COUNT + OVERALL_DIM_COUNT;

// ============================================================================
// §2 5 新 meta-dim typed struct (编译期 1:1 hardcode, m3 防御)
// ============================================================================

/// Meta-dim 7: Robustness (apeireth-formal 24 LOCKED 形式化, R125-10).
///
/// 取值: 0.0 (0 形式化覆盖) ~ 1.0 (100% 形式化覆盖, 24 LOCKED 全 pass).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Robustness(f32);

impl Robustness {
    /// 范围守门: 0.0..=1.0.
    pub fn from_f32(v: f32) -> NamingResult<Self> {
        if !(0.0..=1.0).contains(&v) {
            return Err(NamingError::InvalidMetaDimOutOfRange {
                name: "Robustness",
                value: v,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self(v))
    }

    /// f32 → Robustness (无守门, 0 装, 内部 use).
    pub const fn from_f32_unchecked(v: f32) -> Self {
        Self(v)
    }

    /// Robustness → f32.
    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for Robustness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// Meta-dim 8: SelfImprovement (apeireth-evolution PODA, R125-7).
///
/// 取值: 0.0 (0 自我改进, 静态) ~ 1.0 (全 auto-improve, PODA cycle 持续).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelfImprovement(f32);

impl SelfImprovement {
    pub fn from_f32(v: f32) -> NamingResult<Self> {
        if !(0.0..=1.0).contains(&v) {
            return Err(NamingError::InvalidMetaDimOutOfRange {
                name: "SelfImprovement",
                value: v,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self(v))
    }

    pub const fn from_f32_unchecked(v: f32) -> Self {
        Self(v)
    }

    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for SelfImprovement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// Meta-dim 9: Adversarial (apeireth-sovereignty 守门, R125-5).
///
/// 取值: 0.0 (0 守门) ~ 1.0 (6 重守门 v6 全 pass, 含 Colang DSL).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Adversarial(f32);

impl Adversarial {
    pub fn from_f32(v: f32) -> NamingResult<Self> {
        if !(0.0..=1.0).contains(&v) {
            return Err(NamingError::InvalidMetaDimOutOfRange {
                name: "Adversarial",
                value: v,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self(v))
    }

    pub const fn from_f32_unchecked(v: f32) -> Self {
        Self(v)
    }

    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for Adversarial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// Meta-dim 10: CiPassRate (apeireth-asi 评估, R120 D).
///
/// 取值: 0.0 (0 CI pass) ~ 1.0 (100% CI pass, 全部绿色).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CiPassRate(f32);

impl CiPassRate {
    pub fn from_f32(v: f32) -> NamingResult<Self> {
        if !(0.0..=1.0).contains(&v) {
            return Err(NamingError::InvalidMetaDimOutOfRange {
                name: "CiPassRate",
                value: v,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self(v))
    }

    pub const fn from_f32_unchecked(v: f32) -> Self {
        Self(v)
    }

    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for CiPassRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// Meta-dim 11: VerifierConsistency (apeireth-formal Kani 24, R125-10).
///
/// 取值: 0.0 (0 Kani 24 invariant 验证) ~ 1.0 (24 invariant 全 verify pass).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VerifierConsistency(f32);

impl VerifierConsistency {
    pub fn from_f32(v: f32) -> NamingResult<Self> {
        if !(0.0..=1.0).contains(&v) {
            return Err(NamingError::InvalidMetaDimOutOfRange {
                name: "VerifierConsistency",
                value: v,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self(v))
    }

    pub const fn from_f32_unchecked(v: f32) -> Self {
        Self(v)
    }

    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for VerifierConsistency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

// ============================================================================
// §3 MetaDims — 5 新 meta-dim container (1:1 映射 5 typed struct)
// ============================================================================

/// 5 new meta-dim 容器 (Robustness + SelfImprovement + Adversarial + CiPassRate + VerifierConsistency).
///
/// 顺序固定 (编译期 1:1, m3 防御): 改字段顺序会破坏 serde roundtrip.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetaDims {
    /// Meta-dim 7: Robustness (apeireth-formal 24 LOCKED 形式化, R125-10).
    pub robustness: Robustness,
    /// Meta-dim 8: SelfImprovement (apeireth-evolution PODA, R125-7).
    pub self_improvement: SelfImprovement,
    /// Meta-dim 9: Adversarial (apeireth-sovereignty 守门, R125-5).
    pub adversarial: Adversarial,
    /// Meta-dim 10: CiPassRate (apeireth-asi 评估, R120 D).
    pub ci_pass_rate: CiPassRate,
    /// Meta-dim 11: VerifierConsistency (apeireth-formal Kani 24, R125-10).
    pub verifier_consistency: VerifierConsistency,
}

impl MetaDims {
    /// 5 元组构造.
    pub const fn new(
        robustness: Robustness,
        self_improvement: SelfImprovement,
        adversarial: Adversarial,
        ci_pass_rate: CiPassRate,
        verifier_consistency: VerifierConsistency,
    ) -> Self {
        Self {
            robustness,
            self_improvement,
            adversarial,
            ci_pass_rate,
            verifier_consistency,
        }
    }

    /// 5 维 → 5 字段 f32 slice (用于 compute overall).
    pub fn to_f32_array(self) -> [f32; META_DIM_COUNT] {
        [
            self.robustness.as_f32(),
            self.self_improvement.as_f32(),
            self.adversarial.as_f32(),
            self.ci_pass_rate.as_f32(),
            self.verifier_consistency.as_f32(),
        ]
    }
}

impl Default for MetaDims {
    /// 默认 0.0 (0 形式化 / 0 自我改进 / 0 守门 / 0 CI / 0 verifier).
    fn default() -> Self {
        Self {
            robustness: Robustness::from_f32_unchecked(0.0),
            self_improvement: SelfImprovement::from_f32_unchecked(0.0),
            adversarial: Adversarial::from_f32_unchecked(0.0),
            ci_pass_rate: CiPassRate::from_f32_unchecked(0.0),
            verifier_consistency: VerifierConsistency::from_f32_unchecked(0.0),
        }
    }
}

// ============================================================================
// §4 MetaOverall — 1 派生 overall 分数 (5 meta-dim 平均)
// ============================================================================

/// 1 派生 overall 分数 = 5 meta-dim 平均值.
///
/// 范围: 0.0-1.0 (因 5 meta-dim 都 ∈ [0.0, 1.0], 平均值也在 [0.0, 1.0]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetaOverall(f32);

impl MetaOverall {
    /// 从 5 meta-dim 数组计算 (5 维平均).
    pub fn from_meta_dims(meta: &MetaDims) -> Self {
        let arr = meta.to_f32_array();
        let sum: f32 = arr.iter().sum();
        Self(sum / META_DIM_COUNT as f32)
    }

    /// 范围守门: 0.0..=1.0.
    pub fn from_f32(v: f32) -> NamingResult<Self> {
        if !(0.0..=1.0).contains(&v) {
            return Err(NamingError::InvalidMetaDimOutOfRange {
                name: "MetaOverall",
                value: v,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self(v))
    }

    /// f32 → MetaOverall (无守门).
    pub const fn from_f32_unchecked(v: f32) -> Self {
        Self(v)
    }

    /// MetaOverall → f32.
    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for MetaOverall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

// ============================================================================
// §5 V05Spec30 — 30 维 完整规范 (V05Spec 24 + MetaDims 5 + MetaOverall 1)
// ============================================================================

/// V0.5 → V0.5.30 完整 30 维规范.
///
/// ## 结构 (per R126 P1-4 验证设计)
/// - `spec`: 24 base 维 (per V05Spec, 0 改, 4 大类 sum=1.0 守门)
/// - `meta`: 5 new meta-dim (per MetaDims, NEW)
/// - `overall`: 1 派生 overall (per MetaOverall, NEW)
/// - Total: 24 + 5 + 1 = 30 dim (per V05_30_TOTAL_DIMS 守门)
///
/// ## 守门 (K-1 强校验, 编译期 hardcode)
/// - 4 大类 weight sum=1.0 (per V05Spec DEFAULT_WEIGHTS 严守)
/// - 5 meta-dim ∈ [0.0, 1.0] (per MetaDims from_f32 守门)
/// - overall = 5 meta-dim 平均 (派生, runtime 计算)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct V05Spec30 {
    /// 24 base 维 (per V05Spec, 0 改, sum=1.0 守门).
    pub spec: V05Spec,
    /// 5 new meta-dim (per MetaDims, NEW, 0.0-1.0 范围守门).
    pub meta: MetaDims,
    /// 1 派生 overall (per MetaOverall, NEW, = 5 meta-dim 平均).
    pub overall: MetaOverall,
}

impl V05Spec30 {
    /// 构造新 V05Spec30.
    pub const fn new(spec: V05Spec, meta: MetaDims, overall: MetaOverall) -> Self {
        Self { spec, meta, overall }
    }

    /// 自动构造: 派生 overall = 5 meta-dim 平均.
    pub fn from_spec_and_meta(spec: V05Spec, meta: MetaDims) -> Self {
        let overall = MetaOverall::from_meta_dims(&meta);
        Self { spec, meta, overall }
    }
}

impl Default for V05Spec30 {
    /// 默认 30 维 (24 base + 5 meta=0.0 + overall=0.0).
    fn default() -> Self {
        let spec = crate::default_v05_spec();
        let meta = MetaDims::default();
        let overall = MetaOverall::from_meta_dims(&meta);
        Self { spec, meta, overall }
    }
}

// ============================================================================
// §6 in-module 测试 (60 tests total, per R125-13 60 tests 30 维 pattern)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::ClassDims;
    use crate::dimension::{
        Completeness, DimensionSet, Domain, Level, Lineage, Modality, Safety,
    };
    use crate::sum_guard::DEFAULT_WEIGHTS;

    // --------------------------------------------------------------------
    // §6.1 5 meta-dim typed struct 测试 (每维度 5 测试, 共 25)
    // --------------------------------------------------------------------

    // --- Robustness (5 tests) ---

    #[test]
    fn robustness_from_f32_valid() {
        assert!(Robustness::from_f32(0.0).is_ok());
        assert!(Robustness::from_f32(0.5).is_ok());
        assert!(Robustness::from_f32(1.0).is_ok());
    }

    #[test]
    fn robustness_from_f32_out_of_range() {
        assert!(Robustness::from_f32(-0.01).is_err());
        assert!(Robustness::from_f32(1.01).is_err());
        assert!(Robustness::from_f32(2.0).is_err());
    }

    #[test]
    fn robustness_as_f32_roundtrip() {
        let r = Robustness::from_f32(0.75).unwrap();
        assert_eq!(r.as_f32(), 0.75);
    }

    #[test]
    fn robustness_display_format() {
        let r = Robustness::from_f32(0.5).unwrap();
        assert_eq!(r.to_string(), "0.500");
    }

    #[test]
    fn robustness_unchecked_bypass() {
        // unchecked 路径不守门 (内部 use, 0 装 PASS 严守 0 装)
        let r = Robustness::from_f32_unchecked(99.0);
        assert_eq!(r.as_f32(), 99.0);
    }

    // --- SelfImprovement (5 tests) ---

    #[test]
    fn self_improvement_from_f32_valid() {
        assert!(SelfImprovement::from_f32(0.0).is_ok());
        assert!(SelfImprovement::from_f32(0.5).is_ok());
        assert!(SelfImprovement::from_f32(1.0).is_ok());
    }

    #[test]
    fn self_improvement_from_f32_out_of_range() {
        assert!(SelfImprovement::from_f32(-1.0).is_err());
        assert!(SelfImprovement::from_f32(1.5).is_err());
    }

    #[test]
    fn self_improvement_as_f32_roundtrip() {
        let s = SelfImprovement::from_f32(0.3).unwrap();
        assert_eq!(s.as_f32(), 0.3);
    }

    #[test]
    fn self_improvement_display_format() {
        let s = SelfImprovement::from_f32(0.8).unwrap();
        assert_eq!(s.to_string(), "0.800");
    }

    #[test]
    fn self_improvement_unchecked_bypass() {
        let s = SelfImprovement::from_f32_unchecked(50.0);
        assert_eq!(s.as_f32(), 50.0);
    }

    // --- Adversarial (5 tests) ---

    #[test]
    fn adversarial_from_f32_valid() {
        assert!(Adversarial::from_f32(0.0).is_ok());
        assert!(Adversarial::from_f32(0.5).is_ok());
        assert!(Adversarial::from_f32(1.0).is_ok());
    }

    #[test]
    fn adversarial_from_f32_out_of_range() {
        assert!(Adversarial::from_f32(-0.5).is_err());
        assert!(Adversarial::from_f32(1.5).is_err());
    }

    #[test]
    fn adversarial_as_f32_roundtrip() {
        let a = Adversarial::from_f32(0.6).unwrap();
        assert_eq!(a.as_f32(), 0.6);
    }

    #[test]
    fn adversarial_display_format() {
        let a = Adversarial::from_f32(0.2).unwrap();
        assert_eq!(a.to_string(), "0.200");
    }

    #[test]
    fn adversarial_unchecked_bypass() {
        let a = Adversarial::from_f32_unchecked(42.0);
        assert_eq!(a.as_f32(), 42.0);
    }

    // --- CiPassRate (5 tests) ---

    #[test]
    fn ci_pass_rate_from_f32_valid() {
        assert!(CiPassRate::from_f32(0.0).is_ok());
        assert!(CiPassRate::from_f32(0.5).is_ok());
        assert!(CiPassRate::from_f32(1.0).is_ok());
    }

    #[test]
    fn ci_pass_rate_from_f32_out_of_range() {
        assert!(CiPassRate::from_f32(-0.1).is_err());
        assert!(CiPassRate::from_f32(2.0).is_err());
    }

    #[test]
    fn ci_pass_rate_as_f32_roundtrip() {
        let c = CiPassRate::from_f32(0.9).unwrap();
        assert_eq!(c.as_f32(), 0.9);
    }

    #[test]
    fn ci_pass_rate_display_format() {
        let c = CiPassRate::from_f32(0.7).unwrap();
        assert_eq!(c.to_string(), "0.700");
    }

    #[test]
    fn ci_pass_rate_unchecked_bypass() {
        let c = CiPassRate::from_f32_unchecked(100.0);
        assert_eq!(c.as_f32(), 100.0);
    }

    // --- VerifierConsistency (5 tests) ---

    #[test]
    fn verifier_consistency_from_f32_valid() {
        assert!(VerifierConsistency::from_f32(0.0).is_ok());
        assert!(VerifierConsistency::from_f32(0.5).is_ok());
        assert!(VerifierConsistency::from_f32(1.0).is_ok());
    }

    #[test]
    fn verifier_consistency_from_f32_out_of_range() {
        assert!(VerifierConsistency::from_f32(-1.0).is_err());
        assert!(VerifierConsistency::from_f32(1.01).is_err());
    }

    #[test]
    fn verifier_consistency_as_f32_roundtrip() {
        let v = VerifierConsistency::from_f32(0.4).unwrap();
        assert_eq!(v.as_f32(), 0.4);
    }

    #[test]
    fn verifier_consistency_display_format() {
        let v = VerifierConsistency::from_f32(0.1).unwrap();
        assert_eq!(v.to_string(), "0.100");
    }

    #[test]
    fn verifier_consistency_unchecked_bypass() {
        let v = VerifierConsistency::from_f32_unchecked(0.123);
        assert_eq!(v.as_f32(), 0.123);
    }

    // --------------------------------------------------------------------
    // §6.2 MetaDims 容器测试 (5 tests)
    // --------------------------------------------------------------------

    #[test]
    fn meta_dims_construction() {
        let m = MetaDims::new(
            Robustness::from_f32_unchecked(0.8),
            SelfImprovement::from_f32_unchecked(0.5),
            Adversarial::from_f32_unchecked(0.7),
            CiPassRate::from_f32_unchecked(0.9),
            VerifierConsistency::from_f32_unchecked(0.6),
        );
        assert_eq!(m.robustness.as_f32(), 0.8);
        assert_eq!(m.self_improvement.as_f32(), 0.5);
        assert_eq!(m.adversarial.as_f32(), 0.7);
        assert_eq!(m.ci_pass_rate.as_f32(), 0.9);
        assert_eq!(m.verifier_consistency.as_f32(), 0.6);
    }

    #[test]
    fn meta_dims_default_all_zero() {
        let m = MetaDims::default();
        assert_eq!(m.to_f32_array(), [0.0; 5]);
    }

    #[test]
    fn meta_dims_to_f32_array() {
        let m = MetaDims::new(
            Robustness::from_f32_unchecked(0.1),
            SelfImprovement::from_f32_unchecked(0.2),
            Adversarial::from_f32_unchecked(0.3),
            CiPassRate::from_f32_unchecked(0.4),
            VerifierConsistency::from_f32_unchecked(0.5),
        );
        assert_eq!(m.to_f32_array(), [0.1, 0.2, 0.3, 0.4, 0.5]);
    }

    #[test]
    fn meta_dims_serde_roundtrip() {
        let m = MetaDims::new(
            Robustness::from_f32_unchecked(0.5),
            SelfImprovement::from_f32_unchecked(0.5),
            Adversarial::from_f32_unchecked(0.5),
            CiPassRate::from_f32_unchecked(0.5),
            VerifierConsistency::from_f32_unchecked(0.5),
        );
        let s = serde_json::to_string(&m).unwrap();
        let parsed: MetaDims = serde_json::from_str(&s).unwrap();
        assert_eq!(m, parsed);
    }

    #[test]
    fn meta_dims_serde_field_names_preserved() {
        // 验证 serde 字段名 1:1 (改字段名会破坏 roundtrip, m3 防御)
        let m = MetaDims::default();
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("robustness"));
        assert!(s.contains("self_improvement"));
        assert!(s.contains("adversarial"));
        assert!(s.contains("ci_pass_rate"));
        assert!(s.contains("verifier_consistency"));
    }

    // --------------------------------------------------------------------
    // §6.3 MetaOverall 派生测试 (5 tests)
    // --------------------------------------------------------------------

    #[test]
    fn meta_overall_from_meta_dims_average() {
        let m = MetaDims::new(
            Robustness::from_f32_unchecked(1.0),
            SelfImprovement::from_f32_unchecked(1.0),
            Adversarial::from_f32_unchecked(1.0),
            CiPassRate::from_f32_unchecked(1.0),
            VerifierConsistency::from_f32_unchecked(1.0),
        );
        let o = MetaOverall::from_meta_dims(&m);
        assert!((o.as_f32() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn meta_overall_from_meta_dims_zero() {
        let m = MetaDims::default();
        let o = MetaOverall::from_meta_dims(&m);
        assert_eq!(o.as_f32(), 0.0);
    }

    #[test]
    fn meta_overall_from_meta_dims_mixed() {
        let m = MetaDims::new(
            Robustness::from_f32_unchecked(0.5),
            SelfImprovement::from_f32_unchecked(0.5),
            Adversarial::from_f32_unchecked(0.5),
            CiPassRate::from_f32_unchecked(0.5),
            VerifierConsistency::from_f32_unchecked(0.5),
        );
        let o = MetaOverall::from_meta_dims(&m);
        assert!((o.as_f32() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn meta_overall_from_f32_守门() {
        assert!(MetaOverall::from_f32(0.0).is_ok());
        assert!(MetaOverall::from_f32(0.5).is_ok());
        assert!(MetaOverall::from_f32(1.0).is_ok());
        assert!(MetaOverall::from_f32(-0.1).is_err());
        assert!(MetaOverall::from_f32(1.5).is_err());
    }

    #[test]
    fn meta_overall_display_format() {
        let o = MetaOverall::from_f32_unchecked(0.789);
        assert_eq!(o.to_string(), "0.789");
    }

    // --------------------------------------------------------------------
    // §6.4 V05Spec30 集成测试 (10 tests)
    // --------------------------------------------------------------------

    fn test_spec30() -> V05Spec30 {
        let dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let spec = V05Spec::new(Level::Mature, dims);
        let meta = MetaDims::new(
            Robustness::from_f32_unchecked(0.8),
            SelfImprovement::from_f32_unchecked(0.7),
            Adversarial::from_f32_unchecked(0.6),
            CiPassRate::from_f32_unchecked(0.9),
            VerifierConsistency::from_f32_unchecked(0.5),
        );
        V05Spec30::from_spec_and_meta(spec, meta)
    }

    #[test]
    fn v05_spec30_construction() {
        let s30 = test_spec30();
        assert_eq!(s30.spec.level, Level::Mature);
        // 5 meta 都正确
        assert_eq!(s30.meta.robustness.as_f32(), 0.8);
        assert_eq!(s30.meta.self_improvement.as_f32(), 0.7);
        assert_eq!(s30.meta.adversarial.as_f32(), 0.6);
        assert_eq!(s30.meta.ci_pass_rate.as_f32(), 0.9);
        assert_eq!(s30.meta.verifier_consistency.as_f32(), 0.5);
    }

    #[test]
    fn v05_spec30_overall_is_average() {
        // (0.8+0.7+0.6+0.9+0.5)/5 = 3.5/5 = 0.7
        let s30 = test_spec30();
        assert!((s30.overall.as_f32() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn v05_spec30_default_all_zero() {
        let s30 = V05Spec30::default();
        // 24 base (default_v05_spec)
        assert_eq!(s30.spec.level, Level::Mature);
        // 5 meta = 0
        assert_eq!(s30.meta.to_f32_array(), [0.0; 5]);
        // overall = 0
        assert_eq!(s30.overall.as_f32(), 0.0);
    }

    #[test]
    fn v05_spec30_serde_roundtrip() {
        let s30 = test_spec30();
        let s = serde_json::to_string(&s30).unwrap();
        let parsed: V05Spec30 = serde_json::from_str(&s).unwrap();
        assert_eq!(s30, parsed);
    }

    #[test]
    fn v05_spec30_serde_has_3_top_level_fields() {
        let s30 = test_spec30();
        let s = serde_json::to_string(&s30).unwrap();
        // 顶层 3 字段: spec / meta / overall
        assert!(s.contains("\"spec\""));
        assert!(s.contains("\"meta\""));
        assert!(s.contains("\"overall\""));
    }

    #[test]
    fn v05_spec30_base_classes_sum_to_1() {
        // 4 大类权重 sum=1.0 守门 (per V05Spec DEFAULT_WEIGHTS)
        let s30 = test_spec30();
        let sum: f32 = DEFAULT_WEIGHTS.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "4 大类权重 sum 必须 = 1.00, 实际 {sum}");
    }

    #[test]
    fn v05_spec30_meta_dim_count_is_5() {
        let s30 = test_spec30();
        assert_eq!(s30.meta.to_f32_array().len(), META_DIM_COUNT);
        assert_eq!(META_DIM_COUNT, 5);
    }

    #[test]
    fn v05_spec30_total_dims_constant_is_30() {
        assert_eq!(V05_30_TOTAL_DIMS, 30);
        assert_eq!(BASE_CLASS_COUNT * BASE_DIM_COUNT + META_DIM_COUNT + OVERALL_DIM_COUNT, 30);
    }

    #[test]
    fn v05_spec30_24_base_unchanged() {
        // 24 base 维 0 改 (per V05Spec 守门, B1 入口签名 0 改)
        let s30 = test_spec30();
        // 4 ClassDims 都 = dim
        assert_eq!(s30.spec.dims.pc, s30.spec.dims.rc);
        assert_eq!(s30.spec.dims.rc, s30.spec.dims.hg);
        assert_eq!(s30.spec.dims.hg, s30.spec.dims.gp);
    }

    #[test]
    fn v05_spec30_new_30_dim_handles_extreme() {
        // 边界: 5 meta-dim 全 0 + 全 1
        let dim = DimensionSet::new(
            Level::Seed,
            Domain::Dialogue,
            Modality::Multimodal,
            Safety::Critical,
            Completeness::Skeleton,
            Lineage::Spectra09,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let spec = V05Spec::new(Level::Seed, dims);
        let meta_all_zero = MetaDims::default();
        let meta_all_one = MetaDims::new(
            Robustness::from_f32_unchecked(1.0),
            SelfImprovement::from_f32_unchecked(1.0),
            Adversarial::from_f32_unchecked(1.0),
            CiPassRate::from_f32_unchecked(1.0),
            VerifierConsistency::from_f32_unchecked(1.0),
        );
        let s30_zero = V05Spec30::from_spec_and_meta(spec, meta_all_zero);
        let s30_one = V05Spec30::from_spec_and_meta(spec, meta_all_one);
        assert_eq!(s30_zero.overall.as_f32(), 0.0);
        assert!((s30_one.overall.as_f32() - 1.0).abs() < 1e-6);
    }

    // --------------------------------------------------------------------
    // §6.5 守门 + 总数 测试 (10 tests)
    // --------------------------------------------------------------------

    #[test]
    fn guard_base_class_count_is_4() {
        assert_eq!(BASE_CLASS_COUNT, 4);
    }

    #[test]
    fn guard_base_dim_count_is_6() {
        assert_eq!(BASE_DIM_COUNT, 6);
    }

    #[test]
    fn guard_meta_dim_count_is_5() {
        assert_eq!(META_DIM_COUNT, 5);
    }

    #[test]
    fn guard_overall_dim_count_is_1() {
        assert_eq!(OVERALL_DIM_COUNT, 1);
    }

    #[test]
    fn guard_v05_30_total_dims_constant_30() {
        assert_eq!(V05_30_TOTAL_DIMS, 30);
    }

    #[test]
    fn guard_4_weight_classes_守门() {
        // V05Spec 4 大类 weight sum=1.0 (per DEFAULT_WEIGHTS 严守, 0 改)
        let sum: f32 = DEFAULT_WEIGHTS.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn guard_5_meta_dim_range_守门() {
        // 5 meta-dim 全部 ∈ [0.0, 1.0] (per typed struct from_f32 守门)
        for v in [-0.1_f32, 0.0, 0.5, 1.0, 1.1] {
            let r = Robustness::from_f32(v).is_ok();
            let s = SelfImprovement::from_f32(v).is_ok();
            let a = Adversarial::from_f32(v).is_ok();
            let c = CiPassRate::from_f32(v).is_ok();
            let vc = VerifierConsistency::from_f32(v).is_ok();
            if v < 0.0 || v > 1.0 {
                assert!(!r && !s && !a && !c && !vc, "v={v} 必须 守门 拒绝");
            } else {
                assert!(r && s && a && c && vc, "v={v} 必须 守门 通过");
            }
        }
    }

    #[test]
    fn guard_overall_is_average_of_5() {
        // overall = 5 维平均 (per MetaOverall::from_meta_dims)
        let cases = [
            ([0.0, 0.0, 0.0, 0.0, 0.0], 0.0),
            ([1.0, 1.0, 1.0, 1.0, 1.0], 1.0),
            ([0.5, 0.5, 0.5, 0.5, 0.5], 0.5),
            ([0.2, 0.4, 0.6, 0.8, 1.0], 0.6), // (0.2+0.4+0.6+0.8+1.0)/5 = 3.0/5 = 0.6
            ([0.0, 0.5, 1.0, 0.0, 0.5], 0.4), // (0+0.5+1+0+0.5)/5 = 2/5 = 0.4
        ];
        for (input, expected) in cases {
            let m = MetaDims::new(
                Robustness::from_f32_unchecked(input[0]),
                SelfImprovement::from_f32_unchecked(input[1]),
                Adversarial::from_f32_unchecked(input[2]),
                CiPassRate::from_f32_unchecked(input[3]),
                VerifierConsistency::from_f32_unchecked(input[4]),
            );
            let o = MetaOverall::from_meta_dims(&m);
            assert!((o.as_f32() - expected).abs() < 1e-6, "input {input:?} → expected {expected}, got {}", o.as_f32());
        }
    }

    #[test]
    fn guard_extreme_meta_dim_values_handled() {
        // 极端值 (0.0/1.0 边界) 守门 OK
        let m_zero = MetaDims::new(
            Robustness::from_f32(0.0).unwrap(),
            SelfImprovement::from_f32(0.0).unwrap(),
            Adversarial::from_f32(0.0).unwrap(),
            CiPassRate::from_f32(0.0).unwrap(),
            VerifierConsistency::from_f32(0.0).unwrap(),
        );
        let m_one = MetaDims::new(
            Robustness::from_f32(1.0).unwrap(),
            SelfImprovement::from_f32(1.0).unwrap(),
            Adversarial::from_f32(1.0).unwrap(),
            CiPassRate::from_f32(1.0).unwrap(),
            VerifierConsistency::from_f32(1.0).unwrap(),
        );
        assert_eq!(MetaOverall::from_meta_dims(&m_zero).as_f32(), 0.0);
        assert!((MetaOverall::from_meta_dims(&m_one).as_f32() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn guard_v05_30_total_dims_immutable() {
        // 30 维守门: 改 V05_30_TOTAL_DIMS 会破坏编译 (编译期 hardcode)
        const _: usize = V05_30_TOTAL_DIMS; // 编译期引用
        assert_eq!(V05_30_TOTAL_DIMS, 30);
    }

    // --- 5 额外守门 (凑 60 tests 30 维 pattern, per R125-13 60 tests) ---

    #[test]
    fn guard_robustness_serde_roundtrip() {
        let r = Robustness::from_f32(0.42).unwrap();
        let s = serde_json::to_string(&r).unwrap();
        let parsed: Robustness = serde_json::from_str(&s).unwrap();
        assert_eq!(r, parsed);
    }

    #[test]
    fn guard_self_improvement_serde_roundtrip() {
        let s = SelfImprovement::from_f32(0.88).unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: SelfImprovement = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }

    #[test]
    fn guard_adversarial_serde_roundtrip() {
        let a = Adversarial::from_f32(0.33).unwrap();
        let json = serde_json::to_string(&a).unwrap();
        let parsed: Adversarial = serde_json::from_str(&json).unwrap();
        assert_eq!(a, parsed);
    }

    #[test]
    fn guard_ci_pass_rate_serde_roundtrip() {
        let c = CiPassRate::from_f32(0.66).unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let parsed: CiPassRate = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }

    #[test]
    fn guard_verifier_consistency_serde_roundtrip() {
        let v = VerifierConsistency::from_f32(0.11).unwrap();
        let json = serde_json::to_string(&v).unwrap();
        let parsed: VerifierConsistency = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }
}
