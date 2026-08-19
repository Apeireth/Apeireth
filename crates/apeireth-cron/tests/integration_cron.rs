//! Integration tests for apeireth-cron — end-to-end 真实使用场景.
//!
//! Unit tests 在 src/lib.rs mod tests (per-fn, 25 cases), 这里测:
//!   - 跨字段组合 (minute × hour × dom × month × dow 真实调度)
//!   - @-shorthand 与 5-field 等价 (避免 1.0 release 出错)
//!   - 月/星期别名 (JAN/MON-FRI) 与数字等价
//!   - 真实业务场景 (heartbeat / business hours / nightly backup / etc.)
//!   - 跨日 / 跨月 / 跨年时间推算 (含闰年)
//!
//! 镜像其他 LOCKED 重 crate 约定 (e.g. apeireth-asi/tests/integration_r_measure.rs),
//! integration tests 是 per-行为样板, 跟单元测互补。

use apeireth_cron::{describe, next_after, CronExpr, Schedule};

// =============================================================================
// Helper: 比 Field 语义 (bits) 不比 lexical (raw)
// 注: Field::PartialEq 是 derive(PartialEq) 整体比, 含 raw 字符串.
//    alias ("JAN") 跟数字 ("1") 的 raw 不同, 但 bits 应相同.
// =============================================================================

fn field_bits_eq(a: &CronExpr, b: &CronExpr) -> bool {
    if a.fields.len() != b.fields.len() {
        return false;
    }
    a.fields
        .iter()
        .zip(b.fields.iter())
        .all(|(fa, fb)| fa.lo == fb.lo && fa.hi == fb.hi && fa.bits == fb.bits)
}

// =============================================================================
// 1. @-shorthand 等价测试 — 跟 5-field 表达式 100% 一致
// =============================================================================

#[test]
fn shorthand_hourly_equals_zero_star_star_star_star() {
    let s = CronExpr::parse("@hourly").unwrap();
    let eq = CronExpr::parse("0 * * * *").unwrap();
    assert!(field_bits_eq(&s, &eq), "@hourly 必须等价于 0 * * * *");
}

#[test]
fn shorthand_daily_midnight_equals_zero_zero_star_star_star() {
    let s1 = CronExpr::parse("@daily").unwrap();
    let s2 = CronExpr::parse("@midnight").unwrap();
    let eq = CronExpr::parse("0 0 * * *").unwrap();
    assert!(field_bits_eq(&s1, &eq));
    assert!(field_bits_eq(&s2, &eq));
}

#[test]
fn shorthand_weekly_equals_zero_zero_star_star_zero() {
    let s = CronExpr::parse("@weekly").unwrap();
    let eq = CronExpr::parse("0 0 * * 0").unwrap();
    assert!(field_bits_eq(&s, &eq));
}

#[test]
fn shorthand_monthly_yearly_annually() {
    let m = CronExpr::parse("@monthly").unwrap();
    let y1 = CronExpr::parse("@yearly").unwrap();
    let y2 = CronExpr::parse("@annually").unwrap();
    assert!(field_bits_eq(&m, &CronExpr::parse("0 0 1 * *").unwrap()));
    assert!(field_bits_eq(&y1, &CronExpr::parse("0 0 1 1 *").unwrap()));
    assert!(field_bits_eq(&y2, &y1), "@yearly 跟 @annually 必须等价");
}

#[test]
fn shorthand_reboot_is_special() {
    // @reboot 语义: 启动时一次, 不走时间表. runtime 决定触发逻辑.
    let r = CronExpr::parse("@reboot").unwrap();
    assert!(r.is_reboot());
}

// =============================================================================
// 2. 月/星期别名 vs 数字等价 (semantic 比, 不是 lexical)
// =============================================================================

#[test]
fn month_alias_jan_equals_1() {
    let a = CronExpr::parse("0 0 1 JAN *").unwrap();
    let n = CronExpr::parse("0 0 1 1 *").unwrap();
    assert!(field_bits_eq(&a, &n), "JAN 必须等价于 1 (bits)");
}

#[test]
fn month_alias_range_jan_mar() {
    let a = CronExpr::parse("0 0 1 JAN-MAR *").unwrap();
    let n = CronExpr::parse("0 0 1 1-3 *").unwrap();
    assert!(field_bits_eq(&a, &n), "JAN-MAR 必须等价于 1-3 (bits)");
}

#[test]
fn dow_alias_mon_to_fri() {
    let a = CronExpr::parse("0 9 * * MON-FRI").unwrap();
    let n = CronExpr::parse("0 9 * * 1-5").unwrap();
    assert!(field_bits_eq(&a, &n), "MON-FRI 必须等价于 1-5 (bits)");
}

