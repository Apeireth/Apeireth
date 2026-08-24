//! Reflection - 反思调度 (从 v1.0 apeireth-companion/reflection.rs 329 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ReflectionScheduler + 调度

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionTrigger { Periodic, EventBased, OnDemand }

pub struct ReflectionTask { pub id: String, pub trigger: ReflectionTrigger, pub timestamp_ms: i64 }

pub struct ReflectionScheduler {
    pub tasks: VecDeque<ReflectionTask>,
}

impl ReflectionScheduler {
    pub fn new() -> Self { Self { tasks: VecDeque::new() } }
    /// 0 装 PASS: 真 schedule
    pub fn schedule(&mut self, task: ReflectionTask) {
        self.tasks.push_back(task);
    }
    /// 0 装 PASS: 真按 trigger filter
    pub fn by_trigger(&self, trigger: ReflectionTrigger) -> Vec<&ReflectionTask> {
        self.tasks.iter().filter(|t| t.trigger == trigger).collect()
    }
    pub fn count(&self) -> usize { self.tasks.len() }
}

impl Default for ReflectionScheduler { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_schedule() {
        let mut s = ReflectionScheduler::new();
        s.schedule(ReflectionTask { id: "t1".into(), trigger: ReflectionTrigger::Periodic, timestamp_ms: 0 });
        assert_eq!(s.count(), 1);
    }
    #[test] fn test_by_trigger() {
        let mut s = ReflectionScheduler::new();
        s.schedule(ReflectionTask { id: "t1".into(), trigger: ReflectionTrigger::Periodic, timestamp_ms: 0 });
        s.schedule(ReflectionTask { id: "t2".into(), trigger: ReflectionTrigger::EventBased, timestamp_ms: 0 });
        assert_eq!(s.by_trigger(ReflectionTrigger::Periodic).len(), 1);
    }
    #[test] fn test_default() {
        let s: ReflectionScheduler = Default::default();
        assert_eq!(s.count(), 0);
    }
    #[test] fn test_trigger_eq() { assert_eq!(ReflectionTrigger::Periodic, ReflectionTrigger::Periodic); }
}
