//! seven_fold_guard: 7 重守门 v7 衔接器

use serde::{Deserialize, Serialize};

use crate::colang_dsl::{DslOnionLayer, DslOnionVerdict};
use crate::governance::{Governance, GovernanceOutcome};
use crate::mewg::Decision;
use crate::skill_guard::{SkillGuard, SkillGuardOutcome, SkillRegistry};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SevenFoldGuardOutcome {
    Approved { governance: GovernanceOutcome, dsl: DslOnionVerdict, skill: SkillGuardOutcome },
    BlockedAtDsl { reason: String, line: Option<usize> },
    BlockedAtGovernance { governance: GovernanceOutcome, dsl: DslOnionVerdict, skill: Option<SkillGuardOutcome> },
    BlockedAtSkill { reason: String, governance: GovernanceOutcome, dsl: DslOnionVerdict },
    PendingReview { state: String, governance: Option<GovernanceOutcome>, dsl: Option<DslOnionVerdict>, skill: Option<SkillGuardOutcome> },
}

pub struct SevenFoldGuardRunner<'a> {
    pub governance: &'a Governance,
    pub dsl_layer: DslOnionLayer,
    pub skill_registry: SkillRegistry,
    pub skill_guard: SkillGuard,
}

impl<'a> SevenFoldGuardRunner<'a> {
    pub fn new(governance: &'a Governance) -> Self {
        Self { governance, dsl_layer: DslOnionLayer::new(), skill_registry: SkillRegistry::new(), skill_guard: SkillGuard::new() }
    }
    pub fn with_dsl_layer(mut self, layer: DslOnionLayer) -> Self { self.dsl_layer = layer; self }
    pub fn with_skill_registry(mut self, registry: SkillRegistry) -> Self { self.skill_registry = registry; self }
    pub fn with_skill_guard(mut self, guard: SkillGuard) -> Self { self.skill_guard = guard; self }

    pub async fn process(&self, decision: &Decision, dsl_source: &str) -> Result<SevenFoldGuardOutcome, crate::governance::GovernanceError> {
        let dsl_verdict = self.dsl_layer.evaluate(dsl_source);
        match &dsl_verdict {
            DslOnionVerdict::Block { reason, line, .. } => {
                return Ok(SevenFoldGuardOutcome::BlockedAtDsl { reason: reason.clone(), line: *line });
            }
            DslOnionVerdict::Pending { state, .. } => {
                return Ok(SevenFoldGuardOutcome::PendingReview {
                    state: state.clone(), governance: None, dsl: Some(dsl_verdict), skill: None,
                });
            }
            DslOnionVerdict::Pass { .. } => {}
        }
        let gov_outcome = self.governance.process(decision).await?;
        let gov_passed = matches!(gov_outcome, GovernanceOutcome::Approved { .. });
        match &gov_outcome {
            GovernanceOutcome::Approved { .. } => {
                let mut tdd_red_count = 0usize;
                for id in self.skill_registry.all_ids() {
                    if let Ok(steps) = self.skill_registry.run_skill(id) {
                        tdd_red_count += steps.iter().filter(|s| s.is_tdd_red).count();
                    }
                }
                let skill_outcome = self.skill_guard.check(gov_passed, tdd_red_count);
                match &skill_outcome {
                    SkillGuardOutcome::Approved { .. } => Ok(SevenFoldGuardOutcome::Approved { governance: gov_outcome, dsl: dsl_verdict, skill: skill_outcome }),
                    SkillGuardOutcome::Blocked { reason } => Ok(SevenFoldGuardOutcome::BlockedAtSkill { reason: reason.clone(), governance: gov_outcome, dsl: dsl_verdict }),
                    SkillGuardOutcome::PendingReview { state } => Ok(SevenFoldGuardOutcome::PendingReview { state: state.clone(), governance: Some(gov_outcome), dsl: Some(dsl_verdict), skill: Some(skill_outcome) }),
                }
            }
            GovernanceOutcome::Blocked { .. } => Ok(SevenFoldGuardOutcome::BlockedAtGovernance { governance: gov_outcome, dsl: dsl_verdict, skill: None }),
            GovernanceOutcome::PendingReview { .. } => Ok(SevenFoldGuardOutcome::PendingReview { state: "governance pending".to_string(), governance: Some(gov_outcome), dsl: Some(dsl_verdict), skill: None }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_guard::SkillId;
    #[test]
    fn seven_fold_runner_constructs() {
        let gov = Governance::default();
        let r = SevenFoldGuardRunner::new(&gov);
        assert_eq!(r.skill_registry.count(), 7);
    }
    #[test]
    fn skill_registry_seven_entries() {
        let gov = Governance::default();
        let r = SevenFoldGuardRunner::new(&gov);
        for id in SkillId::ALL { assert!(r.skill_registry.get(id).is_some()); }
    }
    #[test]
    fn blocks_when_six_not_completed() {
        let gov = Governance::default();
        let r = SevenFoldGuardRunner::new(&gov);
        assert!(matches!(r.skill_guard.check(false, 5), SkillGuardOutcome::Blocked { .. }));
    }
    #[test]
    fn blocks_when_tdd_insufficient() {
        let gov = Governance::default();
        let r = SevenFoldGuardRunner::new(&gov);
        assert!(matches!(r.skill_guard.check(true, 0), SkillGuardOutcome::Blocked { .. }));
    }
    #[test]
    fn approves_when_all_conditions() {
        let gov = Governance::default();
        let r = SevenFoldGuardRunner::new(&gov);
        assert!(matches!(r.skill_guard.check(true, 5), SkillGuardOutcome::Approved { .. }));
    }
}
