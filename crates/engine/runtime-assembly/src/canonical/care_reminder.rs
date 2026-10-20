//! 提醒侧消费桥 (K 轴 L2 提醒 + L5 守护告警 → care 引擎同源接缝) 的装配面。
//!
//! 两本信号账 → [`apeireth_organ::care::ReminderCandidate`] 的映射与消费位:
//!
//! ```text
//! tool.weather   → weather-signals.jsonl ─┐
//!   (只交信号)                             ├─→ ReminderFeed (本面) ─→ CareEngine
//! tool.guard_watch → guard-alerts.jsonl ──┘        │                 ::poll_reminder
//!   (只看不动, 告警)                                │                 (同源纪律)
//!                                        消费位 (进程内 mark_consumed;
//!                                        台账去重由内核兜底)
//! ```
//!
//! - [`WeatherSignalFeed`]: 天气提醒信号 → [`CareTier::Reminder`] 候选;
//! - [`GuardAlertFeed`]: 守护告警 → [`CareTier::GuardAlert`] 候选 (高优先,
//!   打扰纪律守护例外档: 破勿扰可配 + 「守护告警」标注 + 频率独立上限);
//! - [`CompositeReminderFeed`]: 多本账一并喂 (天气 + 守护)。
//!
//! # 生物机制候选供给面 (W3.2)
//!
//! 仪式 (SG-22) 与重聚 (SG-19) 两条生物机制候选源在 W3.1 已落在 organ crate
//! ([`ritual_candidates_for`] / [`reunion_candidate_for`], 默认关)。本面是它们
//! 的**供给与产候选装配**:
//!
//! ```text
//! APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1 (默认关, 只认 "1")
//!   │
//!   ▼
//! BioCareInputs (宿主注入的供给口)          ← 生产缺省 = NoBioCareInputs (空供给)
//!   ├─ ritual_inputs()  → RitualState + RitualConfig → ritual_candidates_for
//!   └─ reunion_inputs() → last_seen_ms + 可续线索 + 策略 → plan_reunion
//!                                              → reunion_candidate_for
//!   ▼
//! bio_care_candidates() → Vec<CareCandidate>
//!   ▼
//! CareEngine::poll_bio_candidate (同一条纪律管道: 勿扰 / 专注 / 频率 / 预算
//! / 发起闸 → 判定 → 台账; 本面不绕过任何一道)
//! ```
//!
//! **数据供给边界 (如实, W3.5 更新)**: **仪式状态存储已接** ——
//! [`ritual_state_store::RitualStateStore`] 读 `<care_dir>/ritual_state.json`
//! (StoredDoc 拒开 + 原子写), 生产由 cli 装配面 (`CareHeartbeatSource`) 注入;
//! 排程缺省回落 organ 空排程 ⇒ **默认 0 候选 (不主动)**, 显式配置排程才有到点
//! 候选。重聚的 presence 上次可见 / 可续线索同样由 cli 宿主注入 (W3.3); 装配
//! 缺省仍用 [`NoBioCareInputs`] 空供给。测试用 fake 供给验证「有数据 → 候选 →
//! 过 care 管道」。取不到数据必须返回空 / `None`, 不编造。
//!
//! **候选消费不回写供给 (W3.5 收窄)**: 本面只产候选, care 管道裁决后**仍不**
//! 自动把 `ritual.last_done_ms` 写回 —— 生产没有「仪式完成」事件源, 写回走 cli
//! 装配面的显式注入口 `CareHeartbeatSource::record_ritual_done(kind, now_ms)`
//! (完成事件源: 待后续从 care 交互 / 工具面接); 同一到点状态在频率/预算/发起闸
//! 放行后可能再次发起 —— 现有纪律仍是唯一限流, 本面不加第二套节奏。重聚已读位
//! 仍未回写。
//!
//! # 0 装诚实
//!
//! - 消费位是**进程内值** (重启清零): 重启后已裁决信号由内核的台账去重兜底
//!   (见 `apeireth_organ::care::reminder` 模块注), 不冒充持久消费位;
//! - 信号账读失败 = `Err(真因)` (拒开, 不静默跳本); 坏行按账面语义跳过并计数。
//! - 本面**零设备动作**: 只读两本信号账 (看到 ≠ 动手, 动手走 device_control)。
//! - 生物候选开关默认关; 候选源已接装配, 但生产数据供给 (presence / memory
//!   replay / 仪式状态存储) 未接时**不产候选** —— 不写成已启用。

