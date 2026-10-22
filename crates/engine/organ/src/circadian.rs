//! circadian —— SG-23 昼夜节律 + 超日节律 + 睡眠压力内核 v0（生物机制族 W1 批；机制内核，未接线）。
//!
//! # 生物原型（灵感源，非保真模拟）
//!
//! - **昼夜节律**：视交叉上核的相位意象 → [`CircadianPhase`]（深夜 / 黎明 / 晨 / 昼 /
//!   黄昏 / 夜）与相位振幅 [`CircadianPhase::amplitude`]；节律边界由醒/睡时间配置决定。
//! - **超日节律（ultradian rhythm）**：约 90 分钟的专注-休息循环意象 →
//!   [`UltradianWindow`]（默认 25 分钟专注 / 5 分钟休息，可配置）。
//! - **睡眠压力（腺苷累积意象）**：醒着按负荷累积、休息清账 → [`SleepPressure`]；
//!   压力到档后给出整编紧迫度 [`SleepUrgency`]（低 / 正常 / 高）。
//!
//! # 工程边界（明确不做什么）
//!
//! - **时间全部注入**：`now_ms` / `session_start_ms` 由调用方给；本模块**不读系统时钟**、
//!   不 sleep、不做 IO、无 RNG，同一输入必得同一输出。
//! - **相位是比例窗口，不是光照模型**：醒窗按 25% / 50% / 12.5% / 12.5% 划为
//!   晨 / 昼 / 黄昏 / 夜，睡窗按 50% / 50% 划为深夜 / 黎明；这是可解释的工程简化，
//!   不建模褪黑素、体温、光照或个体差异。`tz_offset_minutes` 是固定偏移，不处理夏令时。
//! - **睡眠压力是线性饱和模型**：醒着 16 小时 @ `load = 1.0` 饱和到 1；休息 8 小时清账到 0；
//!   不做指数衰减、不做睡眠债/个体差异。
//! - **非有限输入折叠为 `0.0`**（见 [`SleepPressure::with_level`]），输出不携带 NaN。
//! - 相位窗口的 1 分钟粒度由 `now_ms` 向下取整得到；超日窗口是毫秒粒度。
//!
//! # 能力级别（四级口径）：IMPLEMENTED（已实现、未接线）
//!
//! 不是 PRODUCTION WIRED、不是 DEFAULT ENABLED、不是 HARDWARE VALIDATED：
//!
//! - **未接 runtime 调度**：没有心跳/回合循环按相位或压力触发行为；
//! - **无 `self_status` 节**：本模块既不产出也不消费自述状态；
//! - **未接 dreaming 调度**：[`SleepUrgency::High`] 尚未驱动整编/做梦批处理；
//! - **未接 care 调度**：相位尚未用于勿扰窗口或主动关怀节律；
//! - 无持久化、无工具面、无遥测、无日志副作用。
//!
//! 接线点（W2 候选，不在本批）：心跳注入 `now_ms` → [`CircadianState::phase_at`] /
//! [`CircadianState::window_at`]；回合开始注入负荷与时长 →
//! [`SleepPressure::accumulate`]；整编批处理完成后 → [`SleepPressure::relieve`]；
//! [`SleepPressure::consolidation_urgency`] → `self_status` 一节 + dreaming 闸。
//!
//! # W3.1 节律门（默认关，crate 内纯函数）
//!
//! [`proactive_gate`] 把相位 / 超日窗口 / 整编紧迫度合成为一个
//! [`ProactiveGate`]（`Allowed` / `QuietHours` / `RestWindow` /
//! `ConsolidationDue`），供调用方在主动接触前咨询。它是纯函数：不读时钟、
//! 不写状态、不调度整编；既有 API 语义与输出不变（本批只新增函数，未接线）。
//!
//! 接线点（W3.2 装配面）：心跳采集 [`CircadianState::phase_at`] /
//! [`CircadianState::window_at`] / [`SleepPressure::consolidation_urgency`] 后调用
//! [`proactive_gate`]；`QuietHours` / `RestWindow` 抑制主动消息，
//! `ConsolidationDue` 路由到整编批处理。**本批不接 runtime**。

use serde::{Deserialize, Serialize};

/// 一天的分钟数。
const MINUTES_PER_DAY: u16 = 24 * 60;

