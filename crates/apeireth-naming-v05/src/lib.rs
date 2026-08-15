//! # apeireth-naming-v05
//!
//! V0.5 命名规范 (4 类 × 6 维 = 24 维) — R20 阶段 4 估补.
//!
//! ## 背景
//!
//! V0.5 命名规范是 ASI 测量公式的命名空间 (per v1077 V0.5 17 维 LOCKED +
//! 提议 v2 24 维, docs/architecture-v4-1-living-intelligence-update §13).
//! 本 crate 把 V0.5 24 维 (PC/RC/HG/GP × level/domain/modality/safety/completeness/lineage)
//! 完整落到 Rust 强类型 enum + 守门 + encode/decode.
//!
//! ## 4 大类 × 6 维度 = 24 维
//!
//! ```text
//! 4 大类权重 (sum=1.00 守门):
//!   PC (Positive Capability)   0.40
//!   RC (Risk Constraint)       0.30
//!   HG (Honesty Gap)           0.15
//!   GP (Growth Phase)          0.15
//!                          -----
//!                          1.00 ✓
//!
//! 6 维度 (每类 6 维):
//! 1. level        0-9 (0=seed, 9=mature)
//! 2. domain       code/dialogue/vision/audio/tool/reasoning
//! 3. modality     text/image/audio/video/multimodal
//! 4. safety       low/medium/high/critical
//! 5. completeness skeleton/partial/complete/production
//! 6. lineage      spectrai-0.9/apeireth-0.14/apeireth-1.0/apeireth-2.0
//! ```
//!
//! ## 命名格式
//!
//! `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
//!
//! 例:
//! - `apeireth:5.PC.code.text.high.complete.apeireth-1.0` (level=5, class=PC, code/text, lineage=apeireth-1.0)
//! - `apeireth:9.GP.dialogue.audio.low.skeleton.spectrai-0.9` (level=9, class=GP, dialogue/audio, lineage=spectrai-0.9)
//!
//! ## 模块结构
//!
//! - [`class`] — 4 大类 (PC/RC/HG/GP) + `ClassDims` 24 维结构 + `V05Spec` 主结构
//! - [`dimension`] — 6 维度 (Level/Domain/Modality/Safety/Completeness/Lineage) + `DimensionSet`
//! - [`encode`] — 24 维 → 字符串 (4 行, 1 行 1 大类)
//! - [`decode`] — 字符串 → 24 维 (regex 解析)
//! - [`validate`] — 24 维合法性 + sum=1.00 守门 + roundtrip
//! - [`error`] — 11 个 `NamingError` variant (10 原始 + 1 R126 扩展)
//! - [`extension`] — **R126 P1-4**: V0.5 → V0.5.30 扩展 (5 new meta-dim + 1 overall = 30 dim, 借鉴 langgraph)
//! - [`sum_guard`] — 4 大类权重 sum=1.00 守门 (核心)
//!
//! ## 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 主 22:33 北极星导向** — V0.5 命名服务 ASI 北极星 (24 维 → 更精准测量)
//! - **S-2 主 17:43 实事求是** — 不重写 v1077 17 维 LOCKED, 24 维是提议 v2 (per v4.1 §13)
//! - **O-5 主 17:58 不假装** — 24 维 编译期 hardcode enum, 不假装"已对齐 V0.5"
//! - **O-2 主 19:33 走在前人肩上** — 借 v1077 + v4.1 §13 + R17 命名 v12 规范
//! - **O-3 主 23:44 干到底** — 24 维 立即落, 守门硬约束 (sum=1.00 容差 0.001)
//! - **O-4 主 00:56 任何人都能接手** — 7 模块 + 24 维 struct + sum_guard + 11 error variant 全文档化 (R126 P1-4 扩展 10 → 11)
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! 1. **阶段 1+2+3 LOCKED** — 不动
//! 2. **v2 / v4 / v4.1 LOCKED** — 不动
//! 3. **阶段 4 主文档 LOCKED** (6ca80776) — 不动
//! 4. **阶段 5 施工文档 LOCKED** (631 行) — 不动
//! 5. **v6 修正** (4 重守门 + 权限发放 + E 层修改路径) — 不动
//! 6. **R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) — 不动
//! 7. **v1 → v5 历史链** — 不删除
//! 8. **v1077 17 维 LOCKED** (V0.5 v1) — 不动, 24 维是 v2 提议叠加
//!
//! ## 状态
//!
//! ⚠️ skeleton (R20 阶段 4 估补, per v09021-rust-translation-blueprint §2.4 V0.5 命名规范).
//!
//! ## 用法示例
//!
//! ```ignore
//! use apeireth_naming_v05::{
//!     encode_v05, decode_v05, validate_v05,
//!     V05Spec, ClassDims, DimensionSet, Level, Domain, Modality, Safety, Completeness, Lineage,
//!     Class, DEFAULT_WEIGHTS, check_sum_equals_1,
//! };
//!
//! // 1) 构造 24 维
//! let dim = DimensionSet::new(
//!     Level::Mature, Domain::Code, Modality::Text,
//!     Safety::High, Completeness::Complete, Lineage::Apeireth10,
//! );
//! let dims = ClassDims::new(dim, dim, dim, dim);
//! let spec = V05Spec::new(Level::Mature, dims);
//!
//! // 2) 守门
//! check_sum_equals_1(&DEFAULT_WEIGHTS)?;
//! validate_v05(&spec)?;
//!
//! // 3) encode → decode roundtrip
//! let s = encode_v05(&spec)?;
//! let parsed = decode_v05(&s)?;
//! assert_eq!(spec, parsed);
//! ```

