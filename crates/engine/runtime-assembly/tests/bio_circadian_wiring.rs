//! 生物机制族 W3.3 · 节律门 → 心跳接线测试 (`APEIRETH_ENABLE_BIO_CIRCADIAN`, 默认关)。
//!
//! 覆盖装配级契约:
//! 1. env 开关: 仅 `"1"` 开, 其余 fail-closed; 关 = 不构造门 (懒加载);
//! 2. 纯函数门: Allowed / QuietHours / RestWindow / ConsolidationDue 四档 +
//!    `should_suppress_proactive` 的 `None` / `Some(gate)` 语义;
//! 3. 时区必须显式配置: 同一 epoch 在不同 `tz_offset_minutes` 下相位不同
//!    (不推断本地时区);
//! 4. 默认关 zero-change: 不注入门 / 注入但开关关 = 原心跳逻辑逐字节不变,
//!    且门关时零时钟读取;
//! 5. 心跳最前拦下: 规则 / 提醒 / 生物候选一拍都不读 (用 panic 供给证明),
//!    `Quiet` 靠 gate 观测字段区分「被门拦下」与「未到点」;
//! 6. 门放行时既有规则面照常发起 (原逻辑不被改写)。
//!
//! **0 装诚实**: 真实本地时区未接 (显式配置); 睡眠压力源未接 (显式注入);
//! `production.rs` 未注册 (注册由 Lead 后续做); 无 live E2E。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_organ::care::{
    CareClock, CareConfig, CareDiscipline, CareEngine, CareRule, CareStore, CareTrigger,
    FrozenCareClock, ReminderCandidate, ReminderFeed, RuleFallbackJudge,
};
use apeireth_organ::circadian::{
    CircadianConfig, CircadianPhase, ProactiveGate, SleepPressure, SleepUrgency, UltradianPhase,
    UltradianWindow,
};
use apeireth_runtime::canonical::heartbeat::{HeartbeatPoll, HeartbeatPollOutcome};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_circadian::{
    bio_circadian_enabled_from_env_value, bio_circadian_wiring_from_env_value,
    should_suppress_proactive, BioCircadianDecision, BioCircadianGate, CircadianGateClock,
    CircadianGateInputs, CircadianGateState, BIO_CIRCADIAN_ENABLE_ENV,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::{
    BioCareClock, BioCarePoll, CareHeartbeatPoll,
};
use apeireth_runtime_assembly::canonical::care_reminder::{
    BioCareInputs, ReunionInputs, RitualInputs,
};

/// 固定注入时刻 (2025-10-09T08:53:20Z = UTC+8 16:53:20, 昼/专注窗)。
const NOW_MS: i64 = 1_760_000_000_000;
/// 测试显式配置的时区偏移 (UTC+8); 不被任何默认值/环境推断覆盖。
const TZ_OFFSET_MINUTES: i32 = 8 * 60;
const MINUTE_MS: i64 = 60_000;

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

fn add_due_rule(dir: &Path, now: i64) {
    CareStore::new(dir)
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: now - 2 * 60 * MINUTE_MS,
        })
        .expect("到点规则入表");
}

fn history(dir: &Path) -> Vec<apeireth_organ::care::CareRecord> {
    CareStore::new(dir).list().expect("表体").history
}

fn gate(
    now: i64,
    session_start_ms: i64,
    tz_offset_minutes: i32,
    pressure_level: Option<f64>,
    enabled: bool,
) -> BioCircadianGate {
    let clock: CircadianGateClock = Arc::new(move || now);
    let mut state = CircadianGateState::new(config(tz_offset_minutes), session_start_ms, now);
    if let Some(level) = pressure_level {
        state.set_pressure(SleepPressure::with_level(level));
    }
    BioCircadianGate::new(enabled, clock, state)
}

/// 供给口/提醒口的 panic 探针: 节律门若没在最前拦下, 测试立刻失败 (零读取证据)。
struct PanicBioInputs;

impl BioCareInputs for PanicBioInputs {
    fn ritual_inputs(&self) -> RitualInputs {
        panic!("节律门拦下时不得读取仪式供给");
    }

    fn reunion_inputs(&self) -> Option<ReunionInputs> {
        panic!("节律门拦下时不得读取重聚供给");
    }
}

struct PanicReminderFeed;

impl ReminderFeed for PanicReminderFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        panic!("节律门拦下时不得读取提醒信号");
    }

    fn mark_consumed(&self, _signal_id: &str) -> Result<(), String> {
        panic!("节律门拦下时不得标记提醒消费");
    }
}

