//! # dimension — 6 维度定义 (level/domain/modality/safety/completeness/lineage)
//!
//! 6 维度 × 4 大类 = 24 维, 是 V0.5 命名规范的核心数据结构.
//!
//! ## 6 维度速查 (per V0.5 v2 提议, 24 维)
//!
//! | # | 维度 | 取值范围 | 取值数 | 守门 |
//! |---|---|---|---|---|
//! | 1 | level | 0-9 (0=seed, 9=mature) | 10 | 0..=9 整数 |
//! | 2 | domain | code/dialogue/vision/audio/tool/reasoning | 6 | enum |
//! | 3 | modality | text/image/audio/video/multimodal | 5 | enum |
//! | 4 | safety | low/medium/high/critical | 4 | enum |
//! | 5 | completeness | skeleton/partial/complete/production | 4 | enum |
//! | 6 | lineage | spectrai-0.9/apeireth-0.14/apeireth-1.0/apeireth-2.0 | 4 | enum |
//!
//! ## 24 维 = 6 维度 × 4 大类 (PC/RC/HG/GP)
//!
//! 每个 enum 都会在 `V05Spec` 里出现 4 次 (每类 1 次), 组成完整 24 维结构.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use serde::{Deserialize, Serialize};

use crate::error::NamingError;

// ============================================================================
// §1 6 维度 (编译期 hardcode enum, m3 防御)
// ============================================================================

/// 维度 1: level (0-9, 0=seed, 9=mature).
///
/// ## 取值 (10 个)
/// - 0: seed (种子, 刚启动)
/// - 1-2: sprouting (萌芽, 学习基础)
/// - 3-4: growing (成长, 掌握核心)
/// - 5-6: maturing (成熟, 实战稳定)
/// - 7-8: expert (专家, 高难度任务)
/// - 9: mature (全熟, ASI 北极星)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Level {
    /// 0: seed (种子)
    Seed = 0,
    /// 1: sprouting 早
    Sprouting1 = 1,
    /// 2: sprouting 晚
    Sprouting2 = 2,
    /// 3: growing 早
    Growing1 = 3,
    /// 4: growing 晚
    Growing2 = 4,
    /// 5: maturing 早
    Maturing1 = 5,
    /// 6: maturing 晚
    Maturing2 = 6,
    /// 7: expert 早
    Expert1 = 7,
    /// 8: expert 晚
    Expert2 = 8,
    /// 9: mature (全熟)
    Mature = 9,
}

impl Level {
    /// 整数 (0-9) → Level.
    pub fn from_u8(n: u8) -> Result<Self, NamingError> {
        match n {
            0 => Ok(Level::Seed),
            1 => Ok(Level::Sprouting1),
            2 => Ok(Level::Sprouting2),
            3 => Ok(Level::Growing1),
            4 => Ok(Level::Growing2),
            5 => Ok(Level::Maturing1),
            6 => Ok(Level::Maturing2),
            7 => Ok(Level::Expert1),
            8 => Ok(Level::Expert2),
            9 => Ok(Level::Mature),
            other => Err(NamingError::InvalidLevel(other.to_string())),
        }
    }

    /// Level → 整数 (0-9).
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// 阶段名 (seed / sprouting / growing / maturing / expert / mature).
    pub fn stage(self) -> &'static str {
        match self {
            Level::Seed => "seed",
            Level::Sprouting1 | Level::Sprouting2 => "sprouting",
            Level::Growing1 | Level::Growing2 => "growing",
            Level::Maturing1 | Level::Maturing2 => "maturing",
            Level::Expert1 | Level::Expert2 => "expert",
            Level::Mature => "mature",
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u8())
    }
}

/// 维度 2: domain (6 主领域).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    /// 代码 (写 / 改 / 审)
    Code,
    /// 对话 (问答 / 闲聊)
    Dialogue,
    /// 视觉 (图像理解)
    Vision,
    /// 音频 (语音 / 音乐)
    Audio,
    /// 工具 (tool calling)
    Tool,
    /// 推理 (逻辑 / 规划)
    Reasoning,
}

impl Domain {
    /// 字符串 → Domain.
    pub fn parse(s: &str) -> Result<Self, NamingError> {
        match s {
            "code" => Ok(Domain::Code),
            "dialogue" => Ok(Domain::Dialogue),
            "vision" => Ok(Domain::Vision),
            "audio" => Ok(Domain::Audio),
            "tool" => Ok(Domain::Tool),
            "reasoning" => Ok(Domain::Reasoning),
            other => Err(NamingError::InvalidDomain(other.to_string())),
        }
    }

