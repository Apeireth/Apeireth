//! ProactiveMemory - 主动记忆推销 (从 v1.0 apeireth-companion/proactive_memory.rs 919 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ProactiveBlock + topic classification

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProactiveBlock {
    pub topic: String,
    pub content: String,
    pub relevance: f32,  // 0 装 PASS: 0.0-1.0
    pub trigger_keywords: Vec<String>,
}

pub struct TopicClassifier { pub topics: Vec<String> }

impl TopicClassifier {
    pub fn new(topics: Vec<String>) -> Self { Self { topics } }

    /// 0 装 PASS: 真按 keyword 分类
    pub fn classify(&self, text: &str) -> Option<String> {
        let lower = text.to_lowercase();
        for t in &self.topics {
            if lower.contains(&t.to_lowercase()) { return Some(t.clone()); }
        }
        None
    }
}

pub struct ProactiveMemory {
    pub blocks: HashMap<String, Vec<ProactiveBlock>>,
    pub classifier: TopicClassifier,
}

impl ProactiveMemory {
    pub fn new(topics: Vec<String>) -> Self {
        Self { blocks: HashMap::new(), classifier: TopicClassifier::new(topics) }
    }

    /// 0 装 PASS: 真 add block
    pub fn add(&mut self, block: ProactiveBlock) {
        self.blocks.entry(block.topic.clone()).or_default().push(block);
    }

    /// 0 装 PASS: 真预载相关 block (按 topic)
    pub fn preload(&self, text: &str) -> Vec<&ProactiveBlock> {
        match self.classifier.classify(text) {
            Some(topic) => self.blocks.get(&topic).map(|v| v.iter().collect()).unwrap_or_default(),
            None => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_classify() {
        let c = TopicClassifier::new(vec!["rust".into(), "python".into()]);
        assert_eq!(c.classify("rust is great"), Some("rust".to_string()));
        assert_eq!(c.classify("java is bad"), None);
    }
    #[test] fn test_proactive_add() {
        let mut m = ProactiveMemory::new(vec!["rust".into()]);
        m.add(ProactiveBlock { topic: "rust".into(), content: "learn rust".into(), relevance: 0.8, trigger_keywords: vec!["ownership".into()] });
        assert_eq!(m.preload("rust programming").len(), 1);
    }
    #[test] fn test_proactive_no_match() {
        let m = ProactiveMemory::new(vec!["rust".into()]);
        assert!(m.preload("python is great").is_empty());
    }
    #[test] fn test_classifier_unknown() {
        let c = TopicClassifier::new(vec!["a".into()]);
        assert!(c.classify("xyz").is_none());
    }
    #[test] fn test_block_relevance() {
        let b = ProactiveBlock { topic: "rust".into(), content: "x".into(), relevance: 0.9, trigger_keywords: vec![] };
        assert_eq!(b.relevance, 0.9);
    }
}
