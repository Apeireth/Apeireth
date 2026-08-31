//! Strict SemVer 2.0.0 parse and compare, recovered from
//! `legacy/canonical/apeireth-skills/src/semver_strict.rs`.
//!
//! Plugin manifests store version as a free-form string. This helper is for
//! callers that need the spec: three numeric segments, optional pre-release,
//! optional build metadata, no leading zeroes, and §11 precedence. Build
//! metadata is ignored when comparing.
//!
//! Zero extra dependencies. Does not replace [`PluginManifest::version`].

use std::fmt;

/// Strict SemVer 2.0.0 parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemverError {
    /// The input was empty.
    Empty,
    /// Not exactly `MAJOR.MINOR.PATCH` in the core (before `-` / `+`).
    NotThreeSegments(String),
    /// A numeric core segment is empty, has a leading zero, or is not an integer.
    InvalidSegment {
        /// Original input.
        input: String,
        /// 1-based segment index.
        index: usize,
        /// Offending segment.
        segment: String,
    },
    /// Pre-release identifier is empty, has illegal characters, or a leading zero.
    InvalidPrerelease {
        /// Original input.
        input: String,
        /// Offending pre-release string.
        prerelease: String,
    },
    /// Build metadata is empty or has illegal characters.
    InvalidBuildMetadata {
        /// Original input.
        input: String,
        /// Offending build-metadata string.
        build: String,
    },
}

impl fmt::Display for SemverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "semver string is empty"),
            Self::NotThreeSegments(v) => {
                write!(
                    f,
                    "semver `{v}` must contain three segments (MAJOR.MINOR.PATCH)"
                )
            }
            Self::InvalidSegment {
                input,
                index,
                segment,
            } => write!(
                f,
                "semver `{input}` segment {index} `{segment}` is not a valid non-negative integer"
            ),
            Self::InvalidPrerelease { input, prerelease } => {
                write!(f, "semver `{input}` pre-release `{prerelease}` is invalid")
            }
            Self::InvalidBuildMetadata { input, build } => {
                write!(f, "semver `{input}` build metadata `{build}` is invalid")
            }
        }
    }
}

impl std::error::Error for SemverError {}

/// A SemVer 2.0.0 version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Semver {
    /// Major.
    pub major: u32,
    /// Minor.
    pub minor: u32,
    /// Patch.
    pub patch: u32,
    /// Pre-release identifiers joined by `.`, if any.
    pub pre_release: Option<String>,
    /// Build metadata, if any. Ignored by [`compare`].
    pub build_metadata: Option<String>,
}

impl Semver {
    /// Whether this is a pre-release.
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    /// Whether build metadata is present.
    pub fn has_build_metadata(&self) -> bool {
        self.build_metadata.is_some()
    }

    /// Serialize back to the spec string.
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

impl fmt::Display for Semver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_semver_string())
    }
}

impl PartialOrd for Semver {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Semver {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match compare(self, other) {
            -1 => std::cmp::Ordering::Less,
            1 => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    }
}

/// Parse a SemVer 2.0.0 string.
///
/// Accepts `1.0.0`, `1.0.0-alpha.1`, `1.0.0+20130313144700`,
/// `1.0.0-alpha.1+exp.sha.5114f85`. Rejects `1.0`, `01.0.0`, `1.0.0-`,
/// `1.0.0+`, `1.0.0-alpha..1`, `1.0.0-alpha.01`.
pub fn parse(v: &str) -> Result<Semver, SemverError> {
    if v.is_empty() {
        return Err(SemverError::Empty);
    }

    let (core, build_metadata) = match v.find('+') {
        Some(idx) => {
            let bm = &v[idx + 1..];
            if bm.is_empty() {
                return Err(SemverError::InvalidBuildMetadata {
                    input: v.to_string(),
                    build: "(empty)".to_string(),
                });
            }
            (&v[..idx], Some(bm.to_string()))
        }
        None => (v, None),
    };

    let (core, pre_release) = match core.find('-') {
        Some(idx) => {
            let pr = &core[idx + 1..];
            if pr.is_empty() {
                return Err(SemverError::InvalidPrerelease {
                    input: v.to_string(),
                    prerelease: "(empty)".to_string(),
                });
            }
            (&core[..idx], Some(pr.to_string()))
        }
        None => (core, None),
    };

    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(SemverError::NotThreeSegments(v.to_string()));
    }

    let major = parse_numeric_segment(v, 0, parts[0])?;
    let minor = parse_numeric_segment(v, 1, parts[1])?;
    let patch = parse_numeric_segment(v, 2, parts[2])?;