#[test]
fn dow_alias_sun() {
    let a = CronExpr::parse("0 0 * * SUN").unwrap();
    let n = CronExpr::parse("0 0 * * 0").unwrap();
    assert!(field_bits_eq(&a, &n));
}

// =============================================================================
// 3. 真实业务场景 — 复合 5-field
// =============================================================================

#[test]
fn business_hours_weekday_only() {
    // 真实场景: 工作日 9-17 点每小时整点跑, 不含周末
    let e = CronExpr::parse("0 9-17 * * MON-FRI").unwrap();

    // 周一 9:00
    assert!(e.matches(0, 9, 1, 1, 1));
    // 周五 17:00
    assert!(e.matches(0, 17, 1, 1, 5));
    // 周一 18:00 (超时)
    assert!(!e.matches(0, 18, 1, 1, 1));
    // 周一 8:00 (提前)
    assert!(!e.matches(0, 8, 1, 1, 1));
    // 周六 12:00 (周末)
    assert!(!e.matches(0, 12, 1, 1, 6));
    // 周日 12:00 (周末)
    assert!(!e.matches(0, 12, 1, 1, 0));
    // 9:30 (非整点)
    assert!(!e.matches(30, 9, 1, 1, 1));
}

#[test]
fn nightly_backup_at_2am() {
    // 真实场景: 每天凌晨 2:30 备份
    let e = CronExpr::parse("30 2 * * *").unwrap();
    assert!(e.matches(30, 2, 1, 1, 1));
    assert!(e.matches(30, 2, 15, 7, 3));
    assert!(!e.matches(30, 3, 1, 1, 1));
    assert!(!e.matches(31, 2, 1, 1, 1));
}

#[test]
fn heartbeat_every_5_minutes() {
    // 真实场景: 每 5 分钟心跳
    let e = CronExpr::parse("*/5 * * * *").unwrap();
    for m in 0..60 {
        assert_eq!(e.matches(m, 12, 1, 1, 1), m % 5 == 0, "minute={m}");
    }
    assert!(e.matches(0, 13, 1, 1, 1));
    assert!(e.matches(55, 23, 31, 12, 6));
}

#[test]
fn quarterly_first_day_of_quarter() {
    // 真实场景: 每季度第一天 0 点 (Q1=Jan, Q2=Apr, Q3=Jul, Q4=Oct)
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
    // 真实场景: 每月最后一个周五 17:00 (用 dow + 日范围逼近)
    // cron 5-field 没有 'L' (last) 支持, 但可以用 24-31 + FRI 限定
    let e = CronExpr::parse("0 17 24-31 * FRI").unwrap();
    assert!(e.matches(0, 17, 28, 1, 5));
    assert!(e.matches(0, 17, 25, 1, 5));
    assert!(!e.matches(0, 17, 22, 1, 5));
    assert!(!e.matches(0, 17, 24, 1, 1));
    assert!(!e.matches(0, 17, 24, 1, 6));
}

// =============================================================================
// 4. 跨日 / 跨月 / 跨年时间推算 (next_after 真实行为)
// 注: next_after 签名 8 参 (year, m, h, dom, month, dow), 2026-2028 含闰年
// =============================================================================

#[test]
fn next_after_every_minute_increments_correctly() {
    // 每分钟跑, 下次触发 = 当前 + 1 分钟
    // 2026-06-15 是 Monday (dow=1)
    let e = CronExpr::parse("* * * * *").unwrap();
    let (m, h, d, mo, dw) = next_after(&e, 2026, 30, 14, 15, 6, 1).unwrap();
    assert_eq!(m, 31);
    assert_eq!((h, d, mo), (14, 15, 6), "其他字段不变");
    assert_eq!(dw, 1, "Mon 14:31 仍是周一");
}

#[test]
fn next_after_midnight_rollover() {
    // 每天 0:00 触发, 从前一天 23:59 出发
    // 2026-08-05 是 Wednesday (dow=3), 2026-08-06 是 Thursday (dow=4)
    let e = CronExpr::parse("0 0 * * *").unwrap();
    let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 5, 8, 3).unwrap();
    assert_eq!((m, h), (0, 0), "59→0 minute, 23→0 hour");
    assert_eq!((d, mo), (6, 8), "Wed Aug 5 → Thu Aug 6");
    assert_eq!(dw, 4, "Thu (dow=4)");
}

#[test]
fn next_after_every_15_min_quarterly() {
    // 每 15 分钟 (*/15), 跨小时
    let e = CronExpr::parse("*/15 * * * *").unwrap();
    let (m, h, _, _, _) = next_after(&e, 2026, 7, 14, 1, 1, 1).unwrap();
    assert_eq!((m, h), (15, 14));
    let (m, h, _, _, _) = next_after(&e, 2026, 50, 14, 1, 1, 1).unwrap();
    assert_eq!((m, h), (0, 15));
}