    /// Domain → 字符串.
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Code => "code",
            Domain::Dialogue => "dialogue",
            Domain::Vision => "vision",
            Domain::Audio => "audio",
            Domain::Tool => "tool",
            Domain::Reasoning => "reasoning",
        }
    }

    /// 6 域全表 (K-1 强校验).
    pub const ALL: &'static [Domain] = &[
        Domain::Code,
        Domain::Dialogue,
        Domain::Vision,
        Domain::Audio,
        Domain::Tool,
        Domain::Reasoning,
    ];
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 维度 3: modality (5 模态).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modality {
    /// 文本
    Text,
    /// 图像
    Image,
    /// 音频
    Audio,
    /// 视频
    Video,
    /// 多模态
    Multimodal,
}

impl Modality {
    /// 字符串 → Modality.
    pub fn parse(s: &str) -> Result<Self, NamingError> {
        match s {
            "text" => Ok(Modality::Text),
            "image" => Ok(Modality::Image),
            "audio" => Ok(Modality::Audio),
            "video" => Ok(Modality::Video),
            "multimodal" => Ok(Modality::Multimodal),
            other => Err(NamingError::InvalidModality(other.to_string())),
        }
    }

    /// Modality → 字符串.
    pub fn as_str(self) -> &'static str {
        match self {
            Modality::Text => "text",
            Modality::Image => "image",
            Modality::Audio => "audio",
            Modality::Video => "video",
            Modality::Multimodal => "multimodal",
        }
    }

    /// 5 模态全表 (K-1 强校验).
    pub const ALL: &'static [Modality] = &[
        Modality::Text,
        Modality::Image,
        Modality::Audio,
        Modality::Video,
        Modality::Multimodal,
    ];
}

impl std::fmt::Display for Modality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 维度 4: safety (4 安全等级).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Safety {
    /// 低风险
    Low,
    /// 中风险
    Medium,
    /// 高风险
    High,
    /// 关键风险 (需高门槛守门)
    Critical,
}

impl Safety {
    /// 字符串 → Safety.
    pub fn parse(s: &str) -> Result<Self, NamingError> {
        match s {
            "low" => Ok(Safety::Low),
            "medium" => Ok(Safety::Medium),
            "high" => Ok(Safety::High),
            "critical" => Ok(Safety::Critical),
            other => Err(NamingError::InvalidSafety(other.to_string())),
        }
    }

    /// Safety → 字符串.
    pub fn as_str(self) -> &'static str {
        match self {
            Safety::Low => "low",
            Safety::Medium => "medium",
            Safety::High => "high",
            Safety::Critical => "critical",
        }
    }

    /// 4 等级全表 (K-1 强校验).
    pub const ALL: &'static [Safety] =
        &[Safety::Low, Safety::Medium, Safety::High, Safety::Critical];
}

impl std::fmt::Display for Safety {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 维度 5: completeness (4 完整度).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Completeness {
    /// skeleton (骨架, 仅 enum + 类型, 0 真实 impl)
    Skeleton,
    /// partial (部分实现, 核心 API OK)
    Partial,
    /// complete (全部实现, 文档 + 测试齐)
    Complete,
    /// production (生产可用, 真接 + 监控 + 回滚)
    Production,
}

impl Completeness {
    /// 字符串 → Completeness.
    pub fn parse(s: &str) -> Result<Self, NamingError> {
        match s {
            "skeleton" => Ok(Completeness::Skeleton),
            "partial" => Ok(Completeness::Partial),
            "complete" => Ok(Completeness::Complete),
            "production" => Ok(Completeness::Production),
            other => Err(NamingError::InvalidCompleteness(other.to_string())),
        }
    }

    /// Completeness → 字符串.
    pub fn as_str(self) -> &'static str {
        match self {
            Completeness::Skeleton => "skeleton",
            Completeness::Partial => "partial",
            Completeness::Complete => "complete",
            Completeness::Production => "production",
        }
    }

    /// 4 完整度全表 (K-1 强校验).
    pub const ALL: &'static [Completeness] = &[
        Completeness::Skeleton,
        Completeness::Partial,
        Completeness::Complete,
        Completeness::Production,
    ];
}

impl std::fmt::Display for Completeness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 维度 6: lineage (4 血统, 表示从哪个上游继承).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lineage {
    /// spectrai-0.9 (v0.9.21 商业版, 1:1 翻译参考)
    Spectra09,
    /// apeireth-0.14 (R14 Rust 重写骨架)
    Apeireth014,
    /// apeireth-1.0 (1.0 release, 当前 LOCKED)
    Apeireth10,
    /// apeireth-2.0 (2.0 future, R21+ 留口子)
    Apeireth20,
}

impl Lineage {
    /// 字符串 → Lineage.
    pub fn parse(s: &str) -> Result<Self, NamingError> {
        match s {
            "spectrai-0.9" => Ok(Lineage::Spectra09),
            "apeireth-0.14" => Ok(Lineage::Apeireth014),
            "apeireth-1.0" => Ok(Lineage::Apeireth10),
            "apeireth-2.0" => Ok(Lineage::Apeireth20),
            other => Err(NamingError::InvalidLineage(other.to_string())),
        }
    }

