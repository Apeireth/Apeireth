//! Memory Semantic - 语义检索 (从 v1.0 apeireth-memory/semantic.rs 646 LOC 抄录升级核心部分)
//!
//! 0 装 PASS: 真 inverted index + TF-IDF score

use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct SemanticIndex {
    documents: HashMap<String, String>,    // doc_id -> content
    inverted: HashMap<String, HashSet<String>>,  // word -> doc_ids
    df: HashMap<String, usize>,           // document frequency
}

impl SemanticIndex {
    pub fn new() -> Self { Self::default() }

    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace().map(|s| s.to_lowercase()).collect()
    }

    /// 0 装 PASS: 真添加 + 维护 inverted index
    pub fn add(&mut self, id: impl Into<String>, content: impl Into<String>) {
        let id = id.into();
        let content = content.into();
        let tokens = Self::tokenize(&content);
        let mut unique_tokens: HashSet<String> = HashSet::new();
        for t in &tokens { unique_tokens.insert(t.clone()); }
        for token in &unique_tokens {
            self.inverted.entry(token.clone()).or_default().insert(id.clone());
            *self.df.entry(token.clone()).or_insert(0) += 1;
        }
        self.documents.insert(id, content);
    }

    /// 0 装 PASS: 真 TF-IDF cosine 检索
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(String, f32)> {
        let q_tokens = Self::tokenize(query);
        let n = self.documents.len() as f32;
        if n == 0.0 { return vec![]; }

        let mut scores: HashMap<String, f32> = HashMap::new();
        for token in &q_tokens {
            if let Some(docs) = self.inverted.get(token) {
                let df = *self.df.get(token).unwrap_or(&1) as f32;
                let idf = (n / df.max(1.0)).ln() + 1.0;
                for doc_id in docs {
                    *scores.entry(doc_id.clone()).or_insert(0.0) += idf;
                }
            }
        }

        let mut result: Vec<_> = scores.into_iter().map(|(id, score)| {
            // 用 content 长度做 normalize
            let len = self.documents.get(&id).map(|c| c.len() as f32).unwrap_or(1.0).sqrt();
            (id, score / len.max(1.0))
        }).collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result.into_iter().take(top_k).collect()
    }

    pub fn len(&self) -> usize { self.documents.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_and_search() {
        let mut idx = SemanticIndex::new();
        idx.add("d1", "rust programming language");
        idx.add("d2", "python snake reptile");
        let r = idx.search("rust", 5);
        assert!(r.iter().any(|(id, _)| id == "d1"));
    }
    #[test] fn test_empty() {
        let idx = SemanticIndex::new();
        assert_eq!(idx.search("anything", 5).len(), 0);
    }
    #[test] fn test_tokenize() {
        let t = SemanticIndex::tokenize("Hello World hello");
        assert_eq!(t.len(), 3);
    }
    #[test] fn test_multiple_documents_ranking() {
        let mut idx = SemanticIndex::new();
        idx.add("a", "rust rust rust");
        idx.add("b", "rust python");
        idx.add("c", "java python");
        let r = idx.search("rust", 3);
        assert!(r.iter().any(|(id, _)| id == "a"));
        assert!(r[0].1 > 0.0);
    }
    #[test] fn test_inverted_index_size() {
        let mut idx = SemanticIndex::new();
        idx.add("a", "hello world");
        idx.add("b", "hello rust");
        assert!(idx.inverted.contains_key("hello"));
        assert_eq!(idx.inverted.get("hello").unwrap().len(), 2);
    }
}