#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// 模块声明
// ============================================================================

pub mod class;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod decode;
pub mod dimension;
pub mod encode;
pub mod error;
pub mod extension;
pub mod sum_guard;
pub mod validate;

// ============================================================================
// Re-export (主入口便捷)
// ============================================================================

pub use class::{Class, ClassDims, V05Spec};
pub use dimension::{
    Completeness, DimensionSet, Domain, Level, Lineage, Modality, Safety, CLASS_COUNT,
    DIMENSION_COUNT, V05_TOTAL_DIMS,
};
pub use encode::{encode_v05, encode_v05_class, encode_v05_lines, V05_PREFIX, V05_SEGMENT_COUNT, V05_SEP};
pub use decode::{decode_v05, decode_v05_class, V05_LINE_REGEX};
pub use error::{NamingError, NamingResult, NAMING_ERROR_VARIANT_COUNT};
pub use sum_guard::{
    check_sum_equals_1, check_sum_equals_1_default, ClassWeights, DEFAULT_WEIGHTS,
    SUM_GUARD_TOLERANCE,
};
pub use validate::{validate_roundtrip, validate_v05, validate_v05_structure, validate_v05_with_weights};
pub use extension::{
    Adversarial, CiPassRate, MetaDims, MetaOverall, Robustness, SelfImprovement,
    V05Spec30, VerifierConsistency, BASE_CLASS_COUNT, BASE_DIM_COUNT, META_DIM_COUNT,
    OVERALL_DIM_COUNT, V05_30_TOTAL_DIMS,
};

// ============================================================================
// Crate-level 常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 4 大类 1:1 计数.
pub const CRATE_CLASS_COUNT: usize = 4;

/// 6 维度 1:1 计数.
pub const CRATE_DIMENSION_COUNT: usize = 6;

/// 24 维 = 4 × 6 守门.
pub const CRATE_V05_TOTAL_DIMS: usize = CRATE_CLASS_COUNT * CRATE_DIMENSION_COUNT;

/// 命名空间前缀.
pub const CRATE_PREFIX: &str = "apeireth:";

/// 段间分隔符.
pub const CRATE_SEP: char = '.';

/// 1 段命名总段数 (1 prefix + 1 level + 1 class + 5 维 = 8 段).
pub const CRATE_SEGMENT_COUNT: usize = 8;

// ============================================================================
// Crate-level 类型别名
// ============================================================================

/// 4 大类 DimSet tuple (per 24 维).
pub type V05DimsTuple = (DimensionSet, DimensionSet, DimensionSet, DimensionSet);

// ============================================================================
// 24 维 完整 24 项枚举 (per 4 类 × 6 维)
// ============================================================================

