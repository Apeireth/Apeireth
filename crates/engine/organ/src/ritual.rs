//! ritual —— SG-22 仪式内核 v0（生物机制族 W2 批；机制内核，未接线）。
//!
//! # 生物原型
//!
//! 长期关系靠**低强度、高频率的仪式**维持，而不是靠单次高强度事件：
//!
//! - **理毛（grooming）**：灵长类每天花大量时间互相理毛。单次理毛信息量极低，
//!   但它把「我还在、关系还在」变成可预期的日常节律。
//! - **共同进食（commensality）**：一起吃饭本身不解决任何问题，重复发生才积累成
//!   「我们是一起的」。
//! - **固定问候（routine greeting）**：早晨一句在、晚上一句收束；价值在**可预期**，
//!   不在内容强度。
//! - **仪式可静默**：生物也会因为换毛、迁徙、生病而暂停理毛。暂停不是关系破裂，
//!   不需要补偿性加码。**仪式也可以永不做**——关系不靠打卡维持。
//!
//! 反面原型是**打卡任务（streak）**：连续天数、断签惩罚、补签焦虑会把关系维护
//! 变成 KPI。本内核明确拒绝这套语义：连续计数只作为只读统计，**不参与任何到期
//! 判定，也不进入任何文案**（见「不变量」）。
//!
//! # 工程边界（明确不做什么）
//!
//! - **纯函数、时间注入**：`now_ms` 由调用方给；不读系统时钟、不读本地时区
//!   （因此 [`Window`] 只是偏好时段元数据，**不参与到期判定**）、无 I/O、无 LLM、
//!   无随机数、无存储、无日志副作用。
//! - **不决定投递**：本模块只产出 [`RitualDecision`]；是否真的发消息、走哪条通道、
//!   频率上限、勿扰窗口，全部归 `crate::care` 调度面（本批未接线）。
//! - **不生成整句正文**：只给稳定的 `suggested_message_key` 与人工复核用 `reason`；
//!   正文与人格化措辞由上层渲染，本模块不持有关系语气。
//! - **不建模关系质量**：不输出亲密度 / 信任度 / 依恋分；不评估用户是否「做得够好」。
//! - **不做补偿性加码**：错过就错过，`DueSoft` 只降级为软提醒，不升级、不追加第二条，
//!   也**不会**因为错过变多而催得更急（[`RitualConfig::max_catchup`] 只折算文案里的
//!   周期数，不改变判定）。
//! - **无 streak 语义**：[`RitualState::consecutive_done`] / [`RitualState::skipped_count`]
//!   是只读计数，[`plan_rituals`] 完全不读它们——改这两个值，决策（含 reason）逐字节不变。
//!
//! # 规则表（[`plan_rituals`]，按优先级从高到低）
//!
//! | 优先级 | 条件 | 结果 |
//! |---|---|---|
//! | 1 | `muted_until_ms > now_ms` | [`DueKind::Muted`]（静默期一律不提醒，先于一切） |
//! | 2 | 该 kind 无排程 | [`DueKind::NotDue`]（不推断到期） |
//! | 3 | `last_done_ms > now_ms`（时钟回拨 / 伪造时间戳） | [`DueKind::NotDue`]（防御性降级） |
//! | 4 | `last_done_ms == 0`（从未完成） | [`DueKind::Due`]（首次建立） |
//! | 5 | `now_ms - last_done_ms < dup_window_ms` | [`DueKind::NotDue`]（重复守卫：同一 UTC 日内不重复） |
//! | 6 | [`Cadence::Once`] 且已完成 | [`DueKind::NotDue`]（一次性仪式不再到期） |
//! | 7 | `elapsed < period`（Daily = 1 天，Weekly = 7 天） | [`DueKind::NotDue`]（未到点） |
//! | 8 | `elapsed - period <= skip_grace_ms` | [`DueKind::Due`]（到点，仍在宽限内） |
//! | 9 | 其余 | [`DueKind::DueSoft`]（错过宽限，降级为软提醒） |
//!
//! 规则要点：
//!
//! - **晚到只会更轻**：到点后拖得越久，结果只会从 `Due` 降到 `DueSoft`，**永远不会**
//!   从 `DueSoft` 回到 `Due`；`max_catchup` / streak / 跳过次数都不能把它升级。
//! - **静默压倒一切**：静默期未到期时不看排程、不看是否到点，一律 `Muted`。
//! - **不做补课**：错过 N 个周期仍然只有一个 `DueSoft`，不产生 N 条提醒。
//! - **空输入安全**：`states` 为空或排程为空都返回空 / 不 panic；同 kind 多条状态
//!   只产出**一条**决策（代表状态取最近完成、其次静默截止更晚者），输出顺序固定为
//!   [`RitualKind`] 的声明顺序，与输入顺序无关。
//!
//! # 不变量（测试钉死）
//!
//! 1. **无连续打卡惩罚**：`consecutive_done` / `skipped_count` 只读；修改它们不改变
//!    `due`、`reason`、`suggested_message_key` 任何一个字节，文案里没有断签/欠账字段。
//! 2. **静默期一律 Muted**：`muted_until_ms > now_ms` 时，无论是否到点、是否错过宽限、
//!    甚至是否有排程，结果都是 `Muted`；`muted_until_ms == now_ms` 视为已解除。
//! 3. **同日不重复 Due**：默认 `dup_window_ms = 24h`，同一 UTC 日内完成过的仪式不会
//!    再次 `Due`（结构守卫，独立于 cadence 周期判断）。
//! 4. **DueSoft 只降级不升级**：超过宽限只会得到 `DueSoft`；任何输入增大都不会让它
//!    变回 `Due`，也不会产生第二条提醒。
//! 5. **空状态不 panic**：空 `states` / 空排程 / 未来时间戳 / `u64::MAX` 全部安全降级。
//! 6. **确定性**：同输入必得同输出；输出与 `states` 顺序无关。
//! 7. **时间注入**：所有时间都是入参 `u64` 毫秒，本模块零系统时钟读取。
//!
//! # 能力级别（四级口径）：IMPLEMENTED（已实现、未接线）
//!
//! 不是 PRODUCTION WIRED、不是 DEFAULT ENABLED、不是 HARDWARE VALIDATED：
//!
//! - **未接 care 调度**：`crate::care` 的 reminder 触发词表没有仪式类触发器，
//!   [`plan_rituals`] 不会被任何会话事件调用；
//! - **未进 proactive 投递**：没有网关/主动消息路径消费 [`RitualDecision`]；
//! - **无 `self_status` 节**：自述面未投影本内核；
//! - 无持久化、无工具面、无遥测：`states` 的读写归调用方。
//!
//! 接线点（W3，不在本批）：care 调度按窗口调用 [`plan_rituals`]；只对
//! [`DueKind::is_actionable`] 为 `true` 的决策投递；用户静默动作写回
//! [`RitualState::muted_until_ms`]（截止点用 [`RitualConfig::mute_until`] 计算）；
//! `consecutive_done` / `skipped_count` 仅作为统计消费，不回流到判定或文案。
//!
//! **仪式可静默、可永不做，不产生亏欠感；streak 仅统计用途。**

