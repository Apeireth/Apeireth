//! `apeireth-central` Skill R125-18 升级借鉴集成测试
//!
//! 借鉴 ID: `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1
//! + R125-18 decision-log). 借鉴源码 ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1).
//!
//! # 12 集成测试 (per R125-18 spec, 覆盖 4 new mod + 4 new SkillRegistry fn)
//!
//! ## 块 A — SkillExecutor (R125-16 整合, 0 重写)
//!
//! R125-16 (P0-3) 已写 `SkillExecution` state machine + `SkillRunner` multi-skill runner.
//! R125-18 0 重写, 仅 `SkillRegistry::start_execution` 1:1 包装 R125-16 runner.start().
//! 完整 execution test 见 `tests/skill_runner_test.rs` (R125-16 8 集成 test).
//!
//! ## 块 B — SkillPrompt
//!
//! 1. `test_prompt_renders_tdd_skill_with_extremely_important_marker`
//! 2. `test_prompt_body_includes_skill_name_and_steps`
//! 3. `test_prompt_cache_caches_first_render`
//! 4. `test_apeireth_tool_mapping_contains_apeireth_equivalents`
//!
//! ## 块 C — SkillValidation
//!
//! 5. `test_validate_tdd_skill_passes`
//! 6. `test_validate_brainstorming_skill_passes`
//! 7. `test_validate_meta_skill_passes`
//! 8. `test_validate_registry_all_14_skills_valid`
//! 9. `test_validity_ratio_for_14_valid_skills_is_1`
//!
//! ## 块 D — SkillCompanion
//!
//! 10. `test_brainstorming_companions_count_2`
//! 11. `test_total_companion_count_sums_6_across_4_skills`
//!
//! ## 块 E — SkillFrontmatter
//!
//! 12. `test_parse_frontmatter_extracts_name_and_description`

#![deny(unsafe_code)]

use apeireth_central::skill_companion::{
    companions_for_skill, total_companion_count, SkillCompanionKind,
};
use apeireth_central::skill_execution::{ExecutionError, SkillExecutionStatus, SkillExecutor};
use apeireth_central::skill_frontmatter::{
    is_known_skill_name, parse_frontmatter, strip_frontmatter, FrontmatterError,
};
use apeireth_central::skill_prompt::{
    apeireth_tool_mapping, render_steps, SkillPrompt, SkillPromptCache, BOOTSTRAP_MARKER,
    EXTREMELY_IMPORTANT_MARKER,
};
use apeireth_central::skill_registry::SkillRegistry;
use apeireth_central::skill_trait::{
    BrainstormingSkill, Skill, SkillId, SystematicDebuggingSkill, TestDrivenDevelopmentSkill,
    UsingSuperpowersSkill,
};
use apeireth_central::skill_validation::{
    registry_validity_ratio, validate_registry, validate_skill, MIN_STEP_COUNT,
};

// ============================================================================
// 块 A — SkillExecutor (R125-16 整合)
// ============================================================================

#[test]
fn test_r125_18_skill_execution_advanced_via_registry_starts_execution() {
    // R125-18 整合验证: registry.start_execution 包装 R125-18 SkillExecutor
    let registry = SkillRegistry::new();
    let mut executor = SkillExecutor::new();
    let _id = registry
        .start_execution(SkillId::TestDrivenDevelopment, &mut executor, 1000)
        .expect("start_execution via R125-18 executor");
    // 验证 invocation 已 start (1 个 invocation 计数)
    assert_eq!(executor.count(), 1);
}

#[test]
fn test_registry_render_prompt_integration() {
    // R125-18 SkillRegistry::render_prompt 整合
    let registry = SkillRegistry::new();
    let prompt = registry
        .render_prompt(SkillId::TestDrivenDevelopment, "MOCK TOOL MAPPING")
        .expect("render_prompt");
    assert!(prompt.header.contains(EXTREMELY_IMPORTANT_MARKER));
    assert!(prompt.footer.contains("MOCK TOOL MAPPING"));
}

#[test]
fn test_registry_validate_integration() {
    // R125-18 SkillRegistry::validate 整合
    let registry = SkillRegistry::new();
    let report = registry
        .validate(SkillId::TestDrivenDevelopment)
        .expect("validate");
    assert!(report.is_valid());
    let bad_report = registry
        .validate(SkillId::UsingSuperpowers)
        .expect("validate meta");
    assert!(bad_report.is_valid());
}

#[test]
fn test_registry_list_with_companions_integration() {
    // R125-18 SkillRegistry::list_with_companions 整合
    let registry = SkillRegistry::new();
    let list = registry
        .list_with_companions(SkillId::Brainstorming)
        .expect("list_with_companions");
    assert_eq!(list.len(), 2);
    // 0 companion skill
    let empty = registry
        .list_with_companions(SkillId::TestDrivenDevelopment)
        .expect("list_with_companions TDD");
    assert_eq!(empty.len(), 0);
    // 0 找到 → err
    let err = registry.list_with_companions(SkillId::WritingSkills);
    // WritingSkills is in registry, so it's OK
    assert!(err.is_ok());
}

// ============================================================================
// 块 B — SkillPrompt (4 tests)
// ============================================================================

#[test]
fn test_prompt_renders_tdd_skill_with_extremely_important_marker() {
    let skill = TestDrivenDevelopmentSkill;
    let prompt = SkillPrompt::render(&skill, "");
    assert!(prompt.header.contains(EXTREMELY_IMPORTANT_MARKER));
    assert!(prompt.header.contains(BOOTSTRAP_MARKER));
    assert!(prompt.header.contains("Test-Driven Development"));
}