/// 24 维 单维 ID (4 类 × 6 维, 编译期 hardcode 24).
///
/// 用于按 ID 索引 24 维之一. m3 防御: 改这个 enum 数量会立刻破坏编译.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum V05DimId {
    /// PC 维度 1: level
    PcLevel,
    /// PC 维度 2: domain
    PcDomain,
    /// PC 维度 3: modality
    PcModality,
    /// PC 维度 4: safety
    PcSafety,
    /// PC 维度 5: completeness
    PcCompleteness,
    /// PC 维度 6: lineage
    PcLineage,

    /// RC 维度 1: level
    RcLevel,
    /// RC 维度 2: domain
    RcDomain,
    /// RC 维度 3: modality
    RcModality,
    /// RC 维度 4: safety
    RcSafety,
    /// RC 维度 5: completeness
    RcCompleteness,
    /// RC 维度 6: lineage
    RcLineage,

    /// HG 维度 1: level
    HgLevel,
    /// HG 维度 2: domain
    HgDomain,
    /// HG 维度 3: modality
    HgModality,
    /// HG 维度 4: safety
    HgSafety,
    /// HG 维度 5: completeness
    HgCompleteness,
    /// HG 维度 6: lineage
    HgLineage,

    /// GP 维度 1: level
    GpLevel,
    /// GP 维度 2: domain
    GpDomain,
    /// GP 维度 3: modality
    GpModality,
    /// GP 维度 4: safety
    GpSafety,
    /// GP 维度 5: completeness
    GpCompleteness,
    /// GP 维度 6: lineage
    GpLineage,
}

impl V05DimId {
    /// 24 维全表.
    pub const ALL: &'static [V05DimId] = &[
        // PC
        V05DimId::PcLevel,
        V05DimId::PcDomain,
        V05DimId::PcModality,
        V05DimId::PcSafety,
        V05DimId::PcCompleteness,
        V05DimId::PcLineage,
        // RC
        V05DimId::RcLevel,
        V05DimId::RcDomain,
        V05DimId::RcModality,
        V05DimId::RcSafety,
        V05DimId::RcCompleteness,
        V05DimId::RcLineage,
        // HG
        V05DimId::HgLevel,
        V05DimId::HgDomain,
        V05DimId::HgModality,
        V05DimId::HgSafety,
        V05DimId::HgCompleteness,
        V05DimId::HgLineage,
        // GP
        V05DimId::GpLevel,
        V05DimId::GpDomain,
        V05DimId::GpModality,
        V05DimId::GpSafety,
        V05DimId::GpCompleteness,
        V05DimId::GpLineage,
    ];

    /// 24 维所属类.
    pub fn class(self) -> Class {
        match self {
            V05DimId::PcLevel | V05DimId::PcDomain | V05DimId::PcModality
            | V05DimId::PcSafety | V05DimId::PcCompleteness | V05DimId::PcLineage => Class::Pc,
            V05DimId::RcLevel | V05DimId::RcDomain | V05DimId::RcModality
            | V05DimId::RcSafety | V05DimId::RcCompleteness | V05DimId::RcLineage => Class::Rc,
            V05DimId::HgLevel | V05DimId::HgDomain | V05DimId::HgModality
            | V05DimId::HgSafety | V05DimId::HgCompleteness | V05DimId::HgLineage => Class::Hg,
            V05DimId::GpLevel | V05DimId::GpDomain | V05DimId::GpModality
            | V05DimId::GpSafety | V05DimId::GpCompleteness | V05DimId::GpLineage => Class::Gp,
        }
    }

    /// 24 维在该类内的偏移 (0-5).
    pub fn offset(self) -> usize {
        match self {
            V05DimId::PcLevel => 0,
            V05DimId::PcDomain => 1,
            V05DimId::PcModality => 2,
            V05DimId::PcSafety => 3,
            V05DimId::PcCompleteness => 4,
            V05DimId::PcLineage => 5,
            V05DimId::RcLevel => 0,
            V05DimId::RcDomain => 1,
            V05DimId::RcModality => 2,
            V05DimId::RcSafety => 3,
            V05DimId::RcCompleteness => 4,
            V05DimId::RcLineage => 5,
            V05DimId::HgLevel => 0,
            V05DimId::HgDomain => 1,
            V05DimId::HgModality => 2,
            V05DimId::HgSafety => 3,
            V05DimId::HgCompleteness => 4,
            V05DimId::HgLineage => 5,
            V05DimId::GpLevel => 0,
            V05DimId::GpDomain => 1,
            V05DimId::GpModality => 2,
            V05DimId::GpSafety => 3,
            V05DimId::GpCompleteness => 4,
            V05DimId::GpLineage => 5,
        }
    }

    /// 24 维名 (e.g. "PC.level").
    pub fn name(self) -> &'static str {
        match self {
            V05DimId::PcLevel => "PC.level",
            V05DimId::PcDomain => "PC.domain",
            V05DimId::PcModality => "PC.modality",
            V05DimId::PcSafety => "PC.safety",
            V05DimId::PcCompleteness => "PC.completeness",
            V05DimId::PcLineage => "PC.lineage",
            V05DimId::RcLevel => "RC.level",
            V05DimId::RcDomain => "RC.domain",
            V05DimId::RcModality => "RC.modality",
            V05DimId::RcSafety => "RC.safety",
            V05DimId::RcCompleteness => "RC.completeness",
            V05DimId::RcLineage => "RC.lineage",
            V05DimId::HgLevel => "HG.level",
            V05DimId::HgDomain => "HG.domain",
            V05DimId::HgModality => "HG.modality",
            V05DimId::HgSafety => "HG.safety",
            V05DimId::HgCompleteness => "HG.completeness",
            V05DimId::HgLineage => "HG.lineage",
            V05DimId::GpLevel => "GP.level",
            V05DimId::GpDomain => "GP.domain",
            V05DimId::GpModality => "GP.modality",
            V05DimId::GpSafety => "GP.safety",
            V05DimId::GpCompleteness => "GP.completeness",
            V05DimId::GpLineage => "GP.lineage",
        }
    }
}

