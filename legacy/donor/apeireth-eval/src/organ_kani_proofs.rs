//! R177 eval organ Kani proofs (W9)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_eval_01_score_new() {
    let s = EvalScore::new("test", 0.8);
    assert_eq!(s.dimension, "test");
}

#[test]
fn r177_eval_02_score_valid() {
    let s = EvalScore::new("t", 0.8);
    assert!(s.is_valid());
}

#[test]
fn r177_eval_03_score_invalid() {
    let s = EvalScore::new("t", 2.0);
    assert!(!s.is_valid());
}

#[test]
fn r177_eval_04_score_nan() {
    let s = EvalScore::new("t", f64::NAN);
    assert!(!s.is_valid());
}

#[test]
fn r177_eval_05_score_value() {
    let s = EvalScore::new("t", 0.5);
    assert_eq!(s.value, 0.5);
}

#[cfg(kani)]
#[kani::proof]
fn r177_eval_kani_01_valid_bounded() {
    let s = EvalScore::new("t", 0.5);
    assert!(s.value >= 0.0 && s.value <= 1.0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_eval_kani_02_invalid_unbounded() {
    let s = EvalScore::new("t", 1.5);
    assert!(!s.is_valid());
}
