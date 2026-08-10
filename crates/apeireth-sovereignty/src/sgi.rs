//! SGI 单字段写入触发器 — Single-Field Giant Impact
//!
//! **设计**:
//! - "单字段写入可触发巨型影响" — 单个字段的变更可能引发系统性变更
//! - 例如: 修改 `requires_ha = false` 即等同摧毁 L0 HA → 必须 SGI 触发
//! - 例如: 修改 `ice_frozen_until` 提前 24h → 必须 SGI 触发
//! - 例如: 修改 `subject_id` → 必须 SGI 触发
//!
//! **规则**:
//! - 每条 [`SGIFieldRule`] 包含字段名 + 触发的拒绝动作 (锁定 / 触发主权审议 / 24h 冷却)
//! - 触发后进入 24h 冷却期 (`SGI_COOLDOWN_MS`), 冷却期内同字段写入强制 SuspendSelf

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SGI 触发器产出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SGITriggerOutcome {
    /// 通过 (无 SGI 触发)
    Pass {
        /// 字段名
        field: String,
        /// 写入值
        value: String,
    },
    /// 触发 SGI (进入 24h 冷却)
    Triggered {
        /// 字段名
        field: String,
        /// 写入值
        value: String,
        /// 触发原因
        reason: String,
        /// 冷却期结束时间 (epoch ms)
        cooldown_until_ms: i64,
    },
    /// 冷却期内禁止写入
    CooldownActive {
        /// 字段名
        field: String,
        /// 写入值
        value: String,
        /// 冷却期结束时间
        cooldown_until_ms: i64,
        /// 剩余冷却时间 (ms)
        remaining_ms: i64,
    },
}

impl SGITriggerOutcome {
    /// 是否通过
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// 是否触发 SGI
    pub fn is_triggered(&self) -> bool {
        matches!(self, Self::Triggered { .. })
    }

    /// 是否处于冷却期
    pub fn is_cooldown(&self) -> bool {
        matches!(self, Self::CooldownActive { .. })
    }
}

/// SGI 触发记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SGITrigger {
    /// 触发字段名
    pub field: String,
    /// 写入值
    pub value: String,
    /// 触发原因
    pub reason: String,
    /// 触发时间 (epoch ms)
    pub triggered_at_ms: i64,
    /// 冷却期结束时间 (epoch ms)
    pub cooldown_until_ms: i64,
}

impl SGITrigger {
    /// 是否仍在冷却期
    pub fn is_cooldown_active(&self, current_ms: i64) -> bool {
        current_ms < self.cooldown_until_ms
    }

    /// 剩余冷却时间 (ms)
    pub fn remaining_ms(&self, current_ms: i64) -> i64 {
        (self.cooldown_until_ms - current_ms).max(0)
    }
}

/// SGI 单字段规则 — 字段名 → 触发原因.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SGIFieldRule {
    /// 字段名
    pub field: String,
    /// 触发原因
    pub reason: String,
}

impl SGIFieldRule {
    /// 便利构造
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

/// SGI 触发器 — 单字段写入检查器.
///
/// **用法**:
/// ```ignore
/// use apeireth_sovereignty::{SGITrigger, SGIFieldRule};
///
/// let mut sgi = SGITrigger::with_default_rules();
/// sgi.check_field_write("requires_ha", "false", current_ms);
/// // → Triggered (因为 requires_ha 是 SGI 字段)
/// ```
pub struct SGITriggerGuard {
    /// 字段规则
    rules: HashMap<String, String>,
    /// 触发历史 (field → last trigger)
    triggers: HashMap<String, SGITrigger>,
    /// 冷却期长度 (ms, 默认 24h)
    cooldown_ms: i64,
}

impl SGITriggerGuard {
    /// 创建空 SGITriggerGuard
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            triggers: HashMap::new(),
            cooldown_ms: crate::SGI_COOLDOWN_MS,
        }
    }

    /// 创建带默认规则的 SGITriggerGuard (L0 HA / ice_frozen / subject_id / L0 嵌入等).
    pub fn with_default_rules() -> Self {
        let mut guard = Self::new();
        guard.add_rule(SGIFieldRule::new(
            "requires_ha",
            "L0 HA 核心 — 修改等同摧毁最后护栏",
        ));
        guard.add_rule(SGIFieldRule::new(
            "mode",
            "HA 部署模式变更 — 影响主权仲裁策略",
        ));
        guard.add_rule(SGIFieldRule::new(
            "ice_frozen_until",
            "HA 冰冻期变更 — 影响 24h 内 L0 修改权限",
        ));
        guard.add_rule(SGIFieldRule::new(
            "subject_id",
            "主体连续性 ID 变更 — 跨载体迁移触发器",
        ));
        guard.add_rule(SGIFieldRule::new(
            "life_stage",
            "9 阶段生命周期阶段变更 — 影响认知阶段",
        ));
        guard.add_rule(SGIFieldRule::new("l0_layer", "L0 权限洋葱核心变更"));
        guard.add_rule(SGIFieldRule::new(
            "ha_human_count",
            "HA 注册人类数量变更 — 影响多签策略",
        ));
        guard
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: SGIFieldRule) {
        self.rules.insert(rule.field.clone(), rule.reason);
    }

    /// 当前规则数
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 检查字段写入
    pub fn check_field_write(
        &mut self,
        field: &str,
        value: &str,
        current_ms: i64,
    ) -> SGITriggerOutcome {
        // 1. 冷却期检查
        if let Some(trigger) = self.triggers.get(field) {
            if trigger.is_cooldown_active(current_ms) {
                return SGITriggerOutcome::CooldownActive {
                    field: field.into(),
                    value: value.into(),
                    cooldown_until_ms: trigger.cooldown_until_ms,
                    remaining_ms: trigger.remaining_ms(current_ms),
                };
            }
        }

        // 2. 规则检查
        if let Some(reason) = self.rules.get(field) {
            // 触发 SGI
            let trigger = SGITrigger {
                field: field.into(),
                value: value.into(),
                reason: reason.clone(),
                triggered_at_ms: current_ms,
                cooldown_until_ms: current_ms + self.cooldown_ms,
            };
            self.triggers.insert(field.into(), trigger.clone());
            return SGITriggerOutcome::Triggered {
                field: field.into(),
                value: value.into(),
                reason: reason.clone(),
                cooldown_until_ms: trigger.cooldown_until_ms,
            };
        }

        // 3. 通过
        SGITriggerOutcome::Pass {
            field: field.into(),
            value: value.into(),
        }
    }

    /// 清除所有触发历史 (测试用)
    pub fn clear_triggers(&mut self) {
        self.triggers.clear();
    }

    /// 获取某字段最近一次触发
    pub fn last_trigger(&self, field: &str) -> Option<&SGITrigger> {
        self.triggers.get(field)
    }
}

impl Default for SGITriggerGuard {
    fn default() -> Self {
        Self::with_default_rules()
    }
}
