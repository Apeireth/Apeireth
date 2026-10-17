//! 关怀触发器数据面: `care_rules.json` (StoredDoc 拒开) + 四类触发器 + 打扰纪律配置。
//!
//! # StoredDoc 拒开语义
//!
//! `care_rules.json` 落盘为 [`stored_doc::StoredDoc`] 信封 JSON: 身份 / 版本戳 /
//! 主体。打开即校验 —— 畸形 JSON、串档身份、未来版本、不自洽信封、坏条目
//! (非法 id / 非法触发器 / 非法素材 / 重复 id) 一律**拒绝打开**并报结构化错误
//! ([`CareStoreError`]), 不静默迁移、不回退默认、不带病运行。文件缺失是合法
//! 空态 (`Ok(None)`), 是否可缺省由调用方决定。
//!
//! # 触发器词表 (闭合, 四类)
//!
//! | 词 | 语义 | 到点条件 |
//! |---|---|---|
//! | `inactive` | 久未联系 | 距上次交互 (无交互记录时以规则创建时刻为锚) N 小时无交互 |
//! | `daily_moment` | 作息时刻 | 每日 `HH:mm` 到点 (UTC 分钟 of day), 每日一次 |
//! | `anniversary` | 纪念日 | 每年 (月, 日) 当天, 每年一次 |
//! | `interval` | 间隔循环 | 距上次关怀发起每 N 小时 |
//!
//! 词表外的 `kind` 一律拒收 (闭合词表, 见 [`CareTrigger::parse_kind`])。
//!
//! # 写面归类 (write_intent 同族)
//!
//! 改规则 = 写文件 (`care_rules.json`): [`CareWriteFace::RuleFileWrite`], 与
//! 文件写入同一治理族 (每次写停人工审批)。关怀台账 ([`CareStore::record_care`])
//! 是关怀动作自带的本地记账 (自述「最近关怀」真值来源), 随关怀回合同账。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use serde::{Deserialize, Serialize};

use crate::emergence::{
    day_key_from_epoch_ms as day_key, minutes_of_day_from_epoch_ms, ymd_from_epoch_ms,
};

/// `care_rules.json` 的文档身份 (StoredDoc 信封 name 字段)。
pub const CARE_RULES_DOC_NAME: &str = "apeireth.care_rules";
/// `care_rules.json` 的格式版本戳 (读端当前版本)。
pub const CARE_RULES_DOC_VERSION: u32 = 1;
/// 数据目录里的文件名。
pub const CARE_RULES_FILE_NAME: &str = "care_rules.json";
/// 规则 id 的最大长度 (字节)。
pub const CARE_RULE_ID_MAX_BYTES: usize = 64;
/// 单表规则条数上限 (有界面: 不随模型调用无界增长)。
pub const CARE_RULES_MAX_COUNT: usize = 128;
/// 单条素材最大长度 (字节)。
pub const CARE_MATERIAL_MAX_BYTES: usize = 2_000;
/// 关怀台账有界条数 (自述只读最近一条, 台账留痕不无界增长)。
pub const CARE_HISTORY_MAX_COUNT: usize = 32;
/// 触发器小时参数的上限 (1 年): 越界不是排程是配置错误。
pub const CARE_TRIGGER_HOURS_MAX: u32 = 24 * 366;

/// 读端版本契约: 只认当前版本 (历史/未来版本拒开)。
pub fn care_rules_doc_compat() -> DocCompat {
    DocCompat::exact(CARE_RULES_DOC_NAME, CARE_RULES_DOC_VERSION)
}

/// 触发器词表 (闭合, 四类)。
///
/// 落盘与工具参数共用同一词表: `kind` 字段是 snake_case 闭合词,
/// 词外拒绝 ([`CareTrigger::parse_kind`])。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CareTrigger {
    /// 久未联系: 距上次交互 N 小时无交互 (无交互记录时以规则创建时刻为锚;
    /// 发起后以发起时刻为锚续算, 即连续沉默每 N 小时一次)。
    Inactive {
        /// 沉默小时数。
        hours: u32,
    },
    /// 作息时刻: 每日 `HH:mm` (分钟 of day, UTC) 到点, 每日一次;
    /// 过点后 [`CareDiscipline::catchup_minutes`] 内仍补发, 超窗当日跳过。
    DailyMoment {
        /// 当天分钟 (0..=1439)。
        minutes_of_day: u32,
    },
    /// 纪念日: 每年 (月, 日) 当天到点, 每年一次。
    Anniversary {
        /// 月 (1..=12)。
        month: u32,
        /// 日 (1..=31, 按月历上限校验; 2 月 29 日只在闰日当天到点)。
        day: u32,
    },
    /// 间隔循环: 距上次关怀发起 (从未发起时以规则创建时刻为锚) 每 N 小时到点。
    Interval {
        /// 间隔小时数。
        hours: u32,
    },
}

