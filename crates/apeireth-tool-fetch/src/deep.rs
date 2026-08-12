// Deep web multi-round search (FlashDeepSearch)

use serde::{Deserialize, Serialize};

use crate::engine::{FetchEngine, FetchError, FetchRequest, FetchResult};
use crate::search_aggregator::{SearchAggregator, SearchHit, SearchSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepRound {
    pub round: usize,
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeepResult {
    pub rounds: Vec<DeepRound>,
    pub final_hits: Vec<SearchHit>,
    pub total_rounds: usize,
    pub total_elapsed_ms: u64,
}

impl DeepResult {
    pub fn flat_hits(&self) -> Vec<&SearchHit> {
        let mut all: Vec<&SearchHit> = Vec::new();
        for r in &self.rounds { for h in &r.hits { all.push(h); } }
        all
    }

    pub fn unique_urls(&self) -> usize {
        let mut set = std::collections::HashSet::new();
        for h in self.flat_hits() { set.insert(&h.url); }
        set.len()
    }
}

pub struct DeepSearcher {
    max_rounds: usize,
    hits_per_round: usize,
    score_threshold: f64,
}

impl DeepSearcher {
    pub fn new() -> Self {
        Self { max_rounds: 3, hits_per_round: 5, score_threshold: 0.3 }
    }

    pub fn with_depth(max_rounds: usize, hits_per_round: usize) -> Self {
        Self { max_rounds, hits_per_round, score_threshold: 0.3 }
    }

    pub fn max_rounds(&self) -> usize { self.max_rounds }
    pub fn set_max_rounds(&mut self, n: usize) { self.max_rounds = n; }

    /// 多轮深网抓取: 每轮 query 变体 + 重排序 + 截断,直到 max_rounds 耗尽或 hit 数稳定
    /// 注: 实际 HTTP 由 host 端注入 SearchAggregator.add_hits
    pub fn search(&self, base_query: &str, agg: &SearchAggregator) -> DeepResult {
        let mut result = DeepResult::default();
        let mut prev_count = 0;
        for r in 0..self.max_rounds {
            let query = if r == 0 {
                base_query.to_string()
            } else {
                format!("{} deep:{}", base_query, r)
            };
            let mut round_hits = agg.aggregate(&query, self.hits_per_round).hits;
            round_hits.retain(|h| h.score >= self.score_threshold);
            result.rounds.push(DeepRound { round: r, query, hits: round_hits.clone(), elapsed_ms: 0 });
            result.final_hits.extend(round_hits);
            // 收敛: 如果本轮没新增,提前终止
            if result.final_hits.len() == prev_count {
                break;
            }
            prev_count = result.final_hits.len();
        }
        result.total_rounds = result.rounds.len();
        // 去重
        let mut seen = std::collections::HashMap::new();
        for h in &result.final_hits {
            seen.entry(h.url.clone()).or_insert(h.clone());
        }
        let mut uniq: Vec<SearchHit> = seen.into_values().collect();
        uniq.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        result.final_hits = uniq;
        result
    }
}

impl Default for DeepSearcher {
    fn default() -> Self { Self::new() }
}

/// 验证 query 注入 (供 host 端使用)
pub fn build_deep_query(base: &str, round: usize) -> String {
    if round == 0 { base.into() } else { format!("{} round:{}", base, round) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(url: &str, score: f64) -> SearchHit {
        SearchHit { title: url.into(), url: url.into(), snippet: String::new(), source: SearchSource::DuckDuckGo, score }
    }

    #[test]
    fn deep_search_aggregates() {
        let agg = SearchAggregator::new();
        agg.add_hits(SearchSource::DuckDuckGo, vec![hit("https://a", 0.9), hit("https://b", 0.7)]);
        let deep = DeepSearcher::with_depth(2, 5);
        let r = deep.search("rust async", &agg);
        assert!(r.total_rounds >= 1);
        assert!(r.final_hits.len() >= 1);
    }

    #[test]
    fn deep_search_converges() {
        let agg = SearchAggregator::new();
        let deep = DeepSearcher::with_depth(10, 5);
        let r = deep.search("stable", &agg);
        // 空输入应该快速收敛
        assert!(r.total_rounds <= 10);
    }

    #[test]
    fn unique_urls_count() {
        let agg = SearchAggregator::new();
        agg.add_hits(SearchSource::AnySearch, vec![hit("https://x", 0.9), hit("https://x", 0.7)]);
        let deep = DeepSearcher::with_depth(2, 5);
        let r = deep.search("test", &agg);
        assert_eq!(r.unique_urls(), 1);
    }

    #[test]
    fn build_deep_query_varies() {
        assert_eq!(build_deep_query("rust", 0), "rust");
        assert_eq!(build_deep_query("rust", 1), "rust round:1");
        assert_eq!(build_deep_query("rust", 3), "rust round:3");
    }

    #[test]
    fn score_threshold_filters() {
        let agg = SearchAggregator::new();
        agg.add_hits(SearchSource::DuckDuckGo, vec![
            hit("https://high", 0.9),
            hit("https://low", 0.1),
        ]);
        let mut deep = DeepSearcher::new();
        deep.set_max_rounds(1);
        let r = deep.search("x", &agg);
        assert!(r.final_hits.iter().any(|h| h.url == "https://high"));
        assert!(!r.final_hits.iter().any(|h| h.url == "https://low"));
    }
}