/// 把任意 `f64` 折叠到 `[0, 1]`：NaN → `0.0`，±∞ 由 `clamp` 收边。
fn sanitize_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// 昼夜节律配置（全部为工程参数，调用方负责给出）。
///
/// 用 [`Self::new`] 构造会做规整：小时对 24 取模、时区偏移夹到 `±14h`、
/// 超日时长下限 1ms；直接写字面量可绕过规整，但所有方法仍保持全函数语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircadianConfig {
    /// 起床小时（本地时间，`0..=23`）。
    pub wake_hour: u8,
    /// 入睡小时（本地时间，`0..=23`）。
    pub sleep_hour: u8,
    /// 本地时区相对 UTC 的偏移（分钟；`-840..=840`，即 `-14h..=+14h`）。
    pub tz_offset_minutes: i32,
    /// 超日节律的专注时长（毫秒，至少 1）。
    pub ultradian_focus_ms: u64,
    /// 超日节律的休息时长（毫秒，至少 1）。
    pub ultradian_rest_ms: u64,
}

impl CircadianConfig {
    /// 默认配置：07:00 起床 / 23:00 入睡 / UTC+8 / 25 分钟专注 / 5 分钟休息。
    pub const fn default_schedule() -> Self {
        Self {
            wake_hour: 7,
            sleep_hour: 23,
            tz_offset_minutes: 8 * 60,
            ultradian_focus_ms: 25 * 60 * 1000,
            ultradian_rest_ms: 5 * 60 * 1000,
        }
    }

    /// 构造并规整：小时 `% 24`、时区夹到 `±14h`、超日时长下限 1ms。
    pub fn new(
        wake_hour: u8,
        sleep_hour: u8,
        tz_offset_minutes: i32,
        ultradian_focus_ms: u64,
        ultradian_rest_ms: u64,
    ) -> Self {
        Self {
            wake_hour: wake_hour % 24,
            sleep_hour: sleep_hour % 24,
            tz_offset_minutes: tz_offset_minutes.clamp(-14 * 60, 14 * 60),
            ultradian_focus_ms: ultradian_focus_ms.max(1),
            ultradian_rest_ms: ultradian_rest_ms.max(1),
        }
    }

    /// 起床时刻的当日分钟数（`0..=1439`）。
    pub const fn wake_minute_of_day(self) -> u16 {
        (self.wake_hour % 24) as u16 * 60
    }

    /// 入睡时刻的当日分钟数（`0..=1439`）。
    pub const fn sleep_minute_of_day(self) -> u16 {
        (self.sleep_hour % 24) as u16 * 60
    }

    /// 醒窗时长（分钟）；`wake_hour == sleep_hour` 约定为 24 小时全醒（返回 1440）。
    pub const fn awake_window_minutes(self) -> u16 {
        let wake = self.wake_minute_of_day();
        let sleep = self.sleep_minute_of_day();
        let awake = (sleep + MINUTES_PER_DAY - wake) % MINUTES_PER_DAY;
        if awake == 0 {
            MINUTES_PER_DAY
        } else {
            awake
        }
    }

    /// 睡窗时长（分钟）；24 小时全醒时为 0。
    pub const fn sleep_window_minutes(self) -> u16 {
        MINUTES_PER_DAY - self.awake_window_minutes()
    }

    /// 一个完整超日周期的时长（毫秒，饱和加法）。
    pub const fn ultradian_cycle_ms(self) -> u64 {
        self.ultradian_focus_ms
            .saturating_add(self.ultradian_rest_ms)
    }
}

impl Default for CircadianConfig {
    fn default() -> Self {
        Self::default_schedule()
    }
}

/// 昼夜相位（一天中的 6 个功能段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircadianPhase {
    /// 深夜（睡窗前半段）。
    DeepNight,
    /// 黎明（睡窗后半段，接近起床）。
    Dawn,
    /// 晨（醒窗最初 25%）。
    Morning,
    /// 昼（醒窗 25%..75%）。
    Day,
    /// 黄昏（醒窗 75%..87.5%）。
    Evening,
    /// 夜（醒窗最后 12.5%，睡前）。
    Night,
}

impl CircadianPhase {
    /// 全部相位（从深夜到夜，按一天推进顺序）。
    pub const ALL: [Self; 6] = [
        Self::DeepNight,
        Self::Dawn,
        Self::Morning,
        Self::Day,
        Self::Evening,
        Self::Night,
    ];

    /// 中文名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::DeepNight => "深夜",
            Self::Dawn => "黎明",
            Self::Morning => "晨",
            Self::Day => "昼",
            Self::Evening => "黄昏",
            Self::Night => "夜",
        }
    }

    /// 是否处于睡窗相位（深夜 / 黎明）。
    pub const fn is_sleep(self) -> bool {
        matches!(self, Self::DeepNight | Self::Dawn)
    }

    /// 相位振幅（工程标度 `0..=1`，不是生理测量）：昼 1.0、晨 0.9、黄昏 0.65、
    /// 夜 0.3、黎明 0.35、深夜 0.05。
    pub const fn amplitude(self) -> f64 {
        match self {
            Self::DeepNight => 0.05,
            Self::Dawn => 0.35,
            Self::Morning => 0.90,
            Self::Day => 1.00,
            Self::Evening => 0.65,
            Self::Night => 0.30,
        }
    }
}

