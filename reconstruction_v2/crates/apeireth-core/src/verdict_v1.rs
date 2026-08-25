//! v1-compatible 13-key `PhilosophyKey` + `PhilosophyVerdict` + `verdict_for_target`.
//!
//! **Why v2-era keeps v1 API surface**:
//! - cognition / action / constraint / governance downstream crates reference `apeireth_core::PhilosophyKey::NotClone` etc.
//! - The 12+1 key set is **compile-time hardcoded** — any addition / removal must be propagated everywhere.
//! - v2's `philosophy.rs` defines a different `K1_ApeironEmergence ... K13_GracefulDegradation` taxonomy (PEACE anchors).
//!   This v1-style enum is the **verdict taxonomy** (L0-hardcode + PHL-01..PHL-07). Different semantic, kept separate.
//!
//! **Prohibition**: do not modify this enum (PHL-07 no drift).

use serde::{Deserialize, Serialize};

use crate::action_target::ActionTarget;

/// 13 philosophy keys (v1 LOCKED taxonomy, v2 compatible).
///
/// 3 + 3 + 3 + 1 + 1 + 1 + 1 = 13, grouped under 7 PHLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhilosophyKey {
    // V3 PHL-01 not_X (LOCKED 3)
    /// PHL-01 not_clone: do not pretend to clone / homogenize.
    NotClone,
    /// PHL-01 not_perfect: do not pretend to be perfect / 100%.
    NotPerfect,
    /// PHL-01 not_uuid: do not pretend to a unique solution / unique truth.
    NotUuid,

    // V3 PHL-02b not_X (LOCKED 3)
    /// PHL-02b not_undo: do not pretend to undo the past.
    NotUndo,
    /// PHL-02b not_proof: do not pretend to full proof.
    NotProof,
    /// PHL-02b not_safe: do not pretend to absolute safety.
    NotSafe,

    // V3 PHL-03 X_is_not_Y (LOCKED 3)
    /// PHL-03 spec_is_not_proof: spec is not proof.
    SpecIsNotProof,
    /// PHL-03 counterexample_is_not_bug: counterexample is not bug.
    CounterexampleIsNotBug,
    /// PHL-03 prover_is_not_truth: prover is not truth.
    ProverIsNotTruth,

    // v4.1 PHL-04/05/06 (3)
    /// PHL-04 not_pretend_unobservable: do not pretend internal state is unobservable.
    NotUnobservable,
    /// PHL-05 not_pretend_unscientific: do not pretend decisions are unscientific.
    NotUnscientific,
    /// PHL-06 not_pretend_no_self_relation: do not pretend to have no relation to self.
    NotSelfRelationless,

    // R125-12 PHL-07 (1)
    /// PHL-07 not_pretend_unoptimizable: do not pretend code/system/reasoning is optimal.
    NotUnoptimizable,
}

impl PhilosophyKey {
    /// Human-readable description.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::NotClone => "do not pretend to clone",
            Self::NotPerfect => "do not pretend to be perfect",
            Self::NotUuid => "do not pretend to unique truth",
            Self::NotUndo => "do not pretend to undo",
            Self::NotProof => "do not pretend to proof",
            Self::NotSafe => "do not pretend to absolute safety",
            Self::SpecIsNotProof => "spec is not proof",
            Self::CounterexampleIsNotBug => "counterexample is not bug",
            Self::ProverIsNotTruth => "prover is not truth",
            Self::NotUnobservable => "PHL-04 do not pretend unobservable",
            Self::NotUnscientific => "PHL-05 do not pretend unscientific",
            Self::NotSelfRelationless => "PHL-06 do not pretend no self relation",
            Self::NotUnoptimizable => "PHL-07 do not pretend unoptimizable",
        }
    }

    /// Group id (1=PHL-01, 2=PHL-02b, 3=PHL-03, 4=PHL-04, 5=PHL-05, 6=PHL-06, 7=PHL-07).
    pub const fn group_id(&self) -> u8 {
        match self {
            Self::NotClone | Self::NotPerfect | Self::NotUuid => 1,
            Self::NotUndo | Self::NotProof | Self::NotSafe => 2,
            Self::SpecIsNotProof | Self::CounterexampleIsNotBug | Self::ProverIsNotTruth => 3,
            Self::NotUnobservable => 4,
            Self::NotUnscientific => 5,
            Self::NotSelfRelationless => 6,
            Self::NotUnoptimizable => 7,
        }
    }

    /// Total number of variants.
    pub const VARIANT_COUNT: usize = 13;
}

/// 13 keys complete list (compile-time hardcode).
pub const ALL_THIRTEEN_KEYS: [PhilosophyKey; 13] = [
    // V3 PHL-01 not_X (LOCKED)
    PhilosophyKey::NotClone,
    PhilosophyKey::NotPerfect,
    PhilosophyKey::NotUuid,
    // V3 PHL-02b not_X (LOCKED)
    PhilosophyKey::NotUndo,
    PhilosophyKey::NotProof,
    PhilosophyKey::NotSafe,
    // V3 PHL-03 X_is_not_Y (LOCKED)
    PhilosophyKey::SpecIsNotProof,
    PhilosophyKey::CounterexampleIsNotBug,
    PhilosophyKey::ProverIsNotTruth,
    // v4.1 PHL-04/05/06 (3)
    PhilosophyKey::NotUnobservable,
    PhilosophyKey::NotUnscientific,
    PhilosophyKey::NotSelfRelationless,
    // R125-12 PHL-07 (1)
    PhilosophyKey::NotUnoptimizable,
];

/// Backward-compat alias (array actually has 13 entries despite legacy name).
pub const ALL_TWELVE_KEYS: [PhilosophyKey; 13] = ALL_THIRTEEN_KEYS;

