//! 守护工具面 (`tool.guard_watch`, 声明名 `guard_watch`) —— K 轴 L5 看家骨架。
//!
//! # 守护规则面 (规则 = 设备状态阈值 / 时序条件)
//!
//! 守护规则落 `guard-rules.json` ([`stored_doc::StoredDoc`] 信封, 与
//! `devices.json` **同族拒开语义**: 畸形 / 串档 / 未来版本 / 坏条目一律拒开,
//! 不静默迁移不回退默认)。规则两半:
//!
//! - **阈值条件** ([`GuardCondition`], 闭合词表): 温度上/下限 / 门磁 /
//!   异常离线 —— 缺读数 = [`GuardMatch::Unknown`] **不告警不编造**;
//! - **时序条件** ([`GuardRule::sustain_minutes`] 连续命中窗口 +
//!   [`GuardRule::active_window`] 生效时段, 跨午夜 = 起 > 止)。
//!
//! 命中 → **告警 = 关怀候选高优先**: 告警入 `guard-alerts.jsonl`
//! (append-only, 逐行一个 StoredDoc 信封), 由提醒侧消费端
//! (`apeireth_organ::care::reminder`) 转关怀候选走打扰纪律**守护例外档**
//! (默认可破勿扰 + 「守护告警」标注 + 频率独立上限)。
//!
//! # 四条安全底线继承: 看到 ≠ 动手 (只看不动)
//!
//! 本面**只读感知面 + 告警输出**: 零设备动作、零执行词表 (闭合动作词表只有
//! `list_rules` / `watch` / `list_alerts`)。看到异常 ≠ 动手 —— 动手仍走
//! `tool.device_control` 三拍人批 (plan → preview → execute, 不可逆零自主),
//! 守护告警**不触发任何设备动作**。
//!
//! # 治理 (开关依赖在场感知族)
//!
//! 注册即闸 ([`GuardWatchConfig::from_env`], 开关
//! [`GUARD_WATCH_ENABLE_ENV`] **依赖** [`DEVICE_SENSE_ENABLE_ENV`] —— 两闸齐开
//! 才有守护面, 默认关 fail-closed)。权力面 = 只读设备表/适配器 + append 告警账,
//! grant 即用 (只读档)。
//!
//! # 0 装诚实
//!
//! - 时序条件的历史观测是**进程内值** ([`GuardWatchActivity`], 重启清零):
//!   `sustain_minutes > 0` 的规则重启后重新累计, 如实;
//! - mock 适配器 fixture 无门磁读数 —— 门磁规则对 mock 设备 =
//!   [`GuardMatch::Unknown`] (不告警不编造), 真实协议适配器后插。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::CapabilityId;
use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc, StoredDocError};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::adapter::{AdapterRegistry, DeviceState};
use super::registry::{DeviceRegistry, DevicesBody};
use super::sense::{device_sense_enabled_from_env, DEVICE_SENSE_ENABLE_ENV};
use crate::exec_pipeline::{AroundPolicy, OutputSchema, SchemaKind, DEFAULT_MAX_TIMEOUT_MS};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const GUARD_WATCH_CAPABILITY_ID: &str = "tool.guard_watch";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const GUARD_WATCH_TOOL_NAME: &str = "guard_watch";

/// 生产执行时限 (毫秒): 守护巡检是设备表读 + 适配器读 + 告警账追加, 到期即归
/// `timeout.*`。
pub const GUARD_WATCH_TIMEOUT_MS: u64 = 5_000;

/// 守护面总开关 (`APEIRETH_ENABLE_GUARD_WATCH`): 注册即闸, 默认关 fail-closed。
///
/// 只认显式 `"1"` (前后空白容忍); 未设 / `"0"` / 其它任何取值 = 关。**依赖
/// 在场感知族总闸** [`DEVICE_SENSE_ENABLE_ENV`]: 感知闸关 = 守护面关
/// (单开本闸也不起, fail-closed)。
pub const GUARD_WATCH_ENABLE_ENV: &str = "APEIRETH_ENABLE_GUARD_WATCH";

/// 守护规则文件 (数据目录落点: `<data_dir>/guard-rules.json`)。
pub const GUARD_RULES_FILE: &str = "guard-rules.json";

/// 守护规则 StoredDoc 的身份戳 (串档打开即拒)。
pub const GUARD_RULES_DOC_NAME: &str = "guard-rules";

/// 守护规则 StoredDoc 的格式版本戳 (读端只认当前版本)。
pub const GUARD_RULES_DOC_VERSION: u32 = 1;

/// 守护告警账文件 (数据目录落点: `<data_dir>/guard-alerts.jsonl`)。
pub const GUARD_ALERTS_FILE: &str = "guard-alerts.jsonl";

/// 守护告警账 StoredDoc 的身份戳。
pub const GUARD_ALERTS_DOC_NAME: &str = "guard-alerts";

/// 守护告警账 StoredDoc 的格式版本戳。
pub const GUARD_ALERTS_DOC_VERSION: u32 = 1;

/// 守护告警标注 (打扰纪律守护例外档破勿扰必带)。
pub const GUARD_ALERT_ANNOTATION: &str = "守护告警";

/// 动作: 列守护规则 (纯读)。
pub const ACTION_LIST_RULES: &str = "list_rules";
/// 动作: 巡检一次 (读状态 → 判定 → 告警入账; **零设备动作**)。
pub const ACTION_WATCH: &str = "watch";
/// 动作: 读告警账 (已产出的告警真值)。
pub const ACTION_LIST_ALERTS: &str = "list_alerts";

/// 闭合动作词表 (list_rules / watch / list_alerts) —— **零执行词表** (看到 ≠
/// 动手: 本面没有任何设备动作)。
pub const GUARD_WATCH_ACTIONS: &[&str] = &[ACTION_LIST_RULES, ACTION_WATCH, ACTION_LIST_ALERTS];

