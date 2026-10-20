//! 生物机制族 W3.4 · 节律门生产装配 (默认关) 的装配级证明。
//!
//! 本文件覆盖 W3.4 新增/接线面:
//! 1. [`CareHeartbeatProducer::start_care_with_feeds_and_circadian`] /
//!    [`CareHeartbeatProducer::start_care_with_feeds_bio_and_circadian`] 的
//!    `None` 档 = 不注入门 = 与旧方法输出逐字节一致;
//! 2. 门开 + 深夜 (睡窗) = 整拍 `Quiet`: 规则 / 提醒 / 生物候选一拍都不读
//!    (计数供给证明零读取), 心跳线程照常 poll (不是线程死了);
//! 3. 门开 + 白天 `Allowed` = 原规则面照常发起 (原逻辑不被改写);
//! 4. 时区显式生效: 同一 epoch 在 `tz = 0` (UTC 昼) 与 `tz = 480` (北京深夜)
//!    下门决策相反 (不推断本地时区);
//! 5. 会话起点口径: `session_start_ms` 决定超日窗口 (26 分钟前 = 休息窗;
//!    1 分钟前 = 专注窗), 决策可经 `circadian_decision()` 观测;
//! 6. env 开关 fail-closed: 只认 `"1"`, 关 = 不构造门 = 零时钟读取;
//! 7. 注入但开关关的门 = `Disabled` 档 = 原心跳逻辑逐字节不变。
//!
//! **0 装诚实**: 开关默认关; 时区默认 UTC (不推断本地时区); 睡眠压力源未接
//! (门内压力恒 0, 只有显式注入才变); 无 live E2E (本文件全部为进程内测试)。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_organ::care::{
    CareClock, CareConfig, CareDiscipline, CareEngine, CareRule, CareStore, CareTrigger,
    FrozenCareClock, ReminderCandidate, ReminderFeed, RuleFallbackJudge,
};
use apeireth_organ::circadian::{CircadianConfig, CircadianPhase, ProactiveGate};
use apeireth_organ::ritual::{
    Cadence, RitualConfig, RitualKind, RitualSchedule, RitualState, Window,
};
use apeireth_runtime::canonical::heartbeat::{
    HeartbeatDispatch, HeartbeatPoll, HeartbeatPollOutcome,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_circadian::{
    bio_circadian_enabled_from_env_value, bio_circadian_wiring_from_env_value,
    BioCircadianDecision, BioCircadianGate, CircadianGateClock, CircadianGateState,
    BIO_CIRCADIAN_ENABLE_ENV,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::{
    BioCareClock, BioCarePoll, CareHeartbeatConfig, CareHeartbeatPoll, CareHeartbeatProducer,
};
use apeireth_runtime_assembly::canonical::care_reminder::{
    BioCareInputs, ReunionInputs, RitualInputs,
};

/// 固定注入时刻 (2025-10-09T08:53:20Z = UTC+8 16:53:20, 昼/专注窗)。
const NOW: i64 = 1_760_000_000_000;
/// 测试显式配置的时区偏移 (UTC+8); 不被任何默认值/环境推断覆盖。
const TZ_BEIJING: i32 = 8 * 60;
const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;

/// 心跳拍测试口径: 25 毫秒一拍 (真间隔, 不假睡)。
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(25);

fn config(tz_offset_minutes: i32) -> CircadianConfig {
    CircadianConfig::new(
        7,
        23,
        tz_offset_minutes,
        25 * MINUTE_MS as u64,
        5 * MINUTE_MS as u64,
    )
}

/// 给定时区下本地 `hour:minute` 对应的 epoch 毫秒。
fn local_ms(hour: i64, minute: i64, tz_offset_minutes: i32) -> i64 {
    (hour * 60 + minute - i64::from(tz_offset_minutes)) * MINUTE_MS
}

fn gate(
    now: i64,
    session_start_ms: i64,
    tz_offset_minutes: i32,
    enabled: bool,
) -> BioCircadianGate {
    let clock: CircadianGateClock = Arc::new(move || now);
    let state = CircadianGateState::new(config(tz_offset_minutes), session_start_ms, now);
    BioCircadianGate::new(enabled, clock, state)
}

fn test_config() -> CareHeartbeatConfig {
    CareHeartbeatConfig {
        enabled: true,
        poll_interval: TEST_POLL_INTERVAL,
    }
}

fn open_discipline() -> CareDiscipline {
    CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
        ..CareDiscipline::default()
    }
}

fn engine_with(dir: &Path, now: i64, bio_enabled: bool) -> CareEngine {
    let store = CareStore::new(dir);
    let mut body = store.list().expect("空表体");
    body.discipline = open_discipline();
    store.save(&body).expect("落纪律");
    let clock: Arc<dyn CareClock> = Arc::new(FrozenCareClock::new(now));
    CareEngine::new(CareStore::new(dir), clock, Arc::new(RuleFallbackJudge)).with_care_config(
        CareConfig {
            bio_candidates_enabled: bio_enabled,
        },
    )
}

/// 既有规则面到点规则 (间隔 1 小时; 创建于 2 小时前 ⇒ 现在到点)。
fn add_due_rule(dir: &Path, now: i64) {
    CareStore::new(dir)
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: now - 2 * HOUR_MS,
        })
        .expect("到点规则入表");
}

