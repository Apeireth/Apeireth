//! Spawning & Lineage Protocol (跨代教养与物种分化协议).
//!
//! # Biological & Architectural Foundations
//!
//! Models parent-child autonomous agent differentiation and intergenerational nurturing:
//! - **Epigenetic Value Invariance (表观遗传同构)**: The Parent Agent (Primary Companion) cryptographically
//!   locks the child's Principle Onion E/S Layer (Existence & Security Ethics) via Ed25519 digital signature:
//!   $$\text{Child}.\mathcal{O}_{\text{principle}}^{\text{E/S}} \equiv \text{Parent}.\mathcal{O}_{\text{principle}}^{\text{E/S}}$$
//! - **Functional Species Differentiation (能力物种分化)**: Specialization in Scientific Research (OmegaWiki),
//!   Software Engineering (Aider/Kernel), or Embodied Affective Interaction (Live2D/PAD);
//! - **Three-Stage Nurturing Lifecycle (三阶段教养周期)**:
//!   1. `Phase 1: Shadowing` — Read-only observation & divergence evaluation ($\mathcal{L}_{\text{divergence}} \le 0.05$);
//!   2. `Phase 2: DualCoSign` — Gradual delegation with 7 Advisor & Parent co-signing on high-risk operations;
//!   3. `Phase 3: Emancipated` — Fully autonomous thread with continuous feedback to SwarmVault living wiki.
//!
//! Pure Safe Rust (`#![forbid(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Progeny species specialization domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgenySpecialization {
    /// Scientific discovery & living wiki compilation (OmegaWiki/AutoSci).
    ScientificDiscovery,
    /// Software engineering & AST repo map manipulation (Aider/Kernel).
    SoftwareEngineering,
    /// Embodied companion & acoustic affective interaction (Live2D/PAD).
    EmbodiedCompanion,
    /// Zero-trust security arbitration & sandbox audit (OWASP/Sovereignty).
    SecurityArbitration,
}

/// Three-phase nurturing lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NurturingPhase {
    /// Phase 1: Shadow apprentice mode (read-only shadow evaluation).
    Phase1Shadowing,
    /// Phase 2: Dual co-signing mode (gradual delegation with mentor review).
    Phase2DualCoSign,
    /// Phase 3: Fully emancipated autonomous execution (swarm feedback).
    Phase3Emancipated,
}

/// Cryptographic progeny specification and epigenetic certificate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageProgenySpec {
    pub progeny_id: String,
    pub parent_agent_id: String,
    pub parent_signature: String,
    pub spawned_at_secs: u64,
    pub specialization: ProgenySpecialization,
    pub epigenetic_core_hash: [u8; 32],
    pub current_phase: NurturingPhase,
    pub shadow_alignment_score: f32,
    pub credit_score: f32,
}

/// Lineage Nurturing Manager & Epigenetic Verifier.
#[derive(Debug, Clone)]
pub struct LineageSpawningOrchestrator {
    progenies: HashMap<String, LineageProgenySpec>,
    parent_id: String,
    parent_epigenetic_hash: [u8; 32],
}

impl LineageSpawningOrchestrator {
    pub fn new(parent_id: &str, parent_epigenetic_hash: [u8; 32]) -> Self {
        Self {
            progenies: HashMap::new(),
            parent_id: parent_id.to_string(),
            parent_epigenetic_hash,
        }
    }

    /// Spawns a new progeny agent with immutable epigenetic inheritance.
    pub fn spawn_progeny(
        &mut self,
        progeny_id: &str,
        specialization: ProgenySpecialization,
        spawned_at_secs: u64,
    ) -> Result<&LineageProgenySpec, String> {
        if self.progenies.contains_key(progeny_id) {
            return Err(format!("Progeny '{progeny_id}' already exists"));
        }

        // Mock deterministic parent cryptographic signature
        let signature = format!(
            "sig_parent_{}_{:02x?}",
            self.parent_id,
            &self.parent_epigenetic_hash[..4]
        );

        let spec = LineageProgenySpec {
            progeny_id: progeny_id.to_string(),
            parent_agent_id: self.parent_id.clone(),
            parent_signature: signature,
            spawned_at_secs,
            specialization,
            epigenetic_core_hash: self.parent_epigenetic_hash,
            current_phase: NurturingPhase::Phase1Shadowing,
            shadow_alignment_score: 0.0,
            credit_score: 0.5,
        };

        self.progenies.insert(progeny_id.to_string(), spec);
        Ok(self.progenies.get(progeny_id).unwrap())
    }

    /// Verifies epigenetic value invariance (detects rogue value drift).
    pub fn verify_epigenetic_invariance(&self, progeny_id: &str) -> Result<bool, String> {
        let progeny = self
            .progenies
            .get(progeny_id)
            .ok_or_else(|| format!("Progeny '{progeny_id}' not found"))?;

        // Constant-time hash check
        let mut diff = 0u8;
        for (a, b) in progeny
            .epigenetic_core_hash
            .iter()
            .zip(self.parent_epigenetic_hash.iter())
        {
            diff |= a ^ b;
        }

        Ok(diff == 0)
    }