use std::collections::BTreeSet;
use std::sync::Mutex;

use apeireth_organ::attachment::{plan_reunion, ResumeThread, ReunionPolicy};
use apeireth_organ::care::{
    reunion_candidate_for, ritual_candidates_for, CareCandidate, CareConfig, CareTier,
    ReminderCandidate, ReminderFeed,
};
use apeireth_organ::ritual::{RitualConfig, RitualState};

use apeireth_tools_canonical::device::guard_watch::{GuardAlert, GuardAlertLedger};
use apeireth_tools_canonical::device::weather::{ReminderSignal, SignalLedger};

/// 仪式状态存储 (`ritual_state.json`, SG-22 补半程): StoredDoc 拒开语义 + 持久档
/// 原子写, 与 `care_rules.json` 同一文件账范式; 由 cli 生物供给在开关打开时读取。
#[path = "ritual_state_store.rs"]
pub mod ritual_state_store;
pub use ritual_state_store::{
    ritual_state_doc_compat, RitualStateBody, RitualStateStore, RitualStateStoreError,
    RITUAL_STATE_DOC_NAME, RITUAL_STATE_DOC_VERSION, RITUAL_STATE_FILE_NAME,
};

/// 天气提醒信号 → 关怀提醒候选的消费桥 (`weather-signals.jsonl`)。
pub struct WeatherSignalFeed {
    ledger: SignalLedger,
    consumed: Mutex<BTreeSet<String>>,
}

impl WeatherSignalFeed {
    /// 绑定天气信号账 (数据目录落点)。
    pub fn under_data_dir(data_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            ledger: SignalLedger::under_data_dir(data_dir),
            consumed: Mutex::new(BTreeSet::new()),
        }
    }

    /// 绑定显式账路径。
    pub fn at(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            ledger: SignalLedger::at(path),
            consumed: Mutex::new(BTreeSet::new()),
        }
    }

    /// 信号账 (自述/测试同账)。
    pub fn ledger(&self) -> &SignalLedger {
        &self.ledger
    }
}

/// 天气提醒信号 → 候选 (同源字段映射, 不改语义)。
pub fn weather_signal_to_candidate(signal: &ReminderSignal) -> ReminderCandidate {
    ReminderCandidate {
        signal_id: signal.signal_id.clone(),
        tier: CareTier::Reminder,
        trigger: signal.trigger.clone(),
        severity: signal.severity.clone(),
        message: signal.message.clone(),
        evidence: signal.evidence.clone(),
        source: signal.source.clone(),
        created_at_ms: signal.created_at_ms,
    }
}

impl ReminderFeed for WeatherSignalFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        let consumed = self
            .consumed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let scan = self.ledger.scan()?;
        Ok(scan
            .records
            .iter()
            .map(|record| &record.body)
            .filter(|signal| !consumed.contains(&signal.signal_id))
            .map(weather_signal_to_candidate)
            .collect())
    }

    fn mark_consumed(&self, signal_id: &str) -> Result<(), String> {
        self.consumed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(signal_id.to_string());
        Ok(())
    }
}

/// 守护告警 → 关怀告警候选的消费桥 (`guard-alerts.jsonl`)。
pub struct GuardAlertFeed {
    ledger: GuardAlertLedger,
    consumed: Mutex<BTreeSet<String>>,
}

impl GuardAlertFeed {
    /// 绑定守护告警账 (数据目录落点)。
    pub fn under_data_dir(data_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            ledger: GuardAlertLedger::under_data_dir(data_dir),
            consumed: Mutex::new(BTreeSet::new()),
        }
    }

    /// 绑定显式账路径。
    pub fn at(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            ledger: GuardAlertLedger::at(path),
            consumed: Mutex::new(BTreeSet::new()),
        }
    }

    /// 告警账 (自述/测试同账)。
    pub fn ledger(&self) -> &GuardAlertLedger {
        &self.ledger
    }
}