fn bio_poll(now: i64) -> BioCarePoll {
    let clock: BioCareClock = Arc::new(move || now);
    BioCarePoll::new(Box::new(PanicBioInputs), clock)
}

// ============================================
// 1. env 开关: 只认 "1", 默认关, 关 = 不构造
// ============================================

#[test]
fn circadian_env_switch_is_fail_closed_and_lazy() {
    assert_eq!(BIO_CIRCADIAN_ENABLE_ENV, "APEIRETH_ENABLE_BIO_CIRCADIAN");
    assert!(bio_circadian_enabled_from_env_value(Some("1")));
    assert!(bio_circadian_enabled_from_env_value(Some(" 1 ")));

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
        Some("1.0"),
    ] {
        assert!(
            !bio_circadian_enabled_from_env_value(raw),
            "非 \"1\" 一律关: {raw:?}"
        );
        assert!(
            bio_circadian_wiring_from_env_value(
                raw,
                Arc::new(|| NOW_MS),
                config(TZ_OFFSET_MINUTES),
                NOW_MS - MINUTE_MS,
            )
            .is_none(),
            "开关关 = 不构造 (懒加载): {raw:?}"
        );
    }

    let gate = bio_circadian_wiring_from_env_value(
        Some("1"),
        Arc::new(|| NOW_MS),
        config(TZ_OFFSET_MINUTES),
        NOW_MS - MINUTE_MS,
    )
    .expect("开关开");
    assert!(gate.enabled());
    assert_eq!(
        gate.state().config().tz_offset_minutes,
        TZ_OFFSET_MINUTES,
        "时区来自显式配置, 不被推断覆盖"
    );
    assert_eq!(gate.ticks(), 0);
    assert_eq!(gate.last_decision(), None, "构造不咨询, 只初始化快照");
}

// ============================================
// 2. 纯函数门: 四档 + Some/None 语义
// ============================================

#[test]
fn should_suppress_covers_all_four_gate_states() {
    let mut state = CircadianGateState::new(config(TZ_OFFSET_MINUTES), 0, 0);
    let focus = UltradianWindow {
        phase: UltradianPhase::Focus,
        remaining_ms: 1,
    };
    let rest = UltradianWindow {
        phase: UltradianPhase::Rest,
        remaining_ms: 1,
    };
    let cases = [
        (CircadianPhase::Day, focus, SleepUrgency::Low, None),
        (
            CircadianPhase::DeepNight,
            focus,
            SleepUrgency::Low,
            Some(ProactiveGate::QuietHours),
        ),
        (
            CircadianPhase::Dawn,
            rest,
            SleepUrgency::High,
            Some(ProactiveGate::QuietHours),
        ),
        (
            CircadianPhase::Morning,
            rest,
            SleepUrgency::Normal,
            Some(ProactiveGate::RestWindow),
        ),
        (
            CircadianPhase::Night,
            focus,
            SleepUrgency::High,
            Some(ProactiveGate::ConsolidationDue),
        ),
    ];
    for (phase, window, urgency, expected) in cases {
        state.set_inputs(CircadianGateInputs {
            at_ms: NOW_MS,
            phase,
            window,
            urgency,
        });
        assert_eq!(state.inputs().at_ms, NOW_MS);
        assert_eq!(
            should_suppress_proactive(&state),
            expected,
            "phase={phase:?}, window={window:?}, urgency={urgency:?}"
        );
        assert_eq!(
            state.gate(),
            expected.unwrap_or(ProactiveGate::Allowed),
            "内核 proactive_gate 是唯一优先级真源"
        );
    }
}

// ============================================
// 3. 时区显式: 同一 epoch, 不同 tz → 不同相位
// ============================================

#[test]
fn tick_uses_the_explicit_timezone_and_never_infers_one() {
    // UTC 00:30 = 北京 08:30 (晨) vs UTC 00:30 (深夜)。
    let now_ms = 30 * MINUTE_MS;
    let beijing = CircadianGateState::new(config(TZ_OFFSET_MINUTES), now_ms, now_ms);
    let utc = CircadianGateState::new(config(0), now_ms, now_ms);
    assert_eq!(beijing.inputs().phase, CircadianPhase::Morning);
    assert_eq!(utc.inputs().phase, CircadianPhase::DeepNight);
    assert_eq!(should_suppress_proactive(&beijing), None);
    assert_eq!(
        should_suppress_proactive(&utc),
        Some(ProactiveGate::QuietHours),
        "同一时刻在未配置 tz 的视图下是深夜"
    );

    // 推进时刻: 相位按显式配置重新计算 (不是静态编造)。
    let mut beijing = beijing;
    beijing.tick(local_ms(1, 0, TZ_OFFSET_MINUTES));
    assert_eq!(beijing.inputs().phase, CircadianPhase::DeepNight);
    assert_eq!(beijing.inputs().at_ms, local_ms(1, 0, TZ_OFFSET_MINUTES));
}