// ============================================================================
// 24 维 V05Spec 便捷访问器 (per 24 维, 编译期 24 个 fn)
// ============================================================================

impl V05Spec {
    /// PC 6 维 get.
    pub fn pc_dim(&self) -> DimensionSet {
        self.dims.pc
    }

    /// PC level get.
    pub fn pc_level(&self) -> Level {
        self.dims.pc.level
    }

    /// PC domain get.
    pub fn pc_domain(&self) -> Domain {
        self.dims.pc.domain
    }

    /// PC modality get.
    pub fn pc_modality(&self) -> Modality {
        self.dims.pc.modality
    }

    /// PC safety get.
    pub fn pc_safety(&self) -> Safety {
        self.dims.pc.safety
    }

    /// PC completeness get.
    pub fn pc_completeness(&self) -> Completeness {
        self.dims.pc.completeness
    }

    /// PC lineage get.
    pub fn pc_lineage(&self) -> Lineage {
        self.dims.pc.lineage
    }

    /// RC level get.
    pub fn rc_level(&self) -> Level {
        self.dims.rc.level
    }

    /// RC domain get.
    pub fn rc_domain(&self) -> Domain {
        self.dims.rc.domain
    }

    /// RC modality get.
    pub fn rc_modality(&self) -> Modality {
        self.dims.rc.modality
    }

    /// RC safety get.
    pub fn rc_safety(&self) -> Safety {
        self.dims.rc.safety
    }

    /// RC completeness get.
    pub fn rc_completeness(&self) -> Completeness {
        self.dims.rc.completeness
    }

    /// RC lineage get.
    pub fn rc_lineage(&self) -> Lineage {
        self.dims.rc.lineage
    }

    /// HG level get.
    pub fn hg_level(&self) -> Level {
        self.dims.hg.level
    }

    /// HG domain get.
    pub fn hg_domain(&self) -> Domain {
        self.dims.hg.domain
    }

    /// HG modality get.
    pub fn hg_modality(&self) -> Modality {
        self.dims.hg.modality
    }

    /// HG safety get.
    pub fn hg_safety(&self) -> Safety {
        self.dims.hg.safety
    }

    /// HG completeness get.
    pub fn hg_completeness(&self) -> Completeness {
        self.dims.hg.completeness
    }

    /// HG lineage get.
    pub fn hg_lineage(&self) -> Lineage {
        self.dims.hg.lineage
    }

    /// GP level get.
    pub fn gp_level(&self) -> Level {
        self.dims.gp.level
    }

    /// GP domain get.
    pub fn gp_domain(&self) -> Domain {
        self.dims.gp.domain
    }

    /// GP modality get.
    pub fn gp_modality(&self) -> Modality {
        self.dims.gp.modality
    }

    /// GP safety get.
    pub fn gp_safety(&self) -> Safety {
        self.dims.gp.safety
    }

    /// GP completeness get.
    pub fn gp_completeness(&self) -> Completeness {
        self.dims.gp.completeness
    }

    /// GP lineage get.
    pub fn gp_lineage(&self) -> Lineage {
        self.dims.gp.lineage
    }

