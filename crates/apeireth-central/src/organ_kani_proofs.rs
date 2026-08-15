//! R177 central organ Kani proofs (W8)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_cen_01_component_count() {
    assert_eq!(COMPONENT_COUNT, 17);
}

#[test]
fn r177_cen_02_stage_count() {
    assert_eq!(STAGE_COUNT, 10);
}

#[test]
fn r177_cen_03_legal_transitions_positive() {
    assert!(LEGAL_TRANSITION_COUNT >= 1);
}

#[test]
fn r177_cen_04_v05_threshold() {
    assert_eq!(V05_MATURITY_THRESHOLD_MILLI, 850);
}

#[test]
fn r177_cen_05_central_new() {
    let c = ApeirethCentral::default(); let _: String = format!("{:?}", c);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cen_kani_01_threshold_invariant() {
    assert!(V05_MATURITY_THRESHOLD_MILLI >= 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cen_kani_02_components_invariant() {
    assert!(COMPONENT_COUNT == 17);
}

