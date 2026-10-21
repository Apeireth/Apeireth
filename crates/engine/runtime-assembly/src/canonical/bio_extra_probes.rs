//! 生物机制族观测面补格（W3.7 末批）：调质 / 昼夜节律 / 仪式 三条真实读数。
//!
//! 背景：`capabilities.bio_mechanisms` 的 14 格中，调质（SG-13）、昼夜节律
//! （SG-23）与仪式（SG-22）在生产装配里都已有**真实状态持有者**，但观测面一直
//! 停在「未接线」——本模块把三格接上**只读探测口**，值一律来自装配根正在使用的
//! 同一份状态（不复制、不旁路、不落盘）：
//!
//! | 格 | 真实数据源 | 读数口径 |
//! |---|---|---|
//! | 调质 | [`ToneState`]（`BioToneModule` / 订阅者同一 `Arc`） | 六维向量、基线、差值、行为参数、语气档 |
//! | 昼夜节律 | 开关板时区偏移 + 注入时刻（纯函数相位/门） | 相位、振幅、超日窗口形态、节律门 |
//! | 仪式 | [`RitualStateStore`]（`tool.ritual_done/skip` 同一账本） | 各仪式统计 + 到点决策 |
//!
//! 0 装口径（诚实边界）：
//!
//! - 三条都是**只读**：探测只读状态、不推进状态、不写账本、不改语气/门/候选；
//! - 开关关 = 这三格一条都不注册（沿用 `install_bio_self_status` 的武装语义），
//!   帧与未接观测面时逐字节相同；
//! - 槽空 = 单格 `null` + 「未接线」真因（不编数值）；
//! - 快照里的 `sleep_pressure` 显式 `null`：睡眠压力累积器在 cli 关怀心跳宿主
//!   （`bio_circadian`）内，装配根不拥有该状态，**不冒充**已有读数；
//! - 仪式账本读失败（损坏 / 版本不符）= 该格 `null` + fail-closed 真因，绝不在
//!   探测路径上重写账本。

use std::sync::Arc;

use apeireth_organ::circadian::{proactive_gate, CircadianConfig, CircadianState, SleepUrgency};
use apeireth_organ::neuromodulation::NeuromodulatorVector;
use apeireth_organ::ritual::{plan_rituals, RitualConfig};
use apeireth_tools_canonical::self_status::{BioMechanismProbe, BioMechanismReading};
use serde_json::{json, Map, Value};

use crate::canonical::bio_tone::ToneState;
use crate::canonical::care_reminder::RitualStateStore;

/// 昼夜节律观测配置（开关板转交的穷举事实：只含时区偏移与门开关）。
///
/// 唤醒/入睡时刻与超日时长沿用内核默认排程（[`CircadianConfig::default_schedule`]）；
/// 本类型不读 env、不读时钟——时刻由探测回调的 `now_ms` 注入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircadianProbeConfig {
    /// 本地时区相对 UTC 的偏移（分钟，构造时由 `CircadianConfig::new` 夹到 ±14h）。
    pub tz_offset_minutes: i32,
    /// 节律门开关事实（开关关时仍可注册探测？不：注册由 self_status 开关统一裁决；
    /// 本字段只用于读数里如实标注门是否随节律开关生效）。
    pub gate_enabled: bool,
}

impl CircadianProbeConfig {
    /// 由开关板事实构造。
    #[must_use]
    pub const fn new(tz_offset_minutes: i32, gate_enabled: bool) -> Self {
        Self {
            tz_offset_minutes,
            gate_enabled,
        }
    }
}

/// f32 工程参数 → JSON 数值（4 位小数，消除 `0.8f32 → 0.800000011920929` 噪声）。
fn round4(value: f32) -> f64 {
    (f64::from(value) * 10_000.0).round() / 10_000.0
}

/// 六维向量 → 稳定 JSON 对象（键名 = 内核字段名）。
fn vector_value(vector: &NeuromodulatorVector) -> Value {
    json!({
        "dopamine": round4(vector.dopamine),
        "serotonin": round4(vector.serotonin),
        "oxytocin": round4(vector.oxytocin),
        "cortisol": round4(vector.cortisol),
        "noradrenaline": round4(vector.noradrenaline),
        "acetylcholine": round4(vector.acetylcholine),
    })
}

// ============================================================
// 调质（SG-13）
// ============================================================