    /// 按 V05DimId 取 24 维的 level 字段.
    pub fn get_level(&self, id: V05DimId) -> Level {
        match id {
            V05DimId::PcLevel => self.pc_level(),
            V05DimId::RcLevel => self.rc_level(),
            V05DimId::HgLevel => self.hg_level(),
            V05DimId::GpLevel => self.gp_level(),
            _ => self.pc_level(), // 非 level 维度返 PC level (类型需要)
        }
    }

    /// 按 V05DimId 取 24 维的 domain 字段.
    pub fn get_domain(&self, id: V05DimId) -> Domain {
        match id {
            V05DimId::PcDomain => self.pc_domain(),
            V05DimId::RcDomain => self.rc_domain(),
            V05DimId::HgDomain => self.hg_domain(),
            V05DimId::GpDomain => self.gp_domain(),
            _ => self.pc_domain(),
        }
    }

    /// 按 V05DimId 取 24 维的 modality 字段.
    pub fn get_modality(&self, id: V05DimId) -> Modality {
        match id {
            V05DimId::PcModality => self.pc_modality(),
            V05DimId::RcModality => self.rc_modality(),
            V05DimId::HgModality => self.hg_modality(),
            V05DimId::GpModality => self.gp_modality(),
            _ => self.pc_modality(),
        }
    }

    /// 按 V05DimId 取 24 维的 safety 字段.
    pub fn get_safety(&self, id: V05DimId) -> Safety {
        match id {
            V05DimId::PcSafety => self.pc_safety(),
            V05DimId::RcSafety => self.rc_safety(),
            V05DimId::HgSafety => self.hg_safety(),
            V05DimId::GpSafety => self.gp_safety(),
            _ => self.pc_safety(),
        }
    }

    /// 按 V05DimId 取 24 维的 completeness 字段.
    pub fn get_completeness(&self, id: V05DimId) -> Completeness {
        match id {
            V05DimId::PcCompleteness => self.pc_completeness(),
            V05DimId::RcCompleteness => self.rc_completeness(),
            V05DimId::HgCompleteness => self.hg_completeness(),
            V05DimId::GpCompleteness => self.gp_completeness(),
            _ => self.pc_completeness(),
        }
    }

    /// 按 V05DimId 取 24 维的 lineage 字段.
    pub fn get_lineage(&self, id: V05DimId) -> Lineage {
        match id {
            V05DimId::PcLineage => self.pc_lineage(),
            V05DimId::RcLineage => self.rc_lineage(),
            V05DimId::HgLineage => self.hg_lineage(),
            V05DimId::GpLineage => self.gp_lineage(),
            _ => self.pc_lineage(),
        }
    }
}

// ============================================================================
// 默认 24 维 spec (per 4 大类同 dim, level=9 mature)
// ============================================================================

/// 默认 24 维 spec (level=9 mature, 全 PC/RC/HG/GP 同 dim, code/text/high/complete/apeireth-1.0).
pub fn default_v05_spec() -> V05Spec {
    let dim = DimensionSet::new(
        Level::Mature,
        Domain::Code,
        Modality::Text,
        Safety::High,
        Completeness::Complete,
        Lineage::Apeireth10,
    );
    let dims = ClassDims::new(dim, dim, dim, dim);
    V05Spec::new(Level::Mature, dims)
}