impl CareTrigger {
    /// 稳定词 (落盘 / 工具参数 / 自述共用)。
    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::Inactive { .. } => "inactive",
            Self::DailyMoment { .. } => "daily_moment",
            Self::Anniversary { .. } => "anniversary",
            Self::Interval { .. } => "interval",
        }
    }

    /// 从工具参数词表解析 kind; 未知词拒绝 (闭合词表)。
    pub fn parse_kind(raw: &str) -> Result<&'static str, String> {
        match raw {
            "inactive" => Ok("inactive"),
            "daily_moment" => Ok("daily_moment"),
            "anniversary" => Ok("anniversary"),
            "interval" => Ok("interval"),
            other => Err(format!(
                "unknown care trigger kind {other:?}; the closed vocabulary is inactive / \
                 daily_moment / anniversary / interval"
            )),
        }
    }

    /// `HH:mm` → 当天分钟 (作息时刻参数面)。
    pub fn parse_hhmm(raw: &str) -> Result<u32, String> {
        let raw = raw.trim();
        let (hour, minute) = raw
            .split_once(':')
            .ok_or_else(|| format!("daily_moment time {raw:?} must be HH:mm"))?;
        let hour: u32 = hour
            .parse()
            .map_err(|_| format!("daily_moment hour {hour:?} must be a number"))?;
        let minute: u32 = minute
            .parse()
            .map_err(|_| format!("daily_moment minute {minute:?} must be a number"))?;
        if hour >= 24 || minute >= 60 {
            return Err(format!(
                "daily_moment time {raw:?} is out of range (hour 0..=23, minute 0..=59)"
            ));
        }
        Ok(hour * 60 + minute)
    }

    /// 当天分钟 → `HH:mm` (自述/工具结果用)。
    pub fn format_hhmm(minutes_of_day: u32) -> String {
        format!(
            "{:02}:{:02}",
            minutes_of_day / 60,
            minutes_of_day.rem_euclid(60)
        )
    }

    /// 参数闭合校验 (装载与写出同口径, 坏条目拒开也拒写)。
    pub fn validate(&self) -> Result<(), String> {
        match *self {
            Self::Inactive { hours } | Self::Interval { hours } => {
                if hours == 0 || hours > CARE_TRIGGER_HOURS_MAX {
                    return Err(format!(
                        "care trigger hours must be 1..={CARE_TRIGGER_HOURS_MAX}, got {hours}"
                    ));
                }
            }
            Self::DailyMoment { minutes_of_day } => {
                if minutes_of_day >= 24 * 60 {
                    return Err(format!(
                        "daily_moment minutes_of_day must be 0..=1439, got {minutes_of_day}"
                    ));
                }
            }
            Self::Anniversary { month, day } => {
                if !(1..=12).contains(&month) {
                    return Err(format!("anniversary month must be 1..=12, got {month}"));
                }
                let max_day = days_in_month(month);
                if day == 0 || day > max_day {
                    return Err(format!(
                        "anniversary day must be 1..={max_day} for month {month}, got {day}"
                    ));
                }
            }
        }
        Ok(())
    }

    /// 到点判定 (确定性, 时钟由调用方注入)。
    ///
    /// - `anchor_ms`: 规则创建时刻 (无交互/无发起记录时的锚点);
    /// - `last_fired_ms`: 该规则上次发起时刻 (跨日去重 + 间隔循环锚点);
    /// - `last_interaction_ms`: 上次交互时刻 (久未联系锚点);
    /// - `catchup_minutes`: 过点补发窗口 (作息时刻用)。
    pub fn is_due(
        &self,
        now_ms: i64,
        anchor_ms: i64,
        last_fired_ms: Option<i64>,
        last_interaction_ms: Option<i64>,
        catchup_minutes: u32,
    ) -> bool {
        match *self {
            Self::Inactive { hours } => {
                let reference = [last_interaction_ms, last_fired_ms, Some(anchor_ms)]
                    .into_iter()
                    .flatten()
                    .max()
                    .unwrap_or(anchor_ms);
                now_ms.saturating_sub(reference) >= i64::from(hours) * 3_600_000
            }
            Self::Interval { hours } => {
                let reference = last_fired_ms.unwrap_or(anchor_ms);
                now_ms.saturating_sub(reference) >= i64::from(hours) * 3_600_000
            }
            Self::DailyMoment { minutes_of_day } => {
                let today = day_key(now_ms);
                if last_fired_ms.map(day_key).as_deref() == Some(today.as_str()) {
                    return false;
                }
                let minutes_now = minutes_of_day_from_epoch_ms(now_ms);
                minutes_now >= minutes_of_day
                    && minutes_now <= minutes_of_day.saturating_add(catchup_minutes)
            }
            Self::Anniversary { month, day } => {
                let today = day_key(now_ms);
                if last_fired_ms.map(day_key).as_deref() == Some(today.as_str()) {
                    return false;
                }
                let (_, month_now, day_now) = ymd_from_epoch_ms(now_ms);
                month_now == month && day_now == day
            }
        }
    }

    /// 下一次到点时刻 (自述「下次触发」真值; 到点未发 = 现在)。
    pub fn next_occurrence_ms(
        &self,
        now_ms: i64,
        anchor_ms: i64,
        last_fired_ms: Option<i64>,
        last_interaction_ms: Option<i64>,
    ) -> i64 {
        match *self {
            Self::Inactive { hours } => {
                let reference = [last_interaction_ms, last_fired_ms, Some(anchor_ms)]
                    .into_iter()
                    .flatten()
                    .max()
                    .unwrap_or(anchor_ms);
                reference + i64::from(hours) * 3_600_000
            }
            Self::Interval { hours } => {
                last_fired_ms.unwrap_or(anchor_ms) + i64::from(hours) * 3_600_000
            }
            Self::DailyMoment { minutes_of_day } => {
                let today_start = now_ms - i64::from(minutes_of_day_from_epoch_ms(now_ms)) * 60_000;
                let target = today_start + i64::from(minutes_of_day) * 60_000;
                if target >= now_ms {
                    target
                } else {
                    target + 86_400_000
                }
            }
            Self::Anniversary { month, day } => {
                let (year, month_now, day_now) = ymd_from_epoch_ms(now_ms);
                let this_year = epoch_ms_from_ymd(year, month, day);
                if (month_now, day_now) <= (month, day) {
                    this_year
                } else {
                    epoch_ms_from_ymd(year + 1, month, day)
                }
            }
        }
    }

    /// 语义化描述 (关怀回合输入 / 自述用)。
    pub fn describe(&self) -> String {
        match *self {
            Self::Inactive { hours } => format!("久未联系 {hours} 小时无交互"),
            Self::DailyMoment { minutes_of_day } => {
                format!("每日作息时刻 {}", Self::format_hhmm(minutes_of_day))
            }
            Self::Anniversary { month, day } => format!("纪念日 {month:02}-{day:02}"),
            Self::Interval { hours } => format!("间隔循环 每 {hours} 小时"),
        }
    }
}

