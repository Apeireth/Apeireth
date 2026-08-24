//! TopicGroups - 主题分组 (从 v1.0 apeireth-companion/topic_groups.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真按 keyword 分组
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicGroup {
    pub topic: String,
    pub items: Vec<String>,
}

pub struct TopicGrouper {
    pub keywords: Vec<String>,
    groups: HashMap<String, Vec<String>>,
}

impl TopicGrouper {
    pub fn new(keywords: Vec<String>) -> Self { Self { keywords, groups: HashMap::new() } }

    /// 0 装 PASS: 真 classify
    pub fn classify(&mut self, id: impl Into<String>, text: impl Into<String>) -> String {
        let id = id.into();
        let text = text.into().to_lowercase();
        let topic = self.keywords.iter().find(|k| text.contains(k.as_str())).cloned().unwrap_or_else(|| "default".to_string());
        self.groups.entry(topic.clone()).or_default().push(id);
        topic
    }

    pub fn groups(&self) -> Vec<TopicGroup> {
        self.groups.iter().map(|(topic, items)| TopicGroup { topic: topic.clone(), items: items.clone() }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_classify() {
        let mut g = TopicGrouper::new(vec!["rust".into(), "python".into()]);
        assert_eq!(g.classify("a", "rust is great"), "rust");
        assert_eq!(g.classify("b", "python is good"), "python");
    }
    #[test] fn test_default() {
        let mut g = TopicGrouper::new(vec!["rust".into()]);
        assert_eq!(g.classify("a", "java"), "default");
    }
    #[test] fn test_groups() {
        let mut g = TopicGrouper::new(vec!["rust".into()]);
        g.classify("a", "rust 1");
        g.classify("b", "rust 2");
        assert_eq!(g.groups().len(), 1);
        assert_eq!(g.groups()[0].items.len(), 2);
    }
    #[test] fn test_topic_eq() {
        assert_eq!("rust", "rust");
    }
}