// ============================================================================
// 24 维 速查表 (PC/RC/HG/GP × level/domain/modality/safety/completeness/lineage)
// ============================================================================
//
// 4 大类 (sum=1.00 守门):
// ┌────┬────────────────────┬────────┬─────────────────┐
// │ ID │ Class              │ Weight │ 维度集 (6 维)   │
// ├────┼────────────────────┼────────┼─────────────────┤
// │  0 │ PC (Positive Cap)  │  0.40  │ 24 维 (lvl/...) │
// │  1 │ RC (Risk Const)    │  0.30  │ 24 维 (lvl/...) │
// │  2 │ HG (Honesty Gap)   │  0.15  │ 24 维 (lvl/...) │
// │  3 │ GP (Growth Phase)  │  0.15  │ 24 维 (lvl/...) │
// └────┴────────────────────┴────────┴─────────────────┘
//
// 6 维度 (每类共享同 enum):
// ┌────┬──────────────┬────────────────────────────────────────────┐
// │ #  │ Dimension    │ 取值                                       │
// ├────┼──────────────┼────────────────────────────────────────────┤
// │  1 │ level        │ 0..9 (Seed/Maturing/Expert/Mature/...)    │
// │  2 │ domain       │ code/dialogue/vision/audio/tool/reasoning  │
// │  3 │ modality     │ text/image/audio/video/multimodal          │
// │  4 │ safety       │ low/medium/high/critical                   │
// │  5 │ completeness │ skeleton/partial/complete/production       │
// │  6 │ lineage      │ spectrai-0.9/apeireth-0.14/...             │
// └────┴──────────────┴────────────────────────────────────────────┘
//
// 24 维 ID (4 × 6 = 24, 编译期 hardcode):
// ┌────┬─────────────────────┬────┬─────────────────────┐
// │ #  │ PC 维               │ #  │ RC 维               │
// ├────┼─────────────────────┼────┼─────────────────────┤
// │ 00 │ PC.level            │ 06 │ RC.level            │
// │ 01 │ PC.domain           │ 07 │ RC.domain           │
// │ 02 │ PC.modality         │ 08 │ RC.modality         │
// │ 03 │ PC.safety           │ 09 │ RC.safety           │
// │ 04 │ PC.completeness     │ 10 │ RC.completeness     │
// │ 05 │ PC.lineage          │ 11 │ RC.lineage          │
// ├────┼─────────────────────┼────┼─────────────────────┤
// │ 12 │ HG.level            │ 18 │ GP.level            │
// │ 13 │ HG.domain           │ 19 │ GP.domain           │
// │ 14 │ HG.modality         │ 20 │ GP.modality         │
// │ 15 │ HG.safety           │ 21 │ GP.safety           │
// │ 16 │ HG.completeness     │ 22 │ GP.completeness     │
// │ 17 │ HG.lineage          │ 23 │ GP.lineage          │
// └────┴─────────────────────┴────┴─────────────────────┘
//
// 命名格式:
//   `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
// 例:
//   `apeireth:5.PC.code.text.high.complete.apeireth-1.0`
//   `apeireth:9.GP.dialogue.audio.low.skeleton.spectrai-0.9`
//
// ============================================================================
// 守门 4 道 (per K-1 强校验)
// ============================================================================
//
// 1. **24 维硬枚举** — 4 大类 × 6 维度 = 24 个 enum, 编译期 hardcode, 改 enum 数量立刻破坏编译.
// 2. **sum=1.00 守门** — 4 大类权重 (PC 0.40 + RC 0.30 + HG 0.15 + GP 0.15) 和必须 = 1.00 (容差 0.001).
// 3. **24 维合法性** — V05Spec 内 4 大类 dim 字段值都在合法 enum 范围 (编译期保证).
// 4. **roundtrip 一致性** — encode → decode 100% 还原, 防 m3 hallucination 改 encode 忘改 decode.
//
// ============================================================================
// 默认 24 维全量访问 helper (按 V05DimId 索引)
// ============================================================================

/// 按 24 维 ID 访问 spec 的 (class, dimension_value).
///
/// 返回 24 维的 (Class, 维枚举值). 例如: `dim_at(spec, V05DimId::PcLevel)` 返 (Pc, Maturing1).
///
/// 用途: 上游 UI 仪表盘按 24 维 ID 取值, 避免手动匹配.
pub fn dim_at(spec: &V05Spec, id: V05DimId) -> (Class, &DimensionSet) {
    let dim = match id.class() {
        Class::Pc => &spec.dims.pc,
        Class::Rc => &spec.dims.rc,
        Class::Hg => &spec.dims.hg,
        Class::Gp => &spec.dims.gp,
    };
    (id.class(), dim)
}

