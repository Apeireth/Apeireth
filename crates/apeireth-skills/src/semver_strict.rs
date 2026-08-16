//! R107: 严格 semver 2.0.0 校验 + 比较 (3-segment + pre-release + build metadata)
//!
//! **依据**: <https://semver.org/spec/v2.0.0.html> 完整 spec 1:1
//!
//! **Apeireth 真接 (本 module)**:
//! - `Semver` struct — major + minor + patch + 可选 pre_release + 可选 build
//! - `parse_strict(v) -> Result<Semver, _>` — 严格按 semver.org 解析 (e.g. "1.0.0-alpha.1+exp.sha.5114f85")
//! - `compare_strict(a, b) -> i32` — pre-release 优先级 (per semver §11):
//!   1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0
//! - `is_valid_strict(v) -> bool` — 校验通过返回 true
//! - `to_string` impl — 反向序列化 (跟 parse 对偶)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `apeireth-skills/src/lib.rs` 已有 `parse_version` / `compare_versions` / `is_valid_id` (R23 LOCKED)
//! - 0 改 `Skill` / `Registry` / workspace 1.0.0
//! - 新模块是 `parse_version` 的超集, 不替换
//!
//! **借鉴锚 (S-6)**:
//! - semver.org spec v2.0.0 (字段 1:1)
//! - cargo `semver` crate API (但 0 dep, 自实现)
//! - npm `semver` (compare + prerelease ordering)

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::SkillError;
use crate::SkillResult;

// ============================================================
// 错误
// ============================================================

/// **R107 严格 semver 错误** (per semver.org spec §2 MAJOR.MINOR.PATCH)
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SemverStrictError {
    /// 不是 3 段
    #[error("semver `{0}` 必须含 3 段 (MAJOR.MINOR.PATCH)")]
    NotThreeSegments(String),
    /// 某段不是非负整数 / 含前导 0
    #[error("semver `{0}` 第 {1} 段 `{2}` 非合法非负整数 (per §2)")]
    InvalidSegment(String, usize, String),
    /// pre-release 标识符非法 (per §9: 只允许 [0-9A-Za-z-], 不允许空段)
    #[error("semver `{0}` pre-release `{1}` 非法 (per §9)")]
    InvalidPrerelease(String, String),
    /// build metadata 非法 (per §10: 只允许 [0-9A-Za-z-], 不允许空段)
    #[error("semver `{0}` build metadata `{1}` 非法 (per §10)")]
    InvalidBuildMetadata(String, String),
    /// 整体空
    #[error("semver 字符串为空")]
    Empty,
}

// ============================================================
// Semver struct
// ============================================================

/// **严格 semver 2.0.0 版本**
///
/// 字段级参考 <https://semver.org/spec/v2.0.0.html> §2:
/// ```text
/// MAJOR.MINOR.PATCH[-PRERELEASE][+BUILDMETADATA]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Semver {
    /// 主版本号 (大版本变更, 0 起步)
    pub major: u32,
    /// 次版本号 (新功能但向后兼容)
    pub minor: u32,
    /// 补丁版本号 (bug fix)
    pub patch: u32,
    /// pre-release 标识符 (e.g. "alpha.1", "rc.1", "x.7.z.92")
    pub pre_release: Option<String>,
    /// build metadata (e.g. "20130313144700", "exp.sha.5114f85")
    pub build_metadata: Option<String>,
}

impl Semver {
    /// 主版本 (per spec §11: pre-release < release)
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    /// 是否有 build metadata
    pub fn has_build_metadata(&self) -> bool {
        self.build_metadata.is_some()
    }

    /// 序列化回 semver.org 字符串 (per spec §2 完整格式)
    pub fn to_semver_string(&self) -> String {
        let mut s = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(pr) = &self.pre_release {
            s.push('-');
            s.push_str(pr);
        }
        if let Some(bm) = &self.build_metadata {
            s.push('+');
            s.push_str(bm);
        }
        s
    }
}

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_semver_string())
    }
}

// ============================================================
// 解析
// ============================================================