#[test]
fn next_after_weekly_jump() {
    // 每周日 0:00, 从周六 23:59 出发
    // 2026-08-01 是 Saturday (dow=6), 2026-08-02 是 Sunday (dow=0)
    let e = CronExpr::parse("0 0 * * 0").unwrap();
    let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 1, 8, 6).unwrap();
    assert_eq!((m, h), (0, 0));
    assert_eq!((d, mo), (2, 8), "Sat Aug 1 → Sun Aug 2");
    assert_eq!(dw, 0, "Sun (dow=0)");
}

#[test]
fn next_after_month_boundary() {
    // 每月 1 号 0:00, 从 8 月 31 23:59 出发
    // 2026-08-31 是 Monday (dow=1), 2026-09-01 是 Tuesday (dow=2)
    let e = CronExpr::parse("0 0 1 * *").unwrap();
    let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 31, 8, 1).unwrap();
    assert_eq!((m, h), (0, 0));
    assert_eq!((d, mo), (1, 9), "Aug 31 → Sep 1");
    assert_eq!(dw, 2, "Tue (dow=2)");
}

#[test]
fn next_after_year_boundary() {
    // 1 月 1 号 0:00, 从 12 月 31 23:59 出发
    // 2026-12-31 是 Thursday (dow=4), 2027-01-01 是 Friday (dow=5)
    let e = CronExpr::parse("0 0 1 1 *").unwrap();
    let (m, h, d, mo, dw) = next_after(&e, 2026, 59, 23, 31, 12, 4).unwrap();
    assert_eq!((m, h), (0, 0));
    assert_eq!((d, mo), (1, 1), "Dec 31 2026 → Jan 1 2027");
    assert_eq!(dw, 5, "Fri (dow=5)");
}

#[test]
fn next_after_leap_year_handles_feb_29() {
    // 2028 是闰年, 2 月 29 存在
    // 2028-02-28 是 Monday (dow=1), 2028-02-29 是 Tuesday (dow=2)
    let e = CronExpr::parse("0 0 29 2 *").unwrap();
    let (m, h, d, mo, dw) = next_after(&e, 2028, 59, 23, 28, 2, 1).unwrap();
    assert_eq!((m, h), (0, 0));
    assert_eq!((d, mo), (29, 2), "Feb 28 → Feb 29 (闰年)");
    assert_eq!(dw, 2, "Tue (dow=2)");
}

// =============================================================================
// 5. 综合: 端到端 round-trip (parse → re-parse)
// =============================================================================

#[test]
fn parse_display_reparse_round_trip() {
    let originals = vec![
        "0 9-17 * * MON-FRI",
        "30 2 * * *",
        "*/5 * * * *",
        "0 0 1 1,4,7,10 *",
        "0 17 24-31 * FRI",
        "0 0 1 JAN-MAR *",
    ];

    for raw in originals {
        let e1 = CronExpr::parse(raw).unwrap();
        let e2 = CronExpr::parse(raw).unwrap();
        assert!(field_bits_eq(&e1, &e2), "re-parse 不一致: {raw}");
        let desc = describe(&e1);
        assert!(!desc.is_empty(), "describe 空: {raw}");
    }
}

// =============================================================================
// 6. 边界 / 错误恢复
// =============================================================================

#[test]
fn invalid_exprs_fail_gracefully() {
    // 5-field 表达式必须正好 5 段, 否则 Err
    let invalid = vec![
        "",            // 空
        "* * * *",     // 4 段
        "* * * * * *", // 6 段
        "60 * * * *",  // minute 越界
        "* 24 * * *",  // hour 越界
        "* * 32 * *",  // dom 越界
        "* * 0 * *",   // dom 下界 0
        "* * * 0 *",   // month 下界 0
        "* * * 13 *",  // month 上界 13
        "* * * * 7",   // dow 上界 7
    ];

    for bad in invalid {
        let r = CronExpr::parse(bad);
        assert!(r.is_err(), "应该 Err 但 Ok: {bad:?}");
    }
}

#[test]
fn unknown_shorthand_fails_with_descriptive_error() {
    let r = CronExpr::parse("@never");
    assert!(r.is_err());
    let err_msg = format!("{:?}", r.unwrap_err());
    assert!(
        err_msg.contains("@never"),
        "错误信息应含原值, got: {err_msg}"
    );
}

#[test]
fn schedule_validation_strict() {
    // interval_secs 必须 > 0
    assert!(Schedule::new("a", 1).validate().is_ok());
    assert!(Schedule::new("a", 60).validate().is_ok());
    assert!(Schedule::new("a", i64::MAX).validate().is_ok());
    assert!(Schedule::new("a", 0).validate().is_err());
    assert!(Schedule::new("a", -1).validate().is_err());
}
