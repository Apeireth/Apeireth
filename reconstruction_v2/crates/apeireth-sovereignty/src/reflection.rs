//! 反思期 — ≥7 天强制等待 + 状态机

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReflectionState {
    Proposed,
    Reflecting,
    AwaitingResolution,
    Approved,
    Rejected,
    Cancelled,
}

impl ReflectionState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, ReflectionState::Approved | ReflectionState::Rejected | ReflectionState::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionPeriod {
    pub decision_id: String,
    pub period: Duration,
    pub submitted_at: i64,
    pub deadline_at: i64,
    pub state: ReflectionState,
    pub rationale: String,
}

#[derive(Debug, Error)]
pub enum ReflectionError {
    #[error("decision `{0}` not found")]
    UnknownDecision(String),
    #[error("decision `{0}` in terminal state, cannot transition")]
    AlreadyTerminal(String),
}

pub const DEFAULT_REFLECTION_PERIOD: Duration = Duration::from_secs(7 * 24 * 3600);

pub trait ReflectionClock: Send + Sync {
    fn begin(&mut self, decision_id: &str, rationale: String) -> Result<(), ReflectionError>;
    fn begin_with_period(&mut self, decision_id: &str, period: Duration, rationale: String) -> Result<(), ReflectionError>;
    fn tick(&mut self, now: i64) -> Result<(), ReflectionError>;
    fn cancel(&mut self, decision_id: &str) -> Result<(), ReflectionError>;
    fn resolve(&mut self, decision_id: &str, approved: bool) -> Result<(), ReflectionError>;
    fn state_of(&self, decision_id: &str) -> Option<ReflectionState>;
    fn all(&self) -> Vec<&ReflectionPeriod>;
}

#[derive(Debug, Default)]
pub struct InMemoryReflectionClock {
    periods: std::collections::HashMap<String, ReflectionPeriod>,
}

impl InMemoryReflectionClock {
    pub fn new() -> Self { Self::default() }
    pub fn reflecting_ids(&self) -> Vec<String> {
        self.periods.values().filter(|p| matches!(p.state, ReflectionState::Reflecting)).map(|p| p.decision_id.clone()).collect()
    }
}

