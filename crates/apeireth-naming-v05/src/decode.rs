//! # decode — V0.5 字符串 → 24 维结构
//!
//! 命名格式: `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
//!
//! ## 设计要点
//!
//! 1. **单 class 解码** (`decode_v05_class`) — 把 1 行 `apeireth:5.PC.code.text.high.complete.apeireth-1.0`
//!    解码成 (Class, Level, DimensionSet).
//! 2. **4 行解码** (`decode_v05`) — 4 行 (4 大类) → 完整 V05Spec. 4 大类 level 必须一致.
//! 3. **regex 解析**: 用 `regex` crate 做段数 + 前缀校验, 各段值用 parse 函数细化.
//! 4. **m3 防御**: 段数错 (≠ 8) / prefix 错 / level/class/dim 任意一段错 → 立刻 `NamingError`.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use crate::class::{Class, ClassDims, V05Spec};
use crate::dimension::{DimensionSet, Level};
use crate::encode::V05_PREFIX;
use crate::error::{NamingError, NamingResult};

// ============================================================================
// §1 编译期正则 (m3 防御: 格式硬编码, 不允许改)
// ============================================================================

/// 单 class 命名正则 (8 段, 段间 '.' 分隔).
///
/// 格式: `apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`
///
/// m3 防御: 改这个 regex 会立刻破坏 decode, 防止 hallucination 改格式.
///
/// 注: 各段值用更宽松的 `[A-Za-z0-9.-]+` 匹配, 具体值合法性交给 parse 函数 (返回特定 error variant).
pub const V05_LINE_REGEX: &str = r"^apeireth:([0-9]+)\.([A-Za-z]+)\.([A-Za-z]+)\.([A-Za-z]+)\.([A-Za-z]+)\.([A-Za-z]+)\.([A-Za-z0-9.-]+)$";

// ============================================================================
// §2 单 class 解码
// ============================================================================

/// 解码单行 → (Class, Level, DimensionSet).
///
/// 输入例: `"apeireth:5.PC.code.text.high.complete.apeireth-1.0"`
/// 输出: `(Class::Pc, Level::Maturing1, DimensionSet {...})`
pub fn decode_v05_class(line: &str) -> NamingResult<(Class, Level, DimensionSet)> {
    // 1) prefix 守门
    if !line.starts_with(V05_PREFIX) {
        return Err(NamingError::InvalidPrefix(line.to_string()));
    }

    // 2) regex 匹配 (宽松, 各段值用 parse 函数细化)
    let re = regex::Regex::new(V05_LINE_REGEX)
        .expect("V05_LINE_REGEX 编译期 hardcode, 必须合法");
    let caps = re.captures(line).ok_or_else(|| {
        NamingError::MalformedFormat(format!("regex 不匹配: {line}"))
    })?;

    // 3) 解析 7 段 (level/class/domain/modality/safety/completeness/lineage)
    let level_str = caps.get(1).ok_or_else(|| {
        NamingError::MalformedFormat("缺 level 段".to_string())
    })?.as_str();
    let class_str = caps.get(2).ok_or_else(|| {
        NamingError::MalformedFormat("缺 class 段".to_string())
    })?.as_str();
    let domain_str = caps.get(3).ok_or_else(|| {
        NamingError::MalformedFormat("缺 domain 段".to_string())
    })?.as_str();
    let modality_str = caps.get(4).ok_or_else(|| {
        NamingError::MalformedFormat("缺 modality 段".to_string())
    })?.as_str();
    let safety_str = caps.get(5).ok_or_else(|| {
        NamingError::MalformedFormat("缺 safety 段".to_string())
    })?.as_str();
    let completeness_str = caps.get(6).ok_or_else(|| {
        NamingError::MalformedFormat("缺 completeness 段".to_string())
    })?.as_str();
    let lineage_str = caps.get(7).ok_or_else(|| {
        NamingError::MalformedFormat("缺 lineage 段".to_string())
    })?.as_str();

    // 4) 转 enum (失败的段返对应 NamingError, 不是 MalformedFormat)
    let level_num: u8 = level_str.parse().map_err(|_| {
        NamingError::InvalidLevel(level_str.to_string())
    })?;
    let level = Level::from_u8(level_num)?;
    let class = Class::parse(class_str)?;
    let domain = crate::dimension::Domain::parse(domain_str)?;
    let modality = crate::dimension::Modality::parse(modality_str)?;
    let safety = crate::dimension::Safety::parse(safety_str)?;
    let completeness = crate::dimension::Completeness::parse(completeness_str)?;
    let lineage = crate::dimension::Lineage::parse(lineage_str)?;

    // 5) 构造 DimensionSet
    let dim = DimensionSet::new(level, domain, modality, safety, completeness, lineage);
    Ok((class, level, dim))
}

// ============================================================================
// §3 完整 24 维解码 (4 行 → V05Spec)
// ============================================================================