/// 严重度闭合词表: `warn` / `critical`。
pub const SEVERITY_WARN: &str = "warn";
/// 严重度闭合词表: `critical`。
pub const SEVERITY_CRITICAL: &str = "critical";

/// 规则 id 校验: 闭合字符集 `[A-Za-z0-9._-]`, 1..=[`GUARD_RULE_ID_MAX_BYTES`]。
pub const GUARD_RULE_ID_MAX_BYTES: usize = 64;

/// 告警建议语上限 (字节, 与关怀素材同界)。
pub const GUARD_MESSAGE_MAX_BYTES: usize = 2_000;

/// 守护条件 (闭合词表: 温度阈值 / 门磁 / 异常离线)。
///
/// 判定边界 (阈值矩阵语义, 严格不等): `temperature_above` = 读数 **>** 阈值;
/// `temperature_below` = 读数 **<** 阈值; 恰好等于阈值 = 不命中 (如实)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuardCondition {
    /// 温度上阈值: `temperature_c` 读数 > celsius 命中。
    TemperatureAbove {
        /// 温度阈值 (摄氏)。
        celsius: f64,
    },
    /// 温度下阈值: `temperature_c` 读数 < celsius 命中。
    TemperatureBelow {
        /// 温度阈值 (摄氏)。
        celsius: f64,
    },
    /// 门磁: `door_open` 读数 == open 命中 (open=true = 门开告警)。
    DoorOpen {
        /// 门磁目标态。
        open: bool,
    },
    /// 异常离线: 设备 `online == false` 命中。
    Offline,
}

impl GuardCondition {
    /// 稳定条件词 (落盘 / 工具结果 / 自述共用)。
    pub fn label(&self) -> String {
        match self {
            Self::TemperatureAbove { celsius } => format!("temperature_above({celsius})"),
            Self::TemperatureBelow { celsius } => format!("temperature_below({celsius})"),
            Self::DoorOpen { open } => format!("door_open({open})"),
            Self::Offline => "offline".to_string(),
        }
    }

    /// 对一次设备读数做条件判定 (纯判定, 缺读数 = [`GuardMatch::Unknown`])。
    pub fn evaluate(&self, state: &DeviceState) -> GuardMatch {
        match *self {
            Self::TemperatureAbove { celsius } => match reading_f64(state, "temperature_c") {
                Some(value) => GuardMatch::from_bool(value > celsius),
                None => GuardMatch::unknown("temperature_c 读数缺位, 不编造"),
            },
            Self::TemperatureBelow { celsius } => match reading_f64(state, "temperature_c") {
                Some(value) => GuardMatch::from_bool(value < celsius),
                None => GuardMatch::unknown("temperature_c 读数缺位, 不编造"),
            },
            Self::DoorOpen { open } => match reading_bool(state, "door_open") {
                Some(value) => GuardMatch::from_bool(value == open),
                None => GuardMatch::unknown("door_open 读数缺位, 不编造"),
            },
            Self::Offline => GuardMatch::from_bool(!state.online),
        }
    }
}

/// 条件判定结果 (三态: 命中 / 不命中 / 无法判定)。
///
/// [`GuardMatch::Unknown`] **永不告警** (缺读数不编造, 如实留因)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardMatch {
    /// 条件命中。
    Met,
    /// 条件不命中。
    NotMet,
    /// 无法判定 (读数缺位; 不告警不编造)。
    Unknown {
        /// 真因。
        reason: String,
    },
}

impl GuardMatch {
    fn from_bool(met: bool) -> Self {
        if met {
            Self::Met
        } else {
            Self::NotMet
        }
    }

    fn unknown(reason: impl Into<String>) -> Self {
        Self::Unknown {
            reason: reason.into(),
        }
    }

    /// 是否命中 (Unknown 不算命中)。
    pub fn is_met(&self) -> bool {
        matches!(self, Self::Met)
    }
}

fn reading_f64(state: &DeviceState, key: &str) -> Option<f64> {
    state.readings.get(key).and_then(Value::as_f64)
}

fn reading_bool(state: &DeviceState, key: &str) -> Option<bool> {
    state.readings.get(key).and_then(Value::as_bool)
}

/// 生效时段 (时序条件: 只在时段内巡检告警; 跨午夜 = 起 > 止)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardWindow {
    /// 时段起 (分钟 of day, 0..=1439)。
    pub start_minutes: u32,
    /// 时段止 (分钟 of day; 跨午夜 = 起 > 止)。
    pub end_minutes: u32,
}

impl GuardWindow {
    /// 时段校验 (坏配置拒开也拒写)。
    pub fn validate(&self) -> Result<(), String> {
        for (label, minutes) in [
            ("start_minutes", self.start_minutes),
            ("end_minutes", self.end_minutes),
        ] {
            if minutes >= 24 * 60 {
                return Err(format!(
                    "guard window {label} must be 0..=1439, got {minutes}"
                ));
            }
        }
        Ok(())
    }

    /// 分钟是否在时段内 (跨午夜 = 起 > 止, 与勿扰窗口同口径)。
    pub fn contains(&self, minutes_of_day: u32) -> bool {
        if self.start_minutes <= self.end_minutes {
            minutes_of_day >= self.start_minutes && minutes_of_day < self.end_minutes
        } else {
            minutes_of_day >= self.start_minutes || minutes_of_day < self.end_minutes
        }
    }
}

/// 一次守护观测 (时序条件的历史输入; 进程内值, 重启清零)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardObservation {
    /// 设备 id。
    pub device_id: String,
    /// 观测时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 该时刻条件是否命中 (Unknown 记 `false` 并注因, 不编造)。
    pub met: bool,
}

