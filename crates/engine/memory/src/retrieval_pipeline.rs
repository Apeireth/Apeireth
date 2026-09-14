//! Governed hybrid retrieval pipeline.
//!
//! The pipeline is intentionally provider-neutral: lexical and vector
//! candidate sources expose safe candidate metadata, while Assembly decides
//! whether an embedding adapter or model reranker is available.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    MemoryCandidate, MemoryError, MemoryRankingConfig, MemoryReranker, MemoryScope, ScoreComponents,
};

/// Candidate source shared by lexical, vector, working, episodic, semantic,
/// and relational implementations.
pub trait MemoryCandidateSource: Send + Sync {
    fn candidates(&self, query: &str, limit: usize) -> Result<Vec<MemoryCandidate>, MemoryError>;
}

/// Marker for a lexical/BM25 source.
pub trait LexicalCandidateSource: MemoryCandidateSource {}

/// Marker for a semantic/vector source.
pub trait VectorCandidateSource: MemoryCandidateSource {}

/// A small deterministic lexical source suitable for local token-overlap fallback and
/// unit tests. It uses Unicode-aware tokens rather than ASCII whitespace.
#[derive(Debug, Clone, Default)]
pub struct BasicLexicalCandidateSource {
    documents: Vec<MemoryCandidate>,
}

impl BasicLexicalCandidateSource {
    pub fn new(documents: Vec<MemoryCandidate>) -> Self {
        Self { documents }
    }

    pub fn push(&mut self, candidate: MemoryCandidate) {
        self.documents.push(candidate);
    }
}

impl MemoryCandidateSource for BasicLexicalCandidateSource {
    fn candidates(&self, query: &str, limit: usize) -> Result<Vec<MemoryCandidate>, MemoryError> {
        let query_tokens = unicode_tokens(query);
        let mut out = self.documents.clone();
        for candidate in &mut out {
            let tokens = unicode_tokens(&candidate.content);
            candidate.score_components.lexical = token_overlap_score(&query_tokens, &tokens);
            candidate.score = candidate.score_components.lexical;
        }
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        out.truncate(limit);
        Ok(out)
    }
}

impl LexicalCandidateSource for BasicLexicalCandidateSource {}

use crate::hybrid_search::{Bm25Config, Bm25Index};

/// Legacy token-overlap source alias for honesty in naming.
pub type TokenOverlapCandidateSource = BasicLexicalCandidateSource;

/// Production lexical candidate source backed by the canonical deterministic Okapi BM25 engine.
#[derive(Debug, Clone)]
pub struct Bm25LexicalCandidateSource {
    index: Bm25Index,
    candidates_by_id: HashMap<String, MemoryCandidate>,
    insertion_order: Vec<String>,
}

impl Bm25LexicalCandidateSource {
    pub fn new(candidates: Vec<MemoryCandidate>) -> Self {
        let mut index = Bm25Index::new(Bm25Config::default());
        let mut candidates_by_id = HashMap::new();
        let mut insertion_order = Vec::new();
        for candidate in candidates {
            index.insert(candidate.id.clone(), &candidate.content);
            insertion_order.push(candidate.id.clone());
            candidates_by_id.insert(candidate.id.clone(), candidate);
        }
        Self {
            index,
            candidates_by_id,
            insertion_order,
        }
    }

    pub fn push(&mut self, candidate: MemoryCandidate) {
        self.index.insert(candidate.id.clone(), &candidate.content);
        self.insertion_order.push(candidate.id.clone());
        self.candidates_by_id
            .insert(candidate.id.clone(), candidate);
    }
}