/// 昼夜节律状态机：持有配置，所有时刻由调用方注入。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircadianState {
    config: CircadianConfig,
}

impl CircadianState {
    /// 用配置构造（配置自身的规整规则适用）。
    pub const fn new(config: CircadianConfig) -> Self {
        Self { config }
    }

    /// 只读配置。
    pub const fn config(&self) -> CircadianConfig {
        self.config
    }

    /// 注入时刻对应的本地当日分钟数（`0..=1439`；负时间戳按欧几里得取模回绕）。
    pub fn local_minute_of_day(&self, now_ms: i64) -> u16 {
        let offset_ms = i64::from(self.config.tz_offset_minutes) * 60_000;
        let local_ms = now_ms.saturating_add(offset_ms);
        local_ms.div_euclid(60_000).rem_euclid(1440) as u16
    }

    /// 注入时刻对应的本地小时（`0..=23`）。
    pub fn local_hour(&self, now_ms: i64) -> u8 {
        (self.local_minute_of_day(now_ms) / 60) as u8
    }

    /// 注入时刻对应的昼夜相位。
    ///
    /// 醒窗按 25% / 50% / 12.5% / 12.5% 划为晨 / 昼 / 黄昏 / 夜；
    /// 睡窗按 50% / 50% 划为深夜 / 黎明。`wake_hour == sleep_hour` 时视为 24 小时全醒。
    pub fn phase_at(&self, now_ms: i64) -> CircadianPhase {
        let minute = u32::from(self.local_minute_of_day(now_ms));
        let wake = u32::from(self.config.wake_minute_of_day());
        let sleep = u32::from(self.config.sleep_minute_of_day());
        let awake = u32::from(self.config.awake_window_minutes());
        let asleep = u32::from(self.config.sleep_window_minutes());

        if asleep > 0 {
            let from_sleep =
                (minute + u32::from(MINUTES_PER_DAY) - sleep) % u32::from(MINUTES_PER_DAY);
            if from_sleep < asleep {
                // 睡窗前半段 = 深夜，后半段 = 黎明。
                return if from_sleep * 2 < asleep {
                    CircadianPhase::DeepNight
                } else {
                    CircadianPhase::Dawn
                };
            }
        }

        let from_wake = (minute + u32::from(MINUTES_PER_DAY) - wake) % u32::from(MINUTES_PER_DAY);
        if from_wake * 4 < awake {
            CircadianPhase::Morning
        } else if from_wake * 8 < awake * 6 {
            CircadianPhase::Day
        } else if from_wake * 8 < awake * 7 {
            CircadianPhase::Evening
        } else {
            CircadianPhase::Night
        }
    }

    /// 注入时刻的相位振幅（等价于 `phase_at(...).amplitude()`）。
    pub fn amplitude_at(&self, now_ms: i64) -> f64 {
        self.phase_at(now_ms).amplitude()
    }

    /// 超日节律窗口：以 `session_start_ms` 为周期起点，返回当前相位与剩余时长。
    ///
    /// `now_ms < session_start_ms` 时按 0 经过处理（返回完整专注窗）；
    /// `focus + rest` 溢出用饱和加法，不会 panic。
    pub fn window_at(&self, now_ms: i64, session_start_ms: i64) -> UltradianWindow {
        let elapsed_ms = now_ms.saturating_sub(session_start_ms).max(0) as u64;
        let focus = self.config.ultradian_focus_ms;
        let rest = self.config.ultradian_rest_ms;
        let cycle = focus.saturating_add(rest);
        if cycle == 0 {
            return UltradianWindow {
                phase: UltradianPhase::Focus,
                remaining_ms: 0,
            };
        }
        let position = elapsed_ms % cycle;
        if position < focus {
            UltradianWindow {
                phase: UltradianPhase::Focus,
                remaining_ms: focus - position,
            }
        } else {
            UltradianWindow {
                phase: UltradianPhase::Rest,
                remaining_ms: cycle - position,
            }
        }
    }
}

impl Default for CircadianState {
    fn default() -> Self {
        Self::new(CircadianConfig::default_schedule())
    }
}

/// 超日节律相位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UltradianPhase {
    /// 专注窗。
    Focus,
    /// 休息窗。
    Rest,
}

impl UltradianPhase {
    /// 中文名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Focus => "专注",
            Self::Rest => "休息",
        }
    }
}

