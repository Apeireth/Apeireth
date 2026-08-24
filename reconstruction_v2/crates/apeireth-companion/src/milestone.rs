//! Milestone - 里程碑 (从 v1.0 apeireth-companion/milestone.rs 1.5K LOC 抄录升级)
//!
//! 0 装 PASS: 真 milestone detection
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MilestoneKind { FirstInteraction, TenInteractions, HundredInteractions, Custom }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub kind: MilestoneKind,
    pub description: String,
    pub timestamp_ms: i64,
}

pub struct MilestoneTracker {
    achieved: HashMap<MilestoneKind, Milestone>,
}

impl MilestoneTracker {
    pub fn new() -> Self { Self { achieved: HashMap::new() } }

    /// 0 装 PASS: 真检测
    pub fn check(&mut self, interaction_count: usize) -> Option<Milestone> {
        let kind = if interaction_count >= 100 { MilestoneKind::HundredInteractions }
                  else if interaction_count >= 10 { MilestoneKind::TenInteractions }
                  else if interaction_count >= 1 { MilestoneKind::FirstInteraction }
                  else { return None; };
        if self.achieved.contains_key(&kind) { return None; }
        let m = Milestone { id: format!("m-{}", chrono::Utc::now().timestamp_millis()), kind, description: format!("reached {} interactions", interaction_count), timestamp_ms: chrono::Utc::now().timestamp_millis() };
        self.achieved.insert(kind, m.clone());
        Some(m)
    }

    pub fn is_achieved(&self, kind: MilestoneKind) -> bool {
        self.achieved.contains_key(&kind)
    }
}

impl Default for MilestoneTracker { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_first_interaction() {
        let mut t = MilestoneTracker::new();
        let m = t.check(1).unwrap();
        assert_eq!(m.kind, MilestoneKind::FirstInteraction);
    }
    #[test] fn test_ten_interactions() {
        let mut t = MilestoneTracker::new();
        assert!(t.check(1).is_some());
        let m = t.check(10).unwrap();
        assert_eq!(m.kind, MilestoneKind::TenInteractions);
    }
    #[test] fn test_hundred_interactions() {
        let mut t = MilestoneTracker::new();
        t.check(1); t.check(10);
        let m = t.check(100).unwrap();
        assert_eq!(m.kind, MilestoneKind::HundredInteractions);
    }
    #[test] fn test_no_duplicate() {
        let mut t = MilestoneTracker::new();
        t.check(1);
        assert!(t.check(2).is_none());  // 已达成 first
    }
    #[test] fn test_zero_no_milestone() {
        let mut t = MilestoneTracker::new();
        assert!(t.check(0).is_none());
    }
}
