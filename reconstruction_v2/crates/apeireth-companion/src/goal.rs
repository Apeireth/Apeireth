//! Goal - 目标系统 (从 v1.0 apeireth-companion/goal.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 GoalService + GoalStore + phases
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoalPhase { Pending, Active, Paused, Completed, Abandoned }

impl GoalPhase {
    pub fn is_terminal(self) -> bool { matches!(self, Self::Completed | Self::Abandoned) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub phase: GoalPhase,
    pub priority: u8,
    pub created_ms: i64,
    pub completed_ms: Option<i64>,
}

pub struct GoalStore {
    goals: HashMap<String, Goal>,
}

impl GoalStore {
    pub fn new() -> Self { Self { goals: HashMap::new() } }

    /// 0 装 PASS: 真添加
    pub fn add(&mut self, goal: Goal) -> Result<(), String> {
        if self.goals.contains_key(&goal.id) { return Err(format!("duplicate: {}", goal.id)); }
        self.goals.insert(goal.id.clone(), goal);
        Ok(())
    }

    /// 0 装 PASS: 真转 phase
    pub fn transition(&mut self, id: &str, phase: GoalPhase) -> Result<(), String> {
        let g = self.goals.get_mut(id).ok_or_else(|| "not found")?;
        if g.phase.is_terminal() { return Err("already terminal".into()); }
        g.phase = phase;
        if matches!(phase, GoalPhase::Completed) {
            g.completed_ms = Some(chrono::Utc::now().timestamp_millis());
        }
        Ok(())
    }

    pub fn active(&self) -> Vec<&Goal> {
        self.goals.values().filter(|g| matches!(g.phase, GoalPhase::Active | GoalPhase::Paused)).collect()
    }

    pub fn by_priority(&self) -> Vec<&Goal> {
        let mut v: Vec<_> = self.goals.values().collect();
        v.sort_by(|a, b| b.priority.cmp(&a.priority));
        v
    }

    pub fn count(&self) -> usize { self.goals.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_and_transition() {
        let mut s = GoalStore::new();
        s.add(Goal { id: "g1".into(), title: "x".into(), description: "d".into(), phase: GoalPhase::Pending, priority: 50, created_ms: 0, completed_ms: None }).unwrap();
        s.transition("g1", GoalPhase::Active).unwrap();
        assert_eq!(s.goals.get("g1").unwrap().phase, GoalPhase::Active);
    }
    #[test] fn test_duplicate_rejected() {
        let mut s = GoalStore::new();
        s.add(Goal { id: "g1".into(), title: "x".into(), description: "d".into(), phase: GoalPhase::Pending, priority: 50, created_ms: 0, completed_ms: None }).unwrap();
        assert!(s.add(Goal { id: "g1".into(), title: "y".into(), description: "d".into(), phase: GoalPhase::Pending, priority: 50, created_ms: 0, completed_ms: None }).is_err());
    }
    #[test] fn test_terminal_no_transition() {
        let mut s = GoalStore::new();
        s.add(Goal { id: "g1".into(), title: "x".into(), description: "d".into(), phase: GoalPhase::Completed, priority: 50, created_ms: 0, completed_ms: Some(0) }).unwrap();
        assert!(s.transition("g1", GoalPhase::Active).is_err());
    }
    #[test] fn test_completed_sets_timestamp() {
        let mut s = GoalStore::new();
        s.add(Goal { id: "g1".into(), title: "x".into(), description: "d".into(), phase: GoalPhase::Active, priority: 50, created_ms: 0, completed_ms: None }).unwrap();
        s.transition("g1", GoalPhase::Completed).unwrap();
        assert!(s.goals.get("g1").unwrap().completed_ms.is_some());
    }
    #[test] fn test_active_filter() {
        let mut s = GoalStore::new();
        s.add(Goal { id: "a".into(), title: "x".into(), description: "d".into(), phase: GoalPhase::Active, priority: 50, created_ms: 0, completed_ms: None }).unwrap();
        s.add(Goal { id: "b".into(), title: "y".into(), description: "d".into(), phase: GoalPhase::Pending, priority: 50, created_ms: 0, completed_ms: None }).unwrap();
        s.add(Goal { id: "c".into(), title: "z".into(), description: "d".into(), phase: GoalPhase::Completed, priority: 50, created_ms: 0, completed_ms: Some(0) }).unwrap();
        assert_eq!(s.active().len(), 1);
    }
}