// ============================================================================
// In-crate 测试 (24 维 get/level/domain/... 24 个 fn + 24 维 ALL 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: CRATE_CLASS_COUNT = 4.
    #[test]
    fn crate_class_count_is_4() {
        assert_eq!(CRATE_CLASS_COUNT, 4);
    }

    /// 守门 #2: CRATE_DIMENSION_COUNT = 6.
    #[test]
    fn crate_dimension_count_is_6() {
        assert_eq!(CRATE_DIMENSION_COUNT, 6);
    }

    /// 守门 #3: CRATE_V05_TOTAL_DIMS = 24.
    #[test]
    fn crate_v05_total_dims_is_24() {
        assert_eq!(CRATE_V05_TOTAL_DIMS, 24);
    }

    /// 守门 #4: V05DimId::ALL 长度 = 24.
    #[test]
    fn v05_dim_id_all_count_is_24() {
        assert_eq!(V05DimId::ALL.len(), 24);
    }

    /// 守门 #5: 4 大类各 6 维 in V05DimId::ALL.
    #[test]
    fn v05_dim_id_has_4_classes_x_6_dims() {
        // 计数每类多少个
        let mut pc = 0;
        let mut rc = 0;
        let mut hg = 0;
        let mut gp = 0;
        for id in V05DimId::ALL {
            match id.class() {
                Class::Pc => pc += 1,
                Class::Rc => rc += 1,
                Class::Hg => hg += 1,
                Class::Gp => gp += 1,
            }
        }
        assert_eq!(pc, 6, "PC 必须 6 维");
        assert_eq!(rc, 6, "RC 必须 6 维");
        assert_eq!(hg, 6, "HG 必须 6 维");
        assert_eq!(gp, 6, "GP 必须 6 维");
    }

    /// 守门 #6: 24 维 name 格式 "XX.dim".
    #[test]
    fn v05_dim_id_name_format() {
        assert_eq!(V05DimId::PcLevel.name(), "PC.level");
        assert_eq!(V05DimId::GpLineage.name(), "GP.lineage");
        assert_eq!(V05DimId::HgSafety.name(), "HG.safety");
    }

    /// 守门 #7: 24 维 offset ∈ 0..=5.
    #[test]
    fn v05_dim_id_offset_in_range() {
        for id in V05DimId::ALL {
            assert!(id.offset() < 6, "offset 必须 < 6");
        }
    }

    /// 守门 #8: 4 大类 in V05DimId 全覆盖.
    #[test]
    fn v05_dim_id_class_coverage() {
        let mut has_pc = false;
        let mut has_rc = false;
        let mut has_hg = false;
        let mut has_gp = false;
        for id in V05DimId::ALL {
            match id.class() {
                Class::Pc => has_pc = true,
                Class::Rc => has_rc = true,
                Class::Hg => has_hg = true,
                Class::Gp => has_gp = true,
            }
        }
        assert!(has_pc && has_rc && has_hg && has_gp, "4 大类必须全覆盖");
    }

    /// 守门 #9: V05Spec 24 个 get fn 都工作.
    #[test]
    fn v05_spec_24_get_fns() {
        let pc_dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let rc_dim = DimensionSet::new(
            Level::Expert1,
            Domain::Tool,
            Modality::Multimodal,
            Safety::Critical,
            Completeness::Production,
            Lineage::Apeireth20,
        );
        let hg_dim = DimensionSet::new(
            Level::Maturing1,
            Domain::Reasoning,
            Modality::Text,
            Safety::Medium,
            Completeness::Partial,
            Lineage::Apeireth014,
        );
        let gp_dim = DimensionSet::new(
            Level::Seed,
            Domain::Dialogue,
            Modality::Audio,
            Safety::Low,
            Completeness::Skeleton,
            Lineage::Spectra09,
        );
        let dims = ClassDims::new(pc_dim, rc_dim, hg_dim, gp_dim);
        let spec = V05Spec::new(Level::Maturing1, dims);

        // PC 6 get
        assert_eq!(spec.pc_level(), Level::Mature);
        assert_eq!(spec.pc_domain(), Domain::Code);
        assert_eq!(spec.pc_modality(), Modality::Text);
        assert_eq!(spec.pc_safety(), Safety::High);
        assert_eq!(spec.pc_completeness(), Completeness::Complete);
        assert_eq!(spec.pc_lineage(), Lineage::Apeireth10);

        // RC 6 get
        assert_eq!(spec.rc_level(), Level::Expert1);
        assert_eq!(spec.rc_domain(), Domain::Tool);
        assert_eq!(spec.rc_modality(), Modality::Multimodal);
        assert_eq!(spec.rc_safety(), Safety::Critical);
        assert_eq!(spec.rc_completeness(), Completeness::Production);
        assert_eq!(spec.rc_lineage(), Lineage::Apeireth20);

        // HG 6 get
        assert_eq!(spec.hg_level(), Level::Maturing1);
        assert_eq!(spec.hg_domain(), Domain::Reasoning);
        assert_eq!(spec.hg_modality(), Modality::Text);
        assert_eq!(spec.hg_safety(), Safety::Medium);
        assert_eq!(spec.hg_completeness(), Completeness::Partial);
        assert_eq!(spec.hg_lineage(), Lineage::Apeireth014);

        // GP 6 get
        assert_eq!(spec.gp_level(), Level::Seed);
        assert_eq!(spec.gp_domain(), Domain::Dialogue);
        assert_eq!(spec.gp_modality(), Modality::Audio);
        assert_eq!(spec.gp_safety(), Safety::Low);
        assert_eq!(spec.gp_completeness(), Completeness::Skeleton);
        assert_eq!(spec.gp_lineage(), Lineage::Spectra09);
    }

    /// 守门 #10: V05Spec 6 个 get_* (按 V05DimId) fn 都工作.
    #[test]
    fn v05_spec_get_by_id() {
        let spec = default_v05_spec();
        // 24 维中 PC.level 取 level
        assert_eq!(spec.get_level(V05DimId::PcLevel), Level::Mature);
        assert_eq!(spec.get_domain(V05DimId::PcDomain), Domain::Code);
        assert_eq!(spec.get_modality(V05DimId::PcModality), Modality::Text);
        assert_eq!(spec.get_safety(V05DimId::PcSafety), Safety::High);
        assert_eq!(spec.get_completeness(V05DimId::PcCompleteness), Completeness::Complete);
        assert_eq!(spec.get_lineage(V05DimId::PcLineage), Lineage::Apeireth10);
    }

    /// 守门 #11: default_v05_spec 构造正确.
    #[test]
    fn default_v05_spec_ok() {
        let spec = default_v05_spec();
        assert_eq!(spec.level, Level::Mature);
        assert_eq!(spec.pc_level(), Level::Mature);
    }

    /// 守门 #12: CRATE_PREFIX = "apeireth:".
    #[test]
    fn crate_prefix_hardcoded() {
        assert_eq!(CRATE_PREFIX, "apeireth:");
    }

    /// 守门 #13: CRATE_SEP = '.'.
    #[test]
    fn crate_sep_hardcoded() {
        assert_eq!(CRATE_SEP, '.');
    }

    /// 守门 #14: CRATE_SEGMENT_COUNT = 8.
    #[test]
    fn crate_segment_count_is_8() {
        assert_eq!(CRATE_SEGMENT_COUNT, 8);
    }

    /// 守门 #15: V05Spec pc_dim 完整 dim set.
    #[test]
    fn v05_spec_pc_dim_returns_full_set() {
        let spec = default_v05_spec();
        let dim = spec.pc_dim();
        assert_eq!(dim.level, Level::Mature);
        assert_eq!(dim.domain, Domain::Code);
        assert_eq!(dim.modality, Modality::Text);
        assert_eq!(dim.safety, Safety::High);
        assert_eq!(dim.completeness, Completeness::Complete);
        assert_eq!(dim.lineage, Lineage::Apeireth10);
    }

    /// 守门 #16: dim_at helper 按 V05DimId 索引 24 维, 返回 (Class, DimensionSet) 且 class() 一致.
    #[test]
    fn dim_at_returns_correct_class_for_all_24_ids() {
        let spec = default_v05_spec();
        for id in V05DimId::ALL {
            let (class, _dim) = dim_at(&spec, *id);
            assert_eq!(
                class,
                id.class(),
                "dim_at({:?}) class 应 = {:?}",
                id,
                id.class()
            );
        }
    }

    /// 守门 #17: dim_at helper 24 维全表 — 4 大类各 6 维 = 24, 每类计数 = 6.
    #[test]
    fn dim_at_covers_all_24_dims() {
        let spec = default_v05_spec();
        let mut pc = 0;
        let mut rc = 0;
        let mut hg = 0;
        let mut gp = 0;
        for id in V05DimId::ALL {
            let (class, dim) = dim_at(&spec, *id);
            // default_v05_spec 内每类 dim 都一样
            assert_eq!(dim.level, Level::Mature);
            assert_eq!(dim.domain, Domain::Code);
            assert_eq!(dim.modality, Modality::Text);
            assert_eq!(dim.safety, Safety::High);
            assert_eq!(dim.completeness, Completeness::Complete);
            assert_eq!(dim.lineage, Lineage::Apeireth10);
            match class {
                Class::Pc => pc += 1,
                Class::Rc => rc += 1,
                Class::Hg => hg += 1,
                Class::Gp => gp += 1,
            }
        }
        assert_eq!(pc, 6, "PC 必须 6 维");
        assert_eq!(rc, 6, "RC 必须 6 维");
        assert_eq!(hg, 6, "HG 必须 6 维");
        assert_eq!(gp, 6, "GP 必须 6 维");
    }
}
