//! virtual_time_simulation — 虚拟时间模拟验收: 时间敏感机制全部虚拟快进触发.
//!
//! 覆盖 (0 真等待, 毫秒级跑完):
//!   1. 做梦机制: SleepCycle 安静期 (60s) → 触发 consolidation → DreamSubsystem 成对合并
//!   2. 权限包时间: 24h 限时包到期 / 永久包 90 天续签提醒
//!   3. 节律 28 天淘汰: 30 天观察 → 最老 2 天被淘汰 (虚拟时间逐日推进)
//!   4. 反思周期: ReflectionCycleScheduler 4 阶段完整周期 → 自动重触发 (cycles=1)
//!
//! 用 `apeireth_core::clock::VirtualClock` 驱动, 不依赖真实时钟.

use apeireth_companion::emergence::RhythmEstimator;
use apeireth_companion::packs::{PackExpiry, PackRegistry, PermissionPack};
use apeireth_core::clock::{Clock, VirtualClock};
use apeireth_memory::lightmemo::{DreamSubsystem, SleepConfig, SleepCycle};
use apeireth_memory::{ReflectionCycleScheduler, ReflectionPhase};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn main() {
    let t0 = std::time::Instant::now();
    let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut check = |name: &str, ok: bool, detail: String| {
        println!("  [{}] {} — {}", if ok { "PASS" } else { "FAIL" }, name, detail);
        if ok { pass += 1 } else { fail += 1 }
    };

    println!("══════════════════════════════════════════════════════");
    println!("虚拟时间模拟验收 (起点 {} · 0 真等待)", vc.current().format("%Y-%m-%d %H:%M"));
    println!("══════════════════════════════════════════════════════\n");

    // ---------- 1. 做梦机制 ----------
    println!("【做梦机制】SleepCycle 安静期触发 → DreamSubsystem 合并");
    let sleep = SleepCycle::with_config_and_clock(
        SleepConfig {
            quiet_threshold: std::time::Duration::from_secs(60),
            max_items_before_consolidate: 1000,
        },
        Arc::new(vc.clone()),
    );
    for _ in 0..3 {
        sleep.record_item_added();
    }
    check("安静期未到不触发", !sleep.should_consolidate(), format!("quiet=0s items=3"));
    vc.advance(chrono::Duration::seconds(61)); // 快进 61s
    check("安静 61s → 做梦触发", sleep.should_consolidate(), format!("quiet=61s ≥ 60s"));

    let dream = DreamSubsystem::new();
    let items = vec![
        "线代: 特征值最后一题卡住".to_string(),
        "高数: 换元忘换 dx".to_string(),
        "明天交线代作业".to_string(),
        "council bug: advisor 低频误报".to_string(),
    ];
    let merged = std::cell::RefCell::new(Vec::new());
    let n = dream.dream_cycle(&items, &|a, b| {
        let m = format!("{a} ◆ {b}");
        merged.borrow_mut().push(m.clone());
        m
    });
    check("dream_cycle 合并 4 条 → 2 对", n == 2, format!("ops={n}"));
    for m in merged.borrow().iter() {
        println!("      ⊙ {m}");
    }
    sleep.reset_after_cycle();
    check("重置后不触发", !sleep.should_consolidate(), "reset_after_cycle 清零".to_string());
    vc.advance(chrono::Duration::seconds(61));
    check("第二夜再触发", sleep.should_consolidate(), "advance 61s 后再做梦".to_string());
    println!();

    // ---------- 2. 权限包时间 ----------
    println!("【权限包时间】24h 限时包到期 / 永久包 90 天续签提醒");
    let packs = PackRegistry::new();
    let mut timed = PermissionPack::timed("调试工程包", vec!["FileOperator".to_string()], 24, Some(5));
    timed.activated_at_ms = vc.current().timestamp_millis(); // 绑定虚拟时钟
    packs.grant(timed);
    check(
        "24h 包有效期内放行",
        packs.check_and_consume("FileOperator", vc.current().timestamp_millis()),
        "now=T0".to_string(),
    );
    vc.advance(chrono::Duration::hours(25)); // 快进 25h
    check(
        "24h 包 25h 后到期拒绝",
        !packs.check_and_consume("FileOperator", vc.current().timestamp_millis()),
        "now=T0+25h (expired)".to_string(),
    );

    let mut daily = PermissionPack::permanent("日常包", vec!["WebSearch".to_string()]);
    daily.created_at_ms = vc.current().timestamp_millis(); // 绑定虚拟时钟
    packs.grant(daily);
    check(
        "永久包刚签无续签提醒",
        packs.renewal_reminders(vc.current().timestamp_millis()).is_empty(),
        "now=T0+25h".to_string(),
    );
    vc.advance(chrono::Duration::days(90)); // 快进 90 天
    let reminders = packs.renewal_reminders(vc.current().timestamp_millis());
    check(
        "永久包 90 天后触发续签提醒",
        reminders.iter().any(|n| n == "日常包"),
        format!("reminders={reminders:?}"),
    );
    println!();

    // ---------- 3. 节律 28 天淘汰 ----------
    println!("【节律淘汰】30 天观察 → 按天淘汰最老 (28 天上限)");
    let mut rhythm = RhythmEstimator::new(28, 30);
    for _ in 0..30 {
        rhythm.observe(vc.current()); // 每天同一时刻观察
        vc.advance(chrono::Duration::days(1));
    }
    let est = rhythm.estimate(8 * 60); // 8:00 桶
    check(
        "观察 30 天 → 只保留最近 28 天",
        est.days == 28,
        format!("days={} (期望 28, 最老 2 天被淘汰)", est.days),
    );
    println!();

    // ---------- 4. 反思周期 ----------
    println!("【反思周期】4 阶段完整周期 (虚拟时间快进)");
    let mut sched = ReflectionCycleScheduler::new("did:sim-001", vc.current().timestamp());
    vc.advance(chrono::Duration::seconds(100));
    sched.advance(ReflectionPhase::Reflecting, vc.current().timestamp()).unwrap();
    vc.advance(chrono::Duration::seconds(200));
    sched.advance(ReflectionPhase::Consolidating, vc.current().timestamp()).unwrap();
    vc.advance(chrono::Duration::seconds(300));
    sched.advance(ReflectionPhase::Concluded, vc.current().timestamp()).unwrap();
    check(
        "Concluded → 自动重触发 Triggered",
        sched.current == ReflectionPhase::Triggered,
        format!("current={:?}", sched.current),
    );
    check("完整周期 cycles_completed=1", sched.cycles_completed == 1, format!("cycles={}", sched.cycles_completed));
    check(
        "phase 已持续时长正确",
        sched.current_phase_duration_secs(vc.current().timestamp()) == 0,
        "重触发后 duration 归零".to_string(),
    );
    println!();

    // ---------- 汇总 ----------
    let elapsed = t0.elapsed();
    println!("══════════════════════════════════════════════════════");
    println!(
        "模拟完成: {pass} PASS / {fail} FAIL · 真实耗时 {:.1?} (0 真等待, 全虚拟快进)",
        elapsed
    );
    println!("══════════════════════════════════════════════════════");
    if fail > 0 {
        std::process::exit(1);
    }
}
