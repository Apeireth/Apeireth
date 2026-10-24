//! 问责闭环 · 第 3 步：后果追踪。
//!
//! 上位：`docs/04-internal/backend-enhancement/accountability-loop-design-2026-10-23.md` §2.3。
//!
//! # 问题
//!
//! 一次外部行动发出去之后，**我们不知道结果**：
//! 邮件发了吗？申请提交了吗？购买完成了吗？
//!
//! 顾问的判据：**没有后果状态，系统就不知道自己是否需要补救。**
//!
//! # 三条纪律
//!
//! 1. **`unknown` 是一等公民**。不是每个行动都有可观测的后果
//!    （「把上下文送进模型」就没有「送达/未送达」这种二元结果）。
//!    把它们强行标成 `confirmed` 是**撒谎**。
//! 2. **不假装能追踪**。设计文档 §7 明确：*不追踪用户在本系统之外的行动*
//!    （管不了的，如实说管不了）。
//! 3. **回填是显式的**。后果由**回执渠道或用户确认**回填，
//!    系统不猜、不自动探测外部世界。
//!
//! # 与第 1 步的关系
//!
//! [`crate::accountability::Outcome`] 是声明里的**当前**状态；
//! 本模块管**状态如何变化**（`unknown → pending → confirmed/failed`）。
//! 事件是 append-only 的 —— 历史不被改写，只被追加。

use serde::{Deserialize, Serialize};

use crate::accountability::Outcome;

/// 后果事件（append-only）。
///
/// 「这条行动的后果变成了 X」—— 一条追加记录，**不改写**原声明。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsequenceEvent {
    /// 行动 id。
    pub action_id: String,
    /// 事件时刻（epoch 毫秒）。
    pub at_ms: i64,
    /// 事件之后的状态。
    pub outcome: Outcome,
    /// 依据（**零真值**：只指涉，如「用户确认」/「回执渠道回报」）。
    pub evidence: String,
}

/// 后果来源（闭合词表）—— **谁说的**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeSource {
    /// 用户确认。
    UserConfirmed,
    /// 回执渠道回报（外部系统告诉我们）。
    ChannelReported,
    /// 我们自己观测到的（本地可验证）。
    LocallyObserved,
    /// **无人回报** —— 状态仍是 `unknown`，但我们记下了「等了很久还没人管」。
    NoReply,
}

impl OutcomeSource {
    /// 稳定标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::UserConfirmed => "user_confirmed",
            Self::ChannelReported => "channel_reported",
            Self::LocallyObserved => "locally_observed",
            Self::NoReply => "no_reply",
        }
    }
}

/// 后果状态机。
///
/// 规则（都被测试钉死）：
/// - `unknown → pending`（开始等）；
/// - `pending → confirmed | failed`（有回报了）；
/// - `unknown → confirmed | failed`（直接确认也行）；
/// - **不能从 `confirmed` 退回 `unknown`**（已确认的事实不能被抹掉）；
/// - `unknown → unknown`（又等了一次，状态不变，但**留痕**了）。
#[derive(Debug, Clone, Default)]
pub struct ConsequenceTracker {
    events: Vec<ConsequenceEvent>,
}

impl ConsequenceTracker {
    /// 新建（空状态机）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次后果变化。
    ///
    /// 返回 `false` 表示**这次变化不被允许**（例如从 `confirmed` 退回 `unknown`），
    /// 事件**不会**被记录。
    pub fn record(
        &mut self,
        action_id: &str,
        outcome: Outcome,
        source: OutcomeSource,
        at_ms: i64,
    ) -> bool {
        let current = self.current_outcome(action_id);
        if !is_transition_allowed(current, outcome) {
            return false;
        }
        self.events.push(ConsequenceEvent {
            action_id: action_id.to_string(),
            at_ms,
            outcome,
            evidence: source.label().to_string(),
        });
        true
    }

    /// 某行动**当前**的后果状态（无记录 = `unknown`）。
    pub fn current_outcome(&self, action_id: &str) -> Outcome {
        self.events
            .iter()
            .filter(|event| event.action_id == action_id)
            .map(|event| event.outcome)
            .next_back()
            .unwrap_or(Outcome::Unknown)
    }

    /// 某行动的全部事件（时间序）。
    pub fn events_for(&self, action_id: &str) -> Vec<&ConsequenceEvent> {
        self.events
            .iter()
            .filter(|event| event.action_id == action_id)
            .collect()
    }

    /// 全部事件。
    pub fn events(&self) -> &[ConsequenceEvent] {
        &self.events
    }

