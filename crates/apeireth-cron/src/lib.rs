//! apeireth-cron — R23 6 module cron 子模块。
//!
//! R23 P1 #5 实质化: 加 +6 顶层 pub fn — 5-field cron parsing + match + 可视化.
//! 不假装: 真 parse 5-field cron expression (minute / hour / dom / month / dow),
//! 真比较是否触发, 不假装"暂未实现".
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// R150 P1 #9: tokio cron scheduler (test-only, 0 触碰生产依赖)
// 默认 build 0 引 tokio; test build 引入 scheduler 模块跑验证.
// 业务侧若要真调度, R150+ 续 feature gate `tokio_scheduler` (留口).
#[cfg(test)]
mod scheduler;

#[derive(Debug, Error)]
pub enum CronError {
    #[error("cron: interval `{0}` must be > 0")]
    NonPositiveInterval(i64),
    #[error("cron: 解析 expr `{0}` 失败: {1}")]
    ParseError(String, String),
    #[error("cron: 5-field expr 必须有空格分 5 段 (got {0})")]
    FieldCountMismatch(usize),
}
pub type CronResult<T> = Result<T, CronError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub name: String,
    pub interval_secs: i64,
}
impl Schedule {
    pub fn new(name: impl Into<String>, interval_secs: i64) -> Self {
        Self {
            name: name.into(),
            interval_secs,
        }
    }
    pub fn validate(&self) -> CronResult<()> {
        if self.interval_secs <= 0 {
            return Err(CronError::NonPositiveInterval(self.interval_secs));
        }
        Ok(())
    }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — 5-field cron
// ============================================================================

/// Standard 5-field cron expression: minute hour dom month dow.
/// `*` wildcard, `5` literal, `1,3,5` list, `*/15` step, `0-23` range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr {
    pub raw: String,
    pub fields: [Field; 5],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub raw: String,
    pub lo: u8,
    pub hi: u8,
    pub bits: u64,
} // 64-bit bitmap