fn store_history(dir: &Path) -> Vec<apeireth_organ::care::CareRecord> {
    CareStore::new(dir).list().expect("表体").history
}

fn due_ritual_inputs() -> RitualInputs {
    RitualInputs {
        states: vec![RitualState::fresh(RitualKind::MorningCheckIn)],
        config: RitualConfig::new(vec![RitualSchedule::new(
            RitualKind::MorningCheckIn,
            Cadence::Daily,
            Window::Flexible,
            6 * 60 * MINUTE_MS as u64,
        )]),
    }
}

fn bio_poll(inputs: impl BioCareInputs + 'static, at_ms: i64) -> BioCarePoll {
    let clock: BioCareClock = Arc::new(move || at_ms);
    BioCarePoll::new(Box::new(inputs), clock)
}

/// 只计数的发起缝 + 最近一次标准回合输入/时刻留痕。
#[derive(Default, Clone)]
struct CountingDispatch {
    turns: Arc<AtomicUsize>,
    last_input: Arc<Mutex<Option<String>>>,
    last_at: Arc<Mutex<Option<i64>>>,
}

impl HeartbeatDispatch for CountingDispatch {
    fn dispatch(&mut self, turn_input: &str, at_ms: i64) {
        self.turns.fetch_add(1, Ordering::SeqCst);
        *self
            .last_input
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(turn_input.to_string());
        *self
            .last_at
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(at_ms);
    }
}

/// 计数提醒源: 门拦下时应零读取。
struct CountingReminderFeed {
    reads: Arc<AtomicUsize>,
}

impl ReminderFeed for CountingReminderFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    fn mark_consumed(&self, _signal_id: &str) -> Result<(), String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// 计数生物候选供给: 门拦下时应零读取。
struct CountingBioInputs {
    reads: Arc<AtomicUsize>,
    ritual: RitualInputs,
}

impl BioCareInputs for CountingBioInputs {
    fn ritual_inputs(&self) -> RitualInputs {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.ritual.clone()
    }

