//! R177 tool-runtime organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tr_01_borrowed_count() {
    assert_eq!(BORROWED_LEGACY_COUNT, 5);
}

#[test]
fn r177_tr_02_module_count() {
    assert_eq!(MODULE_COUNT, 5);
}

#[test]
fn r177_tr_03_default_timeout() {
    assert!(DEFAULT_TIMEOUT_MS > 0);
}

#[test]
fn r177_tr_04_max_fuzzy_distance() {
    assert!(MAX_FUZZY_DISTANCE > 0);
}

#[test]
fn r177_tr_05_sensitive_keys() {
    assert_eq!(SENSITIVE_KEY_COUNT, 13);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tr_kani_01_timeout_positive() {
    assert!(DEFAULT_TIMEOUT_MS > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tr_kani_02_fuzzy_positive() {
    assert!(MAX_FUZZY_DISTANCE > 0);
}
