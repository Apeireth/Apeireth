//! 提醒侧消费端 (K 轴 L2 提醒 + L5 守护告警的同源接缝): 信号 = 关怀候选的一种。
//!
//! # 同源纪律 (提醒 = 关怀候选)
//!
//! 天气提醒信号 (`tool.weather` 只交信号, 落 `weather-signals.jsonl`) 与
//! 守护告警 (`tool.guard_watch` 只看不动, 落 `guard-alerts.jsonl`) 都经本面
//! 映射成 [`ReminderCandidate`] → [`CareCandidate`], 走**同一套打扰纪律四件**
//! (勿扰窗口 / 专注抑制 / 频率上限 / 关怀预算, 配置同一本 `care_rules.json`
//! [`CareDiscipline`]) —— 提醒不另立纪律, 谁都不许绕过勿扰与频率。
//!
//! # 守护例外档 (打扰纪律的唯一例外, 可配)
//!
//! 安全告警 ([`CareTier::GuardAlert`]) 是打扰纪律的**守护例外档**:
//!
//! - **破勿扰** (默认, 可配 [`CareDiscipline::guard_alert_breaks_quiet`] 关掉):
//!   安全告警不受勿扰窗口拦; 破窗发起**必带「守护告警」标注** (标准回合输入
//!   与台账留痕都在), 人看得见它为什么破窗;
//! - **频率独立上限** (可配 [`CareDiscipline::guard_frequency_max`] /
//!   [`CareDiscipline::guard_frequency_window_hours`]): 守护告警不占普通关怀的
//!   频率预算, 也不被普通频率上限误伤, 自己一档有界 (默认 1 小时 ≤ 3 条);
//! - 专注抑制 / 关怀预算**照常生效** (例外档只破勿扰一项, 不是全豁免)。
//!
//! # 四条安全底线继承 (看到 ≠ 动手)
//!
//! 本面只消费**告警信号** (只读感知面的产出), 零设备动作: 告警走标准回合说
//! 给人听, 动手仍走 `tool.device_control` 三拍人批 (不可逆零自主)。
//!
//! # 0 装诚实
//!
//! - 消费位 (已裁决信号的去重) 双查: 台账 ([`rules::CareRecord`], 落盘) 优先,
//!   信号源自带消费位 ([`ReminderFeed::mark_consumed`], 进程内) 兜底 —— 重启后
//!   已裁决信号因台账在而不重发; 台账有界 (32 条) 之外的极老信号理论上可重
//!   裁决, 由频率上限兜底不刷屏 (如实注明, 不冒充完美去重)。
//! - 拦下 ([`CarePoll::Held`]) = 候选不丢: **不标消费**, 窗口滑过重试。

use std::collections::BTreeSet;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::engine::{CareCandidate, CareEngine, CareInitiative, CarePoll};
use super::rules::{CareRecord, CareStoreError};

/// 候选来源档 (打扰纪律分档的输入标签; 台账 `kind` 列同词表)。
///
/// 既有三档 `rule` / `reminder` / `guard_alert` 语义不变; W3.1 新增两档
/// `ritual` / `reunion` 是生物机制候选源 (SG-22 仪式 / SG-19 分离-重聚),
/// 由 [`super::CareConfig::bio_candidates_enabled`] 总开关控制 (默认关,
/// 关闭时永不产出)。两档与 `reminder` 同走**普通频率账** (不占守护独立上限),
/// 台账经 `record_signal_care` 落 `kind` 列 (不碰规则 `fired_at` 锚点)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CareTier {
    /// 规则触发 (四类触发器到点)。
    Rule,
    /// 提醒信号 (天气提醒等: 只交信号 → 提醒侧消费)。
    Reminder,
    /// 守护告警 (L5 看家: 安全告警 = 高优先关怀候选, 带守护例外档)。
    GuardAlert,
    /// 仪式候选 (SG-22 仪式内核 → care 候选源; 默认关, 见 [`super::CareConfig`])。
    Ritual,
    /// 重聚候选 (SG-19 分离-重聚内核 → care 候选源; 默认关, 见 [`super::CareConfig`])。
    Reunion,
}

