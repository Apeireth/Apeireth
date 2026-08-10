//! # encode — V0.5 24 维结构 → 字符串
//!
//! 命名格式: `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
//!
//! 例: `apeireth:5.PC.code.text.high.complete.apeireth-1.0`
//!
//! ## 设计要点
//!
//! 1. **单 class 编码** (`encode_v05_class`) — 把 (class, DimensionSet) 编码成
//!    `apeireth:5.PC.code.text.high.complete.apeireth-1.0` 7 段.
//! 2. **24 维全编码** (`encode_v05_all`) — 4 大类各 1 次, 用换行分隔, 4 行总输出.
//! 3. **m3 防御**: 段间用 `.` 分隔, 段数恒 7, 不允许运行时改格式.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use crate::class::{Class, V05Spec};
use crate::dimension::{DimensionSet, Level};
use crate::error::{NamingError, NamingResult};

// ============================================================================
// §1 命名格式常量 (编译期 hardcode, m3 防御)
// ============================================================================

/// 命名空间前缀 (per V0.5 命名规范, "apeireth:" 硬编码).
pub const V05_PREFIX: &str = "apeireth:";

/// 段间分隔符 (`.` 硬编码).
pub const V05_SEP: char = '.';

/// 命名总段数 (1 prefix + 1 level + 1 class + 1 domain + 1 modality + 1 safety + 1 completeness + 1 lineage = 8 段, 但 prefix 含 ":").
///
/// 实际语义段数: 1 prefix + 1 level + 1 class + 5 维 (domain/modality/safety/completeness/lineage) = 8 段.
///
/// K-1 守门: 改这个数字会破坏 decode, 编译期 hardcode.
pub const V05_SEGMENT_COUNT: usize = 8;

// ============================================================================
// §2 单 class 编码 (1 个 DimensionSet → 字符串)
// ============================================================================

/// 编码单 (class, DimensionSet) → 7 段字符串 (含 level).
///
/// 格式: `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
///
/// 例: `encode_v05_class(Class::Pc, Level::Mature, dim) → "apeireth:9.PC.code.text.high.complete.apeireth-1.0"`
pub fn encode_v05_class(
    class: Class,
    level: Level,
    dim: DimensionSet,
) -> NamingResult<String> {
    let s = format!(
        "{}{}.{}.{}.{}.{}.{}.{}",
        V05_PREFIX,
        level.as_u8(),
        class.as_str(),
        dim.domain.as_str(),
        dim.modality.as_str(),
        dim.safety.as_str(),
        dim.completeness.as_str(),
        dim.lineage.as_str(),
    );
    // 自检: 段数 = 8 (split(".") → 8 段, 因为 prefix 末尾 ":" 后才是 level)
    let segments: Vec<&str> = s.split('.').collect();
    if segments.len() != V05_SEGMENT_COUNT {
        return Err(NamingError::MalformedFormat(format!(
            "encode 后段数 {expected}, 实际 {actual}",
            expected = V05_SEGMENT_COUNT,
            actual = segments.len(),
        )));
    }
    Ok(s)
}

// ============================================================================
// §3 完整 24 维编码 (4 大类 1 行, 总 4 行)
// ============================================================================

/// 编码完整 V05Spec → 4 行字符串 (1 行 1 大类, 24 维).
///
/// 格式:
/// ```text
/// apeireth:5.PC.code.text.high.complete.apeireth-1.0
/// apeireth:5.RC.code.text.high.complete.apeireth-1.0
/// apeireth:5.HG.code.text.high.complete.apeireth-1.0
/// apeireth:5.GP.code.text.high.complete.apeireth-1.0
/// ```
pub fn encode_v05(spec: &V05Spec) -> NamingResult<String> {
    let mut out = String::new();
    for class in Class::ALL {
        let dim = spec.dims.get(*class);
        let line = encode_v05_class(*class, spec.level, dim)?;
        out.push_str(&line);
        out.push('\n');
    }
    // 去掉末尾 '\n'
    if out.ends_with('\n') {
        out.pop();
    }
    Ok(out)
}

