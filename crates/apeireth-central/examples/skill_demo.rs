//! Skill 借鉴 demo (R125-15e 升级)
//!
//! 借鉴 ID: `R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 +
//! 决策 #51 §1.1). 借鉴源码 ✅ cloned (234 files, per 决策 #41 §1).
//!
//! # 演示内容
//!
//! 1. 创建 1 个 `SkillRegistry` (默认 14 skill 注册)
//! 2. 按 `SkillId::TestDrivenDevelopment` 查询 skill
//! 3. 列出全部 14 skill 名字
//! 4. 演示 1 个 TDD skill 流程 (5 步, step 1 = RED)
//! 5. 演示按 name string 查询
//! 6. 演示 `summarize()` 输出

#![deny(unsafe_code)]

use apeireth_central::skill_registry::SkillRegistry;
use apeireth_central::skill_trait::SkillId;

fn main() {
    println!("=== R125-15e 升级 demo: 借鉴 obra/superpowers Skill 系统 ===\n");

    // 1. 创建 registry (默认 14 skill 注册)
    let registry = SkillRegistry::new();
    println!("1. registry.count() = {}", registry.count());

    // 2. 按 id 查询 TDD skill
    let tdd = registry.get(SkillId::TestDrivenDevelopment).expect("TDD skill");
    println!("2. tdd.name = {}", tdd.name());
    println!("   tdd.tdd_required = {}", tdd.tdd_required());

    // 3. 列全部 14 skill 名字
    println!("\n3. 全部 14 skill:");
    for summary in registry.summarize() {
        println!(
            "   - {} ({}) — {} steps, tdd_required = {}",
            summary.id,
            summary.name,
            summary.step_count,
            summary.tdd_required,
        );
    }

    // 4. 演示 TDD 5 步流程
    println!("\n4. Test-Driven Development 步骤:");
    for step in tdd.steps() {
        let marker = if step.is_tdd_red { "🔴 RED" } else { "  •" };
        println!("   {marker} {}. {}", step.order, step.description);
    }

    // 5. 按 name string 查询
    println!("\n5. 按 name 'systematic-debugging' 查 skill:");
    let debug = registry
        .lookup_by_name("systematic-debugging")
        .expect("systematic-debugging skill");
    println!("   skill = {}", debug.name());
    println!("   when_to_use = {}", debug.when_to_use());
    println!("   step count = {}", debug.steps().len());

    // 6. TDD required 13 of 14 (excludes UsingSuperpowers)
    println!("\n6. TDD required skills:");
    for id in registry.tdd_required_skill_ids() {
        println!("   - {}", id);
    }

    // 7. meta skill 例外
    println!("\n7. meta skill (UsingSuperpowers) tdd_required = {}",
        registry.tdd_required(SkillId::UsingSuperpowers));

    println!("\n=== demo done. ✅ cloned = 真实施 ===");
    println!("0 装 PASS 严守: 1:1 映射 superpowers 公开 SKILL.md, 0 装\"已借鉴\"私有 plugin 加载机制.");
}
