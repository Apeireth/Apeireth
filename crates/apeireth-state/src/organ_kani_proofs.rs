//! R177 state organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_st_01_platform_name() {
    assert_eq!(PLATFORM_NAME, "apeireth");
}

#[test]
fn r177_st_02_schema_version() {
    assert_eq!(APEIRETH_STATE_SCHEMA_VERSION, "1");
}

#[test]
fn r177_st_03_borrowed_count() {
    assert_eq!(BORROWED_GOLUTRA_STATE_COUNT, 9);
}

#[test]
fn r177_st_04_mode_count() {
    assert_eq!(STATE_MODE_COUNT, 3);
}

#[test]
fn r177_st_05_error_count() {
    assert_eq!(STATE_ERROR_COUNT, 5);
}

#[cfg(kani)]
#[kani::proof]
fn r177_st_kani_01_modes_positive() {
    assert!(STATE_MODE_COUNT >= 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_st_kani_02_errors_positive() {
    assert!(STATE_ERROR_COUNT >= 1);
}

