//! K 轴 L2 提醒侧消费端 + L5 守护例外档 (care 同源接缝) 行为回归。
//!
//! 钉住: ①信号→候选→标准回合全链 ②打扰纪律同源 (提醒与规则同账勿扰/频率)
//! ③守护例外档 (破勿扰 + 「守护告警」标注 + 可配) ④守护告警频率独立上限
//! ⑤消费位语义 (发起标消费 / 拦下不丢 / 台账去重)。

use std::sync::Arc;

use apeireth_organ::care::{
    consume_reminders, is_guard_alert_record, CareClock, CareDiscipline, CareEngine, CareHold,
    CareJudgmentInput, CareModelPort, CarePoll, CareRecord, CareStore, CareTier, FrozenCareClock,
    MemoryReminderFeed, ModelContextJudge, ReminderCandidate, ReminderFeed, RuleFallbackJudge,
};

const AT: i64 = 1_700_000_000_000; // 2023-11-14 22:13:20 UTC (分钟 of day = 1333)

/// 开窗纪律 (勿扰关): 只测频率/消费位时不让勿扰代跑。
fn open_discipline() -> CareDiscipline {
    CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
        ..CareDiscipline::default()
    }
}

/// 22:00-23:00 勿扰窗 (覆盖 AT 的 22:13)。
fn quiet_at_at() -> CareDiscipline {
    CareDiscipline {
        quiet_start_minutes: Some(22 * 60),
        quiet_end_minutes: Some(23 * 60),
        ..CareDiscipline::default()
    }
}

fn store_with(discipline: CareDiscipline) -> (tempfile::TempDir, CareStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    let mut body = store.list().expect("empty body");
    body.discipline = discipline;
    store.save(&body).expect("save");
    (dir, store)
}

fn engine_with(store: CareStore, at_ms: i64) -> (Arc<FrozenCareClock>, CareEngine) {
    let clock = Arc::new(FrozenCareClock::new(at_ms));
    let clock_for_engine: Arc<dyn CareClock> = clock.clone();
    let engine = CareEngine::new(store, clock_for_engine, Arc::new(RuleFallbackJudge));
    (clock, engine)
}

fn weather_candidate(id: &str) -> ReminderCandidate {
    ReminderCandidate {
        signal_id: id.to_string(),
        tier: CareTier::Reminder,
        trigger: "temperature_drop".to_string(),
        severity: "warn".to_string(),
        message: "降温提醒: 2.5°C 偏冷, 记得添衣".to_string(),
        evidence: "local: temperature_c=2.5 (mock-fixture)".to_string(),
        source: "tool.weather".to_string(),
        created_at_ms: AT,
    }
}

fn guard_candidate(id: &str) -> ReminderCandidate {
    ReminderCandidate {
        signal_id: id.to_string(),
        tier: CareTier::GuardAlert,
        trigger: "temperature_above(30) [守护告警]".to_string(),
        severity: "critical".to_string(),
        message: "室内过热, 请查看".to_string(),
        evidence: "thermo-1 (temperature_above(30)) online=true".to_string(),
        source: "tool.guard_watch".to_string(),
        created_at_ms: AT,
    }
}

fn seed_spoke(store: &CareStore, rule_id: &str, kind: &str, at_ms: i64) {
    // 种子只入台账 (发起留痕锚点归规则, 这些测试无规则行): 走信号档记账面。
    store
        .record_signal_care(CareRecord {
            rule_id: rule_id.to_string(),
            at_ms,
            judge: "rule_fallback".to_string(),
            spoke: true,
            note: "种子台账".to_string(),
            kind: kind.to_string(),
        })
        .expect("seed record");
}

/// ① 信号→候选→标准回合全链: 提醒 = 关怀候选的一种, 走标准回合输入。
#[test]
fn a_weather_signal_flows_signal_to_candidate_to_standard_turn() {
    let (_dir, store) = store_with(open_discipline());
    let (_clock, mut engine) = engine_with(store, AT);
    let signal = weather_candidate("wxsig-1");

    match engine.poll_reminder(&signal).expect("poll_reminder") {
        CarePoll::Initiating(initiative) => {
            assert_eq!(initiative.candidate.rule_id, "wxsig-1");
            assert_eq!(initiative.candidate.tier, CareTier::Reminder);
            assert_eq!(initiative.candidate.material, signal.message);
            let turn = initiative.to_turn_input();
            assert!(
                turn.contains(&signal.message),
                "标准回合输入必须带建议语: {turn}"
            );
            assert!(
                turn.contains("[care:wxsig-1]"),
                "标准回合输入带候选身份: {turn}"
            );
        }
        other => panic!("提醒候选必须发起: {other:?}"),
    }
    // 台账留痕: kind = reminder (同源纪律的计数口径)。
    let body = engine.store().list().expect("list");
    assert_eq!(body.history.len(), 1);
    assert_eq!(body.history[0].kind, "reminder");
    assert_eq!(body.history[0].rule_id, "wxsig-1");
}