/// 调质机制探测口：读 `BioToneModule` / 总线订阅者共享的同一份 [`ToneState`]。
#[must_use]
pub fn neuromodulation_probe(state: Arc<ToneState>) -> Arc<dyn BioMechanismProbe> {
    Arc::new(NeuromodulationProbe { state })
}

struct NeuromodulationProbe {
    state: Arc<ToneState>,
}

impl BioMechanismProbe for NeuromodulationProbe {
    fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
        let vector = self.state.vector();
        let baseline = NeuromodulatorVector::baseline();
        let behavior = self.state.behavior_params();
        let value = json!({
            "vector": vector_value(&vector),
            "baseline": vector_value(&baseline),
            "delta": {
                "dopamine": round4(vector.dopamine - baseline.dopamine),
                "serotonin": round4(vector.serotonin - baseline.serotonin),
                "oxytocin": round4(vector.oxytocin - baseline.oxytocin),
                "cortisol": round4(vector.cortisol - baseline.cortisol),
                "noradrenaline": round4(vector.noradrenaline - baseline.noradrenaline),
                "acetylcholine": round4(vector.acetylcholine - baseline.acetylcholine),
            },
            "behavior": {
                "warmth": round4(behavior.warmth),
                "patience_factor": round4(behavior.patience_factor),
                "initiative_frequency": round4(behavior.initiative_frequency),
                "curiosity_gain": round4(behavior.curiosity_gain),
                "caution_threshold": round4(behavior.caution_threshold),
                "verbosity": round4(behavior.verbosity),
            },
            "tone_style_is_baseline": self.state.is_baseline(),
            "tone_clauses": self.state.tone_style().tone_clauses(),
            "basis": "六维为工程参数（数值不是主观情绪断言）；逻辑时刻由注入时钟给出，事件源经总线订阅/人批发布",
        });
        Ok(BioMechanismReading::wired(Some(value), None, None))
    }
}

// ============================================================
// 昼夜节律（SG-23）
// ============================================================

/// 昼夜节律机制探测口：由开关板时区偏移 + 探测时刻算出的纯函数相位 / 门。
#[must_use]
pub fn circadian_probe(config: CircadianProbeConfig) -> Arc<dyn BioMechanismProbe> {
    Arc::new(CircadianProbe { config })
}

struct CircadianProbe {
    config: CircadianProbeConfig,
}

/// 由观测配置构造内核配置：唤醒/入睡/超日时长取默认排程，只覆盖时区偏移。
fn kernel_config(config: CircadianProbeConfig) -> CircadianConfig {
    let base = CircadianConfig::default_schedule();
    CircadianConfig::new(
        base.wake_hour,
        base.sleep_hour,
        config.tz_offset_minutes,
        base.ultradian_focus_ms,
        base.ultradian_rest_ms,
    )
}

impl BioMechanismProbe for CircadianProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let config = kernel_config(self.config);
        let state = CircadianState::new(config);
        let phase = state.phase_at(now_ms);
        // 会话起点不在装配根（它在 cli 关怀心跳宿主的带内），故此处**不报真实
        // 超日进度**：以快照时刻为周期起点，只报配置形态（读数为"形态"，不是进度）。
        let window = state.window_at(now_ms, now_ms);
        // 睡眠压力累积器同样不在装配根 → 以中性档（Normal）过门，并在值里显式
        // 标注 sleep_pressure = null（不冒充已有压力读数）。
        let gate = proactive_gate(phase, window, SleepUrgency::Normal);
        let value = json!({
            "timezone_offset_minutes": config.tz_offset_minutes,
            "wake_hour": config.wake_hour,
            "sleep_hour": config.sleep_hour,
            "local_minute_of_day": state.local_minute_of_day(now_ms),
            "phase": phase.label(),
            "phase_amplitude": state.amplitude_at(now_ms),
            "ultradian_phase": window.phase.label(),
            "ultradian_remaining_ms": window.remaining_ms,
            "ultradian_basis": "会话起点不在装配根：以快照时刻为周期起点，只报配置形态，不当作真实会话进度",
            "sleep_pressure": null,
            "sleep_pressure_reason": "睡眠压力累积器在 cli 关怀心跳宿主（bio_circadian）内，装配根不复制该状态；门以中性档（Normal）过，不冒充高/低紧迫",
            "proactive_gate": gate.label(),
            "proactive_allows": gate.allows_proactive(),
            "gate_enabled": self.config.gate_enabled,
        });
        Ok(BioMechanismReading::wired(Some(value), None, None))
    }
}