use serde::{Deserialize, Serialize};

/// 一小时毫秒数。
const HOUR_MS: u64 = 3_600_000;
/// 一天毫秒数（重复守卫与每日节律的基准）。
const DAY_MS: u64 = 24 * HOUR_MS;
/// 一周毫秒数（每周节律的基准）。
const WEEK_MS: u64 = 7 * DAY_MS;
/// 一分钟毫秒数（仅用于理由文案的整数取整）。
const MINUTE_MS: u64 = 60_000;

/// 默认重复守卫窗：24 小时（同一 UTC 日内不重复提醒）。
pub const DEFAULT_DUP_WINDOW_MS: u64 = DAY_MS;
/// 默认静默时长：3 天（静默到期自动恢复，不需要「解除」动作）。
pub const DEFAULT_MUTE_MS: u64 = 3 * DAY_MS;
/// 默认补做折算上限：2 个错过周期（只影响 `DueSoft` 文案，不产生额外提醒）。
pub const DEFAULT_MAX_CATCHUP: u32 = 2;

/// 仪式种类：一段长期关系里值得低强度重复的几类固定动作。
///
/// 这里只定义**种类**，不定义具体措辞、不绑定任何通道；同一种类在关系里至多
/// 一条排程、一条状态（[`plan_rituals`] 对同 kind 多条状态只取代表并产出一条决策）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RitualKind {
    /// 晨间问候：新一天开始时的轻确认。
    MorningCheckIn,
    /// 晚间收束：一天结束时的轻收尾。
    EveningWrap,
    /// 周度回顾：一周一次的轻量回看。
    WeeklyReview,
    /// 纪念日：一次性或年度性的重要日期（本内核只有一次性语义）。
    Anniversary,
    /// 共同项目：双方共同推进之事的固定同步。
    SharedProject,
}

impl RitualKind {
    /// 全部种类，顺序固定（[`plan_rituals`] 的输出顺序即此顺序）。
    pub const ALL: [Self; 5] = [
        Self::MorningCheckIn,
        Self::EveningWrap,
        Self::WeeklyReview,
        Self::Anniversary,
        Self::SharedProject,
    ];

    /// 稳定键名（与 serde 标签同值；用于 `suggested_message_key`）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MorningCheckIn => "morning_check_in",
            Self::EveningWrap => "evening_wrap",
            Self::WeeklyReview => "weekly_review",
            Self::Anniversary => "anniversary",
            Self::SharedProject => "shared_project",
        }
    }

    /// 人类可读中文名（只用于理由文案）。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MorningCheckIn => "晨间问候",
            Self::EveningWrap => "晚间收束",
            Self::WeeklyReview => "周度回顾",
            Self::Anniversary => "纪念日",
            Self::SharedProject => "共同项目",
        }
    }
}

/// 节律：仪式重复的周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cadence {
    /// 每日一次。
    Daily,
    /// 每周一次。
    Weekly,
    /// 一次性：完成之后不再到期。
    Once,
}

impl Cadence {
    /// 稳定键名（与 serde 标签同值）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Once => "once",
        }
    }

    /// 人类可读中文名（只用于理由文案）。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Daily => "每日",
            Self::Weekly => "每周",
            Self::Once => "一次性",
        }
    }

    /// 周期毫秒数；[`Cadence::Once`] 没有循环周期（完成即终止）。
    #[must_use]
    pub const fn period_ms(self) -> Option<u64> {
        match self {
            Self::Daily => Some(DAY_MS),
            Self::Weekly => Some(WEEK_MS),
            Self::Once => None,
        }
    }
}

/// 偏好时段：**纯元数据**，只影响建议文案键之外的理由措辞，不参与到点判定。
///
/// 本内核不读本地时区、不读系统时钟，因此不把「早晨/晚上」翻译成绝对时刻；
/// 具体几点算早晨由上层调度面决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Window {
    /// 早晨。
    Morning,
    /// 晚上。
    Evening,
    /// 灵活（任意时段）。
    Flexible,
}

