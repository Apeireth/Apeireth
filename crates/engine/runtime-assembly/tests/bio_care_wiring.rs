//! W3.2 生物机制族 (仪式 SG-22 / 重聚 SG-19) → care 生产装配的装配级证明。
//!
//! 装配点: [`CareHeartbeatPoll`] 在规则轮询 + 提醒消费之后，取
//! [`BioCareInputs`] 供给 → `bio_care_candidates` → `CareEngine::poll_bio_candidate`
//! (同一条打扰纪律管道); 开关 `APEIRETH_ENABLE_BIO_CARE_CANDIDATES` 默认关,
//! 只认 `"1"`, 且 engine 的 `CareConfig` 是第二道门 (defense-in-depth)。
//!
//! **0 装诚实**: 生产数据供给 (presence 上次可见 / memory replay 可续线索 /
//! 仪式状态存储) 未接 (W3.3); 装配缺省 = [`NoBioCareInputs`] 空供给 ⇒ 开关开
//! 也不产候选。本文件用 fake 供给证明「有数据 → 候选 → 过 care 管道」,
//! 不声称生产数据面已接。

#![allow(dead_code)] // D-1（owner 裁定 2026-10-25）：测试夹具字段/方法（供用例构造完整对象，部分字段本文件内未被读取）
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_organ::attachment::{ResumeThread, ReunionPolicy};
use apeireth_organ::care::{
    CareClock, CareConfig, CareDiscipline, CareEngine, CareRule, CareStore, CareTrigger,
    FrozenCareClock, RuleFallbackJudge,
};
use apeireth_organ::ritual::{
    Cadence, RitualConfig, RitualKind, RitualSchedule, RitualState, Window,
};
use apeireth_runtime::canonical::heartbeat::{
    HeartbeatDispatch, HeartbeatPoll, HeartbeatPollOutcome,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::{
    bio_care_assembly_from_env_value, BioCareClock, BioCarePoll, CareHeartbeatConfig,
    CareHeartbeatPoll, CareHeartbeatProducer,
};
use apeireth_runtime_assembly::canonical::care_reminder::{
    bio_care_candidates, bio_care_candidates_enabled_from_env_value, BioCareInputs,
    NoBioCareInputs, ReunionInputs, RitualInputs, BIO_CARE_CANDIDATES_ENABLE_ENV,
};

/// 固定注入时刻 (2025-10-09T08:53:20Z); 测试永不读系统时钟。
const NOW: i64 = 1_760_000_000_000;
const NOW_U64: u64 = 1_760_000_000_000;
const HOUR_MS: u64 = 3_600_000;
const DAY_MS: u64 = 24 * HOUR_MS;

/// 心跳拍测试口径: 25 毫秒一拍 (真间隔, 不假睡)。
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(25);

fn open_discipline() -> CareDiscipline {
    CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
        ..CareDiscipline::default()
    }
}

fn quiet_discipline() -> CareDiscipline {
    // 0..1439 分钟 = 除 23:59 外的整日静音 (引擎现在时刻 = NOW)。
    CareDiscipline {
        quiet_start_minutes: Some(0),
        quiet_end_minutes: Some(1439),
        ..CareDiscipline::default()
    }
}

fn engine_with(dir: &Path, bio_enabled: bool, discipline: CareDiscipline) -> CareEngine {
    let store = CareStore::new(dir);
    let mut body = store.list().expect("空表体");
    body.discipline = discipline;
    store.save(&body).expect("落纪律");
    let clock: Arc<dyn CareClock> = Arc::new(FrozenCareClock::new(NOW));
    CareEngine::new(CareStore::new(dir), clock, Arc::new(RuleFallbackJudge)).with_care_config(
        CareConfig {
            bio_candidates_enabled: bio_enabled,
        },
    )
}

/// 既有规则面到点规则 (间隔 1 小时; 创建于 2 小时前 ⇒ 现在到点)。
fn add_due_rule(dir: &Path) {
    CareStore::new(dir)
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: NOW - i64::try_from(2 * HOUR_MS).expect("fits"),
        })
        .expect("到点规则入表");
}