/// 编码完整 V05Spec → 4 行字符串数组 (1 行 1 大类).
///
/// 比 `encode_v05` 更结构化, 方便下游按行处理.
pub fn encode_v05_lines(spec: &V05Spec) -> NamingResult<Vec<String>> {
    let mut out = Vec::with_capacity(Class::ALL.len());
    for class in Class::ALL {
        let dim = spec.dims.get(*class);
        out.push(encode_v05_class(*class, spec.level, dim)?);
    }
    Ok(out)
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::ClassDims;
    use crate::dimension::{Completeness, Domain, Lineage, Modality, Safety};

    /// 守门 #1: V05_PREFIX 硬编码 "apeireth:".
    #[test]
    fn v05_prefix_hardcoded() {
        assert_eq!(V05_PREFIX, "apeireth:");
    }

    /// 守门 #2: V05_SEP 硬编码 '.'.
    #[test]
    fn v05_sep_hardcoded() {
        assert_eq!(V05_SEP, '.');
    }

    /// 守门 #3: V05_SEGMENT_COUNT = 8.
    #[test]
    fn v05_segment_count_is_8() {
        assert_eq!(V05_SEGMENT_COUNT, 8);
    }

    /// 守门 #4: 编码单 class → 7 段格式正确.
    #[test]
    fn encode_v05_class_full_spec() {
        let dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let s = encode_v05_class(Class::Pc, Level::Mature, dim).unwrap();
        assert_eq!(s, "apeireth:9.PC.code.text.high.complete.apeireth-1.0");
    }

    /// 守门 #5: 编码 4 大类 → 4 行.
    #[test]
    fn encode_v05_full_4_lines() {
        let dim = DimensionSet::new(
            Level::Maturing1, // 5
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let spec = V05Spec::new(Level::Maturing1, dims);
        let s = encode_v05(&spec).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 4, "24 维 = 4 行");
        assert_eq!(lines[0], "apeireth:5.PC.code.text.high.complete.apeireth-1.0");
        assert_eq!(lines[1], "apeireth:5.RC.code.text.high.complete.apeireth-1.0");
        assert_eq!(lines[2], "apeireth:5.HG.code.text.high.complete.apeireth-1.0");
        assert_eq!(lines[3], "apeireth:5.GP.code.text.high.complete.apeireth-1.0");
    }

    /// 守门 #6: encode_v05_lines 返 4 元素 vec.
    #[test]
    fn encode_v05_lines_returns_4() {
        let dim = DimensionSet::new(
            Level::Expert1, // 7
            Domain::Tool,
            Modality::Multimodal,
            Safety::Critical,
            Completeness::Production,
            Lineage::Apeireth20,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let spec = V05Spec::new(Level::Expert1, dims);
        let lines = encode_v05_lines(&spec).unwrap();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "apeireth:7.PC.tool.multimodal.critical.production.apeireth-2.0");
        assert_eq!(lines[1], "apeireth:7.RC.tool.multimodal.critical.production.apeireth-2.0");
    }

    /// 守门 #7: 4 大类不同 dimension 也 OK.
    #[test]
    fn encode_v05_with_different_dims_per_class() {
        let pc_dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let rc_dim = DimensionSet::new(
            Level::Mature,
            Domain::Tool,
            Modality::Multimodal,
            Safety::Critical,
            Completeness::Production,
            Lineage::Apeireth20,
        );
        let hg_dim = DimensionSet::new(
            Level::Mature,
            Domain::Reasoning,
            Modality::Text,
            Safety::Medium,
            Completeness::Partial,
            Lineage::Apeireth014,
        );
        let gp_dim = DimensionSet::new(
            Level::Mature,
            Domain::Dialogue,
            Modality::Audio,
            Safety::Low,
            Completeness::Skeleton,
            Lineage::Spectra09,
        );
        let dims = ClassDims::new(pc_dim, rc_dim, hg_dim, gp_dim);
        let spec = V05Spec::new(Level::Mature, dims);
        let lines = encode_v05_lines(&spec).unwrap();
        assert_eq!(lines[0], "apeireth:9.PC.code.text.high.complete.apeireth-1.0");
        assert_eq!(lines[1], "apeireth:9.RC.tool.multimodal.critical.production.apeireth-2.0");
        assert_eq!(lines[2], "apeireth:9.HG.reasoning.text.medium.partial.apeireth-0.14");
        assert_eq!(lines[3], "apeireth:9.GP.dialogue.audio.low.skeleton.spectrai-0.9");
    }

    /// 守门 #8: encode 不会引入额外空格.
    #[test]
    fn encode_v05_no_trailing_space() {
        let dim = DimensionSet::new(
            Level::Seed,
            Domain::Code,
            Modality::Text,
            Safety::Low,
            Completeness::Skeleton,
            Lineage::Spectra09,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        let spec = V05Spec::new(Level::Seed, dims);
        let s = encode_v05(&spec).unwrap();
        assert!(!s.contains("  "), "不应有双空格");
        assert!(!s.ends_with('\n'), "末尾不应有换行");
    }
}
