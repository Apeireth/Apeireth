//! R177 tools organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_ts_01_borrowed_legacy() {
    assert_eq!(BORROWED_LEGACY_FIELDS, 5);
}

#[test]
fn r177_ts_02_trait_count() {
    assert_eq!(TRAIT_COUNT, 7);
}

#[test]
fn r177_ts_03_impl_count() {
    assert_eq!(IMPL_COUNT, 6);
}

#[test]
fn r177_ts_04_git_op_count() {
    assert_eq!(GIT_OPS_OP_COUNT, 3);
}

#[test]
fn r177_ts_05_code_exec_op_count() {
    assert_eq!(CODE_EXEC_OP_COUNT, 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ts_kani_01_trait_count_invariant() {
    assert_eq!(TRAIT_COUNT, 7);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ts_kani_02_impl_count_invariant() {
    assert_eq!(IMPL_COUNT, 6);
}

