//! apeireth-team-lead - Team lead coordinator (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 TeamLead + 真 task assignment

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask { pub id: String, pub assignee: String, pub priority: u8, pub done: bool }

pub struct TeamLead { pub tasks: HashMap<String, TeamTask> }

impl TeamLead {
    pub fn new() -> Self { Self { tasks: HashMap::new() } }
    pub fn assign(&mut self, t: TeamTask) { self.tasks.insert(t.id.clone(), t); }
    pub fn complete(&mut self, id: &str) -> bool {
        if let Some(t) = self.tasks.get_mut(id) { t.done = true; true } else { false }
    }
    pub fn pending(&self) -> Vec<&TeamTask> {
        self.tasks.values().filter(|t| !t.done).collect()
    }
}

impl Default for TeamLead { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_assign_complete() {
        let mut l = TeamLead::new();
        l.assign(TeamTask { id: "t".into(), assignee: "a".into(), priority: 5, done: false });
        assert!(l.complete("t"));
        assert_eq!(l.pending().len(), 0);
    }
    #[test]
    fn test_complete_unknown() {
        let mut l = TeamLead::new();
        assert!(!l.complete("missing"));
    }
}
