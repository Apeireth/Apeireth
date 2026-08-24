//! ThoughtCluster - 思维簇 (从 v1.0 apeireth-companion/thought_cluster.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 cluster 思维 by tag
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtCluster {
    pub id: String,
    pub thoughts: Vec<String>,  // thought IDs
    pub topic: String,
}

#[derive(Default)]
pub struct ThoughtClusterManager {
    clusters: HashMap<String, ThoughtCluster>,
}

impl ThoughtClusterManager {
    pub fn new() -> Self { Self { clusters: HashMap::new() } }

    /// 0 装 PASS: 真 add thought to cluster
    pub fn add(&mut self, topic: impl Into<String>, thought_id: impl Into<String>) {
        let t = topic.into();
        let id = thought_id.into();
        self.clusters.entry(t.clone()).or_insert_with(|| ThoughtCluster { id: format!("tc-{}", t), thoughts: Vec::new(), topic: t }).thoughts.push(id);
    }

    pub fn by_topic(&self, topic: &str) -> Option<&ThoughtCluster> {
        self.clusters.get(topic)
    }

    pub fn all(&self) -> Vec<&ThoughtCluster> {
        self.clusters.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut m = ThoughtClusterManager::new();
        m.add("math", "t1");
        m.add("math", "t2");
        let c = m.by_topic("math").unwrap();
        assert_eq!(c.thoughts.len(), 2);
    }
    #[test] fn test_multiple_topics() {
        let mut m = ThoughtClusterManager::new();
        m.add("math", "t1");
        m.add("art", "t2");
        assert_eq!(m.all().len(), 2);
    }
    #[test] fn test_unknown() {
        let m = ThoughtClusterManager::new();
        assert!(m.by_topic("missing").is_none());
    }
    #[test] fn test_default() {
        let m: ThoughtClusterManager = Default::default();
        assert!(m.all().is_empty());
    }
}
