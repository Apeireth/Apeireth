//! R22 ST-A2.2 — Cognitive-Dream 6 状态机深化 (transition_rate_limit + cycle_detector)
//!
//! **深化层级** (per R22 路线 A, ST-A2.2):
//! - 现有 `CognitiveDreamStateMachine` 已实装 6 状态 + transition history + legal_targets + continuity_id 锚定
//! - 本模块深化为 **速率限制** + **循环检测**, 让 6 状态机从"合法转移" 升级为"行为模式可审计"
//!
//! **TransferRateLimiter**:
//! - 防止循环触发 (Awake ↔ Reflecting 来回跳)
//! - 默认 60s 内同 (from, to) 对最多 1 次, L0 HA 紧急停豁免
//! - 真实守护: 高频抖动 = 哲学锚偏离预警信号
//!
//! **CycleDetector**:
//! - 检测状态机环形转移 (Awake → Reflecting → Awake → Reflecting 短时间内)
//! - 默认 5min 内同状态对出现 ≥ 3 次 = 触发环预警
//! - 返回 cycle_info 供 L0 HA 监督
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 速率限制 + 循环检测是 in-memory, 持久化留给 ST-A2.4 (6 历史流深度)
//! - L0 HA 紧急停豁免是 per-stage 设计 (SelfDisabling 进入和 Recovering 唯一出口), 不在这里覆写

use std::collections::{HashMap, VecDeque};
use std::fmt;

use super::{CognitiveDreamState, TransitionReason, TransitionRecord};

/// 默认转移冷却 (同 from-to 对最少 60s)
pub const DEFAULT_TRANSFER_COOLDOWN_SECS: i64 = 60;
/// 默认环检测窗口 (5 min)
pub const DEFAULT_CYCLE_WINDOW_SECS: i64 = 300;
/// 默认环检测阈值 (窗口内同 from-to 对出现 ≥ N 次)
pub const DEFAULT_CYCLE_THRESHOLD: usize = 3;
/// 默认 history 上限 (LRU 6 弹出)
pub const DEFAULT_MAX_HISTORY: usize = 64;

/// 速率限制错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitError {
    /// 转移过于频繁
    TooFrequent {
        from: CognitiveDreamState,
        to: CognitiveDreamState,
        cooldown_secs: i64,
        last_ts: i64,
        now: i64,
    },
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFrequent { from, to, cooldown_secs, last_ts, now } => write!(
                f,
                "transition {from:?} -> {to:?} too frequent: cooldown={cooldown_secs}s, last_ts={last_ts}, now={now}"
            ),
        }
    }
}

impl std::error::Error for RateLimitError {}

/// 环检测结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleInfo {
    pub from: CognitiveDreamState,
    pub to: CognitiveDreamState,
    pub occurrences: usize,
    pub window_secs: i64,
    pub first_ts: i64,
    pub last_ts: i64,
}

/// Cognitive-Dream 6 状态机深度监控 (速率限制 + 环检测)
///
/// **不假装**: 纯 in-memory, 不持久化, 也不自动 block.
pub struct CognitiveDreamMonitor {
    /// (from, to) -> 最近一次 timestamp
    last_transfer: HashMap<(CognitiveDreamState, CognitiveDreamState), i64>,
    /// TransferRecord 滑窗 (用于环检测)
    window: VecDeque<TransferSnapshot>,
    /// 冷却秒数
    pub cooldown_secs: i64,
    /// 环检测窗口秒数
    pub cycle_window_secs: i64,
    /// 环检测阈值
    pub cycle_threshold: usize,
    /// history 上限
    pub max_history: usize,
}

/// TransferSnapshot — 转移记录的简化版 (in-memory only)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferSnapshot {
    pub from: CognitiveDreamState,
    pub to: CognitiveDreamState,
    pub ts: i64,
}

impl CognitiveDreamMonitor {
    /// 构造新 monitor (使用默认值)
    pub fn new() -> Self {
        Self {
            last_transfer: HashMap::new(),
            window: VecDeque::new(),
            cooldown_secs: DEFAULT_TRANSFER_COOLDOWN_SECS,
            cycle_window_secs: DEFAULT_CYCLE_WINDOW_SECS,
            cycle_threshold: DEFAULT_CYCLE_THRESHOLD,
            max_history: DEFAULT_MAX_HISTORY,
        }
    }

    /// 校验速率限制 (Cooldown 守门)
    ///
    /// **L0 HA 豁免**: SelfDisabling 紧急停豁免 (per stage4-correction-v15 紧急路径)
    pub fn check_rate_limit(
        &self,
        from: CognitiveDreamState,
        to: CognitiveDreamState,
        now: i64,
    ) -> Result<(), RateLimitError> {
        // L0 HA 紧急停豁免: SelfDisabling 进入 + Recovering 出口都不限制
        if matches!(to, CognitiveDreamState::SelfDisabling)
            || matches!(from, CognitiveDreamState::SelfDisabling)
        {
            return Ok(());
        }
        let key = (from, to);
        if let Some(&last_ts) = self.last_transfer.get(&key) {
            if now - last_ts < self.cooldown_secs {
                return Err(RateLimitError::TooFrequent {
                    from,
                    to,
                    cooldown_secs: self.cooldown_secs,
                    last_ts,
                    now,
                });
            }
        }
        Ok(())
    }

