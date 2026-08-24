//! apeireth-life-force demo — 演示生命力维度穿透 + 反思期 + SGI 单字段
//!
//! 演示:
//! 1. 构造 LifeForce (复用 apeireth-core IdentityCard)
//! 2. 设置 SGI 单字段目标身份
//! 3. 触发反思期 (M3 WeeklyReport)
//! 4. 检查耗竭 + 启动恢复
//! 5. 校验反思期进度

use apeireth_core::IdentityCard;
use apeireth_life_force::{
    exhaustion_check, recovery_start, reflection_progress, reflection_trigger, LifeForce,
    ReflectionPeriod, ReflectionTrigger, SelfGrowthIndicator, StandardReflectionPeriod,
};

fn main() {
    let t0 = 1_700_000_000;
    let identity = IdentityCard {
        continuity_id: "did:apeireth:demo-001".to_string(),
        birth_time: t0,
        carriers: vec!["carrier-demo".to_string()],
        migration_history: vec![],
    };

    let mut life = LifeForce::new(identity, t0);
    println!(
        "[1] LifeForce created: endurance={:.2}, has_sgi={}",
        life.endurance,
        life.has_sgi()
    );

    // [2] 设置 SGI 单字段
    life.sgi = SelfGrowthIndicator::new("assist-and-reflect-with-philosophy-guard", t0);
    println!(
        "[2] SGI set: goal='{}', sgi.goal.len()={}",
        life.sgi.goal,
        life.sgi.goal.len()
    );

    // [3] 触发反思期 (M3 日常反思周报)
    let trigger = ReflectionTrigger::WeeklyReport;
    let started = t0 + 60;
    let new_state = reflection_trigger(&mut life, trigger.clone(), started).expect("trigger");
    println!(
        "[3] Reflection triggered ({:?}): active={}, endurance={:.2}",
        trigger,
        new_state.is_active(started),
        life.endurance
    );

    // [4] 反思期进度 (24h 已过)
    let mid = started + 24 * 3600;
    let p = reflection_progress(&life, mid);
    println!("[4] Reflection progress at 24h: {:.3}", p);

    // [5] 反思期结束 (72h + 1)
    let end = started + 72 * 3600 + 1;
    let p_end = reflection_progress(&life, end);
    let still_active = life.is_in_reflection(end);
    println!(
        "[5] Reflection progress at end: {:.3}, still_active={}",
        p_end, still_active
    );

    // [6] 持续力耗竭检查
    life.endurance = 0.1;
    let is_exhausted = exhaustion_check(&life);
    println!(
        "[6] Exhaustion check (endurance=0.1): exhausted={}",
        is_exhausted
    );

    // [7] 恢复启动
    let after = recovery_start(&mut life);
    println!("[7] Recovery started, endurance={:.2}", after);

    // [8] ReflectionPeriod 72h 默认
    let period = StandardReflectionPeriod;
    println!(
        "[8] Standard reflection period: {} secs ({} hours)",
        period.duration_secs(),
        period.duration_secs() / 3600
    );

    println!("\n✅ apeireth-life-force demo complete.");
}