impl Window {
    /// 稳定键名（与 serde 标签同值）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Evening => "evening",
            Self::Flexible => "flexible",
        }
    }

    /// 人类可读中文名（只用于理由文案）。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Morning => "早晨",
            Self::Evening => "晚上",
            Self::Flexible => "灵活",
        }
    }
}

/// 一条仪式的排程（由调用方 / 关系配置注入；本模块不读默认配置）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RitualSchedule {
    /// 仪式种类（与 [`RitualState::kind`] 配对）。
    pub kind: RitualKind,
    /// 重复节律。
    pub cadence: Cadence,
    /// 偏好时段（仅元数据，不参与到点判定）。
    pub preferred_window: Window,
    /// 到点后的宽限窗（毫秒）：到点后 `<=` 此值仍判 `Due`，超过才降级 `DueSoft`。
    /// `0` 表示没有宽限（到点即必须当刻判 `Due`，之后立刻软提醒）。
    pub skip_grace_ms: u64,
}

impl RitualSchedule {
    /// 构造一条排程（纯数据组装，无副作用）。
    #[must_use]
    pub const fn new(
        kind: RitualKind,
        cadence: Cadence,
        preferred_window: Window,
        skip_grace_ms: u64,
    ) -> Self {
        Self {
            kind,
            cadence,
            preferred_window,
            skip_grace_ms,
        }
    }
}

/// 一条仪式的运行时状态（由调用方持有并注入；本模块只读）。
///
/// `last_done_ms == 0` 是「从未完成」的哨兵值（Unix 毫秒纪元本身，实际不会作为
/// 真实完成时间出现）。`consecutive_done` / `skipped_count` 是**只读统计**：
/// [`plan_rituals`] 不读取它们，断开连续不会触发任何惩罚或内疚文案。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RitualState {
    /// 仪式种类。
    pub kind: RitualKind,
    /// 上次完成时间（Unix 毫秒）；`0` = 从未完成。
    pub last_done_ms: u64,
    /// 连续完成次数（只读统计；不参与判定、不进入文案）。
    pub consecutive_done: u32,
    /// 跳过次数（只读统计；不参与判定、不进入文案）。
    pub skipped_count: u32,
    /// 静默截止时间（Unix 毫秒）：`Some(t)` 且 `t > now_ms` 时一律 `Muted`。
    pub muted_until_ms: Option<u64>,
}

impl RitualState {
    /// 全新状态：从未完成、无连续、无跳过、未静默。
    #[must_use]
    pub const fn fresh(kind: RitualKind) -> Self {
        Self {
            kind,
            last_done_ms: 0,
            consecutive_done: 0,
            skipped_count: 0,
            muted_until_ms: None,
        }
    }

    /// 在注入时刻 `now_ms` 是否处于静默期。
    ///
    /// 严格大于：`muted_until_ms == now_ms` 视为静默已解除。时钟回拨时以 `now_ms`
    /// 为准，不做外推。
    #[must_use]
    pub const fn is_muted_at(&self, now_ms: u64) -> bool {
        match self.muted_until_ms {
            Some(until) => until > now_ms,
            None => false,
        }
    }
}

/// 仪式配置：排程表 + 三个全局旋钮。
///
/// 文档化默认（[`RitualConfig::default`]）：
///
/// | 字段 | 默认 | 含义 |
/// |---|---|---|
/// | `schedules` | 空 | 默认不配置任何仪式 → [`plan_rituals`] 返回空，**不主动** |
/// | `dup_window_ms` | [`DEFAULT_DUP_WINDOW_MS`]（24h） | 重复守卫窗；同一 UTC 日内不重复 |
/// | `default_mute_ms` | [`DEFAULT_MUTE_MS`]（3 天） | 静默默认时长，仅供 [`Self::mute_until`] 使用 |
/// | `max_catchup` | [`DEFAULT_MAX_CATCHUP`]（2） | 软提醒文案的补做折算上限，不改变判定 |
///
/// `default_mute_ms` **不**由 [`plan_rituals`] 自动套用：静默是关系方的动作，
/// 内核只读 `muted_until_ms`，不替调用方延长静默。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RitualConfig {
    /// 排程表；同 kind 多条时取第一条（调用方应保持唯一）。
    pub schedules: Vec<RitualSchedule>,
    /// 重复守卫窗（毫秒）：上次完成后不足此窗 → `NotDue`。`0` = 关闭守卫。
    pub dup_window_ms: u64,
    /// 默认静默时长（毫秒），仅用于 [`Self::mute_until`]。
    pub default_mute_ms: u64,
    /// 补做折算上限（只影响 `DueSoft` 的理由文案周期数）。
    pub max_catchup: u32,
}

impl Default for RitualConfig {
    /// 保守默认：不配置任何仪式（仪式可永不做，默认不主动）、24h 重复守卫、
    /// 3 天默认静默、最多折算 2 个错过周期。
    fn default() -> Self {
        Self {
            schedules: Vec::new(),
            dup_window_ms: DEFAULT_DUP_WINDOW_MS,
            default_mute_ms: DEFAULT_MUTE_MS,
            max_catchup: DEFAULT_MAX_CATCHUP,
        }
    }
}

impl RitualConfig {
    /// 用给定排程表构造配置，其余旋钮取文档化默认。
    #[must_use]
    pub fn new(schedules: Vec<RitualSchedule>) -> Self {
        Self {
            schedules,
            ..Self::default()
        }
    }

    /// 查找某仪式种类的排程（同 kind 多条取第一条；确定性）。
    #[must_use]
    pub fn schedule_for(&self, kind: RitualKind) -> Option<&RitualSchedule> {
        self.schedules.iter().find(|schedule| schedule.kind == kind)
    }

