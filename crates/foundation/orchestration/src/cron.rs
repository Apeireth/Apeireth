//! 5-field cron parse / match / next-after (library, default-off).
//!
//! Recovered from `legacy/donor/apeireth-cron/src/lib.rs`. This is a **parser
//! and matcher**, not a scheduler. There is no tokio interval, no job table,
//! no daemon. Callers that already own a heartbeat (v2
//! `runtime::canonical::heartbeat::HeartbeatScheduler`) may ask `matches` /
//! `next_after` when they need cron *expressions*. They do not get a second
//! loop from this module.
//!
//! Supported:
//! - 5 fields: `minute hour dom month dow`
//! - `*` / literal / `a,b` list / `a-b` range / `*/n` and `a-b/n` step
//! - Vixie `@hourly @daily @midnight @weekly @monthly @yearly @annually @reboot`
//! - Month aliases JAN..DEC and dow aliases SUN..SAT (case-insensitive 3-letter)
//! - `next_after`: enumerate at most ~1 year of minutes, Gregorian leap days,
//!   Sakamoto weekday
//!
//! Discarded: `CronEngine` tokio tick loop (`scheduler.rs`, test-only in donor
//! and a second loop even then). Approximate 30-day epoch→date conversion
//! used by that engine is also discarded.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Cron library errors. No `thiserror` (orchestration crate does not take it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronError {
    NonPositiveInterval(i64),
    ParseError(String, String),
    FieldCountMismatch(usize),
    UnknownShorthand(String),
}

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveInterval(n) => write!(f, "cron: interval `{n}` must be > 0"),
            Self::ParseError(expr, reason) => {
                write!(f, "cron: parse expr `{expr}` failed: {reason}")
            }
            Self::FieldCountMismatch(n) => {
                write!(f, "cron: 5-field expr must split into 5 parts (got {n})")
            }
            Self::UnknownShorthand(s) => write!(
                f,
                "cron: unknown @ shorthand `{s}` (supported: @hourly @daily @midnight @weekly @monthly @yearly @annually @reboot)"
            ),
        }
    }
}

impl std::error::Error for CronError {}

pub type CronResult<T> = Result<T, CronError>;

/// Named interval schedule (not a cron expression). `interval_secs` must be > 0.
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

/// Standard 5-field cron expression: minute hour dom month dow.
///
/// `*` wildcard, `5` literal, `1,3,5` list, `*/15` step, `0-23` range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr {
    pub raw: String,
    pub fields: [Field; 5],
}

/// One cron field as a 64-bit occupancy bitmap over `[lo, hi]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub raw: String,
    pub lo: u8,
    pub hi: u8,
    pub bits: u64,
}

/// Month name aliases (Vixie 3-letter, case-insensitive). JAN=1 .. DEC=12.
pub const MONTH_ALIASES: &[(&str, u8)] = &[
    ("JAN", 1),
    ("FEB", 2),
    ("MAR", 3),
    ("APR", 4),
    ("MAY", 5),
    ("JUN", 6),
    ("JUL", 7),
    ("AUG", 8),
    ("SEP", 9),
    ("OCT", 10),
    ("NOV", 11),
    ("DEC", 12),
];

/// Day-of-week aliases (Vixie 3-letter, case-insensitive). SUN=0 .. SAT=6.
pub const DOW_ALIASES: &[(&str, u8)] = &[
    ("SUN", 0),
    ("MON", 1),
    ("TUE", 2),
    ("WED", 3),
    ("THU", 4),
    ("FRI", 5),
    ("SAT", 6),
];

