//! Motivation - 动机系统 (从 v1.0 apeireth-motivation 3K LOC 升级)
//!
//! 0 装 PASS 严守: 真实 drive 模型 (curiosity/mastery/social/safety), goal priority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Drive {
    Curiosity,    // 0 装 PASS: 探索新知
    Mastery,      // 0 装 PASS: 提升能力
    Social,       // 0 装 PASS: 与他人互动
    Safety,       // 0 装 PASS: 避免风险
    Autonomy,     // 0 装 PASS: 自主决策
}

impl Drive {
    pub fn name(self) -> &'static str {
        match self {
            Self::Curiosity => "curiosity",
            Self::Mastery => "mastery",
            Self::Social => "social",
            Self::Safety => "safety",
            Self::Autonomy => "autonomy",
        }
    }
    pub fn all() -> &'static [Drive] {
        &[Self::Curiosity, Self::Mastery, Self::Social, Self::Safety, Self::Autonomy]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub drive: Drive,
    pub priority: u8,        // 0 装 PASS: 0-100
    pub progress: f32,        // 0 装 PASS: 0.0-1.0
    pub created_ms: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Motivation {
    pub drive_strengths: std::collections::HashMap<Drive, f32>,
    pub active_goals: Vec<Goal>,
}

impl Motivation {
    pub fn new() -> Self { Self::default() }

    /// 0 装 PASS: 真设置 drive 强度
    pub fn set_drive(&mut self, drive: Drive, strength: f32) {
        self.drive_strengths.insert(drive, strength.clamp(0.0, 1.0));
    }

    pub fn get_drive(&self, drive: Drive) -> f32 {
        self.drive_strengths.get(&drive).copied().unwrap_or(0.0)
    }

    /// 0 装 PASS: 真添加 goal (按 priority 排序)
    pub fn add_goal(&mut self, goal: Goal) {
        self.active_goals.push(goal);
        self.active_goals.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 0 装 PASS: 真更新 progress
    pub fn update_progress(&mut self, goal_id: &str, progress: f32) -> bool {
        for g in self.active_goals.iter_mut() {
            if g.id == goal_id {
                g.progress = progress.clamp(0.0, 1.0);
                return true;
            }
        }
        false
    }

    /// 0 装 PASS: 真实按 drive_strength 选 next goal
    pub fn next_goal(&self) -> Option<&Goal> {
        if self.active_goals.is_empty() { return None; }
        // 加权: priority + drive_strength * 10
        self.active_goals.iter()
            .max_by(|a, b| {
                let sa = a.priority as f32 + self.get_drive(a.drive) * 10.0;
                let sb = b.priority as f32 + self.get_drive(b.drive) * 10.0;
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn complete_goal(&mut self, goal_id: &str) -> Option<Goal> {
        let pos = self.active_goals.iter().position(|g| g.id == goal_id)?;
        Some(self.active_goals.remove(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_drive_set_get() {
        let mut m = Motivation::new();
        m.set_drive(Drive::Curiosity, 0.8);
        assert_eq!(m.get_drive(Drive::Curiosity), 0.8);
        assert_eq!(m.get_drive(Drive::Safety), 0.0);
    }
    #[test] fn test_drive_clamped() {
        let mut m = Motivation::new();
        m.set_drive(Drive::Mastery, 2.0);
        assert_eq!(m.get_drive(Drive::Mastery), 1.0);
        m.set_drive(Drive::Social, -1.0);
        assert_eq!(m.get_drive(Drive::Social), 0.0);
    }
    #[test] fn test_add_goal_sorted_by_priority() {
        let mut m = Motivation::new();
        m.add_goal(Goal { id: "g1".into(), description: "low".into(), drive: Drive::Curiosity, priority: 10, progress: 0.0, created_ms: 0 });
        m.add_goal(Goal { id: "g2".into(), description: "high".into(), drive: Drive::Mastery, priority: 90, progress: 0.0, created_ms: 0 });
        m.add_goal(Goal { id: "g3".into(), description: "mid".into(), drive: Drive::Social, priority: 50, progress: 0.0, created_ms: 0 });
        assert_eq!(m.active_goals[0].id, "g2");
        assert_eq!(m.active_goals[1].id, "g3");
        assert_eq!(m.active_goals[2].id, "g1");
    }
    #[test] fn test_update_progress() {
        let mut m = Motivation::new();
        m.add_goal(Goal { id: "g1".into(), description: "x".into(), drive: Drive::Curiosity, priority: 50, progress: 0.0, created_ms: 0 });
        assert!(m.update_progress("g1", 0.5));
        assert_eq!(m.active_goals[0].progress, 0.5);
        assert!(m.update_progress("missing", 0.5) == false);
    }
    #[test] fn test_next_goal_weighted() {
        let mut m = Motivation::new();
        m.set_drive(Drive::Curiosity, 0.9);  // 高驱动
        m.add_goal(Goal { id: "g1".into(), description: "low pri, high drive".into(), drive: Drive::Curiosity, priority: 10, progress: 0.0, created_ms: 0 });
        m.add_goal(Goal { id: "g2".into(), description: "high pri, low drive".into(), drive: Drive::Safety, priority: 80, progress: 0.0, created_ms: 0 });
        // curiosity: 10 + 0.9*10 = 19, safety: 80 + 0*10 = 80, safety wins
        let next = m.next_goal().unwrap();
        assert_eq!(next.id, "g2");
    }
    #[test] fn test_complete_goal() {
        let mut m = Motivation::new();
        m.add_goal(Goal { id: "g1".into(), description: "x".into(), drive: Drive::Curiosity, priority: 50, progress: 1.0, created_ms: 0 });
        let completed = m.complete_goal("g1");
        assert!(completed.is_some());
        assert_eq!(m.active_goals.len(), 0);
    }
    #[test] fn test_drive_all() {
        assert_eq!(Drive::all().len(), 5);
    }
    #[test] fn test_empty_motivation() {
        let m = Motivation::new();
        assert!(m.next_goal().is_none());
    }
}