/// **严格按 semver.org spec 解析版本号**
///
/// 接受: "1.0.0", "1.0.0-alpha.1", "1.0.0-alpha.1+exp.sha.5114f85", "1.0.0+20130313144700"
/// 拒绝: "1.0", "01.0.0" (前导 0), "1.0.0-" (空 prerelease), "1.0.0+" (空 build),
///        "1.0.0-alpha..1" (空段), "1.0.0-alpha+beta" (build 段含 +)
///
/// per spec §2 + §9 + §10
pub fn parse_strict(v: &str) -> Result<Semver, SemverStrictError> {
    if v.is_empty() {
        return Err(SemverStrictError::Empty);
    }

    // 1. 切 build metadata (第 1 个 '+' 后)
    let (core, build_metadata) = match v.find('+') {
        Some(idx) => {
            let core = &v[..idx];
            let bm = &v[idx + 1..];
            if bm.is_empty() {
                return Err(SemverStrictError::InvalidBuildMetadata(
                    v.to_string(),
                    "(empty)".to_string(),
                ));
            }
            (core, Some(bm.to_string()))
        }
        None => (v, None),
    };

    // 2. 切 pre-release (第 1 个 '-' 在 core 部分, 但只在所有段都解析完后判)
    //    注: 第 1 段是 major, 不允许有 '-', 所以第 1 个 '-' 一定是 pre-release 起点
    let (core, pre_release) = match core.find('-') {
        Some(idx) => {
            let pr = &core[idx + 1..];
            let core = &core[..idx];
            if pr.is_empty() {
                return Err(SemverStrictError::InvalidPrerelease(
                    v.to_string(),
                    "(empty)".to_string(),
                ));
            }
            (core, Some(pr.to_string()))
        }
        None => (core, None),
    };

    // 3. 切 3 段 (按 '.')
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(SemverStrictError::NotThreeSegments(v.to_string()));
    }

    // 4. 解析 major / minor / patch
    let major = parse_numeric_segment(v, 0, parts[0])?;
    let minor = parse_numeric_segment(v, 1, parts[1])?;
    let patch = parse_numeric_segment(v, 2, parts[2])?;

    // 5. 校验 pre-release 标识符 (per §9)
    if let Some(pr) = &pre_release {
        validate_dot_identifier(pr, v, true)?;
    }

    // 6. 校验 build metadata (per §10)
    if let Some(bm) = &build_metadata {
        validate_dot_identifier(bm, v, false)?;
    }

    Ok(Semver {
        major,
        minor,
        patch,
        pre_release,
        build_metadata,
    })
}

/// **解析一段 (major/minor/patch)**: 必须是 0+, 不允许前导 0 (除 "0" 本身)
fn parse_numeric_segment(full: &str, idx: usize, seg: &str) -> Result<u32, SemverStrictError> {
    if seg.is_empty() {
        return Err(SemverStrictError::InvalidSegment(
            full.to_string(),
            idx + 1,
            seg.to_string(),
        ));
    }
    // 不允许前导 0 (除 "0" 本身) — per spec §2: "Numeric identifiers MUST NOT include leading zeroes"
    if seg.len() > 1 && seg.starts_with('0') {
        return Err(SemverStrictError::InvalidSegment(
            full.to_string(),
            idx + 1,
            seg.to_string(),
        ));
    }
    seg.parse::<u32>()
        .map_err(|_| SemverStrictError::InvalidSegment(full.to_string(), idx + 1, seg.to_string()))
}

/// **校验 "." 分隔的标识符** (pre-release 或 build metadata)
///
/// per §9: 标识符 MUST comprise only ASCII alphanumerics and hyphens [0-9A-Za-z-]
///        标识符 MUST NOT be empty
///        Numeric identifiers MUST NOT include leading zeroes
/// per §10: 同 §9, 但没有 numeric leading zero 限制
fn validate_dot_identifier(
    s: &str,
    full: &str,
    is_pre_release: bool,
) -> Result<(), SemverStrictError> {
    for (i, id) in s.split('.').enumerate() {
        if id.is_empty() {
            return Err(if is_pre_release {
                SemverStrictError::InvalidPrerelease(full.to_string(), s.to_string())
            } else {
                SemverStrictError::InvalidBuildMetadata(full.to_string(), s.to_string())
            });
        }
        for c in id.chars() {
            if !(c.is_ascii_alphanumeric() || c == '-') {
                return Err(if is_pre_release {
                    SemverStrictError::InvalidPrerelease(full.to_string(), s.to_string())
                } else {
                    SemverStrictError::InvalidBuildMetadata(full.to_string(), s.to_string())
                });
            }
        }
        // pre-release numeric 段不允许前导 0 (除 "0" 本身) — per §9
        if is_pre_release
            && id.len() > 1
            && id.starts_with('0')
            && id.chars().all(|c| c.is_ascii_digit())
        {
            return Err(SemverStrictError::InvalidPrerelease(
                full.to_string(),
                s.to_string(),
            ));
        }
        // 静默 unused
        let _ = i;
    }
    Ok(())
}

