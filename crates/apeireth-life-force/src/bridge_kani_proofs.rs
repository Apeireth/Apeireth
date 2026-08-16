//! R176 Bridge 2 Kani proofs: consciousness -> life-force bridge invariants

#![allow(missing_docs)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};
use apeireth_core::IdentityCard;

use crate::consciousness_bridge::{
    apply_plutchik_to_life_force, plutchik_to_life_force_adjustment,
};
use crate::LifeForce;

fn make_basic(b: PlutchikBasic) -> PlutchikEmotion {
    PlutchikEmotion::Basic(b, PlutchikIntensity::Mild)
}

fn make_advanced(a: PlutchikAdvanced) -> PlutchikEmotion {
    PlutchikEmotion::Advanced(a, PlutchikIntensity::Mild)
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

#[cfg(kani)]
#[kani::proof]
fn proof_bridge2_endurance_delta_clamped() {
    let e = make_basic(PlutchikBasic::Joy);
    let adj = plutchik_to_life_force_adjustment(&e);
    assert!(adj.endurance_delta >= -0.2 && adj.endurance_delta <= 0.2);
}

#[test]
fn r176_b2_01_basic_endurance_delta_clamped() {
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
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(
            adj.endurance_delta >= -0.2 && adj.endurance_delta <= 0.2,
            "endurance_delta {} out of range for {:?}",
            adj.endurance_delta,
            b
        );
    }
}

#[test]
fn r176_b2_02_advanced_endurance_delta_clamped() {
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
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(
            adj.endurance_delta >= -0.2 && adj.endurance_delta <= 0.2,
            "endurance_delta {} out of range for {:?}",
            adj.endurance_delta,
            a
        );
    }
}

#[test]
fn r176_b2_03_apply_returns_ok_for_valid_input() {
    // Note: skip starting from 1.0 because Joy+Trust+Anger stack would overflow
    // before clamping. Bridge code's apply_plutchik_to_life_force calls
    // validate_endurance which rejects >1.0 - this is correct behavior (Err result)
    // start values: 0.3 (positive only), 0.5 (both), 0.7, 0.9 (negative only)
    for start in &[0.3, 0.5, 0.7] {
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
            let mut life = fresh_life_force();
            life.endurance = *start;
            let e = make_basic(*b);
            let result = apply_plutchik_to_life_force(&mut life, &e, 1000);
            assert!(
                result.is_ok(),
                "apply failed for {:?} from {}: {:?}",
                b,
                start,
                result
            );
            assert!(
                life.endurance >= 0.0 && life.endurance <= 1.0,
                "endurance {} out of range after apply for {:?} from {}",
                life.endurance,
                b,
                start
            );
        }
    }
}

// Property (R177 fix): clamp + validate - starting from 1.0 with positive emotion clamps to 1.0 (Ok, not Err)
#[test]
fn r177_b2_03b_clamp_instead_of_err() {
    for b in &[
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Anticipation,
    ] {
        let mut life = fresh_life_force();
        life.endurance = 1.0;
        let e = make_basic(*b);
        let result = apply_plutchik_to_life_force(&mut life, &e, 1000);
        // R177 fix: clamp to [0,1] before validate, so Ok with endurance = 1.0
        assert!(
            result.is_ok(),
            "expected Ok after clamp for {:?} from 1.0",
            b
        );
        assert_eq!(
            life.endurance, 1.0,
            "endurance should be clamped to 1.0 for {:?}",
            b
        );
    }
}

// Property (R177 fix): starting from 0.0 with negative emotion clamps to 0.0 (Ok, not Err)
#[test]
fn r177_b2_03c_clamp_zero_instead_of_err() {
    for b in &[
        PlutchikBasic::Fear,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
    ] {
        let mut life = fresh_life_force();
        life.endurance = 0.0;
        let e = make_basic(*b);
        let result = apply_plutchik_to_life_force(&mut life, &e, 1000);
        assert!(
            result.is_ok(),
            "expected Ok after clamp for {:?} from 0.0",
            b
        );
        assert_eq!(
            life.endurance, 0.0,
            "endurance should be clamped to 0.0 for {:?}",
            b
        );
    }
}

#[test]
fn r176_b2_04_trigger_reason_consistent() {
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
        let adj = plutchik_to_life_force_adjustment(&e);
        if adj.should_trigger_reflection {
            assert!(
                adj.reflection_reason.is_some(),
                "triggered but no reason for {:?}",
                b
            );
        } else {
            assert!(
                adj.reflection_reason.is_none(),
                "not triggered but reason set for {:?}",
                b
            );
        }
    }
}