/// 规则条目 (每条规则 → 一个关怀候选: 素材 + 时机)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareRule {
    /// 规则 id (闭合字符集, 见 [`validate_care_rule_id`])。
    pub id: String,
    /// 触发器 (四类词表)。
    pub trigger: CareTrigger,
    /// 素材: 关怀候选的内容素材 (主题/方向)。
    pub material: String,
    /// 是否启用 (关 = 不参与触发, 留档不删)。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 入表时刻 (epoch 毫秒; 久未联系/间隔循环的初始锚点)。
    pub created_at_ms: i64,
}

fn default_enabled() -> bool {
    true
}

/// 一条关怀台账 (自述「最近关怀」真值来源)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareRecord {
    /// 触发的规则 id。
    pub rule_id: String,
    /// 发起/判定时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 发起意愿判定档 (`rule_fallback` / `model_context`)。
    pub judge: String,
    /// 是否真的发起 (false = 显式判定说不, 留痕)。
    pub spoke: bool,
    /// 如实备注 (判定理由 / 内容方向; 不编造)。
    pub note: String,
}

/// 打扰纪律配置 (先立规矩再谈温度)。
///
/// 四件中的三件是本配置面: 勿扰窗口 / 频率上限 / 专注抑制; 第四件「关怀预算」
/// 走同账本接缝 (见 [`super::engine::CareBudgetGate`]), 不在配置面。
///
/// 默认口径 (设计裁定 R1⑬⑭⑮): 勿扰窗口 23:00-8:00 (与发声面勿扰窗口同源),
/// 频率 4h≤1 次 + 日≤5 次, 专注抑制 3 分钟。显式字段照常覆盖默认 (旧行为兼容)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareDiscipline {
    /// 勿扰窗口起 (分钟 of day; 缺省 = 23:00 默认窗, 显式 null = 关窗)。
    #[serde(default = "default_quiet_start_minutes")]
    pub quiet_start_minutes: Option<u32>,
    /// 勿扰窗口止 (分钟 of day; 跨午夜 = 起点 > 终点; 缺省 = 08:00 默认窗)。
    #[serde(default = "default_quiet_end_minutes")]
    pub quiet_end_minutes: Option<u32>,
    /// 频率上限 (滚动窗口档): 每 `frequency_window_hours` 小时最多 `frequency_max` 次。
    #[serde(default = "default_frequency_max")]
    pub frequency_max: u32,
    /// 频率窗口 (小时)。
    #[serde(default = "default_frequency_window_hours")]
    pub frequency_window_hours: u32,
    /// 频率上限 (日档): 每个 UTC 日最多 `daily_max` 次 (与滚动窗口档同时生效)。
    #[serde(default = "default_daily_max")]
    pub daily_max: u32,
    /// 专注抑制: 距上次交互不足此分钟数 = 连续交互期, 不打扰。
    #[serde(default = "default_focus_gap_minutes")]
    pub focus_gap_minutes: u32,
    /// 作息时刻过点补发窗口 (分钟): 过点后此窗口内仍补发, 超窗当日跳过。
    #[serde(default = "default_catchup_minutes")]
    pub catchup_minutes: u32,
}

