//! Memory Semantic - 语义检索 (抄 v1 apeireth-memory/semantic.rs)
use std::collections::{HashMap, HashSet};
pub struct SemanticIndex { pub docs: HashMap<String, String>, pub inverted: HashMap<String, HashSet<String>>, pub df: HashMap<String, usize> }
impl SemanticIndex {
    pub fn new() -> Self { Self { docs: HashMap::new(), inverted: HashMap::new(), df: HashMap::new() } }
    pub fn add(&mut self, id: impl Into<String>, content: impl Into<String>) {
        let id = id.into();
        let content = content.into();
        let tokens: HashSet<String> = content.split_whitespace().map(|s| s.to_lowercase()).collect();
        for t in &tokens { self.inverted.entry(t.clone()).or_default().insert(id.clone()); *self.df.entry(t.clone()).or_insert(0) += 1; }
        self.docs.insert(id, content);
    }
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(String, f32)> {
        let q_tokens: HashSet<String> = query.split_whitespace().map(|s| s.to_lowercase()).collect();
        let n = self.docs.len() as f32;
        if n == 0.0 { return vec![]; }
        let mut scores: HashMap<String, f32> = HashMap::new();
        for t in &q_tokens {
            if let Some(docs) = self.inverted.get(t) {
                let df = *self.df.get(t).unwrap_or(&1) as f32;
                let idf = (n / df.max(1.0)).ln() + 1.0;
                for d in docs { *scores.entry(d.clone()).or_insert(0.0) += idf; }
            }
        }
        let mut r: Vec<_> = scores.into_iter().collect();
        r.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        r.truncate(top_k);
        r
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_basic() { let mut s = SemanticIndex::new(); s.add("d1", "rust programming language"); s.add("d2", "python snake"); assert!(s.search("rust", 5).iter().any(|(id, _)| id == "d1")); } }