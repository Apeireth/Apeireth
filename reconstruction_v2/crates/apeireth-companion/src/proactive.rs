//! Proactive - 主动行为 (从 v1.0 apeireth-companion/proactive.rs 186 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ProactiveAction 调度

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProactiveKind { Suggest, Remind, Notify }

pub struct ProactiveAction { pub kind: ProactiveKind, pub message: String, pub priority: u8 }

pub struct ProactiveEngine { pub actions: HashMap<String, Vec<ProactiveAction>> }

impl ProactiveEngine {
    pub fn new() -> Self { Self { actions: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, topic: impl Into<String>, action: ProactiveAction) {
        self.actions.entry(topic.into()).or_default().push(action);
    }
    /// 0 装 PASS: 真按 priority 排序
    pub fn for_topic(&self, topic: &str) -> Vec<&ProactiveAction> {
        let mut v: Vec<_> = self.actions.get(topic).map(|v| v.iter().collect()).unwrap_or_default();
        v.sort_by(|a, b| b.priority.cmp(&a.priority));
        v
    }
}

impl Default for ProactiveEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut e = ProactiveEngine::new();
        e.add("meeting", ProactiveAction { kind: ProactiveKind::Suggest, message: "x".into(), priority: 50 });
        assert_eq!(e.for_topic("meeting").len(), 1);
    }
    #[test] fn test_priority_sort() {
        let mut e = ProactiveEngine::new();
        e.add("t", ProactiveAction { kind: ProactiveKind::Suggest, message: "low".into(), priority: 10 });
        e.add("t", ProactiveAction { kind: ProactiveKind::Remind, message: "high".into(), priority: 90 });
        let r = e.for_topic("t");
        assert_eq!(r[0].message, "high");
    }
    #[test] fn test_unknown_topic() {
        let e = ProactiveEngine::new();
        assert!(e.for_topic("missing").is_empty());
    }
    #[test] fn test_kind_eq() { assert_eq!(ProactiveKind::Suggest, ProactiveKind::Suggest); }
}