/// 守护告警 → 候选 (守护例外档: 高优先, 破勿扰可配 + 标注 + 独立频率上限)。
pub fn guard_alert_to_candidate(alert: &GuardAlert) -> ReminderCandidate {
    ReminderCandidate {
        signal_id: alert.alert_id.clone(),
        tier: CareTier::GuardAlert,
        trigger: format!("{} [{}]", alert.condition, alert.annotation),
        severity: alert.severity.clone(),
        message: alert.message.clone(),
        evidence: alert.evidence.clone(),
        source: alert.source.clone(),
        created_at_ms: alert.created_at_ms,
    }
}

impl ReminderFeed for GuardAlertFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        let consumed = self
            .consumed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let scan = self.ledger.scan()?;
        Ok(scan
            .records
            .iter()
            .map(|record| &record.body)
            .filter(|alert| !consumed.contains(&alert.alert_id))
            .map(guard_alert_to_candidate)
            .collect())
    }

    fn mark_consumed(&self, signal_id: &str) -> Result<(), String> {
        self.consumed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(signal_id.to_string());
        Ok(())
    }
}

/// 多本信号账一并喂 (天气提醒 + 守护告警): 消费位按身份 fan-out 到各源。
pub struct CompositeReminderFeed {
    feeds: Vec<Box<dyn ReminderFeed>>,
}

impl CompositeReminderFeed {
    /// 组合若干信号源 (顺序 = 消费顺序)。
    pub fn new(feeds: Vec<Box<dyn ReminderFeed>>) -> Self {
        Self { feeds }
    }
}

impl ReminderFeed for CompositeReminderFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        let mut all = Vec::new();
        for feed in &self.feeds {
            all.extend(feed.pending()?);
        }
        Ok(all)
    }

    fn mark_consumed(&self, signal_id: &str) -> Result<(), String> {
        // 身份前缀不跨账冲突 (wxsig- / gwalert-); 逐源标一遍, 无源认领也不报错。
        for feed in &self.feeds {
            feed.mark_consumed(signal_id)?;
        }
        Ok(())
    }
}

/// 生物机制候选总闸 (`APEIRETH_ENABLE_BIO_CARE_CANDIDATES`): 默认关,
/// 注册即闸 (fail-closed), **只认 `"1"`** —— 缺省 / 空串 / 垃圾值一律 = 关。
pub const BIO_CARE_CANDIDATES_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_CARE_CANDIDATES";

/// 解析生物候选 env 原始值 (纯函数, 测试可注入): 只认 `"1"`, 其余任何取值 = 关。
#[must_use]
pub fn bio_care_candidates_enabled_from_env_value(raw: Option<&str>) -> bool {
    raw == Some("1")
}

/// 原始 env 值 → care 接线配置 (engine `with_care_config` 用; 默认关 fail-closed)。
#[must_use]
pub fn bio_care_config_from_env_value(raw: Option<&str>) -> CareConfig {
    CareConfig {
        bio_candidates_enabled: bio_care_candidates_enabled_from_env_value(raw),
    }
}

/// 从 env 现值解析生物候选开关 → care 接线配置 (默认关 fail-closed)。
#[must_use]
pub fn bio_care_config_from_env() -> CareConfig {
    bio_care_config_from_env_value(
        std::env::var(BIO_CARE_CANDIDATES_ENABLE_ENV)
            .ok()
            .as_deref(),
    )
}

/// 仪式候选输入: 状态 + 排程配置 (空 `states` 或空排程 = 无从产候选)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RitualInputs {
    /// 仪式运行时状态 (由宿主状态存储注入; 本面只读)。
    pub states: Vec<RitualState>,
    /// 仪式排程与全局旋钮 (缺省空排程 = 不主动)。
    pub config: RitualConfig,
}

/// 重聚候选输入: 上次可见时刻 + 可续线索 + 策略 (时间与线索都由宿主注入)。
#[derive(Debug, Clone, PartialEq)]
pub struct ReunionInputs {
    /// 上次可见时刻 (epoch 毫秒; 生产取 presence / 会话表)。
    pub last_seen_ms: u64,
    /// 可续接的关系线索 (生产取 memory replay; 空 = 无可续线索, 不产候选)。
    pub threads: Vec<ResumeThread>,
    /// 重聚策略 (问候强度会被分离类别上限压低, 不会抬高)。
    pub policy: ReunionPolicy,
}