fn due_ritual_inputs() -> RitualInputs {
    RitualInputs {
        states: vec![RitualState::fresh(RitualKind::MorningCheckIn)],
        config: RitualConfig::new(vec![RitualSchedule::new(
            RitualKind::MorningCheckIn,
            Cadence::Daily,
            Window::Flexible,
            6 * HOUR_MS,
        )]),
    }
}

fn reunion_inputs() -> ReunionInputs {
    ReunionInputs {
        last_seen_ms: NOW_U64 - 3 * DAY_MS,
        threads: vec![ResumeThread {
            id: "thread-1".to_string(),
            summary: "共同项目".to_string(),
            salience: 0.8,
            emotion_weight: 0.6,
            last_touched_ms: NOW_U64 - HOUR_MS,
        }],
        policy: ReunionPolicy::default(),
    }
}

/// fake 供给: 记录读取次数 (开关关 = 零读取的证据)。
struct FakeInputs {
    ritual: RitualInputs,
    reunion: Option<ReunionInputs>,
    reads: Arc<AtomicUsize>,
}

impl FakeInputs {
    fn ritual() -> Self {
        Self {
            ritual: due_ritual_inputs(),
            reunion: None,
            reads: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn reunion() -> Self {
        Self {
            ritual: RitualInputs::default(),
            reunion: Some(reunion_inputs()),
            reads: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl BioCareInputs for FakeInputs {
    fn ritual_inputs(&self) -> RitualInputs {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.ritual.clone()
    }

    fn reunion_inputs(&self) -> Option<ReunionInputs> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.reunion.clone()
    }
}

fn bio_poll(inputs: impl BioCareInputs + 'static) -> BioCarePoll {
    let clock: BioCareClock = Arc::new(|| NOW);
    BioCarePoll::new(Box::new(inputs), clock)
}

fn bio_clock() -> BioCareClock {
    Arc::new(|| NOW)
}

fn test_config() -> CareHeartbeatConfig {
    CareHeartbeatConfig {
        enabled: true,
        poll_interval: TEST_POLL_INTERVAL,
    }
}

fn store_history(dir: &Path) -> Vec<apeireth_organ::care::CareRecord> {
    CareStore::new(dir).list().expect("表体").history
}

/// 只计数的发起缝 + 最近一次标准回合输入留痕。
#[derive(Default)]
struct CountingDispatch {
    turns: Arc<AtomicUsize>,
    last_input: Arc<Mutex<Option<String>>>,
}

impl HeartbeatDispatch for CountingDispatch {
    fn dispatch(&mut self, turn_input: &str, _at_ms: i64) {
        self.turns.fetch_add(1, Ordering::SeqCst);
        *self
            .last_input
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(turn_input.to_string());
    }
}

fn wait_polls(producer: &CareHeartbeatProducer, target: u64) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if producer.status().polls >= target {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("超时未等到 {target} 拍 poll");
}

fn wait_turns(turns: &Arc<AtomicUsize>, target: usize) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if turns.load(Ordering::SeqCst) >= target {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("超时未等到 {target} 次发起");
}

/// ① 开关解析 fail-closed: 默认关, 只认 `"1"`; 关 = 不造供给面 (零读取)。
#[test]
fn the_bio_switch_is_off_by_default_and_only_accepts_one() {
    assert_eq!(
        BIO_CARE_CANDIDATES_ENABLE_ENV,
        "APEIRETH_ENABLE_BIO_CARE_CANDIDATES"
    );
    for raw in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("TRUE"),
        Some(" 1"),
    ] {
        assert!(
            !bio_care_candidates_enabled_from_env_value(raw),
            "非 \"1\" 一律关: {raw:?}"
        );
        assert!(
            bio_care_assembly_from_env_value(raw, Box::new(NoBioCareInputs), bio_clock()).is_none(),
            "关 = 不造供给面: {raw:?}"
        );
    }

    let (care_config, _bio) =
        bio_care_assembly_from_env_value(Some("1"), Box::new(NoBioCareInputs), bio_clock())
            .expect("开关开 = 造出供给面");
    assert!(care_config.bio_candidates_enabled());
    assert!(!CareConfig::default().bio_candidates_enabled());
}

/// ② 默认关 zero-change: 注入供给面但 engine 未开 = 供给口零读取、零候选;
/// 既有规则面输出与不注入供给面逐字节一致 (Quiet 与 Initiate 两档都比)。
#[test]
fn bio_off_is_zero_change_even_when_a_supply_is_attached() {
    // A: 无到点规则 = Quiet 档不变。
    let dir = tempfile::tempdir().expect("tempdir");
    let fake = FakeInputs::ritual();
    let reads = Arc::clone(&fake.reads);
    let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), false, open_discipline()))
        .with_bio_care(bio_poll(fake));
    for _ in 0..3 {
        assert_eq!(
            poll.poll().expect("poll"),
            HeartbeatPollOutcome::Quiet,
            "默认关 = 旧输出 (Quiet)"
        );
    }
    assert_eq!(reads.load(Ordering::SeqCst), 0, "开关关: 供给口零读取");
    assert!(store_history(dir.path()).is_empty(), "开关关: 零候选零台账");

