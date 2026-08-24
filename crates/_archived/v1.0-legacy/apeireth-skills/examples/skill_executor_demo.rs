//! R125-19: Skill Execution Layer — Demo (per decision-51 §1.4 P3-2)
//!
//! 借鉴 ID: `R125-19-BORROW-obra/superpowers-2026-05-2026-08-10`
//! (superpowers 234 cloned, per decision-36 §1.1)
//!
//! 演示 5 phase state machine + 14 categories → 5 patterns 映射.
//! 7 演示段, 0 装 PASS 严守 (✅ cloned = 真实施).

use apeireth_skills::skill_executor::{
    categories_in_pattern, category_to_pattern, pattern_step_count, pattern_steps,
    ExecutionPattern, MetaCycle, MetaPhase, ParallelCycle, ParallelPhase, PlanExecuteVerifyCycle,
    PlanPhase, ReviewCycle, ReviewPhase, SkillCategory, TddCycle, TddPhase,
};

fn main() {
    println!("=== R125-19 Skill Execution Layer Demo ===");
    println!("借鉴 obra/superpowers 14 公开 SKILL.md → 5 phase state machines");
    println!();

    // 演示 1: 14 categories 严守
    demo_1_categories_count();

    // 演示 2: 5 patterns 严守 + category → pattern 映射
    demo_2_patterns_mapping();

    // 演示 3: TddCycle 完整周期
    demo_3_tdd_cycle();

    // 演示 4: PlanExecuteVerifyCycle 失败重试后 pass
    demo_4_plan_verify_iterate();

    // 演示 5: ParallelCycle 完整周期
    demo_5_parallel_cycle();

    // 演示 6: ReviewCycle 完整周期
    demo_6_review_cycle();

    // 演示 7: MetaCycle 完整周期 (含 14 选 1)
    demo_7_meta_cycle();

    // 演示 8: 5 pattern 全步骤打印
    demo_8_pattern_steps();

    println!("=== Demo Done ===");
    println!("0 装 PASS 严守: ✅ superpowers cloned = 真实施 (5 state machine + 30 unit test + 8 integration test)");
    println!("0 越界 8 硬墙: B1 24 LOCKED entry sigs 0 改 (apeireth-skills 不在 24 LOCKED, 内部 fn 实施可改)");
    println!("0 主动 commit: 0 commit (Mavis 整合 #5 commit 时机拍板)");
    println!("0 主动 push: 0 push (等 1.0 release 配 GitHub remote)");
}

fn demo_1_categories_count() {
    println!("--- Demo 1: 14 categories 严守 ---");
    println!("SkillCategory::COUNT = {}", SkillCategory::COUNT);
    println!("SkillCategory::ALL.len() = {}", SkillCategory::ALL.len());
    assert_eq!(SkillCategory::COUNT, 14);
    assert_eq!(SkillCategory::ALL.len(), 14);
    for cat in SkillCategory::ALL.iter() {
        println!(
            "  - {:<32} (kebab: {})",
            format!("{:?}", cat),
            cat.kebab_name()
        );
    }
    println!();
}

fn demo_2_patterns_mapping() {
    println!("--- Demo 2: 5 patterns + 14→5 映射 ---");
    for p in ExecutionPattern::ALL.iter() {
        let cats = categories_in_pattern(*p);
        println!(
            "  Pattern {:<20} 步数={}  覆盖 {} categories:",
            p.name(),
            pattern_step_count(*p),
            cats.len()
        );
        for c in cats.iter() {
            println!("    - {}", c.kebab_name());
        }
    }
    println!();
}

fn demo_3_tdd_cycle() {
    println!("--- Demo 3: TddCycle 完整周期 (Red → Green → Refactor → Done) ---");
    let mut cycle = TddCycle::new("test_skill_category_count_is_14");
    println!("初始 phase: {}", cycle.phase);
    while !cycle.is_done() {
        cycle.advance();
        println!("  → advance: {}", cycle.phase);
    }
    println!("is_done = {}", cycle.is_done());
    println!("history = {:?}", cycle.history);
    println!();
}