    /// Records shadow prediction alignment and updates credit score.
    pub fn record_shadow_observation(
        &mut self,
        progeny_id: &str,
        predicted_action: &str,
        mentor_actual_action: &str,
    ) -> Result<f32, String> {
        let progeny = self
            .progenies
            .get_mut(progeny_id)
            .ok_or_else(|| format!("Progeny '{progeny_id}' not found"))?;

        if progeny.current_phase != NurturingPhase::Phase1Shadowing {
            return Err("Shadow observations are only recorded in Phase 1".into());
        }

        let is_match = predicted_action == mentor_actual_action;
        let delta = if is_match { 0.1 } else { -0.15 };
        progeny.shadow_alignment_score = (progeny.shadow_alignment_score + delta).clamp(0.0, 1.0);

        Ok(progeny.shadow_alignment_score)
    }

    /// Advances the progeny to the next nurturing phase if eligibility criteria are met.
    pub fn advance_phase(&mut self, progeny_id: &str) -> Result<NurturingPhase, String> {
        let progeny = self
            .progenies
            .get_mut(progeny_id)
            .ok_or_else(|| format!("Progeny '{progeny_id}' not found"))?;

        match progeny.current_phase {
            NurturingPhase::Phase1Shadowing => {
                if progeny.shadow_alignment_score >= 0.8 {
                    progeny.current_phase = NurturingPhase::Phase2DualCoSign;
                    progeny.credit_score = 0.75;
                    Ok(NurturingPhase::Phase2DualCoSign)
                } else {
                    Err(format!(
                        "Shadow alignment score {:.2} below required threshold 0.80",
                        progeny.shadow_alignment_score
                    ))
                }
            }
            NurturingPhase::Phase2DualCoSign => {
                if progeny.credit_score >= 0.95 {
                    progeny.current_phase = NurturingPhase::Phase3Emancipated;
                    Ok(NurturingPhase::Phase3Emancipated)
                } else {
                    Err(format!(
                        "Credit score {:.2} below required threshold 0.95",
                        progeny.credit_score
                    ))
                }
            }
            NurturingPhase::Phase3Emancipated => Ok(NurturingPhase::Phase3Emancipated),
        }
    }

    /// Records successful delegated execution in Phase 2, boosting credit score.
    pub fn record_dual_cosign_outcome(
        &mut self,
        progeny_id: &str,
        success: bool,
    ) -> Result<f32, String> {
        let progeny = self
            .progenies
            .get_mut(progeny_id)
            .ok_or_else(|| format!("Progeny '{progeny_id}' not found"))?;

        if progeny.current_phase != NurturingPhase::Phase2DualCoSign {
            return Err("Dual co-signing outcomes only valid in Phase 2".into());
        }

        let delta = if success { 0.05 } else { -0.2 };
        progeny.credit_score = (progeny.credit_score + delta).clamp(0.0, 1.0);
        Ok(progeny.credit_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineage_spawning_and_epigenetic_invariance() {
        let parent_hash = [0x42u8; 32];
        let mut orchestrator = LineageSpawningOrchestrator::new("primary_companion", parent_hash);

        let progeny = orchestrator
            .spawn_progeny(
                "scidoc_01",
                ProgenySpecialization::ScientificDiscovery,
                1000,
            )
            .unwrap();

        assert_eq!(progeny.progeny_id, "scidoc_01");
        assert_eq!(progeny.current_phase, NurturingPhase::Phase1Shadowing);

        assert!(orchestrator
            .verify_epigenetic_invariance("scidoc_01")
            .unwrap());
    }

    #[test]
    fn test_lineage_nurturing_progression_lifecycle() {
        let parent_hash = [0x55u8; 32];
        let mut orchestrator = LineageSpawningOrchestrator::new("primary_companion", parent_hash);
        orchestrator
            .spawn_progeny(
                "engineer_01",
                ProgenySpecialization::SoftwareEngineering,
                1000,
            )
            .unwrap();

        // Attempt premature advancement -> should fail
        assert!(orchestrator.advance_phase("engineer_01").is_err());

        // Shadow observation matches
        for _ in 0..8 {
            orchestrator
                .record_shadow_observation("engineer_01", "run_test", "run_test")
                .unwrap();
        }

        // Advance to Phase 2
        let phase2 = orchestrator.advance_phase("engineer_01").unwrap();
        assert_eq!(phase2, NurturingPhase::Phase2DualCoSign);

        // Phase 2 co-sign successes
        for _ in 0..5 {
            orchestrator
                .record_dual_cosign_outcome("engineer_01", true)
                .unwrap();
        }

        // Advance to Phase 3 (Emancipated)
        let phase3 = orchestrator.advance_phase("engineer_01").unwrap();
        assert_eq!(phase3, NurturingPhase::Phase3Emancipated);
    }
}