impl CareTier {
    /// 稳定标签 (台账 `kind` 列 / 判定留痕共用, 闭合词表)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Reminder => "reminder",
            Self::GuardAlert => "guard_alert",
            Self::Ritual => "ritual",
            Self::Reunion => "reunion",
        }
    }

    /// 守护告警标注 (破勿扰必带; 其余档无标注)。
    pub const fn annotation(self) -> Option<&'static str> {
        match self {
            Self::GuardAlert => Some(GUARD_ALERT_ANNOTATION),
            Self::Rule | Self::Reminder | Self::Ritual | Self::Reunion => None,
        }
    }

    /// 守护例外档判定 (只有守护告警带例外档语义)。
    pub const fn is_guard_alert(self) -> bool {
        matches!(self, Self::GuardAlert)
    }

    /// 生物机制候选档判定 (仪式 / 重聚; 默认关, 开关见 [`super::CareConfig`])。
    pub const fn is_bio(self) -> bool {
        matches!(self, Self::Ritual | Self::Reunion)
    }
}

/// 守护告警标注 (破勿扰发起必带, 标准回合输入 + 台账留痕都在)。
pub const GUARD_ALERT_ANNOTATION: &str = "守护告警";

/// 提醒/告警信号 (提醒侧消费的载荷: 天气提醒信号 / 守护告警的共同形状)。
///
/// 落点类型 (`weather-signals.jsonl` 的天气提醒信号 / `guard-alerts.jsonl` 的
/// 守护告警) 由组装根映射成本载荷 —— 内核不认落点格式, 只认这一份字段合同:
/// `{signal_id, tier, trigger, severity, message, evidence, source, created_at_ms}`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderCandidate {
    /// 信号稳定身份 (去重键; 天气 `wxsig-*` / 守护 `gwalert-*`)。
    pub signal_id: String,
    /// 来源档 (提醒 / 守护告警)。
    pub tier: CareTier,
    /// 触发词表 (天气: temperature_drop 等; 守护: 条件词)。
    pub trigger: String,
    /// 严重度 (`info` / `warn` / `critical`)。
    pub severity: String,
    /// 建议语 (提示措辞, 由标准回合转成关怀消息)。
    pub message: String,
    /// 数据依据 (来自读数的有界摘要)。
    pub evidence: String,
    /// 信号来源工具 (`tool.weather` / `tool.guard_watch`)。
    pub source: String,
    /// 信号生成时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
}

impl ReminderCandidate {
    /// 信号 → 关怀候选 (提醒 = 关怀候选的一种)。
    ///
    /// 来源档进 [`CareCandidate::tier`] (打扰纪律分档 + 守护告警标注同源);
    /// 素材 = 建议语, 依据进触发语义 (标准回合看得见为什么)。
    pub fn to_care_candidate(&self) -> CareCandidate {
        CareCandidate {
            rule_id: self.signal_id.clone(),
            trigger: format!(
                "{}信号 {} ({}, 依据: {})",
                self.tier.label(),
                self.trigger,
                self.severity,
                self.evidence
            ),
            due_ms: self.created_at_ms,
            material: self.message.clone(),
            tier: self.tier,
        }
    }
}

/// 提醒信号源合同 (提醒侧消费端的取信号缝)。
///
/// 实现方: 组装根接落点账 (`weather-signals.jsonl` / `guard-alerts.jsonl` 的
/// 读桥); 测试接内存源。**消费位归实现方**: 内核只在发起/说不后回调
/// [`Self::mark_consumed`], 拦下不标 (候选不丢, 窗口滑过重试)。
pub trait ReminderFeed: Send + Sync {
    /// 现取未消费的信号 (按信号时刻升序; 空 = 无待消费)。
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String>;

    /// 标记信号已消费 (发起/说不后由内核回调)。
    fn mark_consumed(&self, signal_id: &str) -> Result<(), String>;
}

/// 内存信号源 (测试/无账装配用): 收信号 + 消费位, 不落盘。
#[derive(Debug, Default)]
pub struct MemoryReminderFeed {
    pending: Mutex<Vec<ReminderCandidate>>,
    consumed: Mutex<BTreeSet<String>>,
}

impl MemoryReminderFeed {
    /// 内存信号源。
    pub fn new() -> Self {
        Self::default()
    }