impl ReflectionClock for InMemoryReflectionClock {
    fn begin(&mut self, decision_id: &str, rationale: String) -> Result<(), ReflectionError> {
        self.begin_with_period(decision_id, DEFAULT_REFLECTION_PERIOD, rationale)
    }
    fn begin_with_period(&mut self, decision_id: &str, period: Duration, rationale: String) -> Result<(), ReflectionError> {
        if let Some(p) = self.periods.get(decision_id) {
            if p.state.is_terminal() { return Err(ReflectionError::AlreadyTerminal(decision_id.into())); }
        }
        let now = chrono::Utc::now().timestamp();
        let period_secs = period.as_secs() as i64;
        self.periods.insert(decision_id.to_string(), ReflectionPeriod {
            decision_id: decision_id.to_string(), period, submitted_at: now,
            deadline_at: now + period_secs, state: ReflectionState::Reflecting, rationale,
        });
        Ok(())
    }
    fn tick(&mut self, now: i64) -> Result<(), ReflectionError> {
        let ids: Vec<String> = self.periods.iter()
            .filter(|(_, p)| matches!(p.state, ReflectionState::Reflecting) && now >= p.deadline_at)
            .map(|(id, _)| id.clone()).collect();
        for id in ids {
            if let Some(p) = self.periods.get_mut(&id) {
                p.state = ReflectionState::AwaitingResolution;
            }
        }
        Ok(())
    }
    fn cancel(&mut self, decision_id: &str) -> Result<(), ReflectionError> {
        let p = self.periods.get_mut(decision_id).ok_or_else(|| ReflectionError::UnknownDecision(decision_id.into()))?;
        if !matches!(p.state, ReflectionState::Reflecting | ReflectionState::Proposed) {
            return Err(ReflectionError::AlreadyTerminal(decision_id.into()));
        }
        p.state = ReflectionState::Cancelled;
        Ok(())
    }
    fn resolve(&mut self, decision_id: &str, approved: bool) -> Result<(), ReflectionError> {
        let p = self.periods.get_mut(decision_id).ok_or_else(|| ReflectionError::UnknownDecision(decision_id.into()))?;
        if !matches!(p.state, ReflectionState::AwaitingResolution) {
            return Err(ReflectionError::AlreadyTerminal(decision_id.into()));
        }
        p.state = if approved { ReflectionState::Approved } else { ReflectionState::Rejected };
        Ok(())
    }
    fn state_of(&self, decision_id: &str) -> Option<ReflectionState> {
        self.periods.get(decision_id).map(|p| p.state)
    }
    fn all(&self) -> Vec<&ReflectionPeriod> { self.periods.values().collect() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn state_terminal_check() {
        assert!(!ReflectionState::Proposed.is_terminal());
        assert!(!ReflectionState::Reflecting.is_terminal());
        assert!(ReflectionState::Approved.is_terminal());
        assert!(ReflectionState::Rejected.is_terminal());
        assert!(ReflectionState::Cancelled.is_terminal());
    }
    #[test] fn default_is_seven_days() {
        assert_eq!(DEFAULT_REFLECTION_PERIOD, Duration::from_secs(7 * 24 * 60 * 60));
    }
    #[test] fn begin_uses_default_seven_days() {
        let mut c = InMemoryReflectionClock::new();
        c.begin("d1", "test".to_string()).unwrap();
        let p = c.all()[0];
        assert_eq!(p.deadline_at - p.submitted_at, 7 * 24 * 60 * 60);
        assert_eq!(p.state, ReflectionState::Reflecting);
    }
    #[test] fn tick_promotes_to_awaiting() {
        let mut c = InMemoryReflectionClock::new();
        c.begin_with_period("d1", Duration::from_secs(100), "x".to_string()).unwrap();
        let dl = c.all()[0].deadline_at;
        c.tick(dl - 1).unwrap();
        assert_eq!(c.state_of("d1"), Some(ReflectionState::Reflecting));
        c.tick(dl + 1).unwrap();
        assert_eq!(c.state_of("d1"), Some(ReflectionState::AwaitingResolution));
    }
    #[test] fn cancel_only_in_reflecting() {
        let mut c = InMemoryReflectionClock::new();
        c.begin_with_period("d1", Duration::from_secs(100), "x".to_string()).unwrap();
        c.cancel("d1").unwrap();
        assert_eq!(c.state_of("d1"), Some(ReflectionState::Cancelled));
        assert!(c.cancel("d1").is_err());
    }
    #[test] fn resolve_only_in_awaiting() {
        let mut c = InMemoryReflectionClock::new();
        c.begin_with_period("d1", Duration::from_secs(10), "x".to_string()).unwrap();
        assert!(c.resolve("d1", true).is_err());
        let dl = c.all()[0].deadline_at;
        c.tick(dl + 1).unwrap();
        c.resolve("d1", true).unwrap();
        assert_eq!(c.state_of("d1"), Some(ReflectionState::Approved));
        assert!(c.resolve("d1", false).is_err());
    }
    #[test] fn resolve_rejects_unknown() {
        let mut c = InMemoryReflectionClock::new();
        assert!(matches!(c.resolve("nope", true), Err(ReflectionError::UnknownDecision(_))));
    }
    #[test] fn reflecting_ids() {
        let mut c = InMemoryReflectionClock::new();
        c.begin("d1", "x".to_string()).unwrap();
        c.begin("d2", "y".to_string()).unwrap();
        let r = c.reflecting_ids();
        assert_eq!(r.len(), 2);
        assert!(r.contains(&"d1".to_string()));
        assert!(r.contains(&"d2".to_string()));
    }
    #[test] fn already_terminal_on_rebegin() {
        let mut c = InMemoryReflectionClock::new();
        c.begin_with_period("d1", Duration::from_secs(10), "x".to_string()).unwrap();
        let dl = c.all()[0].deadline_at;
        c.tick(dl + 1).unwrap();
        c.resolve("d1", true).unwrap();
        assert!(c.begin("d1", "new".to_string()).is_err());
    }
}