/// 连续命中窗口判定 (时序条件, 纯判定):
///
/// `sustain_minutes == 0` = 单帧命中即告警; `> 0` = 窗口内观测**全部命中**且
/// 历史跨度 ≥ 窗口 (才算连续满足 N 分钟)。
pub fn condition_sustained(
    history: &[GuardObservation],
    device_id: &str,
    now_ms: i64,
    sustain_minutes: u32,
) -> bool {
    if sustain_minutes == 0 {
        return true;
    }
    let window_start = now_ms - i64::from(sustain_minutes) * 60_000;
    let window: Vec<&GuardObservation> = history
        .iter()
        .filter(|obs| {
            obs.device_id == device_id && obs.at_ms >= window_start && obs.at_ms <= now_ms
        })
        .collect();
    if window.is_empty() {
        return false;
    }
    window.iter().all(|obs| obs.met)
        && now_ms - window[0].at_ms >= i64::from(sustain_minutes) * 60_000
}

/// 一条守护规则 (阈值条件 + 时序条件)。
///
/// 字段合同 (稳定, 未知字段拒开): `{id, device_id, condition, sustain_minutes,
/// active_window, severity, message, enabled, created_at_ms}`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardRule {
    /// 规则 id (闭合字符集, 见 [`validate_guard_rule_id`])。
    pub id: String,
    /// 目标设备 id (巡检时核白名单: 表外/停用 = 跳过不编造读数)。
    pub device_id: String,
    /// 阈值条件 (闭合词表)。
    pub condition: GuardCondition,
    /// 时序条件①: 连续命中窗口 (分钟; 0 = 单帧命中即告警)。
    #[serde(default)]
    pub sustain_minutes: u32,
    /// 时序条件②: 生效时段 (`None` = 全天)。
    #[serde(default)]
    pub active_window: Option<GuardWindow>,
    /// 严重度 (`warn` / `critical`)。
    pub severity: String,
    /// 告警建议语 (由提醒侧消费端转关怀候选素材)。
    pub message: String,
    /// 是否启用 (关 = 不参与巡检, 留档不删)。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 入表时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
}

fn default_enabled() -> bool {
    true
}

impl GuardRule {
    /// 参数闭合校验 (装载与写出同口径, 坏条目拒开也拒写)。
    pub fn validate(&self) -> Result<(), String> {
        validate_guard_rule_id(&self.id)?;
        if self.device_id.trim().is_empty() {
            return Err(format!("guard rule {} 的 device_id 不能为空", self.id));
        }
        if !matches!(self.severity.as_str(), SEVERITY_WARN | SEVERITY_CRITICAL) {
            return Err(format!(
                "guard rule {} 的 severity 必须是 {SEVERITY_WARN} / {SEVERITY_CRITICAL}, got {:?}",
                self.id, self.severity
            ));
        }
        validate_guard_message(&self.message)?;
        if let Some(window) = self.active_window {
            window.validate()?;
        }
        Ok(())
    }

    /// 对一次读数 + 时序历史做整条规则判定 (纯判定):
    /// 生效时段 → 阈值条件 → 连续命中窗口。
    pub fn evaluate(
        &self,
        state: &DeviceState,
        history: &[GuardObservation],
        now_ms: i64,
    ) -> GuardMatch {
        if let Some(window) = self.active_window {
            let minutes_of_day = minutes_of_day_from_epoch_ms(now_ms);
            if !window.contains(minutes_of_day) {
                return GuardMatch::NotMet;
            }
        }
        match self.condition.evaluate(state) {
            GuardMatch::Met => {
                if condition_sustained(history, &self.device_id, now_ms, self.sustain_minutes) {
                    GuardMatch::Met
                } else {
                    GuardMatch::unknown(format!(
                        "连续命中窗口 {} 分钟未满 (时序条件未到点)",
                        self.sustain_minutes
                    ))
                }
            }
            other => other,
        }
    }
}

/// epoch 毫秒 → 当天分钟 (UTC; 与关怀纪律同口径)。
fn minutes_of_day_from_epoch_ms(at_ms: i64) -> u32 {
    let day_seconds = at_ms.div_euclid(86_400_000);
    let since_day = at_ms - day_seconds * 86_400_000;
    (since_day.div_euclid(60_000) as u32).min(1439)
}

/// 规则 id 校验: 闭合字符集 `[A-Za-z0-9._-]`, 1..=[`GUARD_RULE_ID_MAX_BYTES`]。
pub fn validate_guard_rule_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("guard rule id must not be empty".to_string());
    }
    if raw.len() > GUARD_RULE_ID_MAX_BYTES {
        return Err(format!(
            "guard rule id is {} bytes; the maximum is {GUARD_RULE_ID_MAX_BYTES}",
            raw.len()
        ));
    }
    if !raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err("guard rule id must match [A-Za-z0-9._-] only".to_string());
    }
    Ok(())
}

/// 告警建议语校验: 非空、有界、无控制符 (进关怀回合输入)。
pub fn validate_guard_message(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("guard alert message must not be empty".to_string());
    }
    if raw.len() > GUARD_MESSAGE_MAX_BYTES {
        return Err(format!(
            "guard alert message is {} bytes; the maximum is {GUARD_MESSAGE_MAX_BYTES}",
            raw.len()
        ));
    }
    if raw.chars().any(char::is_control) {
        return Err("guard alert message must not contain control characters".to_string());
    }
    Ok(())
}

/// 守护规则表主体 (`guard-rules.json` 的 StoredDoc body)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardRulesBody {
    /// 规则列表 (有序; 缺省 = 空表, 无守护规则不编造告警)。
    pub rules: Vec<GuardRule>,
}