impl MemoryCandidateSource for Bm25LexicalCandidateSource {
    fn candidates(&self, query: &str, limit: usize) -> Result<Vec<MemoryCandidate>, MemoryError> {
        if self.candidates_by_id.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let hits = self.index.search(query, self.candidates_by_id.len());
        let max_score = hits.first().map(|h| h.score).unwrap_or(0.0);
        let mut hit_scores: HashMap<String, f64> = HashMap::new();
        for hit in hits {
            let normalized = if max_score > 0.0 {
                f64::from(hit.score / max_score)
            } else {
                0.0
            };
            hit_scores.insert(hit.id, normalized);
        }

        let mut out = Vec::new();
        for id in &self.insertion_order {
            if let Some(candidate) = self.candidates_by_id.get(id) {
                let mut c = candidate.clone();
                if let Some(&lexical) = hit_scores.get(id) {
                    c.score_components.lexical = lexical;
                } else {
                    c.score_components.lexical = 0.0;
                }
                c.score = c.score_components.lexical;
                out.push(c);
            }
        }

        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        out.truncate(limit);
        Ok(out)
    }
}

impl LexicalCandidateSource for Bm25LexicalCandidateSource {}

/// An already-embedded candidate source. Embedding creation remains outside
/// the Memory crate; this type only consumes validated candidate metadata.
#[derive(Debug, Clone, Default)]
pub struct StaticVectorCandidateSource {
    documents: Vec<MemoryCandidate>,
}

impl StaticVectorCandidateSource {
    pub fn new(documents: Vec<MemoryCandidate>) -> Self {
        Self { documents }
    }
}

impl MemoryCandidateSource for StaticVectorCandidateSource {
    fn candidates(&self, _query: &str, limit: usize) -> Result<Vec<MemoryCandidate>, MemoryError> {
        let mut out = self.documents.clone();
        out.sort_by(|a, b| {
            b.score_components
                .semantic
                .partial_cmp(&a.score_components.semantic)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        out.truncate(limit);
        Ok(out)
    }
}

impl VectorCandidateSource for StaticVectorCandidateSource {}

/// Output of the hybrid pipeline, including whether the vector stage was
/// available so status projections can be truthful.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalStatus {
    pub lexical_candidates: usize,
    pub vector_candidates: usize,
    pub used_lexical_fallback: bool,
    pub reranked: bool,
}

/// Deterministic two-stage retrieval with explicit governance and budgets.
#[derive(Debug, Clone)]
pub struct HybridRetrievalPipeline {
    pub ranking: MemoryRankingConfig,
    pub candidate_cap: usize,
    pub max_rerank_tokens: usize,
}

impl Default for HybridRetrievalPipeline {
    fn default() -> Self {
        Self {
            ranking: MemoryRankingConfig::default(),
            candidate_cap: 64,
            max_rerank_tokens: 1_024,
        }
    }
}

impl HybridRetrievalPipeline {
    pub fn new(ranking: MemoryRankingConfig) -> Self {
        Self {
            ranking,
            ..Self::default()
        }
    }

    /// Run scope filter, union, deduplication, ranking, diversity, and budget
    /// without mutating storage or access metadata.
    pub fn retrieve(
        &self,
        query: &str,
        visible_scopes: &[MemoryScope],
        sources: &[&dyn MemoryCandidateSource],
        limit: usize,
        max_chars: usize,
    ) -> Result<Vec<MemoryCandidate>, MemoryError> {
        self.retrieve_with_status(query, visible_scopes, sources, limit, max_chars)
            .map(|(items, _)| items)
    }