    if let Some(pr) = &pre_release {
        validate_dot_identifier(pr, v, true)?;
    }
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

fn parse_numeric_segment(full: &str, idx: usize, seg: &str) -> Result<u32, SemverError> {
    if seg.is_empty() {
        return Err(SemverError::InvalidSegment {
            input: full.to_string(),
            index: idx + 1,
            segment: seg.to_string(),
        });
    }
    if seg.len() > 1 && seg.starts_with('0') {
        return Err(SemverError::InvalidSegment {
            input: full.to_string(),
            index: idx + 1,
            segment: seg.to_string(),
        });
    }
    seg.parse::<u32>().map_err(|_| SemverError::InvalidSegment {
        input: full.to_string(),
        index: idx + 1,
        segment: seg.to_string(),
    })
}

fn validate_dot_identifier(s: &str, full: &str, is_pre_release: bool) -> Result<(), SemverError> {
    for id in s.split('.') {
        if id.is_empty() {
            return Err(if is_pre_release {
                SemverError::InvalidPrerelease {
                    input: full.to_string(),
                    prerelease: s.to_string(),
                }
            } else {
                SemverError::InvalidBuildMetadata {
                    input: full.to_string(),
                    build: s.to_string(),
                }
            });
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(if is_pre_release {
                SemverError::InvalidPrerelease {
                    input: full.to_string(),
                    prerelease: s.to_string(),
                }
            } else {
                SemverError::InvalidBuildMetadata {
                    input: full.to_string(),
                    build: s.to_string(),
                }
            });
        }
        if is_pre_release
            && id.len() > 1
            && id.starts_with('0')
            && id.chars().all(|c| c.is_ascii_digit())
        {
            return Err(SemverError::InvalidPrerelease {
                input: full.to_string(),
                prerelease: s.to_string(),
            });
        }
    }
    Ok(())
}

/// Whether `v` is a valid strict SemVer 2.0.0 string.
pub fn is_valid(v: &str) -> bool {
    parse(v).is_ok()
}

/// Compare two versions per spec §11. Returns `-1` / `0` / `+1`.
///
/// Build metadata is ignored. A pre-release is less than the corresponding
/// release. Numeric pre-release identifiers have lower precedence than
/// non-numeric ones.
pub fn compare(a: &Semver, b: &Semver) -> i32 {
    let mmp = (a.major, a.minor, a.patch).cmp(&(b.major, b.minor, b.patch));
    if mmp != std::cmp::Ordering::Equal {
        return match mmp {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
        };
    }
    match (&a.pre_release, &b.pre_release) {
        (None, None) => 0,
        (Some(_), None) => -1,
        (None, Some(_)) => 1,
        (Some(apr), Some(bpr)) => compare_prerelease(apr, bpr),
    }
}

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
    match a_ids.len().cmp(&b_ids.len()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

fn compare_prerelease_identifier(a: &str, b: &str) -> i32 {
    let a_is_num = !a.is_empty() && a.chars().all(|c| c.is_ascii_digit());
    let b_is_num = !b.is_empty() && b.chars().all(|c| c.is_ascii_digit());
    match (a_is_num, b_is_num) {
        (true, false) => -1,
        (false, true) => 1,
        (true, true) => {
            let an = a.parse::<u64>().unwrap_or(0);
            let bn = b.parse::<u64>().unwrap_or(0);
            match an.cmp(&bn) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
            }
        }
        (false, false) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
        },
    }
}