impl GuardRulesBody {
    /// 空表 (缺省态)。
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 表体校验 (纯判定): 重复 id / 坏条目 = 拒开拒写。
    pub fn validate(&self) -> Result<(), GuardStoreError> {
        let mut seen = std::collections::BTreeSet::new();
        for rule in &self.rules {
            rule.validate().map_err(GuardStoreError::Refused)?;
            if !seen.insert(rule.id.clone()) {
                return Err(GuardStoreError::Refused(format!(
                    "guard rule id 重复: {}",
                    rule.id
                )));
            }
        }
        Ok(())
    }
}

/// 守护规则表存储面错误 (闭合词表: 拒开 / 坏表)。
#[derive(Debug)]
pub enum GuardStoreError {
    /// `guard-rules.json` 存在但拒开 (StoredDoc 语义: 不静默修复)。
    Stored(StoredDocError),
    /// 规则条目坏 (拒开也拒写)。
    Refused(String),
}

impl std::fmt::Display for GuardStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stored(error) => write!(f, "守护规则表拒开: {error}"),
            Self::Refused(reason) => write!(f, "守护规则坏 (拒开也拒写): {reason}"),
        }
    }
}

impl std::error::Error for GuardStoreError {}

impl From<StoredDocError> for GuardStoreError {
    fn from(error: StoredDocError) -> Self {
        Self::Stored(error)
    }
}

/// 守护规则读端契约 (仅当前格式版本可读, 其余拒开)。
fn guard_rules_compat() -> DocCompat {
    DocCompat::exact(GUARD_RULES_DOC_NAME, GUARD_RULES_DOC_VERSION)
}

/// 守护规则表: `<data_dir>/guard-rules.json` 的读写面 (StoredDoc 拒开语义,
/// 与 `devices.json` 同族)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRuleStore {
    path: PathBuf,
}

impl GuardRuleStore {
    /// 数据目录落点 (`<data_dir>/guard-rules.json`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(GUARD_RULES_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 守护规则表路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 装载规则表:
    ///
    /// - 文件缺失 = [`GuardRulesBody::new`] (空表, 无规则不编造告警);
    /// - 文件存在但坏 (畸形 / 串档 / 未来版本 / 坏条目) = `Err` (拒开, 不回退)。
    pub fn load(&self) -> Result<GuardRulesBody, GuardStoreError> {
        if !self.path.exists() {
            return Ok(GuardRulesBody::new());
        }
        let doc = apeireth_core::stored_doc::open_single::<GuardRulesBody>(
            &self.path,
            &guard_rules_compat(),
        )?;
        doc.body.validate()?;
        Ok(doc.body)
    }

    /// 落盘规则表 (持久档原子写): 先校验后落盘, 坏表拒写 (不带病落盘)。
    pub fn save(&self, body: &GuardRulesBody) -> Result<(), GuardStoreError> {
        body.validate()?;
        apeireth_core::stored_doc::save_single(
            &self.path,
            &guard_rules_compat(),
            body,
            apeireth_core::stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }
}

/// 一条守护告警 (命中产出; 提醒侧消费端转关怀候选)。
///
/// 字段合同 (稳定, 未知字段拒开): `{alert_id, rule_id, device_id, kind,
/// condition, severity, message, evidence, source, source_data, created_at_ms,
/// annotation}`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardAlert {
    /// 告警稳定身份 (`gwalert-<时刻毫秒>-<进程内序号>`)。
    pub alert_id: String,
    /// 命中的规则 id。
    pub rule_id: String,
    /// 命中的设备 id。
    pub device_id: String,
    /// 信号族 (恒 `guard`)。
    pub kind: String,
    /// 条件词 (阈值描述)。
    pub condition: String,
    /// 严重度 (`warn` / `critical`)。
    pub severity: String,
    /// 告警建议语。
    pub message: String,
    /// 数据依据 (读数的有界摘要)。
    pub evidence: String,
    /// 信号来源工具 (`tool.guard_watch`)。
    pub source: String,
    /// 数据源标注 (适配器如实: `mock-fixture` / `live:<适配器名>`)。
    pub source_data: String,
    /// 告警时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 标注 (恒 [`GUARD_ALERT_ANNOTATION`]: 打扰纪律守护例外档破勿扰必带)。
    pub annotation: String,
}

/// 告警身份 (`gwalert-<时刻毫秒>-<进程内序号>`)。
pub fn guard_alert_id(created_at_ms: i64, ordinal: u64) -> String {
    format!("gwalert-{created_at_ms}-{ordinal}")
}

/// 进程内告警序号 (身份唯一性用, 非安全边界)。
static ALERT_ORDINAL: AtomicU64 = AtomicU64::new(1);

/// 告警账 (`guard-alerts.jsonl`, append-only, 逐行一个 [`StoredDoc`] 信封)。
///
/// 提醒侧消费端 (care 同源接缝) 从本账消费告警; 坏行读作不存在 (跳过并计入
/// `skipped`), 不砖化整本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardAlertLedger {
    path: PathBuf,
}

impl GuardAlertLedger {
    /// 数据目录落点 (`<data_dir>/guard-alerts.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(GUARD_ALERTS_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 告警账路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行告警 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, alert: &GuardAlert) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("告警账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            GUARD_ALERTS_DOC_NAME,
            GUARD_ALERTS_DOC_VERSION,
            vec![GUARD_ALERTS_DOC_VERSION],
            alert,
        );
        // ⚠️ **2026-10-27 修同族缺陷**：原写法**两次写**（行体 + 换行）
        // ⇒ 并发写者会把两行挤成一行 ⇒ **告警记录丢失**。
        // ⇒ 改用**共用原语**（行体与行尾换行同在一次写）。
        crate::proposal::append_jsonl_row(&self.path, "告警账", &doc)
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<GuardAlert>, String> {
        scan_records::<GuardAlert>(&self.path, &alert_ledger_compat())
            .map_err(|error| format!("告警账读取失败 {}: {error}", self.path.display()))
    }
}

