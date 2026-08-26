//! R177 companion organ Kani proofs (W2)

#![allow(missing_docs)]

use crate::bond::{Bond, BondCharacter, BondDepth, BondStage};

#[test]
fn r177_cmp_01_bond_stages_7() {
    assert_eq!(BondStage::ALL.len(), 7);
}

#[test]
fn r177_cmp_02_bond_stage_labels_distinct() {
    let labels: Vec<&str> = BondStage::ALL.iter().map(|s| s.label()).collect();
    let mut seen = std::collections::HashSet::new();
    for l in &labels {
        assert!(seen.insert(*l), "label 重复: {}", l);
    }
}

#[test]
fn r177_cmp_03_bond_stage_is_terminal() {
    assert!(BondStage::Ended.is_terminal());
    assert!(!BondStage::Initial.is_terminal());
    assert!(!BondStage::LongTerm.is_terminal());
    assert!(!BondStage::Paused.is_terminal());
}

#[test]
fn r177_cmp_04_bond_depth_zero_one() {
    assert_eq!(BondDepth::ZERO.value(), 0.0);
    assert_eq!(BondDepth::ONE.value(), 1.0);
}

#[test]
fn r177_cmp_05_bond_depth_clamps() {
    assert_eq!(BondDepth::new(2.0).value(), 1.0);
    assert_eq!(BondDepth::new(-1.0).value(), 0.0);
    assert_eq!(BondDepth::new(0.5).value(), 0.5);
}

#[test]
fn r177_cmp_06_bond_new_initial_state() {
    let b = Bond::new();
    assert_eq!(b.stage(), BondStage::Initial);
    assert_eq!(b.depth(), BondDepth::ZERO);
}

#[test]
fn r177_cmp_07_bond_evolve_advances() {
    let mut b = Bond::new();
    b.evolve(BondStage::Trusted, 0.5);
    assert_eq!(b.stage(), BondStage::Trusted);
    assert!((b.depth().value() - 0.5).abs() < 1e-6);
}

#[test]
fn r177_cmp_08_bond_evolve_clamps() {
    let mut b = Bond::new();
    b.evolve(BondStage::Trusted, 2.0);
    assert_eq!(b.depth().value(), 1.0, "depth > 1 应 clamp 到 1");
}

#[test]
fn r177_cmp_09_bond_character_default_zero() {
    let c = BondCharacter::new();
    assert_eq!(c.interdependency, 0.0);
    assert_eq!(c.resilience, 0.0);
    assert_eq!(c.resonance, 0.0);
    assert_eq!(c.creativity, 0.0);
    assert_eq!(c.trust, 0.0);
}

#[test]
fn r177_cmp_10_bond_character_apply_emotion_clamps() {
    let mut c = BondCharacter::new();
    c.apply_emotion(2.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0);
    assert!(c.resonance <= 1.0, "resonance 应 clamp 到 [0,1]");
    assert!(c.resonance >= 0.0);
    assert!(c.trust <= 1.0);
}

#[test]
fn r177_cmp_11_bond_character_serialize_5_keys() {
    let c = BondCharacter::new();
    let s = c.serialize();
    assert_eq!(s.len(), 5);
    assert!(s.contains_key("interdependency"));
    assert!(s.contains_key("resilience"));
    assert!(s.contains_key("resonance"));
    assert!(s.contains_key("creativity"));
    assert!(s.contains_key("trust"));
}

#[test]
fn r177_cmp_12_bond_apply_emotion_via_bond() {
    let mut b = Bond::new();
    b.apply_emotion(1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    let c = b.character();
    assert!(c.trust > 0.0, "apply_emotion 后 trust 应 > 0");
    assert!(c.resonance > 0.0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cmp_kani_01_bond_depth_clamps() {
    assert_eq!(BondDepth::new(2.0).value(), 1.0);
    assert_eq!(BondDepth::new(-1.0).value(), 0.0);
    assert_eq!(BondDepth::new(0.5).value(), 0.5);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cmp_kani_02_bond_stage_count() {
    assert_eq!(BondStage::ALL.len(), 7);
}