/// 默认勿扰窗口起 = 23:00 (设计裁定 R1⑭; 与发声面 [`QuietHours`] 默认同源)。
///
/// [`QuietHours`]: 语义参照发声面安静纪律 (同一窗口值, 同源接缝见
/// runtime-assembly 的 `voice_quiet_hours_from_care_rules`)。
fn default_quiet_start_minutes() -> Option<u32> {
    Some(23 * 60)
}

/// 默认勿扰窗口止 = 08:00 (跨午夜: 起 > 止)。
fn default_quiet_end_minutes() -> Option<u32> {
    Some(8 * 60)
}

/// 默认频率上限 (滚动窗口档) = 每 4 小时≤1 次 (设计裁定 R1⑬)。
fn default_frequency_max() -> u32 {
    1
}

/// 默认频率窗口 = 4 小时 (设计裁定 R1⑬: 4h≤1 次)。
fn default_frequency_window_hours() -> u32 {
    4
}

/// 默认频率上限 (日档) = 每日≤5 次 (设计裁定 R1⑬)。
fn default_daily_max() -> u32 {
    5
}

/// 默认专注抑制 = 3 分钟 (设计裁定 R1⑮: 连续交互 3 分钟内不打扰)。
fn default_focus_gap_minutes() -> u32 {
    3
}

fn default_catchup_minutes() -> u32 {
    120
}

impl Default for CareDiscipline {
    fn default() -> Self {
        Self {
            quiet_start_minutes: default_quiet_start_minutes(),
            quiet_end_minutes: default_quiet_end_minutes(),
            frequency_max: default_frequency_max(),
            frequency_window_hours: default_frequency_window_hours(),
            daily_max: default_daily_max(),
            focus_gap_minutes: default_focus_gap_minutes(),
            catchup_minutes: default_catchup_minutes(),
        }
    }
}

impl CareDiscipline {
    /// 在勿扰窗口内 (与 initiative 语义的安静窗口同口径: 跨午夜 = 起 > 止)。
    pub fn in_quiet_window(&self, minutes_of_day: u32) -> bool {
        match (self.quiet_start_minutes, self.quiet_end_minutes) {
            (Some(start), Some(end)) if start <= end => {
                minutes_of_day >= start && minutes_of_day < end
            }
            (Some(start), Some(end)) => minutes_of_day >= start || minutes_of_day < end,
            _ => false,
        }
    }

    /// 配置闭合校验 (坏配置拒开也拒写)。
    pub fn validate(&self) -> Result<(), String> {
        for (label, minutes) in [
            ("quiet_start_minutes", self.quiet_start_minutes),
            ("quiet_end_minutes", self.quiet_end_minutes),
        ] {
            if let Some(minutes) = minutes {
                if minutes >= 24 * 60 {
                    return Err(format!("{label} must be 0..=1439, got {minutes}"));
                }
            }
        }
        if self.frequency_max == 0 {
            return Err("frequency_max must be at least 1".to_string());
        }
        if self.daily_max == 0 {
            return Err("daily_max must be at least 1".to_string());
        }
        if self.frequency_window_hours == 0 || self.frequency_window_hours > CARE_TRIGGER_HOURS_MAX
        {
            return Err(format!(
                "frequency_window_hours must be 1..={CARE_TRIGGER_HOURS_MAX}, got {}",
                self.frequency_window_hours
            ));
        }
        Ok(())
    }

    /// 频率窗口毫秒数。
    pub fn frequency_window_ms(&self) -> i64 {
        i64::from(self.frequency_window_hours) * 3_600_000
    }

