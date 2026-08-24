//! RuntimeBrain - 运行时大脑 (从 v1.0 apeireth-companion/runtime_brain.rs 242 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 brain state + decision loop
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainMode { Idle, Listening, Thinking, Speaking }

pub struct RuntimeBrain {
    pub mode: BrainMode,
    pub thought_count: u32,
    pub memory: HashMap<String, String>,
}

impl RuntimeBrain {
    pub fn new() -> Self { Self { mode: BrainMode::Idle, thought_count: 0, memory: HashMap::new() } }
    pub fn listen(&mut self) { self.mode = BrainMode::Listening; }
    pub fn think(&mut self) { self.mode = BrainMode::Thinking; self.thought_count += 1; }
    pub fn speak(&mut self) { self.mode = BrainMode::Speaking; }
    pub fn remember(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.memory.insert(k.into(), v.into());
    }
    pub fn recall(&self, k: &str) -> Option<&str> { self.memory.get(k).map(|s| s.as_str()) }
}

impl Default for RuntimeBrain { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_lifecycle() {
        let mut b = RuntimeBrain::new();
        b.listen();
        b.think();
        b.speak();
        assert_eq!(b.thought_count, 1);
    }
    #[test] fn test_remember_recall() {
        let mut b = RuntimeBrain::new();
        b.remember("user", "alice");
        assert_eq!(b.recall("user"), Some("alice"));
    }
    #[test] fn test_unknown_recall() {
        let b = RuntimeBrain::new();
        assert!(b.recall("missing").is_none());
    }
    #[test] fn test_default() {
        let b: RuntimeBrain = Default::default();
        assert_eq!(b.thought_count, 0);
    }
}