/// Parse two strings and compare them. Invalid input is an error, not a panic.
pub fn compare_str(a: &str, b: &str) -> Result<i32, SemverError> {
    Ok(compare(&parse(a)?, &parse(b)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_three_segment() {
        let s = parse("1.2.3").unwrap();
        assert_eq!(s.major, 1);
        assert_eq!(s.minor, 2);
        assert_eq!(s.patch, 3);
        assert!(s.pre_release.is_none());
        assert!(s.build_metadata.is_none());
    }

    #[test]
    fn parse_with_prerelease_and_build() {
        let s = parse("1.0.0-alpha.1+exp.sha.5114f85").unwrap();
        assert_eq!(s.pre_release.as_deref(), Some("alpha.1"));
        assert_eq!(s.build_metadata.as_deref(), Some("exp.sha.5114f85"));
        assert!(s.is_pre_release());
        assert!(s.has_build_metadata());
    }

    #[test]
    fn parse_zero_and_large() {
        assert_eq!(parse("0.0.0").unwrap().major, 0);
        let s = parse("999.888.777").unwrap();
        assert_eq!((s.major, s.minor, s.patch), (999, 888, 777));
    }

    #[test]
    fn parse_rejects_two_and_four_segments() {
        assert!(matches!(
            parse("1.0"),
            Err(SemverError::NotThreeSegments(_))
        ));
        assert!(parse("1.2.3.4").is_err());
    }

    #[test]
    fn parse_rejects_leading_zero() {
        assert!(matches!(
            parse("01.0.0"),
            Err(SemverError::InvalidSegment { .. })
        ));
        assert!(matches!(
            parse("1.0.01"),
            Err(SemverError::InvalidSegment { .. })
        ));
    }

    #[test]
    fn parse_rejects_empty_and_empty_parts() {
        assert_eq!(parse("").unwrap_err(), SemverError::Empty);
        assert!(matches!(
            parse("1.0.0-"),
            Err(SemverError::InvalidPrerelease { .. })
        ));
        assert!(matches!(
            parse("1.0.0+"),
            Err(SemverError::InvalidBuildMetadata { .. })
        ));
        assert!(matches!(
            parse("1.0.0-alpha..1"),
            Err(SemverError::InvalidPrerelease { .. })
        ));
    }

    #[test]
    fn parse_rejects_illegal_chars_and_prerelease_leading_zero() {
        assert!(matches!(
            parse("1.0.0-alpha_1"),
            Err(SemverError::InvalidPrerelease { .. })
        ));
        assert!(matches!(
            parse("1.0.0+build@1"),
            Err(SemverError::InvalidBuildMetadata { .. })
        ));
        assert!(matches!(
            parse("1.0.0-alpha.01"),
            Err(SemverError::InvalidPrerelease { .. })
        ));
    }

    #[test]
    fn parse_accepts_hyphens_and_alphanumeric_prerelease() {
        assert_eq!(
            parse("1.0.0-x.7.z.92").unwrap().pre_release.as_deref(),
            Some("x.7.z.92")
        );
        assert_eq!(
            parse("1.0.0-x-y-z").unwrap().pre_release.as_deref(),
            Some("x-y-z")
        );
    }

    #[test]
    fn is_valid_samples() {
        assert!(is_valid("1.0.0"));
        assert!(is_valid("1.0.0-alpha"));
        assert!(is_valid("1.0.0+build"));
        assert!(!is_valid("1.0"));
        assert!(!is_valid(""));
        assert!(!is_valid("1.0.0-"));
    }

    #[test]
    fn compare_major_minor_patch() {
        assert_eq!(
            compare(&parse("2.0.0").unwrap(), &parse("1.0.0").unwrap()),
            1
        );
        assert_eq!(
            compare(&parse("1.2.0").unwrap(), &parse("1.1.0").unwrap()),
            1
        );
        assert_eq!(
            compare(&parse("1.0.2").unwrap(), &parse("1.0.1").unwrap()),
            1
        );
        assert_eq!(
            compare(&parse("1.0.0").unwrap(), &parse("1.0.0").unwrap()),
            0
        );
    }

    #[test]
    fn compare_prerelease_less_than_release() {
        assert_eq!(
            compare(&parse("1.0.0-alpha").unwrap(), &parse("1.0.0").unwrap()),
            -1
        );
    }

    #[test]
    fn compare_spec_section_11_examples() {
        let versions = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        let semvers: Vec<Semver> = versions.iter().map(|v| parse(v).unwrap()).collect();
        for i in 0..semvers.len() - 1 {
            assert_eq!(
                compare(&semvers[i], &semvers[i + 1]),
                -1,
                "{} should be < {}",
                versions[i],
                versions[i + 1]
            );
        }
    }

    #[test]
    fn compare_numeric_prerelease_vs_alpha() {
        assert_eq!(
            compare(&parse("1.0.0-1").unwrap(), &parse("1.0.0-alpha").unwrap()),
            -1
        );
    }

    #[test]
    fn compare_ignores_build_metadata() {
        assert_eq!(
            compare(
                &parse("1.0.0+build1").unwrap(),
                &parse("1.0.0+build2").unwrap()
            ),
            0
        );
        assert_eq!(
            compare(
                &parse("1.0.0-alpha+build1").unwrap(),
                &parse("1.0.0+build2").unwrap()
            ),
            -1
        );
    }

    #[test]
    fn round_trip() {
        for v in [
            "1.2.3",
            "1.0.0-alpha.1",
            "1.0.0+20130313144700",
            "1.0.0-alpha.1+exp.sha.5114f85",
        ] {
            assert_eq!(parse(v).unwrap().to_semver_string(), v);
        }
        assert_eq!(format!("{}", parse("1.0.0-alpha").unwrap()), "1.0.0-alpha");
    }

    #[test]
    fn compare_str_rejects_invalid() {
        assert!(compare_str("1.0", "1.0.0").is_err());
        assert_eq!(compare_str("1.0.0", "1.0.1").unwrap(), -1);
        assert_eq!(compare_str("2.0.0", "1.9.9").unwrap(), 1);
    }

    #[test]
    fn ord_matches_compare() {
        let a = parse("1.0.0-alpha").unwrap();
        let b = parse("1.0.0").unwrap();
        assert!(a < b);
        assert!(b > a);
    }
}