/// 解码 4 行 (4 大类) → 完整 V05Spec.
///
/// 约束: 4 行 level 必须一致, 4 大类必须齐全 (PC/RC/HG/GP 各 1 行).
pub fn decode_v05(s: &str) -> NamingResult<V05Spec> {
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() != 4 {
        return Err(NamingError::MalformedFormat(format!(
            "期望 4 行 (4 大类), 实际 {actual}",
            actual = lines.len(),
        )));
    }

    // 1) 4 行逐个解码
    let mut pc_dim: Option<DimensionSet> = None;
    let mut rc_dim: Option<DimensionSet> = None;
    let mut hg_dim: Option<DimensionSet> = None;
    let mut gp_dim: Option<DimensionSet> = None;
    let mut common_level: Option<Level> = None;

    for line in &lines {
        let (class, level, dim) = decode_v05_class(line)?;

        // 2) 4 行 level 必须一致
        match common_level {
            None => common_level = Some(level),
            Some(prev) if prev == level => {}
            Some(other) => {
                return Err(NamingError::MalformedFormat(format!(
                    "4 大类 level 必须一致, 之前 {other:?}, 实际 {level:?}",
                )));
            }
        }

        // 3) 按类存 DimensionSet
        match class {
            Class::Pc => {
                if pc_dim.is_some() {
                    return Err(NamingError::MalformedFormat("PC 重复".to_string()));
                }
                pc_dim = Some(dim);
            }
            Class::Rc => {
                if rc_dim.is_some() {
                    return Err(NamingError::MalformedFormat("RC 重复".to_string()));
                }
                rc_dim = Some(dim);
            }
            Class::Hg => {
                if hg_dim.is_some() {
                    return Err(NamingError::MalformedFormat("HG 重复".to_string()));
                }
                hg_dim = Some(dim);
            }
            Class::Gp => {
                if gp_dim.is_some() {
                    return Err(NamingError::MalformedFormat("GP 重复".to_string()));
                }
                gp_dim = Some(dim);
            }
        }
    }

    // 4) 4 大类必须齐全
    let pc_dim = pc_dim.ok_or_else(|| {
        NamingError::MalformedFormat("缺 PC 行".to_string())
    })?;
    let rc_dim = rc_dim.ok_or_else(|| {
        NamingError::MalformedFormat("缺 RC 行".to_string())
    })?;
    let hg_dim = hg_dim.ok_or_else(|| {
        NamingError::MalformedFormat("缺 HG 行".to_string())
    })?;
    let gp_dim = gp_dim.ok_or_else(|| {
        NamingError::MalformedFormat("缺 GP 行".to_string())
    })?;
    let level = common_level.ok_or_else(|| {
        NamingError::MalformedFormat("缺 level".to_string())
    })?;

    let dims = ClassDims::new(pc_dim, rc_dim, hg_dim, gp_dim);
    Ok(V05Spec::new(level, dims))
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::{Completeness, Domain, Lineage, Modality, Safety};

    /// 守门 #1: 单 class 解码正确.
    #[test]
    fn decode_v05_class_full_spec() {
        let line = "apeireth:9.PC.code.text.high.complete.apeireth-1.0";
        let (class, level, dim) = decode_v05_class(line).unwrap();
        assert_eq!(class, Class::Pc);
        assert_eq!(level, Level::Mature);
        assert_eq!(dim.domain, Domain::Code);
        assert_eq!(dim.modality, Modality::Text);
        assert_eq!(dim.safety, Safety::High);
        assert_eq!(dim.completeness, Completeness::Complete);
        assert_eq!(dim.lineage, Lineage::Apeireth10);
    }

    /// 守门 #2: 4 类 (PC/RC/HG/GP) 都能解码.
    #[test]
    fn decode_v05_class_all_four() {
        for (line, expected) in [
            ("apeireth:5.PC.code.text.high.complete.apeireth-1.0", Class::Pc),
            ("apeireth:5.RC.code.text.high.complete.apeireth-1.0", Class::Rc),
            ("apeireth:5.HG.code.text.high.complete.apeireth-1.0", Class::Hg),
            ("apeireth:5.GP.code.text.high.complete.apeireth-1.0", Class::Gp),
        ] {
            let (class, _, _) = decode_v05_class(line).unwrap();
            assert_eq!(class, expected);
        }
    }

    /// 守门 #3: 缺 prefix → InvalidPrefix.
    #[test]
    fn decode_v05_class_missing_prefix() {
        let line = "5.PC.code.text.high.complete.apeireth-1.0";
        let err = decode_v05_class(line).unwrap_err();
        assert!(matches!(err, NamingError::InvalidPrefix(_)));
    }

    /// 守门 #4: level 越界 → InvalidLevel.
    #[test]
    fn decode_v05_class_level_out_of_range() {
        let line = "apeireth:15.PC.code.text.high.complete.apeireth-1.0";
        let err = decode_v05_class(line).unwrap_err();
        assert!(matches!(err, NamingError::InvalidLevel(_)));
    }

    /// 守门 #5: 非法 class → InvalidClass.
    #[test]
    fn decode_v05_class_invalid_class() {
        let line = "apeireth:5.XX.code.text.high.complete.apeireth-1.0";
        let err = decode_v05_class(line).unwrap_err();
        assert!(matches!(err, NamingError::InvalidClass(_)));
    }

    /// 守门 #6: 非法 domain → InvalidDomain.
    #[test]
    fn decode_v05_class_invalid_domain() {
        let line = "apeireth:5.PC.nonsense.text.high.complete.apeireth-1.0";
        let err = decode_v05_class(line).unwrap_err();
        assert!(matches!(err, NamingError::InvalidDomain(_)));
    }

    /// 守门 #7: 非法 lineage → InvalidLineage.
    #[test]
    fn decode_v05_class_invalid_lineage() {
        let line = "apeireth:5.PC.code.text.high.complete.apeireth-99.99";
        let err = decode_v05_class(line).unwrap_err();
        assert!(matches!(err, NamingError::InvalidLineage(_)));
    }

    /// 守门 #8: 段数错 → MalformedFormat.
    #[test]
    fn decode_v05_class_wrong_segment_count() {
        let line = "apeireth:5.PC.code.text.high.complete";
        let err = decode_v05_class(line).unwrap_err();
        assert!(matches!(err, NamingError::MalformedFormat(_)));
    }

    /// 守门 #9: 完整 4 行解码.
    #[test]
    fn decode_v05_full_4_lines() {
        let s = "\
apeireth:5.PC.code.text.high.complete.apeireth-1.0
apeireth:5.RC.code.text.high.complete.apeireth-1.0
apeireth:5.HG.code.text.high.complete.apeireth-1.0
apeireth:5.GP.code.text.high.complete.apeireth-1.0";
        let spec = decode_v05(s).unwrap();
        assert_eq!(spec.level, Level::Maturing1);
        // 4 大类 dim 应该都相同
        let pc = spec.dims.get(Class::Pc);
        let rc = spec.dims.get(Class::Rc);
        let hg = spec.dims.get(Class::Hg);
        let gp = spec.dims.get(Class::Gp);
        assert_eq!(pc, rc);
        assert_eq!(rc, hg);
        assert_eq!(hg, gp);
    }

    /// 守门 #10: 行数错 (3 行) → MalformedFormat.
    #[test]
    fn decode_v05_wrong_line_count() {
        let s = "\
apeireth:5.PC.code.text.high.complete.apeireth-1.0
apeireth:5.RC.code.text.high.complete.apeireth-1.0
apeireth:5.HG.code.text.high.complete.apeireth-1.0";
        let err = decode_v05(s).unwrap_err();
        assert!(matches!(err, NamingError::MalformedFormat(_)));
    }

    /// 守门 #11: 大类重复 (HG 出现 2 次) → MalformedFormat.
    #[test]
    fn decode_v05_duplicate_class_hg() {
        let s = "\
apeireth:5.PC.code.text.high.complete.apeireth-1.0
apeireth:5.RC.code.text.high.complete.apeireth-1.0
apeireth:5.HG.code.text.high.complete.apeireth-1.0
apeireth:5.HG.code.text.high.complete.apeireth-1.0";
        let err = decode_v05(s).unwrap_err();
        // HG 重复 → "HG 重复" 错误 (重复检查先于齐全检查)
        assert!(matches!(err, NamingError::MalformedFormat(s) if s.contains("HG 重复")));
    }

    /// 守门 #12: 4 大类 level 不一致 → MalformedFormat.
    #[test]
    fn decode_v05_level_mismatch() {
        let s = "\
apeireth:5.PC.code.text.high.complete.apeireth-1.0
apeireth:6.RC.code.text.high.complete.apeireth-1.0
apeireth:5.HG.code.text.high.complete.apeireth-1.0
apeireth:5.GP.code.text.high.complete.apeireth-1.0";
        let err = decode_v05(s).unwrap_err();
        assert!(matches!(err, NamingError::MalformedFormat(_)));
    }

    /// 守门 #13: PC 重复 → MalformedFormat.
    #[test]
    fn decode_v05_duplicate_class_pc() {
        let s = "\
apeireth:5.PC.code.text.high.complete.apeireth-1.0
apeireth:5.RC.code.text.high.complete.apeireth-1.0
apeireth:5.PC.code.text.high.complete.apeireth-1.0
apeireth:5.GP.code.text.high.complete.apeireth-1.0";
        let err = decode_v05(s).unwrap_err();
        assert!(matches!(err, NamingError::MalformedFormat(s) if s.contains("PC 重复")));
    }
}
