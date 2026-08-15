//! R177 context-fold organ Kani proofs (W11)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_cf_01_deliverables() {
    assert_eq!(R144_DELIVERABLES, 3);
}

#[test]
fn r177_cf_02_fold_strategy() {
    let s = FoldStrategy::Truncate;
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_cf_03_marker_kind() {
    let k = MarkerKind::Full;
    let _: String = format!("{:?}", k);
}

#[test]
fn r177_cf_04_fold_marker() {
    let m = FoldMarker::new(MarkerKind::Full, "hello");
    assert_eq!(m.payload, "hello");
}

#[test]
fn r177_cf_05_token_accumulator() {
    let a = TokenAccumulator::new();
    let n = a.total_tokens();
    assert_eq!(n, 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cf_kani_01_deliverables() {
    assert_eq!(R144_DELIVERABLES, 3);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cf_kani_02_strategy_invariant() {
    let s = FoldStrategy::Truncate;
    assert!(!format!("{:?}", s).is_empty());
}