/// 告警账读端契约 (仅当前格式版本可读)。
fn alert_ledger_compat() -> DocCompat {
    DocCompat::exact(GUARD_ALERTS_DOC_NAME, GUARD_ALERTS_DOC_VERSION)
}

/// 守护巡检的进程内计数与时序历史 (自述/测试真值; 重启清零, 如实)。
#[derive(Debug, Default)]
pub struct GuardWatchActivity {
    watch_calls: AtomicU64,
    alerts_emitted: AtomicU64,
    observations: Mutex<Vec<GuardObservation>>,
    last: Mutex<Option<(i64, String)>>,
}

/// 时序历史有界条数 (防无界增长; 时序窗口只看窗口内)。
const GUARD_OBSERVATION_MAX_COUNT: usize = 512;

impl GuardWatchActivity {
    /// 计数器 (进程内)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 记一次观测 (时序条件历史; 有界)。
    pub fn record_observation(&self, observation: GuardObservation) {
        let mut observations = self
            .observations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        observations.push(observation);
        while observations.len() > GUARD_OBSERVATION_MAX_COUNT {
            observations.remove(0);
        }
    }

    /// 取历史观测 (顺序保持)。
    pub fn observations(&self) -> Vec<GuardObservation> {
        self.observations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 记一次巡检 (时刻 + 有界摘要)。
    pub fn record_watch(&self, at_ms: i64, note: impl Into<String>) {
        self.watch_calls.fetch_add(1, Ordering::Relaxed);
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *last = Some((at_ms, note.into()));
    }

    /// 记一次告警产出。
    pub fn record_alert(&self) {
        self.alerts_emitted.fetch_add(1, Ordering::Relaxed);
    }

    /// 巡检累计 (进程内)。
    pub fn watch_calls(&self) -> u64 {
        self.watch_calls.load(Ordering::Relaxed)
    }

    /// 告警累计 (进程内)。
    pub fn alerts_emitted(&self) -> u64 {
        self.alerts_emitted.load(Ordering::Relaxed)
    }

    /// 最近巡检 (时刻 + 摘要; `None` = 尚无巡检)。
    pub fn last_watch(&self) -> Option<(i64, String)> {
        self.last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// 守护工具 (`tool.guard_watch`, 声明名 `guard_watch`) —— **只看不动**。
///
/// `list_rules` 列守护规则 (StoredDoc 拒开即帧); `watch` 巡检一次
/// (白名单设备读状态 → 规则判定 → 命中告警入账); `list_alerts` 读告警账。
/// 闭合动作词表**零执行词表**: 本面不触发任何设备动作。
pub struct GuardWatchTool {
    id: CapabilityId,
    registry: DeviceRegistry,
    rules: GuardRuleStore,
    alerts: GuardAlertLedger,
    adapters: AdapterRegistry,
    activity: Arc<GuardWatchActivity>,
}

impl GuardWatchTool {
    /// 绑定设备表 / 守护规则表 / 告警账 / 适配器注册表 / 进程内计数。
    pub fn new(
        registry: DeviceRegistry,
        rules: GuardRuleStore,
        alerts: GuardAlertLedger,
        adapters: AdapterRegistry,
        activity: Arc<GuardWatchActivity>,
    ) -> Self {
        Self {
            id: CapabilityId::new(GUARD_WATCH_CAPABILITY_ID).expect("valid capability id"),
            registry,
            rules,
            alerts,
            adapters,
            activity,
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 进程内巡检计数 (测试/诊断真值)。
    pub fn activity(&self) -> Arc<GuardWatchActivity> {
        Arc::clone(&self.activity)
    }

    fn list_rules(&self, call: &ToolCall) -> ToolResult {
        let body = match self.rules.load() {
            Ok(body) => body,
            Err(error) => {
                return self.refuse(call, format!("守护规则表拒开 (不回退默认表): {error}"));
            }
        };
        let rows: Vec<Value> = body
            .rules
            .iter()
            .map(|rule| {
                json!({
                    "id": rule.id,
                    "device_id": rule.device_id,
                    "condition": rule.condition.label(),
                    "sustain_minutes": rule.sustain_minutes,
                    "active_window": rule.active_window,
                    "severity": rule.severity,
                    "message": rule.message,
                    "enabled": rule.enabled,
                    "created_at_ms": rule.created_at_ms,
                })
            })
            .collect();
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_LIST_RULES,
                "status": "ok",
                "rules": rows,
                "rules_total": body.rules.len(),
                "table_present": self.rules.path().exists(),
                "note": "守护规则面: 规则 = 设备状态阈值 / 时序条件; 坏表拒开 (StoredDoc 语义)",
            }),
        )
        .with_name(GUARD_WATCH_TOOL_NAME)
    }

    fn watch(&self, call: &ToolCall) -> ToolResult {
        let body = match self.rules.load() {
            Ok(body) => body,
            Err(error) => {
                return self.refuse(call, format!("守护规则表拒开 (不回退默认表): {error}"));
            }
        };
        let devices: DevicesBody = match self.registry.load() {
            Ok(devices) => devices,
            Err(error) => {
                return self.refuse(call, format!("设备表拒开 (不回退默认表): {error}"));
            }
        };
        let now_ms = now_epoch_ms();

        // 白名单核对 (只读消费同一本设备表): 表外/停用设备的规则跳过, 不编读数。
        let mut skipped: Vec<Value> = Vec::new();
        let mut targets: Vec<(&GuardRule, &super::registry::Device)> = Vec::new();
        for rule in body.rules.iter().filter(|rule| rule.enabled) {
            match devices.find(&rule.device_id) {
                Some(device) if device.enabled => targets.push((rule, device)),
                Some(_) => skipped.push(json!({
                    "rule_id": rule.id,
                    "reason": "设备已登记但未启用 (白名单外, enabled=false), 跳过不编造读数",
                })),
                None => skipped.push(json!({
                    "rule_id": rule.id,
                    "reason": "设备不在设备表上 (白名单制默认零设备), 跳过不编造读数",
                })),
            }
        }

        // 适配器读 (只读感知面): 插槽未接线 / 读失败 = 整帧拒绝即帧 (真因如实)。
        let mut states: BTreeMap<String, DeviceState> = BTreeMap::new();
        for (_rule, device) in &targets {
            if states.contains_key(&device.id) {
                continue;
            }
            let adapter = match self.adapters.resolve(&device.adapter) {
                Ok(adapter) => adapter,
                Err(error) => {
                    return self.refuse(
                        call,
                        format!("拒绝巡检: 设备 {} 的适配器不可用: {error}", device.id),
                    );
                }
            };
            match adapter.read_state(device, now_ms) {
                Ok(state) => {
                    states.insert(device.id.clone(), state);
                }
                Err(error) => {
                    return self.refuse(
                        call,
                        format!("拒绝巡检: 设备 {} 读失败: {error}", device.id),
                    );
                }
            }
        }

        // 判定 (纯判定: 生效时段 → 阈值 → 时序) + 命中告警入账。
        let history = self.activity.observations();
        let mut alerts: Vec<Value> = Vec::new();
        let mut unknowns: Vec<Value> = Vec::new();
        for (rule, device) in &targets {
            let Some(state) = states.get(&device.id) else {
                continue;
            };
            // 时序历史记**阈值条件**逐帧判定 (窗口内全命中才算连续, 不被最终
            // 裁决档污染)。
            self.activity.record_observation(GuardObservation {
                device_id: device.id.clone(),
                at_ms: now_ms,
                met: rule.condition.evaluate(state).is_met(),
            });
            let outcome = rule.evaluate(state, &history, now_ms);
            match outcome {
                GuardMatch::Met => {
                    let alert = GuardAlert {
                        alert_id: guard_alert_id(
                            now_ms,
                            ALERT_ORDINAL.fetch_add(1, Ordering::Relaxed),
                        ),
                        rule_id: rule.id.clone(),
                        device_id: device.id.clone(),
                        kind: "guard".to_string(),
                        condition: rule.condition.label(),
                        severity: rule.severity.clone(),
                        message: rule.message.clone(),
                        evidence: format!(
                            "{} ({}) online={} readings={:?}",
                            device.id,
                            rule.condition.label(),
                            state.online,
                            state.readings
                        ),
                        source: GUARD_WATCH_CAPABILITY_ID.to_string(),
                        source_data: state.source_note.clone(),
                        created_at_ms: now_ms,
                        annotation: GUARD_ALERT_ANNOTATION.to_string(),
                    };
                    if let Err(error) = self.alerts.append(&alert) {
                        // 告警账写不进 = 整帧拒绝即帧 (fail-closed: 告警必须留痕)。
                        return self.refuse(
                            call,
                            format!("拒绝巡检 (告警缺位不放行): 告警入账失败: {error}"),
                        );
                    }
                    self.activity.record_alert();
                    alerts.push(json!(alert));
                }
                GuardMatch::Unknown { reason } => {
                    unknowns.push(json!({
                        "rule_id": rule.id,
                        "device_id": device.id,
                        "reason": reason,
                    }));
                }
                GuardMatch::NotMet => {}
            }
        }

        let note = if alerts.is_empty() {
            "只看不动: 本次巡检零命中 (或无规则), 零设备动作 —— 守护只读感知面 + 告警输出,\
             动手仍走 tool.device_control 三拍人批"
                .to_string()
        } else {
            format!(
                "只看不动: {} 条守护告警已入账 (关怀候选高优先, 打扰纪律守护例外档);\
                 零设备动作 —— 看到≠动手, 动手仍走 tool.device_control 三拍人批",
                alerts.len()
            )
        };
        let at_ms = now_epoch_ms();
        self.activity.record_watch(at_ms, note.clone());
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_WATCH,
                "status": "ok",
                "rules_evaluated": targets.len(),
                "rules_skipped": skipped,
                "devices_read": states.len(),
                "alerts": alerts,
                "alerts_emitted": alerts.len(),
                "unknown": unknowns,
                "annotation": GUARD_ALERT_ANNOTATION,
                "note": note,
            }),
        )
        .with_name(GUARD_WATCH_TOOL_NAME)
    }

    fn list_alerts(&self, call: &ToolCall) -> ToolResult {
        let scan = match self.alerts.scan() {
            Ok(scan) => scan,
            Err(error) => return self.refuse(call, format!("告警账读取失败: {error}")),
        };
        let recent: Vec<&GuardAlert> = scan
            .records
            .iter()
            .map(|record| &record.body)
            .rev()
            .take(GUARD_ALERTS_RECENT_LIMIT)
            .collect();
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_LIST_ALERTS,
                "status": "ok",
                "total": scan.records.len(),
                "skipped_bad_rows": scan.skipped,
                "recent": recent,
                "note": "告警账 (append-only): 告警 = 关怀候选高优先; 零设备动作 (只看不动)",
            }),
        )
        .with_name(GUARD_WATCH_TOOL_NAME)
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, reason: impl Into<String>) -> ToolResult {
        ToolResult::permanent_error(&call.id, reason.into()).with_name(GUARD_WATCH_TOOL_NAME)
    }
}

