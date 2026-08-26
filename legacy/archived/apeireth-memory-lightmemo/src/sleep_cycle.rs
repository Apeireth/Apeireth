//! Sleep cycle: offline consolidation trigger.

use chrono::{DateTime, Utc};
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
}

impl SleepCycle {
    pub fn new() -> Self {
        Self {
            config: SleepConfig::default(),
            last_activity: std::sync::Mutex::new(Utc::now()),
            items_since_last_cycle: std::sync::Mutex::new(0),
        }
    }
    pub fn with_config(config: SleepConfig) -> Self {
        Self { config, last_activity: std::sync::Mutex::new(Utc::now()), items_since_last_cycle: std::sync::Mutex::new(0) }
    }

    /// Record an activity event (resets quiet timer).
    pub fn record_activity(&self) {
        *self.last_activity.lock().expect("poisoned") = Utc::now();
    }

    /// Record an item addition.
    pub fn record_item_added(&self) {
        *self.items_since_last_cycle.lock().expect("poisoned") += 1;
    }

    /// Should we trigger a consolidation cycle now?
    pub fn should_consolidate(&self) -> bool {
        let last = *self.last_activity.lock().expect("poisoned");
        let quiet = Utc::now() - last;
        let items = *self.items_since_last_cycle.lock().expect("poisoned");
        quiet >= chrono::Duration::from_std(self.config.quiet_threshold).unwrap_or(chrono::Duration::seconds(60))
            || items >= self.config.max_items_before_consolidate
    }

    /// Reset state after a cycle runs.
    pub fn reset_after_cycle(&self) {
        *self.last_activity.lock().expect("poisoned") = Utc::now();
        *self.items_since_last_cycle.lock().expect("poisoned") = 0;
    }
}

impl Default for SleepCycle {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cycle_no_consolidate() {
        let c = SleepCycle::new();
        assert!(!c.should_consolidate());
    }

    #[test]
    fn consolidate_after_quiet() {
        let c = SleepCycle::with_config(SleepConfig {
            quiet_threshold: Duration::from_millis(50),
            max_items_before_consolidate: usize::MAX,
        });
        std::thread::sleep(Duration::from_millis(100));
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
        let c = SleepCycle::with_config(SleepConfig {
            quiet_threshold: Duration::from_millis(50),
            max_items_before_consolidate: usize::MAX,
        });
        std::thread::sleep(Duration::from_millis(60));
        c.record_activity();
        assert!(!c.should_consolidate());
    }
}