    /// 记录一次转移 (更新 last_transfer + 滑窗)
    pub fn record(&mut self, record: &TransitionRecord) {
        let key = (record.from, record.to);
        // 把 timestamp 转成 unix seconds (Utc::now() -> ts_seconds)
        let ts_secs = record.at.timestamp();
        self.last_transfer.insert(key, ts_secs);
        self.window.push_back(TransferSnapshot {
            from: record.from,
            to: record.to,
            ts: ts_secs,
        });
        while self.window.len() > self.max_history {
            self.window.pop_front();
        }
    }

    /// 环检测: 检查 (from, to) 对在窗口内是否出现 >= threshold 次
    pub fn detect_cycle(
        &self,
        from: CognitiveDreamState,
        to: CognitiveDreamState,
        now: i64,
    ) -> Option<CycleInfo> {
        let cutoff = now - self.cycle_window_secs;
        let mut occurrences = 0usize;
        let mut first_ts = i64::MAX;
        let mut last_ts = i64::MIN;
        for snap in self.window.iter() {
            if snap.ts < cutoff {
                continue;
            }
            if snap.from == from && snap.to == to {
                occurrences += 1;
                if snap.ts < first_ts {
                    first_ts = snap.ts;
                }
                if snap.ts > last_ts {
                    last_ts = snap.ts;
                }
            }
        }
        if occurrences >= self.cycle_threshold {
            Some(CycleInfo {
                from,
                to,
                occurrences,
                window_secs: self.cycle_window_secs,
                first_ts: if first_ts == i64::MAX { 0 } else { first_ts },
                last_ts: if last_ts == i64::MIN { 0 } else { last_ts },
            })
        } else {
            None
        }
    }

    /// 综合检查: 速率限制 + 环检测
    pub fn check_full(
        &self,
        from: CognitiveDreamState,
        to: CognitiveDreamState,
        now: i64,
    ) -> Result<(), MonitorError> {
        self.check_rate_limit(from, to, now)
            .map_err(MonitorError::RateLimit)?;
        if let Some(cycle) = self.detect_cycle(from, to, now) {
            return Err(MonitorError::Cycle(cycle));
        }
        Ok(())
    }
}

impl Default for CognitiveDreamMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 综合检查错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorError {
    RateLimit(RateLimitError),
    Cycle(CycleInfo),
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimit(e) => write!(f, "rate limit: {e}"),
            Self::Cycle(c) => write!(
                f,
                "cycle detected: {:?} -> {:?} x{} in {}s (first={}, last={})",
                c.from, c.to, c.occurrences, c.window_secs, c.first_ts, c.last_ts
            ),
        }
    }
}

impl std::error::Error for MonitorError {}