/// 告警账「最近告警」的读回上限 (条)。
pub const GUARD_ALERTS_RECENT_LIMIT: usize = 5;

#[async_trait]
impl ToolCapability for GuardWatchTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": GUARD_WATCH_ACTIONS,
                        "description": "list_rules reads the guard rules; watch runs one sensing pass (whitelist device states -> rule evaluation -> alerts); list_alerts reads the alert ledger. This face is read-only: it never performs a device action."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(GUARD_WATCH_TOOL_NAME)
            .with_description(
                "Home-guard watch face (K-axis L5). Guard rules (guard-rules.json, strict \
                 stored document like devices.json) pair a device-state threshold (temperature \
                 bounds / door sensor / abnormal offline) with timing conditions (sustained \
                 window + active time window). A hit emits an alert into guard-alerts.jsonl \
                 (append-only) which the reminder side consumes as a high-priority care \
                 candidate under the guard exception tier of the disturbance discipline. \
                 This face only senses and alerts - seeing is not acting: it never performs a \
                 device action; acting still goes through tool.device_control's three-beat \
                 human approval.",
            )
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<GuardWatchParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(call, format!("invalid guard_watch parameters: {error}"));
            }
        };
        let Some(action) = params.action.as_deref() else {
            return self.refuse(
                call,
                "拒绝守护巡检: 必须显式给 action (list_rules / watch / list_alerts)",
            );
        };
        match action {
            ACTION_LIST_RULES => self.list_rules(call),
            ACTION_WATCH => self.watch(call),
            ACTION_LIST_ALERTS => self.list_alerts(call),
            other => self.refuse(
                call,
                format!(
                    "unknown guard_watch action {other:?}; the closed vocabulary is \
                     list_rules / watch / list_alerts (零执行词表: 本面不触发设备动作)"
                ),
            ),
        }
    }
}

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardWatchParams {
    action: Option<String>,
}