    // B: 有到点规则 = Initiate 档也不变 (同规则同输入, 逐字节同回合输入)。
    let dir_with_bio = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir_with_bio.path());
    let fake = FakeInputs::ritual();
    let reads = Arc::clone(&fake.reads);
    let mut with_bio =
        CareHeartbeatPoll::new(engine_with(dir_with_bio.path(), false, open_discipline()))
            .with_bio_care(bio_poll(fake));
    let HeartbeatPollOutcome::Initiate {
        turn_input: with_bio_input,
        at_ms: with_bio_at,
    } = with_bio.poll().expect("poll")
    else {
        panic!("既有规则面到点必须照常发起");
    };

    let dir_plain = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir_plain.path());
    let mut plain = CareHeartbeatPoll::new(engine_with(dir_plain.path(), false, open_discipline()));
    let HeartbeatPollOutcome::Initiate {
        turn_input: plain_input,
        at_ms: plain_at,
    } = plain.poll().expect("poll")
    else {
        panic!("旧装配同样发起");
    };
    assert_eq!(
        reads.load(Ordering::SeqCst),
        0,
        "规则先发起 = 生物候选不被读"
    );
    assert_eq!(with_bio_at, plain_at, "同一时钟真值");
    assert_eq!(
        with_bio_input, plain_input,
        "开关关: 既有 care 面输出逐字节不变"
    );
    assert!(
        with_bio_input.contains("[care:check-in]"),
        "既有规则路径: {with_bio_input}"
    );
}

/// ③ 有数据 → 仪式候选 → 过 care 管道 → 标准回合 (真常驻心跳线程)。
#[test]
fn a_ritual_candidate_reaches_the_heartbeat_dispatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake = FakeInputs::ritual();
    let reads = Arc::clone(&fake.reads);
    let dispatch = CountingDispatch::default();
    let turns = Arc::clone(&dispatch.turns);
    let last_input = Arc::clone(&dispatch.last_input);
    let mut producer = CareHeartbeatProducer::start_care_with_feeds_and_bio(
        &test_config(),
        engine_with(dir.path(), true, open_discipline()),
        || NOW,
        dispatch,
        Vec::new(),
        bio_poll(fake),
    )
    .expect("心跳开关开 = 注册即起");

    wait_turns(&turns, 1);
    assert!(producer.shutdown(), "停机收心跳线程");

    let turn_input = last_input
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .expect("标准回合输入");
    assert!(
        turn_input.contains("[care:ritual.morning_check_in.due]"),
        "仪式候选进标准回合: {turn_input}"
    );
    let status = producer.status();
    assert_eq!(status.hits, 1, "至多一个发起, 频率上限挡住重复");
    assert!(reads.load(Ordering::SeqCst) >= 1, "供给口被读取");
    let history = store_history(dir.path());
    assert_eq!(history.len(), 1, "发起留痕一条: {history:?}");
    assert_eq!(history[0].kind, "ritual");
    assert!(history[0].spoke);
}