    fn reunion_inputs(&self) -> Option<ReunionInputs> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        None
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

/// 等首次发起 → 停机收线程 → 取 (回合输入, 时刻, 命中数)。
fn one_hit(dispatch: &CountingDispatch, mut producer: CareHeartbeatProducer) -> (String, i64, u64) {
    wait_turns(&dispatch.turns, 1);
    let hits = producer.status().hits;
    assert!(producer.shutdown(), "停机收心跳线程");
    let input = dispatch
        .last_input
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .expect("标准回合输入");
    let at = dispatch
        .last_at
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .expect("标准回合时刻");
    (input, at, hits)
}

// ============================================
// 1. 新增 producer 方法 `None` 档 = 与旧方法逐字节一致
// ============================================

#[test]
fn no_gate_on_the_new_producer_is_byte_identical_to_the_original_path() {
    let plain_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(plain_dir.path(), NOW);
    let plain_dispatch = CountingDispatch::default();
    let plain = CareHeartbeatProducer::start_care_with_feeds(
        &test_config(),
        engine_with(plain_dir.path(), NOW, false),
        || NOW,
        plain_dispatch.clone(),
        Vec::new(),
    )
    .expect("原路径起");
    let plain_outcome = one_hit(&plain_dispatch, plain);

    let none_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(none_dir.path(), NOW);
    let none_dispatch = CountingDispatch::default();
    let none = CareHeartbeatProducer::start_care_with_feeds_and_circadian(
        &test_config(),
        engine_with(none_dir.path(), NOW, false),
        || NOW,
        none_dispatch.clone(),
        Vec::new(),
        None,
    )
    .expect("新方法 None 档起");
    let none_outcome = one_hit(&none_dispatch, none);

    assert_eq!(
        none_outcome, plain_outcome,
        "circadian = None = 不注入门: 回合输入/时刻/命中数与旧路径逐字节一致"
    );
    assert!(
        plain_outcome.0.contains("[care:check-in]"),
        "既有规则面照常发起: {}",
        plain_outcome.0
    );
    assert_eq!(store_history(plain_dir.path()).len(), 1, "旧路径留痕一条");
    assert_eq!(
        store_history(none_dir.path()).len(),
        1,
        "新方法 None 档留痕一条"
    );
}

#[test]
fn bio_and_no_gate_is_byte_identical_to_the_existing_bio_path() {
    let old_dir = tempfile::tempdir().expect("tempdir");
    let old_dispatch = CountingDispatch::default();
    let old = CareHeartbeatProducer::start_care_with_feeds_and_bio(
        &test_config(),
        engine_with(old_dir.path(), NOW, true),
        || NOW,
        old_dispatch.clone(),
        Vec::new(),
        bio_poll(
            CountingBioInputs {
                reads: Arc::new(AtomicUsize::new(0)),
                ritual: due_ritual_inputs(),
            },
            NOW,
        ),
    )
    .expect("既有生物路径起");
    let old_outcome = one_hit(&old_dispatch, old);

    let new_dir = tempfile::tempdir().expect("tempdir");
    let new_dispatch = CountingDispatch::default();
    let new = CareHeartbeatProducer::start_care_with_feeds_bio_and_circadian(
        &test_config(),
        engine_with(new_dir.path(), NOW, true),
        || NOW,
        new_dispatch.clone(),
        Vec::new(),
        bio_poll(
            CountingBioInputs {
                reads: Arc::new(AtomicUsize::new(0)),
                ritual: due_ritual_inputs(),
            },
            NOW,
        ),
        None,
    )
    .expect("新方法 None 档起");
    let new_outcome = one_hit(&new_dispatch, new);

    assert_eq!(
        new_outcome, old_outcome,
        "circadian = None = 不注入门: 生物候选路径输出逐字节一致"
    );
    assert!(
        old_outcome.0.contains("[care:ritual.morning_check_in.due]"),
        "仪式候选照常进标准回合: {}",
        old_outcome.0
    );
    assert_eq!(store_history(old_dir.path())[0].kind, "ritual");
    assert_eq!(store_history(new_dir.path())[0].kind, "ritual");
}

// ============================================
// 2. 门开 + 深夜 = 整拍 Quiet, 规则/提醒/生物候选零读取
// ============================================

#[test]
fn a_deep_night_gate_keeps_the_whole_producer_quiet_with_zero_candidate_reads() {
    // 北京 01:00 = 深夜 (睡窗), 门 = QuietHours; 到点规则故意存在。
    let now = local_ms(1, 0, TZ_BEIJING);
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), now);

    let reminder_reads = Arc::new(AtomicUsize::new(0));
    let bio_reads = Arc::new(AtomicUsize::new(0));
    let dispatch = CountingDispatch::default();
    let mut producer = CareHeartbeatProducer::start_care_with_feeds_bio_and_circadian(
        &test_config(),
        engine_with(dir.path(), now, true),
        move || now,
        dispatch.clone(),
        vec![Arc::new(CountingReminderFeed {
            reads: Arc::clone(&reminder_reads),
        })],
        bio_poll(
            CountingBioInputs {
                reads: Arc::clone(&bio_reads),
                ritual: due_ritual_inputs(),
            },
            now,
        ),
        Some(gate(now, now, TZ_BEIJING, true)),
    )
    .expect("心跳开关开 = 注册即起");

    wait_polls(&producer, 3);
    assert!(producer.is_running(), "被门拦下 ≠ 线程死掉 (照常 poll)");
    let status = producer.status();
    assert!(producer.shutdown(), "停机收心跳线程");

    assert_eq!(
        dispatch.turns.load(Ordering::SeqCst),
        0,
        "深夜 = 整拍 Quiet: 零发起"
    );
    assert_eq!(status.hits, 0, "零命中");
    assert_eq!(
        reminder_reads.load(Ordering::SeqCst),
        0,
        "门拦下: 提醒信号零读取"
    );
    assert_eq!(
        bio_reads.load(Ordering::SeqCst),
        0,
        "门拦下: 生物候选供给零读取"
    );
    assert!(
        store_history(dir.path()).is_empty(),
        "到点规则未发起: 零台账 (候选不丢)"
    );
}

