//! Deterministic, candidate-only proactive memory recall.
//!
//! This module only ranks and bounds candidates supplied by the caller. It does
//! not access storage, start workers, call an LLM, or modify provider prompts.

use crate::{MemoryCandidate, MemoryScope, TopicCue, TopicPredictor};
use serde::{Deserialize, Serialize};

/// Opt-in policy for proactive candidate selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProactiveRecallPolicy {
    /// Proactive recall is disabled by default.
    pub enabled: bool,
    /// Maximum number of candidates returned per invocation.
    pub budget: usize,
    /// Minimum predicted topic confidence required to activate recall.
    pub confidence_threshold: f32,
    /// Existing candidate score floor. A zero floor accepts lexical candidates.
    pub min_candidate_score: f64,
    /// Closed-world visibility boundary. Empty means no scope filtering here.
    pub visible_scopes: Vec<MemoryScope>,
}

impl Default for ProactiveRecallPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            budget: 2,
            confidence_threshold: 0.10,
            min_candidate_score: 0.0,
            visible_scopes: Vec::new(),
        }
    }
}

impl ProactiveRecallPolicy {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn with_budget(mut self, budget: usize) -> Self {
        self.budget = budget;
        self
    }

    #[must_use]
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_min_candidate_score(mut self, score: f64) -> Self {
        self.min_candidate_score = score;
        self
    }

    #[must_use]
    pub fn with_visible_scopes(mut self, scopes: Vec<MemoryScope>) -> Self {
        self.visible_scopes = scopes;
        self
    }
}

/// Stateless proactive recall service. The input candidates remain candidates;
/// no memory is persisted or promoted by this service.
#[derive(Debug, Clone, Default)]
pub struct ProactiveRecallService {
    policy: ProactiveRecallPolicy,
}

impl ProactiveRecallService {
    #[must_use]
    pub fn new(policy: ProactiveRecallPolicy) -> Self {
        Self { policy }
    }

    #[must_use]
    pub fn policy(&self) -> &ProactiveRecallPolicy {
        &self.policy
    }

    /// Select relevant candidates for the supplied conversational cue.
    pub fn recall(&self, cue: &TopicCue, candidates: &[MemoryCandidate]) -> Vec<MemoryCandidate> {
        if !self.policy.enabled || self.policy.budget == 0 || candidates.is_empty() {
            return Vec::new();
        }
        let prediction = TopicPredictor::predict(cue);
        let topics: Vec<(&str, f32)> = prediction
            .hints
            .iter()
            .filter(|hint| hint.confidence >= self.policy.confidence_threshold)
            .map(|hint| (hint.topic.as_str(), hint.confidence))
            .collect();
        if topics.is_empty() {
            return Vec::new();
        }

        let mut ranked: Vec<(f32, &MemoryCandidate)> = candidates
            .iter()
            .filter(|candidate| {
                self.policy.visible_scopes.is_empty()
                    || candidate.scope.is_visible_in(&self.policy.visible_scopes)
            })
            .filter(|candidate| candidate.score >= self.policy.min_candidate_score)
            .filter_map(|candidate| {
                let content = candidate.content.to_lowercase();
                let relevance = topics
                    .iter()
                    .filter(|(topic, _)| content.contains(&topic.to_lowercase()))
                    .map(|(_, confidence)| *confidence)
                    .fold(0.0_f32, f32::max);
                (relevance > 0.0).then_some((relevance, candidate))
            })
            .collect();
        ranked.sort_by(|(a_score, a), (b_score, b)| {
            b_score
                .total_cmp(a_score)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.id.cmp(&b.id))
        });
        ranked
            .into_iter()
            .take(self.policy.budget)
            .map(|(_, candidate)| candidate.clone())
            .collect()
    }

    /// Alias emphasizing that this operation never fetches from storage.
    pub fn select_candidates(
        &self,
        cue: &TopicCue,
        candidates: &[MemoryCandidate],
    ) -> Vec<MemoryCandidate> {
        self.recall(cue, candidates)
    }

    /// Convenience entry point for callers that do not need to retain a service.
    pub fn recall_with_policy(
        policy: &ProactiveRecallPolicy,
        cue: &TopicCue,
        candidates: &[MemoryCandidate],
    ) -> Vec<MemoryCandidate> {
        Self::new(policy.clone()).recall(cue, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryProvenance, ScoreComponents};

    fn candidate(id: &str, content: &str, score: f64) -> MemoryCandidate {
        MemoryCandidate {
            id: id.into(),
            layer: "episodic".into(),
            scope: MemoryScope::Global,
            content: content.into(),
            score,
            score_components: ScoreComponents::default(),
            provenance: MemoryProvenance::default(),
        }
    }
    fn cue(text: &str) -> TopicCue {
        TopicCue {
            recent_user_messages: vec![text.into()],
            ..Default::default()
        }
    }

    #[test]
    fn budget_is_hard_limit_and_order_is_deterministic() {
        let service = ProactiveRecallService::new(
            ProactiveRecallPolicy::default()
                .enabled(true)
                .with_budget(1),
        );
        let got = service.recall(
            &cue("请看 rust 项目"),
            &[
                candidate("b", "project", 0.8),
                candidate("a", "project", 0.8),
            ],
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "a");
    }

    #[test]
    fn threshold_excludes_weak_prediction() {
        let service = ProactiveRecallService::new(
            ProactiveRecallPolicy::default()
                .enabled(true)
                .with_confidence_threshold(0.9),
        );
        assert!(service
            .recall(&cue("请看 rust 项目"), &[candidate("x", "project", 1.0)])
            .is_empty());
    }

    #[test]
    fn disabled_returns_no_candidates() {
        let service = ProactiveRecallService::new(ProactiveRecallPolicy::default());
        assert!(service
            .recall(&cue("rust 项目"), &[candidate("x", "project", 1.0)])
            .is_empty());
    }

    #[test]
    fn irrelevant_candidates_are_not_recalled() {
        let service = ProactiveRecallService::new(ProactiveRecallPolicy::default().enabled(true));
        assert!(service
            .recall(&cue("rust 项目"), &[candidate("x", "旅行和音乐", 1.0)])
            .is_empty());
    }
}
