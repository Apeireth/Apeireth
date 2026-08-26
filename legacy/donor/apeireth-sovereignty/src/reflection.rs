//! 反思期 — ≥7 天强制等待 + 状态机
//!
//! **设计** (阶段 1 §18.6 + 阶段 2 D2):
//! - 关键操作需等待反思期 (默认 ≥7 天), 期间任何修改可被撤回
//! - 状态机: Proposed → Reflecting → AwaitingResolution → Approved / Rejected / Cancelled
//! - 反思期可配置 (默认 7 天, 测试用 50ms)
//!
//! **硬约束**: tokio::time, 不引入 chrono 业务逻辑 (仅 timestamp)

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 反思期状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReflectionState {
    /// 提案已提交
    Proposed,
    /// 反思期进行中 (等待 ≥7 天)
    Reflecting,
    /// 反思期已结束, 等待最终裁决
    AwaitingResolution,
    /// 已批准
    Approved,
    /// 已拒绝
    Rejected,
    /// 已撤回 (反思期内可撤回)
    Cancelled,
}

impl ReflectionState {
    /// 是否终态 (Approved / Rejected / Cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ReflectionState::Approved | ReflectionState::Rejected | ReflectionState::Cancelled
        )
    }
}

/// 反思期记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionPeriod {
    /// 决策 ID
    pub decision_id: String,
    /// 反思期长度 (默认 7 天)
    pub period: Duration,
    /// 提交时间 (epoch seconds)
    pub submitted_at: i64,
    /// 截止时间 (epoch seconds) = submitted_at + period.as_secs()
    pub deadline_at: i64,
    /// 当前状态
    pub state: ReflectionState,
    /// 提案理由
    pub rationale: String,
}

/// 反思期错误
#[derive(Debug, Error)]
pub enum ReflectionError {
    #[error("decision `{0}` not found")]
    UnknownDecision(String),
    #[error("decision `{0}` in terminal state, cannot transition")]
    AlreadyTerminal(String),
}

/// 默认反思期 — 7 天
pub const DEFAULT_REFLECTION_PERIOD: Duration = Duration::from_secs(7 * 24 * 3600);

/// 反思期 trait
///
/// **dyn 兼容性**: `rationale` 参数为 `String` 而非 `impl Into<String>`
pub trait ReflectionClock: Send + Sync {
    /// 启动反思期 (进入 Reflecting 状态)
    fn begin(&mut self, decision_id: &str, rationale: String) -> Result<(), ReflectionError>;

    /// 设置自定义反思期长度 (用于测试 / 特殊场景)
    fn begin_with_period(
        &mut self,
        decision_id: &str,
        period: Duration,
        rationale: String,
    ) -> Result<(), ReflectionError>;

    /// 推进状态机 (检查是否进入 AwaitingResolution)
    fn tick(&mut self, now: i64) -> Result<(), ReflectionError>;

    /// 撤回决策 (仅在 Reflecting 期间有效)
    fn cancel(&mut self, decision_id: &str) -> Result<(), ReflectionError>;

    /// 决议 (仅在 AwaitingResolution 状态有效)
    fn resolve(&mut self, decision_id: &str, approved: bool) -> Result<(), ReflectionError>;

    /// 查询当前状态
    fn state_of(&self, decision_id: &str) -> Option<ReflectionState>;

    /// 查询当前所有反思期
    fn all(&self) -> Vec<&ReflectionPeriod>;
}

/// 反思期管理器 (内存 mock)
#[derive(Debug, Default)]
pub struct InMemoryReflectionClock {
    periods: std::collections::HashMap<String, ReflectionPeriod>,
}

impl InMemoryReflectionClock {
    /// 新建空 manager
    pub fn new() -> Self {
        Self::default()
    }

    /// 列出当前所有 Reflecting 中的决策 (辅助 API)
    pub fn reflecting_ids(&self) -> Vec<String> {
        self.periods
            .values()
            .filter(|p| matches!(p.state, ReflectionState::Reflecting))
            .map(|p| p.decision_id.clone())
            .collect()
    }
}

impl ReflectionClock for InMemoryReflectionClock {
    fn begin(&mut self, decision_id: &str, rationale: String) -> Result<(), ReflectionError> {
        self.begin_with_period(decision_id, DEFAULT_REFLECTION_PERIOD, rationale)
    }

    fn begin_with_period(
        &mut self,
        decision_id: &str,
        period: Duration,
        rationale: String,
    ) -> Result<(), ReflectionError> {
        if let Some(p) = self.periods.get(decision_id) {
            if p.state.is_terminal() {
                return Err(ReflectionError::AlreadyTerminal(decision_id.into()));
            }
        }
        let now = chrono::Utc::now().timestamp();
        let period_secs = period.as_secs() as i64;
        self.periods.insert(
            decision_id.to_string(),
            ReflectionPeriod {
                decision_id: decision_id.to_string(),
                period,
                submitted_at: now,
                deadline_at: now + period_secs,
                state: ReflectionState::Reflecting,
                rationale,
            },
        );
        Ok(())
    }