    /// 专注抑制毫秒数。
    pub fn focus_gap_ms(&self) -> i64 {
        i64::from(self.focus_gap_minutes) * 60_000
    }
}

/// `care_rules.json` 的主体 (规则 + 打扰纪律 + 发起留痕)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareRulesBody {
    /// 规则列表 (有序, 按入表先后; 触发按序取第一个到点的)。
    #[serde(default)]
    pub rules: Vec<CareRule>,
    /// 打扰纪律配置。
    #[serde(default)]
    pub discipline: CareDiscipline,
    /// 每条规则上次发起/判定时刻 (跨日去重 + 间隔循环锚点)。
    #[serde(default)]
    pub fired_at: BTreeMap<String, i64>,
    /// 关怀台账 (有界留痕, 自述「最近关怀」真值来源)。
    #[serde(default)]
    pub history: Vec<CareRecord>,
}

impl CareRulesBody {
    /// 最近一条关怀台账 (None = 尚无关怀)。
    pub fn last_care(&self) -> Option<&CareRecord> {
        self.history.last()
    }

    /// 某规则上次发起/判定时刻。
    pub fn last_fired(&self, rule_id: &str) -> Option<i64> {
        self.fired_at.get(rule_id).copied()
    }
}

/// 规则面错误 (闭合词表: 拒开 / 拒收 / 表满 / 重复 / 缺席)。
#[derive(Debug, thiserror::Error)]
pub enum CareStoreError {
    /// `care_rules.json` 存在但拒开 (StoredDoc 语义: 不静默修复)。
    #[error("care_rules.json refused to open: {0}")]
    Stored(#[from] StoredDocError),
    /// 规则条目被拒收 (非法 id / 非法触发器 / 非法素材)。
    #[error("care rule refused: {0}")]
    Refused(String),
    /// 规则 id 重复。
    #[error("care rule id already scheduled: {0}")]
    DuplicateId(String),
    /// 规则 id 不在表内。
    #[error("care rule id not scheduled: {0}")]
    UnknownId(String),
    /// 规则表已满 (有界面)。
    #[error("care rule list is full (max {0})")]
    Full(usize),
}

/// 写面归类 (write_intent 同族: 改规则 = 写文件)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CareWriteFace {
    /// 纯读 (列规则), 零本地写。
    ReadOnly,
    /// 改规则 = 写 `care_rules.json` —— 与文件写入同一治理族。
    RuleFileWrite,
}

impl CareWriteFace {
    /// 稳定标签 (审计/测试用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::RuleFileWrite => "rule_file_write",
        }
    }
}

/// 规则 id 校验: 闭合字符集 `[A-Za-z0-9._-]`, 1..=[`CARE_RULE_ID_MAX_BYTES`] 字节。
///
/// id 会进关怀台账与关怀回合输入, 故字符集收紧 (无空白/无分隔符/无控制符)。
pub fn validate_care_rule_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("care rule id must not be empty".to_string());
    }
    if raw.len() > CARE_RULE_ID_MAX_BYTES {
        return Err(format!(
            "care rule id is {} bytes; the maximum is {CARE_RULE_ID_MAX_BYTES}",
            raw.len()
        ));
    }
    if !raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(
            "care rule id must match [A-Za-z0-9._-] only (it is embedded in care records and \
             turn inputs)"
                .to_string(),
        );
    }
    Ok(())
}

/// 素材校验: 非空、有界、无控制符 (素材进关怀回合输入, 控制符会伪造结构)。
pub fn validate_care_material(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("care material must not be empty".to_string());
    }
    if raw.len() > CARE_MATERIAL_MAX_BYTES {
        return Err(format!(
            "care material is {} bytes; the maximum is {CARE_MATERIAL_MAX_BYTES}",
            raw.len()
        ));
    }
    if raw.chars().any(char::is_control) {
        return Err("care material must not contain control characters".to_string());
    }
    Ok(())
}

/// 规则表存储 (数据目录 `care_rules.json`)。
#[derive(Debug, Clone)]
pub struct CareStore {
    path: PathBuf,
}