/// 一次超日节律窗口查询结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UltradianWindow {
    /// 当前处于专注窗还是休息窗。
    pub phase: UltradianPhase,
    /// 当前窗口剩余时长（毫秒，`0..=窗口长度`）。
    pub remaining_ms: u64,
}

/// 整编紧迫度（睡眠压力的三档离散化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SleepUrgency {
    /// 低（压力 `< 0.35`）。
    Low,
    /// 正常（`0.35 <= 压力 < 0.70`）。
    Normal,
    /// 高（压力 `>= 0.70`）：该安排整编/休息了。
    High,
}

impl SleepUrgency {
    /// 中文名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "低",
            Self::Normal => "正常",
            Self::High => "高",
        }
    }
}

/// 主动接触闸（节律门）结果。
///
/// [`proactive_gate`] 的返回值；门只给出「现在适合主动接触吗」的确定性判断，
/// 不发送消息、不调度整编、不落盘，由调用方按结果决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveGate {
    /// 无节律阻碍：允许主动接触。
    Allowed,
    /// 睡眠窗勿扰（深夜 / 黎明）：抑制主动接触。
    QuietHours,
    /// 超日休息窗：抑制主动接触。
    RestWindow,
    /// 睡眠压力高：应安排整编；抑制主动接触。
    ConsolidationDue,
}

impl ProactiveGate {
    /// 中文名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allowed => "允许",
            Self::QuietHours => "勿扰",
            Self::RestWindow => "休息窗",
            Self::ConsolidationDue => "待整编",
        }
    }

    /// 是否允许主动接触（只有 [`Self::Allowed`] 为真）。
    pub const fn allows_proactive(self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// 是否应安排整编（只有 [`Self::ConsolidationDue`] 为真）。
    pub const fn should_consolidate(self) -> bool {
        matches!(self, Self::ConsolidationDue)
    }
}

/// 节律门：`(phase, window, urgency) → ProactiveGate`（纯函数）。
///
/// 规则按顺序求值，**首个命中生效**：
///
/// 1. 相位为深夜 / 黎明（[`CircadianPhase::is_sleep`]）→ [`ProactiveGate::QuietHours`]；
/// 2. 睡眠压力紧迫度为 [`SleepUrgency::High`] → [`ProactiveGate::ConsolidationDue`]；
/// 3. 超日窗口为休息（[`UltradianPhase::Rest`]）→ [`ProactiveGate::RestWindow`]；
/// 4. 其余 → [`ProactiveGate::Allowed`]。
///
/// 优先级是显式的：睡窗勿扰压过紧迫度与超日休息（睡眠窗内不主动接触）；
/// 高紧迫度压过超日休息（整编是后台动作，不与「休息窗不发主动消息」冲突）。
/// `window.remaining_ms` 被忽略——快照的相位是唯一依据；本函数不读时钟、
/// 无 IO、无 RNG，同一输入必得同一输出。
pub const fn proactive_gate(
    phase: CircadianPhase,
    window: UltradianWindow,
    urgency: SleepUrgency,
) -> ProactiveGate {
    if phase.is_sleep() {
        ProactiveGate::QuietHours
    } else if matches!(urgency, SleepUrgency::High) {
        ProactiveGate::ConsolidationDue
    } else if matches!(window.phase, UltradianPhase::Rest) {
        ProactiveGate::RestWindow
    } else {
        ProactiveGate::Allowed
    }
}

/// 睡满 16 小时 @ `load = 1.0` 时睡眠压力饱和到 1 的标定常量。
pub const SLEEP_PRESSURE_FULL_WAKE_MS: u64 = 16 * 60 * 60 * 1000;
/// 休息 8 小时把压力从 1 清到 0 的标定常量。
pub const SLEEP_PRESSURE_FULL_REST_MS: u64 = 8 * 60 * 60 * 1000;
/// 整编紧迫度进入 `Normal` 的压力阈值（含）。
pub const SLEEP_URGENCY_NORMAL_THRESHOLD: f64 = 0.35;
/// 整编紧迫度进入 `High` 的压力阈值（含）。
pub const SLEEP_URGENCY_HIGH_THRESHOLD: f64 = 0.70;

/// 睡眠压力（腺苷累积意象的线性饱和模型）。
///
/// - [`Self::accumulate`]：`level += load × elapsed / 16h`，饱和到 1；
/// - [`Self::relieve`]：`level -= rest / 8h`，饱和到 0；
/// - [`Self::consolidation_urgency`]：`< 0.35` 低、`[0.35, 0.70)` 正常、`>= 0.70` 高。
///
/// `level` 只能通过上述方法变化；读取与写入都净化，故 `level()` 恒在 `[0, 1]`。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SleepPressure {
    level: f64,
}