// ============================================
// 4. 默认关 zero-change + 门关零时钟读取
// ============================================

#[test]
fn default_off_is_byte_identical_and_never_reads_the_gate_clock() {
    // A: 到点规则 = Initiate 档, 有门 (开关关) vs 无门, 输出逐字节一致。
    let plain_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(plain_dir.path(), NOW_MS);
    let mut plain = CareHeartbeatPoll::new(engine_with(plain_dir.path(), NOW_MS, false));
    let plain_outcome = plain.poll().expect("poll");
    let HeartbeatPollOutcome::Initiate {
        turn_input: plain_input,
        at_ms: plain_at,
    } = &plain_outcome
    else {
        panic!("既有规则面必须发起: {plain_outcome:?}");
    };
    assert!(plain_input.contains("[care:check-in]"));

    let reads = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&reads);
    let clock: CircadianGateClock = Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        NOW_MS
    });
    let disabled = BioCircadianGate::new(
        false,
        clock,
        CircadianGateState::new(config(TZ_OFFSET_MINUTES), NOW_MS - MINUTE_MS, NOW_MS),
    );
    let gated_dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(gated_dir.path(), NOW_MS);
    let mut gated = CareHeartbeatPoll::new(engine_with(gated_dir.path(), NOW_MS, false))
        .with_circadian_gate(disabled);
    let gated_outcome = gated.poll().expect("poll");
    let HeartbeatPollOutcome::Initiate {
        turn_input: gated_input,
        at_ms: gated_at,
    } = &gated_outcome
    else {
        panic!("门关 = 原逻辑照常发起: {gated_outcome:?}");
    };
    assert_eq!(
        (gated_input, gated_at),
        (plain_input, plain_at),
        "开关关: 既有 care 面输出逐字节不变"
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0, "门关: 零时钟读取");
    assert_eq!(
        gated.circadian_decision(),
        Some(BioCircadianDecision::Disabled)
    );

    // B: 无到点规则 = Quiet 档也不变。
    let plain_dir = tempfile::tempdir().expect("tempdir");
    let mut plain = CareHeartbeatPoll::new(engine_with(plain_dir.path(), NOW_MS, false));
    assert_eq!(plain.poll().expect("poll"), HeartbeatPollOutcome::Quiet);

    let gated_dir = tempfile::tempdir().expect("tempdir");
    let disabled = BioCircadianGate::new(
        false,
        Arc::new(|| NOW_MS),
        CircadianGateState::new(config(TZ_OFFSET_MINUTES), NOW_MS - MINUTE_MS, NOW_MS),
    );
    let mut gated = CareHeartbeatPoll::new(engine_with(gated_dir.path(), NOW_MS, false))
        .with_circadian_gate(disabled);
    assert_eq!(
        gated.poll().expect("poll"),
        HeartbeatPollOutcome::Quiet,
        "开关关: 既有 Quiet 档逐字节不变"
    );
    assert!(history(gated_dir.path()).is_empty());
}

// ============================================
// 5. QuietHours: 心跳最前拦下, 规则/提醒/生物候选一拍都不读
// ============================================

#[test]
fn quiet_hours_suppress_the_whole_heartbeat_before_any_candidate() {
    let now = local_ms(1, 0, TZ_OFFSET_MINUTES); // 北京 01:00 = 深夜
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), now);

    let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), now, true))
        .with_reminder_feed(Arc::new(PanicReminderFeed))
        .with_bio_care(bio_poll(now))
        .with_circadian_gate(gate(now, now, TZ_OFFSET_MINUTES, None, true));

    // panic 供给已经证明: 规则 / 提醒 / 生物候选都没有被读。
    assert_eq!(
        poll.poll().expect("poll"),
        HeartbeatPollOutcome::Quiet,
        "节律门拦下 = Quiet (不新增 outcome 类型)"
    );
    assert_eq!(
        poll.circadian_decision(),
        Some(BioCircadianDecision::Suppressed(ProactiveGate::QuietHours))
    );
    assert!(
        history(dir.path()).is_empty(),
        "拦下不写台账, 到点规则下拍仍可发"
    );
}

// ============================================
// 6. RestWindow: 心跳最前拦下
// ============================================

