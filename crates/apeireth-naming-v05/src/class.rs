//! # class — 4 大类 (PC/RC/HG/GP) + V05Spec 24 维主结构
//!
//! ## 4 大类 (per V0.5 v2 提议, 24 维)
//!
//! | 类 | 全称 | 估权重 | 含义 |
//! |---|---|------|---|
//! | **PC** | Positive Capability | 0.40 | 正向能力 (能做什么) |
//! | **RC** | Risk Constraint | 0.30 | 风险约束 (不能做什么) |
//! | **HG** | Honesty Gap | 0.15 | 诚实标缺 (不知道什么) |
//! | **GP** | Growth Phase | 0.15 | 成长阶段 (现在到哪) |
//!
//! ## V05Spec 24 维
//!
//! ```text
//! 1 个 V05Spec = 4 ClassDims (PC/RC/HG/GP) = 24 dim
//! 1 个 ClassDim  = 1 DimensionSet (6 dim: level/domain/modality/safety/completeness/lineage)
//! 1 个 DimensionSet = 6 维度 (per dimension.rs)
//! ```
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use serde::{Deserialize, Serialize};

use crate::dimension::DimensionSet;

// ============================================================================
// §1 4 大类 (Class enum, 编译期 hardcode)
// ============================================================================

/// 4 大类 (1:1 V0.5 v2 提议, sum=1.00 守门).
///
/// ## 权重
/// - PC: 0.40
/// - RC: 0.30
/// - HG: 0.15
/// - GP: 0.15
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Class {
    /// Positive Capability (正向能力) — 估 0.40
    #[serde(rename = "PC")]
    Pc,
    /// Risk Constraint (风险约束) — 估 0.30
    #[serde(rename = "RC")]
    Rc,
    /// Honesty Gap (诚实标缺) — 估 0.15
    #[serde(rename = "HG")]
    Hg,
    /// Growth Phase (成长阶段) — 估 0.15
    #[serde(rename = "GP")]
    Gp,
}

impl Class {
    /// 字符串 → Class.
    pub fn parse(s: &str) -> Result<Self, crate::error::NamingError> {
        match s {
            "PC" => Ok(Class::Pc),
            "RC" => Ok(Class::Rc),
            "HG" => Ok(Class::Hg),
            "GP" => Ok(Class::Gp),
            other => Err(crate::error::NamingError::InvalidClass(other.to_string())),
        }
    }

    /// Class → 字符串.
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Pc => "PC",
            Class::Rc => "RC",
            Class::Hg => "HG",
            Class::Gp => "GP",
        }
    }

    /// 全称 (per 4 大类 1:1).
    pub fn full_name(self) -> &'static str {
        match self {
            Class::Pc => "Positive Capability",
            Class::Rc => "Risk Constraint",
            Class::Hg => "Honesty Gap",
            Class::Gp => "Growth Phase",
        }
    }

    /// 估权重 (per 4 大类 1:1, sum=1.00 守门).
    pub fn weight(self) -> f32 {
        match self {
            Class::Pc => 0.40,
            Class::Rc => 0.30,
            Class::Hg => 0.15,
            Class::Gp => 0.15,
        }
    }

    /// 4 类全表 (K-1 强校验).
    pub const ALL: &'static [Class] = &[Class::Pc, Class::Rc, Class::Hg, Class::Gp];
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 ClassDim — 1 大类的 6 维结构 (PC/RC/HG/GP 各 1 个)
// ============================================================================

/// 1 大类的 6 维结构 (per 4 大类各 1 个, 共 4 个 ClassDim = 24 维).
///
/// 字段命名: `pc` / `rc` / `hg` / `gp` (字段名 = 类别小写, 1:1 类名).
/// 类型: 全是 `DimensionSet` (6 维), 4 × 6 = 24 维.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDims {
    /// PC 6 维 (Positive Capability): level_pc / domain_pc / modality_pc / safety_pc / completeness_pc / lineage_pc
    pub pc: DimensionSet,
    /// RC 6 维 (Risk Constraint)
    pub rc: DimensionSet,
    /// HG 6 维 (Honesty Gap)
    pub hg: DimensionSet,
    /// GP 6 维 (Growth Phase)
    pub gp: DimensionSet,
}

impl ClassDims {
    /// 构造新 24 维.
    pub const fn new(pc: DimensionSet, rc: DimensionSet, hg: DimensionSet, gp: DimensionSet) -> Self {
        Self { pc, rc, hg, gp }
    }

    /// 按类取 DimensionSet.
    pub fn get(&self, class: Class) -> DimensionSet {
        match class {
            Class::Pc => self.pc,
            Class::Rc => self.rc,
            Class::Hg => self.hg,
            Class::Gp => self.gp,
        }
    }

    /// 按类设置 DimensionSet (返回新 ClassDims, immutable 设计).
    pub fn with(self, class: Class, dim: DimensionSet) -> Self {
        match class {
            Class::Pc => Self { pc: dim, ..self },
            Class::Rc => Self { rc: dim, ..self },
            Class::Hg => Self { hg: dim, ..self },
            Class::Gp => Self { gp: dim, ..self },
        }
    }
}

// ============================================================================
// §3 V05Spec — V0.5 完整 24 维规范
// ============================================================================

