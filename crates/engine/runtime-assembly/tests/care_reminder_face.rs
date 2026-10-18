//! K 轴 L2 提醒侧 + L5 守护告警的装配级证明: 两本信号账 → 关怀候选 → 标准
//! 回合输入 (信号→候选→标准回合全链, 跨 crate) + 消费位台账去重 + 自述关怀节
//! 天气提醒计数真值。

use std::sync::Arc;

use apeireth_organ::care::{
    consume_reminders, CareClock, CareDiscipline, CareEngine, CarePoll, CareStore, CareTier,
    FrozenCareClock, ReminderFeed, RuleFallbackJudge,
};
use apeireth_runtime_assembly::{care_status_probe, GuardAlertFeed, WeatherSignalFeed};
use apeireth_tools_canonical::device::guard_watch::{
    GuardAlert, GuardAlertLedger, GUARD_ALERT_ANNOTATION,
};
use apeireth_tools_canonical::device::weather::{weather_signals, MockWeatherSource, SignalLedger};

const AT: i64 = 1_700_000_000_000;

/// 开窗纪律 (勿扰关): 本文件钉消费链, 不让勿扰代跑 (例外档行为在 organ 套件)。
fn open_discipline() -> CareDiscipline {
    CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
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

fn engine_with(store: CareStore, at_ms: i64) -> CareEngine {
    let clock: Arc<dyn CareClock> = Arc::new(FrozenCareClock::new(at_ms));
    CareEngine::new(store, clock, Arc::new(RuleFallbackJudge))
}

/// ① 信号→候选→标准回合全链 (跨 crate): `tool.weather` 信号入账 → 消费桥 →
/// care 引擎 → 标准回合输入 (建议语如实进回合)。
#[test]
fn a_weather_signal_flows_from_the_ledger_to_the_standard_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 信号面: tool.weather 只交信号 (mock 档低温 fixture → 降温提醒信号)。
    let signals = weather_signals(&MockWeatherSource::fixture("local", AT));
    assert_eq!(signals.len(), 1);
    let ledger = SignalLedger::under_data_dir(dir.path());
    ledger.append(&signals[0]).expect("信号入账");

    // 消费端: 同数据目录的消费桥 → care 引擎 (同源纪律)。
    let (_care_dir, store) = store_with(open_discipline());
    let mut engine = engine_with(store, AT);
    let feed = WeatherSignalFeed::under_data_dir(dir.path());
    let initiative = consume_reminders(&mut engine, &feed)
        .expect("consume")
        .expect("信号必须发起关怀");
    assert_eq!(initiative.candidate.rule_id, signals[0].signal_id);
    assert_eq!(initiative.candidate.tier, CareTier::Reminder);
    let turn = initiative.to_turn_input();
    assert!(
        turn.contains(&signals[0].message),
        "标准回合输入必须带建议语: {turn}"
    );
    assert!(
        turn.contains(&signals[0].signal_id),
        "标准回合输入带信号身份: {turn}"
    );
    // 消费位: 发起即标, 不重发。
    assert!(feed.pending().expect("pending").is_empty());
}

/// ② 守护告警 → 高优先关怀候选 (守护例外档): 告警账 → 消费桥 → 候选带
/// 「守护告警」标注, 破勿扰发起的回合输入带标注前缀。
#[test]
fn a_guard_alert_flows_as_a_high_priority_care_candidate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let alerts = GuardAlertLedger::under_data_dir(dir.path());
    alerts
        .append(&GuardAlert {
            alert_id: "gwalert-1".to_string(),
            rule_id: "heat".to_string(),
            device_id: "thermo-1".to_string(),
            kind: "guard".to_string(),
            condition: "temperature_above(30)".to_string(),
            severity: "critical".to_string(),
            message: "室内过热, 请查看".to_string(),
            evidence: "thermo-1 (temperature_above(30))".to_string(),
            source: "tool.guard_watch".to_string(),
            source_data: "mock-fixture".to_string(),
            created_at_ms: AT,
            annotation: GUARD_ALERT_ANNOTATION.to_string(),
        })
        .expect("告警入账");

    let feed = GuardAlertFeed::under_data_dir(dir.path());
    let candidates = feed.pending().expect("pending");
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].tier,
        CareTier::GuardAlert,
        "守护告警 = 高优先档"
    );
    assert!(
        candidates[0].trigger.contains(GUARD_ALERT_ANNOTATION),
        "候选带守护告警标注: {}",
        candidates[0].trigger
    );

    // 走引擎: 告警候选发起 (开窗纪律) → 标准回合输入。
    let (_care_dir, store) = store_with(open_discipline());
    let mut engine = engine_with(store, AT);
    let initiative = consume_reminders(&mut engine, &feed)
        .expect("consume")
        .expect("守护告警必须发起");
    let turn = initiative.to_turn_input();
    assert!(
        turn.starts_with("[守护告警]"),
        "守护告警回合输入带标注前缀: {turn}"
    );
}

/// ③ 消费位台账去重 (跨重启): 新消费位实例不重发已裁决信号 (台账兜底)。
#[test]
fn adjudicated_signals_are_not_redelivered_across_a_fresh_cursor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let signals = weather_signals(&MockWeatherSource::fixture("local", AT));
    let ledger = SignalLedger::under_data_dir(dir.path());
    ledger.append(&signals[0]).expect("信号入账");

    let (_care_dir, store) = store_with(open_discipline());
    let mut engine = engine_with(store, AT);
    let feed = WeatherSignalFeed::under_data_dir(dir.path());
    consume_reminders(&mut engine, &feed).expect("consume");

    // 新消费位 (模拟重启: 进程内消费位清零) —— 台账去重兜底, 不重发。
    let fresh = WeatherSignalFeed::under_data_dir(dir.path());
    let pending = fresh.pending().expect("pending");
    assert_eq!(pending.len(), 1, "新消费位看到未标信号");
    assert!(
        matches!(
            engine.poll_reminder(&pending[0]).expect("poll"),
            CarePoll::Idle
        ),
        "台账里已裁决的信号不再裁决 (不重发不重复计数)"
    );
}

/// ④ 自述关怀节天气提醒计数: `weather-signals.jsonl` 真值计数 (缺账 = 0 如实)。
#[test]
fn the_care_section_counts_weather_reminders_honestly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let now_ms: Arc<dyn Fn() -> i64 + Send + Sync> = Arc::new(|| AT);
    let last_interaction: Arc<dyn Fn() -> Option<i64> + Send + Sync> = Arc::new(|| None);
    let probe = care_status_probe(dir.path().to_path_buf(), now_ms, last_interaction);

    // 缺账 = 0 条 (合法空态, 不编数)。
    let status = probe().expect("缺账不是错误");
    assert_eq!(status.weather_reminders, Some(0), "缺账如实报 0");

    // 两条信号入账 = 计数 2 (与提醒侧消费面同一本信号账)。
    let signals = weather_signals(&MockWeatherSource::fixture("local", AT));
    let ledger = SignalLedger::under_data_dir(dir.path());
    ledger.append(&signals[0]).expect("信号入账");
    let mut second = signals[0].clone();
    second.signal_id = "wxsig-dup-2".to_string();
    ledger.append(&second).expect("信号入账");
    let status = probe().expect("可读");
    assert_eq!(status.weather_reminders, Some(2), "计数 = 信号账真值");
    assert_eq!(status.rules, Some(0), "规则数如实 (空表)");
    assert_eq!(status.last_care_at_ms, None, "尚无关怀不编时间");
}
