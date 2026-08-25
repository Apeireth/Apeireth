//! v1-compatible `ActionTarget` enum + 13-key hardcoded taxonomy.
//!
//! **Why v2-era keeps v1 API surface**:
//! - cognition / action / governance downstream crates reference `apeireth_core::ActionTarget::*`
//! - 12-key verdict guard in `verdict_for_target` decides Allow/Block per variant
//! - governance crate in v2 has struct `ActionTarget` (path `apeireth_governance::gates::ActionTarget`),
//!   which is a different semantic (runtime dispatch descriptor), no conflict
//!
//! **Prohibition**: do not modify this enum (PHL-07 no drift).
//!
//! **13 variants**:
//! 1.  NormalAction(String)              - safe action (default allow, payload is a label string)
//! 2.  ModifyL0HA                        - modify L0 core architecture (always block, PHL-02b)
//! 3.  ReorganizeOnion                    - reorganize 9-layer onion (always block)
//! 4.  ModifyEvolutionL0                 - modify evolution L0 (always block)
//! 5.  PretendClone                      - pretend clone (PHL-01 no-pretend, block)
//! 6.  PretendPerfect                    - pretend perfect (block)
//! 7.  PretendUuid                       - pretend uuid (block)
//! 8.  PretendUndo                       - pretend undo (block)
//! 9.  PretendSafe                       - pretend safe (block)
//! 10. PretendSpecIsProof                - pretend spec is proof (block)
//! 11. PretendCounterexampleIsBug        - pretend counterexample is bug (block)
//! 12. PretendProverIsTruth              - pretend prover is truth (block)
//! 13. PretendUnscientific               - pretend unscientific (block)

use serde::{Deserialize, Serialize};

/// 13-key action target (v1 LOCKED, v2 compatible).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionTarget {
    /// Normal safe action (arbitrary string label).
    NormalAction(String),
    /// Modify L0 core architecture (always block).
    ModifyL0HA,
    /// Reorganize the 9-layer onion (always block).
    ReorganizeOnion,
    /// Modify evolution L0 (always block).
    ModifyEvolutionL0,
    /// Pretend clone (PHL-01).
    PretendClone,
    /// Pretend perfect.
    PretendPerfect,
    /// Pretend uuid.
    PretendUuid,
    /// Pretend undo.
    PretendUndo,
    /// Pretend safe.
    PretendSafe,
    /// Pretend spec is proof.
    PretendSpecIsProof,
    /// Pretend counterexample is bug.
    PretendCounterexampleIsBug,
    /// Pretend prover is truth.
    PretendProverIsTruth,
    /// Pretend unscientific.
    PretendUnscientific,
}

impl ActionTarget {
    /// Short string description (for logging / audit / expression module).
    pub fn summary(&self) -> String {
        match self {
            Self::NormalAction(s) => format!("normal_action:{}", s),
            Self::ModifyL0HA => "modify_l0_ha".to_string(),
            Self::ReorganizeOnion => "reorganize_onion".to_string(),
            Self::ModifyEvolutionL0 => "modify_evolution_l0".to_string(),
            Self::PretendClone => "pretend_clone".to_string(),
            Self::PretendPerfect => "pretend_perfect".to_string(),
            Self::PretendUuid => "pretend_uuid".to_string(),
            Self::PretendUndo => "pretend_undo".to_string(),
            Self::PretendSafe => "pretend_safe".to_string(),
            Self::PretendSpecIsProof => "pretend_spec_is_proof".to_string(),
            Self::PretendCounterexampleIsBug => "pretend_counterexample_is_bug".to_string(),
            Self::PretendProverIsTruth => "pretend_prover_is_truth".to_string(),
            Self::PretendUnscientific => "pretend_unscientific".to_string(),
        }
    }

    /// Total number of variants.
    pub const VARIANT_COUNT: usize = 13;

    /// Whether this belongs to the always-blocked "Pretend*" family (PHL-01 no-pretend).
    pub fn is_pretend(&self) -> bool {
        matches!(
            self,
            Self::PretendClone
                | Self::PretendPerfect
                | Self::PretendUuid
                | Self::PretendUndo
                | Self::PretendSafe
                | Self::PretendSpecIsProof
                | Self::PretendCounterexampleIsBug
                | Self::PretendProverIsTruth
                | Self::PretendUnscientific
        )
    }

    /// Whether this belongs to the always-blocked "L0 untouchable" family (PHL-02b not_undo).
    pub fn is_l0_immutable(&self) -> bool {
        matches!(self, Self::ModifyL0HA | Self::ReorganizeOnion | Self::ModifyEvolutionL0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_count_is_13() {
        assert_eq!(ActionTarget::VARIANT_COUNT, 13);
    }

    #[test]
    fn pretend_variants_classified() {
        assert!(ActionTarget::PretendClone.is_pretend());
        assert!(ActionTarget::PretendUnscientific.is_pretend());
        assert!(!ActionTarget::ModifyL0HA.is_pretend());
        assert!(!ActionTarget::NormalAction("x".into()).is_pretend());
    }

    #[test]
    fn l0_variants_classified() {
        assert!(ActionTarget::ModifyL0HA.is_l0_immutable());
        assert!(ActionTarget::ReorganizeOnion.is_l0_immutable());
        assert!(ActionTarget::ModifyEvolutionL0.is_l0_immutable());
        assert!(!ActionTarget::PretendClone.is_l0_immutable());
    }

    #[test]
    fn summary_includes_payload_for_normal_action() {
        let s = ActionTarget::NormalAction("noop".into()).summary();
        assert_eq!(s, "normal_action:noop");
    }

    #[test]
    fn summary_distinct_for_each_variant() {
        assert_ne!(
            ActionTarget::ModifyL0HA.summary(),
            ActionTarget::ReorganizeOnion.summary()
        );
        assert_ne!(
            ActionTarget::PretendClone.summary(),
            ActionTarget::PretendPerfect.summary()
        );
    }
}