    /// 喂一条信号。
    pub fn push(&self, candidate: ReminderCandidate) {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(candidate);
    }

    /// 已消费的信号身份 (顺序无关)。
    pub fn consumed(&self) -> BTreeSet<String> {
        self.consumed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl ReminderFeed for MemoryReminderFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        let consumed = self.consumed.lock().unwrap_or_else(|p| p.into_inner());
        let pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        Ok(pending
            .iter()
            .filter(|signal| !consumed.contains(&signal.signal_id))
            .cloned()
            .collect())
    }

    fn mark_consumed(&self, signal_id: &str) -> Result<(), String> {
        self.consumed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(signal_id.to_string());
        Ok(())
    }
}

/// 消费一轮信号的错误 (闭合词表: 信号源读失败 / 关怀存储拒开)。
#[derive(Debug, thiserror::Error)]
pub enum ReminderConsumeError {
    /// 信号源读失败 (真因如实)。
    #[error("reminder feed read failed: {0}")]
    Feed(String),
    /// 关怀规则表拒开 (StoredDoc 语义)。
    #[error("care store refused: {0}")]
    Care(#[from] CareStoreError),
}

/// 消费一轮信号: 逐条取未消费信号 → 同源纪律 → 发起闸 → 判定。
///
/// 语义 (与 [`CareEngine::poll`] 同一条纪律管道):
///
/// - **发起 / 说不** = 该信号裁决完, 标记消费 (台账留痕);
/// - **拦下** ([`CarePoll::Held`]) = **不标消费** (候选不丢, 下轮重试);
/// - 至多产出一个发起 ([`CareInitiative`]): 首个发起即停, 剩余信号留给下轮
///   (心跳一次只发一回合, 防唠叨)。
pub fn consume_reminders(
    engine: &mut CareEngine,
    feed: &dyn ReminderFeed,
) -> Result<Option<CareInitiative>, ReminderConsumeError> {
    for candidate in feed.pending().map_err(ReminderConsumeError::Feed)? {
        match engine.poll_reminder(&candidate)? {
            CarePoll::Initiating(initiative) => {
                feed.mark_consumed(&candidate.signal_id)
                    .map_err(ReminderConsumeError::Feed)?;
                return Ok(Some(*initiative));
            }
            CarePoll::Declined { .. } => {
                feed.mark_consumed(&candidate.signal_id)
                    .map_err(ReminderConsumeError::Feed)?;
            }
            CarePoll::Held { .. } | CarePoll::Idle => {}
        }
    }
    Ok(None)
}

/// 信号消费面的关怀内核入口 (与规则轮询同一引擎, 同一套纪律)。
impl CareEngine {
    /// 消费一条提醒/告警信号: 提醒 = 关怀候选的一种 (同源纪律)。
    ///
    /// 管道与 [`CareEngine::poll`] 同源: 打扰纪律四件 (守护例外档按
    /// [`CareTier`] 分档) → 发起闸 (try_take_initiative 同源) → 发起意愿判定
    /// → 台账留痕。已裁决过的信号 (台账里已有同 id 记录) 不再裁决 (消费位
    /// 台账优先, 见模块 0 装诚实注)。
    pub fn poll_reminder(
        &mut self,
        candidate: &ReminderCandidate,
    ) -> Result<CarePoll, CareStoreError> {
        let now_ms = self.now_ms();
        let body = self.store().list()?;
        if body
            .history
            .iter()
            .any(|record| record.rule_id == candidate.signal_id)
        {
            // 已裁决 (台账在) = 消费位台账优先, 不重发不重复计数。
            return Ok(CarePoll::Idle);
        }
        let care_candidate = candidate.to_care_candidate();
        if let Some(hold) = self.discipline_hold_for(&body, now_ms, care_candidate.tier) {
            return Ok(CarePoll::Held {
                rule_id: care_candidate.rule_id.clone(),
                hold,
            });
        }
        self.admit_candidate(now_ms, care_candidate)
    }
}

/// 台账是否守护告警档 (独立频率上限的计数口径)。
pub fn is_guard_alert_record(record: &CareRecord) -> bool {
    record.kind == CareTier::GuardAlert.label()
}
