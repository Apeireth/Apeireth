//! R125-19: Skill Execution Layer — Integration Tests (per decision-51 §1.4 P3-2)
//!
//! 借鉴 ID: `R125-19-BORROW-obra/superpowers-2026-05-2026-08-10`
//! (superpowers 234 cloned, per decision-36 §1.1)
//!
//! 8 集成 test 严守 (per decision-51 §1.4 P0-1 spec "8 unit test 必过" 模式):
//! 1. `test_skill_category_count_is_14` — 14 categories 严守
//! 2. `test_execution_pattern_count_is_5` — 5 patterns 严守
//! 3. `test_category_to_pattern_14_to_5_total` — 14→5 覆盖
//! 4. `test_tdd_cycle_full_workflow` — TDD 完整周期
//! 5. `test_plan_execute_verify_pass_immediately` — Plan-Verify 直接 pass
//! 6. `test_plan_execute_verify_iterate_then_pass` — Plan-Verify 失败重试后 pass
//! 7. `test_parallel_cycle_collect_and_merge` — Parallel 完整周期
//! 8. `test_review_cycle_submit_receive_apply` — Review 完整周期
//! 9. `test_meta_cycle_full_workflow` — Meta 完整周期
//! 10. `test_all_5_patterns_have_phase_machines` — 5 pattern 都有可工作 state machine

use apeireth_skills::skill_executor::{
    categories_in_pattern, category_to_pattern, pattern_step_count, pattern_steps,
    ExecutionPattern, MetaCycle, ParallelCycle, PlanExecuteVerifyCycle, ReviewCycle, SkillCategory,
    TddCycle, TddPhase,
};

// 1. 14 categories 严守
#[test]
fn test_skill_category_count_is_14() {
    assert_eq!(SkillCategory::COUNT, 14);
    assert_eq!(SkillCategory::ALL.len(), 14);
    // 全 kebab name 唯一
    let names: Vec<&str> = SkillCategory::ALL.iter().map(|c| c.kebab_name()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 14, "kebab name 0 重复");
}

// 2. 5 patterns 严守
#[test]
fn test_execution_pattern_count_is_5() {
    assert_eq!(ExecutionPattern::COUNT, 5);
    assert_eq!(ExecutionPattern::ALL.len(), 5);
}

// 3. 14 → 5 覆盖 (2+3+2+2+5=14)
#[test]
fn test_category_to_pattern_14_to_5_total() {
    let mut counts = [0usize; 5];
    for cat in SkillCategory::ALL.iter() {
        let p = category_to_pattern(*cat);
        let idx = ExecutionPattern::ALL.iter().position(|x| *x == p).unwrap();
        counts[idx] += 1;
    }
    assert_eq!(
        counts,
        [2, 3, 2, 2, 5],
        "Tdd=2 + PlanExecuteVerify=3 + Parallel=2 + Review=2 + Meta=5 = 14"
    );
    // 总数
    let total: usize = counts.iter().sum();
    assert_eq!(total, 14);
}

// 4. TDD 完整周期
#[test]
fn test_tdd_cycle_full_workflow() {
    let mut cycle = TddCycle::new("test_executor_works");
    assert_eq!(cycle.phase, TddPhase::Red);
    assert!(!cycle.is_done());

    // Red → Green
    cycle.advance();
    assert_eq!(cycle.phase, TddPhase::Green);

    // Green → Refactor
    cycle.advance();
    assert_eq!(cycle.phase, TddPhase::Refactor);

    // Refactor → Done
    cycle.advance();
    assert_eq!(cycle.phase, TddPhase::Done);
    assert!(cycle.is_done());

    // 历史全 4 phase 严守
    assert_eq!(cycle.history_len(), 4);
    assert_eq!(
        cycle.history,
        vec![
            TddPhase::Red,
            TddPhase::Green,
            TddPhase::Refactor,
            TddPhase::Done
        ]
    );
}

// 5. Plan-Verify 直接 pass
#[test]
fn test_plan_execute_verify_pass_immediately() {
    let mut cycle = PlanExecuteVerifyCycle::new("ship feature X");
    assert!(!cycle.is_done());

    cycle.advance(); // Plan → Execute
    cycle.advance(); // Execute → Verify
    assert!(cycle.record_verify_outcome(true));

    assert!(cycle.is_done());
    assert_eq!(cycle.verify_outcomes, vec![true]);
    assert_eq!(cycle.verify_pass_rate(), Some(1.0));
}

// 6. Plan-Verify 失败重试后 pass
#[test]
fn test_plan_execute_verify_iterate_then_pass() {
    let mut cycle = PlanExecuteVerifyCycle::new("ship feature Y");
    cycle.advance(); // → Execute
    cycle.advance(); // → Verify

    // 第一次 fail
    cycle.record_verify_outcome(false);
    assert!(!cycle.is_done());

    // Iterate → Execute
    cycle.advance();
    cycle.advance(); // → Verify
                     // 第二次 pass
    cycle.record_verify_outcome(true);
    assert!(cycle.is_done());
    assert_eq!(cycle.verify_outcomes, vec![false, true]);
    assert_eq!(cycle.verify_pass_rate(), Some(0.5));
}