impl CareStore {
    /// 绑定数据目录。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: data_dir.into().join(CARE_RULES_FILE_NAME),
        }
    }

    /// `care_rules.json` 路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 装载规则表 (StoredDoc 拒开语义):
    ///
    /// - 文件缺失 = `Ok(None)` (合法空态);
    /// - 存在但坏 (畸形 / 串档 / 未来版本 / 坏条目) = `Err` (拒开, 不回退)。
    pub fn load(&self) -> Result<Option<CareRulesBody>, CareStoreError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<CareRulesBody>(&self.path, &care_rules_doc_compat())?;
        validate_body(&doc.body)?;
        Ok(Some(doc.body))
    }

    /// 落盘规则表 (持久档原子写, 盖当前版本戳; 坏表拒写)。
    pub fn save(&self, body: &CareRulesBody) -> Result<(), CareStoreError> {
        validate_body(body)?;
        stored_doc::save_single(
            &self.path,
            &care_rules_doc_compat(),
            body,
            stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }

    /// 增规则 (改规则 = 写文件, [`CareWriteFace::RuleFileWrite`])。
    pub fn add_rule(&self, rule: CareRule) -> Result<CareRulesBody, CareStoreError> {
        validate_care_rule_id(&rule.id).map_err(CareStoreError::Refused)?;
        rule.trigger.validate().map_err(CareStoreError::Refused)?;
        validate_care_material(&rule.material).map_err(CareStoreError::Refused)?;
        let mut body = self.load()?.unwrap_or_default();
        if body.rules.iter().any(|item| item.id == rule.id) {
            return Err(CareStoreError::DuplicateId(rule.id));
        }
        if body.rules.len() >= CARE_RULES_MAX_COUNT {
            return Err(CareStoreError::Full(CARE_RULES_MAX_COUNT));
        }
        body.rules.push(rule);
        self.save(&body)?;
        Ok(body)
    }

    /// 删规则 (改规则 = 写文件, [`CareWriteFace::RuleFileWrite`])。
    ///
    /// 发起留痕 (`fired_at`) 随规则清掉 (锚点随规则走); 关怀台账**保留**
    /// (发生过的事不改写, 自述「最近关怀」仍是真值)。
    pub fn remove_rule(&self, id: &str) -> Result<CareRulesBody, CareStoreError> {
        let mut body = self
            .load()?
            .ok_or_else(|| CareStoreError::UnknownId(id.to_string()))?;
        let before = body.rules.len();
        body.rules.retain(|item| item.id != id);
        if body.rules.len() == before {
            return Err(CareStoreError::UnknownId(id.to_string()));
        }
        body.fired_at.remove(id);
        self.save(&body)?;
        Ok(body)
    }

    /// 列规则 (纯读)。
    pub fn list(&self) -> Result<CareRulesBody, CareStoreError> {
        Ok(self.load()?.unwrap_or_default())
    }

    /// 记一笔关怀台账 + 发起留痕 (关怀动作自带的本地记账, 随关怀回合同档)。
    pub fn record_care(
        &self,
        record: CareRecord,
        fired_at_ms: i64,
    ) -> Result<CareRulesBody, CareStoreError> {
        let mut body = self.load()?.unwrap_or_default();
        body.fired_at.insert(record.rule_id.clone(), fired_at_ms);
        body.history.push(record);
        while body.history.len() > CARE_HISTORY_MAX_COUNT {
            body.history.remove(0);
        }
        self.save(&body)?;
        Ok(body)
    }
}

/// 表体校验 (装载与写出同口径, 坏表拒开也拒写)。
fn validate_body(body: &CareRulesBody) -> Result<(), CareStoreError> {
    body.discipline
        .validate()
        .map_err(CareStoreError::Refused)?;
    if body.rules.len() > CARE_RULES_MAX_COUNT {
        return Err(CareStoreError::Full(CARE_RULES_MAX_COUNT));
    }
    let mut seen = std::collections::BTreeSet::new();
    for rule in &body.rules {
        validate_care_rule_id(&rule.id).map_err(CareStoreError::Refused)?;
        rule.trigger.validate().map_err(CareStoreError::Refused)?;
        validate_care_material(&rule.material).map_err(CareStoreError::Refused)?;
        if !seen.insert(rule.id.clone()) {
            return Err(CareStoreError::DuplicateId(rule.id.clone()));
        }
    }
    for rule_id in body.fired_at.keys() {
        if !seen.contains(rule_id) {
            return Err(CareStoreError::Refused(format!(
                "fired_at entry {rule_id:?} has no matching rule"
            )));
        }
    }
    Ok(())
}

/// 关怀节自述真值 (探测口投影的中间形态, 由组装根映射到自述关怀节)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareStoreStatus {
    /// 规则数 (`care_rules.json` 真值计数)。
    pub rules: u64,
    /// 下次触发时刻 (epoch 毫秒; `None` = 无启用规则)。
    pub next_trigger_at_ms: Option<i64>,
    /// 最近关怀台账 (`None` = 尚无关怀)。
    pub last_care: Option<CareRecord>,
}

