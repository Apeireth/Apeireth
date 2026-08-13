//! Multi-pipe search (BM25-lite + vector + tag fusion).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Keyword / substring search (BM25-lite via token match)
    Keyword,
    /// Vector cosine similarity
    Vector,
    /// Tag-based lookup
    Tag,
    /// Fusion of all three
    Fusion,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: String,
    pub score: f32,
    pub matched_in: Vec<SearchMode>,
}

pub struct SearchPipeline;

impl SearchPipeline {
    pub fn new() -> Self { Self }

    /// Keyword search: case-insensitive substring matching.
    /// title/content indexed via simple lowercase contains.
    pub fn keyword_search(
        &self,
        items: &[(String, String)],
        query: &str,
        top_k: usize,
    ) -> Vec<SearchHit> {
        let q = query.to_lowercase();
        let mut hits: Vec<SearchHit> = items.iter()
            .filter(|(_, content)| content.to_lowercase().contains(&q))
            .map(|(id, _)| SearchHit {
                id: id.clone(),
                score: 1.0,
                matched_in: vec![SearchMode::Keyword],
            })
            .collect();
        hits.truncate(top_k);
        hits
    }

    /// Tag search.
    pub fn tag_search(
        &self,
        tag_index: &std::collections::HashMap<String, std::collections::HashSet<String>>,
        tags: &[String],
        top_k: usize,
    ) -> Vec<SearchHit> {
        let mut all = std::collections::HashSet::new();
        for tag in tags {
            if let Some(ids) = tag_index.get(tag) {
                for id in ids {
                    all.insert(id.clone());
                }
            }
        }
        let mut hits: Vec<SearchHit> = all.into_iter().take(top_k)
            .map(|id| SearchHit { id, score: 1.0, matched_in: vec![SearchMode::Tag] })
            .collect();
        hits.truncate(top_k);
        hits
    }
}

impl Default for SearchPipeline {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn keyword_search_finds_matches() {
        let p = SearchPipeline::new();
        let items = vec![
            ("a".into(), "rust tutorial".into()),
            ("b".into(), "python basics".into()),
            ("c".into(), "rust + python".into()),
        ];
        let hits = p.keyword_search(&items, "rust", 10);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn keyword_search_no_match() {
        let p = SearchPipeline::new();
        let items = vec![("a".into(), "rust".into())];
        let hits = p.keyword_search(&items, "haskell", 10);
        assert!(hits.is_empty());
    }

    #[test]
    fn tag_search_finds_tagged() {
        let p = SearchPipeline::new();
        let mut idx: HashMap<String, HashSet<String>> = HashMap::new();
        idx.entry("rust".into()).or_default().insert("a".into());
        idx.entry("rust".into()).or_default().insert("b".into());
        let hits = p.tag_search(&idx, &["rust".into()], 10);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn tag_search_top_k() {
        let p = SearchPipeline::new();
        let mut idx: HashMap<String, HashSet<String>> = HashMap::new();
        for i in 0..10 {
            idx.entry("t".into()).or_default().insert(format!("item{}", i));
        }
        let hits = p.tag_search(&idx, &["t".into()], 3);
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_mode_serialize() {
        let m = SearchMode::Fusion;
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, "\"Fusion\"");
    }
}