/// ② 打扰纪律同源: 提醒与规则**同一本纪律同一本频率账** (勿扰照拦 / 频率同账)。
#[test]
fn reminders_walk_the_same_disturbance_discipline_as_rules() {
    // 勿扰窗口: 提醒照拦 (提醒不许绕过勿扰)。
    let (_dir, store) = store_with(quiet_at_at());
    let (_clock, mut engine) = engine_with(store, AT);
    match engine
        .poll_reminder(&weather_candidate("wxsig-q"))
        .expect("poll")
    {
        CarePoll::Held { hold, .. } => {
            assert_eq!(hold, CareHold::QuietWindow, "提醒在勿扰窗内被拦")
        }
        other => panic!("提醒必须被勿扰拦下: {other:?}"),
    }

    // 频率同账: 规则档已发 1 次 (frequency_max=1) → 提醒被同一份预算拦下。
    let (_dir, store) = store_with(CareDiscipline {
        frequency_max: 1,
        frequency_window_hours: 12,
        ..open_discipline()
    });
    seed_spoke(&store, "check-in", "rule", AT - 3_600_000);
    let (_clock, mut engine) = engine_with(store, AT);
    match engine
        .poll_reminder(&weather_candidate("wxsig-f"))
        .expect("poll")
    {
        CarePoll::Held { hold, .. } => {
            assert_eq!(hold, CareHold::FrequencyCap, "提醒与规则同账频率上限")
        }
        other => panic!("同源频率上限必须拦下提醒: {other:?}"),
    }
}

/// ③ 守护例外档: 默认破勿扰 + 必带「守护告警」标注 (回合输入 + 台账双留痕)。
#[test]
fn a_guard_alert_breaks_the_quiet_window_with_the_guard_annotation() {
    let (_dir, store) = store_with(quiet_at_at());
    let (_clock, mut engine) = engine_with(store, AT);
    match engine
        .poll_reminder(&guard_candidate("gwalert-1"))
        .expect("poll")
    {
        CarePoll::Initiating(initiative) => {
            let turn = initiative.to_turn_input();
            assert!(
                turn.starts_with("[守护告警]"),
                "破勿扰发起必带守护告警标注: {turn}"
            );
        }
        other => panic!("守护告警必须破勿扰发起: {other:?}"),
    }
    let body = engine.store().list().expect("list");
    assert_eq!(body.history[0].kind, "guard_alert");
    assert!(
        is_guard_alert_record(&body.history[0]),
        "独立频率上限的计数口径 = kind 列守护档"
    );
    assert!(
        body.history[0].note.contains("守护告警"),
        "台账留痕带标注: {}",
        body.history[0].note
    );
}

/// ③ 可配: `guard_alert_breaks_quiet=false` = 例外档收起, 守护告警照常被勿扰拦。
#[test]
fn the_guard_quiet_break_is_configurable_and_defaults_on() {
    let discipline = quiet_at_at();
    assert!(
        discipline.guard_alert_breaks_quiet,
        "裁定默认: 安全告警可破勿扰"
    );
    let (_dir, store) = store_with(CareDiscipline {
        guard_alert_breaks_quiet: false,
        ..quiet_at_at()
    });
    let (_clock, mut engine) = engine_with(store, AT);
    match engine
        .poll_reminder(&guard_candidate("gwalert-c"))
        .expect("poll")
    {
        CarePoll::Held { hold, .. } => {
            assert_eq!(hold, CareHold::QuietWindow, "例外档收起 = 照常被勿扰拦")
        }
        other => panic!("可配关掉后守护告警必须被拦: {other:?}"),
    }
}

