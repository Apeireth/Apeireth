//! virtual_time_simulation — 虚拟时间模拟验收: 时间敏感机制全部虚拟快进触发.
//!
//! 覆盖 (0 真等待, 毫秒级跑完):
//!   1. 做梦机制: SleepCycle 安静期 (60s) → 触发 consolidation → DreamSubsystem 成对合并
//!   2. 权限包时间: 24h 限时包到期 / 永久包 90 天续签提醒
//!   3. 节律 28 天淘汰: 30 天观察 → 最老 2 天被淘汰 (虚拟时间逐日推进)
//!   4. 反思周期: ReflectionCycleScheduler 4 阶段完整周期 → 自动重触发 (cycles=1)
//!   5. 能力生命周期: propose → approve → activate → retire (严格状态机)
//!   6. 套件装配: base/沙盒/教育/渗透 三件套目录装配校验
//!   7. 提示词装配引擎 (N9): 分型变量源 + AgentGuard 特权守卫 + 虚拟时钟时间变量
//!
//! 用 `apeireth_core::clock::VirtualClock` 驱动, 不依赖真实时钟.

use apeireth_companion::capability::{CapabilityKind, CapabilityRegistry, CapabilityStatus};
use apeireth_companion::emergence::RhythmEstimator;
use apeireth_companion::packs::{PackExpiry, PackRegistry, PermissionPack};
use apeireth_companion::prompt_assembler::{
    AssemblyGuard, AssemblyRole, PromptAssembler, SourceKind, StaticSource, TimeSource,
};
use apeireth_companion::suites::SuiteCatalog;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_companion::DreamScheduler;
use apeireth_core::clock::{Clock, VirtualClock};
use apeireth_memory::lightmemo::{DreamSubsystem, SleepConfig, SleepCycle};
use apeireth_memory::{
    CoreEpisode, EpisodeStore, ReflectionCycleScheduler, ReflectionPhase, SqliteMemoryStore,
};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let t0 = std::time::Instant::now();
    let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut check = |name: &str, ok: bool, detail: String| {
        println!(
            "  [{}] {} — {}",
            if ok { "PASS" } else { "FAIL" },
            name,
            detail
        );
        if ok {
            pass += 1
        } else {
            fail += 1
        }
    };

    println!("══════════════════════════════════════════════════════");
    println!(
        "虚拟时间模拟验收 (起点 {} · 0 真等待)",
        vc.current().format("%Y-%m-%d %H:%M")
    );
    println!("══════════════════════════════════════════════════════\n");

    // ---------- 1. 做梦机制 ----------
    println!("【做梦机制】SleepCycle 安静期触发 → DreamSubsystem 合并 → 写回真库");
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
    check(
        "安静期未到不触发",
        !sleep.should_consolidate(),
        format!("quiet=0s items=3"),
    );
    vc.advance(chrono::Duration::seconds(61)); // 快进 61s
    check(
        "安静 61s → 做梦触发",
        sleep.should_consolidate(),
        format!("quiet=61s ≥ 60s"),
    );

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
    check(
        "重置后不触发",
        !sleep.should_consolidate(),
        "reset_after_cycle 清零".to_string(),
    );
    vc.advance(chrono::Duration::seconds(61));
    check(
        "第二夜再触发",
        sleep.should_consolidate(),
        "advance 61s 后再做梦".to_string(),
    );

    // 端到端: DreamScheduler 合并写回真 SQLite (虚拟时钟驱动)
    println!("\n  -- DreamScheduler 端到端: 合并写回真库 --");
    let dream_store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
    for (i, c) in items.iter().enumerate() {
        dream_store
            .put_episode(&apeireth_memory::CoreEpisode {
                id: format!("mem-{i}"),
                timestamp: i as i64,
                role: "assistant".into(),
                content: c.clone(),
                session_id: "me".into(),
            })
            .unwrap();
    }
    let sched = DreamScheduler::new(Arc::clone(&dream_store), Arc::new(vc.clone()));
    vc.advance(chrono::Duration::seconds(61));
    let merged_n = sched.tick().await;
    let eps = dream_store.recent_episodes("me", 100).unwrap();
    let dream_eps: Vec<_> = eps
        .iter()
        .filter(|e| e.id.starts_with("mem-dream-"))
        .collect();
    check(
        "DreamScheduler 合并写回真库",
        merged_n == 2 && dream_eps.len() == 2 && dream_eps[0].content.contains("◆"),
        format!(
            "合并 {merged_n} 条 → 写回 {} 条 (content 含 ◆)",
            dream_eps.len()
        ),
    );
    println!();

    // ---------- 2. 权限包时间 ----------
    println!("【权限包时间】24h 限时包到期 / 永久包 90 天续签提醒");
    let packs = PackRegistry::new();
    let mut timed =
        PermissionPack::timed("调试工程包", vec!["FileOperator".to_string()], 24, Some(5));
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
        packs
            .renewal_reminders(vc.current().timestamp_millis())
            .is_empty(),
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
    sched
        .advance(ReflectionPhase::Reflecting, vc.current().timestamp())
        .unwrap();
    vc.advance(chrono::Duration::seconds(200));
    sched
        .advance(ReflectionPhase::Consolidating, vc.current().timestamp())
        .unwrap();
    vc.advance(chrono::Duration::seconds(300));
    sched
        .advance(ReflectionPhase::Concluded, vc.current().timestamp())
        .unwrap();
    check(
        "Concluded → 自动重触发 Triggered",
        sched.current == ReflectionPhase::Triggered,
        format!("current={:?}", sched.current),
    );
    check(
        "完整周期 cycles_completed=1",
        sched.cycles_completed == 1,
        format!("cycles={}", sched.cycles_completed),
    );
    check(
        "phase 已持续时长正确",
        sched.current_phase_duration_secs(vc.current().timestamp()) == 0,
        "重触发后 duration 归零".to_string(),
    );
    println!();

    // ---------- 5. 能力生命周期 ----------
    println!("【能力生命周期】propose → approve → activate → retire (严格状态机)");
    let cap_store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let reg = CapabilityRegistry::new(Arc::clone(&cap_store), "me");
    let p = reg
        .propose(
            "换元检查",
            "做换元法时自动提醒检查 dx",
            CapabilityKind::Skill,
            "apeireth",
        )
        .unwrap();
    check(
        "提案 → pending",
        p.status == CapabilityStatus::Pending,
        format!("status={:?}", p.status),
    );
    reg.approve(&p.id).unwrap();
    reg.activate(&p.id).unwrap();
    let active = reg.active_capabilities().unwrap();
    check(
        "批准激活 → active",
        active.len() == 1 && active[0].name == "换元检查",
        format!("active={}", active.len()),
    );
    reg.retire(&p.id).unwrap();
    check(
        "退役 → active 空",
        reg.active_capabilities().unwrap().is_empty(),
        "retire 后清空".to_string(),
    );
    // 非法迁移拒绝
    let p2 = reg
        .propose("直接激活", "跳过审批", CapabilityKind::Action, "apeireth")
        .unwrap();
    check(
        "跳过审批的激活被拒",
        reg.activate(&p2.id).is_err(),
        "pending→active 非法".to_string(),
    );
    println!();

    // ---------- 6. 套件装配 ----------
    println!("【三件套装配】本体 + 能力包 + 升级套件目录校验");
    let bridge_store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let bridge = ToolBridge::new(bridge_store);
    let cat = SuiteCatalog::builtin();
    let base_ok = cat.install(&bridge, "base").is_ok();
    check(
        "本体 base 装配",
        base_ok,
        "base 工具已注册 + 权限包".to_string(),
    );
    let sandbox_ok = cat.install(&bridge, "sandbox-pack").is_ok();
    check("能力包 sandbox-pack 装配", sandbox_ok, "沙盒包".to_string());
    let tutor_ok = cat.install(&bridge, "education-suite").is_ok();
    check(
        "升级套件 education-suite 装配",
        tutor_ok,
        "教育套件".to_string(),
    );
    check(
        "未知套件拒绝",
        cat.install(&bridge, "nope").is_err(),
        "no-such-suite".to_string(),
    );
    check(
        "三类齐全",
        cat.list(apeireth_companion::suites::SuiteKind::Base).len() >= 1
            && cat
                .list(apeireth_companion::suites::SuiteKind::CapabilityPack)
                .len()
                >= 2
            && cat
                .list(apeireth_companion::suites::SuiteKind::UpgradeSuite)
                .len()
                >= 3,
        "base≥1 / pack≥2 / suite≥3".to_string(),
    );
    println!();

    // ---------- 7. 提示词装配引擎 (N9) ----------
    println!("【提示词装配】占位符变量宇宙 (分型变量源 + AgentGuard 特权守卫 + 虚拟时钟时间变量)");
    let pa_clock: Arc<dyn Clock> = Arc::new(vc.clone());
    let id_src = StaticSource::new(SourceKind::Identity)
        .set("name", "小夜")
        .unwrap();
    let assembler = PromptAssembler::new()
        .with_source(Box::new(id_src))
        .with_source(Box::new(TimeSource::new(pa_clock)))
        .with_agent("小夜", "我是{{name}}, 今天是{{date}}")
        .unwrap()
        .with_agent("小白", "另一个灵魂")
        .unwrap();
    let mut guard_sys = AssemblyGuard::new();
    let (sys_txt, sys_report) =
        assembler.expand_text("{{agent:小夜}}", AssemblyRole::System, &mut guard_sys);
    check(
        "system 特权展开 (agent 内嵌套变量递归)",
        sys_txt.starts_with("我是小夜, 今天是 ") && sys_report.expanded.len() == 1,
        sys_txt.clone(),
    );
    let date0 = sys_txt.trim_start_matches("我是小夜, 今天是 ").to_string();
    let mut guard_user = AssemblyGuard::new();
    let (user_txt, user_report) =
        assembler.expand_text("{{agent:小夜}}", AssemblyRole::User, &mut guard_user);
    check(
        "user 非特权静默移除 (防注入)",
        user_txt.is_empty()
            && user_report.removed.len() == 1
            && guard_user.expanded_agent().is_none(),
        "agent 内容不外泄".to_string(),
    );
    let mut guard_second = AssemblyGuard::new();
    let (_second_txt, second_report) = assembler.expand_text(
        "{{agent:小夜}} 与 {{agent:小白}}",
        AssemblyRole::System,
        &mut guard_second,
    );
    check(
        "AgentGuard 全上下文单 agent",
        second_report.expanded.len() == 1 && !second_report.removed.is_empty(),
        format!(
            "expanded={} removed={}",
            second_report.expanded.len(),
            second_report.removed.len()
        ),
    );
    // 虚拟时钟快进 1 天 → 时间变量跟随 (0 真等待)
    vc.advance(chrono::Duration::days(1));
    let mut guard_ff = AssemblyGuard::new();
    let (sys_txt2, _) =
        assembler.expand_text("{{agent:小夜}}", AssemblyRole::System, &mut guard_ff);
    let date1 = sys_txt2.trim_start_matches("我是小夜, 今天是 ").to_string();
    check(
        "虚拟时钟快进 → 时间变量变化",
        date0 != date1 && date1.len() == 10,
        format!("{date0} → {date1}"),
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