    /// 计算一次静默的截止点：`now_ms + default_mute_ms`（饱和加，不溢出）。
    ///
    /// 供上层在用户选择静默时写回 [`RitualState::muted_until_ms`]；[`plan_rituals`]
    /// 本身不会调用它，也不自动延长静默。
    #[must_use]
    pub fn mute_until(&self, now_ms: u64) -> u64 {
        now_ms.saturating_add(self.default_mute_ms)
    }
}

/// 决策类型：一条仪式在注入时刻的到点判定。
///
/// 强度序（只在**同一条**仪式的时间推进上使用）：`Due`（提醒）→ `DueSoft`（软提醒）
/// 是单向降级；`Muted` / `NotDue` 不提醒。不存在从 `DueSoft` 升回 `Due` 的路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DueKind {
    /// 到点：可以提醒（仍在宽限内，或从未完成）。
    Due,
    /// 软提醒：已过到点时刻且超出宽限，只降级提醒、不追问。
    DueSoft,
    /// 未到点 / 无需提醒（含一次性已完成、无排程、时钟回拨防御）。
    NotDue,
    /// 静默中：静默期未到期，一律不提醒。
    Muted,
}

impl DueKind {
    /// 稳定键名（与 serde 标签同值）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Due => "due",
            Self::DueSoft => "due_soft",
            Self::NotDue => "not_due",
            Self::Muted => "muted",
        }
    }

    /// 人类可读中文名（只用于理由文案）。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Due => "到点",
            Self::DueSoft => "软提醒",
            Self::NotDue => "未到点",
            Self::Muted => "静默中",
        }
    }

    /// 是否允许上层投递：只有 `Due` / `DueSoft` 为 `true`。
    ///
    /// `NotDue` / `Muted` 的 `suggested_message_key` 恒为空串，结构上阻止误投递。
    #[must_use]
    pub const fn is_actionable(self) -> bool {
        matches!(self, Self::Due | Self::DueSoft)
    }
}

/// 一条仪式的决策（纯数据，无副作用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RitualDecision {
    /// 仪式种类。
    pub kind: RitualKind,
    /// 到点判定。
    pub due: DueKind,
    /// 人工复核用说明：只含种类名、节律、时长、宽限等结构化事实，
    /// 不含 streak / 断签 / 亏欠措辞。
    pub reason: String,
    /// 建议文案键：`ritual.<kind>.<due>`；只有可投递决策（`Due` / `DueSoft`）非空。
    pub suggested_message_key: String,
}

/// 规划一组仪式的到点决策（纯函数、确定性、时间注入）。
///
/// 输入：
///
/// - `now_ms`：调用方注入的当前时刻（Unix 毫秒）；
/// - `states`：仪式状态；同 kind 多条时取「最近完成优先、其次静默截止更晚」的
///   代表状态，**每个 kind 至多产出一条**决策；
/// - `config`：排程表与全局旋钮（见 [`RitualConfig`]）。
///
/// 输出顺序固定为 [`RitualKind::ALL`] 的声明顺序，与 `states` 顺序无关。规则表见
/// 模块文档；空 `states` / 空排程返回空向量，不 panic。
///
/// # 不变量
///
/// - 静默期未到期 → 一律 [`DueKind::Muted`]，不看排程与到点；
/// - `consecutive_done` / `skipped_count` 不参与判定，也不进入文案；
/// - 到点后拖得越久只会 `Due` → `DueSoft`，不会反向升级；
/// - 同一 UTC 日内完成过的仪式不会重复 `Due`（默认 24h 重复守卫）。
#[must_use]
pub fn plan_rituals(
    now_ms: u64,
    states: &[RitualState],
    config: &RitualConfig,
) -> Vec<RitualDecision> {
    let mut decisions = Vec::new();
    for kind in RitualKind::ALL {
        let matching: Vec<&RitualState> =
            states.iter().filter(|state| state.kind == kind).collect();
        if matching.is_empty() {
            continue;
        }
        let state = representative(&matching);
        decisions.push(decide(now_ms, state, config));
    }
    decisions
}

/// 同 kind 多条状态的代表：最近完成优先，其次静默截止更晚者（确定性，与输入顺序无关）。
fn representative<'a>(states: &[&'a RitualState]) -> &'a RitualState {
    debug_assert!(!states.is_empty(), "调用方保证非空");
    let mut best = states[0];
    let best_key = representative_key(best);
    for candidate in &states[1..] {
        let candidate_key = representative_key(candidate);
        if candidate_key > best_key {
            best = candidate;
        }
    }
    best
}

/// 代表状态的排序键；不读取 streak 计数。
fn representative_key(state: &RitualState) -> (u64, u64) {
    (state.last_done_ms, state.muted_until_ms.unwrap_or(0))
}