// ============================================
// 3. 门开 + 白天 Allowed = 原逻辑照常发起
// ============================================

#[test]
fn a_daytime_allowed_gate_keeps_the_original_initiate_on_the_producer_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), NOW);
    let dispatch = CountingDispatch::default();
    let producer = CareHeartbeatProducer::start_care_with_feeds_and_circadian(
        &test_config(),
        engine_with(dir.path(), NOW, false),
        || NOW,
        dispatch.clone(),
        Vec::new(),
        Some(gate(NOW, NOW - MINUTE_MS, TZ_BEIJING, true)),
    )
    .expect("心跳开关开 = 注册即起");

    let (turn_input, at_ms, hits) = one_hit(&dispatch, producer);
    assert!(
        turn_input.contains("[care:check-in]"),
        "Allowed: 原规则面照常进标准回合: {turn_input}"
    );
    assert_eq!(at_ms, NOW, "发起时刻 = 注入时钟真值");
    assert_eq!(hits, 1, "恰发起一次 (频率上限挡住重复)");
    assert_eq!(store_history(dir.path()).len(), 1, "发起留痕一条");
}

// ============================================
// 4. 时区显式生效: 同一 epoch, tz 0 vs 480 决策相反
// ============================================

#[test]
fn the_timezone_offset_changes_the_gate_outcome_for_the_same_epoch() {
    // 该 epoch = UTC 17:00 (昼/放行) = 北京 01:00 (深夜/拦下)。
    let now = local_ms(1, 0, TZ_BEIJING);

    // tz = 0 (UTC 视图): 门放行 → 原规则面照常发起。
    let utc_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(utc_dir.path(), now);
    let utc_dispatch = CountingDispatch::default();
    let utc = CareHeartbeatProducer::start_care_with_feeds_and_circadian(
        &test_config(),
        engine_with(utc_dir.path(), now, false),
        move || now,
        utc_dispatch.clone(),
        Vec::new(),
        Some(gate(now, now - MINUTE_MS, 0, true)),
    )
    .expect("UTC 视图起");
    let (utc_input, _, utc_hits) = one_hit(&utc_dispatch, utc);
    assert!(
        utc_input.contains("[care:check-in]"),
        "UTC 17:00 = 昼: 放行照常发起: {utc_input}"
    );
    assert_eq!(utc_hits, 1);

    // tz = 480 (北京视图): 同一 epoch 是深夜 → 门拦下, 零发起。
    let beijing_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(beijing_dir.path(), now);
    let beijing_dispatch = CountingDispatch::default();
    let mut beijing = CareHeartbeatProducer::start_care_with_feeds_and_circadian(
        &test_config(),
        engine_with(beijing_dir.path(), now, false),
        move || now,
        beijing_dispatch.clone(),
        Vec::new(),
        Some(gate(now, now - MINUTE_MS, TZ_BEIJING, true)),
    )
    .expect("北京视图起");
    wait_polls(&beijing, 3);
    let beijing_hits = beijing.status().hits;
    assert!(beijing.shutdown(), "停机收心跳线程");
    assert_eq!(
        beijing_dispatch.turns.load(Ordering::SeqCst),
        0,
        "北京 01:00 = 深夜: 同一 epoch 零发起"
    );
    assert_eq!(beijing_hits, 0);
    assert!(store_history(beijing_dir.path()).is_empty());
}

// ============================================
// 5. 会话起点口径: 显式 session_start 决定超日窗口
// ============================================

