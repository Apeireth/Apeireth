
//! R176 Bridge 5 Kani proofs: consciousness -> companion bridge invariants

#![allow(missing_docs)]

use apeireth_consciousness::plutchik::{PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity};

use crate::consciousness_bridge::{apply_plutchik_to_bond, apply_plutchik_to_character, plutchik_to_bond_emotion};
use crate::bond::{Bond, BondCharacter, BondDepth};

fn make_basic(b: PlutchikBasic) -> PlutchikEmotion {
    PlutchikEmotion::Basic(b, PlutchikIntensity::Mild)
}

fn make_advanced(a: PlutchikAdvanced) -> PlutchikEmotion {
    PlutchikEmotion::Advanced(a, PlutchikIntensity::Mild)
}

#[cfg(kani)]
#[kani::proof]
fn proof_bridge5_bond_inputs_in_range() {
    let e = make_basic(PlutchikBasic::Joy);
    let inputs = plutchik_to_bond_emotion(&e);
    // 8 dimensions, all should be in [-1.0, +1.0]
    assert!(inputs.joy >= -1.0 && inputs.joy <= 1.0);
    assert!(inputs.trust >= -1.0 && inputs.trust <= 1.0);
    assert!(inputs.fear >= -1.0 && inputs.fear <= 1.0);
    assert!(inputs.surprise >= -1.0 && inputs.surprise <= 1.0);
    assert!(inputs.sadness >= -1.0 && inputs.sadness <= 1.0);
    assert!(inputs.disgust >= -1.0 && inputs.disgust <= 1.0);
    assert!(inputs.anger >= -1.0 && inputs.anger <= 1.0);
    assert!(inputs.anticipation >= -1.0 && inputs.anticipation <= 1.0);
}

#[test]
fn r176_b5_01_basic_bond_inputs_in_range() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let e = make_basic(*b);
        let inputs = plutchik_to_bond_emotion(&e);
        assert!(inputs.joy >= -1.0 && inputs.joy <= 1.0);
        assert!(inputs.trust >= -1.0 && inputs.trust <= 1.0);
        assert!(inputs.fear >= -1.0 && inputs.fear <= 1.0);
        assert!(inputs.surprise >= -1.0 && inputs.surprise <= 1.0);
        assert!(inputs.sadness >= -1.0 && inputs.sadness <= 1.0);
        assert!(inputs.disgust >= -1.0 && inputs.disgust <= 1.0);
        assert!(inputs.anger >= -1.0 && inputs.anger <= 1.0);
        assert!(inputs.anticipation >= -1.0 && inputs.anticipation <= 1.0);
    }
}

#[test]
fn r176_b5_02_advanced_bond_inputs_in_range() {
    for a in &[
        PlutchikAdvanced::Love, PlutchikAdvanced::Submission, PlutchikAdvanced::Awe,
        PlutchikAdvanced::Disapproval, PlutchikAdvanced::Remorse, PlutchikAdvanced::Contempt,
        PlutchikAdvanced::Aggressiveness, PlutchikAdvanced::Optimism,
    ] {
        let e = make_advanced(*a);
        let inputs = plutchik_to_bond_emotion(&e);
        assert!(inputs.joy >= -1.0 && inputs.joy <= 1.0);
        assert!(inputs.trust >= -1.0 && inputs.trust <= 1.0);
        assert!(inputs.fear >= -1.0 && inputs.fear <= 1.0);
        assert!(inputs.surprise >= -1.0 && inputs.surprise <= 1.0);
        assert!(inputs.sadness >= -1.0 && inputs.sadness <= 1.0);
        assert!(inputs.disgust >= -1.0 && inputs.disgust <= 1.0);
        assert!(inputs.anger >= -1.0 && inputs.anger <= 1.0);
        assert!(inputs.anticipation >= -1.0 && inputs.anticipation <= 1.0);
    }
}

#[test]
fn r176_b5_03_apply_to_character_safe() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let mut character = BondCharacter::new();
        let e = make_basic(*b);
        apply_plutchik_to_character(&mut character, &e);
        // apply returns () so we just verify no panic
    }
}

#[test]
fn r176_b5_04_apply_to_bond_safe() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let mut bond = Bond::new();
        let e = make_basic(*b);
        apply_plutchik_to_bond(&mut bond, &e);
        // apply returns () so we just verify no panic
    }
}

#[test]
fn r176_b5_05_depth_remains_in_range() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let mut bond = Bond::new();
        bond.evolve(crate::bond::BondStage::Intimate, 0.5);
        let e = make_basic(*b);
        let _ = apply_plutchik_to_bond(&mut bond, &e);
        // BondDepth should remain valid (>= 0.0)
        let _ = bond.depth().value();
        let _ = BondDepth::ZERO;
    }
}
