//! SGI 单字段写入触发器

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SGITriggerOutcome {
    Pass { field: String, value: String },
    Triggered { field: String, value: String, reason: String, cooldown_until_ms: i64 },
    CooldownActive { field: String, value: String, cooldown_until_ms: i64, remaining_ms: i64 },
}

impl SGITriggerOutcome {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass { .. }) }
    pub fn is_triggered(&self) -> bool { matches!(self, Self::Triggered { .. }) }
    pub fn is_cooldown(&self) -> bool { matches!(self, Self::CooldownActive { .. }) }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SGITrigger {
    pub field: String,
    pub value: String,
    pub reason: String,
    pub triggered_at_ms: i64,
    pub cooldown_until_ms: i64,
}

impl SGITrigger {
    pub fn is_cooldown_active(&self, current_ms: i64) -> bool { current_ms < self.cooldown_until_ms }
    pub fn remaining_ms(&self, current_ms: i64) -> i64 { (self.cooldown_until_ms - current_ms).max(0) }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SGIFieldRule {
    pub field: String,
    pub reason: String,
}

impl SGIFieldRule {
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self { field: field.into(), reason: reason.into() }
    }
}

pub struct SGITriggerGuard {
    rules: HashMap<String, String>,
    triggers: HashMap<String, SGITrigger>,
    pub cooldown_ms: i64,
}

impl SGITriggerGuard {
    pub fn new() -> Self {
        Self { rules: HashMap::new(), triggers: HashMap::new(), cooldown_ms: 86_400_000 }
    }

    pub fn with_default_rules() -> Self {
        let mut guard = Self::new();
        guard.add_rule(SGIFieldRule::new("requires_ha", "L0 HA 核心 — 修改等同摧毁最后护栏"));
        guard.add_rule(SGIFieldRule::new("mode", "HA 部署模式变更"));
        guard.add_rule(SGIFieldRule::new("ice_frozen_until", "HA 冰冻期变更"));
        guard.add_rule(SGIFieldRule::new("subject_id", "主体连续性 ID 变更"));
        guard.add_rule(SGIFieldRule::new("life_stage", "9 阶段生命周期阶段变更"));
        guard.add_rule(SGIFieldRule::new("l0_layer", "L0 权限洋葱核心变更"));
        guard.add_rule(SGIFieldRule::new("ha_human_count", "HA 注册人类数量变更"));
        guard
    }

    pub fn add_rule(&mut self, rule: SGIFieldRule) {
        self.rules.insert(rule.field.clone(), rule.reason);
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    pub fn check_field_write(&mut self, field: &str, value: &str, current_ms: i64) -> SGITriggerOutcome {
        if let Some(trigger) = self.triggers.get(field) {
            if trigger.is_cooldown_active(current_ms) {
                return SGITriggerOutcome::CooldownActive {
                    field: field.into(), value: value.into(),
                    cooldown_until_ms: trigger.cooldown_until_ms,
                    remaining_ms: trigger.remaining_ms(current_ms),
                };
            }
        }
        if let Some(reason) = self.rules.get(field) {
            let trigger = SGITrigger {
                field: field.into(), value: value.into(), reason: reason.clone(),
                triggered_at_ms: current_ms, cooldown_until_ms: current_ms + self.cooldown_ms,
            };
            self.triggers.insert(field.into(), trigger.clone());
            return SGITriggerOutcome::Triggered {
                field: field.into(), value: value.into(),
                reason: reason.clone(), cooldown_until_ms: trigger.cooldown_until_ms,
            };
        }
        SGITriggerOutcome::Pass { field: field.into(), value: value.into() }
    }

    pub fn clear_triggers(&mut self) { self.triggers.clear(); }
    pub fn last_trigger(&self, field: &str) -> Option<&SGITrigger> { self.triggers.get(field) }
}

impl Default for SGITriggerGuard { fn default() -> Self { Self::with_default_rules() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn default_rules_seven() {
        let g = SGITriggerGuard::with_default_rules();
        assert_eq!(g.rule_count(), 7);
    }
    #[test] fn pass_for_unknown_field() {
        let mut g = SGITriggerGuard::with_default_rules();
        match g.check_field_write("normal_field", "v", 1000) {
            SGITriggerOutcome::Pass { .. } => {}
            other => panic!("expected Pass, got {:?}", other),
        }
    }
    #[test] fn sgi_field_triggers() {
        let mut g = SGITriggerGuard::with_default_rules();
        match g.check_field_write("requires_ha", "false", 1000) {
            SGITriggerOutcome::Triggered { reason, .. } => assert!(reason.contains("L0 HA")),
            other => panic!("expected Triggered, got {:?}", other),
        }
    }
    #[test] fn cooldown_active_blocks_second_write() {
        let mut g = SGITriggerGuard::with_default_rules();
        g.check_field_write("requires_ha", "false", 1000);
        let r2 = g.check_field_write("requires_ha", "true", 1000 + 100);
        assert!(matches!(r2, SGITriggerOutcome::CooldownActive { .. }));
    }
    #[test] fn cooldown_expires() {
        let mut g = SGITriggerGuard::with_default_rules();
        g.check_field_write("requires_ha", "false", 0);
        // 25h 后
        let r = g.check_field_write("requires_ha", "true", 25 * 3600 * 1000);
        assert!(matches!(r, SGITriggerOutcome::Triggered { .. }));
    }
}