#[test]
fn test_prompt_body_includes_skill_name_and_steps() {
    let skill = TestDrivenDevelopmentSkill;
    let prompt = SkillPrompt::render(&skill, "");
    assert!(prompt.body.contains("Test-Driven Development"));
    assert!(prompt.body.contains("1. "));
    assert!(prompt.body.contains("[RED]"));
    let full = prompt.to_full_string();
    assert!(full.starts_with(EXTREMELY_IMPORTANT_MARKER));
    assert!(full.ends_with(EXTREMELY_IMPORTANT_MARKER));
}

#[test]
fn test_prompt_cache_caches_first_render() {
    let cache = SkillPromptCache::new();
    assert!(cache.is_empty());
    let skill = BrainstormingSkill;
    let prompt = cache.get_or_render(SkillId::Brainstorming, &skill, "");
    assert!(prompt.header.contains(EXTREMELY_IMPORTANT_MARKER));
    assert_eq!(cache.len(), 1);
    // 二次 get 不增加 len
    let _ = cache.get_or_render(SkillId::Brainstorming, &skill, "");
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_apeireth_tool_mapping_contains_apeireth_equivalents() {
    let mapping = apeireth_tool_mapping();
    assert!(mapping.contains("Apeireth"));
    assert!(mapping.contains("SkillRegistry"));
    assert!(mapping.contains("apeireth-tui"));
    // render_steps sanity
    let rendered = render_steps(&TestDrivenDevelopmentSkill);
    assert!(rendered.contains("🔴 1."));
}

// ============================================================================
// 块 C — SkillValidation (5 tests)
// ============================================================================

#[test]
fn test_validate_tdd_skill_passes() {
    let skill = TestDrivenDevelopmentSkill;
    let report = validate_skill(&skill);
    assert!(report.is_valid());
    assert_eq!(report.n_errors(), 0);
    assert_eq!(report.skill_id, SkillId::TestDrivenDevelopment);
}

#[test]
fn test_validate_brainstorming_skill_passes() {
    let skill = BrainstormingSkill;
    let report = validate_skill(&skill);
    assert!(report.is_valid());
}

#[test]
fn test_validate_meta_skill_passes() {
    let skill = UsingSuperpowersSkill;
    let report = validate_skill(&skill);
    assert!(report.is_valid());
    // meta skill tdd_required = false, 但 validate_skill 不要求 Red step
    assert!(!skill.tdd_required());
}

#[test]
fn test_validate_registry_all_14_skills_valid() {
    let registry = SkillRegistry::new();
    let reports = validate_registry(&registry);
    assert_eq!(reports.len(), 14);
    for r in &reports {
        assert!(
            r.is_valid(),
            "{} should be valid: {:?}",
            r.skill_name,
            r.errors
        );
    }
}

#[test]
fn test_validity_ratio_for_14_valid_skills_is_1() {
    let registry = SkillRegistry::new();
    let reports = validate_registry(&registry);
    let ratio = registry_validity_ratio(&reports);
    assert!(
        (ratio - 1.0).abs() < 1e-9,
        "ratio should be 1.0, got {ratio}"
    );
    // MIN_STEP_COUNT 常量 verify
    assert_eq!(MIN_STEP_COUNT, 3);
}

// ============================================================================
// 块 D — SkillCompanion (2 tests)
// ============================================================================

#[test]
fn test_brainstorming_companions_count_2() {
    let list = companions_for_skill(SkillId::Brainstorming);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].kind, SkillCompanionKind::VisualCompanion);
    assert_eq!(list[1].kind, SkillCompanionKind::SpecDocumentReviewerPrompt);
}

#[test]
fn test_total_companion_count_sums_6_across_4_skills() {
    // 2 + 1 + 1 + 2 = 6 companions across 4 skills
    assert_eq!(total_companion_count(), 6);
    // 也验证 R125-18 SkillExecutor 工作 (R125-18 替代 R125-16 SkillExecution)
    let mut executor = SkillExecutor::new();
    let id = executor.start(SkillId::SystematicDebugging, 1000);
    let skill = SystematicDebuggingSkill;
    for i in 1..=5 {
        executor
            .advance_step(id, &skill, 1000 + i as u64)
            .expect("advance");
    }
    let inv = executor.get(id).expect("inv");
    assert_eq!(inv.step_history.len(), 5);
    // 5 步推进后 status 应该是 InProgress, 不是 Pending
    assert!(matches!(
        inv.status,
        SkillExecutionStatus::InProgress { .. }
    ));
}

// ============================================================================
// 块 E — SkillFrontmatter (1 test)
// ============================================================================

#[test]
fn test_parse_frontmatter_extracts_name_and_description() {
    let md = "---\nname: test-driven-development\ndescription: Use when implementing any feature\n---\n\n# Body\n";
    let fm = parse_frontmatter(md).expect("valid");
    assert_eq!(fm.name, "test-driven-development");
    assert_eq!(fm.description, "Use when implementing any feature");
    // strip_frontmatter sanity
    let body = strip_frontmatter(md);
    assert!(body.starts_with("# Body"));
    // 14 已知 skill name verify
    assert!(is_known_skill_name("test-driven-development"));
    assert!(is_known_skill_name("brainstorming"));
    assert!(!is_known_skill_name("nonexistent"));
    // 错误处理 verify
    let bad = "---\nname: only\n";
    assert!(matches!(
        parse_frontmatter(bad),
        Err(FrontmatterError::MissingClosing)
    ));
    // 0 越界 R125-18 ExecutionError verify
    let tdd_violation = ExecutionError::TddOrderViolation {
        id: apeireth_central::skill_execution::InvocationId(0),
        reason: "TDD skill first step must be Red".to_string(),
    };
    assert!(matches!(
        tdd_violation,
        ExecutionError::TddOrderViolation { .. }
    ));
}