/// 生物机制候选供给口 (W3.2 装配缝): 仪式状态 / 重聚输入由宿主注入。
///
/// 生产侧由 cli 宿主注入真实供给 (presence / 会话线索 / 仪式状态存储, 见
/// `crates/adapters/cli/src/care_heartbeat.rs`); 装配缺省仍用
/// [`NoBioCareInputs`] (恒空供给, 测试/无宿主场景)。取不到数据必须返回空 /
/// `None` —— 不编造「上次可见」「可续线索」「仪式状态」。
pub trait BioCareInputs: Send {
    /// 仪式状态 + 排程; 缺省空 = 无仪式候选。
    fn ritual_inputs(&self) -> RitualInputs {
        RitualInputs::default()
    }

    /// 重聚输入; 缺省 `None` = 取不到 (不产候选, 不编造)。
    fn reunion_inputs(&self) -> Option<ReunionInputs> {
        None
    }
}

/// 装配缺省空供给: 仪式 / 重聚都不产候选 (无宿主注入供给时的诚实默认)。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoBioCareInputs;

impl BioCareInputs for NoBioCareInputs {}

/// 一轮生物候选 (仪式 → 重聚, 确定性顺序): **只产候选, 不裁决**。
///
/// - 开关关 (默认) 恒返回空 (候选源自身也拦截, 双保险);
/// - 仪式: [`RitualState`] + [`RitualConfig`] → [`ritual_candidates_for`];
/// - 重聚: `last_seen_ms` + 可续线索 + 策略 → [`plan_reunion`] →
///   [`reunion_candidate_for`]; 取不到 (`None`) / 分离为 `Brief` / 无线索 =
///   不产候选 (不编造)。
///
/// 返回的候选必须由调用方喂
/// [`CareEngine::poll_bio_candidate`](apeireth_organ::care::CareEngine::poll_bio_candidate)
/// —— 勿扰 / 专注 / 频率 / 预算 / 发起闸全在内核, 本函数**不绕过**任何一道,
/// 也不投递、不写台账。
#[must_use]
pub fn bio_care_candidates(
    care_config: &CareConfig,
    inputs: &dyn BioCareInputs,
    now_ms: i64,
) -> Vec<CareCandidate> {
    if !care_config.bio_candidates_enabled() {
        // 默认关: 连供给口都不读 (零副作用 / 零候选 / 既有 care 行为不变)。
        return Vec::new();
    }
    let now_u64 = u64::try_from(now_ms).unwrap_or(0);
    let ritual = inputs.ritual_inputs();
    let mut candidates = ritual_candidates_for(care_config, now_ms, &ritual.states, &ritual.config);
    if let Some(reunion) = inputs.reunion_inputs() {
        let plan = plan_reunion(
            reunion.last_seen_ms,
            now_u64,
            &reunion.threads,
            reunion.policy,
        );
        if let Some(candidate) = reunion_candidate_for(care_config, &plan) {
            candidates.push(candidate);
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_organ::attachment::GreetingStyle;
    use apeireth_organ::ritual::{Cadence, RitualKind, RitualSchedule, Window};

    /// 固定注入时刻 (2025-10-09T08:53:20Z); 测试永不读系统时钟。
    const NOW: i64 = 1_760_000_000_000;
    const NOW_U64: u64 = 1_760_000_000_000;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 24 * HOUR_MS;

    fn enabled() -> CareConfig {
        CareConfig {
            bio_candidates_enabled: true,
        }
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

    fn threads() -> Vec<ResumeThread> {
        vec![ResumeThread {
            id: "thread-1".to_string(),
            summary: "共同项目".to_string(),
            salience: 0.8,
            emotion_weight: 0.6,
            last_touched_ms: NOW_U64 - HOUR_MS,
        }]
    }

    fn reunion_inputs(gap_ms: u64) -> ReunionInputs {
        ReunionInputs {
            last_seen_ms: NOW_U64.saturating_sub(gap_ms),
            threads: threads(),
            policy: ReunionPolicy::default(),
        }
    }

    struct FakeInputs {
        ritual: RitualInputs,
        reunion: Option<ReunionInputs>,
    }

    impl FakeInputs {
        fn new(ritual: RitualInputs, reunion: Option<ReunionInputs>) -> Self {
            Self { ritual, reunion }
        }
    }

    impl BioCareInputs for FakeInputs {
        fn ritual_inputs(&self) -> RitualInputs {
            self.ritual.clone()
        }

        fn reunion_inputs(&self) -> Option<ReunionInputs> {
            self.reunion.clone()
        }
    }

    #[test]
    fn bio_switch_is_fail_closed_and_only_accepts_one() {
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
                !bio_care_config_from_env_value(raw).bio_candidates_enabled(),
                "配置面同口径: {raw:?}"
            );
        }
        assert!(bio_care_candidates_enabled_from_env_value(Some("1")));
        assert!(bio_care_config_from_env_value(Some("1")).bio_candidates_enabled());
        assert_eq!(
            BIO_CARE_CANDIDATES_ENABLE_ENV,
            "APEIRETH_ENABLE_BIO_CARE_CANDIDATES"
        );
        assert!(
            !CareConfig::default().bio_candidates_enabled(),
            "缺省配置 = 关"
        );
    }

    #[test]
    fn default_empty_supply_produces_no_candidates_even_with_the_switch_on() {
        let empty = NoBioCareInputs;
        assert_eq!(empty.ritual_inputs(), RitualInputs::default());
        assert_eq!(empty.reunion_inputs(), None);
        assert!(
            bio_care_candidates(&enabled(), &empty, NOW).is_empty(),
            "空供给 = 零候选 (生产缺省: 数据源未接)"
        );
    }

    #[test]
    fn switch_off_ignores_a_populated_supply() {
        let inputs = FakeInputs::new(due_ritual_inputs(), Some(reunion_inputs(3 * DAY_MS)));
        assert!(bio_care_candidates(&CareConfig::default(), &inputs, NOW).is_empty());
        assert_eq!(bio_care_candidates(&enabled(), &inputs, NOW).len(), 2);
    }

    #[test]
    fn a_due_ritual_and_a_non_brief_reunion_become_candidates_in_order() {
        let inputs = FakeInputs::new(due_ritual_inputs(), Some(reunion_inputs(3 * DAY_MS)));
        let candidates = bio_care_candidates(&enabled(), &inputs, NOW);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].tier, CareTier::Ritual);
        assert_eq!(candidates[0].rule_id, "ritual.morning_check_in.due");
        assert_eq!(candidates[1].tier, CareTier::Reunion);
        assert_eq!(candidates[1].rule_id, "reunion.days");
        assert!(
            candidates[1].trigger.contains("重聚候选"),
            "重聚候选可读描述: {}",
            candidates[1].trigger
        );
    }

    #[test]
    fn brief_separation_and_empty_threads_leave_no_reunion_candidate() {
        let brief = FakeInputs::new(RitualInputs::default(), Some(reunion_inputs(30 * 60_000)));
        assert!(
            bio_care_candidates(&enabled(), &brief, NOW).is_empty(),
            "Brief 不演戏"
        );

        let no_threads = FakeInputs::new(
            RitualInputs::default(),
            Some(ReunionInputs {
                threads: Vec::new(),
                ..reunion_inputs(3 * DAY_MS)
            }),
        );
        assert!(
            bio_care_candidates(&enabled(), &no_threads, NOW).is_empty(),
            "无可续线索不产候选"
        );

        let no_reunion = FakeInputs::new(RitualInputs::default(), None);
        assert!(bio_care_candidates(&enabled(), &no_reunion, NOW).is_empty());
    }

    #[test]
    fn reunion_style_is_never_escalated_beyond_the_separation_class() {
        // 短分离 (Hours) 即便 policy 要 WarmReturn, 也只按类别上限收敛。
        let inputs = FakeInputs::new(
            RitualInputs::default(),
            Some(ReunionInputs {
                policy: ReunionPolicy {
                    greeting_style: GreetingStyle::WarmReturn,
                    ..ReunionPolicy::default()
                },
                ..reunion_inputs(5 * HOUR_MS)
            }),
        );
        let candidates = bio_care_candidates(&enabled(), &inputs, NOW);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule_id, "reunion.hours");
    }
}