impl CareStore {
    /// 自述关怀节真值 (纯读, 现探测)。
    ///
    /// - `last_interaction_ms` = 进程内观察到的上次交互 (缺位时久未联系类规则
    ///   以规则创建时刻为锚, 如实);
    /// - 拒开 (StoredDoc 语义) = `Err` —— 由调用方 (探测口) 转「显式 null +
    ///   原因」, 本层不吞错不编数。
    pub fn status(
        &self,
        now_ms: i64,
        last_interaction_ms: Option<i64>,
    ) -> Result<CareStoreStatus, CareStoreError> {
        let body = self.load()?.unwrap_or_default();
        let next_trigger_at_ms = body
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .map(|rule| {
                rule.trigger.next_occurrence_ms(
                    now_ms,
                    rule.created_at_ms,
                    body.last_fired(&rule.id),
                    last_interaction_ms,
                )
            })
            .min();
        Ok(CareStoreStatus {
            rules: body.rules.len() as u64,
            next_trigger_at_ms,
            last_care: body.last_care().cloned(),
        })
    }
}

/// (年, 月, 日) → epoch ms (UTC 零点; 下次纪念日推算用)。
fn epoch_ms_from_ymd(year: i64, month: u32, day: u32) -> i64 {
    let days = days_from_civil(year, month, day);
    days * 86_400_000
}