/// ④ 重聚候选 (Days, 非 Brief) → 过 care 管道 → 标准回合输入带既有安全模板。
#[test]
fn a_non_brief_reunion_candidate_reaches_the_heartbeat_dispatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dispatch = CountingDispatch::default();
    let turns = Arc::clone(&dispatch.turns);
    let last_input = Arc::clone(&dispatch.last_input);
    let mut producer = CareHeartbeatProducer::start_care_with_feeds_and_bio(
        &test_config(),
        engine_with(dir.path(), true, open_discipline()),
        || NOW,
        dispatch,
        Vec::new(),
        bio_poll(FakeInputs::reunion()),
    )
    .expect("心跳开关开 = 注册即起");

    wait_turns(&turns, 1);
    assert!(producer.shutdown(), "停机收心跳线程");

    let turn_input = last_input
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .expect("标准回合输入");
    assert!(
        turn_input.contains("[care:reunion.days]"),
        "重聚候选进标准回合: {turn_input}"
    );
    assert!(
        turn_input.contains("你回来了"),
        "重聚文案 = 既有安全模板: {turn_input}"
    );
    assert_eq!(producer.status().hits, 1);
    assert_eq!(store_history(dir.path())[0].kind, "reunion");
}

/// ⑤ 无数据 → 无候选 (开关开也一样): 空供给 = 心跳零发起零台账。
#[test]
fn no_supplied_data_means_no_candidate_on_the_heartbeat_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dispatch = CountingDispatch::default();
    let turns = Arc::clone(&dispatch.turns);
    let mut producer = CareHeartbeatProducer::start_care_with_feeds_and_bio(
        &test_config(),
        engine_with(dir.path(), true, open_discipline()),
        || NOW,
        dispatch,
        Vec::new(),
        bio_poll(NoBioCareInputs),
    )
    .expect("心跳开关开 = 注册即起");

    wait_polls(&producer, 3);
    let status = producer.status();
    assert!(producer.shutdown(), "停机收心跳线程");
    assert_eq!(status.hits, 0, "空供给: 零命中");
    assert_eq!(turns.load(Ordering::SeqCst), 0, "空供给: 零发起");
    assert!(store_history(dir.path()).is_empty(), "空供给: 零台账");
}

/// ⑥ 纪律不绕过: 勿扰窗口内, 到点仪式候选被 care 管道拦下 (零发起零台账)。
#[test]
fn the_quiet_window_suppresses_bio_candidates_on_the_heartbeat_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake = FakeInputs::ritual();
    let reads = Arc::clone(&fake.reads);
    let dispatch = CountingDispatch::default();
    let turns = Arc::clone(&dispatch.turns);
    let mut producer = CareHeartbeatProducer::start_care_with_feeds_and_bio(
        &test_config(),
        engine_with(dir.path(), true, quiet_discipline()),
        || NOW,
        dispatch,
        Vec::new(),
        bio_poll(fake),
    )
    .expect("心跳开关开 = 注册即起");

    wait_polls(&producer, 3);
    let status = producer.status();
    assert!(producer.shutdown(), "停机收心跳线程");
    assert_eq!(turns.load(Ordering::SeqCst), 0, "勿扰窗口内不发起");
    assert_eq!(status.hits, 0, "零命中");
    assert!(
        reads.load(Ordering::SeqCst) >= 1,
        "候选已产 (供给被读) 但被纪律拦下"
    );
    assert!(
        store_history(dir.path()).is_empty(),
        "拦下不写台账, 候选不丢"
    );

    // 同一候选直接问内核: 勿扰窗口 = Held(QuietWindow)。
    let mut engine = engine_with(dir.path(), true, quiet_discipline());
    let candidate = bio_care_candidates(
        &CareConfig {
            bio_candidates_enabled: true,
        },
        &FakeInputs::ritual(),
        NOW,
    )
    .into_iter()
    .next()
    .expect("仪式候选");
    assert_eq!(
        engine.poll_bio_candidate(&candidate).expect("内核裁决"),
        apeireth_organ::care::CarePoll::Held {
            rule_id: candidate.rule_id.clone(),
            hold: apeireth_organ::care::CareHold::QuietWindow,
        },
        "装配面不绕过纪律: 勿扰窗口 = Held(QuietWindow)"
    );
}
