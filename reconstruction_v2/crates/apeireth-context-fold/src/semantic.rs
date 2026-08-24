//! Semantic fold (deterministic embedding-free scorer).

use serde::{Deserialize, Serialize};

/// Embedder trait (default: deterministic hash-based for testing).
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f64>;
}

/// Cosine similarity.
pub fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { 0.0 }
    else { dot / (mag_a * mag_b) }
}

/// Relevance scorer trait.
pub trait RelevanceScorer: Send + Sync {
    fn score(&self, query: &[f64], candidate: &[f64]) -> f64;
}

/// Default bigram-overlap scorer (deterministic, no embedding).
pub struct BigramOverlapScorer;

impl RelevanceScorer for BigramOverlapScorer {
    fn score(&self, query: &[f64], candidate: &[f64]) -> f64 {
        // simple overlap measure
        let intersection = query.iter().filter(|x| candidate.contains(x)).count();
        intersection as f64 / (query.len().max(candidate.len()) as f64).max(1.0)
    }
}

/// Embedding scorer (requires an Embedder).
pub struct EmbeddingScorer<E: Embedder> {
    pub embedder: E,
}

impl<E: Embedder> EmbeddingScorer<E> {
    pub fn new(embedder: E) -> Self { Self { embedder } }
}

impl<E: Embedder + Send + Sync> RelevanceScorer for EmbeddingScorer<E> {
    fn score(&self, query: &[f64], candidate: &[f64]) -> f64 {
        cosine(query, candidate)
    }
}

/// A folded segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldedSegment {
    pub text: String,
    pub score: f64,
    pub folded: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticFoldOptions {
    pub threshold: f64,
    pub max_segments: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticFoldOutcome {
    pub segments: Vec<FoldedSegment>,
    pub folded_count: usize,
}

/// Fold segments by relevance (lower score = fold).
pub fn fold_segments<S: RelevanceScorer>(
    query: &[f64],
    segments: Vec<String>,
    scorer: &S,
    opts: &SemanticFoldOptions,
) -> SemanticFoldOutcome {
    let mut folded_count = 0;
    let segments: Vec<FoldedSegment> = segments.into_iter().take(opts.max_segments.max(1)).map(|s| {
        // Embed via BigramOverlapScorer-like: convert text to vec<f64> via char codes
        let cand: Vec<f64> = s.chars().take(64).map(|c| c as u32 as f64).collect();
        let score = scorer.score(query, &cand);
        let folded = score < opts.threshold;
        if folded { folded_count += 1; }
        FoldedSegment { text: s, score, folded }
    }).collect();
    SemanticFoldOutcome { segments, folded_count }
}

/// Unfold semantic fold (restore original segments).
pub fn unfold_semantic(outcome: SemanticFoldOutcome) -> Vec<String> {
    outcome.segments.into_iter().map(|s| s.text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let s = cosine(&v, &v);
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_zero_mag() {
        let z = vec![0.0, 0.0];
        let v = vec![1.0, 2.0];
        assert_eq!(cosine(&z, &v), 0.0);
    }

    #[test]
    fn cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine(&a, &b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn fold_segments_basic() {
        let query = vec![65.0, 66.0, 67.0]; // "ABC"
        let segs = vec!["ABC".to_string(), "XYZ".to_string()];
        let opts = SemanticFoldOptions { threshold: 0.5, max_segments: 10 };
        let out = fold_segments(&query, segs, &BigramOverlapScorer, &opts);
        assert_eq!(out.segments.len(), 2);
        assert_eq!(out.folded_count, 1); // XYZ folded
    }

    #[test]
    fn unfold_restores() {
        let query = vec![65.0, 66.0];
        let segs = vec!["a".to_string(), "b".to_string()];
        let opts = SemanticFoldOptions { max_segments: 10, threshold: 0.0 };
        let out = fold_segments(&query, segs, &BigramOverlapScorer, &opts);
        let restored = unfold_semantic(out);
        assert_eq!(restored, vec!["a", "b"]);
    }

    #[test]
    fn bigram_overlap_scorer_score() {
        let s = BigramOverlapScorer;
        let q = vec![1.0, 2.0, 3.0];
        let c = vec![1.0, 4.0, 5.0];
        let sc = s.score(&q, &c);
        assert!(sc > 0.0);
        assert!(sc < 1.0);
    }
}
