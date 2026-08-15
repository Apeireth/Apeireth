//! R177 gateway organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_gw_01_modes_supported() {
    assert_eq!(MODES_SUPPORTED, 1);
}

#[test]
fn r177_gw_02_node_kinds() {
    assert_eq!(NODE_KINDS, 5);
}

#[test]
fn r177_gw_03_modules() {
    assert_eq!(MODULES, 7);
}

#[test]
fn r177_gw_04_modes_positive() {
    assert!(MODES_SUPPORTED >= 1);
}

#[test]
fn r177_gw_05_modules_positive() {
    assert!(MODULES >= 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_gw_kani_01_modes_invariant() {
    assert!(MODES_SUPPORTED <= 10);
}

#[cfg(kani)]
#[kani::proof]
fn r177_gw_kani_02_modules_invariant() {
    assert!(MODULES >= 1 && MODULES <= 20);
}

