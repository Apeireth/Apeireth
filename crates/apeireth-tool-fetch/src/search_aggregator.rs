// Search aggregator (multi-source: Tavily/AnySearch/DuckDuckGo placeholder)

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchSource {
    Tavily,
    AnySearch,
    DuckDuckGo,
    SearXng,
}

impl SearchSource {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::AnySearch => "anysearch",
            Self::DuckDuckGo => "duckduckgo",
            Self::SearXng => "searxng",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: SearchSource,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregatedResults {
    pub hits: Vec<SearchHit>,
    pub total_sources: usize,
    pub query: String,
    pub elapsed_ms: u64,
}

impl AggregatedResults {
    pub fn dedup_by_url(&mut self) {
        let mut seen = HashMap::new();
        let mut out = Vec::new();
        for h in self.hits.drain(..) {
            let entry = seen.entry(h.url.clone()).or_insert((h.score, h.clone()));
            if h.score > entry.0 {
                *entry = (h.score, h.clone());
            }
        }
        for (_, (_, h)) in seen {
            out.push(h);
        }
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        self.hits = out;
    }
}

pub struct SearchAggregator {
    sources: RwLock<Vec<SearchSource>>,
    hits: RwLock<Vec<SearchHit>>,
}

impl SearchAggregator {
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(vec![SearchSource::DuckDuckGo, SearchSource::AnySearch]),
            hits: RwLock::new(Vec::new()),
        }
    }

    pub fn with_sources(sources: Vec<SearchSource>) -> Self {
        Self {
            sources: RwLock::new(sources),
            hits: RwLock::new(Vec::new()),
        }
    }

    pub fn sources(&self) -> Vec<SearchSource> {
        self.sources.read().clone()
    }

    /// 真实现的多源合并: 每个 source 加自己的 hits,然后按 URL 去重 + 排序
    /// 注: 实际 HTTP 调用由 host 端注入,本 crate 只做数据层
    pub fn add_hits(&self, source: SearchSource, mut hits: Vec<SearchHit>) {
        for h in &mut hits {
            h.source = source;
        }
        self.hits.write().extend(hits);
    }

    pub fn aggregate(&self, query: impl Into<String>, limit: usize) -> AggregatedResults {
        let query = query.into();
        let mut agg = AggregatedResults {
            hits: self.hits.read().clone(),
            total_sources: self.sources.read().len(),
            query: query.clone(),
            elapsed_ms: 0,
        };
        agg.dedup_by_url();
        agg.hits.truncate(limit);
        agg
    }

    pub fn clear(&self) {
        self.hits.write().clear();
    }
}

impl Default for SearchAggregator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(url: &str, score: f64) -> SearchHit {
        SearchHit { title: url.into(), url: url.into(), snippet: String::new(), source: SearchSource::DuckDuckGo, score }
    }

    #[test]
    fn dedup_keeps_highest_score() {
        let mut a = AggregatedResults::default();
        a.hits.push(hit("https://x.com", 0.5));
        a.hits.push(hit("https://x.com", 0.9));
        a.hits.push(hit("https://y.com", 0.3));
        a.dedup_by_url();
        assert_eq!(a.hits.len(), 2);
        let x = a.hits.iter().find(|h| h.url == "https://x.com").unwrap();
        assert_eq!(x.score, 0.9);
    }

    #[test]
    fn aggregate_truncates() {
        let agg = SearchAggregator::new();
        agg.add_hits(SearchSource::DuckDuckGo, vec![hit("https://a", 0.9), hit("https://b", 0.7), hit("https://c", 0.5)]);
        let r = agg.aggregate("test", 2);
        assert_eq!(r.hits.len(), 2);
        assert_eq!(r.hits[0].url, "https://a");
    }

    #[test]
    fn sources_default() {
        let agg = SearchAggregator::new();
        assert!(agg.sources().contains(&SearchSource::DuckDuckGo));
    }

    #[test]
    fn source_name() {
        assert_eq!(SearchSource::Tavily.name(), "tavily");
        assert_eq!(SearchSource::DuckDuckGo.name(), "duckduckgo");
    }

    #[test]
    fn clear_empties_hits() {
        let agg = SearchAggregator::new();
        agg.add_hits(SearchSource::AnySearch, vec![hit("https://x", 0.5)]);
        agg.clear();
        let r = agg.aggregate("x", 10);
        assert_eq!(r.hits.len(), 0);
    }
}
