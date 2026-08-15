//! R177 tui-e2e organ Kani proofs (W11)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tui2_01_navpage_count() {
    assert_eq!(NavPage::COUNT, 5);
}

#[test]
fn r177_tui2_02_nav_count() {
    assert_eq!(Nav::COUNT, 5);
}

#[test]
fn r177_tui2_03_organ_count() {
    assert_eq!(Organ::COUNT, 9);
}

#[test]
fn r177_tui2_04_six_phi_anchors() {
    assert_eq!(SIX_PHI_ANCHORS.len(), 6);
}

#[test]
fn r177_tui2_05_eight_promises() {
    assert_eq!(EIGHT_PROMISES.len(), 8);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tui2_kani_01_navpage_count() {
    assert_eq!(NavPage::COUNT, 5);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tui2_kani_02_organ_count() {
    assert_eq!(Organ::COUNT, 9);
}

