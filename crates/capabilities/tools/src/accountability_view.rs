//! 问责视图层：`AccountabilityStats` 类型与它的 JSON 渲染。
//!
//! 上位：`docs/04-internal/backend-enhancement/accountability-loop-design-2026-10-23.md` §4
//! （「自述面 `self_status` | 新增只读子对象：近期影响声明计数 / 待确认回执数 / 未处理通知数」）。
//!
//! # 为什么单独成文件
//!
//! `self_status.rs` 在 arch_guard 尺寸基线内（只减不增）。视图层独立成文件后，
//! 账本字段的增删不再被那个文件的行数卡住 —— 与 `memory_ledger_view.rs` 同款做法。
//!
//! # 语义边界
//!
//! **只回计数，永不回内容原文**：只有数字。缺失的计数显式 `null` 并在同节
//! `reason` 给出原因，不编数。
//!
//! 挂在 `capabilities` 节内的子对象上（**不新增顶层节** —— 顶层仍是 21 节），
//! 与 `capabilities.bio_mechanisms` 同款做法。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::self_status::{SelfStatusSnapshot, DATA_PROBE_NOT_WIRED};

/// 问责计数（纯计数元数据；缺失的计数显式 `None` → JSON `null`）。
///
/// **永不回内容原文**：只有数字，没有行动内容、没有事实内容。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AccountabilityStats {
    /// 影响声明总数。
    pub impact_statements: Option<u64>,
    /// 不可逆的行动数。
    pub irreversible_actions: Option<u64>,
    /// 影响到他人（非 self）的行动数。
    pub actions_affecting_others: Option<u64>,
    /// 后果已确认成功的行动数。
    pub outcomes_confirmed: Option<u64>,
    /// 后果已确认失败的行动数。
    pub outcomes_failed: Option<u64>,
    /// 后果仍未知（**显式**：我们不知道结果）的行动数。
    pub outcomes_unknown: Option<u64>,
    /// 需要补救的行动数（失败 + 不可逆且未知）。
    pub needs_remedy: Option<u64>,
    /// 缺失原因（数据源没接上时给出）。
    pub reason: Option<String>,
}

/// 渲染 `capabilities.accountability` 子对象。
///
/// 观测面未挂载 = 省略该键（与 `bio_mechanisms` 同款：旧帧形状不变）。
pub fn render_accountability(stats: &AccountabilityStats, reason: Option<String>) -> Value {
    json!({
        "impact_statements": stats.impact_statements,
        "irreversible_actions": stats.irreversible_actions,
        "actions_affecting_others": stats.actions_affecting_others,
        "outcomes_confirmed": stats.outcomes_confirmed,
        "outcomes_failed": stats.outcomes_failed,
        "outcomes_unknown": stats.outcomes_unknown,
        "needs_remedy": stats.needs_remedy,
        "reason": reason.or_else(|| stats.reason.clone()),
    })
}

/// 从快照取问责子对象（`None` = 未挂载）。
pub fn snapshot_accountability(
    snapshot: &SelfStatusSnapshot,
) -> Option<(AccountabilityStats, Option<String>)> {
    let stats = snapshot.accountability.clone()?;
    Some((stats, snapshot.accountability_reason.clone()))
}

/// 未挂载时的原因文案（供调用方复用）。
pub fn not_wired_reason() -> String {
    DATA_PROBE_NOT_WIRED.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_counts_render_as_null_with_reason() {
        let stats = AccountabilityStats::default();
        let value = render_accountability(&stats, Some("问责观测面未接".to_string()));
        assert!(value["impact_statements"].is_null());
        assert_eq!(value["reason"], "问责观测面未接");
    }

    #[test]
    fn counts_render_as_numbers() {
        let stats = AccountabilityStats {
            impact_statements: Some(7),
            irreversible_actions: Some(3),
            actions_affecting_others: Some(1),
            outcomes_confirmed: Some(2),
            outcomes_failed: Some(1),
            outcomes_unknown: Some(4),
            needs_remedy: Some(5),
            reason: None,
        };
        let value = render_accountability(&stats, None);
        assert_eq!(value["impact_statements"], 7);
        assert_eq!(value["outcomes_unknown"], 4);
        assert_eq!(value["needs_remedy"], 5);
    }

    #[test]
    fn rendered_object_never_contains_content() {
        let stats = AccountabilityStats {
            impact_statements: Some(1),
            ..Default::default()
        };
        let raw = render_accountability(&stats, None).to_string();
        // 只有计数与键名，没有内容。
        assert!(!raw.contains("alice"));
        assert!(!raw.contains("@"));
    }
}