/// 单条状态的规则求值（见模块文档规则表）。
fn decide(now_ms: u64, state: &RitualState, config: &RitualConfig) -> RitualDecision {
    // 规则 1：静默压倒一切（哪怕没有排程）。
    if let Some(until) = state.muted_until_ms {
        if until > now_ms {
            let remaining = until - now_ms;
            return build_decision(
                state.kind,
                DueKind::Muted,
                format!(
                    "已静默：静默期还剩 {}，期间一律不提醒；静默由关系方设定，不是催促理由。",
                    format_span(remaining)
                ),
            );
        }
    }

    // 规则 2：没有排程就不推断到期。
    let Some(schedule) = config.schedule_for(state.kind) else {
        return build_decision(
            state.kind,
            DueKind::NotDue,
            "无排程：该仪式未配置节律，不推断到期；仪式可以永不做。".to_string(),
        );
    };

    // 规则 3：时钟回拨 / 未来时间戳，防御性降级，不外推负时长。
    if state.last_done_ms > now_ms {
        return build_decision(
            state.kind,
            DueKind::NotDue,
            "未到点：上次完成时间晚于注入时钟（时钟回拨），本内核不推断到期。".to_string(),
        );
    }

    // 规则 4：从未完成 → 首次建立（任何节律都从此刻起算）。
    if state.last_done_ms == 0 {
        return build_decision(
            state.kind,
            DueKind::Due,
            format!(
                "到点：首次建立{}仪式（从未完成）；之后按{}节律，偏好时段为{}（仅作文案提示，不参与判定）。",
                state.kind.label(),
                schedule.cadence.label(),
                schedule.preferred_window.label()
            ),
        );
    }

    let elapsed = now_ms - state.last_done_ms;

    // 规则 5：重复守卫（默认 24h 覆盖同一 UTC 日内任意两个时刻）。
    if elapsed < config.dup_window_ms {
        return build_decision(
            state.kind,
            DueKind::NotDue,
            format!(
                "未到点：距上次完成 {}，短于重复守卫 {}（同一 UTC 日内不重复提醒）。",
                format_span(elapsed),
                format_span(config.dup_window_ms)
            ),
        );
    }

    // 规则 6：一次性仪式已完成，不再到期。
    let Some(period) = schedule.cadence.period_ms() else {
        return build_decision(
            state.kind,
            DueKind::NotDue,
            "一次性仪式已完成：不再到期，也不产生任何提醒；可永不做。".to_string(),
        );
    };

    // 规则 7：未到点。
    if elapsed < period {
        return build_decision(
            state.kind,
            DueKind::NotDue,
            format!(
                "未到点：距下次到点还有 {}（{}节律）。",
                format_span(period - elapsed),
                schedule.cadence.label()
            ),
        );
    }

    let late = elapsed - period;

    // 规则 8：到点，仍在宽限内。
    if late <= schedule.skip_grace_ms {
        return build_decision(
            state.kind,
            DueKind::Due,
            format!(
                "到点：{}节律已到期（到点后 {}，宽限 {} 内）；偏好时段为{}（仅作文案提示，不参与判定）。",
                schedule.cadence.label(),
                format_span(late),
                format_span(schedule.skip_grace_ms),
                schedule.preferred_window.label()
            ),
        );
    }

    // 规则 9：错过宽限 → 只降级为软提醒，不升级、不追加。
    let mut reason = format!(
        "{}：{}节律到点后已过 {}，超过宽限 {}；降级为软提醒（只降级不升级）",
        DueKind::DueSoft.label(),
        schedule.cadence.label(),
        format_span(late),
        format_span(schedule.skip_grace_ms)
    );
    reason.push_str(&catchup_note(elapsed, period, config.max_catchup));
    reason.push('。');
    build_decision(state.kind, DueKind::DueSoft, reason)
}

/// 组装决策：只有可投递类型才给非空文案键。
fn build_decision(kind: RitualKind, due: DueKind, reason: String) -> RitualDecision {
    let suggested_message_key = if due.is_actionable() {
        format!("ritual.{}.{}", kind.as_str(), due.as_str())
    } else {
        String::new()
    };
    RitualDecision {
        kind,
        due,
        reason,
        suggested_message_key,
    }
}

/// 补做折算说明：只影响 `DueSoft` 文案，不产生额外提醒、不改变判定。
fn catchup_note(elapsed: u64, period: u64, max_catchup: u32) -> String {
    let missed_cycles = elapsed / period;
    let extra = missed_cycles.saturating_sub(1);
    if extra == 0 {
        return String::new();
    }
    if max_catchup == 0 {
        return "；补做折算关闭（上限 0，只发这一条软提醒）".to_string();
    }
    let cap = u64::from(max_catchup);
    if extra > cap {
        format!("；补做折算已达上限 {max_catchup} 个周期（只发这一条软提醒）")
    } else {
        format!("；补做折算 {extra} 个周期（上限 {max_catchup}，只发这一条软提醒）")
    }
}