    pub fn retrieve_with_status(
        &self,
        query: &str,
        visible_scopes: &[MemoryScope],
        sources: &[&dyn MemoryCandidateSource],
        limit: usize,
        max_chars: usize,
    ) -> Result<(Vec<MemoryCandidate>, RetrievalStatus), MemoryError> {
        let mut by_id: HashMap<String, MemoryCandidate> = HashMap::new();
        let mut status = RetrievalStatus::default();
        for source in sources {
            let candidates = source.candidates(query, self.candidate_cap)?;
            for mut candidate in candidates {
                if !candidate.scope.is_visible_in(visible_scopes) {
                    continue;
                }
                if candidate.score_components.semantic > 0.0 {
                    status.vector_candidates += 1;
                } else {
                    status.lexical_candidates += 1;
                }
                candidate.score = candidate.score_components.weighted(&self.ranking);
                match by_id.get_mut(&candidate.id) {
                    Some(existing) => {
                        existing.score_components.semantic = existing
                            .score_components
                            .semantic
                            .max(candidate.score_components.semantic);
                        existing.score_components.lexical = existing
                            .score_components
                            .lexical
                            .max(candidate.score_components.lexical);
                        existing.score = existing.score_components.weighted(&self.ranking);
                    }
                    None => {
                        by_id.insert(candidate.id.clone(), candidate);
                    }
                }
            }
        }
        status.used_lexical_fallback =
            status.vector_candidates == 0 && status.lexical_candidates > 0;
        let mut candidates: Vec<_> = by_id.into_values().collect();
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        // Greedy, deterministic MMR selection.  Candidate sources do not expose
        // vectors, so lexical Jaccard is the honest pairwise relation fallback;
        // semantic cosine is already used by the coordinator for relevance when
        // valid vectors are available.
        let lambda = self.ranking.diversity_lambda.clamp(0.0, 1.0);
        let mut remaining = candidates;
        let mut seen_content = HashSet::new();
        let mut selected_tokens: Vec<HashSet<String>> = Vec::new();
        let mut result = Vec::new();
        let mut chars = 0;
        while result.len() < limit && !remaining.is_empty() {
            let mut best: Option<(usize, f64, f64)> = None;
            for (index, candidate) in remaining.iter().enumerate() {
                let normalized: String = candidate
                    .content
                    .chars()
                    .filter(|ch| ch.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect();
                if normalized.is_empty() || seen_content.contains(&normalized) {
                    continue;
                }
                let candidate_tokens = token_set(&candidate.content);
                let max_similarity = selected_tokens
                    .iter()
                    .map(|selected| jaccard_similarity(&candidate_tokens, selected))
                    .fold(0.0, f64::max);
                let novelty = 1.0 - max_similarity;
                let mmr = (1.0 - lambda) * candidate.score + lambda * novelty;
                let better = match best {
                    None => true,
                    Some((best_index, best_mmr, _)) => {
                        mmr > best_mmr + f64::EPSILON
                            || ((mmr - best_mmr).abs() <= f64::EPSILON
                                && candidate.id < remaining[best_index].id)
                    }
                };
                if better {
                    best = Some((index, mmr, novelty));
                }
            }
            let Some((index, _, novelty)) = best else {
                break;
            };
            let mut candidate = remaining.swap_remove(index);
            let content_chars = candidate.content.chars().count();
            if chars + content_chars > max_chars {
                // Skip oversized candidates and continue looking for a bounded item.
                continue;
            }
            let normalized: String = candidate
                .content
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect();
            seen_content.insert(normalized);
            // Keep source relevance and make novelty explainable without
            // letting the post-selection annotation change ordering.
            candidate.score_components.novelty = novelty;
            selected_tokens.push(token_set(&candidate.content));
            chars += content_chars;
            result.push(candidate);
        }
        Ok((result, status))
    }

    /// Optional model reranking is bounded and falls back to deterministic
    /// ranking if the adapter fails by returning an empty result.
    pub async fn retrieve_with_reranker(
        &self,
        query: &str,
        visible_scopes: &[MemoryScope],
        sources: &[&dyn MemoryCandidateSource],
        limit: usize,
        max_chars: usize,
        reranker: Option<&dyn MemoryReranker>,
    ) -> Result<(Vec<MemoryCandidate>, RetrievalStatus), MemoryError> {
        let (mut items, mut status) =
            self.retrieve_with_status(query, visible_scopes, sources, limit, max_chars)?;
        if let Some(reranker) = reranker {
            let reranked = reranker
                .rerank(query, items.clone(), self.max_rerank_tokens)
                .await;
            if !reranked.is_empty() || items.is_empty() {
                items = reranked;
                status.reranked = true;
            }
        }
        Ok((items, status))
    }
}

fn token_set(text: &str) -> HashSet<String> {
    unicode_tokens(text).into_iter().collect()
}

fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn token_overlap_score(query: &[String], document: &[String]) -> f64 {
    if query.is_empty() || document.is_empty() {
        return 0.0;
    }
    let unique: HashSet<&String> = document.iter().collect();
    let matched = query.iter().filter(|token| unique.contains(token)).count();
    matched as f64 / query.len() as f64
}

/// Unicode fallback tokenizer: alphanumeric runs for Latin text and one
/// character tokens for CJK text.
pub fn unicode_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut run = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if is_cjk(ch) {
                if !run.is_empty() {
                    tokens.push(run.to_lowercase());
                    run.clear();
                }
                tokens.push(ch.to_string());
            } else {
                run.push(ch);
            }
        } else if !run.is_empty() {
            tokens.push(run.to_lowercase());
            run.clear();
        }
    }
    if !run.is_empty() {
        tokens.push(run.to_lowercase());
    }
    tokens
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryProvenance, ScoreComponents};

    fn candidate(id: &str, content: &str, scope: MemoryScope) -> MemoryCandidate {
        MemoryCandidate {
            id: id.into(),
            layer: "episodic".into(),
            scope,
            content: content.into(),
            score: 0.0,
            score_components: ScoreComponents::default(),
            provenance: MemoryProvenance::default(),
        }
    }

    #[test]
    fn mmr_suppresses_near_duplicates_and_populates_novelty() {
        let scope = MemoryScope::Global;
        let source = BasicLexicalCandidateSource::new(vec![
            candidate("a", "rust memory retrieval design", scope.clone()),
            candidate("b", "rust memory retrieval design details", scope.clone()),
            candidate("c", "sqlite governance retention", scope.clone()),
        ]);
        let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
            diversity_lambda: 0.8,
            ..MemoryRankingConfig::default()
        });
        let items = pipeline
            .retrieve("rust memory", &[scope], &[&source], 3, 1_000)
            .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "c", "b"]
        );
        assert!(items[0].score_components.novelty > 0.99);
        assert!(items[1].score_components.novelty > 0.99);
        assert!(items[2].score_components.novelty < 0.5);
    }

    #[test]
    fn mmr_tie_order_is_stable_and_character_budget_is_strict() {
        let scope = MemoryScope::Global;
        let source = BasicLexicalCandidateSource::new(vec![
            candidate("b", "alpha", scope.clone()),
            candidate("a", "beta", scope.clone()),
            candidate("c", "gamma", scope.clone()),
        ]);
        let pipeline = HybridRetrievalPipeline::default();
        let items = pipeline.retrieve("", &[scope], &[&source], 3, 9).unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(
            items
                .iter()
                .map(|item| item.content.chars().count())
                .sum::<usize>(),
            9
        );
    }

    #[test]
    fn invalid_vector_metadata_is_ignored_by_lexical_pipeline() {
        let scope = MemoryScope::Global;
        let source = BasicLexicalCandidateSource::new(vec![candidate(
            "invalid",
            "fallback lexical content",
            scope.clone(),
        )]);
        let pipeline = HybridRetrievalPipeline::default();
        let (items, status) = pipeline
            .retrieve_with_status("fallback", &[scope], &[&source], 1, 100)
            .unwrap();
        assert_eq!(items[0].id, "invalid");
        assert!(status.used_lexical_fallback);
        assert_eq!(items[0].score_components.semantic, 0.0);
        assert!(items[0].score_components.lexical > 0.0);
    }
    #[test]
    fn chinese_lexical_fallback_and_scope_filter_are_deterministic() {
        let source = BasicLexicalCandidateSource::new(vec![
            candidate(
                "a",
                "项目 Alpha 的记忆",
                MemoryScope::Project {
                    project_id: "a".into(),
                },
            ),
            candidate(
                "b",
                "项目 Beta 的记忆",
                MemoryScope::Project {
                    project_id: "b".into(),
                },
            ),
        ]);
        let pipeline = HybridRetrievalPipeline::default();
        let items = pipeline
            .retrieve(
                "项目",
                &[MemoryScope::Project {
                    project_id: "a".into(),
                }],
                &[&source],
                4,
                400,
            )
            .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a"]
        );
    }
}