impl Field {
    /// Parse `*` / `5` / `1,3,5` / `*/15` / `0-23` / `1-30/2`, with optional
    /// name aliases (e.g. `JAN`, `MON-FRI`). Alias match is case-insensitive
    /// on the 3-letter prefix (Vixie).
    pub fn parse_alias(s: &str, lo: u8, hi: u8, aliases: &[(&str, u8)]) -> CronResult<Field> {
        let upper = s.to_ascii_uppercase();
        let mut bits: u64 = 0;
        for piece in upper.split(',') {
            if let Some((_, v)) = aliases.iter().find(|(name, _)| piece == *name) {
                if *v < lo || *v > hi {
                    return Err(CronError::ParseError(
                        s.into(),
                        format!("alias `{piece}` = {v} out of range {lo}..={hi}"),
                    ));
                }
                bits |= 1u64 << *v;
                continue;
            }
            if let Some((alias_start, alias_end)) = piece.split_once('-').and_then(|(a, b)| {
                let start = aliases.iter().find(|(name, _)| a == *name).map(|(_, v)| *v);
                let end = aliases.iter().find(|(name, _)| b == *name).map(|(_, v)| *v);
                match (start, end) {
                    (Some(s), Some(e)) => Some((s, e)),
                    _ => None,
                }
            }) {
                if alias_start < lo || alias_end > hi || alias_start > alias_end {
                    return Err(CronError::ParseError(
                        s.into(),
                        format!("alias range {alias_start}..{alias_end} out of range {lo}..={hi}"),
                    ));
                }
                let mut v = alias_start;
                loop {
                    bits |= 1u64 << v;
                    if v == alias_end {
                        break;
                    }
                    v += 1;
                }
                continue;
            }
            or_numeric_piece(s, piece, lo, hi, &mut bits)?;
        }
        Ok(Field {
            raw: s.into(),
            lo,
            hi,
            bits,
        })
    }

