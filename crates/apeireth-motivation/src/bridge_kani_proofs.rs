//! R176 Bridge 3 Kani proofs: consciousness -> motivation bridge invariants
//! R176 Bridge 6 Kani proofs: life-force -> motivation bridge invariants

#![allow(missing_docs)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};

use crate::consciousness_bridge::{
    apply_plutchik_to_external_drive, apply_plutchik_to_internal_drive,
    plutchik_to_motivation_adjustment,
};
use crate::life_force_bridge::{
    apply_life_force_to_external_drive, apply_life_force_to_internal_drive,
    life_force_to_motivation_adjustment,
};
use crate::{ExternalDrive, InternalDrive};
use apeireth_core::IdentityCard;
use apeireth_life_force::LifeForce;

fn make_basic(b: PlutchikBasic) -> PlutchikEmotion {
    PlutchikEmotion::Basic(b, PlutchikIntensity::Mild)
}

fn make_advanced(a: PlutchikAdvanced) -> PlutchikEmotion {
    PlutchikEmotion::Advanced(a, PlutchikIntensity::Mild)
}

fn fresh_internal_drive() -> InternalDrive {
    InternalDrive {
        label: "test".into(),
        intensity: 0.5,
    }
}

fn fresh_external_drive() -> ExternalDrive {
    ExternalDrive {
        label: "test".into(),
        intensity: 0.5,
    }
}

fn fresh_life_force() -> LifeForce {
    let card = IdentityCard {
        continuity_id: "test-id".into(),
        birth_time: 1000,
        carriers: vec!["test-carrier".into()],
        migration_history: vec![],
    };
    LifeForce::new(card, 1000)
}

// =====================================================================
// Bridge 3 (consciousness -> motivation)
// =====================================================================

#[cfg(kani)]
#[kani::proof]
fn proof_bridge3_internal_delta_clamped() {
    let e = make_basic(PlutchikBasic::Joy);
    let adj = plutchik_to_motivation_adjustment(&e);
    assert!(adj.internal_drive_delta >= -0.2 && adj.internal_drive_delta <= 0.2);
    assert!(adj.external_drive_delta >= -0.2 && adj.external_drive_delta <= 0.2);
}

#[test]
fn r176_b3_01_basic_deltas_clamped() {
    for b in &[
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Fear,
        PlutchikBasic::Surprise,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
        PlutchikBasic::Anger,
        PlutchikBasic::Anticipation,
    ] {
        let e = make_basic(*b);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(adj.internal_drive_delta >= -0.2 && adj.internal_drive_delta <= 0.2);
        assert!(adj.external_drive_delta >= -0.2 && adj.external_drive_delta <= 0.2);
    }
}

#[test]
fn r176_b3_02_advanced_deltas_clamped() {
    for a in &[
        PlutchikAdvanced::Love,
        PlutchikAdvanced::Submission,
        PlutchikAdvanced::Awe,
        PlutchikAdvanced::Disapproval,
        PlutchikAdvanced::Remorse,
        PlutchikAdvanced::Contempt,
        PlutchikAdvanced::Aggressiveness,
        PlutchikAdvanced::Optimism,
    ] {
        let e = make_advanced(*a);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(adj.internal_drive_delta >= -0.2 && adj.internal_drive_delta <= 0.2);
        assert!(adj.external_drive_delta >= -0.2 && adj.external_drive_delta <= 0.2);
    }
}

#[test]
fn r176_b3_03_apply_internal_drive_safe() {
    for b in &[
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Fear,
        PlutchikBasic::Surprise,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
        PlutchikBasic::Anger,
        PlutchikBasic::Anticipation,
    ] {
        for start in &[0.3, 0.5, 0.7] {
            let mut drive = fresh_internal_drive();
            drive.intensity = *start;
            let e = make_basic(*b);
            let new_intensity = apply_plutchik_to_internal_drive(&mut drive, &e);
            assert!(
                new_intensity >= 0.0 && new_intensity <= 1.0,
                "intensity {} out of range for {:?} from {}",
                new_intensity,
                b,
                start
            );
        }
    }
}

#[test]
fn r176_b3_04_apply_external_drive_safe() {
    for b in &[
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Fear,
        PlutchikBasic::Surprise,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
        PlutchikBasic::Anger,
        PlutchikBasic::Anticipation,
    ] {
        for start in &[0.3, 0.5, 0.7] {
            let mut drive = fresh_external_drive();
            drive.intensity = *start;
            let e = make_basic(*b);
            let new_intensity = apply_plutchik_to_external_drive(&mut drive, &e);
            assert!(
                new_intensity >= 0.0 && new_intensity <= 1.0,
                "intensity {} out of range for {:?} from {}",
                new_intensity,
                b,
                start
            );
        }
    }
}

#[test]
fn r176_b3_05_sgi_intensity_valid() {
    for b in &[
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Fear,
        PlutchikBasic::Surprise,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
        PlutchikBasic::Anger,
        PlutchikBasic::Anticipation,
    ] {
        let e = make_basic(*b);
        let adj = plutchik_to_motivation_adjustment(&e);
        if let Some(sgi) = &adj.sgi_suggestion {
            assert!(
                sgi.intensity >= 0.0 && sgi.intensity <= 1.0,
                "sgi intensity {} out of range for {:?}",
                sgi.intensity,
                b
            );
        }
    }
}

// =====================================================================
// Bridge 6 (life-force -> motivation)
// =====================================================================

#[test]
fn r176_b6_01_life_force_adjustment_valid() {
    let life = fresh_life_force();
    for endurance in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        let mut l = life.clone();
        l.endurance = *endurance;
        let adj = life_force_to_motivation_adjustment(&l, 1000);
        // adjustment multiplier in [0.3, 1.5] (per spec)
        assert!(
            adj.drive_intensity_multiplier >= 0.3 && adj.drive_intensity_multiplier <= 1.5,
            "drive_intensity_multiplier {} out of range for endurance {}",
            adj.drive_intensity_multiplier,
            endurance
        );
    }
}

#[test]
fn r176_b6_02_apply_life_force_to_internal_safe() {
    for endurance in &[0.3, 0.5, 0.7] {
        let mut drive = fresh_internal_drive();
        drive.intensity = 0.5;
        let life = fresh_life_force();
        let mut l = life.clone();
        l.endurance = *endurance;
        let new_intensity = apply_life_force_to_internal_drive(&mut drive, &l, 1000);
        assert!(
            new_intensity >= 0.0 && new_intensity <= 1.0,
            "intensity {} out of range for endurance {}",
            new_intensity,
            endurance
        );
    }
}

#[test]
fn r176_b6_03_apply_life_force_to_external_safe() {
    for endurance in &[0.3, 0.5, 0.7] {
        let mut drive = fresh_external_drive();
        drive.intensity = 0.5;
        let life = fresh_life_force();
        let mut l = life.clone();
        l.endurance = *endurance;
        let new_intensity = apply_life_force_to_external_drive(&mut drive, &l, 1000);
        assert!(
            new_intensity >= 0.0 && new_intensity <= 1.0,
            "intensity {} out of range for endurance {}",
            new_intensity,
            endurance
        );
    }
}