#[test]
fn the_session_start_anchors_the_ultradian_window_at_the_poll_seam() {
    let now = NOW; // 北京 16:53 = 昼
    let rest_start = now - 26 * MINUTE_MS;
    let rest_gate = gate(now, rest_start, TZ_BEIJING, true);
    assert_eq!(
        rest_gate.state().session_start_ms(),
        rest_start,
        "会话起点 = 调用方显式给, 构造不改写"
    );
    assert_eq!(
        rest_gate.state().config().tz_offset_minutes,
        TZ_BEIJING,
        "时区来自显式配置"
    );

    let rest_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(rest_dir.path(), now);
    let mut rest = CareHeartbeatPoll::new(engine_with(rest_dir.path(), now, false))
        .with_circadian_gate(rest_gate);
    assert_eq!(
        rest.poll().expect("poll"),
        HeartbeatPollOutcome::Quiet,
        "26 分钟前起点 = 25 分钟专注窗已过 ⇒ 休息窗拦下"
    );
    assert_eq!(
        rest.circadian_decision(),
        Some(BioCircadianDecision::Suppressed(ProactiveGate::RestWindow)),
        "被门拦下 ≠ 未到点 (决策可观测)"
    );
    assert!(store_history(rest_dir.path()).is_empty());

    // 同一时刻, 起点 1 分钟前 = 专注窗 ⇒ 放行, 原规则面照常发起。
    let focus_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(focus_dir.path(), now);
    let mut focus = CareHeartbeatPoll::new(engine_with(focus_dir.path(), now, false))
        .with_circadian_gate(gate(now, now - MINUTE_MS, TZ_BEIJING, true));
    let outcome = focus.poll().expect("poll");
    assert!(
        matches!(outcome, HeartbeatPollOutcome::Initiate { .. }),
        "专注窗 + 昼 = 放行, 原逻辑照常发起: {outcome:?}"
    );
    assert_eq!(
        focus.circadian_decision(),
        Some(BioCircadianDecision::Allowed)
    );
}

// ============================================
// 6. env 开关 fail-closed + 懒加载 (关 = 不构造 = 零时钟读取)
// ============================================

#[test]
fn circadian_env_switch_is_fail_closed_and_wiring_off_is_lazy() {
    assert_eq!(BIO_CIRCADIAN_ENABLE_ENV, "APEIRETH_ENABLE_BIO_CIRCADIAN");
    for raw in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("TRUE"),
        Some("yes"),
        Some("on"),
        Some("2"),
        Some("01"),
    ] {
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&reads);
        let clock: CircadianGateClock = Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            NOW
        });
        assert!(
            !bio_circadian_enabled_from_env_value(raw),
            "非 \"1\" 一律关 (fail-closed): {raw:?}"
        );
        assert!(
            bio_circadian_wiring_from_env_value(raw, clock, config(TZ_BEIJING), NOW - MINUTE_MS)
                .is_none(),
            "开关关 = 不构造门 (懒加载): {raw:?}"
        );
        assert_eq!(
            reads.load(Ordering::SeqCst),
            0,
            "开关关 = 零时钟读取: {raw:?}"
        );
    }

    let gate = bio_circadian_wiring_from_env_value(
        Some("1"),
        Arc::new(|| NOW),
        config(TZ_BEIJING),
        NOW - MINUTE_MS,
    )
    .expect("开关开 = 构造门");
    assert!(gate.enabled());
    assert_eq!(
        gate.state().config().tz_offset_minutes,
        TZ_BEIJING,
        "时区来自显式配置, 不被默认值覆盖"
    );
    assert_eq!(gate.state().inputs().phase, CircadianPhase::Day);
    assert_eq!(gate.ticks(), 0, "构造不咨询, 只初始化快照");
}

// ============================================
// 7. 注入但开关关的门 = Disabled 档 = 原心跳逻辑零变化
// ============================================

#[test]
fn an_injected_gate_with_the_switch_off_is_zero_change_at_the_poll_seam() {
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), NOW);

    let reads = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&reads);
    let clock: CircadianGateClock = Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        NOW
    });
    let disabled = BioCircadianGate::new(
        false,
        clock,
        CircadianGateState::new(config(TZ_BEIJING), NOW - MINUTE_MS, NOW),
    );

    let plain_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(plain_dir.path(), NOW);
    let mut plain = CareHeartbeatPoll::new(engine_with(plain_dir.path(), NOW, false));
    let HeartbeatPollOutcome::Initiate {
        turn_input: plain_input,
        at_ms: plain_at,
    } = plain.poll().expect("poll")
    else {
        panic!("既有规则面必须发起");
    };

    let mut gated =
        CareHeartbeatPoll::new(engine_with(dir.path(), NOW, false)).with_circadian_gate(disabled);
    let HeartbeatPollOutcome::Initiate {
        turn_input: gated_input,
        at_ms: gated_at,
    } = gated.poll().expect("poll")
    else {
        panic!("门关 = 原逻辑照常发起");
    };
    assert_eq!(
        (gated_input, gated_at),
        (plain_input, plain_at),
        "门关: 既有 care 面输出逐字节不变"
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0, "门关: 零时钟读取");
    assert_eq!(
        gated.circadian_decision(),
        Some(BioCircadianDecision::Disabled),
        "门关 = Disabled 档 (未咨询)"
    );
    assert_eq!(store_history(dir.path()).len(), 1);
}