// ---------------------------------------------------------------------------
// 配置 (注册即闸, 依赖在场感知族总闸, 默认关 fail-closed)
// ---------------------------------------------------------------------------

/// 守护面配置 (设备表 / 规则表 / 告警账落位)。
///
/// 组装根按 [`GuardWatchConfig::from_env`] 注入 (注册即闸): 缺配置 =
/// `tool.guard_watch` 不注册, 自述名册 `guard_watch` 行照实 false。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardWatchConfig {
    /// 设备表 / 规则表 / 告警账所在数据目录。
    pub data_dir: PathBuf,
}

impl GuardWatchConfig {
    /// 落位 = 数据目录。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// 本配置的设备表 (只读消费, 与 L1 同一本)。
    pub fn registry(&self) -> DeviceRegistry {
        DeviceRegistry::under_data_dir(&self.data_dir)
    }

    /// 本配置的守护规则表。
    pub fn rules(&self) -> GuardRuleStore {
        GuardRuleStore::under_data_dir(&self.data_dir)
    }

    /// 本配置的告警账。
    pub fn alerts(&self) -> GuardAlertLedger {
        GuardAlertLedger::under_data_dir(&self.data_dir)
    }

    /// 开关实况: 双闸齐开才 `true` (fail-closed)。
    pub fn enabled_from_env() -> bool {
        guard_watch_enabled_from_env()
    }

    /// 从 env 构造: 双闸齐开才 `Some` (默认关 → [`None`], fail-closed)。
    pub fn from_env(data_dir: impl Into<PathBuf>) -> Option<Self> {
        guard_watch_enabled_from_env().then(|| Self::new(data_dir))
    }
}

/// 守护面开关 (`APEIRETH_ENABLE_GUARD_WATCH`) 实况: **依赖在场感知族总闸**。
///
/// [`DEVICE_SENSE_ENABLE_ENV`] 与 [`GUARD_WATCH_ENABLE_ENV`] 都显式 `"1"` 才开;
/// 其余任何取值 = 关 (fail-closed: 守护面挂在感知族之下, 感知闸关 = 守护面关)。
pub fn guard_watch_enabled_from_env() -> bool {
    device_sense_enabled_from_env()
        && std::env::var(GUARD_WATCH_ENABLE_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1")
}

/// 组装守护面五段流水线: 只读档 → 超时 → 输出归一合同。
pub fn guard_watch_pipeline() -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new().with_around(
            AroundPolicy::new().with_timeout(GUARD_WATCH_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
        ),
    )
}

