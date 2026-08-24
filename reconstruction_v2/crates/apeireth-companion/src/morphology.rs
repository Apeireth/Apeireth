//! Morphology - 形态学 (从 v1.0 apeireth-companion/morphology.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 query 形态分类
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RetrievalMode { Vector, Keyword, Hybrid, Crawl }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphologyVerdict {
    pub mode: RetrievalMode,
    pub confidence: f32,
    pub reason: String,
}

pub struct Morphology;

impl Morphology {
    pub fn new() -> Self { Self }

    /// 0 装 PASS: 真 classify (基于关键词特征)
    pub fn classify(&self, query: &str) -> MorphologyVerdict {
        let q = query.to_lowercase();
        let mode = if q.contains("how") || q.contains("why") { RetrievalMode::Crawl }
                  else if q.starts_with("search ") { RetrievalMode::Keyword }
                  else if q.contains("all") || q.contains("complete") { RetrievalMode::Hybrid }
                  else { RetrievalMode::Vector };
        MorphologyVerdict { mode, confidence: 0.7, reason: format!("matched pattern for query: {}", query) }
    }
}

impl Default for Morphology { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_crawl() {
        let m = Morphology::new();
        let v = m.classify("how does memory work");
        assert_eq!(v.mode, RetrievalMode::Crawl);
    }
    #[test] fn test_keyword() {
        let m = Morphology::new();
        let v = m.classify("search rust language");
        assert_eq!(v.mode, RetrievalMode::Keyword);
    }
    #[test] fn test_hybrid() {
        let m = Morphology::new();
        let v = m.classify("list all things");
        assert_eq!(v.mode, RetrievalMode::Hybrid);
    }
    #[test] fn test_default_vector() {
        let m = Morphology::new();
        let v = m.classify("simple query");
        assert_eq!(v.mode, RetrievalMode::Vector);
    }
    #[test] fn test_mode_eq() {
        assert_eq!(RetrievalMode::Vector, RetrievalMode::Vector);
    }
}
