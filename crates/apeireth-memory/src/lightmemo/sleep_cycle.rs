//! Sleep cycle: offline consolidation trigger.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use apeireth_core::clock::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SleepConfig {
    /// Trigger consolidation after this many seconds of "quiet" (no activity)
    pub quiet_threshold: Duration,
    /// Or when item count exceeds this threshold
    pub max_items_before_consolidate: usize,
}

impl Default for SleepConfig {
    fn default() -> Self {
        Self {
            quiet_threshold: Duration::from_secs(60),
            max_items_before_consolidate: 1000,
        }
    }
}

pub struct SleepCycle {
    config: SleepConfig,
    last_activity: std::sync::Mutex<DateTime<Utc>>,
    items_since_last_cycle: std::sync::Mutex<usize>,
    /// 可注入时钟: 默认系统时钟, 模拟/测试注入 VirtualClock 快进 (0 真等待).
    clock: Arc<dyn Clock>,
}

impl SleepCycle {
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemClock))
    }
    pub fn with_config(config: SleepConfig) -> Self {
        Self::with_config_and_clock(config, Arc::new(SystemClock))
    }
    /// 注入时钟版 (虚拟时间模拟/测试).
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self {
            config: SleepConfig::default(),
            last_activity: std::sync::Mutex::new(clock.now()),
            items_since_last_cycle: std::sync::Mutex::new(0),
            clock,
        }
    }
    pub fn with_config_and_clock(config: SleepConfig, clock: Arc<dyn Clock>) -> Self {
        Self {
            config,
            last_activity: std::sync::Mutex::new(clock.now()),
            items_since_last_cycle: std::sync::Mutex::new(0),
            clock,
        }
    }

    /// Record an activity event (resets quiet timer).
    pub fn record_activity(&self) {
        *self.last_activity.lock().expect("poisoned") = self.clock.now();
    }

    /// Record an item addition.
    pub fn record_item_added(&self) {
        *self.items_since_last_cycle.lock().expect("poisoned") += 1;
    }

    /// Should we trigger a consolidation cycle now?
    pub fn should_consolidate(&self) -> bool {
        let last = *self.last_activity.lock().expect("poisoned");
        let quiet = self.clock.now() - last;
        let items = *self.items_since_last_cycle.lock().expect("poisoned");
        quiet >= chrono::Duration::from_std(self.config.quiet_threshold).unwrap_or(chrono::Duration::seconds(60))
            || items >= self.config.max_items_before_consolidate
    }

    /// Reset state after a cycle runs.
    pub fn reset_after_cycle(&self) {
        *self.last_activity.lock().expect("poisoned") = self.clock.now();
        *self.items_since_last_cycle.lock().expect("poisoned") = 0;
    }
}

impl Default for SleepCycle {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use chrono::TimeZone;

    fn vclock() -> VirtualClock {
        VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap())
    }

    #[test]
    fn new_cycle_no_consolidate() {
        let c = SleepCycle::new();
        assert!(!c.should_consolidate());
    }

    #[test]
    fn consolidate_after_quiet() {
        // 虚拟时钟快进 100ms, 0 真等待
        let vc = vclock();
        let c = SleepCycle::with_config_and_clock(
            SleepConfig {
                quiet_threshold: Duration::from_millis(50),
                max_items_before_consolidate: usize::MAX,
            },
            Arc::new(vc.clone()),
        );
        vc.advance(chrono::Duration::milliseconds(100));
        assert!(c.should_consolidate());
    }

    #[test]
    fn consolidate_after_too_many_items() {
        let c = SleepCycle::with_config(SleepConfig {
            quiet_threshold: Duration::from_secs(3600),
            max_items_before_consolidate: 3,
        });
        c.record_item_added();
        c.record_item_added();
        c.record_item_added();
        assert!(c.should_consolidate());
    }

    #[test]
    fn reset_clears_state() {
        let c = SleepCycle::with_config(SleepConfig {
            quiet_threshold: Duration::from_secs(3600),
            max_items_before_consolidate: 1,
        });
        c.record_item_added();
        assert!(c.should_consolidate());
        c.reset_after_cycle();
        assert!(!c.should_consolidate());
    }

    #[test]
    fn activity_resets_quiet() {
        // 虚拟时钟: 快进 60ms 后 record_activity 重置安静计时
        let vc = vclock();
        let c = SleepCycle::with_config_and_clock(
            SleepConfig {
                quiet_threshold: Duration::from_millis(50),
                max_items_before_consolidate: usize::MAX,
            },
            Arc::new(vc.clone()),
        );
        vc.advance(chrono::Duration::milliseconds(60));
        c.record_activity();
        assert!(!c.should_consolidate());
    }

    #[test]
    fn long_quiet_triggers_after_reset() {
        // 完整周期: 触发 → reset → 再快进再触发 (0 真等待)
        let vc = vclock();
        let c = SleepCycle::with_config_and_clock(
            SleepConfig {
                quiet_threshold: Duration::from_secs(60),
                max_items_before_consolidate: usize::MAX,
            },
            Arc::new(vc.clone()),
        );
        assert!(!c.should_consolidate());
        vc.advance(chrono::Duration::seconds(61));
        assert!(c.should_consolidate());
        c.reset_after_cycle();
        assert!(!c.should_consolidate());
        vc.advance(chrono::Duration::seconds(61));
        assert!(c.should_consolidate());
    }
}