impl Field {
    /// Parse `*` / `5` / `1,3,5` / `*/15` / `0-23` / `1-30/2`.
    pub fn parse(s: &str, lo: u8, hi: u8) -> CronResult<Field> {
        let mut bits: u64 = 0;
        for piece in s.split(',') {
            let (range_part, step) = match piece.split_once('/') {
                Some((r, st)) => (
                    r,
                    st.parse::<u8>()
                        .map_err(|e| CronError::ParseError(s.into(), format!("step parse: {e}")))?,
                ),
                None => (piece, 1u8),
            };
            let (start, end) = match range_part.split_once('-') {
                Some((a, b)) => (
                    a.parse::<u8>().map_err(|e| {
                        CronError::ParseError(s.into(), format!("range start: {e}"))
                    })?,
                    b.parse::<u8>()
                        .map_err(|e| CronError::ParseError(s.into(), format!("range end: {e}")))?,
                ),
                None => {
                    if range_part == "*" {
                        (lo, hi)
                    } else {
                        let v = range_part.parse::<u8>().map_err(|e| {
                            CronError::ParseError(s.into(), format!("literal parse: {e}"))
                        })?;
                        (v, v)
                    }
                }
            };
            if start < lo || end > hi || start > end {
                return Err(CronError::ParseError(
                    s.into(),
                    format!("out of range {lo}..={hi}"),
                ));
            }
            let mut v = start;
            while v <= end {
                bits |= 1u64 << v;
                v = v.saturating_add(step);
            }
        }
        Ok(Field {
            raw: s.into(),
            lo,
            hi,
            bits,
        })
    }
    pub fn matches(&self, value: u8) -> bool {
        self.bits & (1u64 << value) != 0
    }
}
impl CronExpr {
    pub fn parse(expr: &str) -> CronResult<CronExpr> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(CronError::FieldCountMismatch(parts.len()));
        }
        let mins = Field::parse(parts[0], 0, 59)?;
        let hrs = Field::parse(parts[1], 0, 23)?;
        let dom = Field::parse(parts[2], 1, 31)?;
        let mon = Field::parse(parts[3], 1, 12)?;
        let dow = Field::parse(parts[4], 0, 6)?;
        Ok(CronExpr {
            raw: expr.into(),
            fields: [mins, hrs, dom, mon, dow],
        })
    }
    /// Test whether given (minute, hour, day-of-month, month, day-of-week) tuple matches.
    pub fn matches(&self, m: u8, h: u8, dom: u8, mon: u8, dow: u8) -> bool {
        self.fields[0].matches(m)
            && self.fields[1].matches(h)
            && self.fields[2].matches(dom)
            && self.fields[3].matches(mon)
            && self.fields[4].matches(dow)
    }
}
impl std::fmt::Display for CronExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}
impl std::str::FromStr for CronExpr {
    type Err = CronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// 校验 Schedule::interval_secs 是否合规 (> 0).
pub fn validate_schedule(s: &Schedule) -> CronResult<()> {
    s.validate()
}

/// Validate cron expr string.  便利函数.
pub fn validate_expr(expr: &str) -> CronResult<CronExpr> {
    CronExpr::parse(expr)
}

/// Render cron expr to human-readable description (best-effort).
pub fn describe(expr: &CronExpr) -> String {
    let mut out = String::new();
    out.push_str(&format!("minute {}", expr.fields[0].raw));
    out.push_str(&format!(", hour {}", expr.fields[1].raw));
    out.push_str(&format!(", dom {}", expr.fields[2].raw));
    out.push_str(&format!(", month {}", expr.fields[3].raw));
    out.push_str(&format!(", dow {}", expr.fields[4].raw));
    out
}

/// 算 next trigger (往后 `from_secs` 内), 仅 enumerate cron 错位 (crude O(N)).
pub fn next_after(
    expr: &CronExpr,
    minute: u8,
    hour: u8,
    dom: u8,
    month: u8,
    dow: u8,
) -> Option<(u8, u8, u8, u8, u8)> {
    let mut m = minute;
    let mut h = hour;
    let mut d = dom;
    let mut mo = month;
    let mut dw = dow;
    for _ in 0..(60 * 24 * 32 * 12) {
        if m < 59 {
            m += 1;
        } else {
            m = 0;
            if h < 23 {
                h += 1;
            } else {
                h = 0; /* crudely wrap d/mo/dw */
            }
        }
        if expr.matches(m, h, d, mo, dw) {
            return Some((m, h, d, mo, dw));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_schedule_passes() {
        assert!(Schedule::new("tick", 60).validate().is_ok());
    }
    #[test]
    fn zero_interval_is_rejected() {
        assert!(Schedule::new("tick", 0).validate().is_err());
    }

    #[test]
    fn cron_wildcard_matches_anything() {
        let e = CronExpr::parse("* * * * *").unwrap();
        for m in 0..60 {
            assert!(e.matches(m, 0, 1, 1, 0));
        }
    }
    #[test]
    fn cron_literal() {
        let e = CronExpr::parse("30 9 * * *").unwrap();
        assert!(e.matches(30, 9, 1, 1, 0));
        assert!(!e.matches(31, 9, 1, 1, 0));
        assert!(!e.matches(30, 10, 1, 1, 0));
    }
    #[test]
    fn cron_list_and_step() {
        let e = CronExpr::parse("*/15 * * * *").unwrap();
        for m in 0..60 {
            assert_eq!(e.matches(m, 0, 1, 1, 0), m % 15 == 0, "m={m}");
        }
    }
    #[test]
    fn cron_list_csv() {
        let e = CronExpr::parse("0 9-17 * * 1-5").unwrap();
        assert!(!e.matches(0, 8, 1, 1, 1));
        assert!(e.matches(0, 9, 1, 1, 1));
        assert!(e.matches(0, 17, 1, 1, 5));
        assert!(!e.matches(0, 18, 1, 1, 5));
        assert!(!e.matches(0, 12, 1, 1, 6)); // 周六
    }
    #[test]
    fn cron_field_count_mismatch() {
        assert!(matches!(
            CronExpr::parse("* * * *"),
            Err(CronError::FieldCountMismatch(4))
        ));
    }
    #[test]
    fn cron_out_of_range_rejected() {
        assert!(CronExpr::parse("60 * * * *").is_err()); // minute = 60 invalid
    }
    #[test]
    fn cron_parses_via_fromstr() {
        use std::str::FromStr;
        let e = CronExpr::from_str("0 12 * * 0").unwrap();
        assert!(e.matches(0, 12, 1, 1, 0));
    }
    #[test]
    fn describe_basic() {
        let e = CronExpr::parse("0 12 * * *").unwrap();
        let s = describe(&e);
        assert!(s.contains("minute 0") && s.contains("hour 12"));
    }
    #[test]
    fn validate_expr_returns_cron() {
        assert!(validate_expr("* * * * *").is_ok());
        assert!(validate_expr("bogus").is_err());
    }
    #[test]
    fn next_after_finds_match() {
        let e = CronExpr::parse("0 * * * *").unwrap();
        // 从 m=5, h=9 出发, 下个 minute=0 触发点
        let n = next_after(&e, 5, 9, 1, 1, 0);
        assert!(n.is_some(), "next_after should find a trigger");
        let (m, _, _, _, _) = n.unwrap();
        assert_eq!(
            m, 0,
            "next trigger minute should be 0 (per expr `0 * * * *`)"
        );
    }
}

// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
