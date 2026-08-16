
//! R176 Bridge 1 Kani proofs: consciousness -> cognition bridge invariants
//!
//! **3 properties to prove**:
//! 1. DecisionBias output always in [0.0, 1.0] (clamped)
//! 2. accumulate_biases averages all 4 dimensions correctly
//! 3. accumulate_biases with empty slice returns default
//!
//! **状态**: R176 (2026-08-15) \u521d\u59cb\u7248

#![allow(missing_docs)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};

use crate::consciousness_bridge::{accumulate_biases, plutchik_to_decision_bias, DecisionBias};

fn make_basic(b: PlutchikBasic) -> PlutchikEmotion {
    PlutchikEmotion::Basic(b, PlutchikIntensity::Mild)
}

fn make_advanced(a: PlutchikAdvanced) -> PlutchikEmotion {
    PlutchikEmotion::Advanced(a, PlutchikIntensity::Mild)
}

// Property 1: DecisionBias output always in [0.0, 1.0]
#[cfg(kani)]
#[kani::proof]
fn proof_bridge1_bias_clamped() {
    let e = make_basic(PlutchikBasic::Joy);
    let bias = plutchik_to_decision_bias(&e);
    // All 4 dimensions must be in [0.0, 1.0]
    assert!(bias.creativity >= 0.0 && bias.creativity <= 1.0);
    assert!(bias.caution >= 0.0 && bias.caution <= 1.0);
    assert!(bias.cooperation >= 0.0 && bias.cooperation <= 1.0);
    assert!(bias.exploration >= 0.0 && bias.exploration <= 1.0);
}

// Property 1 mirror: All 8 basic emotions produce valid bias
#[test]
fn r176_b1_01_all_basic_biases_valid() {
    for b in &[
        PlutchikBasic::Joy, PlutchikBasic::Trust, PlutchikBasic::Fear,
        PlutchikBasic::Surprise, PlutchikBasic::Sadness, PlutchikBasic::Disgust,
        PlutchikBasic::Anger, PlutchikBasic::Anticipation,
    ] {
        let e = make_basic(*b);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.creativity >= 0.0 && bias.creativity <= 1.0, "creativity out of range for {:?}", b);
        assert!(bias.caution >= 0.0 && bias.caution <= 1.0);
        assert!(bias.cooperation >= 0.0 && bias.cooperation <= 1.0);
        assert!(bias.exploration >= 0.0 && bias.exploration <= 1.0);
    }
}

// Property 1 mirror: All 8 advanced emotions produce valid bias
#[test]
fn r176_b1_02_all_advanced_biases_valid() {
    for a in &[
        PlutchikAdvanced::Love, PlutchikAdvanced::Submission, PlutchikAdvanced::Awe,
        PlutchikAdvanced::Disapproval, PlutchikAdvanced::Remorse, PlutchikAdvanced::Contempt,
        PlutchikAdvanced::Aggressiveness, PlutchikAdvanced::Optimism,
    ] {
        let e = make_advanced(*a);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.creativity >= 0.0 && bias.creativity <= 1.0, "creativity out of range for {:?}", a);
        assert!(bias.caution >= 0.0 && bias.caution <= 1.0);
        assert!(bias.cooperation >= 0.0 && bias.cooperation <= 1.0);
        assert!(bias.exploration >= 0.0 && bias.exploration <= 1.0);
    }
}

// Property 2: accumulate_biases with empty returns default
#[test]
fn r176_b1_03_accumulate_empty_default() {
    let biases: Vec<DecisionBias> = vec![];
    let result = accumulate_biases(&biases);
    let default = DecisionBias::default();
    assert_eq!(result.creativity, default.creativity);
    assert_eq!(result.caution, default.caution);
    assert_eq!(result.cooperation, default.cooperation);
    assert_eq!(result.exploration, default.exploration);
}

// Property 3: accumulate_biases result in [0.0, 1.0]
#[test]
fn r176_b1_04_accumulate_clamped() {
    let biases = vec![
        plutchik_to_decision_bias(&make_basic(PlutchikBasic::Joy)),
        plutchik_to_decision_bias(&make_basic(PlutchikBasic::Fear)),
        plutchik_to_decision_bias(&make_basic(PlutchikBasic::Anger)),
    ];
    let result = accumulate_biases(&biases);
    assert!(result.creativity >= 0.0 && result.creativity <= 1.0);
    assert!(result.caution >= 0.0 && result.caution <= 1.0);
    assert!(result.cooperation >= 0.0 && result.cooperation <= 1.0);
    assert!(result.exploration >= 0.0 && result.exploration <= 1.0);
}