    /// Parse `*` / `5` / `1,3,5` / `*/15` / `0-23` / `1-30/2`.
    pub fn parse(s: &str, lo: u8, hi: u8) -> CronResult<Field> {
        let mut bits: u64 = 0;
        for piece in s.split(',') {
            or_numeric_piece(s, piece, lo, hi, &mut bits)?;
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

fn or_numeric_piece(orig: &str, piece: &str, lo: u8, hi: u8, bits: &mut u64) -> CronResult<()> {
    let (range_part, step) = match piece.split_once('/') {
        Some((r, st)) => (
            r,
            st.parse::<u8>()
                .map_err(|e| CronError::ParseError(orig.into(), format!("step parse: {e}")))?,
        ),
        None => (piece, 1u8),
    };
    if step == 0 {
        return Err(CronError::ParseError(
            orig.into(),
            "step must be > 0".into(),
        ));
    }
    let (start, end) = match range_part.split_once('-') {
        Some((a, b)) => (
            a.parse::<u8>()
                .map_err(|e| CronError::ParseError(orig.into(), format!("range start: {e}")))?,
            b.parse::<u8>()
                .map_err(|e| CronError::ParseError(orig.into(), format!("range end: {e}")))?,
        ),
        None => {
            if range_part == "*" {
                (lo, hi)
            } else {
                let v = range_part.parse::<u8>().map_err(|e| {
                    CronError::ParseError(orig.into(), format!("literal parse: {e}"))
                })?;
                (v, v)
            }
        }
    };
    if start < lo || end > hi || start > end {
        return Err(CronError::ParseError(
            orig.into(),
            format!("out of range {lo}..={hi}"),
        ));
    }
    let mut v = start;
    loop {
        *bits |= 1u64 << v;
        let Some(next) = v.checked_add(step) else {
            break;
        };
        if next > end {
            break;
        }
        v = next;
    }
    Ok(())
}

impl CronExpr {
    /// Parse a 5-field expression or a Vixie `@` shorthand.
    ///
    /// `@hourly` → `0 * * * *`
    /// `@daily` / `@midnight` → `0 0 * * *`
    /// `@weekly` → `0 0 * * 0`
    /// `@monthly` → `0 0 1 * *`
    /// `@yearly` / `@annually` → `0 0 1 1 *`
    /// `@reboot` is a marker (`is_reboot()`), not a timetable.
    pub fn parse(expr: &str) -> CronResult<CronExpr> {
        let expr = expr.trim();
        if let Some(stripped) = expr.strip_prefix('@') {
            let resolved = match stripped.to_ascii_lowercase().as_str() {
                "hourly" => "0 * * * *",
                "daily" | "midnight" => "0 0 * * *",
                "weekly" => "0 0 * * 0",
                "monthly" => "0 0 1 * *",
                "yearly" | "annually" => "0 0 1 1 *",
                "reboot" => {
                    return Ok(CronExpr {
                        raw: "@reboot".into(),
                        fields: [
                            Field {
                                raw: "0".into(),
                                lo: 0,
                                hi: 59,
                                bits: 1,
                            },
                            Field {
                                raw: "0".into(),
                                lo: 0,
                                hi: 23,
                                bits: 1,
                            },
                            Field {
                                raw: "1".into(),
                                lo: 1,
                                hi: 31,
                                bits: 1 << 1,
                            },
                            Field {
                                raw: "1".into(),
                                lo: 1,
                                hi: 12,
                                bits: 1 << 1,
                            },
                            Field {
                                raw: "0".into(),
                                lo: 0,
                                hi: 6,
                                bits: 1,
                            },
                        ],
                    });
                }
                other => return Err(CronError::UnknownShorthand(format!("@{other}"))),
            };
            return CronExpr::parse(resolved);
        }
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(CronError::FieldCountMismatch(parts.len()));
        }
        let mins = Field::parse(parts[0], 0, 59)?;
        let hrs = Field::parse(parts[1], 0, 23)?;
        let dom = Field::parse(parts[2], 1, 31)?;
        let mon = Field::parse_alias(parts[3], 1, 12, MONTH_ALIASES)?;
        let dow = Field::parse_alias(parts[4], 0, 6, DOW_ALIASES)?;
        Ok(CronExpr {
            raw: expr.into(),
            fields: [mins, hrs, dom, mon, dow],
        })
    }

    /// `@reboot` special (one-shot at startup; matcher is a dummy timetable).
    pub fn is_reboot(&self) -> bool {
        self.raw == "@reboot"
    }

    /// Test (minute, hour, day-of-month, month, day-of-week).
    pub fn matches(&self, m: u8, h: u8, dom: u8, mon: u8, dow: u8) -> bool {
        self.fields[0].matches(m)
            && self.fields[1].matches(h)
            && self.fields[2].matches(dom)
            && self.fields[3].matches(mon)
            && self.fields[4].matches(dow)
    }
}

impl fmt::Display for CronExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl FromStr for CronExpr {
    type Err = CronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

pub fn validate_schedule(s: &Schedule) -> CronResult<()> {
    s.validate()
}

pub fn validate_expr(expr: &str) -> CronResult<CronExpr> {
    CronExpr::parse(expr)
}

/// Best-effort human description. `@reboot` → `"at startup (one-shot)"`.
pub fn describe(expr: &CronExpr) -> String {
    if expr.is_reboot() {
        return "at startup (one-shot)".to_string();
    }
    format!(
        "minute {}, hour {}, dom {}, month {}, dow {}",
        expr.fields[0].raw,
        expr.fields[1].raw,
        expr.fields[2].raw,
        expr.fields[3].raw,
        expr.fields[4].raw
    )
}

/// Next match strictly after the given civil time, searching at most ~1 year.
///
/// Returns `(minute, hour, day, month, dow)`. `year` is used for leap-day
/// length and Sakamoto weekday. `@reboot` never matches a wall clock.
pub fn next_after(
    expr: &CronExpr,
    year: u16,
    minute: u8,
    hour: u8,
    dom: u8,
    month: u8,
    dow: u8,
) -> Option<(u8, u8, u8, u8, u8)> {
    if expr.is_reboot() {
        return None;
    }

    let is_leap = |y: u16| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days_in_month = |y: u16, mo: u8| -> u8 {
        match mo {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap(y) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    };
    // Sakamoto: 0=Sunday .. 6=Saturday.
    let compute_dow = |y: u16, mo: u8, d: u8| -> u8 {
        let y_adj: u16 = if mo < 3 { y.wrapping_sub(1) } else { y };
        let t: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let m_idx = (mo - 1) as usize;
        let y_i = i32::from(y_adj);
        let d_i = i32::from(d);
        let dow = (d_i + t[m_idx] + y_i + y_i / 4 - y_i / 100 + y_i / 400) % 7;
        dow as u8
    };

    let mut y = year;
    let mut mo = month;
    let mut d = dom;
    let mut h = hour;
    let mut m = minute;
    let mut dw = dow;

    for _ in 0..(366 * 24 * 60) {
        m += 1;
        if m >= 60 {
            m = 0;
            h += 1;
            if h >= 24 {
                h = 0;
                d += 1;
                if d > days_in_month(y, mo) {
                    d = 1;
                    mo += 1;
                    if mo > 12 {
                        mo = 1;
                        y += 1;
                        if y > year + 1 {
                            return None;
                        }
                    }
                }
                dw = compute_dow(y, mo, d);
            }
        }
        if expr.matches(m, h, d, mo, dw) {
            return Some((m, h, d, mo, dw));
        }
    }
    None
}

/// Bitmap equality (aliases vs digits share bits, not `raw`).
pub fn field_bits_eq(a: &CronExpr, b: &CronExpr) -> bool {
    a.fields
        .iter()
        .zip(b.fields.iter())
        .all(|(fa, fb)| fa.lo == fb.lo && fa.hi == fb.hi && fa.bits == fb.bits)
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
        assert!(Schedule::new("tick", -1).validate().is_err());
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
        assert!(!e.matches(0, 12, 1, 1, 6));
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
        assert!(CronExpr::parse("60 * * * *").is_err());
    }

    #[test]
    fn cron_parses_via_fromstr() {
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
    fn describe_reboot() {
        let e = CronExpr::parse("@reboot").unwrap();
        assert_eq!(describe(&e), "at startup (one-shot)");
    }

    #[test]
    fn validate_expr_returns_cron() {
        assert!(validate_expr("* * * * *").is_ok());
        assert!(validate_expr("bogus").is_err());
    }

    #[test]
    fn next_after_finds_match() {
        let e = CronExpr::parse("0 * * * *").unwrap();
        let n = next_after(&e, 2026, 5, 9, 1, 1, 0);
        assert!(n.is_some());
        let (m, _, _, _, _) = n.unwrap();
        assert_eq!(m, 0);
    }

    #[test]
    fn next_after_reboot_is_none() {
        let e = CronExpr::parse("@reboot").unwrap();
        assert!(next_after(&e, 2026, 0, 0, 1, 1, 0).is_none());
    }

    #[test]
    fn shorthand_hourly() {
        let e = CronExpr::parse("@hourly").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));
        assert!(!e.matches(1, 0, 1, 1, 0));
        assert!(field_bits_eq(&e, &CronExpr::parse("0 * * * *").unwrap()));
    }

    #[test]
    fn shorthand_daily_and_midnight() {
        let e1 = CronExpr::parse("@daily").unwrap();
        let e2 = CronExpr::parse("@midnight").unwrap();
        assert!(field_bits_eq(&e1, &e2));
        assert!(e1.matches(0, 0, 1, 1, 0));
        assert!(!e1.matches(0, 1, 1, 1, 0));
    }

    #[test]
    fn shorthand_weekly() {
        let e = CronExpr::parse("@weekly").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));
        assert!(!e.matches(0, 0, 1, 1, 1));
    }

