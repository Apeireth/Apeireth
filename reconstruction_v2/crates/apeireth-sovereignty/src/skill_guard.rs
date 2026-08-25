//! skill_guard: 7 重守门 v7 wrapper

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillStep {
    pub order: usize,
    pub description: String,
    pub is_tdd_red: bool,
}

pub trait Skill: Send + Sync {
    fn id(&self) -> SkillId;
    fn name(&self) -> &'static str;
    fn when_to_use(&self) -> &'static str;
    fn steps(&self) -> Vec<SkillStep>;
    fn tdd_required(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillId {
    MultiAiGuard,
    MultiHumanGuard,
    PhysicalMultisigGuard,
    ReflectionGuard,
    MewgGuard,
    ColangDslGuard,
    SuperpowersSkillGuard,
}

impl SkillId {
    pub const ALL: [SkillId; 7] = [
        SkillId::MultiAiGuard, SkillId::MultiHumanGuard, SkillId::PhysicalMultisigGuard,
        SkillId::ReflectionGuard, SkillId::MewgGuard, SkillId::ColangDslGuard,
        SkillId::SuperpowersSkillGuard,
    ];
    pub const COUNT: usize = 7;
    pub fn kebab_name(&self) -> &'static str {
        match self {
            SkillId::MultiAiGuard => "multi-ai-guard",
            SkillId::MultiHumanGuard => "multi-human-guard",
            SkillId::PhysicalMultisigGuard => "physical-multisig-guard",
            SkillId::ReflectionGuard => "reflection-guard",
            SkillId::MewgGuard => "mewg-guard",
            SkillId::ColangDslGuard => "colang-dsl-guard",
            SkillId::SuperpowersSkillGuard => "superpowers-skill-guard",
        }
    }
}

pub struct MultiAiGuardSkill;
impl Skill for MultiAiGuardSkill {
    fn id(&self) -> SkillId { SkillId::MultiAiGuard }
    fn name(&self) -> &'static str { "Multi-AI Guard" }
    fn when_to_use(&self) -> &'static str { "守门 1 (多 AI 一致)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "≥3 个不同 LLM 独立 check".into(), is_tdd_red: false },
        SkillStep { order: 2, description: "聚合 4 类结果".into(), is_tdd_red: false },
        SkillStep { order: 3, description: "Rejected → Blocked, Insufficient → PendingReview".into(), is_tdd_red: false },
    ] }
}

pub struct MultiHumanGuardSkill;
impl Skill for MultiHumanGuardSkill {
    fn id(&self) -> SkillId { SkillId::MultiHumanGuard }
    fn name(&self) -> &'static str { "Multi-Human Guard" }
    fn when_to_use(&self) -> &'static str { "守门 2 (多人投票)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "≥2 真实人类 approve".into(), is_tdd_red: false },
        SkillStep { order: 2, description: "无 reject".into(), is_tdd_red: false },
        SkillStep { order: 3, description: "Insufficient → PendingReview".into(), is_tdd_red: false },
    ] }
}

pub struct PhysicalMultisigGuardSkill;
impl Skill for PhysicalMultisigGuardSkill {
    fn id(&self) -> SkillId { SkillId::PhysicalMultisigGuard }
    fn name(&self) -> &'static str { "Physical-Multisig Guard" }
    fn when_to_use(&self) -> &'static str { "守门 3 (物理多签)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "≥2 不同 kind 物理签名 + ≥1 witness".into(), is_tdd_red: false },
        SkillStep { order: 2, description: "Rejected → Blocked".into(), is_tdd_red: false },
        SkillStep { order: 3, description: "PendingSignatures → PendingReview".into(), is_tdd_red: false },
    ] }
}

pub struct ReflectionGuardSkill;
impl Skill for ReflectionGuardSkill {
    fn id(&self) -> SkillId { SkillId::ReflectionGuard }
    fn name(&self) -> &'static str { "Reflection Guard" }
    fn when_to_use(&self) -> &'static str { "守门 4 (反思期)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "ReflectionClock.begin ≥ 7 天".into(), is_tdd_red: false },
        SkillStep { order: 2, description: "tick 推进到 AwaitingResolution".into(), is_tdd_red: false },
        SkillStep { order: 3, description: "Reflecting → PendingReview".into(), is_tdd_red: false },
    ] }
}