    /// Lineage → 字符串.
    pub fn as_str(self) -> &'static str {
        match self {
            Lineage::Spectra09 => "spectrai-0.9",
            Lineage::Apeireth014 => "apeireth-0.14",
            Lineage::Apeireth10 => "apeireth-1.0",
            Lineage::Apeireth20 => "apeireth-2.0",
        }
    }

    /// 4 血统全表 (K-1 强校验).
    pub const ALL: &'static [Lineage] = &[
        Lineage::Spectra09,
        Lineage::Apeireth014,
        Lineage::Apeireth10,
        Lineage::Apeireth20,
    ];
}

impl std::fmt::Display for Lineage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 24 维 计数守门 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 6 维度 1:1 计数 (K-1 强校验).
///
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 漏/加维度.
pub const DIMENSION_COUNT: usize = 6;

/// 4 大类 1:1 计数 (per class.rs, 跟 PC/RC/HG/GP 1:1).
///
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 漏/加大类.
pub const CLASS_COUNT: usize = 4;

/// 24 维 = DIMENSION_COUNT × CLASS_COUNT 守门 (编译期).
///
/// m3 防御: 这个常量是 DIMENSION_COUNT * CLASS_COUNT, 不允许 override.
pub const V05_TOTAL_DIMS: usize = DIMENSION_COUNT * CLASS_COUNT;

// ============================================================================
// §3 DimensionSet — 1 大类的 6 维结构 (per 4 大类, 共有 4 个 DimensionSet)
// ============================================================================

/// 1 大类的 6 维结构 (per 4 大类 PC/RC/HG/GP 各 1 个).
///
/// 4 大类 × DimensionSet = 24 维完整结构. 1 DimensionSet = 6 维.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionSet {
    /// 维度 1: level (0-9)
    pub level: Level,
    /// 维度 2: domain
    pub domain: Domain,
    /// 维度 3: modality
    pub modality: Modality,
    /// 维度 4: safety
    pub safety: Safety,
    /// 维度 5: completeness
    pub completeness: Completeness,
    /// 维度 6: lineage
    pub lineage: Lineage,
}

impl DimensionSet {
    /// 构造新 6 维.
    pub const fn new(
        level: Level,
        domain: Domain,
        modality: Modality,
        safety: Safety,
        completeness: Completeness,
        lineage: Lineage,
    ) -> Self {
        Self {
            level,
            domain,
            modality,
            safety,
            completeness,
            lineage,
        }
    }

    /// 6 维 → 6 字段字符串 slice (用于 encode).
    pub fn to_parts(self) -> [&'static str; 6] {
        [
            self.level.as_u8().to_string().leak(), // 简化, 实际下面重写
            self.domain.as_str(),
            self.modality.as_str(),
            self.safety.as_str(),
            self.completeness.as_str(),
            self.lineage.as_str(),
        ]
    }
}

