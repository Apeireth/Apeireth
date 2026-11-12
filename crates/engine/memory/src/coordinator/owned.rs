//! Stable-id additive persistence for module-owned, explicitly scoped memory.
use super::*;
impl MemoryCoordinator {
    pub fn with_evidence_source(
        mut self,
        source: std::sync::Arc<dyn crate::context_compiler::MemoryEvidenceSource>,
    ) -> Self {
        self.evidence_source = Some(source);
        self
    }
    pub(super) fn source_evidence(
        &self,
        id: &str,
        content: &str,
    ) -> crate::context_compiler::MemorySourceEvidence {
        self.evidence_source
            .as_ref()
            .and_then(|source| {
                source
                    .evidence_for(id.strip_prefix("typed:episode:").unwrap_or(id), content)
                    .ok()
            })
            .unwrap_or_default()
    }
    pub(super) fn source_coverage(
        &self,
        candidates: Vec<MemoryCandidate>,
        ranking: &MemoryRankingConfig,
    ) -> Vec<MemoryCandidate> {
        use crate::context_compiler::SourceRole;
        if self.evidence_source.is_none() {
            return candidates;
        }
        let evidence: HashMap<_, _> = candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.id.clone(),
                    self.source_evidence(&candidate.id, &candidate.content),
                )
            })
            .collect();
        let group = |candidate: &MemoryCandidate| {
            let source = &evidence[&candidate.id];
            (
                source.source_group.is_some(),
                source
                    .source_group
                    .clone()
                    .unwrap_or_else(|| candidate.id.clone()),
            )
        };
        let mut representatives: HashMap<_, usize> = HashMap::new();
        for (index, candidate) in candidates.iter().enumerate() {
            let source = &evidence[&candidate.id];
            if !source.citable_as_fact {
                continue;
            }
            let representative = representatives.entry(group(candidate)).or_insert(index);
            let prior = &candidates[*representative];
            if candidate.score > prior.score
                || (candidate.score.total_cmp(&prior.score).is_eq()
                    && source.source_role == SourceRole::User
                    && evidence[&prior.id].source_role != SourceRole::User)
            {
                *representative = index;
            }
        }
        // Independent qualified sources precede their aliases, then traces.
        // This source policy changes delivery priority, never relevance scores
        // or eligibility. Configured MMR still orders each bounded stratum.
        let mut strata = [Vec::new(), Vec::new(), Vec::new()];
        for (index, candidate) in candidates.into_iter().enumerate() {
            let source = &evidence[&candidate.id];
            let stratum = if !source.citable_as_fact {
                2
            } else if representatives.get(&group(&candidate)) == Some(&index) {
                0
            } else {
                1
            };
            strata[stratum].push(candidate);
        }
        strata
            .into_iter()
            .flat_map(|stratum| diversity::order_bounded(ranking, stratum))
            .collect()
    }

    /// Additional frozen projection evidence is committed with the episode.
    /// Scope, provenance, layer and content hash always come from this call.
    pub fn writeback_scoped_episode_with_metadata(
        &self,
        episode: &Episode,
        scope: &MemoryScope,
        provenance: &MemoryProvenance,
        extra: serde_json::Value,
    ) -> Result<String, MemoryError> {
        self.persist_episode_with_metadata(episode, scope, provenance, extra)?;
        Ok(episode.id.clone())
    }
    pub fn writeback_scoped_episode(
        &self,
        episode: &Episode,
        scope: &MemoryScope,
        provenance: &MemoryProvenance,
    ) -> Result<String, MemoryError> {
        self.persist_episode(episode, scope, provenance)?;
        Ok(episode.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_compiler::{
        MemoryClaimKind, MemoryEvidenceSource, MemorySourceEvidence, SourceRole,
    };

    struct Sources(HashMap<String, MemorySourceEvidence>);
    impl MemoryEvidenceSource for Sources {
        fn evidence_for(
            &self,
            id: &str,
            _content: &str,
        ) -> crate::MemoryResult<MemorySourceEvidence> {
            Ok(self.0.get(id).cloned().unwrap_or_default())
        }
    }
    fn candidate(id: &str, text: &str, score: f64) -> MemoryCandidate {
        MemoryCandidate {
            id: id.into(),
            content: text.into(),
            score,
            layer: "episodic".into(),
            scope: MemoryScope::Global,
            score_components: ScoreComponents::default(),
            provenance: MemoryProvenance::default(),
        }
    }
    fn source(role: SourceRole, group: &str) -> MemorySourceEvidence {
        MemorySourceEvidence {
            source_role: role,
            claim_kind: MemoryClaimKind::DirectUserAssertion,
            citable_as_fact: role != SourceRole::Assistant,
            source_group: Some(group.into()),
            ..Default::default()
        }
    }

    #[test]
    fn source_strata_keep_configured_mmr_and_original_relevance() {
        let store = Arc::new(crate::fresh_store());
        let coordinator = MemoryCoordinator::new(store.clone(), store).with_evidence_source(
            Arc::new(Sources(HashMap::from([
                ("a".into(), source(SourceRole::User, "a")),
                ("b".into(), source(SourceRole::User, "b")),
                ("c".into(), source(SourceRole::User, "c")),
                ("trace".into(), source(SourceRole::Assistant, "trace")),
            ]))),
        );
        let candidates = vec![
            candidate("trace", "assistant city confirmation", 10.0),
            candidate("a", "rust memory retrieval design", 1.0),
            candidate("b", "rust memory retrieval design details", 0.99),
            candidate("c", "sqlite governance retention", 0.8),
        ];
        for (lambda, expected) in [
            (0.0, vec!["a", "b", "c", "trace"]),
            (0.8, vec!["a", "c", "b", "trace"]),
        ] {
            let selected = coordinator.source_coverage(
                candidates.clone(),
                &MemoryRankingConfig {
                    diversity_lambda: lambda,
                    ..Default::default()
                },
            );
            assert_eq!(
                selected
                    .iter()
                    .map(|item| item.id.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            for item in selected {
                assert_eq!(
                    item.score,
                    candidates
                        .iter()
                        .find(|candidate| candidate.id == item.id)
                        .unwrap()
                        .score
                );
            }
        }
    }

    #[test]
    fn group_representatives_use_relevance_before_original_role_ties() {
        let store = Arc::new(crate::fresh_store());
        let coordinator = MemoryCoordinator::new(store.clone(), store).with_evidence_source(
            Arc::new(Sources(HashMap::from([
                ("raw".into(), source(SourceRole::User, "one")),
                ("focused".into(), source(SourceRole::Memory, "one")),
                ("independent".into(), source(SourceRole::User, "two")),
            ]))),
        );
        let mut candidates = vec![
            candidate("focused", "focused original fact", 0.9),
            candidate("independent", "independent project report", 0.8),
            candidate("raw", "long original statement", 0.2),
        ];
        let ranked =
            coordinator.source_coverage(candidates.clone(), &MemoryRankingConfig::default());
        assert_eq!(
            ranked
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["focused", "independent", "raw"]
        );
        candidates[2].score = 0.9;
        let tied = coordinator.source_coverage(candidates, &MemoryRankingConfig::default());
        assert_eq!(
            tied.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            vec!["raw", "independent", "focused"]
        );
    }
}
