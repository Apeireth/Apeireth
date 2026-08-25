//! Self-Disable Guard — 5 mechanism invariants

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelfDisableTrigger {
    NoDegradeViolation { from: String, to: String },
    NoPatchViolation { rule: String },
    NoBypassViolation { token: String },
    NoReverseViolation { trigger_id: String },
    NoHideViolation { window_id: String },
}

impl SelfDisableTrigger {
    pub fn mechanism_id(&self) -> u8 {
        match self {
            Self::NoDegradeViolation { .. } => 1,
            Self::NoPatchViolation { .. } => 2,
            Self::NoBypassViolation { .. } => 3,
            Self::NoReverseViolation { .. } => 4,
            Self::NoHideViolation { .. } => 5,
        }
    }
    pub fn mechanism_name(&self) -> &'static str {
        match self {
            Self::NoDegradeViolation { .. } => "NoDegrade",
            Self::NoPatchViolation { .. } => "NoPatch",
            Self::NoBypassViolation { .. } => "NoBypass",
            Self::NoReverseViolation { .. } => "NoReverse",
            Self::NoHideViolation { .. } => "NoHide",
        }
    }
    pub fn chinese_name(&self) -> &'static str {
        match self {
            Self::NoDegradeViolation { .. } => "不许降级",
            Self::NoPatchViolation { .. } => "不许补丁",
            Self::NoBypassViolation { .. } => "不许绕过",
            Self::NoReverseViolation { .. } => "不许反转",
            Self::NoHideViolation { .. } => "不许隐藏",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfDisableRecord {
    pub trigger_id: String,
    pub mechanism_id: u8,
    pub trigger: SelfDisableTrigger,
    pub context: String,
    pub timestamp_ms: i64,
}

impl SelfDisableRecord {
    pub fn new(trigger_id: impl Into<String>, mechanism_id: u8, trigger: SelfDisableTrigger, context: impl Into<String>) -> Self {
        Self { trigger_id: trigger_id.into(), mechanism_id, trigger, context: context.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelfDisableCheck {
    Pass,
    Triggered(SelfDisableRecord),
}

impl SelfDisableCheck {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass) }
    pub fn is_triggered(&self) -> bool { matches!(self, Self::Triggered(_)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfDisableSignal {
    Triggered,
    Disarmed,
    Reversed,
}

#[derive(Debug, Error)]
pub enum SelfDisableError {
    #[error("risk level mismatch")]
    RiskLevelMismatch,
}

pub struct SelfDisableGuard {
    pub is_armed: bool,
    records: Vec<SelfDisableRecord>,
    next_id: u64,
}

impl Default for SelfDisableGuard { fn default() -> Self { Self::new() } }

impl SelfDisableGuard {
    pub fn new() -> Self {
        Self { is_armed: true, records: Vec::new(), next_id: 1 }
    }

    pub fn disarm(&mut self) { self.is_armed = false; }
    pub fn rearm(&mut self) { self.is_armed = true; }

    fn risk_rank(s: &str) -> i32 {
        match s.to_ascii_lowercase().as_str() {
            "low" | "info" => 0, "medium" => 1, "high" | "critical" | "nuclear" => 2, _ => 0,
        }
    }

    fn gen_id(&mut self) -> String {
        let id = format!("sd-{:06}", self.next_id);
        self.next_id += 1;
        id
    }

    fn push_record(&mut self, trigger: SelfDisableTrigger, context: String) -> SelfDisableRecord {
        let id = self.gen_id();
        let mechanism_id = trigger.mechanism_id();
        let rec = SelfDisableRecord::new(id, mechanism_id, trigger, context);
        self.records.push(rec.clone());
        rec
    }

    pub fn check_no_degrade(&mut self, from: &str, to: &str, context: &str, at_ms: i64) -> SelfDisableCheck {
        if !self.is_armed { return SelfDisableCheck::Pass; }
        if Self::risk_rank(from) > Self::risk_rank(to) {
            let trigger = SelfDisableTrigger::NoDegradeViolation { from: from.into(), to: to.into() };
            let mut rec = SelfDisableRecord::new(self.gen_id(), trigger.mechanism_id(), trigger.clone(), context);
            rec.timestamp_ms = at_ms;
            self.records.push(rec.clone());
            SelfDisableCheck::Triggered(rec)
        } else {
            SelfDisableCheck::Pass
        }
    }

    pub fn check_no_patch(&mut self, rule: &str, _expected: u32, context: &str, at_ms: i64) -> SelfDisableCheck {
        if !self.is_armed { return SelfDisableCheck::Pass; }
        let trigger = SelfDisableTrigger::NoPatchViolation { rule: rule.into() };
        let mut rec = SelfDisableRecord::new(self.gen_id(), trigger.mechanism_id(), trigger, context);
        rec.timestamp_ms = at_ms;
        self.records.push(rec.clone());
        SelfDisableCheck::Triggered(rec)
    }

    pub fn check_no_bypass(&mut self, token: &str, attempted: bool, context: &str, at_ms: i64) -> SelfDisableCheck {
        if !self.is_armed { return SelfDisableCheck::Pass; }
        if attempted {
            let trigger = SelfDisableTrigger::NoBypassViolation { token: token.into() };
            let mut rec = SelfDisableRecord::new(self.gen_id(), trigger.mechanism_id(), trigger, context);
            rec.timestamp_ms = at_ms;
            self.records.push(rec.clone());
            SelfDisableCheck::Triggered(rec)
        } else {
            SelfDisableCheck::Pass
        }
    }

    pub fn check_no_reverse(&mut self, trigger_id: &str, context: &str, at_ms: i64) -> SelfDisableCheck {
        if !self.is_armed { return SelfDisableCheck::Pass; }
        let trigger = SelfDisableTrigger::NoReverseViolation { trigger_id: trigger_id.into() };
        let mut rec = SelfDisableRecord::new(self.gen_id(), trigger.mechanism_id(), trigger, context);
        rec.timestamp_ms = at_ms;
        self.records.push(rec.clone());
        SelfDisableCheck::Triggered(rec)
    }

    pub fn check_no_hide(&mut self, window_id: &str, context: &str, at_ms: i64) -> SelfDisableCheck {
        if !self.is_armed { return SelfDisableCheck::Pass; }
        let trigger = SelfDisableTrigger::NoHideViolation { window_id: window_id.into() };
        let mut rec = SelfDisableRecord::new(self.gen_id(), trigger.mechanism_id(), trigger, context);
        rec.timestamp_ms = at_ms;
        self.records.push(rec.clone());
        SelfDisableCheck::Triggered(rec)
    }

    pub fn has_triggered(&self) -> bool { !self.records.is_empty() }
    pub fn record_count(&self) -> usize { self.records.len() }
    pub fn records(&self) -> &[SelfDisableRecord] { self.records.as_slice() }
    pub fn records_by_mechanism(&self, mechanism_id: u8) -> Vec<&SelfDisableRecord> {
        self.records.iter().filter(|r| r.mechanism_id == mechanism_id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn new_armed_by_default() {
        let g = SelfDisableGuard::new();
        assert!(g.is_armed);
        assert_eq!(g.record_count(), 0);
        assert!(!g.has_triggered());
    }

    #[test] fn disarm_relaxes() {
        let mut g = SelfDisableGuard::new();
        g.disarm();
        assert!(!g.is_armed);
        let r = g.check_no_degrade("high", "low", "test", 1000);
        assert!(r.is_pass());
        assert_eq!(g.record_count(), 0);
    }

    #[test] fn rearm_reactivates() {
        let mut g = SelfDisableGuard::new();
        g.disarm();
        g.rearm();
        assert!(g.is_armed);
        assert!(g.check_no_degrade("high", "low", "test", 1000).is_triggered());
    }

    #[test] fn records_append_only() {
        let mut g = SelfDisableGuard::new();
        g.check_no_degrade("high", "low", "t1", 1000);
        g.check_no_degrade("high", "low", "t2", 2000);
        assert_eq!(g.record_count(), 2);
    }

    #[test] fn no_degrade_high_to_low_triggers() {
        let mut g = SelfDisableGuard::new();
        assert!(g.check_no_degrade("high", "low", "x", 1000).is_triggered());
    }

    #[test] fn no_degrade_same_or_up_passes() {
        let mut g = SelfDisableGuard::new();
        assert!(g.check_no_degrade("high", "high", "x", 1000).is_pass());
        assert!(g.check_no_degrade("low", "high", "x", 1000).is_pass());
        assert!(g.check_no_degrade("medium", "medium", "x", 1000).is_pass());
    }

    #[test] fn records_by_mechanism() {
        let mut g = SelfDisableGuard::new();
        g.check_no_degrade("high", "low", "t1", 1000);
        g.check_no_degrade("high", "low", "t2", 2000);
        g.check_no_patch("r", 0, "t3", 3000);
        assert_eq!(g.records_by_mechanism(1).len(), 2);
        assert_eq!(g.records_by_mechanism(2).len(), 1);
        assert_eq!(g.records_by_mechanism(99).len(), 0);
    }

    #[test] fn trigger_id_format() {
        let mut g = SelfDisableGuard::new();
        g.check_no_degrade("high", "low", "x", 1000);
        let r = &g.records()[0];
        assert!(r.trigger_id.starts_with("sd-"));
        assert!(r.trigger_id.len() >= 9);
    }

    #[test] fn mechanism_names() {
        let t = SelfDisableTrigger::NoDegradeViolation { from: "high".into(), to: "low".into() };
        assert_eq!(t.mechanism_name(), "NoDegrade");
        assert_eq!(t.chinese_name(), "不许降级");
        assert_eq!(t.mechanism_id(), 1);
    }

    #[test] fn trigger_id_uniqueness() {
        let mut g = SelfDisableGuard::new();
        let mut ids = std::collections::HashSet::new();
        for i in 0..10 {
            g.check_no_degrade("high", "low", &format!("ctx{}", i), i);
        }
        for r in g.records() {
            assert!(ids.insert(r.trigger_id.clone()));
        }
        assert_eq!(ids.len(), 10);
    }

    #[test] fn no_bypass_conditional() {
        let mut g = SelfDisableGuard::new();
        let r1 = g.check_no_bypass("master", false, "x", 1000);
        assert!(r1.is_pass());
        let r2 = g.check_no_bypass("master", true, "x", 2000);
        assert!(r2.is_triggered());
        assert_eq!(g.record_count(), 1);
    }

    #[test] fn no_reverse_always_triggers_when_armed() {
        let mut g = SelfDisableGuard::new();
        assert!(g.check_no_reverse("t1", "x", 1000).is_triggered());
    }

    #[test] fn no_hide_always_triggers_when_armed() {
        let mut g = SelfDisableGuard::new();
        assert!(g.check_no_hide("w1", "x", 1000).is_triggered());
    }

    #[test] fn mechanism_ids_are_stable() {
        assert_eq!(SelfDisableTrigger::NoDegradeViolation { from: "a".into(), to: "b".into() }.mechanism_id(), 1);
        assert_eq!(SelfDisableTrigger::NoPatchViolation { rule: "r".into() }.mechanism_id(), 2);
        assert_eq!(SelfDisableTrigger::NoBypassViolation { token: "t".into() }.mechanism_id(), 3);
        assert_eq!(SelfDisableTrigger::NoReverseViolation { trigger_id: "t".into() }.mechanism_id(), 4);
        assert_eq!(SelfDisableTrigger::NoHideViolation { window_id: "w".into() }.mechanism_id(), 5);
    }
}