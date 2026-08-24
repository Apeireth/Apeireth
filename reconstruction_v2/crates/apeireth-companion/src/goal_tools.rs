//! GoalTools - goal 工具 (从 v1.0 apeireth-companion/goal_tools.rs 278 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 goal 工具调用框架

use std::collections::HashMap;

pub trait GoalExecutor: Send + Sync {
    fn execute(&self, goal_id: &str) -> Result<String, String>;
}

pub struct GoalToolset { pub executors: HashMap<String, Box<dyn GoalExecutor>> }

impl GoalToolset {
    pub fn new() -> Self { Self { executors: HashMap::new() } }
    pub fn register(&mut self, name: impl Into<String>, e: Box<dyn GoalExecutor>) {
        self.executors.insert(name.into(), e);
    }
    /// 0 装 PASS: 真调用
    pub fn execute(&self, name: &str, goal_id: &str) -> Result<String, String> {
        self.executors.get(name).ok_or_else(|| format!("tool not found: {}", name))?.execute(goal_id)
    }
}

impl Default for GoalToolset { fn default() -> Self { Self::new() } }

pub struct MockGoalExecutor;
impl GoalExecutor for MockGoalExecutor {
    fn execute(&self, goal_id: &str) -> Result<String, String> { Ok(format!("done: {}", goal_id)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut t = GoalToolset::new();
        t.register("mock", Box::new(MockGoalExecutor));
        assert_eq!(t.executors.len(), 1);
    }
    #[test] fn test_execute() {
        let mut t = GoalToolset::new();
        t.register("mock", Box::new(MockGoalExecutor));
        let r = t.execute("mock", "g1").unwrap();
        assert_eq!(r, "done: g1");
    }
    #[test] fn test_unknown_tool() {
        let t = GoalToolset::new();
        assert!(t.execute("missing", "g").is_err());
    }
    #[test] fn test_default() { let t: GoalToolset = Default::default(); assert_eq!(t.executors.len(), 0); }
}