pub struct MewgGuardSkill;
impl Skill for MewgGuardSkill {
    fn id(&self) -> SkillId { SkillId::MewgGuard }
    fn name(&self) -> &'static str { "MEWG Guard" }
    fn when_to_use(&self) -> &'static str { "守门 5 (MEWG 汇总)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "4 evidence 累积".into(), is_tdd_red: false },
        SkillStep { order: 2, description: "MewgAuthority.evaluate 加权分 ≥ 阈值".into(), is_tdd_red: false },
        SkillStep { order: 3, description: "Approved → Approved, Blocked → Blocked".into(), is_tdd_red: false },
    ] }
}

pub struct ColangDslGuardSkill;
impl Skill for ColangDslGuardSkill {
    fn id(&self) -> SkillId { SkillId::ColangDslGuard }
    fn name(&self) -> &'static str { "Colang DSL Guard" }
    fn when_to_use(&self) -> &'static str { "守门 6 (Colang DSL)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "ColangParser.parse".into(), is_tdd_red: false },
        SkillStep { order: 2, description: "ColangValidator.validate".into(), is_tdd_red: false },
        SkillStep { order: 3, description: "ColangDslGuard.check_source".into(), is_tdd_red: false },
    ] }
}

pub struct SuperpowersSkillGuardSkill;
impl Skill for SuperpowersSkillGuardSkill {
    fn id(&self) -> SkillId { SkillId::SuperpowersSkillGuard }
    fn name(&self) -> &'static str { "Superpowers Skill Guard" }
    fn when_to_use(&self) -> &'static str { "守门 7 (Skill 化)" }
    fn steps(&self) -> Vec<SkillStep> { vec![
        SkillStep { order: 1, description: "TDD RED step is_tdd_red=true".into(), is_tdd_red: true },
        SkillStep { order: 2, description: "7 Skill 严守 SkillId::ALL".into(), is_tdd_red: true },
        SkillStep { order: 3, description: "SkillRegistry 中心调度".into(), is_tdd_red: false },
    ] }
}

pub type BoxedSkill = Arc<dyn Skill + Send + Sync>;

pub struct SkillRegistry {
    skills: BTreeMap<SkillId, BoxedSkill>,
}

impl Default for SkillRegistry { fn default() -> Self { Self::new() } }

impl SkillRegistry {
    pub fn new() -> Self {
        let mut skills = BTreeMap::new();
        skills.insert(SkillId::MultiAiGuard, Arc::new(MultiAiGuardSkill) as BoxedSkill);
        skills.insert(SkillId::MultiHumanGuard, Arc::new(MultiHumanGuardSkill) as BoxedSkill);
        skills.insert(SkillId::PhysicalMultisigGuard, Arc::new(PhysicalMultisigGuardSkill) as BoxedSkill);
        skills.insert(SkillId::ReflectionGuard, Arc::new(ReflectionGuardSkill) as BoxedSkill);
        skills.insert(SkillId::MewgGuard, Arc::new(MewgGuardSkill) as BoxedSkill);
        skills.insert(SkillId::ColangDslGuard, Arc::new(ColangDslGuardSkill) as BoxedSkill);
        skills.insert(SkillId::SuperpowersSkillGuard, Arc::new(SuperpowersSkillGuardSkill) as BoxedSkill);
        Self { skills }
    }