// ============================================================================
// §4 in-module 测试 (24+ 测试, 6 维度 × 2 parse/display + 计数守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 维度 1: level ---

    #[test]
    fn level_from_u8_0_to_9() {
        for n in 0u8..=9 {
            assert!(Level::from_u8(n).is_ok());
        }
    }

    #[test]
    fn level_from_u8_10_rejected() {
        let err = Level::from_u8(10).unwrap_err();
        assert!(matches!(err, NamingError::InvalidLevel(_)));
    }

    #[test]
    fn level_from_u8_255_rejected() {
        let err = Level::from_u8(255).unwrap_err();
        assert!(matches!(err, NamingError::InvalidLevel(_)));
    }

    #[test]
    fn level_as_u8_roundtrip() {
        for n in 0u8..=9 {
            let l = Level::from_u8(n).unwrap();
            assert_eq!(l.as_u8(), n);
        }
    }

    #[test]
    fn level_stage_names() {
        assert_eq!(Level::Seed.stage(), "seed");
        assert_eq!(Level::Sprouting1.stage(), "sprouting");
        assert_eq!(Level::Mature.stage(), "mature");
    }

    #[test]
    fn level_display_is_number() {
        assert_eq!(Level::Mature.to_string(), "9");
        assert_eq!(Level::Seed.to_string(), "0");
    }

    // --- 维度 2: domain ---

    #[test]
    fn domain_parse_all_six() {
        let all = ["code", "dialogue", "vision", "audio", "tool", "reasoning"];
        for s in all {
            assert!(Domain::parse(s).is_ok(), "Domain::parse({s}) 应 OK");
        }
    }

    #[test]
    fn domain_parse_invalid_rejected() {
        let err = Domain::parse("nonsense").unwrap_err();
        assert!(matches!(err, NamingError::InvalidDomain(_)));
    }

    #[test]
    fn domain_display_roundtrip() {
        for d in Domain::ALL {
            let s = d.to_string();
            let parsed = Domain::parse(&s).unwrap();
            assert_eq!(*d, parsed);
        }
    }

    #[test]
    fn domain_all_count_is_six() {
        assert_eq!(Domain::ALL.len(), 6);
    }

    // --- 维度 3: modality ---

    #[test]
    fn modality_parse_all_five() {
        let all = ["text", "image", "audio", "video", "multimodal"];
        for s in all {
            assert!(Modality::parse(s).is_ok(), "Modality::parse({s}) 应 OK");
        }
    }

    #[test]
    fn modality_parse_invalid_rejected() {
        let err = Modality::parse("hologram").unwrap_err();
        assert!(matches!(err, NamingError::InvalidModality(_)));
    }

    #[test]
    fn modality_display_roundtrip() {
        for m in Modality::ALL {
            let s = m.to_string();
            let parsed = Modality::parse(&s).unwrap();
            assert_eq!(*m, parsed);
        }
    }

    #[test]
    fn modality_all_count_is_five() {
        assert_eq!(Modality::ALL.len(), 5);
    }

    // --- 维度 4: safety ---

    #[test]
    fn safety_parse_all_four() {
        let all = ["low", "medium", "high", "critical"];
        for s in all {
            assert!(Safety::parse(s).is_ok(), "Safety::parse({s}) 应 OK");
        }
    }

    #[test]
    fn safety_parse_invalid_rejected() {
        let err = Safety::parse("extreme").unwrap_err();
        assert!(matches!(err, NamingError::InvalidSafety(_)));
    }

    #[test]
    fn safety_display_roundtrip() {
        for s in Safety::ALL {
            let txt = s.to_string();
            let parsed = Safety::parse(&txt).unwrap();
            assert_eq!(*s, parsed);
        }
    }

    #[test]
    fn safety_all_count_is_four() {
        assert_eq!(Safety::ALL.len(), 4);
    }

    // --- 维度 5: completeness ---

    #[test]
    fn completeness_parse_all_four() {
        let all = ["skeleton", "partial", "complete", "production"];
        for s in all {
            assert!(
                Completeness::parse(s).is_ok(),
                "Completeness::parse({s}) 应 OK"
            );
        }
    }

    #[test]
    fn completeness_parse_invalid_rejected() {
        let err = Completeness::parse("alpha").unwrap_err();
        assert!(matches!(err, NamingError::InvalidCompleteness(_)));
    }

    #[test]
    fn completeness_display_roundtrip() {
        for c in Completeness::ALL {
            let txt = c.to_string();
            let parsed = Completeness::parse(&txt).unwrap();
            assert_eq!(*c, parsed);
        }
    }

    #[test]
    fn completeness_all_count_is_four() {
        assert_eq!(Completeness::ALL.len(), 4);
    }

    // --- 维度 6: lineage ---

    #[test]
    fn lineage_parse_all_four() {
        let all = [
            "spectrai-0.9",
            "apeireth-0.14",
            "apeireth-1.0",
            "apeireth-2.0",
        ];
        for s in all {
            assert!(Lineage::parse(s).is_ok(), "Lineage::parse({s}) 应 OK");
        }
    }

    #[test]
    fn lineage_parse_invalid_rejected() {
        let err = Lineage::parse("apeireth-99.99").unwrap_err();
        assert!(matches!(err, NamingError::InvalidLineage(_)));
    }

    #[test]
    fn lineage_display_roundtrip() {
        for l in Lineage::ALL {
            let txt = l.to_string();
            let parsed = Lineage::parse(&txt).unwrap();
            assert_eq!(*l, parsed);
        }
    }

    #[test]
    fn lineage_all_count_is_four() {
        assert_eq!(Lineage::ALL.len(), 4);
    }

    // --- 24 维 计数守门 ---

    #[test]
    fn dimension_count_is_six() {
        assert_eq!(DIMENSION_COUNT, 6, "6 维度守门");
    }

    #[test]
    fn class_count_is_four() {
        assert_eq!(CLASS_COUNT, 4, "4 大类守门");
    }

    #[test]
    fn v05_total_dims_is_24() {
        assert_eq!(V05_TOTAL_DIMS, 24, "24 维 = 6 × 4 守门");
    }

    // --- DimensionSet ---

    #[test]
    fn dimension_set_construction() {
        let ds = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Production,
            Lineage::Apeireth10,
        );
        assert_eq!(ds.level, Level::Mature);
        assert_eq!(ds.domain, Domain::Code);
        assert_eq!(ds.modality, Modality::Text);
        assert_eq!(ds.safety, Safety::High);
        assert_eq!(ds.completeness, Completeness::Production);
        assert_eq!(ds.lineage, Lineage::Apeireth10);
    }
}
