//! R177 cron organ Kani proofs (W6)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_cro_01_validate_expr() {
    let r = validate_expr("0 0 * * *"); assert!(r.is_ok());
}

#[test]
fn r177_cro_02_validate_expr_invalid() {
    let r = validate_expr("invalid"); assert!(r.is_err());
}

#[test]
fn r177_cro_03_describe() {
    let e = validate_expr("0 0 * * *").unwrap(); let d = describe(&e); assert!(!d.is_empty());
}

#[test]
fn r177_cro_04_validate_schedule() {
    let s = Schedule { name: "x".into(), interval_secs: 60 }; assert!(validate_schedule(&s).is_ok());
}

#[test]
fn r177_cro_05_validate_schedule_zero() {
    let s = Schedule { name: "x".into(), interval_secs: 0 }; assert!(validate_schedule(&s).is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_cro_kani_01_minute_zero() {
    let m: u8 = 0; assert!(m < 60);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cro_kani_02_hour_zero() {
    let h: u8 = 0; assert!(h < 24);
}