// ============================================================
// 仪式（SG-22）
// ============================================================

/// 仪式机制探测口：读 `tool.ritual_done/skip` 写入的同一本仪式账本。
#[must_use]
pub fn ritual_state_probe(store: Arc<RitualStateStore>) -> Arc<dyn BioMechanismProbe> {
    Arc::new(RitualProbe { store })
}

struct RitualProbe {
    store: Arc<RitualStateStore>,
}

/// 单条仪式统计 → JSON（字段全部来自账本，不补造）。
fn ritual_state_value(state: &apeireth_organ::ritual::RitualState) -> Value {
    json!({
        "last_done_ms": state.last_done_ms,
        "consecutive_done": state.consecutive_done,
        "skipped_count": state.skipped_count,
        "muted_until_ms": state.muted_until_ms,
    })
}

impl BioMechanismProbe for RitualProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let (config, states) = self
            .store
            .inputs()
            .map_err(|error| format!("仪式状态账本读取失败 (fail-closed, 未编读数): {error}"))?;
        let now = u64::try_from(now_ms).unwrap_or(0);
        let decisions = plan_rituals(now, &states, &config);
        let mut kinds = Map::new();
        for state in &states {
            kinds.insert(state.kind.as_str().to_string(), ritual_state_value(state));
        }
        let actionable = decisions.iter().filter(|d| d.due.is_actionable()).count();
        let value = json!({
            "state_count": states.len(),
            "schedule_count": config.schedules.len(),
            "kinds": Value::Object(kinds),
            "decisions": decisions
                .iter()
                .map(|decision| json!({
                    "kind": decision.kind.as_str(),
                    "due": decision.due.as_str(),
                    "actionable": decision.due.is_actionable(),
                    "suggested_message_key": decision.suggested_message_key,
                }))
                .collect::<Vec<_>>(),
            "actionable_count": actionable,
            "basis": "账本 = tool.ritual_done / tool.ritual_skip 写入的同一文件（statistics only）；本格只读，不投递、不打卡、不看 streak 文案",
        });
        Ok(BioMechanismReading::wired(Some(value), None, None))
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use apeireth_organ::ritual::RitualKind;

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(tag: &str) -> PathBuf {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-bio-extra-{tag}-{}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn tone_state() -> Arc<ToneState> {
        let clock: crate::canonical::bio_tone::ToneClock = Arc::new(|| 1_000_u64);
        Arc::new(ToneState::new(clock))
    }

    #[test]
    fn neuromodulation_probe_reports_the_shared_state_vector_verbatim() {
        let state = tone_state();
        let probe = neuromodulation_probe(Arc::clone(&state));
        let reading = probe.snapshot(1_000).expect("probe");
        assert!(reading.wired);
        assert!(reading.reason.is_none());
        let value = reading.value.expect("value");
        let vector = state.vector();
        assert_eq!(value["vector"]["dopamine"], round4(vector.dopamine));
        assert_eq!(value["tone_style_is_baseline"], state.is_baseline());
        assert_eq!(
            value["behavior"]["warmth"],
            round4(state.behavior_params().warmth)
        );
    }

    #[test]
    fn neuromodulation_probe_is_read_only_and_repeatable() {
        let state = tone_state();
        let before = state.vector();
        let probe = neuromodulation_probe(Arc::clone(&state));
        let first = probe.snapshot(1_000).expect("probe").value.expect("value");
        let second = probe.snapshot(2_000).expect("probe").value.expect("value");
        assert_eq!(first["vector"], second["vector"]);
        assert_eq!(state.vector(), before, "探测不得推进调质状态");
    }

    #[test]
    fn circadian_probe_reports_the_injected_phase_and_flags_missing_pressure() {
        // 2023-11-14T22:13:20Z；+08:00 → 次日 06:13 本地（默认 07:00 起床前的睡窗后段）。
        let now_ms = 1_700_000_000_000_i64;
        let config = CircadianProbeConfig::new(8 * 60, true);
        let reading = circadian_probe(config).snapshot(now_ms).expect("probe");
        assert!(reading.wired);
        let value = reading.value.expect("value");
        let kernel = kernel_config(config);
        let expected_phase = CircadianState::new(kernel).phase_at(now_ms).label();
        assert_eq!(value["phase"], expected_phase);
        assert_eq!(value["timezone_offset_minutes"], 480);
        assert!(value["sleep_pressure"].is_null());
        assert!(value["sleep_pressure_reason"]
            .as_str()
            .expect("reason")
            .contains("装配根"));
        assert_eq!(value["gate_enabled"], true);
        // 睡窗内：节律门不得放行主动接触。
        assert_eq!(value["proactive_allows"], false);
    }

    #[test]
    fn circadian_probe_with_unknown_offset_is_clamped_not_fabricated() {
        let config = CircadianProbeConfig::new(99 * 60, false);
        let value = circadian_probe(config)
            .snapshot(0)
            .expect("probe")
            .value
            .expect("value");
        assert_eq!(value["timezone_offset_minutes"], 840, "±14h 夹取");
        assert_eq!(value["gate_enabled"], false);
    }

    #[test]
    fn ritual_probe_reports_ledger_states_without_writing() {
        let dir = temp_dir("ritual-read");
        let store = Arc::new(RitualStateStore::under_data_dir(&dir));
        assert!(store.record_done(RitualKind::MorningCheckIn, 5_000).is_ok());
        let path = store.path().to_path_buf();
        let before = std::fs::read(&path).expect("ledger bytes before");

        let reading = ritual_state_probe(Arc::clone(&store))
            .snapshot(10_000)
            .expect("probe");
        assert!(reading.wired);
        let value = reading.value.expect("value");
        assert_eq!(value["state_count"], 1);
        assert_eq!(value["kinds"]["morning_check_in"]["last_done_ms"], 5_000);
        assert_eq!(value["kinds"]["morning_check_in"]["consecutive_done"], 1);

        let after = std::fs::read(&path).expect("ledger bytes after");
        assert_eq!(before, after, "探测不得改写仪式账本");
    }

    #[test]
    fn ritual_probe_fails_closed_on_a_corrupt_ledger_and_never_overwrites() {
        let dir = temp_dir("ritual-corrupt");
        let path = dir.join("ritual_state.json");
        let garbage = b"{not-json".to_vec();
        std::fs::write(&path, &garbage).expect("write garbage");

        let store = Arc::new(RitualStateStore::at(&path));
        let error = ritual_state_probe(store)
            .snapshot(10_000)
            .expect_err("corrupt ledger must fail closed");
        assert!(error.contains("读取失败"), "error = {error}");
        assert_eq!(
            std::fs::read(&path).expect("bytes"),
            garbage,
            "不得覆盖损坏账本"
        );
    }

    #[test]
    fn ritual_probe_reports_an_empty_ledger_as_zero_states_not_an_error() {
        let dir = temp_dir("ritual-empty");
        let store = Arc::new(RitualStateStore::under_data_dir(&dir));
        let value = ritual_state_probe(store)
            .snapshot(10_000)
            .expect("missing ledger = empty, not error")
            .value
            .expect("value");
        assert_eq!(value["state_count"], 0);
        assert_eq!(value["actionable_count"], 0);
    }

    #[test]
    fn ritual_config_schedules_are_reported_verbatim() {
        // 默认配置无排程：账本有状态也不产生到点决策（决策来自配置，不来自统计）。
        let dir = temp_dir("ritual-config");
        let store = Arc::new(RitualStateStore::under_data_dir(&dir));
        assert!(store.record_done(RitualKind::EveningWrap, 1_000).is_ok());
        let value = ritual_state_probe(store)
            .snapshot(10_000)
            .expect("probe")
            .value
            .expect("value");
        assert_eq!(value["schedule_count"], 0);
        // 无排程不产「到点」：每个已有状态各一条决策，但一律 `not_due`、不可投递。
        let decisions = value["decisions"].as_array().expect("decisions");
        assert_eq!(decisions.len(), 1, "一个已有状态一条决策");
        assert_eq!(decisions[0]["kind"], "evening_wrap");
        assert_eq!(decisions[0]["due"], "not_due");
        assert_eq!(decisions[0]["actionable"], false);
        assert_eq!(value["actionable_count"], 0, "空排程不得编出到点决策");
    }

    #[test]
    fn ritual_default_config_is_the_kernel_default() {
        let dir = temp_dir("ritual-default-config");
        let store = Arc::new(RitualStateStore::under_data_dir(&dir));
        let (config, states) = store.inputs().expect("inputs");
        assert_eq!(config, RitualConfig::default());
        assert!(states.is_empty());
    }
}