/// 把毫秒时长渲染成中文人类可读片段（整数取整，确定性）。
fn format_span(ms: u64) -> String {
    if ms < MINUTE_MS {
        return format!("{ms} 毫秒");
    }
    let days = ms / DAY_MS;
    let hours = (ms % DAY_MS) / HOUR_MS;
    let minutes = (ms % HOUR_MS) / MINUTE_MS;
    let mut parts: Vec<String> = Vec::new();
    if days > 0 {
        parts.push(format!("{days} 天"));
    }
    if hours > 0 {
        parts.push(format!("{hours} 小时"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes} 分钟"));
    }
    if parts.is_empty() {
        "0 分钟".to_string()
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 固定注入时刻（2025-10-09T08:53:20Z；测试不依赖真实日期语义）。
    const NOW: u64 = 1_760_000_000_000;
    /// 测试用宽限：6 小时。
    const GRACE: u64 = 6 * HOUR_MS;

    fn schedule(kind: RitualKind, cadence: Cadence, grace: u64) -> RitualSchedule {
        RitualSchedule::new(kind, cadence, Window::Flexible, grace)
    }

    fn state(kind: RitualKind, last_done_ms: u64) -> RitualState {
        RitualState {
            kind,
            last_done_ms,
            consecutive_done: 0,
            skipped_count: 0,
            muted_until_ms: None,
        }
    }

    fn config_of(schedules: Vec<RitualSchedule>) -> RitualConfig {
        RitualConfig::new(schedules)
    }

    /// 单条状态 + 单条排程的决策。
    fn single(now_ms: u64, state: RitualState, schedule: RitualSchedule) -> RitualDecision {
        let decisions = plan_rituals(now_ms, &[state], &config_of(vec![schedule]));
        assert_eq!(decisions.len(), 1, "单状态单排程必须产出一条决策");
        decisions.into_iter().next().expect("已断言长度为 1")
    }

    fn all_kind_schedules(cadence: Cadence, grace: u64) -> Vec<RitualSchedule> {
        RitualKind::ALL
            .iter()
            .map(|kind| schedule(*kind, cadence, grace))
            .collect()
    }

    #[test]
    fn never_done_is_due_for_every_kind_with_stable_key() {
        let schedules = all_kind_schedules(Cadence::Daily, GRACE);
        let states: Vec<RitualState> = RitualKind::ALL.iter().map(|k| state(*k, 0)).collect();
        let decisions = plan_rituals(NOW, &states, &config_of(schedules));

        assert_eq!(decisions.len(), RitualKind::ALL.len());
        for (decision, kind) in decisions.iter().zip(RitualKind::ALL) {
            assert_eq!(decision.kind, kind);
            assert_eq!(decision.due, DueKind::Due);
            assert_eq!(
                decision.suggested_message_key,
                format!("ritual.{}.due", kind.as_str())
            );
            assert!(
                decision.reason.contains("首次"),
                "理由: {}",
                decision.reason
            );
        }
    }

    #[test]
    fn daily_due_then_soft_after_grace_boundary() {
        let kind = RitualKind::MorningCheckIn;
        let sched = schedule(kind, Cadence::Daily, GRACE);

        let at_due = single(NOW, state(kind, NOW - DAY_MS), sched.clone());
        assert_eq!(at_due.due, DueKind::Due);

        let in_grace = single(NOW, state(kind, NOW - DAY_MS - GRACE), sched.clone());
        assert_eq!(in_grace.due, DueKind::Due, "宽限边界（<=）仍算到点");

        let just_out = single(NOW, state(kind, NOW - DAY_MS - GRACE - 1), sched);
        assert_eq!(just_out.due, DueKind::DueSoft, "超出宽限 1ms 即降级");
        assert_eq!(
            just_out.suggested_message_key,
            "ritual.morning_check_in.due_soft"
        );
    }

    #[test]
    fn weekly_cadence_uses_seven_days() {
        let kind = RitualKind::WeeklyReview;
        let sched = schedule(kind, Cadence::Weekly, GRACE);

        let before = single(NOW, state(kind, NOW - (WEEK_MS - 1)), sched.clone());
        assert_eq!(before.due, DueKind::NotDue);
        assert!(before.reason.contains("未到点"), "理由: {}", before.reason);

        let due = single(NOW, state(kind, NOW - WEEK_MS), sched.clone());
        assert_eq!(due.due, DueKind::Due);

        let soft = single(NOW, state(kind, NOW - WEEK_MS - GRACE - 1), sched);
        assert_eq!(soft.due, DueKind::DueSoft);
    }

    #[test]
    fn once_completed_is_never_due_again() {
        let kind = RitualKind::Anniversary;
        let sched = schedule(kind, Cadence::Once, GRACE);

        let never = single(NOW, state(kind, 0), sched.clone());
        assert_eq!(never.due, DueKind::Due);

        let done = single(NOW, state(kind, NOW - 10 * WEEK_MS), sched);
        assert_eq!(done.due, DueKind::NotDue);
        assert!(
            done.reason.contains("一次性仪式已完成"),
            "理由: {}",
            done.reason
        );
        assert!(done.suggested_message_key.is_empty());
    }

    #[test]
    fn muted_wins_over_everything_until_expiry() {
        let kind = RitualKind::MorningCheckIn;
        let sched = schedule(kind, Cadence::Daily, GRACE);

        // 已错过宽限（本应 DueSoft）+ 静默未到期 → Muted。
        let mut muted = state(kind, NOW - DAY_MS - GRACE - 1);
        muted.muted_until_ms = Some(NOW + 2 * DAY_MS);
        let decision = single(NOW, muted.clone(), sched.clone());
        assert_eq!(decision.due, DueKind::Muted);
        assert!(decision.suggested_message_key.is_empty());
        assert!(
            decision.reason.contains("已静默"),
            "理由: {}",
            decision.reason
        );

        // 静默截止 == now → 已解除，回到正常规则。
        muted.muted_until_ms = Some(NOW);
        assert_eq!(
            single(NOW, muted.clone(), sched.clone()).due,
            DueKind::DueSoft
        );

        // 静默已过期 → 正常规则。
        muted.muted_until_ms = Some(NOW - 1);
        assert_eq!(single(NOW, muted, sched).due, DueKind::DueSoft);

        // 没有排程 + 静默未到期 → 仍然 Muted（静默先于一切）。
        let mut no_schedule = state(RitualKind::SharedProject, 0);
        no_schedule.muted_until_ms = Some(NOW + HOUR_MS);
        let decisions = plan_rituals(NOW, &[no_schedule], &RitualConfig::default());
        assert_eq!(decisions[0].due, DueKind::Muted);
    }

    #[test]
    fn same_day_duplicate_guard_prevents_second_due() {
        let kind = RitualKind::EveningWrap;
        let sched = schedule(kind, Cadence::Daily, GRACE);
        let day_start = (NOW / DAY_MS) * DAY_MS;
        let now = day_start + 12 * HOUR_MS;

        // 同一 UTC 日内 1 小时前完成 → 不重复提醒，且理由应指向重复守卫而非周期。
        let same_day = single(now, state(kind, now - HOUR_MS), sched.clone());
        assert_eq!(same_day.due, DueKind::NotDue);
        assert!(
            same_day.reason.contains("重复守卫"),
            "理由: {}",
            same_day.reason
        );

        // 前一天完成且已到点 → 正常 Due。
        let next_day = single(now, state(kind, now - 25 * HOUR_MS), sched);
        assert_eq!(next_day.due, DueKind::Due);
    }

    #[test]
    fn streak_and_skip_counts_are_never_read() {
        let kind = RitualKind::WeeklyReview;
        let sched = schedule(kind, Cadence::Weekly, GRACE);
        let baseline_state = state(kind, NOW - WEEK_MS - GRACE - 1);
        let baseline = single(NOW, baseline_state.clone(), sched.clone());

        for consecutive in [0u32, 1, 7, 999] {
            for skipped in [0u32, 1, 12, 999] {
                let mut variant = baseline_state.clone();
                variant.consecutive_done = consecutive;
                variant.skipped_count = skipped;
                let decision = single(NOW, variant, sched.clone());
                assert_eq!(
                    decision, baseline,
                    "streak/跳过计数不得影响决策（consecutive={consecutive}, skipped={skipped}）"
                );
            }
        }
    }

    #[test]
    fn duesoft_only_downgrades_as_lateness_grows() {
        let kind = RitualKind::MorningCheckIn;
        let sched = schedule(kind, Cadence::Daily, GRACE);

        let mut saw_soft = false;
        for late in [0, GRACE, GRACE + 1, GRACE + DAY_MS, 7 * DAY_MS, 30 * DAY_MS] {
            let decision = single(NOW, state(kind, NOW - DAY_MS - late), sched.clone());
            let expected = if late <= GRACE {
                DueKind::Due
            } else {
                DueKind::DueSoft
            };
            assert_eq!(decision.due, expected, "late={late}");
            if decision.due == DueKind::DueSoft {
                saw_soft = true;
            } else {
                assert!(!saw_soft, "DueSoft 之后不得再回到 Due（late={late}）");
            }
        }

        // max_catchup 只影响文案，不改变判定，更不会升级。
        let late = 30 * DAY_MS;
        for cap in [0u32, 1, 2, 100] {
            let mut config = config_of(vec![sched.clone()]);
            config.max_catchup = cap;
            let decisions = plan_rituals(NOW, &[state(kind, NOW - DAY_MS - late)], &config);
            assert_eq!(decisions[0].due, DueKind::DueSoft, "cap={cap}");
        }
    }

    #[test]
    fn empty_inputs_and_missing_schedule_do_not_panic() {
        assert!(plan_rituals(NOW, &[], &RitualConfig::default()).is_empty());
        assert!(
            plan_rituals(NOW, &[], &config_of(all_kind_schedules(Cadence::Daily, 0))).is_empty()
        );

        let unknown = state(RitualKind::SharedProject, 0);
        let decisions = plan_rituals(NOW, &[unknown], &RitualConfig::default());
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].due, DueKind::NotDue);
        assert!(decisions[0].suggested_message_key.is_empty());
        assert!(
            decisions[0].reason.contains("无排程"),
            "理由: {}",
            decisions[0].reason
        );
    }

    #[test]
    fn deterministic_and_order_invariant_with_one_decision_per_kind() {
        let schedules = all_kind_schedules(Cadence::Daily, GRACE);
        let mut states: Vec<RitualState> = RitualKind::ALL
            .iter()
            .map(|kind| state(*kind, NOW - DAY_MS - GRACE - 1))
            .collect();
        states.push(state(RitualKind::MorningCheckIn, NOW)); // 同 kind 重复状态

        let config = config_of(schedules);
        let baseline = plan_rituals(NOW, &states, &config);
        assert_eq!(
            baseline,
            plan_rituals(NOW, &states, &config),
            "同输入同输出"
        );

        let mut reversed = states.clone();
        reversed.reverse();
        assert_eq!(
            plan_rituals(NOW, &reversed, &config),
            baseline,
            "输入顺序不得改变输出"
        );

        let kinds: Vec<RitualKind> = baseline.iter().map(|d| d.kind).collect();
        assert_eq!(kinds, RitualKind::ALL.to_vec(), "输出顺序固定为声明顺序");
        assert_eq!(baseline.len(), RitualKind::ALL.len(), "每 kind 至多一条");
    }

    #[test]
    fn clock_skew_degrades_without_due() {
        let kind = RitualKind::MorningCheckIn;
        let sched = schedule(kind, Cadence::Daily, GRACE);

        let future = single(NOW, state(kind, NOW + 10 * DAY_MS), sched.clone());
        assert_eq!(future.due, DueKind::NotDue);
        assert!(
            future.reason.contains("时钟回拨"),
            "理由: {}",
            future.reason
        );

        let max = single(NOW, state(kind, u64::MAX), sched.clone());
        assert_eq!(max.due, DueKind::NotDue);

        let epoch = single(0, state(kind, 0), sched);
        assert_eq!(epoch.due, DueKind::Due, "epoch 0 仍是安全输入");
    }

    #[test]
    fn serde_roundtrips_and_uses_stable_snake_case_tags() {
        let sched = RitualSchedule::new(
            RitualKind::MorningCheckIn,
            Cadence::Daily,
            Window::Morning,
            GRACE,
        );
        let json = serde_json::to_string(&sched).expect("排程序列化");
        for needle in [
            "\"kind\":\"morning_check_in\"",
            "\"cadence\":\"daily\"",
            "\"preferred_window\":\"morning\"",
            "\"skip_grace_ms\":21600000",
        ] {
            assert!(json.contains(needle), "JSON 缺少 {needle}: {json}");
        }
        let back: RitualSchedule = serde_json::from_str(&json).expect("排程反序列化");
        assert_eq!(back, sched);

        let mut st = state(RitualKind::EveningWrap, NOW - HOUR_MS);
        st.consecutive_done = 3;
        st.skipped_count = 2;
        st.muted_until_ms = Some(NOW + HOUR_MS);
        let st_json = serde_json::to_string(&st).expect("状态序列化");
        let st_back: RitualState = serde_json::from_str(&st_json).expect("状态反序列化");
        assert_eq!(st_back, st);

        let config = RitualConfig::new(vec![sched]);
        let config_json = serde_json::to_string(&config).expect("配置序列化");
        let config_back: RitualConfig = serde_json::from_str(&config_json).expect("配置反序列化");
        assert_eq!(config_back, config);

        let decision = single(
            NOW,
            state(RitualKind::EveningWrap, NOW - DAY_MS - GRACE - 1),
            schedule(RitualKind::EveningWrap, Cadence::Daily, GRACE),
        );
        let decision_json = serde_json::to_string(&decision).expect("决策序列化");
        let decision_back: RitualDecision =
            serde_json::from_str(&decision_json).expect("决策反序列化");
        assert_eq!(decision_back, decision);

        assert_eq!(
            serde_json::to_string(&DueKind::DueSoft).expect("DueKind 序列化"),
            "\"due_soft\""
        );
    }

    #[test]
    fn mute_until_helper_is_saturating() {
        let mut config = RitualConfig::default();
        assert_eq!(config.default_mute_ms, DEFAULT_MUTE_MS);
        assert_eq!(config.mute_until(NOW), NOW + DEFAULT_MUTE_MS);

        config.default_mute_ms = 5 * HOUR_MS;
        assert_eq!(config.mute_until(NOW), NOW + 5 * HOUR_MS);
        assert_eq!(config.mute_until(u64::MAX), u64::MAX);
        assert_eq!(config.mute_until(u64::MAX - 1), u64::MAX);
    }

    #[test]
    fn reasons_never_carry_streak_or_guilt_and_keys_gate_delivery() {
        let kind = RitualKind::MorningCheckIn;
        let sched = schedule(kind, Cadence::Daily, GRACE);
        let day_start = (NOW / DAY_MS) * DAY_MS;
        let now = day_start + 12 * HOUR_MS;

        let mut cases = vec![
            single(NOW, state(kind, 0), sched.clone()),
            single(NOW, state(kind, NOW - DAY_MS), sched.clone()),
            single(NOW, state(kind, NOW - DAY_MS - GRACE - 1), sched.clone()),
            single(NOW, state(kind, NOW - HOUR_MS), sched.clone()),
        ];
        let mut muted = state(kind, 0);
        muted.muted_until_ms = Some(NOW + HOUR_MS);
        cases.push(single(NOW, muted, sched.clone()));

        let mut no_schedule = state(RitualKind::SharedProject, 0);
        no_schedule.muted_until_ms = None;
        cases.extend(plan_rituals(NOW, &[no_schedule], &RitualConfig::default()));

        let forbidden = [
            "连续",
            "断签",
            "streak",
            "内疚",
            "亏欠",
            "欠账",
            "应该",
            "必须",
            "又没",
            "你怎么",
            "拖了",
            "迟到",
        ];
        let mut seen = Vec::new();
        for decision in &cases {
            for needle in forbidden {
                assert!(
                    !decision.reason.contains(needle),
                    "reason 含禁用片段 {needle}: {}",
                    decision.reason
                );
            }
            assert!(
                !decision.suggested_message_key.contains("not_due")
                    && !decision.suggested_message_key.contains("muted"),
                "不可投递决策不得给出可误用的键: {}",
                decision.suggested_message_key
            );
            if decision.due.is_actionable() {
                assert_eq!(
                    decision.suggested_message_key,
                    format!(
                        "ritual.{}.{}",
                        decision.kind.as_str(),
                        decision.due.as_str()
                    )
                );
            } else {
                assert!(
                    decision.suggested_message_key.is_empty(),
                    "不可投递决策的键必须为空: {}",
                    decision.suggested_message_key
                );
            }
            seen.push(decision.due);
        }
        for expected in [
            DueKind::Due,
            DueKind::DueSoft,
            DueKind::NotDue,
            DueKind::Muted,
        ] {
            assert!(seen.contains(&expected), "用例未覆盖 {expected:?}");
        }

        // 同日 now 变量用于证明重复守卫理由同样不含禁用词（避免 unused 变量）。
        let same_day = single(now, state(kind, now - HOUR_MS), sched);
        for needle in forbidden {
            assert!(!same_day.reason.contains(needle));
        }
    }

    #[test]
    fn preferred_window_is_advisory_only() {
        let kind = RitualKind::MorningCheckIn;
        let morning = RitualSchedule::new(kind, Cadence::Daily, Window::Morning, GRACE);
        let evening = RitualSchedule::new(kind, Cadence::Daily, Window::Evening, GRACE);
        let st = state(kind, 0);

        let first = single(NOW, st.clone(), morning);
        let second = single(NOW, st, evening);
        assert_eq!(first.due, second.due, "偏好时段不得改变到期判定");
        assert_eq!(first.suggested_message_key, second.suggested_message_key);
        assert_ne!(first.reason, second.reason, "偏好时段只影响理由措辞");
    }
}
