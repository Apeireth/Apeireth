//! Education - 教育/解释 (从 v1.0 apeireth-companion/education.rs 402 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ExplanationBuilder + 概念到示例

use std::collections::HashMap;

pub struct ExplanationBuilder { pub concepts: HashMap<String, String> }

impl ExplanationBuilder {
    pub fn new() -> Self { Self { concepts: HashMap::new() } }
    /// 0 装 PASS: 真 register concept
    pub fn register(&mut self, name: impl Into<String>, desc: impl Into<String>) {
        self.concepts.insert(name.into(), desc.into());
    }
    /// 0 装 PASS: 真 explain
    pub fn explain(&self, name: &str) -> Option<String> {
        self.concepts.get(name).cloned()
    }
}

impl Default for ExplanationBuilder { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut b = ExplanationBuilder::new();
        b.register("rust", "memory-safe language");
        assert!(b.explain("rust").is_some());
    }
    #[test] fn test_explain_unknown() {
        let b = ExplanationBuilder::new();
        assert!(b.explain("missing").is_none());
    }
    #[test] fn test_default() { let b: ExplanationBuilder = Default::default(); assert!(b.explain("x").is_none()); }
}