// 7. Parallel 完整周期
#[test]
fn test_parallel_cycle_collect_and_merge() {
    let mut cycle = ParallelCycle::new(3);
    assert_eq!(cycle.task_count, 3);

    cycle.advance(); // → Collect
    cycle.record_collected(3);
    assert_eq!(cycle.collected, 3);
    assert_eq!(cycle.collection_rate(), 1.0);

    cycle.advance(); // → Merge
    cycle.record_merged(3);
    assert_eq!(cycle.merged, 3);

    cycle.advance(); // → Done
    assert!(cycle.is_done());
}

// 8. Review 完整周期
#[test]
fn test_review_cycle_submit_receive_apply() {
    let mut cycle = ReviewCycle::new();
    cycle.record_submit();
    cycle.record_submit();
    assert_eq!(cycle.submit_count, 2);

    cycle.advance(); // Submit → Receive
    cycle.record_feedback(3);
    assert_eq!(cycle.feedback_count, 3);

    cycle.advance(); // Receive → Apply
    cycle.record_applied(2);
    assert_eq!(cycle.applied_count, 2);

    cycle.advance(); // Apply → Done
    assert!(cycle.is_done());
}

// 9. Meta 完整周期
#[test]
fn test_meta_cycle_full_workflow() {
    let mut cycle = MetaCycle::new();
    cycle.record_identified(SkillCategory::WritingSkills);
    assert_eq!(cycle.identified_skill, Some(SkillCategory::WritingSkills));

    cycle.advance(); // Identify → Author
    cycle.record_authored("my-new-skill");
    assert_eq!(cycle.authored_skill.as_deref(), Some("my-new-skill"));

    cycle.advance(); // Author → Lifecycle
    cycle.record_branch("feat/my-new-skill");
    assert_eq!(cycle.branch_name.as_deref(), Some("feat/my-new-skill"));

    cycle.advance(); // Lifecycle → Done
    assert!(cycle.is_done());
}

// 10. 5 pattern 都有可工作 state machine + 步骤严守
#[test]
fn test_all_5_patterns_have_phase_machines() {
    // TddCycle
    let mut tdd = TddCycle::new("t");
    tdd.advance();
    tdd.advance();
    tdd.advance();
    assert!(tdd.is_done());

    // PlanExecuteVerifyCycle
    let mut plan = PlanExecuteVerifyCycle::new("g");
    plan.advance();
    plan.advance();
    plan.record_verify_outcome(true);
    assert!(plan.is_done());

    // ParallelCycle
    let mut parallel = ParallelCycle::new(2);
    parallel.advance();
    parallel.advance();
    parallel.advance();
    assert!(parallel.is_done());

    // ReviewCycle
    let mut review = ReviewCycle::new();
    review.advance();
    review.advance();
    review.advance();
    assert!(review.is_done());

    // MetaCycle
    let mut meta = MetaCycle::new();
    meta.advance();
    meta.advance();
    meta.advance();
    assert!(meta.is_done());

    // 5 pattern 的 step 数严守
    for p in ExecutionPattern::ALL.iter() {
        let expected = pattern_step_count(*p);
        let actual = pattern_steps(*p).len();
        assert_eq!(actual, expected, "pattern {p} step count mismatch");
        // 最后 1 步是 terminal (bind to let 延长临时值生命周期)
        let steps = pattern_steps(*p);
        let last = steps.last().unwrap();
        assert!(last.is_terminal, "pattern {p} 末 step 0 terminal");
    }
}

// 11. (bonus) 14 categories 都能反向 roundtrip from_kebab
#[test]
fn test_14_categories_roundtrip_from_kebab() {
    for cat in SkillCategory::ALL.iter() {
        let name = cat.kebab_name();
        let recovered = SkillCategory::from_kebab(name);
        assert_eq!(recovered, Some(*cat), "roundtrip fail for {name}");
    }
    // unknown → None
    assert_eq!(SkillCategory::from_kebab("not-a-skill"), None);
}

// 12. (bonus) categories_in_pattern 严守 (14 categories 分到 5 patterns)
#[test]
fn test_categories_in_pattern_5_to_14_total() {
    let mut total = 0;
    for p in ExecutionPattern::ALL.iter() {
        let cats = categories_in_pattern(*p);
        // 每 pattern 下所有 cat 都映射回本 pattern (roundtrip)
        for c in cats.iter() {
            assert_eq!(category_to_pattern(*c), *p);
        }
        total += cats.len();
    }
    assert_eq!(total, 14);
}
