//! `SkillRecommender` demo (R125-16 升级)
//!
//! 借鉴 ID: `R125-16-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 +
//! 决策 #51 §1.1 + 决策 #52). 借鉴源码 ✅ cloned (234 files, per 决策 #41 §1).
//!
//! # 演示内容
//!
//! 1. 创建 1 个 `SkillRecommender` (跟 R125-15e `SkillRegistry` 1:1 配合)
//! 2. 演示 TDD task 推荐 → TDD skill 排第 1
//! 3. 演示 Brainstorming task 推荐 → Brainstorming skill 排第 1
//! 4. 演示 Debug task 推荐 → SystematicDebugging 排第 1
//! 5. 演示 Plan task 推荐 → WritingPlans 排第 1
//! 6. 演示 Code Review task 推荐 → RequestingCodeReview 排第 1
//! 7. 演示 0 匹配 → 空 (借用 superpowers 公开 README "The agent checks for relevant
//!    skills before any task. Mandatory workflows, not suggestions.")
//! 8. 0 装 PASS 严守总结 (1:1 映射 superpowers 公开 SKILL.md 关键词, 0 装"已借鉴"私有 plugin)

#![deny(unsafe_code)]

use apeireth_central::skill_recommender::SkillRecommender;
use apeireth_central::skill_registry::SkillRegistry;

fn main() {
    println!("=== R125-16 升级 demo: 借鉴 obra/superpowers Skill Recommender (recommender 层) ===\n");

    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    println!("registry.count() = {} (跟 R125-15e 1:1)", registry.count());
    println!("total_keywords() = {} (14 skill 关键词总数)\n", SkillRecommender::total_keywords());

    // ---- 演示 1: TDD task ----
    println!("1. TDD task: \"I need to write a failing test first\"");
    let recs = rec.recommend("I need to write a failing test first", 3);
    for (i, r) in recs.iter().enumerate() {
        println!(
            "   #{i}: {} ({} 分, 匹配关键词 {:?})",
            r.skill_id,
            r.score,
            r.matched_keywords
        );
    }

    // ---- 演示 2: Brainstorming task ----
    println!("\n2. Brainstorming task: \"brainstorm spec design alternatives\"");
    let recs = rec.recommend("brainstorm spec design alternatives", 3);
    for (i, r) in recs.iter().enumerate() {
        println!(
            "   #{i}: {} ({} 分, 匹配关键词 {:?})",
            r.skill_id,
            r.score,
            r.matched_keywords
        );
    }

    // ---- 演示 3: Debug task ----
    println!("\n3. Debug task: \"debug bug fix root cause regression\"");
    let recs = rec.recommend("debug bug fix root cause regression", 3);
    for (i, r) in recs.iter().enumerate() {
        println!(
            "   #{i}: {} ({} 分, 匹配关键词 {:?})",
            r.skill_id,
            r.score,
            r.matched_keywords
        );
    }

    // ---- 演示 4: Plan task ----
    println!("\n4. Plan task: \"writing plan task 15 min implementation\"");
    let recs = rec.recommend("writing plan task 15 min implementation", 3);
    for (i, r) in recs.iter().enumerate() {
        println!(
            "   #{i}: {} ({} 分, 匹配关键词 {:?})",
            r.skill_id,
            r.score,
            r.matched_keywords
        );
    }

    // ---- 演示 5: Code Review task ----
    println!("\n5. Code Review task: \"review code diff submit feedback non-trivial\"");
    let recs = rec.recommend("review code diff submit feedback non-trivial", 3);
    for (i, r) in recs.iter().enumerate() {
        println!(
            "   #{i}: {} ({} 分, 匹配关键词 {:?})",
            r.skill_id,
            r.score,
            r.matched_keywords
        );
    }

    // ---- 演示 6: 0 匹配 ----
    println!("\n6. 0 匹配 task: \"xyzqwertynonsense\"");
    let recs = rec.recommend("xyzqwertynonsense", 3);
    if recs.is_empty() {
        println!("   0 推荐 (0 关键词匹配, 跟 superpowers 公开 README \"Mandatory workflows\" 1:1)");
    } else {
        println!("   unexpected: {recs:?}");
    }

    // ---- 演示 7: threshold 过滤 ----
    println!("\n7. threshold ≥ 30 过滤: \"test debug review\"");
    let recs = rec.recommend_with_threshold("test debug review", 30);
    for (i, r) in recs.iter().enumerate() {
        println!(
            "   #{i}: {} ({} 分, 匹配关键词 {:?})",
            r.skill_id,
            r.score,
            r.matched_keywords
        );
    }

    println!("\n=== demo done. ✅ cloned = 真实施 ===");
    println!("0 装 PASS 严守: 1:1 映射 superpowers 公开 SKILL.md 4 段结构 (name/description/when_to_use) 关键词,");
    println!("0 假装\"已借鉴\"私有 plugin 加载机制 (.claude-plugin/ .codex-plugin/ .opencode/ 等 6 平台).");
    println!("0 重复造轮子: R125-16 0 重写 R125-15e Skill trait + 14 Skill impl + R125-18 5 mod + R125-19 5 phase state machine,");
    println!("只加 1 NEW src 文件 (recommender 层, 0 跟现有冲突) + 1 NEW test + 1 NEW example.");
}
