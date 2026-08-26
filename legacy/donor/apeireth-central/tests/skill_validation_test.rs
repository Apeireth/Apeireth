//! `apeireth-central` Skill 验证借鉴集成测试 (R125-18 升级)
//!
//! 借鉴 ID: `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1
//! + R125-18 decision-log). 借鉴源码 ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1).
//!
//! # 8 集成测试 (per R125-18 spec)
//!
//! 1. `test_validate_tdd_skill` — TestDrivenDevelopmentSkill 0 error
//! 2. `test_validate_brainstorming_skill` — BrainstormingSkill 0 error
//! 3. `test_validate_meta_skill` — UsingSuperpowersSkill 0 error
//! 4. `test_validate_registry_all_valid` — 14/14 skill 全 valid
//! 5. `test_validity_ratio_is_1` — 14 valid 比例 = 1.0
//! 6. `test_min_step_count_constant` — MIN_STEP_COUNT = 3
//! 7. `test_error_display_human_readable` — 4 error variant 格式化
//! 8. `test_registry_validate_returns_report` — SkillRegistry::validate 整合

#![deny(unsafe_code)]

use apeireth_central::skill_registry::SkillRegistry;
use apeireth_central::skill_trait::{
    BrainstormingSkill, Skill, SkillId, TestDrivenDevelopmentSkill, UsingSuperpowersSkill,
};
use apeireth_central::skill_validation::{
    registry_validity_ratio, validate_registry, validate_skill, SkillValidationError,
    SkillValidationReport, MIN_STEP_COUNT,
};

#[test]
fn test_validate_tdd_skill() {
    let skill = TestDrivenDevelopmentSkill;
    let report = validate_skill(&skill);
    assert!(report.is_valid());
    assert_eq!(report.skill_id, SkillId::TestDrivenDevelopment);
    assert_eq!(report.n_errors(), 0);
}

#[test]
fn test_validate_brainstorming_skill() {
    let skill = BrainstormingSkill;
    let report = validate_skill(&skill);
    assert!(report.is_valid());
    assert_eq!(report.n_errors(), 0);
}

#[test]
fn test_validate_meta_skill() {
    let skill = UsingSuperpowersSkill;
    let report = validate_skill(&skill);
    assert!(report.is_valid());
    assert_eq!(report.n_errors(), 0);
}

#[test]
fn test_validate_registry_all_valid() {
    let registry = SkillRegistry::new();
    let reports = validate_registry(&registry);
    assert_eq!(reports.len(), 14);
    for report in &reports {
        assert!(
            report.is_valid(),
            "{} should be valid: {:?}",
            report.skill_name,
            report.errors
        );
    }
}

#[test]
fn test_validity_ratio_is_1() {
    let registry = SkillRegistry::new();
    let reports = validate_registry(&registry);
    let ratio = registry_validity_ratio(&reports);
    assert!(
        (ratio - 1.0).abs() < 1e-9,
        "ratio should be 1.0, got {ratio}"
    );
}

#[test]
fn test_min_step_count_constant() {
    assert_eq!(MIN_STEP_COUNT, 3);
}

#[test]
fn test_error_display_human_readable() {
    let cases: [(SkillValidationError, &str); 4] = [
        (SkillValidationError::NameEmpty, "skill name is empty"),
        (
            SkillValidationError::TooFewSteps { count: 2, min: 3 },
            "skill has 2 steps, minimum is 3",
        ),
        (
            SkillValidationError::MissingTddRedStep,
            "TDD skill is missing RED step (iron law violation)",
        ),
        (
            SkillValidationError::StepDescriptionEmpty { step_order: 3 },
            "step 3 has empty description",
        ),
    ];
    for (err, expected) in cases.iter() {
        assert_eq!(format!("{err}"), *expected);
    }
}

#[test]
fn test_registry_validate_returns_report() {
    let registry = SkillRegistry::new();
    let report = registry
        .validate(SkillId::TestDrivenDevelopment)
        .expect("validate");
    assert!(report.is_valid());
    assert_eq!(report.skill_id, SkillId::TestDrivenDevelopment);
}
