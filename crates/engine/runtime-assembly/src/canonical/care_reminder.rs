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
//! # 0 装诚实
//!
//! - 消费位是**进程内值** (重启清零): 重启后已裁决信号由内核的台账去重兜底
//!   (见 `apeireth_organ::care::reminder` 模块注), 不冒充持久消费位;
//! - 信号账读失败 = `Err(真因)` (拒开, 不静默跳本); 坏行按账面语义跳过并计数。
//! - 本面**零设备动作**: 只读两本信号账 (看到 ≠ 动手, 动手走 device_control)。

use std::collections::BTreeSet;
use std::sync::Mutex;

use apeireth_organ::care::{CareTier, ReminderCandidate, ReminderFeed};

use apeireth_tools_canonical::device::guard_watch::{GuardAlert, GuardAlertLedger};
use apeireth_tools_canonical::device::weather::{ReminderSignal, SignalLedger};

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
