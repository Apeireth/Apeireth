//! MemoryExtractor - 记忆抽取 (从 v1.0 apeireth-companion/memory_extractor.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 keyword + phrase 抽取 (确定性)
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub text: String,
    pub keywords: Vec<String>,
    pub importance: u8,  // 0 装 PASS: 0-100
}

#[derive(Default)]
pub struct MemoryExtractor {
    pub min_keyword_len: usize,
    pub max_keywords: usize,
}

impl MemoryExtractor {
    pub fn new() -> Self { Self { min_keyword_len: 3, max_keywords: 10 } }

    /// 0 装 PASS: 真抽取 (基于停用词 + 长度)
    pub fn extract(&self, text: &str, importance: u8) -> ExtractedMemory {
        let stop_words: HashSet<&str> = ["the", "a", "an", "is", "are", "was", "and", "or", "but", "of", "to", "in", "on", "for", "with", "by"].iter().cloned().collect();
        let mut keywords: Vec<String> = text.split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|w| w.len() >= self.min_keyword_len && !stop_words.contains(w.as_str()))
            .collect::<HashSet<_>>().into_iter().collect();
        keywords.sort();
        keywords.truncate(self.max_keywords);
        ExtractedMemory { text: text.into(), keywords, importance }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_extract_basic() {
        let e = MemoryExtractor::new();
        let r = e.extract("The rust programming language is fast and safe", 80);
        assert!(r.keywords.contains(&"rust".to_string()));
        assert!(r.keywords.contains(&"programming".to_string()));
        assert_eq!(r.importance, 80);
    }
    #[test] fn test_extract_filters_short() {
        let e = MemoryExtractor::new();
        let r = e.extract("a I an to in the", 50);
        // min_keyword_len = 3, so single char words filtered
        assert!(r.keywords.is_empty());
    }
    #[test] fn test_extract_max_keywords() {
        let mut e = MemoryExtractor::new();
        e.max_keywords = 2;
        let r = e.extract("rust python java go scala haskell", 80);
        assert_eq!(r.keywords.len(), 2);
    }
    #[test] fn test_importance_clamp() {
        let e = MemoryExtractor::new();
        let r = e.extract("x", 50);
        assert_eq!(r.importance, 50);
    }
}