impl SleepPressure {
    /// 从 0 压力开始。
    pub const fn new() -> Self {
        Self { level: 0.0 }
    }

    /// 从指定压力开始（NaN → `0.0`，越界夹到 `[0, 1]`）。
    pub fn with_level(level: f64) -> Self {
        Self {
            level: sanitize_unit(level),
        }
    }

    /// 当前压力 `[0, 1]`。
    pub fn level(&self) -> f64 {
        sanitize_unit(self.level)
    }

    /// 醒着累积：`load` 是本次时段的负荷强度（`0..=1`，NaN → 0），
    /// `elapsed_ms` 是本次经过时长；结果饱和到 1。
    pub fn accumulate(&mut self, load: f64, elapsed_ms: u64) {
        let load = sanitize_unit(load);
        let delta = load * (elapsed_ms as f64) / (SLEEP_PRESSURE_FULL_WAKE_MS as f64);
        self.level = sanitize_unit(self.level() + delta);
    }

    /// 休息清账：`rest_ms` 是本次休息时长；结果饱和到 0。
    pub fn relieve(&mut self, rest_ms: u64) {
        let delta = (rest_ms as f64) / (SLEEP_PRESSURE_FULL_REST_MS as f64);
        self.level = sanitize_unit(self.level() - delta);
    }

