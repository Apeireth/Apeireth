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
    #[error("cron: 未知 @ shorthand `{0}` (支持: @hourly @daily @midnight @weekly @monthly @yearly @annually @reboot)")]
    UnknownShorthand(String),
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

/// Month name aliases (case-insensitive, 3-letter prefix per Vixie cron convention).
/// JAN/FEB/MAR/APR/MAY/JUN/JUL/AUG/SEP/OCT/NOV/DEC → 1..=12.
pub const MONTH_ALIASES: &[(&str, u8)] = &[
    ("JAN", 1), ("FEB", 2), ("MAR", 3), ("APR", 4),
    ("MAY", 5), ("JUN", 6), ("JUL", 7), ("AUG", 8),
    ("SEP", 9), ("OCT", 10), ("NOV", 11), ("DEC", 12),
];

/// Day-of-week aliases (case-insensitive, 3-letter prefix per Vixie cron convention).
/// SUN/MON/TUE/WED/THU/FRI/SAT → 0..=6 (Sunday = 0).
pub const DOW_ALIASES: &[(&str, u8)] = &[
    ("SUN", 0), ("MON", 1), ("TUE", 2), ("WED", 3),
    ("THU", 4), ("FRI", 5), ("SAT", 6),
];

impl Field {
    /// Parse `*` / `5` / `1,3,5` / `*/15` / `0-23` / `1-30/2`,
    /// with optional name aliases for month/dow fields (e.g. `JAN`, `MON`).
    /// `aliases` slice: each entry `(NAME, numeric_value)`. Name match is
    /// case-insensitive on the 3-letter prefix (Vixie cron convention).
    pub fn parse_alias(s: &str, lo: u8, hi: u8, aliases: &[(&str, u8)]) -> CronResult<Field> {
        // 先试 alias (e.g. "JAN", "JAN,FEB,MAR") — 用 alias 解析, 不再走数字 parse
        let upper = s.to_ascii_uppercase();
        let mut bits: u64 = 0;
        let mut consumed_any_alias = false;
        for piece in upper.split(',') {
            // 1. 单 alias 匹配 (完整 3 字母)
            if let Some((_, v)) = aliases.iter().find(|(name, _)| piece == *name) {
                if *v < lo || *v > hi {
                    return Err(CronError::ParseError(
                        s.into(),
                        format!("alias `{piece}` = {v} out of range {lo}..={hi}"),
                    ));
                }
                bits |= 1u64 << *v;
                consumed_any_alias = true;
                continue;
            }
            // 2. alias 范围 (e.g. `MON-FRI`) — 把别名转数字后当数字范围
            if let Some((alias_start, alias_end)) =
                piece.split_once('-').and_then(|(a, b)| {
                    let start = aliases.iter().find(|(name, _)| a == *name).map(|(_, v)| *v);
                    let end = aliases.iter().find(|(name, _)| b == *name).map(|(_, v)| *v);
                    match (start, end) {
                        (Some(s), Some(e)) => Some((s, e)),
                        _ => None,
                    }
                })
            {
                if alias_start < lo || alias_end > hi || alias_start > alias_end {
                    return Err(CronError::ParseError(
                        s.into(),
                        format!("alias range {alias_start}..{alias_end} out of range {lo}..={hi}"),
                    ));
                }
                let mut v = alias_start;
                while v <= alias_end {
                    bits |= 1u64 << v;
                    v = v.saturating_add(1);
                }
                consumed_any_alias = true;
                continue;
            }
            // 3. 标准 parse (`*` / `5` / `1,3,5` / `*/15` / `0-23` / `1-30/2`)
            // 复用逻辑, 但只对没匹配 alias 的 piece 走标准路径
            let (range_part, step) = match piece.split_once('/') {
                Some((r, st)) => (
                    r,
                    st.parse::<u8>().map_err(|e| {
                        CronError::ParseError(s.into(), format!("step parse: {e}"))
                    })?,
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
        // 检查: 如果有任何 piece 不是 alias 也不是数字/range, parse 会自然 fail (u8::parse_err)
        // 这里 consumed_any_alias 仅用于将来优化 (跳过纯 alias 的 s == "JAN" 快速路径), 现在 noop
        let _ = consumed_any_alias;
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
    /// Parse cron expression, including `@`-shorthand aliases.
    ///
    /// Standard 5-field: `minute hour dom month dow`
    /// Shorthands (per Vixie cron convention):
    ///   `@hourly`     → `0 * * * *`      (top of every hour)
    ///   `@daily`      → `0 0 * * *`      (midnight every day)
    ///   `@midnight`   → `0 0 * * *`      (alias for @daily)
    ///   `@weekly`     → `0 0 * * 0`      (midnight every Sunday)
    ///   `@monthly`    → `0 0 1 * *`      (midnight 1st of month)
    ///   `@yearly`     → `0 0 1 1 *`      (midnight Jan 1)
    ///   `@annually`   → `0 0 1 1 *`      (alias for @yearly)
    ///   `@reboot`     → marker only (no schedule); see `is_reboot()` method
    pub fn parse(expr: &str) -> CronResult<CronExpr> {
        let expr = expr.trim();
        if let Some(stripped) = expr.strip_prefix('@') {
            // @shorthand alias (per Vixie cron convention)
            let resolved = match stripped.to_ascii_lowercase().as_str() {
                "hourly" => "0 * * * *",
                "daily" | "midnight" => "0 0 * * *",
                "weekly" => "0 0 * * 0",
                "monthly" => "0 0 1 * *",
                "yearly" | "annually" => "0 0 1 1 *",
                "reboot" => {
                    // @reboot 不是时间表, 是触发器; 解析成一个 dummy expr
                    // 调用方用 is_reboot() 判断
                    return Ok(CronExpr {
                        raw: "@reboot".into(),
                        fields: [
                            Field { raw: "0".into(), lo: 0, hi: 59, bits: 1 },
                            Field { raw: "0".into(), lo: 0, hi: 23, bits: 1 },
                            Field { raw: "1".into(), lo: 1, hi: 31, bits: 1 << 1 },
                            Field { raw: "1".into(), lo: 1, hi: 12, bits: 1 << 1 },
                            Field { raw: "0".into(), lo: 0, hi: 6, bits: 1 },
                        ],
                    });
                }
                other => {
                    return Err(CronError::UnknownShorthand(format!("@{other}")));
                }
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
        let mon = Field::parse_alias(parts[3], 1, 12, &MONTH_ALIASES)?;
        let dow = Field::parse_alias(parts[4], 0, 6, &DOW_ALIASES)?;
        Ok(CronExpr {
            raw: expr.into(),
            fields: [mins, hrs, dom, mon, dow],
        })
    }
    /// Whether this expression is the `@reboot` special (one-shot at startup).
    pub fn is_reboot(&self) -> bool {
        self.raw == "@reboot"
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
/// For `@reboot`, returns the literal "at startup (one-shot)".
pub fn describe(expr: &CronExpr) -> String {
    if expr.is_reboot() {
        return "at startup (one-shot)".to_string();
    }
    let mut out = String::new();
    out.push_str(&format!("minute {}", expr.fields[0].raw));
    out.push_str(&format!(", hour {}", expr.fields[1].raw));
    out.push_str(&format!(", dom {}", expr.fields[2].raw));
    out.push_str(&format!(", month {}", expr.fields[3].raw));
    out.push_str(&format!(", dow {}", expr.fields[4].raw));
    out
}

/// 算 next trigger (往后 1 年内), 跨日 / 跨月 / 跨年正确处理.
///
/// 返回 (`minute`, `hour`, `day`, `month`, `dow`) 元组, 从给定时间后 1 分钟开始枚举。
/// `year` 用于计算闰年 (影响 2 月天数) 和 dow 推导。
///
/// 实现: O(N) 枚举, N = 1 年分钟数 (525,600, 非闰年). 简单但正确.
/// 真实 production 用 croniter (Python) 或 tokio-cron-scheduler (Rust), 我们只做
/// 基础 next_after, 不优化.
pub fn next_after(
    expr: &CronExpr,
    year: u16,
    minute: u8,
    hour: u8,
    dom: u8,
    month: u8,
    dow: u8,
) -> Option<(u8, u8, u8, u8, u8)> {
    // 闰年判断 (per Gregorian: %4 == 0 && (%100 != 0 || %400 == 0))
    let is_leap = |y: u16| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);

    // 月天数 (平年 / 闰年)
    let days_in_month = |y: u16, mo: u8| -> u8 {
        match mo {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if is_leap(y) { 29 } else { 28 },
            _ => 0,
        }
    };

    // Zeller's congruence 计算 dow (0=Sunday, 1=Monday, ..., 6=Saturday)
    // Sakamoto's algorithm 计算 dow (0=Sunday, 1=Monday, ..., 6=Saturday)
    // 注: 我们的 dow 0 = Sunday 跟 Sakamoto 直接对齐
    let compute_dow = |y: u16, mo: u8, d: u8| -> u8 {
        // Sakamoto 不 remap month, 只在 m<3 时把 y 减 1
        // t 数组是 [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4] 跟 natural month 1-12 对应
        // (Jan=0, Feb=3, Mar=2, Apr=5, May=0, Jun=3, Jul=5, Aug=1, Sep=4, Oct=6, Nov=2, Dec=4)
        let y_adj: u16 = if mo < 3 { y.wrapping_sub(1) } else { y };
        let t: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let m_idx = (mo - 1) as usize;
        let y_i = y_adj as i32;
        let d_i = d as i32;
        // Sakamoto 公式: (y + y/4 - y/100 + y/400 + t[m-1] + d) % 7
        // 直接返 0=Sunday, 1=Monday, ..., 6=Saturday, 无需 remap
        let dow = (d_i + t[m_idx] + y_i + y_i / 4 - y_i / 100 + y_i / 400) % 7;
        dow as u8
    };

    let mut y = year;
    let mut mo = month;
    let mut d = dom;
    let mut h = hour;
    let mut m = minute;
    let mut dw = dow;

    // 枚举 1 年 (525,600 分钟, 非闰年; 525,948 含 2-29)
    for _ in 0..(366 * 24 * 60) {
        // advance 1 minute
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
        let n = next_after(&e, 2026, 5, 9, 1, 1, 0);
        assert!(n.is_some(), "next_after should find a trigger");
        let (m, _, _, _, _) = n.unwrap();
        assert_eq!(
            m, 0,
            "next trigger minute should be 0 (per expr `0 * * * *`)"
        );
    }

    // === @shorthand aliases (per Vixie cron convention) ===
    #[test]
    fn shorthand_hourly() {
        let e = CronExpr::parse("@hourly").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));
        assert!(!e.matches(1, 0, 1, 1, 0));
    }
    #[test]
    fn shorthand_daily_and_midnight() {
        let e1 = CronExpr::parse("@daily").unwrap();
        let e2 = CronExpr::parse("@midnight").unwrap();
        assert_eq!(e1.raw, e2.raw);  // both → 0 0 * * *
        assert!(e1.matches(0, 0, 1, 1, 0));
        assert!(!e1.matches(0, 1, 1, 1, 0));
    }
    #[test]
    fn shorthand_weekly() {
        let e = CronExpr::parse("@weekly").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));   // Sun midnight
        assert!(!e.matches(0, 0, 1, 1, 1)); // Mon
    }
    #[test]
    fn shorthand_monthly_yearly() {
        let m = CronExpr::parse("@monthly").unwrap();
        let y = CronExpr::parse("@yearly").unwrap();
        assert!(m.matches(0, 0, 1, 1, 0)); // 1st of any month
        assert!(!m.matches(0, 0, 2, 1, 0));
        assert!(y.matches(0, 0, 1, 1, 0));  // Jan 1 midnight
        assert!(!y.matches(0, 0, 1, 2, 0)); // Feb 1
    }
    #[test]
    fn shorthand_yearly_annually_alias() {
        let y = CronExpr::parse("@yearly").unwrap();
        let a = CronExpr::parse("@annually").unwrap();
        assert_eq!(y.raw, a.raw);
    }
    #[test]
    fn shorthand_reboot_is_special() {
        let e = CronExpr::parse("@reboot").unwrap();
        assert!(e.is_reboot(), "@reboot should be marked as reboot");
        let h = CronExpr::parse("@hourly").unwrap();
        assert!(!h.is_reboot());
    }
    #[test]
    fn shorthand_unknown_rejected() {
        let r = CronExpr::parse("@never");
        assert!(matches!(r, Err(CronError::UnknownShorthand(_))));
    }

    // === month/dow name aliases (Vixie cron convention) ===
    #[test]
    fn month_alias_jan() {
        let e = CronExpr::parse("0 0 1 JAN *").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));  // Jan 1
        assert!(!e.matches(0, 0, 1, 2, 0)); // Feb 1
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
    fn dow_alias_mon_to_fri() {
        let e = CronExpr::parse("0 9 * * MON-FRI").unwrap();
        assert!(e.matches(0, 9, 1, 1, 1));  // Mon
        assert!(e.matches(0, 9, 1, 1, 5));  // Fri
        assert!(!e.matches(0, 9, 1, 1, 0)); // Sun
        assert!(!e.matches(0, 9, 1, 1, 6)); // Sat
    }
    #[test]
    fn dow_alias_sun() {
        let e = CronExpr::parse("0 0 * * SUN").unwrap();
        assert!(e.matches(0, 0, 1, 1, 0));  // Sun
        assert!(!e.matches(0, 0, 1, 1, 1)); // Mon
    }
    #[test]
    fn backward_compat_numeric_still_works() {
        // alias 加了, 数字必须仍然能用 (避免 breaking change)
        let e = CronExpr::parse("30 14 1 6 3").unwrap();
        assert!(e.matches(30, 14, 1, 6, 3));  // 14:30 Wed Jun 1
        assert!(!e.matches(30, 14, 1, 7, 3)); // Jul 1
    }
}

// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