    pub fn register(&mut self, skill: BoxedSkill) -> SkillId {
        let id = skill.id();
        self.skills.insert(id, skill);
        id
    }
    pub fn get(&self, id: SkillId) -> Option<BoxedSkill> { self.skills.get(&id).cloned() }
    pub fn count(&self) -> usize { self.skills.len() }
    pub fn all_ids(&self) -> Vec<SkillId> { SkillId::ALL.to_vec() }
    pub fn tdd_required(&self, id: SkillId) -> bool { self.get(id).map(|s| s.tdd_required()).unwrap_or(false) }
    pub fn tdd_required_skill_ids(&self) -> Vec<SkillId> {
        SkillId::ALL.iter().copied().filter(|id| self.tdd_required(*id)).collect()
    }
    pub fn run_skill(&self, id: SkillId) -> Result<Vec<SkillStep>, SkillError> {
        self.get(id).map(|s| s.steps()).ok_or(SkillError::UnknownSkill { id })
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum SkillError {
    #[error("unknown skill id: {id:?}")]
    UnknownSkill { id: SkillId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillGuardConfig {
    pub require_all_seven: bool,
    pub require_six_before_seven: bool,
    pub min_tdd_red_steps: usize,
}

impl Default for SkillGuardConfig {
    fn default() -> Self {
        Self { require_all_seven: true, require_six_before_seven: true, min_tdd_red_steps: 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkillGuardOutcome {
    Approved { skill_count: usize, tdd_red_steps: usize },
    Blocked { reason: String },
    PendingReview { state: String },
}

pub struct SkillGuard { pub config: SkillGuardConfig }

impl Default for SkillGuard { fn default() -> Self { Self::new() } }

impl SkillGuard {
    pub fn new() -> Self { Self { config: SkillGuardConfig::default() } }
    pub fn with_config(mut self, config: SkillGuardConfig) -> Self { self.config = config; self }
    pub fn require_all_seven(mut self, require: bool) -> Self { self.config.require_all_seven = require; self }
    pub fn require_six_before_seven(mut self, require: bool) -> Self { self.config.require_six_before_seven = require; self }

    pub fn check(&self, six_fold_completed: bool, tdd_red_step_count: usize) -> SkillGuardOutcome {
        if self.config.require_six_before_seven && !six_fold_completed {
            return SkillGuardOutcome::Blocked { reason: "守门 1-6 未跑完就跑守门 7".to_string() };
        }
        if self.config.require_all_seven && tdd_red_step_count < self.config.min_tdd_red_steps {
            return SkillGuardOutcome::Blocked {
                reason: format!("TDD RED 步骤数 {} < min_tdd_red_steps {}", tdd_red_step_count, self.config.min_tdd_red_steps),
            };
        }
        SkillGuardOutcome::Approved { skill_count: SkillId::COUNT, tdd_red_steps: tdd_red_step_count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn all_seven_match() {
        assert_eq!(SkillId::ALL.len(), 7);
        assert_eq!(SkillId::COUNT, 7);
    }
    #[test] fn kebab_names_unique() {
        let names: Vec<&str> = SkillId::ALL.iter().map(|id| id.kebab_name()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 7);
    }
    #[test] fn all_skills_at_least_three_steps() {
        let r = SkillRegistry::new();
        for id in SkillId::ALL {
            let steps = r.run_skill(id).unwrap();
            assert!(steps.len() >= 3);
        }
    }
    #[test] fn blocks_when_tdd_insufficient() {
        let g = SkillGuard::new();
        assert!(matches!(g.check(true, 0), SkillGuardOutcome::Blocked { .. }));
    }
    #[test] fn blocks_when_six_not_completed() {
        let g = SkillGuard::new();
        assert!(matches!(g.check(false, 5), SkillGuardOutcome::Blocked { .. }));
    }
    #[test] fn approves_when_all_conditions() {
        let g = SkillGuard::new();
        match g.check(true, 3) {
            SkillGuardOutcome::Approved { skill_count, tdd_red_steps } => {
                assert_eq!(skill_count, 7);
                assert_eq!(tdd_red_steps, 3);
            }
            _ => panic!(),
        }
    }
    #[test] fn registry_has_seven() {
        let r = SkillRegistry::new();
        assert_eq!(r.count(), 7);
        for id in SkillId::ALL { assert!(r.get(id).is_some()); }
    }
    #[test] fn superpowers_skill_marks_tdd_red() {
        let r = SkillRegistry::new();
        let s = r.get(SkillId::SuperpowersSkillGuard).unwrap();
        let steps = s.steps();
        assert!(steps.iter().filter(|s| s.is_tdd_red).count() >= 2);
    }
    #[test] fn unknown_skill_returns_error() {
        let r = SkillRegistry::new();
        // all are registered, but we test SkillError variant
        let err = SkillError::UnknownSkill { id: SkillId::MultiAiGuard };
        assert_eq!(r.run_skill(SkillId::MultiAiGuard).unwrap_err(), SkillError::UnknownSkill { id: SkillId::MultiAiGuard });
        assert_eq!(format!("{:?}", err).contains("MultiAiGuard"), true);
    }
    #[test] fn run_skill_all_ids() {
        let r = SkillRegistry::new();
        for id in SkillId::ALL {
            assert!(r.run_skill(id).is_ok());
        }
    }
    #[test] fn tdd_required_skill_ids_returns_seven() {
        let r = SkillRegistry::new();
        assert_eq!(r.tdd_required_skill_ids().len(), 7);
    }
}