/// 组装守护工具面 (模型侧一个能力; 适配器 = 内置 mock, 真实协议族后插)。
pub fn guard_watch_capability(config: GuardWatchConfig) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> = Arc::new(GuardWatchTool::new(
        config.registry(),
        config.rules(),
        config.alerts(),
        AdapterRegistry::builtin(),
        Arc::new(GuardWatchActivity::new()),
    ));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        guard_watch_pipeline(),
    ))
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::registry::DeviceKind;

    fn state(readings: BTreeMap<String, Value>, online: bool) -> DeviceState {
        DeviceState {
            device_id: "thermo-1".to_string(),
            kind: DeviceKind::Sensor,
            adapter: "mock".to_string(),
            online,
            readings,
            observed_at_ms: 1_700_000_000_000,
            source_note: "mock-fixture".to_string(),
        }
    }

    fn readings(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    /// ① 阈值矩阵: 温度上/下限严格不等 + 恰等边界 + 缺读数三态。
    #[test]
    fn the_temperature_threshold_matrix_is_strict_and_honest_about_missing_readings() {
        let above = GuardCondition::TemperatureAbove { celsius: 30.0 };
        let below = GuardCondition::TemperatureBelow { celsius: 5.0 };
        let hot = state(readings(&[("temperature_c", Value::from(30.1))]), true);
        let exactly = state(readings(&[("temperature_c", Value::from(30.0))]), true);
        let cold = state(readings(&[("temperature_c", Value::from(4.9))]), true);
        let mild = state(readings(&[("temperature_c", Value::from(21.5))]), true);
        let missing = state(readings(&[]), true);

        assert_eq!(above.evaluate(&hot), GuardMatch::Met, "高于上阈值命中");
        assert_eq!(
            above.evaluate(&exactly),
            GuardMatch::NotMet,
            "恰等不命中 (严格不等)"
        );
        assert_eq!(above.evaluate(&mild), GuardMatch::NotMet);
        assert_eq!(below.evaluate(&cold), GuardMatch::Met, "低于下阈值命中");
        assert_eq!(below.evaluate(&exactly), GuardMatch::NotMet);
        assert!(matches!(
            above.evaluate(&missing),
            GuardMatch::Unknown { .. }
        ));
        assert!(matches!(
            below.evaluate(&missing),
            GuardMatch::Unknown { .. }
        ));
    }

    /// ① 阈值矩阵: 门磁 + 异常离线 + 缺读数三态。
    #[test]
    fn the_door_and_offline_conditions_are_boolean_and_never_guess() {
        let door_open = GuardCondition::DoorOpen { open: true };
        let open = state(readings(&[("door_open", Value::from(true))]), true);
        let closed = state(readings(&[("door_open", Value::from(false))]), true);
        let no_reading = state(readings(&[("temperature_c", Value::from(21.5))]), true);
        assert_eq!(door_open.evaluate(&open), GuardMatch::Met);
        assert_eq!(door_open.evaluate(&closed), GuardMatch::NotMet);
        assert!(matches!(
            door_open.evaluate(&no_reading),
            GuardMatch::Unknown { .. },
        ));

        let offline = GuardCondition::Offline;
        assert_eq!(
            offline.evaluate(&state(readings(&[]), false)),
            GuardMatch::Met
        );
        assert_eq!(
            offline.evaluate(&state(readings(&[]), true)),
            GuardMatch::NotMet
        );
    }

    /// ② 时序条件: 连续命中窗口 (跨度不够 = 不命中; 全程命中 = 命中)。
    #[test]
    fn the_sustained_window_requires_unbroken_history_spanning_the_window() {
        let history = vec![
            GuardObservation {
                device_id: "thermo-1".to_string(),
                at_ms: 1_700_000_000_000,
                met: true,
            },
            GuardObservation {
                device_id: "thermo-1".to_string(),
                at_ms: 1_700_000_000_000 + 4 * 60_000,
                met: true,
            },
        ];
        assert!(
            !condition_sustained(&history, "thermo-1", 1_700_000_000_000 + 4 * 60_000, 5),
            "历史跨度不满 5 分钟 = 未满"
        );
        assert!(
            condition_sustained(&history, "thermo-1", 1_700_000_000_000 + 5 * 60_000, 5),
            "窗口内全程命中且跨度满 5 分钟 = 命中"
        );
        assert!(
            !condition_sustained(&history, "thermo-1", 1_700_000_000_000 + 5 * 60_000, 6),
            "窗口长于历史跨度 = 未满"
        );
        assert!(
            condition_sustained(&[], "thermo-1", 1_700_000_000_000, 0),
            "0 分钟 = 单帧命中即告警"
        );
        let broken = vec![
            GuardObservation {
                device_id: "thermo-1".to_string(),
                at_ms: 1_700_000_000_000,
                met: true,
            },
            GuardObservation {
                device_id: "thermo-1".to_string(),
                at_ms: 1_700_000_000_000 + 2 * 60_000,
                met: false,
            },
        ];
        assert!(
            !condition_sustained(&broken, "thermo-1", 1_700_000_000_000 + 5 * 60_000, 5),
            "窗口内断过 = 不算连续命中"
        );
    }

    /// ② 时序条件: 生效时段 (跨午夜 = 起 > 止, 与勿扰窗口同口径)。
    #[test]
    fn the_active_window_matches_the_quiet_window_semantics() {
        let night = GuardWindow {
            start_minutes: 22 * 60,
            end_minutes: 6 * 60,
        };
        night.validate().expect("合法时段");
        assert!(night.contains(23 * 60) && night.contains(3 * 60));
        assert!(!night.contains(12 * 60));
        let day = GuardWindow {
            start_minutes: 9 * 60,
            end_minutes: 18 * 60,
        };
        assert!(day.contains(12 * 60) && !day.contains(20 * 60));
        assert!(GuardWindow {
            start_minutes: 1440,
            end_minutes: 0
        }
        .validate()
        .is_err());
    }

    /// ③ 规则整条判定: 时段外不命中; 时序未满 = 无法判定 (不告警)。
    #[test]
    fn a_rule_evaluates_window_then_threshold_then_sustain() {
        let rule = GuardRule {
            id: "heat".to_string(),
            device_id: "thermo-1".to_string(),
            condition: GuardCondition::TemperatureAbove { celsius: 30.0 },
            sustain_minutes: 5,
            active_window: Some(GuardWindow {
                start_minutes: 9 * 60,
                end_minutes: 18 * 60,
            }),
            severity: SEVERITY_CRITICAL.to_string(),
            message: "室内过热".to_string(),
            enabled: true,
            created_at_ms: 0,
        };
        rule.validate().expect("合法规则");
        let hot = state(readings(&[("temperature_c", Value::from(35.0))]), true);
        // 时段外 (UTC 0 点 = 00:00): 不命中。
        assert_eq!(
            rule.evaluate(&hot, &[], 1_700_000_000_000),
            GuardMatch::NotMet
        );
        // 时段内 (UTC 12:00) + 时序未满: 无法判定 (不告警)。
        let noon = 1_700_000_000_000 + 12 * 3_600_000;
        assert!(matches!(
            rule.evaluate(&hot, &[], noon),
            GuardMatch::Unknown { .. }
        ));
        // 时段内 + 连续 5 分钟命中: 告警。
        let history = vec![GuardObservation {
            device_id: "thermo-1".to_string(),
            at_ms: noon - 5 * 60_000,
            met: true,
        }];
        assert_eq!(rule.evaluate(&hot, &history, noon), GuardMatch::Met);
    }
}
