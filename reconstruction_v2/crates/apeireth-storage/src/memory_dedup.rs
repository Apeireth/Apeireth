//! Memory Dedup - 去重 (抄 v1 apeireth-memory/dedup.rs)
use std::collections::HashSet;
pub enum DedupStrategy { Exact, Jaccard, Cosine, MinHash }
pub struct DedupConfig { pub strategy: DedupStrategy, pub threshold: f32 }
impl Default for DedupConfig { fn default() -> Self { Self { strategy: DedupStrategy::Jaccard, threshold: 0.85 } } }
pub struct DedupIndex { pub seen: Vec<String> }
impl DedupIndex {
    pub fn new() -> Self { Self { seen: Vec::new() } }
    pub fn is_duplicate(&mut self, text: &str, cfg: &DedupConfig) -> bool {
        let lower = text.to_lowercase();
        let tokens: HashSet<String> = lower.split_whitespace().map(String::from).collect();
        for seen in &self.seen {
            let seen_tokens: HashSet<String> = seen.to_lowercase().split_whitespace().map(String::from).collect();
            let inter: HashSet<_> = tokens.intersection(&seen_tokens).collect();
            let uni: HashSet<_> = tokens.union(&seen_tokens).collect();
            let sim = if uni.is_empty() { 0.0 } else { inter.len() as f32 / uni.len() as f32 };
            if sim >= cfg.threshold { return true; }
        }
        self.seen.push(text.to_string());
        false
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_no_dup() { let mut d = DedupIndex::new(); let cfg = DedupConfig::default(); assert!(!d.is_duplicate("hello world", &cfg)); } #[test] fn test_dup() { let mut d = DedupIndex::new(); let cfg = DedupConfig::default(); d.is_duplicate("hello world", &cfg); assert!(d.is_duplicate("hello world", &cfg)); } }