    fn tick(&mut self, now: i64) -> Result<(), ReflectionError> {
        let ids: Vec<String> = self
            .periods
            .iter()
            .filter(|(_, p)| matches!(p.state, ReflectionState::Reflecting) && now >= p.deadline_at)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(p) = self.periods.get_mut(&id) {
                p.state = ReflectionState::AwaitingResolution;
            }
        }
        Ok(())
    }

    fn cancel(&mut self, decision_id: &str) -> Result<(), ReflectionError> {
        let p = self
            .periods
            .get_mut(decision_id)
            .ok_or_else(|| ReflectionError::UnknownDecision(decision_id.into()))?;
        if !matches!(
            p.state,
            ReflectionState::Reflecting | ReflectionState::Proposed
        ) {
            return Err(ReflectionError::AlreadyTerminal(decision_id.into()));
        }
        p.state = ReflectionState::Cancelled;
        Ok(())
    }

    fn resolve(&mut self, decision_id: &str, approved: bool) -> Result<(), ReflectionError> {
        let p = self
            .periods
            .get_mut(decision_id)
            .ok_or_else(|| ReflectionError::UnknownDecision(decision_id.into()))?;
        if !matches!(p.state, ReflectionState::AwaitingResolution) {
            return Err(ReflectionError::AlreadyTerminal(decision_id.into()));
        }
        p.state = if approved {
            ReflectionState::Approved
        } else {
            ReflectionState::Rejected
        };
        Ok(())
    }

    fn state_of(&self, decision_id: &str) -> Option<ReflectionState> {
        self.periods.get(decision_id).map(|p| p.state)
    }

    fn all(&self) -> Vec<&ReflectionPeriod> {
        self.periods.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_state_terminal_check() {
        assert!(!ReflectionState::Proposed.is_terminal());
        assert!(!ReflectionState::Reflecting.is_terminal());
        assert!(ReflectionState::Approved.is_terminal());
        assert!(ReflectionState::Rejected.is_terminal());
        assert!(ReflectionState::Cancelled.is_terminal());
    }

    #[test]
    fn reflection_period_default_is_seven_days() {
        assert_eq!(
            DEFAULT_REFLECTION_PERIOD,
            Duration::from_secs(7 * 24 * 60 * 60)
        );
    }

    #[test]
    fn reflection_begin_uses_default_seven_days() {
        let mut clock = InMemoryReflectionClock::new();
        clock.begin("d1", "test".to_string()).unwrap();
        let p = clock.all()[0];
        assert_eq!(p.deadline_at - p.submitted_at, 7 * 24 * 60 * 60);
        assert_eq!(p.state, ReflectionState::Reflecting);
    }

    #[test]
    fn reflection_tick_promotes_to_awaiting() {
        let mut clock = InMemoryReflectionClock::new();
        clock
            .begin_with_period("d1", Duration::from_secs(100), "x".to_string())
            .unwrap();
        // 先记下 submitted_at → deadline_at
        let deadline = clock.all()[0].deadline_at;
        clock.tick(deadline - 1).unwrap(); // 未到
        assert_eq!(clock.state_of("d1"), Some(ReflectionState::Reflecting));
        clock.tick(deadline + 1).unwrap(); // 超过
        assert_eq!(
            clock.state_of("d1"),
            Some(ReflectionState::AwaitingResolution)
        );
    }

    #[test]
    fn reflection_cancel_only_in_reflecting() {
        let mut clock = InMemoryReflectionClock::new();
        clock
            .begin_with_period("d1", Duration::from_secs(100), "x".to_string())
            .unwrap();
        clock.cancel("d1").unwrap();
        assert_eq!(clock.state_of("d1"), Some(ReflectionState::Cancelled));
        // 终态再 cancel 报错
        assert!(clock.cancel("d1").is_err());
    }

    #[test]
    fn reflection_resolve_only_in_awaiting() {
        let mut clock = InMemoryReflectionClock::new();
        clock
            .begin_with_period("d1", Duration::from_secs(10), "x".to_string())
            .unwrap();
        // Reflecting 状态不能 resolve
        assert!(clock.resolve("d1", true).is_err());
        // 推进到 AwaitingResolution
        let deadline = clock.all()[0].deadline_at;
        clock.tick(deadline + 1).unwrap();
        assert_eq!(
            clock.state_of("d1"),
            Some(ReflectionState::AwaitingResolution)
        );
        clock.resolve("d1", true).unwrap();
        assert_eq!(clock.state_of("d1"), Some(ReflectionState::Approved));
        // 第二次 resolve 在 Approved 状态应失败 (终态)
        assert!(clock.resolve("d1", false).is_err());
    }
}