/// **快捷判断: 字符串是否为合法 semver.org 严格版本**
pub fn is_valid_strict(v: &str) -> bool {
    parse_strict(v).is_ok()
}

// ============================================================
// 比较 (per spec §11)
// ============================================================

/// **严格 semver 比较** (per spec §11)
///
/// 规则:
/// 1. 先比较 major.minor.patch (numeric)
/// 2. 如果 major.minor.patch 相同, 都有 pre_release, 按 pre_release 标识符比较
///    - 数字段 vs 非数字段: 数字段优先级低
///    - 数字段: numeric 比较
///    - 非数字段: ASCII 比较
///    - 短者优先 (1.0.0-alpha < 1.0.0-alpha.1)
/// 3. 有 pre_release 的版本 < 无 pre_release 的版本
/// 4. build metadata 忽略 (per §10)
///
/// 返回: -1 (a < b), 0 (a == b), +1 (a > b)
pub fn compare_strict(a: &Semver, b: &Semver) -> i32 {
    // 1. major.minor.patch
    let mmp = (a.major, a.minor, a.patch).cmp(&(b.major, b.minor, b.patch));
    if mmp != std::cmp::Ordering::Equal {
        return match mmp {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
        };
    }

    // 2. pre-release 优先级
    match (&a.pre_release, &b.pre_release) {
        (None, None) => 0,     // build metadata 差异忽略
        (Some(_), None) => -1, // pre-release < release
        (None, Some(_)) => 1,  // release > pre-release
        (Some(apr), Some(bpr)) => compare_prerelease(apr, bpr),
    }
}

/// **pre-release 标识符比较** (per spec §11)
fn compare_prerelease(a: &str, b: &str) -> i32 {
    let a_ids: Vec<&str> = a.split('.').collect();
    let b_ids: Vec<&str> = b.split('.').collect();
    let n = a_ids.len().min(b_ids.len());

    for i in 0..n {
        let cmp = compare_prerelease_identifier(a_ids[i], b_ids[i]);
        if cmp != 0 {
            return cmp;
        }
    }

    // 短者优先 (per spec §11: "A larger set of pre-release fields has a higher precedence than a smaller set")
    match a_ids.len().cmp(&b_ids.len()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

/// **单个 pre-release 标识符比较**
///
/// 数字 vs 非数字: 数字小
/// 数字 vs 数字: numeric
/// 非数字 vs 非数字: ASCII
fn compare_prerelease_identifier(a: &str, b: &str) -> i32 {
    let a_is_num = a.chars().all(|c| c.is_ascii_digit());
    let b_is_num = b.chars().all(|c| c.is_ascii_digit());

    match (a_is_num, b_is_num) {
        (true, false) => -1,
        (false, true) => 1,
        (true, true) => {
            // numeric 比较
            let an = a.parse::<u64>().unwrap_or(0);
            let bn = b.parse::<u64>().unwrap_or(0);
            match an.cmp(&bn) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
            }
        }
        (false, false) => {
            // ASCII 比较
            match a.cmp(b) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
            }
        }
    }
}

// ============================================================
// 与 R23 已有 parse_version / compare_versions 的桥接
// ============================================================