/// ④ 频率独立上限: 守护告警不占普通预算, 自己一档有界 (默认 1h ≤ 3 条)。
#[test]
fn guard_alerts_have_an_independent_frequency_cap() {
    // 普通预算被规则发满 (frequency_max=1) → 守护告警照发 (独立上限)。
    let (_dir, store) = store_with(CareDiscipline {
        frequency_max: 1,
        frequency_window_hours: 12,
        ..open_discipline()
    });
    seed_spoke(&store, "check-in", "rule", AT - 3_600_000);
    let (_clock, mut engine) = engine_with(store, AT);
    assert!(
        matches!(
            engine
                .poll_reminder(&guard_candidate("gwalert-i1"))
                .expect("poll"),
            CarePoll::Initiating(_)
        ),
        "普通频率已满不得误伤安全告警"
    );

    // 守护独立上限: 1h 窗口内已发满 (guard_frequency_max=2) → 拦下。
    let (_dir, store) = store_with(CareDiscipline {
        guard_frequency_max: 2,
        guard_frequency_window_hours: 1,
        ..open_discipline()
    });
    seed_spoke(&store, "gwalert-s1", "guard_alert", AT - 600_000);
    seed_spoke(&store, "gwalert-s2", "guard_alert", AT - 300_000);
    let (_clock, mut engine) = engine_with(store, AT);
    match engine
        .poll_reminder(&guard_candidate("gwalert-i2"))
        .expect("poll")
    {
        CarePoll::Held { hold, .. } => {
            assert_eq!(hold, CareHold::FrequencyCap, "守护频率独立上限自己有界")
        }
        other => panic!("守护独立频率上限必须拦下: {other:?}"),
    }

    // 守护告警不占普通预算: 发过守护告警后提醒照发。
    let (_dir, store) = store_with(open_discipline());
    seed_spoke(&store, "gwalert-s3", "guard_alert", AT - 60_000);
    let (_clock, mut engine) = engine_with(store, AT);
    assert!(
        matches!(
            engine
                .poll_reminder(&weather_candidate("wxsig-z"))
                .expect("poll"),
            CarePoll::Initiating(_)
        ),
        "守护告警不得占普通关怀频率预算"
    );
}

/// ⑤ 消费位: 发起/说不标消费 (不重发); 拦下不标 (候选不丢); 台账去重兜底。
#[test]
fn the_consumption_cursor_marks_only_adjudicated_signals() {
    // 发起 = 标消费, 第二轮不再交付。
    let (_dir, store) = store_with(open_discipline());
    let (_clock, mut engine) = engine_with(store, AT);
    let feed = MemoryReminderFeed::new();
    feed.push(weather_candidate("wxsig-c1"));
    let initiative = consume_reminders(&mut engine, &feed)
        .expect("consume")
        .expect("必须发起");
    assert_eq!(initiative.candidate.rule_id, "wxsig-c1");
    assert!(feed.consumed().contains("wxsig-c1"), "发起后标消费");
    assert!(
        consume_reminders(&mut engine, &feed)
            .expect("consume")
            .is_none(),
        "已消费信号不再交付"
    );

    // 拦下 = 不标消费 (候选不丢, 窗口滑过重试)。
    let (_dir, store) = store_with(quiet_at_at());
    let (_clock, mut engine) = engine_with(store, AT);
    let feed = MemoryReminderFeed::new();
    feed.push(weather_candidate("wxsig-h1"));
    assert!(consume_reminders(&mut engine, &feed)
        .expect("consume")
        .is_none());
    assert!(feed.consumed().is_empty(), "拦下不得标消费 (候选不丢)");
    assert_eq!(feed.pending().expect("pending").len(), 1, "候选仍在源里");

    // 台账去重兜底 (重启后新消费位也查台账): 已裁决信号不再裁决。
    let (_dir, store) = store_with(open_discipline());
    let (_clock, mut engine) = engine_with(store, AT);
    let feed = MemoryReminderFeed::new();
    feed.push(weather_candidate("wxsig-d1"));
    consume_reminders(&mut engine, &feed).expect("consume");
    // 新源 (消费位清零) 但台账在: poll_reminder 显式 Idle (不重发不重复计数)。
    let fresh = MemoryReminderFeed::new();
    fresh.push(weather_candidate("wxsig-d1"));
    assert!(matches!(
        engine
            .poll_reminder(&fresh.pending().expect("pending")[0])
            .expect("poll"),
        CarePoll::Idle
    ));
}

/// 说不 (模型判定) 也标消费 —— 裁决完就消费, 台账留痕如实。
#[test]
fn a_declined_signal_is_adjudicated_and_marked_consumed() {
    struct NoSpeakPort;
    impl CareModelPort for NoSpeakPort {
        fn verdict(&self, _input: &CareJudgmentInput) -> Result<String, String> {
            Ok(r#"{"speak": false, "focus": null, "reason": "他正在忙"}"#.to_string())
        }
    }
    let (_dir, store) = store_with(open_discipline());
    let clock = Arc::new(FrozenCareClock::new(AT));
    let clock_for_engine: Arc<dyn CareClock> = clock.clone();
    let mut engine = CareEngine::new(
        store,
        clock_for_engine,
        Arc::new(ModelContextJudge::new(Arc::new(NoSpeakPort))),
    );
    let feed = MemoryReminderFeed::new();
    feed.push(weather_candidate("wxsig-n1"));
    assert!(consume_reminders(&mut engine, &feed)
        .expect("consume")
        .is_none());
    assert!(
        feed.consumed().contains("wxsig-n1"),
        "说不 = 裁决完 = 标消费"
    );
    let body = engine.store().list().expect("list");
    assert_eq!(body.history.len(), 1, "说不也留痕");
    assert!(!body.history[0].spoke);
    assert_eq!(body.history[0].kind, "reminder");
}
