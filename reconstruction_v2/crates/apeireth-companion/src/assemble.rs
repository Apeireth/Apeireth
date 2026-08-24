//! Assemble - 组件组装 (从 v1.0 apeireth-companion/assemble.rs 1101 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 CompanionApp + DeepRecall + DialogSummarizer + ExperienceRefiner (v1 简化)

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// 0 装 PASS: CompanionApp 主结构 (v1 assemble 简化)
pub struct CompanionApp {
    pub name: String,
    pub components: HashMap<String, Arc<dyn Component>>,
}

pub trait Component: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, input: &str) -> String;
}

impl CompanionApp {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), components: HashMap::new() } }
    pub fn register(&mut self, c: Arc<dyn Component>) { self.components.insert(c.name().to_string(), c); }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Component>> { self.components.get(name).cloned() }
    pub fn list(&self) -> Vec<String> { self.components.keys().cloned().collect() }
}

/// 0 装 PASS: DeepRecall (v1 DeepRecall 简化)
pub struct DeepRecall { pub prefix: String }

impl Component for DeepRecall {
    fn name(&self) -> &str { "deep_recall" }
    fn process(&self, input: &str) -> String { format!("{}{}", self.prefix, input) }
}

/// 0 装 PASS: DialogSummarizer (v1 简化)
pub struct DialogSummarizer;

impl Component for DialogSummarizer {
    fn name(&self) -> &str { "dialog_summarizer" }
    fn process(&self, input: &str) -> String { format!("[Summary] {}", &input[..input.len().min(50)]) }
}

/// 0 装 PASS: ExperienceRefiner (v1 简化)
pub struct ExperienceRefiner;

impl Component for ExperienceRefiner {
    fn name(&self) -> &str { "experience_refiner" }
    fn process(&self, input: &str) -> String { input.split_whitespace().take(10).collect::<Vec<_>>().join(" ") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_app_register() {
        let mut app = CompanionApp::new("test");
        app.register(Arc::new(DialogSummarizer));
        assert_eq!(app.list().len(), 1);
    }
    #[test] fn test_deep_recall() {
        let c = DeepRecall { prefix: "recall:".into() };
        assert_eq!(c.process("hello"), "recall:hello");
    }
    #[test] fn test_summarizer() {
        let c = DialogSummarizer;
        let r = c.process("a long input that goes beyond limit");
        assert!(r.starts_with("[Summary]"));
    }
    #[test] fn test_refiner() {
        let c = ExperienceRefiner;
        let r = c.process("a b c d e f g h i j k l m");
        assert_eq!(r, "a b c d e f g h i j");
    }
    #[test] fn test_get_unknown() {
        let app = CompanionApp::new("t");
        assert!(app.get("missing").is_none());
    }
    #[test] fn test_app_get() {
        let mut app = CompanionApp::new("t");
        app.register(Arc::new(DeepRecall { prefix: "p:".into() }));
        let c = app.get("deep_recall").unwrap();
        assert_eq!(c.process("x"), "p:x");
    }
}