    #[test]
    fn shorthand_monthly_yearly() {
        let m = CronExpr::parse("@monthly").unwrap();
        let y = CronExpr::parse("@yearly").unwrap();
        assert!(m.matches(0, 0, 1, 1, 0));
        assert!(!m.matches(0, 0, 2, 1, 0));
        assert!(y.matches(0, 0, 1, 1, 0));
        assert!(!y.matches(0, 0, 1, 2, 0));
    }

    #[test]
    fn shorthand_yearly_annually_alias() {
        let y = CronExpr::parse("@yearly").unwrap();
        let a = CronExpr::parse("@annually").unwrap();
        assert!(field_bits_eq(&y, &a));
    }

    #[test]
    fn shorthand_reboot_is_special() {
        let e = CronExpr::parse("@reboot").unwrap();
        assert!(e.is_reboot());
        let h = CronExpr::parse("@hourly").unwrap();
        assert!(!h.is_reboot());
    }

    #[test]
    fn shorthand_unknown_rejected() {
        let r = CronExpr::parse("@never");
        assert!(matches!(r, Err(CronError::UnknownShorthand(_))));
    }

    #[test]
    fn month_alias_jan() {
        let e = CronExpr::parse("0 0 1 JAN *").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));
        assert!(!e.matches(0, 0, 1, 2, 0));
        assert!(field_bits_eq(&e, &CronExpr::parse("0 0 1 1 *").unwrap()));
    }

    #[test]
    fn month_alias_list() {
        let e = CronExpr::parse("0 0 1 JAN,APR,JUL,OCT *").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));
        assert!(e.matches(0, 0, 1, 4, 0));
        assert!(!e.matches(0, 0, 1, 2, 0));
    }

    #[test]
    fn month_alias_case_insensitive() {
        let e1 = CronExpr::parse("0 0 1 jan *").unwrap();
        let e2 = CronExpr::parse("0 0 1 Jan *").unwrap();
        let e3 = CronExpr::parse("0 0 1 JAN *").unwrap();
        assert_eq!(e1.matches(0, 0, 1, 1, 0), e2.matches(0, 0, 1, 1, 0));
        assert_eq!(e2.matches(0, 0, 1, 1, 0), e3.matches(0, 0, 1, 1, 0));
    }

    #[test]
    fn month_alias_range_jan_mar() {
        let a = CronExpr::parse("0 0 1 JAN-MAR *").unwrap();
        let n = CronExpr::parse("0 0 1 1-3 *").unwrap();
        assert!(field_bits_eq(&a, &n));
    }

    #[test]
    fn dow_alias_mon_to_fri() {
        let e = CronExpr::parse("0 9 * * MON-FRI").unwrap();
        assert!(e.matches(0, 9, 1, 1, 1));
        assert!(e.matches(0, 9, 1, 1, 5));
        assert!(!e.matches(0, 9, 1, 1, 0));
        assert!(!e.matches(0, 9, 1, 1, 6));
        assert!(field_bits_eq(&e, &CronExpr::parse("0 9 * * 1-5").unwrap()));
    }

    #[test]
    fn dow_alias_sun() {
        let e = CronExpr::parse("0 0 * * SUN").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));
        assert!(!e.matches(0, 0, 1, 1, 1));
    }

    #[test]
    fn backward_compat_numeric_still_works() {
        let e = CronExpr::parse("30 14 1 6 3").unwrap();
        assert!(e.matches(30, 14, 1, 6, 3));
        assert!(!e.matches(30, 14, 1, 7, 3));
    }

    #[test]
    fn business_hours_weekday_only() {
        let e = CronExpr::parse("0 9-17 * * MON-FRI").unwrap();
        assert!(e.matches(0, 9, 1, 1, 1));
        assert!(e.matches(0, 17, 1, 1, 5));
        assert!(!e.matches(0, 18, 1, 1, 1));
        assert!(!e.matches(0, 8, 1, 1, 1));
        assert!(!e.matches(0, 12, 1, 1, 6));
        assert!(!e.matches(0, 12, 1, 1, 0));
        assert!(!e.matches(30, 9, 1, 1, 1));
    }

    #[test]
    fn nightly_backup_at_2am() {
        let e = CronExpr::parse("30 2 * * *").unwrap();
        assert!(e.matches(30, 2, 1, 1, 1));
        assert!(e.matches(30, 2, 15, 7, 3));
        assert!(!e.matches(30, 3, 1, 1, 1));
        assert!(!e.matches(31, 2, 1, 1, 1));
    }

    #[test]
    fn heartbeat_every_5_minutes() {
        let e = CronExpr::parse("*/5 * * * *").unwrap();
        for m in 0..60 {
            assert_eq!(e.matches(m, 12, 1, 1, 1), m % 5 == 0, "minute={m}");
        }
        assert!(e.matches(0, 13, 1, 1, 1));
        assert!(e.matches(55, 23, 31, 12, 6));
    }

    #[test]
    fn quarterly_first_day_of_quarter() {
        let e = CronExpr::parse("0 0 1 1,4,7,10 *").unwrap();
        assert!(e.matches(0, 0, 1, 1, 1));
        assert!(e.matches(0, 0, 1, 4, 1));
        assert!(e.matches(0, 0, 1, 7, 1));
        assert!(e.matches(0, 0, 1, 10, 1));
        assert!(!e.matches(0, 0, 1, 2, 1));
        assert!(!e.matches(0, 0, 1, 3, 1));
        assert!(!e.matches(0, 0, 1, 11, 1));
    }

    #[test]
    fn last_friday_of_month_pattern() {
        let e = CronExpr::parse("0 17 24-31 * FRI").unwrap();
        assert!(e.matches(0, 17, 28, 1, 5));
        assert!(e.matches(0, 17, 25, 1, 5));
        assert!(!e.matches(0, 17, 22, 1, 5));
        assert!(!e.matches(0, 17, 24, 1, 1));
        assert!(!e.matches(0, 17, 24, 1, 6));
    }

    #[test]
    fn next_after_every_minute_increments_correctly() {
        let e = CronExpr::parse("* * * * *").unwrap();
        let (m, h, d, mo, dw) = next_after(&e, 2026, 30, 14, 15, 6, 1).unwrap();
        assert_eq!(m, 31);
        assert_eq!((h, d, mo), (14, 15, 6));
        assert_eq!(dw, 1);
    }

    #[test]
    fn next_after_midnight_rollover() {
        let e = CronExpr::parse("0 0 * * *").unwrap();
        let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 5, 8, 3).unwrap();
        assert_eq!((m, h), (0, 0));
        assert_eq!((d, mo), (6, 8));
        assert_eq!(dw, 4);
    }

    #[test]
    fn next_after_every_15_min_quarterly() {
        let e = CronExpr::parse("*/15 * * * *").unwrap();
        let (m, h, _, _, _) = next_after(&e, 2026, 7, 14, 1, 1, 1).unwrap();
        assert_eq!((m, h), (15, 14));
        let (m, h, _, _, _) = next_after(&e, 2026, 50, 14, 1, 1, 1).unwrap();
        assert_eq!((m, h), (0, 15));
    }

    #[test]
    fn next_after_weekly_jump() {
        let e = CronExpr::parse("0 0 * * 0").unwrap();
        let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 1, 8, 6).unwrap();
        assert_eq!((m, h), (0, 0));
        assert_eq!((d, mo), (2, 8));
        assert_eq!(dw, 0);
    }

    #[test]
    fn next_after_month_boundary() {
        let e = CronExpr::parse("0 0 1 * *").unwrap();
        let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 31, 8, 1).unwrap();
        assert_eq!((m, h), (0, 0));
        assert_eq!((d, mo), (1, 9));
        assert_eq!(dw, 2);
    }

    #[test]
    fn next_after_year_boundary() {
        let e = CronExpr::parse("0 0 1 1 *").unwrap();
        let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 31, 12, 4).unwrap();
        assert_eq!((m, h), (0, 0));
        assert_eq!((d, mo), (1, 1));
        assert_eq!(dw, 5);
    }

    #[test]
    fn next_after_leap_year_handles_feb_29() {
        let e = CronExpr::parse("0 0 29 2 *").unwrap();
        let (m, h, d, mo, dw) = next_after(&e, 2028, 59, 23, 28, 2, 1).unwrap();
        assert_eq!((m, h), (0, 0));
        assert_eq!((d, mo), (29, 2));
        assert_eq!(dw, 2);
    }

    #[test]
    fn parse_display_reparse_round_trip() {
        let originals = [
            "0 9-17 * * MON-FRI",
            "30 2 * * *",
            "*/5 * * * *",
            "0 0 1 1,4,7,10 *",
            "0 17 24-31 * FRI",
            "0 0 1 JAN-MAR *",
        ];
        for raw in originals {
            let e1 = CronExpr::parse(raw).unwrap();
            let e2 = CronExpr::parse(&e1.to_string()).unwrap();
            assert!(field_bits_eq(&e1, &e2), "re-parse mismatch: {raw}");
            assert!(!describe(&e1).is_empty());
        }
    }

    #[test]
    fn invalid_exprs_fail_gracefully() {
        let invalid = [
            "",
            "* * * *",
            "* * * * * *",
            "60 * * * *",
            "* 24 * * *",
            "* * 32 * *",
            "* * 0 * *",
            "* * * 0 *",
            "* * * 13 *",
            "* * * * 7",
        ];
        for bad in invalid {
            assert!(CronExpr::parse(bad).is_err(), "should Err: {bad:?}");
        }
    }

    #[test]
    fn unknown_shorthand_fails_with_descriptive_error() {
        let err = CronExpr::parse("@never").unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("@never"), "got {msg}");
    }

    #[test]
    fn step_zero_rejected() {
        assert!(CronExpr::parse("*/0 * * * *").is_err());
    }

    #[test]
    fn range_step() {
        let e = CronExpr::parse("1-10/3 * * * *").unwrap();
        assert!(e.matches(1, 0, 1, 1, 0));
        assert!(e.matches(4, 0, 1, 1, 0));
        assert!(e.matches(7, 0, 1, 1, 0));
        assert!(e.matches(10, 0, 1, 1, 0));
        assert!(!e.matches(2, 0, 1, 1, 0));
        assert!(!e.matches(11, 0, 1, 1, 0));
    }
}