    /// 整编紧迫度（三档，阈值见 [`SLEEP_URGENCY_NORMAL_THRESHOLD`] /
    /// [`SLEEP_URGENCY_HIGH_THRESHOLD`]）。
    pub fn consolidation_urgency(&self) -> SleepUrgency {
        let level = self.level();
        if level >= SLEEP_URGENCY_HIGH_THRESHOLD {
            SleepUrgency::High
        } else if level >= SLEEP_URGENCY_NORMAL_THRESHOLD {
            SleepUrgency::Normal
        } else {
            SleepUrgency::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: i32 = 8 * 60;

    const FOCUS_WINDOW: UltradianWindow = UltradianWindow {
        phase: UltradianPhase::Focus,
        remaining_ms: 1,
    };
    const REST_WINDOW: UltradianWindow = UltradianWindow {
        phase: UltradianPhase::Rest,
        remaining_ms: 1,
    };

    /// 默认配置下，本地 `hour:minute` 对应的 epoch 毫秒（UTC+8）。
    fn local_ms(hour: u8, minute: u8) -> i64 {
        (i64::from(hour) * 60 + i64::from(minute) - i64::from(TZ)) * 60_000
    }

    #[test]
    fn phase_boundaries_follow_default_schedule() {
        let state = CircadianState::default();
        let cases = [
            (0, 0, CircadianPhase::DeepNight),
            (2, 59, CircadianPhase::DeepNight),
            (3, 0, CircadianPhase::Dawn),
            (6, 59, CircadianPhase::Dawn),
            (7, 0, CircadianPhase::Morning),
            (10, 59, CircadianPhase::Morning),
            (11, 0, CircadianPhase::Day),
            (18, 59, CircadianPhase::Day),
            (19, 0, CircadianPhase::Evening),
            (20, 59, CircadianPhase::Evening),
            (21, 0, CircadianPhase::Night),
            (22, 59, CircadianPhase::Night),
            (23, 0, CircadianPhase::DeepNight),
            (23, 59, CircadianPhase::DeepNight),
        ];
        for (hour, minute, expected) in cases {
            assert_eq!(
                state.phase_at(local_ms(hour, minute)),
                expected,
                "{hour:02}:{minute:02} 相位错误"
            );
        }
        // 1 分钟粒度的边界：06:59:59.999 仍是黎明，07:00:00 是晨。
        assert_eq!(
            state.phase_at(local_ms(6, 59) + 59_999),
            CircadianPhase::Dawn
        );
        assert_eq!(state.phase_at(local_ms(7, 0)), CircadianPhase::Morning);
    }

    #[test]
    fn phase_wraps_across_midnight_with_custom_schedule() {
        // 22:00 起床 / 06:00 入睡：醒窗 22:00→06:00（8h），睡窗 06:00→22:00（16h）。
        let state = CircadianState::new(CircadianConfig::new(22, 6, TZ, 25 * 60_000, 5 * 60_000));
        let cases = [
            (22, 0, CircadianPhase::Morning),    // 醒窗起点
            (23, 59, CircadianPhase::Morning),   // 醒窗 25% 之前
            (0, 0, CircadianPhase::Day),         // 跨午夜仍在昼
            (3, 59, CircadianPhase::Day),        // 醒窗 75%
            (4, 0, CircadianPhase::Evening),     // 醒窗 75%..87.5%
            (5, 0, CircadianPhase::Night),       // 醒窗最后 12.5%
            (6, 0, CircadianPhase::DeepNight),   // 睡窗起点
            (13, 59, CircadianPhase::DeepNight), // 睡窗前半
            (14, 0, CircadianPhase::Dawn),       // 睡窗后半
            (21, 59, CircadianPhase::Dawn),      // 睡窗末尾
        ];
        for (hour, minute, expected) in cases {
            assert_eq!(
                state.phase_at(local_ms(hour, minute)),
                expected,
                "{hour:02}:{minute:02} 相位错误"
            );
        }
    }

    #[test]
    fn tz_offset_shifts_local_phase() {
        // epoch 0 = UTC 1970-01-01T00:00。
        let utc = CircadianState::new(CircadianConfig::new(7, 23, 0, 1_500_000, 300_000));
        assert_eq!(utc.local_minute_of_day(0), 0);
        assert_eq!(utc.phase_at(0), CircadianPhase::DeepNight);

        let beijing = CircadianState::new(CircadianConfig::new(7, 23, 480, 1_500_000, 300_000));
        assert_eq!(beijing.local_hour(0), 8);
        assert_eq!(beijing.phase_at(0), CircadianPhase::Morning);

        let west = CircadianState::new(CircadianConfig::new(7, 23, -300, 1_500_000, 300_000));
        assert_eq!(west.local_hour(0), 19);
        assert_eq!(west.phase_at(0), CircadianPhase::Evening);
    }

    #[test]
    fn negative_epoch_and_minute_wrap_are_total() {
        let state = CircadianState::new(CircadianConfig::new(7, 23, 0, 1_500_000, 300_000));
        // -1ms = 1969-12-31T23:59:59.999Z。
        assert_eq!(state.local_minute_of_day(-1), 1439);
        assert_eq!(state.local_hour(-1), 23);
        assert_eq!(state.phase_at(-1), CircadianPhase::DeepNight);
        // 跨日回绕：整 24h 后相位相同。
        let a = state.phase_at(local_ms(15, 30));
        let b = state.phase_at(local_ms(15, 30) + 24 * 60 * 60 * 1000);
        assert_eq!(a, b);
    }

    #[test]
    fn ultradian_window_advances_and_wraps() {
        let state = CircadianState::new(CircadianConfig::new(7, 23, 0, 25 * 60_000, 5 * 60_000));
        let start = 1_000_000;
        assert_eq!(
            state.window_at(start, start),
            UltradianWindow {
                phase: UltradianPhase::Focus,
                remaining_ms: 25 * 60_000,
            }
        );
        assert_eq!(
            state.window_at(start + 25 * 60_000 - 1, start),
            UltradianWindow {
                phase: UltradianPhase::Focus,
                remaining_ms: 1,
            }
        );
        assert_eq!(
            state.window_at(start + 25 * 60_000, start),
            UltradianWindow {
                phase: UltradianPhase::Rest,
                remaining_ms: 5 * 60_000,
            }
        );
        assert_eq!(
            state.window_at(start + 30 * 60_000 - 1, start),
            UltradianWindow {
                phase: UltradianPhase::Rest,
                remaining_ms: 1,
            }
        );
        // 一个完整周期后回绕到专注窗。
        assert_eq!(
            state.window_at(start + 30 * 60_000, start),
            UltradianWindow {
                phase: UltradianPhase::Focus,
                remaining_ms: 25 * 60_000,
            }
        );
    }

    #[test]
    fn ultradian_negative_elapsed_is_total() {
        let state = CircadianState::default();
        let start = 5_000_000;
        assert_eq!(
            state.window_at(start - 60_000, start),
            UltradianWindow {
                phase: UltradianPhase::Focus,
                remaining_ms: 25 * 60_000,
            }
        );
    }

    #[test]
    fn sleep_pressure_accumulates_monotonically_and_saturates() {
        let mut pressure = SleepPressure::new();
        assert_eq!(pressure.level(), 0.0);
        let mut previous = pressure.level();
        for _ in 0..8 {
            pressure.accumulate(0.8, 30 * 60 * 1000);
            assert!(pressure.level() > previous, "累积必须单调上升");
            previous = pressure.level();
        }
        // 16h @ load = 1.0 饱和到 1.0。
        pressure.accumulate(1.0, SLEEP_PRESSURE_FULL_WAKE_MS);
        assert_eq!(pressure.level(), 1.0);
        pressure.accumulate(1.0, u64::MAX);
        assert_eq!(pressure.level(), 1.0);
    }

    #[test]
    fn sleep_pressure_relieves_and_floors_at_zero() {
        let mut pressure = SleepPressure::with_level(0.5);
        pressure.relieve(2 * 60 * 60 * 1000);
        assert!((pressure.level() - 0.25).abs() < 1e-12);
        pressure.relieve(4 * 60 * 60 * 1000);
        assert_eq!(pressure.level(), 0.0);
        pressure.relieve(u64::MAX);
        assert_eq!(pressure.level(), 0.0);
    }

    #[test]
    fn sleep_urgency_uses_documented_thresholds() {
        let cases = [
            (0.0, SleepUrgency::Low),
            (0.349_9, SleepUrgency::Low),
            (SLEEP_URGENCY_NORMAL_THRESHOLD, SleepUrgency::Normal),
            (0.699_9, SleepUrgency::Normal),
            (SLEEP_URGENCY_HIGH_THRESHOLD, SleepUrgency::High),
            (1.0, SleepUrgency::High),
        ];
        for (level, expected) in cases {
            assert_eq!(
                SleepPressure::with_level(level).consolidation_urgency(),
                expected,
                "level={level}"
            );
        }
    }

    #[test]
    fn non_finite_pressure_inputs_are_sanitized() {
        assert_eq!(SleepPressure::with_level(f64::NAN).level(), 0.0);
        assert_eq!(SleepPressure::with_level(f64::INFINITY).level(), 1.0);
        assert_eq!(SleepPressure::with_level(f64::NEG_INFINITY).level(), 0.0);
        let mut pressure = SleepPressure::with_level(0.4);
        pressure.accumulate(f64::NAN, 60_000);
        assert_eq!(pressure.level(), 0.4);
        assert!(pressure.level().is_finite());
    }

    #[test]
    fn config_normalizes_and_twenty_four_hour_wake_has_no_sleep_phase() {
        let config = CircadianConfig::new(24, 48, -2_000, 0, 0);
        assert_eq!(config.wake_hour, 0);
        assert_eq!(config.sleep_hour, 0);
        assert_eq!(config.tz_offset_minutes, -840);
        assert_eq!(config.ultradian_focus_ms, 1);
        assert_eq!(config.ultradian_rest_ms, 1);
        assert_eq!(config.ultradian_cycle_ms(), 2);
        assert_eq!(config.awake_window_minutes(), 1440);
        assert_eq!(config.sleep_window_minutes(), 0);

        let state = CircadianState::new(config);
        for hour in 0..24 {
            let phase = state.phase_at(local_ms(hour, 0));
            assert!(
                !phase.is_sleep(),
                "24 小时全醒时 {hour}:00 不应出现睡眠相位"
            );
        }
    }

    #[test]
    fn amplitude_follows_phase() {
        let state = CircadianState::default();
        assert_eq!(
            state.amplitude_at(local_ms(3, 0)),
            CircadianPhase::Dawn.amplitude()
        );
        assert_eq!(
            state.amplitude_at(local_ms(12, 0)),
            CircadianPhase::Day.amplitude()
        );
        // 昼 > 黄昏 > 黎明 > 夜 > 深夜。
        let day = state.amplitude_at(local_ms(12, 0));
        let evening = state.amplitude_at(local_ms(20, 0));
        let dawn = state.amplitude_at(local_ms(4, 0));
        let night = state.amplitude_at(local_ms(22, 0));
        let deep_night = state.amplitude_at(local_ms(1, 0));
        assert!(day > evening);
        assert!(evening > dawn);
        assert!(dawn > night);
        assert!(night > deep_night);
    }

    #[test]
    fn outputs_are_deterministic_and_serde_round_trip() {
        let state = CircadianState::default();
        let now = local_ms(9, 30);
        assert_eq!(state.phase_at(now), state.phase_at(now));
        assert_eq!(state.window_at(now, 0), state.window_at(now, 0));
        assert_eq!(
            SleepPressure::with_level(0.42),
            SleepPressure::with_level(0.42)
        );

        let config = state.config();
        let json = serde_json::to_string(&config).expect("配置应可序列化");
        let back: CircadianConfig = serde_json::from_str(&json).expect("配置应可反序列化");
        assert_eq!(config, back);
    }

    #[test]
    fn proactive_gate_quiet_hours_for_sleep_phases() {
        for phase in [CircadianPhase::DeepNight, CircadianPhase::Dawn] {
            for urgency in [SleepUrgency::Low, SleepUrgency::Normal, SleepUrgency::High] {
                for window in [FOCUS_WINDOW, REST_WINDOW] {
                    let gate = proactive_gate(phase, window, urgency);
                    assert_eq!(
                        gate,
                        ProactiveGate::QuietHours,
                        "{phase:?}/{window:?}/{urgency:?} 应为勿扰"
                    );
                    assert!(!gate.allows_proactive());
                    assert!(!gate.should_consolidate());
                    assert_eq!(gate.label(), "勿扰");
                }
            }
        }
    }

    #[test]
    fn proactive_gate_high_urgency_is_consolidation_due_outside_quiet_hours() {
        for phase in [
            CircadianPhase::Morning,
            CircadianPhase::Day,
            CircadianPhase::Evening,
            CircadianPhase::Night,
        ] {
            for window in [FOCUS_WINDOW, REST_WINDOW] {
                let gate = proactive_gate(phase, window, SleepUrgency::High);
                assert_eq!(
                    gate,
                    ProactiveGate::ConsolidationDue,
                    "{phase:?}/{window:?} 高紧迫度应待整编"
                );
                assert!(gate.should_consolidate());
                assert!(!gate.allows_proactive());
            }
        }
    }

    #[test]
    fn proactive_gate_rest_window_blocks_low_and_normal_urgency() {
        for phase in [
            CircadianPhase::Morning,
            CircadianPhase::Day,
            CircadianPhase::Evening,
            CircadianPhase::Night,
        ] {
            for urgency in [SleepUrgency::Low, SleepUrgency::Normal] {
                let gate = proactive_gate(phase, REST_WINDOW, urgency);
                assert_eq!(
                    gate,
                    ProactiveGate::RestWindow,
                    "{phase:?}/{urgency:?} 休息窗应抑制主动接触"
                );
                assert!(!gate.allows_proactive());
                assert!(!gate.should_consolidate());
            }
        }
    }

    #[test]
    fn proactive_gate_allows_focus_window_without_high_urgency() {
        for phase in [
            CircadianPhase::Morning,
            CircadianPhase::Day,
            CircadianPhase::Evening,
            CircadianPhase::Night,
        ] {
            for urgency in [SleepUrgency::Low, SleepUrgency::Normal] {
                let gate = proactive_gate(phase, FOCUS_WINDOW, urgency);
                assert_eq!(
                    gate,
                    ProactiveGate::Allowed,
                    "{phase:?}/{urgency:?} 专注窗应允许主动接触"
                );
                assert!(gate.allows_proactive());
                assert!(!gate.should_consolidate());
                assert_eq!(gate.label(), "允许");
            }
        }
    }

    #[test]
    fn proactive_gate_precedence_is_explicit() {
        // 睡窗勿扰 > 高紧迫度 > 超日休息 > 允许。
        assert_eq!(
            proactive_gate(CircadianPhase::Dawn, REST_WINDOW, SleepUrgency::High),
            ProactiveGate::QuietHours
        );
        assert_eq!(
            proactive_gate(CircadianPhase::Night, REST_WINDOW, SleepUrgency::High),
            ProactiveGate::ConsolidationDue
        );
        assert_eq!(
            proactive_gate(CircadianPhase::Night, REST_WINDOW, SleepUrgency::Normal),
            ProactiveGate::RestWindow
        );
        assert_eq!(
            proactive_gate(CircadianPhase::Day, FOCUS_WINDOW, SleepUrgency::Low),
            ProactiveGate::Allowed
        );
    }

    #[test]
    fn proactive_gate_is_deterministic_and_ignores_remaining_ms() {
        let rest_zero = UltradianWindow {
            phase: UltradianPhase::Rest,
            remaining_ms: 0,
        };
        let rest_long = UltradianWindow {
            phase: UltradianPhase::Rest,
            remaining_ms: u64::MAX,
        };
        assert_eq!(
            proactive_gate(CircadianPhase::Night, rest_zero, SleepUrgency::Normal),
            proactive_gate(CircadianPhase::Night, rest_long, SleepUrgency::Normal)
        );
        assert_eq!(
            proactive_gate(CircadianPhase::Night, rest_zero, SleepUrgency::Normal),
            ProactiveGate::RestWindow
        );

        let focus_short = UltradianWindow {
            phase: UltradianPhase::Focus,
            remaining_ms: 1,
        };
        let focus_long = UltradianWindow {
            phase: UltradianPhase::Focus,
            remaining_ms: u64::MAX,
        };
        assert_eq!(
            proactive_gate(CircadianPhase::Day, focus_short, SleepUrgency::Low),
            proactive_gate(CircadianPhase::Day, focus_long, SleepUrgency::Low)
        );
        assert_eq!(
            proactive_gate(CircadianPhase::Day, focus_short, SleepUrgency::Low),
            ProactiveGate::Allowed
        );
    }
}