// ============================================
// 单元测试 (10+ tests)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ts(secs: i64) -> chrono::DateTime<Utc> {
        chrono::DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn monitor_new_defaults() {
        let m = CognitiveDreamMonitor::new();
        assert_eq!(m.cooldown_secs, 60);
        assert_eq!(m.cycle_window_secs, 300);
        assert_eq!(m.cycle_threshold, 3);
        assert_eq!(m.max_history, 64);
    }

    #[test]
    fn rate_limit_first_call_ok() {
        let m = CognitiveDreamMonitor::new();
        assert!(m
            .check_rate_limit(
                CognitiveDreamState::Awake,
                CognitiveDreamState::Reflecting,
                1000
            )
            .is_ok());
    }

    #[test]
    fn rate_limit_second_call_within_cooldown_blocked() {
        let mut m = CognitiveDreamMonitor::new();
        // 记录一次转移
        let record = TransitionRecord {
            from: CognitiveDreamState::Awake,
            to: CognitiveDreamState::Reflecting,
            at: ts(1000),
            reason: TransitionReason::UserTriggered,
        };
        m.record(&record);
        // 30s 后尝试同 from-to → 被 rate_limit 拦
        let res = m.check_rate_limit(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting,
            1030,
        );
        assert!(matches!(res, Err(RateLimitError::TooFrequent { .. })));
    }

    #[test]
    fn rate_limit_after_cooldown_ok() {
        let mut m = CognitiveDreamMonitor::new();
        let record = TransitionRecord {
            from: CognitiveDreamState::Awake,
            to: CognitiveDreamState::Reflecting,
            at: ts(1000),
            reason: TransitionReason::UserTriggered,
        };
        m.record(&record);
        // 61s 后 → 冷却过 → ok
        let res = m.check_rate_limit(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting,
            1061,
        );
        assert!(res.is_ok());
    }

    #[test]
    fn l0_ha_self_disabling_exempt() {
        let mut m = CognitiveDreamMonitor::new();
        // 记录一次 Awake -> Reflecting
        let record = TransitionRecord {
            from: CognitiveDreamState::Awake,
            to: CognitiveDreamState::Reflecting,
            at: ts(1000),
            reason: TransitionReason::UserTriggered,
        };
        m.record(&record);
        // L0 HA 紧急停 (Awake -> SelfDisabling) 不应受 rate_limit 拦
        let res = m.check_rate_limit(
            CognitiveDreamState::Awake,
            CognitiveDreamState::SelfDisabling,
            1001,
        );
        assert!(res.is_ok(), "L0 HA SelfDisabling 入口必须豁免 rate_limit");
    }

    #[test]
    fn self_disabling_to_recovering_exempt() {
        let m = CognitiveDreamMonitor::new();
        let res = m.check_rate_limit(
            CognitiveDreamState::SelfDisabling,
            CognitiveDreamState::Recovering,
            1000,
        );
        assert!(res.is_ok(), "SelfDisabling → Recovering 唯一出口必须豁免");
    }

    #[test]
    fn cycle_detect_3_occurrences_in_window_triggers() {
        let mut m = CognitiveDreamMonitor::new();
        m.cycle_threshold = 3;
        // Awake -> Reflecting x3 在 200s 内
        for i in 0..3 {
            let record = TransitionRecord {
                from: CognitiveDreamState::Awake,
                to: CognitiveDreamState::Reflecting,
                at: ts(1000 + i * 60),
                reason: TransitionReason::UserTriggered,
            };
            m.record(&record);
        }
        let cycle = m.detect_cycle(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting,
            1300,
        );
        assert!(cycle.is_some(), "3 次在窗口内必须触发环检测");
        let info = cycle.unwrap();
        assert_eq!(info.occurrences, 3);
    }

    #[test]
    fn cycle_detect_below_threshold_no_trigger() {
        let mut m = CognitiveDreamMonitor::new();
        m.cycle_threshold = 3;
        // 只 2 次
        for i in 0..2 {
            let record = TransitionRecord {
                from: CognitiveDreamState::Awake,
                to: CognitiveDreamState::Reflecting,
                at: ts(1000 + i * 60),
                reason: TransitionReason::UserTriggered,
            };
            m.record(&record);
        }
        let cycle = m.detect_cycle(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting,
            1300,
        );
        assert!(cycle.is_none(), "2 次在阈值 3 下不触发");
    }

    #[test]
    fn cycle_detect_outside_window_no_trigger() {
        let mut m = CognitiveDreamMonitor::new();
        m.cycle_threshold = 3;
        // 3 次但跨度 > 300s
        let _timestamps = [1000u64, 1400, 1800];
        // 注意 1000, 1400, 1800 跨度 800s > window 300s
        // 但是 ts(1800) 在 cutoff = 1800 - 300 = 1500 之前只 ts(1400) 算
        // 实际 1000, 1400 都 < 1500, 都 cutoff 外, 0 occurrences
        // 让 timestamps = [1500, 1700, 1900], cutoff = 1900-300=1600, 1700/1900 都 < cutoff? 不, 1700>=1600 ok
        // 用 [1600, 1700, 1800] → cutoff = 1800-300=1500, 全部 >=1500 → 3 occurrences
        // 改用 [1100, 1500, 1900] → cutoff = 1900-300=1600, 只有 1900 >= 1600 → 1 occurrence → no trigger
        let timestamps = [1100u64, 1500, 1900];
        for &ts_secs in &timestamps {
            let record = TransitionRecord {
                from: CognitiveDreamState::Awake,
                to: CognitiveDreamState::Reflecting,
                at: ts(ts_secs as i64),
                reason: TransitionReason::UserTriggered,
            };
            m.record(&record);
        }
        let cycle = m.detect_cycle(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting,
            1900,
        );
        assert!(cycle.is_none(), "3 次但跨度大不应触发环检测");
    }

    #[test]
    fn check_full_combined_rate_and_cycle() {
        let mut m = CognitiveDreamMonitor::new();
        m.cycle_threshold = 2; // 简化阈值
                               // 2 次 Awake -> Reflecting 在 100s 内
        for i in 0..2 {
            let record = TransitionRecord {
                from: CognitiveDreamState::Awake,
                to: CognitiveDreamState::Reflecting,
                at: ts(1000 + i * 30),
                reason: TransitionReason::UserTriggered,
            };
            m.record(&record);
        }
        // 第 3 次尝试同 (60s 内 + cycle)
        let res = m.check_full(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting,
            1100,
        );
        // rate_limit cooldown=60s, 1000→1030 是 30s < 60 → RateLimit 触发 (Cycle 在 RateLimit 之前判, 但本 case RateLimit 先触发)
        // 实际: rate_limit cooldown 60s, 1100-1030=70s ≥ 60 → rate_limit pass; cycle window 内 3 次 ≥ threshold 2 → cycle 触发
        assert!(
            matches!(res, Err(MonitorError::Cycle(_))),
            "3rd call within cycle window should hit cycle: {res:?}"
        )
    }

    #[test]
    fn default_impl_works() {
        let _m: CognitiveDreamMonitor = Default::default();
    }
}