#[test]
fn the_rest_window_suppresses_at_the_heartbeat_seam() {
    let now = local_ms(16, 53, TZ_OFFSET_MINUTES);
    // 会话起点 = 26 分钟前 → 25 分钟专注窗已过, 当前在 5 分钟休息窗。
    let session_start = now - 26 * MINUTE_MS;
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), now);

    let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), now, true))
        .with_reminder_feed(Arc::new(PanicReminderFeed))
        .with_bio_care(bio_poll(now))
        .with_circadian_gate(gate(now, session_start, TZ_OFFSET_MINUTES, None, true));
    assert_eq!(poll.poll().expect("poll"), HeartbeatPollOutcome::Quiet);
    assert_eq!(
        poll.circadian_decision(),
        Some(BioCircadianDecision::Suppressed(ProactiveGate::RestWindow))
    );
    assert!(history(dir.path()).is_empty());
}

// ============================================
// 7. ConsolidationDue: 心跳最前拦下
// ============================================

#[test]
fn consolidation_due_suppresses_at_the_heartbeat_seam() {
    let now = local_ms(16, 53, TZ_OFFSET_MINUTES);
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), now);

    let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), now, true))
        .with_reminder_feed(Arc::new(PanicReminderFeed))
        .with_bio_care(bio_poll(now))
        .with_circadian_gate(gate(
            now,
            now - MINUTE_MS,
            TZ_OFFSET_MINUTES,
            Some(0.8),
            true,
        ));
    assert_eq!(poll.poll().expect("poll"), HeartbeatPollOutcome::Quiet);
    assert_eq!(
        poll.circadian_decision(),
        Some(BioCircadianDecision::Suppressed(
            ProactiveGate::ConsolidationDue
        ))
    );
    assert!(history(dir.path()).is_empty());
}

// ============================================
// 8. 门放行: 原逻辑照常发起; Quiet 时用决策字段区分「未到点」
// ============================================

#[test]
fn an_allowed_gate_keeps_the_original_initiate_and_quiet_is_distinguishable() {
    let now = local_ms(16, 53, TZ_OFFSET_MINUTES);

    // 放行 + 到点规则 = 原 Initiate 路径。
    let dir = tempfile::tempdir().expect("tempdir");
    add_due_rule(dir.path(), now);
    let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), now, false))
        .with_circadian_gate(gate(now, now - MINUTE_MS, TZ_OFFSET_MINUTES, None, true));
    let outcome = poll.poll().expect("poll");
    let HeartbeatPollOutcome::Initiate { turn_input, at_ms } = outcome else {
        panic!("门放行 = 既有规则面照常发起: {outcome:?}");
    };
    assert!(turn_input.contains("[care:check-in]"), "{turn_input}");
    assert_eq!(at_ms, now);
    assert_eq!(
        poll.circadian_decision(),
        Some(BioCircadianDecision::Allowed)
    );
    assert_eq!(history(dir.path()).len(), 1, "发起留痕一条");

    // 放行 + 无到点规则 = Quiet, 但决策 = Allowed (不是被门拦下)。
    let dir = tempfile::tempdir().expect("tempdir");
    let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), now, false))
        .with_circadian_gate(gate(now, now - MINUTE_MS, TZ_OFFSET_MINUTES, None, true));
    assert_eq!(poll.poll().expect("poll"), HeartbeatPollOutcome::Quiet);
    assert_eq!(
        poll.circadian_decision(),
        Some(BioCircadianDecision::Allowed),
        "Quiet + Allowed = 未到点; Quiet + Suppressed = 被节律门拦下"
    );
    assert!(history(dir.path()).is_empty());
}

// ============================================
// 9. 显式睡眠压力源: 累积到高紧迫度才拦, 清账后放行
// ============================================

#[test]
fn explicit_pressure_accumulation_toggles_consolidation_due() {
    let now = local_ms(16, 53, TZ_OFFSET_MINUTES);
    let mut state = CircadianGateState::new(config(TZ_OFFSET_MINUTES), now - MINUTE_MS, now);
    assert_eq!(should_suppress_proactive(&state), None);

    // 醒着 16 小时 @ load=1.0 → 饱和 1.0 → 高紧迫度。
    state.accumulate_pressure(1.0, 16 * 60 * MINUTE_MS as u64);
    state.tick(now);
    assert_eq!(state.pressure().level(), 1.0);
    assert_eq!(state.inputs().urgency, SleepUrgency::High);
    assert_eq!(
        should_suppress_proactive(&state),
        Some(ProactiveGate::ConsolidationDue)
    );

    // 休息 8 小时清账 → 回到 Low → 放行。
    state.relieve_pressure(8 * 60 * MINUTE_MS as u64);
    state.tick(now);
    assert_eq!(state.pressure().level(), 0.0);
    assert_eq!(should_suppress_proactive(&state), None);
}