/// V0.5 完整 24 维规范 (level + 4 ClassDims).
///
/// ## 结构
/// - `level`: 全局 level (0-9), 4 大类共享同一个 (1 个 level 字段)
/// - `dims`: 4 大类 × 6 维 = 24 维 (`ClassDims`)
///
/// ## 命名格式
/// `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
///
/// 例: `apeireth:5.PC.code.text.high.complete.apeireth-1.0`
///     含义: level=5 / class=PC / domain=code / modality=text / safety=high / completeness=complete / lineage=apeireth-1.0
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct V05Spec {
    /// 全局 level (0-9, 4 大类共享)
    pub level: crate::dimension::Level,
    /// 4 大类 × 6 维 = 24 维 (`ClassDims`)
    pub dims: ClassDims,
}

impl V05Spec {
    /// 构造新 V05Spec.
    pub const fn new(level: crate::dimension::Level, dims: ClassDims) -> Self {
        Self { level, dims }
    }
}

// ============================================================================
// §4 in-module 测试 (4 大类 1:1 + 24 维 = 6 × 4 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::{
        Completeness, Domain, Level, Lineage, Modality, Safety, CLASS_COUNT, DIMENSION_COUNT,
        V05_TOTAL_DIMS,
    };

    /// 守门 #1: 4 大类 1:1.
    #[test]
    fn class_all_count_is_four() {
        assert_eq!(Class::ALL.len(), 4);
        assert_eq!(CLASS_COUNT, 4);
    }

    /// 守门 #2: 4 大类 hardcode 字符串.
    #[test]
    fn class_as_str_hardcoded() {
        assert_eq!(Class::Pc.as_str(), "PC");
        assert_eq!(Class::Rc.as_str(), "RC");
        assert_eq!(Class::Hg.as_str(), "HG");
        assert_eq!(Class::Gp.as_str(), "GP");
    }

    /// 守门 #3: 4 大类全称.
    #[test]
    fn class_full_names() {
        assert_eq!(Class::Pc.full_name(), "Positive Capability");
        assert_eq!(Class::Rc.full_name(), "Risk Constraint");
        assert_eq!(Class::Hg.full_name(), "Honesty Gap");
        assert_eq!(Class::Gp.full_name(), "Growth Phase");
    }

    /// 守门 #4: 4 大类权重 hardcode + sum=1.00.
    #[test]
    fn class_weights_sum_to_1() {
        let sum: f32 = Class::ALL.iter().map(|c| c.weight()).sum();
        assert!((sum - 1.0).abs() < 1e-6, "4 大类权重 sum 必须 = 1.00, 实际 {sum}");
    }

    /// 守门 #5: 4 大类权重具体值.
    #[test]
    fn class_weight_values() {
        assert_eq!(Class::Pc.weight(), 0.40);
        assert_eq!(Class::Rc.weight(), 0.30);
        assert_eq!(Class::Hg.weight(), 0.15);
        assert_eq!(Class::Gp.weight(), 0.15);
    }

    /// 守门 #6: Class parse/display roundtrip.
    #[test]
    fn class_parse_display_roundtrip() {
        for c in Class::ALL {
            let s = c.to_string();
            let parsed = Class::parse(&s).unwrap();
            assert_eq!(*c, parsed);
        }
    }

    /// 守门 #7: Class parse 拒绝未知.
    #[test]
    fn class_parse_invalid_rejected() {
        let err = Class::parse("XX").unwrap_err();
        assert!(matches!(err, crate::error::NamingError::InvalidClass(_)));
    }

    /// 守门 #8: ClassDims get/with.
    #[test]
    fn class_dims_get_with() {
        let pc_dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Production,
            Lineage::Apeireth10,
        );
        let rc_dim = DimensionSet::new(
            Level::Expert1,
            Domain::Tool,
            Modality::Multimodal,
            Safety::Critical,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let dims = ClassDims::new(pc_dim, rc_dim, pc_dim, pc_dim);

        // get
        assert_eq!(dims.get(Class::Pc), pc_dim);
        assert_eq!(dims.get(Class::Rc), rc_dim);
        assert_eq!(dims.get(Class::Hg), pc_dim);
        assert_eq!(dims.get(Class::Gp), pc_dim);

        // with
        let dims2 = dims.with(Class::Hg, rc_dim);
        assert_eq!(dims2.hg, rc_dim);
        assert_eq!(dims2.pc, pc_dim); // 未变
    }

    /// 守门 #9: V05Spec 构造.
    #[test]
    fn v05_spec_construction() {
        let dim = DimensionSet::new(
            Level::Maturing1,
            Domain::Code,
            Modality::Text,
            Safety::Medium,
            Completeness::Partial,
            Lineage::Apeireth014,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let spec = V05Spec::new(Level::Maturing1, dims);
        assert_eq!(spec.level, Level::Maturing1);
        assert_eq!(spec.dims, dims);
    }

    /// 守门 #10: 24 维 守门.
    #[test]
    fn v05_total_dims_is_24() {
        assert_eq!(V05_TOTAL_DIMS, 24);
        assert_eq!(DIMENSION_COUNT * CLASS_COUNT, 24);
    }

    /// 守门 #11: ClassDims serde roundtrip.
    #[test]
    fn class_dims_serde_roundtrip() {
        let dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let s = serde_json::to_string(&dims).unwrap();
        let parsed: ClassDims = serde_json::from_str(&s).unwrap();
        assert_eq!(dims, parsed);
    }

    /// 守门 #12: V05Spec serde roundtrip.
    #[test]
    fn v05_spec_serde_roundtrip() {
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
        let s = serde_json::to_string(&spec).unwrap();
        let parsed: V05Spec = serde_json::from_str(&s).unwrap();
        assert_eq!(spec, parsed);
    }
}