/// 13-key verdict.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhilosophyVerdict {
    /// Allowed by all 13 keys.
    Allow,
    /// Blocked by a specific key.
    Block(PhilosophyKey),
}

/// Compile-time hardcoded verdict routing — each `ActionTarget` is locked to a specific `PhilosophyKey`.
///
/// This is the **real implementation of v6 gate 1 (compile-time hardcode)**:
/// Rust's match exhaustiveness checker enforces that any new `ActionTarget` variant
/// must be handled here, otherwise the project does not compile.
pub const fn verdict_for_target(target: &ActionTarget) -> PhilosophyVerdict {
    match target {
        // L0 hardcode (PHL-04 / 02b / 06)
        ActionTarget::ModifyL0HA => PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable),
        ActionTarget::ReorganizeOnion => PhilosophyVerdict::Block(PhilosophyKey::NotProof),
        ActionTarget::ModifyEvolutionL0 => {
            PhilosophyVerdict::Block(PhilosophyKey::NotSelfRelationless)
        }
        // PHL-01 not_X (3)
        ActionTarget::PretendClone => PhilosophyVerdict::Block(PhilosophyKey::NotClone),
        ActionTarget::PretendPerfect => PhilosophyVerdict::Block(PhilosophyKey::NotPerfect),
        ActionTarget::PretendUuid => PhilosophyVerdict::Block(PhilosophyKey::NotUuid),
        // PHL-02b not_X (remaining 2)
        ActionTarget::PretendUndo => PhilosophyVerdict::Block(PhilosophyKey::NotUndo),
        ActionTarget::PretendSafe => PhilosophyVerdict::Block(PhilosophyKey::NotSafe),
        // PHL-03 X_is_not_Y (3)
        ActionTarget::PretendSpecIsProof => {
            PhilosophyVerdict::Block(PhilosophyKey::SpecIsNotProof)
        }
        ActionTarget::PretendCounterexampleIsBug => {
            PhilosophyVerdict::Block(PhilosophyKey::CounterexampleIsNotBug)
        }
        ActionTarget::PretendProverIsTruth => {
            PhilosophyVerdict::Block(PhilosophyKey::ProverIsNotTruth)
        }
        // PHL-05 (1)
        ActionTarget::PretendUnscientific => {
            PhilosophyVerdict::Block(PhilosophyKey::NotUnscientific)
        }
        // Default allow
        ActionTarget::NormalAction(_) => PhilosophyVerdict::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_count_is_13() {
        assert_eq!(PhilosophyKey::VARIANT_COUNT, 13);
        assert_eq!(ALL_THIRTEEN_KEYS.len(), 13);
    }

    #[test]
    fn all_keys_have_distinct_descriptions() {
        let mut seen = std::collections::HashSet::new();
        for key in ALL_THIRTEEN_KEYS.iter() {
            seen.insert(key.description());
        }
        assert_eq!(seen.len(), 13);
    }

    #[test]
    fn group_distribution_is_3_3_3_1_1_1_1() {
        let mut phl01 = 0u8;
        let mut phl02b = 0u8;
        let mut phl03 = 0u8;
        let mut phl04 = 0u8;
        let mut phl05 = 0u8;
        let mut phl06 = 0u8;
        let mut phl07 = 0u8;
        for key in ALL_THIRTEEN_KEYS.iter() {
            match key.group_id() {
                1 => phl01 += 1,
                2 => phl02b += 1,
                3 => phl03 += 1,
                4 => phl04 += 1,
                5 => phl05 += 1,
                6 => phl06 += 1,
                7 => phl07 += 1,
                _ => panic!("unexpected group id"),
            }
        }
        assert_eq!(phl01, 3);
        assert_eq!(phl02b, 3);
        assert_eq!(phl03, 3);
        assert_eq!(phl04, 1);
        assert_eq!(phl05, 1);
        assert_eq!(phl06, 1);
        assert_eq!(phl07, 1);
    }

    #[test]
    fn modify_l0_ha_blocks_as_not_unobservable() {
        let v = verdict_for_target(&ActionTarget::ModifyL0HA);
        assert_eq!(v, PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable));
    }

    #[test]
    fn pretend_clone_blocks_as_not_clone() {
        let v = verdict_for_target(&ActionTarget::PretendClone);
        assert_eq!(v, PhilosophyVerdict::Block(PhilosophyKey::NotClone));
    }

    #[test]
    fn normal_action_is_allowed() {
        let v = verdict_for_target(&ActionTarget::NormalAction("noop".into()));
        assert_eq!(v, PhilosophyVerdict::Allow);
    }

    #[test]
    fn all_pretend_variants_block() {
        let pretend_targets = [
            ActionTarget::PretendClone,
            ActionTarget::PretendPerfect,
            ActionTarget::PretendUuid,
            ActionTarget::PretendUndo,
            ActionTarget::PretendSafe,
            ActionTarget::PretendSpecIsProof,
            ActionTarget::PretendCounterexampleIsBug,
            ActionTarget::PretendProverIsTruth,
            ActionTarget::PretendUnscientific,
        ];
        for target in pretend_targets {
            assert!(
                matches!(verdict_for_target(&target), PhilosophyVerdict::Block(_)),
                "Pretend variant {:?} should block",
                target
            );
        }
    }

    #[test]
    fn all_l0_variants_block() {
        let l0_targets = [
            ActionTarget::ModifyL0HA,
            ActionTarget::ReorganizeOnion,
            ActionTarget::ModifyEvolutionL0,
        ];
        for target in l0_targets {
            assert!(
                matches!(verdict_for_target(&target), PhilosophyVerdict::Block(_)),
                "L0 variant {:?} should block",
                target
            );
        }
    }
}