    /// **需要补救**的行动：后果已确认为失败，**或**不可逆且仍是未知。
    ///
    /// 这就是「系统知道自己是否需要补救」那个判据的落地。
    pub fn needs_remedy(&self, irreversible_action_ids: &[String]) -> Vec<String> {
        self.events
            .iter()
            .filter(|event| {
                event.outcome == Outcome::Failed
                    || (irreversible_action_ids.contains(&event.action_id)
                        && event.outcome == Outcome::Unknown)
            })
            .map(|event| event.action_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// 计数（自述面用）。
    pub fn counts(&self) -> ConsequenceCounts {
        let mut counts = ConsequenceCounts::default();
        let ids: std::collections::BTreeSet<&String> =
            self.events.iter().map(|event| &event.action_id).collect();
        counts.tracked_actions = ids.len();
        for id in ids {
            match self.current_outcome(id) {
                Outcome::Confirmed => counts.confirmed += 1,
                Outcome::Failed => counts.failed += 1,
                Outcome::Pending => counts.pending += 1,
                Outcome::Unknown => counts.unknown += 1,
            }
        }
        counts
    }
}

/// 允许的状态迁移。
fn is_transition_allowed(from: Outcome, to: Outcome) -> bool {
    match from {
        Outcome::Confirmed | Outcome::Failed => to == from,
        Outcome::Unknown | Outcome::Pending => true,
    }
}

/// 后果计数（自述面用）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsequenceCounts {
    /// 被追踪的行动数。
    pub tracked_actions: usize,
    /// 已确认成功。
    pub confirmed: usize,
    /// 已确认失败。
    pub failed: usize,
    /// 等待中。
    pub pending: usize,
    /// **不知道**（显式）。
    pub unknown: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_record_means_unknown() {
        let tracker = ConsequenceTracker::new();
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Unknown);
        assert_eq!(tracker.counts().unknown, 0, "没记录的行动不算进计数");
    }

    #[test]
    fn unknown_to_pending_to_confirmed() {
        let mut tracker = ConsequenceTracker::new();
        assert!(tracker.record("act-1", Outcome::Pending, OutcomeSource::NoReply, 100));
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Pending);
        assert!(tracker.record(
            "act-1",
            Outcome::Confirmed,
            OutcomeSource::UserConfirmed,
            200
        ));
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Confirmed);
        assert_eq!(tracker.events_for("act-1").len(), 2);
    }

    #[test]
    fn confirmed_cannot_regress_to_unknown() {
        let mut tracker = ConsequenceTracker::new();
        tracker.record(
            "act-1",
            Outcome::Confirmed,
            OutcomeSource::UserConfirmed,
            100,
        );
        // 退回 unknown 被拒 —— 已确认的事实不能被抹掉。
        assert!(!tracker.record("act-1", Outcome::Unknown, OutcomeSource::NoReply, 200));
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Confirmed);
        // 被拒的事件不留痕。
        assert_eq!(tracker.events_for("act-1").len(), 1);
    }

    #[test]
    fn repeated_no_reply_keeps_unknown_but_leaves_trace() {
        let mut tracker = ConsequenceTracker::new();
        assert!(tracker.record("act-1", Outcome::Unknown, OutcomeSource::NoReply, 100));
        assert!(tracker.record("act-1", Outcome::Unknown, OutcomeSource::NoReply, 200));
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Unknown);
        assert_eq!(
            tracker.events_for("act-1").len(),
            2,
            "等了两次要有两次的留痕"
        );
    }

    #[test]
    fn needs_remedy_covers_failed_and_irreversible_unknown() {
        let mut tracker = ConsequenceTracker::new();
        tracker.record(
            "act-1",
            Outcome::Failed,
            OutcomeSource::ChannelReported,
            100,
        );
        tracker.record("act-2", Outcome::Unknown, OutcomeSource::NoReply, 100);
        tracker.record(
            "act-3",
            Outcome::Confirmed,
            OutcomeSource::UserConfirmed,
            100,
        );
        let irreversible = vec!["act-2".to_string()];
        let remedy = tracker.needs_remedy(&irreversible);
        assert!(remedy.contains(&"act-1".to_string()), "失败的行动要补救");
        assert!(
            remedy.contains(&"act-2".to_string()),
            "不可逆且未知的行动要补救"
        );
        assert!(
            !remedy.contains(&"act-3".to_string()),
            "已确认成功的不用补救"
        );
    }

    #[test]
    fn counts_reflect_current_outcomes() {
        let mut tracker = ConsequenceTracker::new();
        tracker.record(
            "act-1",
            Outcome::Confirmed,
            OutcomeSource::UserConfirmed,
            100,
        );
        tracker.record(
            "act-2",
            Outcome::Failed,
            OutcomeSource::ChannelReported,
            100,
        );
        tracker.record("act-3", Outcome::Pending, OutcomeSource::NoReply, 100);
        let counts = tracker.counts();
        assert_eq!(counts.tracked_actions, 3);
        assert_eq!(counts.confirmed, 1);
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.pending, 1);
    }
}