/// Gregorian 日期 → 距 epoch 天数 (Howard Hinnant `days_from_civil`)。
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_i = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month_i - 3 } else { month_i + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// 某月的天数 (2 月按 29 上限: 闰日纪念日合法, 只在闰日当天到点)。
const fn days_in_month(month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => 29,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, trigger: CareTrigger) -> CareRule {
        CareRule {
            id: id.to_string(),
            trigger,
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn trigger_vocabulary_is_closed_and_round_trips() {
        assert_eq!(CareTrigger::parse_kind("inactive").unwrap(), "inactive");
        assert_eq!(
            CareTrigger::parse_kind("daily_moment").unwrap(),
            "daily_moment"
        );
        assert_eq!(
            CareTrigger::parse_kind("anniversary").unwrap(),
            "anniversary"
        );
        assert_eq!(CareTrigger::parse_kind("interval").unwrap(), "interval");
        assert!(CareTrigger::parse_kind("cron").is_err(), "词表外拒绝");
        assert!(CareTrigger::parse_kind("").is_err());
    }

    #[test]
    fn trigger_labels_and_hhmm_helpers_agree_with_the_wire_format() {
        assert_eq!(CareTrigger::Inactive { hours: 1 }.kind_label(), "inactive");
        assert_eq!(
            CareTrigger::DailyMoment { minutes_of_day: 0 }.kind_label(),
            "daily_moment"
        );
        assert_eq!(
            CareTrigger::Anniversary { month: 1, day: 1 }.kind_label(),
            "anniversary"
        );
        assert_eq!(CareTrigger::Interval { hours: 1 }.kind_label(), "interval");
        assert_eq!(CareTrigger::parse_hhmm("08:30").unwrap(), 510);
        assert_eq!(CareTrigger::parse_hhmm(" 23:59 ").unwrap(), 1439);
        assert_eq!(CareTrigger::format_hhmm(510), "08:30");
        assert!(CareTrigger::parse_hhmm("8:5").is_ok());
        assert!(CareTrigger::parse_hhmm("24:00").is_err());
        assert!(CareTrigger::parse_hhmm("12:60").is_err());
        assert!(CareTrigger::parse_hhmm("noon").is_err());
    }

    #[test]
    fn trigger_validation_refuses_impossible_parameters() {
        assert!(CareTrigger::Inactive { hours: 0 }.validate().is_err());
        assert!(CareTrigger::Interval { hours: 0 }.validate().is_err());
        assert!(CareTrigger::Interval {
            hours: CARE_TRIGGER_HOURS_MAX + 1
        }
        .validate()
        .is_err());
        assert!(CareTrigger::DailyMoment {
            minutes_of_day: 1440
        }
        .validate()
        .is_err());
        assert!(CareTrigger::Anniversary { month: 13, day: 1 }
            .validate()
            .is_err());
        assert!(CareTrigger::Anniversary { month: 4, day: 31 }
            .validate()
            .is_err());
        assert!(CareTrigger::Anniversary { month: 2, day: 30 }
            .validate()
            .is_err());
        assert!(CareTrigger::Anniversary { month: 2, day: 29 }
            .validate()
            .is_ok());
        assert!(CareTrigger::Anniversary { month: 12, day: 31 }
            .validate()
            .is_ok());
    }

    #[test]
    fn add_list_remove_round_trip_persists_the_stored_doc() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = CareStore::new(dir.path());
        assert!(store.list().expect("empty list").rules.is_empty());

        store
            .add_rule(rule("check-in", CareTrigger::Inactive { hours: 48 }))
            .expect("add");
        store
            .add_rule(rule("evening", CareTrigger::Interval { hours: 6 }))
            .expect("add");
        let body = store.list().expect("list");
        assert_eq!(body.rules.len(), 2);
        assert_eq!(body.rules[0].id, "check-in");

        // 落盘形态是 StoredDoc 信封 JSON (身份 + 版本戳)。
        let raw = std::fs::read_to_string(store.path()).expect("read");
        assert!(raw.contains(CARE_RULES_DOC_NAME), "{raw}");
        assert!(raw.contains("\"version\""), "{raw}");

        let body = store.remove_rule("check-in").expect("remove");
        assert_eq!(body.rules.len(), 1);
        assert_eq!(body.rules[0].id, "evening");
    }

    #[test]
    fn duplicate_ids_and_oversized_tables_are_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = CareStore::new(dir.path());
        store
            .add_rule(rule("dup", CareTrigger::Interval { hours: 6 }))
            .expect("add");
        let err = store
            .add_rule(rule("dup", CareTrigger::Interval { hours: 6 }))
            .unwrap_err();
        assert!(matches!(err, CareStoreError::DuplicateId(_)), "{err}");
        let err = store.remove_rule("missing").unwrap_err();
        assert!(matches!(err, CareStoreError::UnknownId(_)), "{err}");
        assert!(validate_care_rule_id("").is_err());
        assert!(validate_care_rule_id("has space").is_err());
        assert!(validate_care_rule_id(&"x".repeat(CARE_RULE_ID_MAX_BYTES + 1)).is_err());
        assert!(validate_care_material("").is_err());
        assert!(validate_care_material("   ").is_err());
        assert!(validate_care_material(&"x".repeat(CARE_MATERIAL_MAX_BYTES + 1)).is_err());
        assert!(validate_care_material("带\n控制符").is_err());
    }

    #[test]
    fn discipline_config_is_validated_and_the_quiet_window_matches_initiative_semantics() {
        let discipline = CareDiscipline {
            quiet_start_minutes: Some(22 * 60),
            quiet_end_minutes: Some(7 * 60),
            ..CareDiscipline::default()
        };
        assert!(discipline.validate().is_ok());
        assert!(discipline.in_quiet_window(23 * 60), "跨午夜窗口覆盖深夜");
        assert!(discipline.in_quiet_window(3 * 60));
        assert!(!discipline.in_quiet_window(12 * 60), "白天不在窗口内");
        assert!(
            !CareDiscipline::default().in_quiet_window(12 * 60),
            "默认窗口 (23:00-8:00) 白天不拦"
        );
        assert!(
            CareDiscipline::default().in_quiet_window(23 * 60)
                && CareDiscipline::default().in_quiet_window(3 * 60),
            "默认窗口覆盖深夜 (设计裁定 R1⑭)"
        );

        let mut bad = CareDiscipline::default();
        bad.frequency_max = 0;
        assert!(bad.validate().is_err());
        bad.frequency_max = 1;
        bad.daily_max = 0;
        assert!(bad.validate().is_err(), "daily_max 至少 1 (坏配置拒开拒写)");
        bad.daily_max = 1;
        bad.quiet_start_minutes = Some(1440);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn record_care_keeps_firing_state_and_history_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = CareStore::new(dir.path());
        store
            .add_rule(rule("check-in", CareTrigger::Interval { hours: 6 }))
            .expect("add");
        for i in 0..(CARE_HISTORY_MAX_COUNT + 5) {
            store
                .record_care(
                    CareRecord {
                        rule_id: "check-in".to_string(),
                        at_ms: 1_700_000_000_000 + i as i64 * 1_000,
                        judge: "rule_fallback".to_string(),
                        spoke: true,
                        note: "规则兜底发起".to_string(),
                    },
                    1_700_000_000_000 + i as i64 * 1_000,
                )
                .expect("record");
        }
        let body = store.list().expect("list");
        assert_eq!(body.history.len(), CARE_HISTORY_MAX_COUNT, "台账有界");
        assert_eq!(
            body.last_fired("check-in"),
            Some(1_700_000_000_000 + (CARE_HISTORY_MAX_COUNT + 4) as i64 * 1_000)
        );
        assert_eq!(
            body.last_care().map(|record| record.at_ms),
            Some(1_700_000_000_000 + (CARE_HISTORY_MAX_COUNT + 4) as i64 * 1_000)
        );
        // 删规则清发起留痕, 台账保留 (发生过的事不改写)。
        let body = store.remove_rule("check-in").expect("remove");
        assert!(body.fired_at.is_empty());
        assert_eq!(body.history.len(), CARE_HISTORY_MAX_COUNT);
    }
}
