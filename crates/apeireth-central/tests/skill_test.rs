//! `apeireth-central` Skill 借鉴集成测试 (R125-15e 升级)
//!
//! 借鉴 ID: `R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 +
//! 决策 #51 §1.1). 借鉴源码 ✅ cloned (234 files, per 决策 #41 §1).
//!
//! # 8 集成测试 (per R125-15e spec)
//!
//! 1. `test_skill_registry_has_14_skills` — registry 14 entries
//! 2. `test_all_14_skills_1to1_match_superpowers` — 14 SkillId ALL 1:1 匹配
//! 3. `test_tdd_skill_marks_red_step` — TDD step 1 is RED
//! 4. `test_meta_skill_excluded_from_tdd_required` — UsingSuperpowers 0 要求 TDD
//! 5. `test_lookup_by_kebab_name_works` — 按 name string 查
//! 6. `test_lookup_unknown_name_returns_error` — unknown name → error
//! 7. `test_summarize_covers_all_skills` — summarize 14 entries
//! 8. `test_steps_match_skill_spec` — 1 个 skill 的 steps 数 >= 3

#![deny(unsafe_code)]

use apeireth_central::skill_registry::{SkillRegistry, SkillSummary};
use apeireth_central::skill_trait::{BrainstormingSkill, Skill, SkillId, TestDrivenDevelopmentSkill};

#[test]
fn test_skill_registry_has_14_skills() {
    // R125-15e 升级: registry 默认注册 14 skill (1:1 映射 superpowers 14 SKILL.md)
    let registry = SkillRegistry::new();
    assert_eq!(registry.count(), 14);
    assert_eq!(registry.all().len(), 14);
    assert_eq!(registry.all_ids().len(), 14);
    assert_eq!(SkillId::COUNT, 14);
    assert_eq!(SkillId::ALL.len(), 14);
}

#[test]
fn test_all_14_skills_1to1_match_superpowers() {
    // 14 SkillId 1:1 匹配 superpowers 公开 `skills/<name>/SKILL.md` (per 决策 #36 §1.1)
    let registry = SkillRegistry::new();
    let expected_kebab_names = [
        "brainstorming",
        "test-driven-development",
        "systematic-debugging",
        "verification-before-completion",
        "writing-plans",
        "executing-plans",
        "subagent-driven-development",
        "dispatching-parallel-agents",
        "requesting-code-review",
        "receiving-code-review",
        "using-git-worktrees",
        "finishing-a-development-branch",
        "writing-skills",
        "using-superpowers",
    ];
    assert_eq!(expected_kebab_names.len(), 14);

    for name in &expected_kebab_names {
        let skill = registry
            .lookup_by_name(name)
            .unwrap_or_else(|_| panic!("missing skill {name}"));
        assert_eq!(skill.id().kebab_name(), *name);
    }
}

#[test]
fn test_tdd_skill_marks_red_step() {
    // 借鉴 superpowers TDD iron law: step 1 = RED (写失败 test)
    let tdd = TestDrivenDevelopmentSkill;
    let steps = tdd.steps();
    assert!(!steps.is_empty(), "TDD skill should have at least one step");
    assert!(
        steps[0].is_tdd_red,
        "TDD step 1 must be RED (write failing test first)"
    );
    assert!(tdd.tdd_required(), "TDD skill must require TDD");
}

#[test]
fn test_meta_skill_excluded_from_tdd_required() {
    // 13 of 14 require TDD; only UsingSuperpowers is the meta exception
    let registry = SkillRegistry::new();
    let required = registry.tdd_required_skill_ids();
    assert_eq!(required.len(), 13, "13 of 14 skills require TDD");
    assert!(!required.contains(&SkillId::UsingSuperpowers));
    assert!(!registry.tdd_required(SkillId::UsingSuperpowers));
}

#[test]
fn test_lookup_by_kebab_name_works() {
    let registry = SkillRegistry::new();
    let skill = registry.lookup_by_name("brainstorming").unwrap();
    assert_eq!(skill.id(), SkillId::Brainstorming);
    assert_eq!(skill.name(), "Brainstorming");
}

#[test]
fn test_lookup_unknown_name_returns_error() {
    let registry = SkillRegistry::new();
    match registry.lookup_by_name("nonexistent-skill") {
        Err(apeireth_central::skill_registry::SkillLookupError::UnknownSkill { name }) => {
            assert_eq!(name, "nonexistent-skill");
        }
        Ok(_) => panic!("expected UnknownSkill error"),
    }
}

#[test]
fn test_summarize_covers_all_skills() {
    let registry = SkillRegistry::new();
    let summaries: Vec<SkillSummary> = registry.summarize();
    assert_eq!(summaries.len(), 14);
    for summary in &summaries {
        assert!(summary.step_count >= 3, "skill {} too few steps", summary.name);
    }
    // summary 按 SkillId::Ord 排序
    for i in 1..summaries.len() {
        assert!(summaries[i].id > summaries[i - 1].id);
    }
}

#[test]
fn test_steps_match_skill_spec() {
    // 1 个 skill 的 steps 数 >= 3 (借鉴 superpowers checklist 模式)
    let registry = SkillRegistry::new();
    let brainstorming = BrainstormingSkill;
    assert!(
        brainstorming.steps().len() >= 3,
        "brainstorming skill should have >= 3 steps"
    );
    assert!(brainstorming.tdd_required(), "brainstorming should require TDD");

    // registry 也应该能查到
    let step_count = registry.step_count(SkillId::Brainstorming);
    assert_eq!(step_count, brainstorming.steps().len());

    // TDD RED step count = 1 (only step 1 is RED)
    let red_count = registry.tdd_red_step_count(SkillId::TestDrivenDevelopment);
    assert_eq!(red_count, 1, "TDD skill should have exactly 1 RED step");
}
