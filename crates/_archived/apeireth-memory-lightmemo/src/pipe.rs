//! Search pipe + fusion strategy.

use crate::search::{SearchHit, SearchMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionStrategy {
    /// Weighted average (per-mode weights)
    Weighted,
    /// Reciprocal rank fusion
    Rrf,
    /// Max score
    Max,
}

pub struct SearchPipe {
    pub fusion: FusionStrategy,
}

impl SearchPipe {
    pub fn new() -> Self { Self { fusion: FusionStrategy::Rrf } }
    pub fn with_fusion(mut self, f: FusionStrategy) -> Self { self.fusion = f; self }

    /// Fuse multiple search results into one ranked list.
    pub fn fuse(&self, results: Vec<Vec<SearchHit>>) -> Vec<SearchHit> {
        match self.fusion {
            FusionStrategy::Rrf => self.fuse_rrf(results),
            FusionStrategy::Max => self.fuse_max(results),
            FusionStrategy::Weighted => self.fuse_weighted(results, &[0.4, 0.4, 0.2]),
        }
    }

    fn fuse_rrf(&self, results: Vec<Vec<SearchHit>>) -> Vec<SearchHit> {
        let mut scores: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut modes: std::collections::HashMap<String, Vec<SearchMode>> = std::collections::HashMap::new();
        for hits in results {
            for (rank, hit) in hits.iter().enumerate() {
                let rrf_score = 1.0 / (rank as f32 + 60.0);
                *scores.entry(hit.id.clone()).or_insert(0.0) += rrf_score;
                modes.entry(hit.id.clone()).or_default().extend(hit.matched_in.clone());
            }
        }
        let mut out: Vec<SearchHit> = scores.into_iter()
            .map(|(id, score)| SearchHit { id: id.clone(), score, matched_in: modes.get(&id).cloned().unwrap_or_default() })
            .collect();
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    fn fuse_max(&self, results: Vec<Vec<SearchHit>>) -> Vec<SearchHit> {
        let mut scores: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut modes: std::collections::HashMap<String, Vec<SearchMode>> = std::collections::HashMap::new();
        for hits in results {
            for hit in hits {
                let entry = scores.entry(hit.id.clone()).or_insert(0.0);
                if hit.score > *entry { *entry = hit.score; }
                modes.entry(hit.id.clone()).or_default().extend(hit.matched_in.clone());
            }
        }
        let mut out: Vec<SearchHit> = scores.into_iter()
            .map(|(id, score)| {
                let modes = modes.remove(&id).unwrap_or_default();
                SearchHit { id, score, matched_in: modes }
            })
            .collect();
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    fn fuse_weighted(&self, results: Vec<Vec<SearchHit>>, weights: &[f32]) -> Vec<SearchHit> {
        let mut scores: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut modes: std::collections::HashMap<String, Vec<SearchMode>> = std::collections::HashMap::new();
        for (i, hits) in results.iter().enumerate() {
            let w = weights.get(i).copied().unwrap_or(1.0);
            for hit in hits {
                *scores.entry(hit.id.clone()).or_insert(0.0) += hit.score * w;
                modes.entry(hit.id.clone()).or_default().extend(hit.matched_in.clone());
            }
        }
        let mut out: Vec<SearchHit> = scores.into_iter()
            .map(|(id, score)| {
                let modes = modes.remove(&id).unwrap_or_default();
                SearchHit { id, score, matched_in: modes }
            })
            .collect();
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out
    }
}

impl Default for SearchPipe {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str, score: f32) -> SearchHit {
        SearchHit { id: id.into(), score, matched_in: vec![SearchMode::Keyword] }
    }

    #[test]
    fn rrf_fusion() {
        let p = SearchPipe::new();
        let r = p.fuse(vec![
            vec![hit("a", 1.0), hit("b", 0.5)],
            vec![hit("a", 0.8), hit("c", 0.6)],
        ]);
        assert_eq!(r.len(), 3);
        // "a" appears in both → highest RRF score
        assert_eq!(r[0].id, "a");
    }

    #[test]
    fn max_fusion() {
        let p = SearchPipe::with_fusion(SearchPipe::new(), FusionStrategy::Max);
        let r = p.fuse(vec![
            vec![hit("a", 0.5)],
            vec![hit("a", 0.9)],
        ]);
        assert_eq!(r.len(), 1);
        assert!((r[0].score - 0.9).abs() < 1e-6);
    }

    #[test]
    fn weighted_fusion() {
        let p = SearchPipe::with_fusion(SearchPipe::new(), FusionStrategy::Weighted);
        let r = p.fuse(vec![
            vec![hit("a", 1.0)],
            vec![hit("a", 1.0)],
        ]);
        // Weights default 0.4 + 0.4 = 0.8
        assert!(r[0].score > 0.5);
    }

    #[test]
    fn empty_fusion() {
        let p = SearchPipe::new();
        let r = p.fuse(vec![]);
        assert!(r.is_empty());
    }
}