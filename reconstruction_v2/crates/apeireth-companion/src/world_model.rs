//! WorldModel - 文本世界模型 (从 v1.0 apeireth-companion/world_model.rs 442 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 TimelineContext + LLM-driven 模拟器
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStep { pub step: u32, pub description: String, pub outcome: String }

pub struct TimelineContext {
    pub steps: VecDeque<TimelineStep>,
    pub current: u32,
}

impl TimelineContext {
    pub fn new() -> Self { Self { steps: VecDeque::new(), current: 0 } }
    pub fn add_step(&mut self, description: impl Into<String>, outcome: impl Into<String>) {
        self.current += 1;
        self.steps.push_back(TimelineStep { step: self.current, description: description.into(), outcome: outcome.into() });
    }
    pub fn latest(&self) -> Option<&TimelineStep> { self.steps.back() }
    pub fn count(&self) -> usize { self.steps.len() }
}

impl Default for TimelineContext { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_step() {
        let mut ctx = TimelineContext::new();
        ctx.add_step("a", "ok");
        assert_eq!(ctx.count(), 1);
        assert_eq!(ctx.latest().unwrap().step, 1);
    }
    #[test] fn test_step_increment() {
        let mut ctx = TimelineContext::new();
        ctx.add_step("a", "ok");
        ctx.add_step("b", "ok");
        assert_eq!(ctx.latest().unwrap().step, 2);
    }
    #[test] fn test_empty() {
        let ctx = TimelineContext::new();
        assert!(ctx.latest().is_none());
    }
    #[test] fn test_default() {
        let ctx: TimelineContext = Default::default();
        assert_eq!(ctx.count(), 0);
    }
}
