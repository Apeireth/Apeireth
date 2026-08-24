//! Memory Dedup - 去重 (从 v1.0 apeireth-memory/dedup.rs 410 LOC 抄录升级)
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DedupStrategy { Exact, Jaccard, Cosine, MinHash }

#[derive(Debug, Clone)]
pub struct DedupConfig { pub strategy: DedupStrategy, pub threshold: f32, pub shingle_size: usize }

impl Default for DedupConfig { fn default() -> Self { Self { strategy: DedupStrategy::Jaccard, threshold: 0.85, shingle_size: 3 } } }

#[derive(Default)]
pub struct DedupIndex { seen: Vec<String> }

impl DedupIndex {
    pub fn new() -> Self { Self::default() }
    fn tokenize(text: &str) -> Vec<String> { text.split_whitespace().map(|s| s.to_lowercase()).collect() }
    fn shingle(tokens: &[String], k: usize) -> HashSet<String> {
        if tokens.len() < k { return tokens.iter().cloned().collect(); }
        (0..=tokens.len() - k).map(|i| tokens[i..i+k].join(" ")).collect()
    }
    pub fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
        if a.is_empty() && b.is_empty() { return 1.0; }
        let inter = a.intersection(b).count() as f32;
        let union = a.union(b).count() as f32;
        if union == 0.0 { 0.0 } else { inter / union }
    }
    pub fn cosine(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
        let inter = a.intersection(b).count() as f32;
        let na = (a.len() as f32).sqrt();
        let nb = (b.len() as f32).sqrt();
        if na == 0.0 || nb == 0.0 { return 0.0; }
        inter / (na * nb)
    }
    pub fn minhash(a: &HashSet<String>, b: &HashSet<String>, num_hashes: usize) -> f32 {
        let mut matches = 0;
        for i in 0..num_hashes {
            let hash = |s: &String| -> u64 { use std::hash::{BuildHasher, Hasher, RandomState}; let mut h = RandomState::new().build_hasher(); h.write_u64(i as u64); h.write(s.as_bytes()); h.finish() };
            let ha = a.iter().map(hash).min();
            let hb = b.iter().map(hash).min();
            if ha == hb { matches += 1; }
        }
        matches as f32 / num_hashes as f32
    }
    pub fn is_duplicate(&mut self, text: &str, config: &DedupConfig) -> bool {
        let tokens = Self::tokenize(text);
        let shingles = Self::shingle(&tokens, config.shingle_size);
        for seen_text in &self.seen {
            let seen_tokens = Self::tokenize(seen_text);
            let seen_shingles = Self::shingle(&seen_tokens, config.shingle_size);
            let sim = match config.strategy {
                DedupStrategy::Exact => if seen_text == text { 1.0 } else { 0.0 },
                DedupStrategy::Jaccard => Self::jaccard(&shingles, &seen_shingles),
                DedupStrategy::Cosine => Self::cosine(&shingles, &seen_shingles),
                DedupStrategy::MinHash => Self::minhash(&shingles, &seen_shingles, 64),
            };
            if sim >= config.threshold { return true; }
        }
        self.seen.push(text.to_string());
        false
    }
    pub fn clear(&mut self) { self.seen.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_jaccard_identical() {
        let a: HashSet<_> = ["hello", "world"].iter().map(|s| s.to_string()).collect();
        let b = a.clone();
        assert!((DedupIndex::jaccard(&a, &b) - 1.0).abs() < 1e-6);
    }
    #[test] fn test_jaccard_disjoint() {
        let a: HashSet<_> = ["hello"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<_> = ["world"].iter().map(|s| s.to_string()).collect();
        assert_eq!(DedupIndex::jaccard(&a, &b), 0.0);
    }
    #[test] fn test_cosine() {
        let a: HashSet<_> = ["hello", "world"].iter().map(|s| s.to_string()).collect();
        let b = a.clone();
        assert!((DedupIndex::cosine(&a, &b) - 1.0).abs() < 1e-6);
    }
    #[test] fn test_exact_duplicate() {
        let mut d = DedupIndex::new();
        let cfg = DedupConfig { strategy: DedupStrategy::Exact, threshold: 1.0, shingle_size: 3 };
        assert!(!d.is_duplicate("hello world", &cfg));
        assert!(d.is_duplicate("hello world", &cfg));
    }
    #[test] fn test_jaccard_threshold() {
        let mut d = DedupIndex::new();
        let cfg = DedupConfig { strategy: DedupStrategy::Jaccard, threshold: 0.5, shingle_size: 1 };
        assert!(!d.is_duplicate("hello world", &cfg));
        assert!(d.is_duplicate("hello world rust", &cfg));
    }
    #[test] fn test_shingle() {
        let tokens = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        let s = DedupIndex::shingle(&tokens, 2);
        assert!(s.contains("a b"));
        assert!(s.contains("b c"));
        assert!(!s.contains("a c"));
    }
}