/// **R107 严格 semver 包装 R23 简单 3 段比较** (无 pre-release)
///
/// 适用于: 需要 strict 解析 (拒绝 "1.0") 但又只用 3 段比较的场景
pub fn parse_strict_then_compare_3seg(a: &str, b: &str) -> SkillResult<i32> {
    let sa = parse_strict(a).map_err(|_| SkillError::InvalidVersion(a.to_string()))?;
    let sb = parse_strict(b).map_err(|_| SkillError::InvalidVersion(b.to_string()))?;
    Ok(compare_strict(&sa, &sb))
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_3_segment() {
        let s = parse_strict("1.2.3").unwrap();
        assert_eq!(s.major, 1);
        assert_eq!(s.minor, 2);
        assert_eq!(s.patch, 3);
        assert!(s.pre_release.is_none());
        assert!(s.build_metadata.is_none());
    }

    #[test]
    fn parse_with_prerelease() {
        let s = parse_strict("1.0.0-alpha.1").unwrap();
        assert_eq!(s.major, 1);
        assert_eq!(s.pre_release.as_deref(), Some("alpha.1"));
        assert!(s.build_metadata.is_none());
        assert!(s.is_pre_release());
    }

    #[test]
    fn parse_with_build_metadata() {
        let s = parse_strict("1.0.0+20130313144700").unwrap();
        assert_eq!(s.patch, 0);
        assert_eq!(s.build_metadata.as_deref(), Some("20130313144700"));
        assert!(!s.is_pre_release());
        assert!(s.has_build_metadata());
    }

    #[test]
    fn parse_with_prerelease_and_build() {
        let s = parse_strict("1.0.0-alpha.1+exp.sha.5114f85").unwrap();
        assert_eq!(s.pre_release.as_deref(), Some("alpha.1"));
        assert_eq!(s.build_metadata.as_deref(), Some("exp.sha.5114f85"));
    }

    #[test]
    fn parse_zero_version() {
        let s = parse_strict("0.0.0").unwrap();
        assert_eq!(s.major, 0);
    }

    #[test]
    fn parse_large_numbers() {
        let s = parse_strict("999.888.777").unwrap();
        assert_eq!(s.major, 999);
        assert_eq!(s.minor, 888);
        assert_eq!(s.patch, 777);
    }

    #[test]
    fn parse_rejects_two_segments() {
        assert_eq!(
            parse_strict("1.0").unwrap_err(),
            SemverStrictError::NotThreeSegments("1.0".to_string())
        );
    }

    #[test]
    fn parse_rejects_four_segments() {
        assert!(parse_strict("1.2.3.4").is_err());
    }

    #[test]
    fn parse_rejects_leading_zero() {
        // "01.0.0" 不允许
        let err = parse_strict("01.0.0").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidSegment(..)));
    }

    #[test]
    fn parse_rejects_leading_zero_patch() {
        let err = parse_strict("1.0.01").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidSegment(..)));
    }

    #[test]
    fn parse_rejects_empty_string() {
        assert_eq!(parse_strict("").unwrap_err(), SemverStrictError::Empty);
    }

    #[test]
    fn parse_rejects_empty_prerelease() {
        let err = parse_strict("1.0.0-").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidPrerelease(..)));
    }

    #[test]
    fn parse_rejects_empty_build() {
        let err = parse_strict("1.0.0+").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidBuildMetadata(..)));
    }

    #[test]
    fn parse_rejects_invalid_prerelease_chars() {
        let err = parse_strict("1.0.0-alpha_1").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidPrerelease(..)));
    }

    #[test]
    fn parse_rejects_invalid_build_chars() {
        let err = parse_strict("1.0.0+build@1").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidBuildMetadata(..)));
    }

    #[test]
    fn parse_rejects_empty_prerelease_segment() {
        // "1.0.0-alpha..1" 中间有空段
        let err = parse_strict("1.0.0-alpha..1").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidPrerelease(..)));
    }

    #[test]
    fn parse_rejects_leading_zero_in_prerelease_numeric() {
        // "1.0.0-alpha.01" prerelease numeric 段不允许前导 0
        let err = parse_strict("1.0.0-alpha.01").unwrap_err();
        assert!(matches!(err, SemverStrictError::InvalidPrerelease(..)));
    }

    #[test]
    fn parse_accepts_alphanumeric_prerelease() {
        let s = parse_strict("1.0.0-x.7.z.92").unwrap();
        assert_eq!(s.pre_release.as_deref(), Some("x.7.z.92"));
    }

    #[test]
    fn parse_accepts_hyphens_in_prerelease() {
        let s = parse_strict("1.0.0-x-y-z").unwrap();
        assert_eq!(s.pre_release.as_deref(), Some("x-y-z"));
    }

    #[test]
    fn is_valid_strict_basic() {
        assert!(is_valid_strict("1.0.0"));
        assert!(is_valid_strict("1.0.0-alpha"));
        assert!(is_valid_strict("1.0.0+build"));
        assert!(!is_valid_strict("1.0"));
        assert!(!is_valid_strict(""));
        assert!(!is_valid_strict("1.0.0-"));
    }

    // ----- compare_strict -----

    #[test]
    fn compare_basic_major() {
        let a = parse_strict("2.0.0").unwrap();
        let b = parse_strict("1.0.0").unwrap();
        assert_eq!(compare_strict(&a, &b), 1);
        assert_eq!(compare_strict(&b, &a), -1);
    }

    #[test]
    fn compare_basic_minor() {
        let a = parse_strict("1.2.0").unwrap();
        let b = parse_strict("1.1.0").unwrap();
        assert_eq!(compare_strict(&a, &b), 1);
    }

    #[test]
    fn compare_basic_patch() {
        let a = parse_strict("1.0.2").unwrap();
        let b = parse_strict("1.0.1").unwrap();
        assert_eq!(compare_strict(&a, &b), 1);
    }

    #[test]
    fn compare_equal_versions() {
        let a = parse_strict("1.0.0").unwrap();
        let b = parse_strict("1.0.0").unwrap();
        assert_eq!(compare_strict(&a, &b), 0);
    }

    #[test]
    fn compare_prerelease_less_than_release() {
        let a = parse_strict("1.0.0-alpha").unwrap();
        let b = parse_strict("1.0.0").unwrap();
        assert_eq!(compare_strict(&a, &b), -1);
        assert_eq!(compare_strict(&b, &a), 1);
    }

    #[test]
    fn compare_prerelease_examples_from_spec() {
        // 来自 semver.org §11:
        // 1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta
        //   < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0
        let versions = vec![
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        let semvers: Vec<Semver> = versions.iter().map(|v| parse_strict(v).unwrap()).collect();
        for i in 0..semvers.len() - 1 {
            assert_eq!(
                compare_strict(&semvers[i], &semvers[i + 1]),
                -1,
                "{} should be < {}",
                versions[i],
                versions[i + 1]
            );
        }
    }

    #[test]
    fn compare_prerelease_numeric_vs_alpha() {
        // 数字段 vs 非数字段: 数字小
        let a = parse_strict("1.0.0-1").unwrap();
        let b = parse_strict("1.0.0-alpha").unwrap();
        assert_eq!(compare_strict(&a, &b), -1);
    }

    #[test]
    fn compare_prerelease_numeric_less_than() {
        let a = parse_strict("1.0.0-alpha.1").unwrap();
        let b = parse_strict("1.0.0-alpha.2").unwrap();
        assert_eq!(compare_strict(&a, &b), -1);
    }

    #[test]
    fn compare_prerelease_alpha_less_than_beta() {
        let a = parse_strict("1.0.0-alpha").unwrap();
        let b = parse_strict("1.0.0-beta").unwrap();
        assert_eq!(compare_strict(&a, &b), -1);
    }

    #[test]
    fn compare_ignores_build_metadata() {
        // per spec §10: build metadata MUST be ignored when determining version precedence
        let a = parse_strict("1.0.0+build1").unwrap();
        let b = parse_strict("1.0.0+build2").unwrap();
        assert_eq!(compare_strict(&a, &b), 0);
    }

    #[test]
    fn compare_ignores_build_metadata_but_considers_prerelease() {
        let a = parse_strict("1.0.0-alpha+build1").unwrap();
        let b = parse_strict("1.0.0+build2").unwrap();
        assert_eq!(compare_strict(&a, &b), -1);
    }

    #[test]
    fn compare_short_prerelease_less_than_longer() {
        // 1.0.0-alpha < 1.0.0-alpha.1 (per spec §11)
        let a = parse_strict("1.0.0-alpha").unwrap();
        let b = parse_strict("1.0.0-alpha.1").unwrap();
        assert_eq!(compare_strict(&a, &b), -1);
    }

    // ----- to_semver_string round-trip -----

    #[test]
    fn round_trip_basic() {
        let v = "1.2.3";
        let s = parse_strict(v).unwrap();
        assert_eq!(s.to_semver_string(), v);
    }

    #[test]
    fn round_trip_with_prerelease() {
        let v = "1.0.0-alpha.1";
        let s = parse_strict(v).unwrap();
        assert_eq!(s.to_semver_string(), v);
    }

    #[test]
    fn round_trip_with_build() {
        let v = "1.0.0+20130313144700";
        let s = parse_strict(v).unwrap();
        assert_eq!(s.to_semver_string(), v);
    }

    #[test]
    fn round_trip_full() {
        let v = "1.0.0-alpha.1+exp.sha.5114f85";
        let s = parse_strict(v).unwrap();
        assert_eq!(s.to_semver_string(), v);
    }

    #[test]
    fn display_trait_works() {
        let s = parse_strict("1.0.0-alpha").unwrap();
        assert_eq!(format!("{}", s), "1.0.0-alpha");
    }

    // ----- parse_strict_then_compare_3seg bridge -----

    #[test]
    fn bridge_3seg_works() {
        assert_eq!(
            parse_strict_then_compare_3seg("1.0.0", "1.0.1").unwrap(),
            -1
        );
        assert_eq!(parse_strict_then_compare_3seg("2.0.0", "1.9.9").unwrap(), 1);
        assert_eq!(parse_strict_then_compare_3seg("1.0.0", "1.0.0").unwrap(), 0);
    }

    #[test]
    fn bridge_rejects_invalid() {
        assert!(parse_strict_then_compare_3seg("1.0", "1.0.0").is_err());
    }
}
