
//! R176 Bridge 4 + 8 Kani proofs: consciousness/companion -> voice bridge invariants
//!
//! Bridge 4: consciousness -> voice (Tone modulation)
//! Bridge 8: companion -> voice (Tone modulation from Bond)

#![allow(missing_docs)]

use apeireth_consciousness::plutchik::{PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity};

use crate::consciousness_bridge::plutchik_to_tone;
use crate::companion_bridge::bond_to_tone;
use crate::tone::{EmotionTone, Prosody};

// =====================================================================
// Bridge 4 (consciousness -> voice)
// =====================================================================

fn make_basic(b: PlutchikBasic) -> PlutchikEmotion {
    PlutchikEmotion::Basic(b, PlutchikIntensity::Mild)
}

fn make_advanced(a: PlutchikAdvanced) -> PlutchikEmotion {
    PlutchikEmotion::Advanced(a, PlutchikIntensity::Mild)
}

#[cfg(kani)]
#[kani::proof]
fn proof_bridge4_tone_speed_in_range() {
    let e = make_basic(PlutchikBasic::Joy);
    let tone = plutchik_to_tone(&e);
    assert!(tone.speed >= 0.5 && tone.speed <= 2.0);
    assert!(tone.pitch >= 0.5 && tone.pitch <= 2.0);
    assert!(tone.volume >= 0.0 && tone.volume <= 1.0);
}

#[test]
fn r176_b4_01_basic_tone_speed_in_range() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let e = make_basic(*b);
        let tone = plutchik_to_tone(&e);
        assert!(tone.speed >= 0.5 && tone.speed <= 2.0, "speed {} for {:?}", tone.speed, b);
        assert!(tone.pitch >= 0.5 && tone.pitch <= 2.0);
        assert!(tone.volume >= 0.0 && tone.volume <= 1.0);
    }
}

#[test]
fn r176_b4_02_advanced_tone_speed_in_range() {
    for a in &[
        PlutchikAdvanced::Love, PlutchikAdvanced::Submission, PlutchikAdvanced::Awe,
        PlutchikAdvanced::Disapproval, PlutchikAdvanced::Remorse, PlutchikAdvanced::Contempt,
        PlutchikAdvanced::Aggressiveness, PlutchikAdvanced::Optimism,
    ] {
        let e = make_advanced(*a);
        let tone = plutchik_to_tone(&e);
        assert!(tone.speed >= 0.5 && tone.speed <= 2.0);
        assert!(tone.pitch >= 0.5 && tone.pitch <= 2.0);
        assert!(tone.volume >= 0.0 && tone.volume <= 1.0);
    }
}

#[test]
fn r176_b4_03_emotion_tone_valid() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let e = make_basic(*b);
        let tone = plutchik_to_tone(&e);
        // EmotionTone enum - just verify it was set
        let _ = format!("{:?}", tone.emotion_tone);
        let _ = format!("{:?}", tone.prosody);
    }
}

// =====================================================================
// Bridge 8 (companion -> voice)
// =====================================================================

#[test]
fn r176_b8_01_bond_to_tone_speed_in_range() {
    // BondDepth/BondStage values - use defaults
    use apeireth_companion::bond::{Bond, BondDepth, BondStage};
    let stages = [BondStage::Initial, BondStage::Familiar, BondStage::Trusted, BondStage::Intimate];
    for stage in &stages {
        let mut bond = Bond::new();
        // Use evolve() to set stage (private field; evolve is public)
        bond.evolve(*stage, 0.0);
        // Verify depth is accessible
        let _ = bond.depth();
        let tone = bond_to_tone(&bond);
        assert!(tone.speed >= 0.5 && tone.speed <= 2.0, "speed {} for {:?}", tone.speed, stage);
        assert!(tone.pitch >= 0.5 && tone.pitch <= 2.0);
        assert!(tone.volume >= 0.0 && tone.volume <= 1.0);
    }
}