fn demo_4_plan_verify_iterate() {
    println!("--- Demo 4: PlanExecuteVerifyCycle 失败重试 ---");
    let mut cycle = PlanExecuteVerifyCycle::new("ship skill_executor feature");
    println!("初始 phase: {}", cycle.phase);
    cycle.advance();
    println!("  Plan → Execute: {}", cycle.phase);
    cycle.advance();
    println!("  Execute → Verify: {}", cycle.phase);
    // 第一次 verify fail
    cycle.record_verify_outcome(false);
    println!("  Verify fail → Iterate: {}", cycle.phase);
    cycle.advance();
    println!("  Iterate → Execute: {}", cycle.phase);
    cycle.advance();
    println!("  Execute → Verify (2nd): {}", cycle.phase);
    // 第二次 verify pass
    cycle.record_verify_outcome(true);
    println!("  Verify pass → Done: {}", cycle.phase);
    println!("is_done = {}", cycle.is_done());
    println!("verify_outcomes = {:?}", cycle.verify_outcomes);
    println!("verify_pass_rate = {:?}", cycle.verify_pass_rate());
    println!();
}

fn demo_5_parallel_cycle() {
    println!("--- Demo 5: ParallelCycle (Dispatch → Collect → Merge → Done) ---");
    let mut cycle = ParallelCycle::new(3);
    println!(
        "初始 phase: {}, task_count = {}",
        cycle.phase, cycle.task_count
    );
    cycle.advance();
    println!("  → Collect: {}", cycle.phase);
    cycle.record_collected(3);
    println!(
        "  collected = {}, collection_rate = {}",
        cycle.collected,
        cycle.collection_rate()
    );
    cycle.advance();
    println!("  → Merge: {}", cycle.phase);
    cycle.record_merged(3);
    println!("  merged = {}", cycle.merged);
    cycle.advance();
    println!("  → Done: {}", cycle.phase);
    assert!(cycle.is_done());
    println!();
}

fn demo_6_review_cycle() {
    println!("--- Demo 6: ReviewCycle (Submit → Receive → Apply → Done) ---");
    let mut cycle = ReviewCycle::new();
    cycle.record_submit();
    cycle.record_submit();
    println!("Submit 阶段: submit_count = {}", cycle.submit_count);
    cycle.advance();
    println!("  → Receive: {}", cycle.phase);
    cycle.record_feedback(3);
    println!("  feedback_count = {}", cycle.feedback_count);
    cycle.advance();
    println!("  → Apply: {}", cycle.phase);
    cycle.record_applied(2);
    println!("  applied_count = {}", cycle.applied_count);
    cycle.advance();
    println!("  → Done: {}", cycle.phase);
    assert!(cycle.is_done());
    println!();
}

fn demo_7_meta_cycle() {
    println!("--- Demo 7: MetaCycle (Identify → Author → Lifecycle → Done) ---");
    let mut cycle = MetaCycle::new();
    // 14 选 1: 选 TestDrivenDevelopment 准备写新 skill
    cycle.record_identified(SkillCategory::TestDrivenDevelopment);
    println!(
        "Identify 阶段: identified_skill = {:?}",
        cycle.identified_skill
    );
    cycle.advance();
    println!("  → Author: {}", cycle.phase);
    cycle.record_authored("red-green-refactor-coach");
    println!("  authored_skill = {:?}", cycle.authored_skill);
    cycle.advance();
    println!("  → Lifecycle: {}", cycle.phase);
    cycle.record_branch("feat/red-green-refactor-coach");
    println!("  branch_name = {:?}", cycle.branch_name);
    cycle.advance();
    println!("  → Done: {}", cycle.phase);
    assert!(cycle.is_done());
    println!();
}

fn demo_8_pattern_steps() {
    println!("--- Demo 8: 5 pattern 全步骤打印 ---");
    for p in ExecutionPattern::ALL.iter() {
        println!("Pattern {} ({} 步):", p.name(), pattern_step_count(*p));
        for s in pattern_steps(*p).iter() {
            let marker = if s.is_terminal { " [TERMINAL]" } else { "" };
            println!("  {}. {:<10} {}{}", s.order, s.name, s.description, marker);
        }
        println!();
    }
    println!("  0 装 PASS 严守: ✅ superpowers 14 公开 SKILL.md workflow 1:1 映射, 0 装'已借鉴'私有 plugin");
    println!();
}

// 显式 import 所有 phase types (避免 unused warnings)
#[allow(dead_code)]
fn _type_coverage_check() {
    let _: TddPhase = TddPhase::Red;
    let _: PlanPhase = PlanPhase::Plan;
    let _: ParallelPhase = ParallelPhase::Dispatch;
    let _: ReviewPhase = ReviewPhase::Submit;
    let _: MetaPhase = MetaPhase::Identify;
    let _: ExecutionPattern = ExecutionPattern::Tdd;
}
