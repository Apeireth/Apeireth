//! Unified Memory Coordinator (`MemoryCoordinator`).
//!
//! Serves as the central orchestrator across all 4 memory layers:
//! - Working Memory (in-memory fast ring-buffer)
//! - Episodic Memory (governed SQLite, active/forgotten status, content override)
//! - Semantic / Personal Memory (user preferences, profile cards)
//! - Relational / Temporal Memory (knowledge graph facts, entity links)

use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_plugin::experience::{AssociationStore, KnowledgeGraphStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::preference::PreferenceStore;

use crate::consolidation::{ConsolidationReport, MemoryConsolidationJob};
use crate::context_compiler::ClosedWorldContextCompiler;
use crate::continuity_state::{ContinuityCompressor, ContinuityState};
use crate::layers::{
    MemoryLayerKind, MemoryRecallQuery, MemoryRecallResult, MemoryWritebackEntry,
    RecalledMemoryItem,
};
use crate::memory_governance::{
    GovernedEpisode, MemoryGovernanceError, MemoryGovernanceStatus, MemoryGovernanceStore,
};
use crate::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource, RetrievalStatus,
    StaticVectorCandidateSource,
};
use crate::scope::{
    EmbeddingProvider, MemoryCandidate, MemoryCandidateQuery, MemoryProvenance,
    MemoryRankingConfig, MemoryScope, ScopedMemoryBackend, ScoreComponents,
};
use crate::MemoryError;

const WORKING_RING_BUFFER_CAP: usize = 30;

/// Unified memory orchestrator coordinating all memory layers and pipelines.
pub struct MemoryCoordinator {
    backend: Arc<dyn MemoryBackend>,
    scoped_backend: Option<Arc<dyn ScopedMemoryBackend>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    governance: Arc<dyn MemoryGovernanceStore>,
    working: Mutex<HashMap<String, VecDeque<Episode>>>,
    preferences: Option<Arc<dyn PreferenceStore>>,
    graph: Option<Arc<dyn KnowledgeGraphStore>>,
    associations: Option<Arc<dyn AssociationStore>>,
    compressor: ContinuityCompressor,
    compiler: ClosedWorldContextCompiler,
    consolidation: MemoryConsolidationJob,
    ranking: MemoryRankingConfig,
}

impl MemoryCoordinator {
    /// Create a new memory coordinator with core backend and governance store.
    pub fn new(
        backend: Arc<dyn MemoryBackend>,
        governance: Arc<dyn MemoryGovernanceStore>,
    ) -> Self {
        Self {
            backend,
            scoped_backend: None,
            embedding_provider: None,
            governance,
            working: Mutex::new(HashMap::new()),
            preferences: None,
            graph: None,
            associations: None,
            compressor: ContinuityCompressor::new(),
            compiler: ClosedWorldContextCompiler::new(),
            consolidation: MemoryConsolidationJob::new(),
            ranking: MemoryRankingConfig::default(),
        }
    }

    /// Attach optional scoped storage backend for cross-session queries.
    #[must_use]
    pub fn with_scoped_backend(mut self, scoped_backend: Arc<dyn ScopedMemoryBackend>) -> Self {
        self.scoped_backend = Some(scoped_backend);
        self
    }

    /// Attach optional embedding provider for semantic candidate retrieval.
    #[must_use]
    pub fn with_embedding_provider(
        mut self,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        self.embedding_provider = Some(embedding_provider);
        self
    }

    /// Attach optional semantic preference store.
    #[must_use]
    pub fn with_preferences(mut self, preferences: Arc<dyn PreferenceStore>) -> Self {
        self.preferences = Some(preferences);
        self
    }

    /// Attach optional experience and relational stores.
    #[must_use]
    pub fn with_experience(
        mut self,
        graph: Arc<dyn KnowledgeGraphStore>,
        associations: Arc<dyn AssociationStore>,
    ) -> Self {
        self.graph = Some(graph);
        self.associations = Some(associations);
        self
    }

    /// Reference to the underlying governance store for direct mutations.
    pub fn governance(&self) -> &dyn MemoryGovernanceStore {
        self.governance.as_ref()
    }

    /// Reference to the underlying memory backend.
    pub fn backend(&self) -> &dyn MemoryBackend {
        self.backend.as_ref()
    }

    /// Reference to the optional scoped memory backend.
    pub fn scoped_backend(&self) -> Option<&dyn ScopedMemoryBackend> {
        self.scoped_backend.as_deref()
    }

    /// Reference to the optional embedding provider.
    pub fn embedding_provider(&self) -> Option<&dyn EmbeddingProvider> {
        self.embedding_provider.as_deref()
    }

    /// Configure the centralized deterministic ranking weights.
    #[must_use]
    pub fn with_ranking_config(mut self, ranking: MemoryRankingConfig) -> Self {
        self.ranking = ranking;
        self
    }

    /// Collect memory candidates across all requested layers.
    fn collect_candidates(
        &self,
        query: &MemoryRecallQuery,
        now_ms: i64,
    ) -> (
        Vec<MemoryCandidate>,
        HashMap<String, (MemoryLayerKind, i64, Option<String>)>,
        usize,
    ) {
        let mut candidates = Vec::new();
        let mut candidate_meta = HashMap::new();
        let mut governance_filtered = 0;

        // Layer 1: Working Memory
        if query.layers.contains(&MemoryLayerKind::Working) {
            let working_lock = self.working.lock().expect("working memory mutex");
            if let Some(session_episodes) = working_lock.get(&query.session_id) {
                for ep in session_episodes
                    .iter()
                    .rev()
                    .take((query.limit * 4).max(32))
                {
                    if let Ok(Some(gov)) = self.governance.get_governed(&ep.id) {
                        if gov.status == MemoryGovernanceStatus::Forgotten {
                            governance_filtered += 1;
                            continue;
                        }
                    }
                    let (scope, provenance) = self.episode_scope_metadata(ep);
                    if !scope.is_visible_in(&query.visible_scopes) {
                        governance_filtered += 1;
                        continue;
                    }
                    let delta_hours =
                        (now_ms - ep.timestamp * 1000).max(0) as f64 / (1000.0 * 3600.0);
                    let s_rec = (-query.recency_decay_lambda * delta_hours)
                        .exp()
                        .clamp(0.1, 1.0);
                    let id = ep.id.clone();
                    candidate_meta.insert(
                        id.clone(),
                        (
                            MemoryLayerKind::Working,
                            ep.timestamp * 1000,
                            Some(format!("working:{}", ep.session_id)),
                        ),
                    );
                    candidates.push(MemoryCandidate {
                        id,
                        layer: "working".to_string(),
                        scope,
                        content: ep.content.clone(),
                        score: 0.0,
                        score_components: ScoreComponents {
                            recency: s_rec,
                            importance: 0.8,
                            confidence: 0.8,
                            ..Default::default()
                        },
                        provenance,
                    });
                }
            }
        }

        // Layer 2: Episodic Memory (Governed SQLite / Scoped Storage)
        if query.layers.contains(&MemoryLayerKind::Episodic) {
            if let Some(scoped) = &self.scoped_backend {
                let candidate_query = MemoryCandidateQuery {
                    source_session: Some(query.session_id.clone()),
                    visible_scopes: query.visible_scopes.clone(),
                    limit: (query.limit * 8).max(64),
                    as_of_ms: query.as_of_ms,
                };
                if let Ok(scoped_episodes) = scoped.query_candidates(&candidate_query) {
                    for ep in scoped_episodes {
                        let (scope, provenance) = self.episode_scope_metadata(&ep);
                        if let Ok(Some(gov)) = self.governance.get_governed(&ep.id) {
                            if gov.status == MemoryGovernanceStatus::Forgotten {
                                governance_filtered += 1;
                                continue;
                            }
                        }
                        let mut content = ep.content.clone();
                        let mut importance = 0.5;
                        if let Ok(Some(gov)) = self.governance.get_governed(&ep.id) {
                            if let Some(c_override) = gov.content_override {
                                content = c_override;
                            }
                            if gov.protected {
                                importance = 0.9;
                            }
                        }
                        let delta_hours =
                            (now_ms - ep.timestamp * 1000).max(0) as f64 / (1000.0 * 3600.0);
                        let s_rec = (-query.recency_decay_lambda * delta_hours)
                            .exp()
                            .clamp(0.1, 1.0);
                        let id = ep.id.clone();
                        candidate_meta.insert(
                            id.clone(),
                            (
                                MemoryLayerKind::Episodic,
                                ep.timestamp * 1000,
                                Some(format!("episodic:{}", ep.session_id)),
                            ),
                        );
                        candidates.push(MemoryCandidate {
                            id,
                            layer: "episodic".to_string(),
                            scope,
                            content,
                            score: 0.0,
                            score_components: ScoreComponents {
                                recency: s_rec,
                                importance,
                                confidence: importance,
                                ..Default::default()
                            },
                            provenance,
                        });
                    }
                }
            } else if let Ok(raw_eps) = self
                .backend
                .recent_episodes(&query.session_id, (query.limit * 8).max(64))
            {
                for ep in raw_eps {
                    let (scope, provenance) = self.episode_scope_metadata(&ep);
                    if !scope.is_visible_in(&query.visible_scopes) {
                        governance_filtered += 1;
                        continue;
                    }
                    if let Ok(Some(gov)) = self.governance.get_governed(&ep.id) {
                        if gov.status == MemoryGovernanceStatus::Forgotten {
                            governance_filtered += 1;
                            continue;
                        }
                    }
                    let mut content = ep.content.clone();
                    let mut importance = 0.5;
                    if let Ok(Some(gov)) = self.governance.get_governed(&ep.id) {
                        if let Some(c_override) = gov.content_override {
                            content = c_override;
                        }
                        if gov.protected {
                            importance = 0.9;
                        }
                    }
                    let delta_hours =
                        (now_ms - ep.timestamp * 1000).max(0) as f64 / (1000.0 * 3600.0);
                    let s_rec = (-query.recency_decay_lambda * delta_hours)
                        .exp()
                        .clamp(0.1, 1.0);
                    let id = ep.id.clone();
                    candidate_meta.insert(
                        id.clone(),
                        (
                            MemoryLayerKind::Episodic,
                            ep.timestamp * 1000,
                            Some(format!("episodic:{}", ep.session_id)),
                        ),
                    );
                    candidates.push(MemoryCandidate {
                        id,
                        layer: "episodic".to_string(),
                        scope,
                        content,
                        score: 0.0,
                        score_components: ScoreComponents {
                            recency: s_rec,
                            importance,
                            confidence: importance,
                            ..Default::default()
                        },
                        provenance,
                    });
                }
            }
        }

        // Layer 3: Semantic / Personal Memory (Preferences)
        if query.layers.contains(&MemoryLayerKind::Semantic) {
            if let Some(pref_store) = &self.preferences {
                let session_id_parsed =
                    SessionId::from_str(&query.session_id).unwrap_or_else(|_| SessionId::new());
                if let Ok(prefs) = pref_store.recall_for_context(
                    &session_id_parsed,
                    &query.query_text,
                    (query.limit * 2).max(10) as u32,
                ) {
                    for pref in prefs {
                        let pref_ts_ms = if pref.created_at < 10_000_000_000 {
                            pref.created_at * 1000
                        } else {
                            pref.created_at
                        };
                        let delta_hours = (now_ms - pref_ts_ms).max(0) as f64 / (1000.0 * 3600.0);
                        let s_rec = (-query.recency_decay_lambda * delta_hours)
                            .exp()
                            .clamp(0.1, 1.0);
                        let s_imp = pref.confidence.clamp(0.1, 1.0);
                        let id = format!("pref:{}", pref.id);
                        candidate_meta.insert(
                            id.clone(),
                            (
                                MemoryLayerKind::Semantic,
                                pref_ts_ms,
                                Some(format!("preference:{}", pref.id)),
                            ),
                        );
                        candidates.push(MemoryCandidate {
                            id,
                            layer: "semantic".to_string(),
                            scope: MemoryScope::Session {
                                session_id: query.session_id.clone(),
                            },
                            content: format!("Topic: {}. Preference: {}", pref.topic, pref.stance),
                            score: 0.0,
                            score_components: ScoreComponents {
                                recency: s_rec,
                                importance: s_imp,
                                confidence: s_imp,
                                ..Default::default()
                            },
                            provenance: MemoryProvenance {
                                source: "preference".to_string(),
                                source_session: Some(query.session_id.clone()),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
        }

        // Layer 4: Relational / Temporal Memory (Graph & Associations)
        if query.layers.contains(&MemoryLayerKind::Relational)
            && !query.query_text.trim().is_empty()
        {
            if let Some(graph_store) = &self.graph {
                if let Ok(facts) =
                    graph_store.facts_from(&query.query_text, (query.limit * 2).max(10) as u32)
                {
                    for fact in facts {
                        let id = format!(
                            "fact:{}:{}:{}",
                            fact.subject_id, fact.predicate, fact.object_id
                        );
                        candidate_meta.insert(
                            id.clone(),
                            (
                                MemoryLayerKind::Relational,
                                now_ms,
                                Some("knowledge_graph".to_string()),
                            ),
                        );
                        candidates.push(MemoryCandidate {
                            id,
                            layer: "relational".to_string(),
                            scope: MemoryScope::Session {
                                session_id: query.session_id.clone(),
                            },
                            content: format!(
                                "{} {} {}",
                                fact.subject_id, fact.predicate, fact.object_id
                            ),
                            score: 0.0,
                            score_components: ScoreComponents {
                                recency: 1.0,
                                importance: 0.6,
                                confidence: 0.6,
                                ..Default::default()
                            },
                            provenance: MemoryProvenance {
                                source: "knowledge_graph".to_string(),
                                source_session: Some(query.session_id.clone()),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
        }

        (candidates, candidate_meta, governance_filtered)
    }

    /// Execute the Unified Recall Pipeline across requested layers synchronously.
    pub fn recall(&self, query: &MemoryRecallQuery) -> Result<MemoryRecallResult, MemoryError> {
        if self.embedding_provider.is_none() {
            let now_ms = query
                .as_of_ms
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
            let (candidates, candidate_meta, governance_filtered) =
                self.collect_candidates(query, now_ms);
            let total_candidates = candidates.len();
            if total_candidates == 0 {
                return Ok(MemoryRecallResult {
                    items: Vec::new(),
                    total_candidates: 0,
                    governance_filtered,
                    total_chars: 0,
                    retrieval_status: Some(RetrievalStatus {
                        lexical_candidates: 0,
                        vector_candidates: 0,
                        used_lexical_fallback: true,
                        reranked: false,
                    }),
                });
            }
            self.execute_hybrid_retrieval(
                query,
                candidates,
                candidate_meta,
                governance_filtered,
                total_candidates,
                None,
                now_ms,
            )
        } else {
            block_on_future(self.recall_async(query))
        }
    }

    /// Execute the Unified Recall Pipeline asynchronously, running semantic embeddings if configured.
    pub async fn recall_async(
        &self,
        query: &MemoryRecallQuery,
    ) -> Result<MemoryRecallResult, MemoryError> {
        let now_ms = query
            .as_of_ms
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let (candidates, candidate_meta, governance_filtered) =
            self.collect_candidates(query, now_ms);

        let total_candidates = candidates.len();
        if total_candidates == 0 {
            return Ok(MemoryRecallResult {
                items: Vec::new(),
                total_candidates: 0,
                governance_filtered,
                total_chars: 0,
                retrieval_status: Some(RetrievalStatus {
                    lexical_candidates: 0,
                    vector_candidates: 0,
                    used_lexical_fallback: self.embedding_provider.is_none(),
                    reranked: false,
                }),
            });
        }

        let mut vector_source = None;
        if let Some(ref provider) = self.embedding_provider {
            if !query.query_text.trim().is_empty() {
                if let Ok(query_vector) = provider.embed(&query.query_text).await {
                    if !query_vector.is_empty() {
                        let mut vector_candidates = Vec::new();
                        for cand in &candidates {
                            let cand_vec_opt = if let Ok(Some(meta)) =
                                self.backend.get_episode_metadata(&cand.id)
                            {
                                meta.get("vector").and_then(|v| {
                                    serde_json::from_value::<Vec<f32>>(v.clone()).ok()
                                })
                            } else {
                                None
                            };
                            let cand_vec = match cand_vec_opt {
                                Some(v) => Some(v),
                                None => provider.embed(&cand.content).await.ok(),
                            };
                            if let Some(cand_vec) = cand_vec {
                                if cand_vec.len() == query_vector.len() {
                                    let sim = crate::canonical::vector::cosine_similarity(
                                        &query_vector,
                                        &cand_vec,
                                    );
                                    let mut vc = cand.clone();
                                    vc.score_components.semantic = f64::from(sim).clamp(0.0, 1.0);
                                    vector_candidates.push(vc);
                                }
                            }
                        }
                        if !vector_candidates.is_empty() {
                            vector_source =
                                Some(StaticVectorCandidateSource::new(vector_candidates));
                        }
                    }
                }
            }
        }

        self.execute_hybrid_retrieval(
            query,
            candidates,
            candidate_meta,
            governance_filtered,
            total_candidates,
            vector_source,
            now_ms,
        )
    }

    fn execute_hybrid_retrieval(
        &self,
        query: &MemoryRecallQuery,
        candidates: Vec<MemoryCandidate>,
        candidate_meta: HashMap<String, (MemoryLayerKind, i64, Option<String>)>,
        governance_filtered: usize,
        total_candidates: usize,
        vector_source: Option<StaticVectorCandidateSource>,
        now_ms: i64,
    ) -> Result<MemoryRecallResult, MemoryError> {
        let lexical_source = Bm25LexicalCandidateSource::new(candidates);
        let mut sources: Vec<&dyn MemoryCandidateSource> = vec![&lexical_source];
        if let Some(ref vs) = vector_source {
            sources.push(vs);
        }

        let pipeline = HybridRetrievalPipeline::new(self.ranking);
        let (ranked_candidates, mut status) = pipeline.retrieve_with_status(
            &query.query_text,
            &query.visible_scopes,
            &sources,
            query.limit,
            query.max_chars,
        )?;

        if self.embedding_provider.is_none() {
            status.used_lexical_fallback = true;
        }

        let mut final_items = Vec::new();
        let mut total_chars = 0;
        for cand in ranked_candidates {
            if cand.score < query.min_score {
                continue;
            }
            if final_items.len() >= query.limit {
                break;
            }
            if total_chars + cand.content.len() > query.max_chars && !final_items.is_empty() {
                break;
            }
            total_chars += cand.content.len();
            let (layer, timestamp_ms, source_ref) = candidate_meta
                .get(&cand.id)
                .cloned()
                .unwrap_or((MemoryLayerKind::Episodic, now_ms, None));
            final_items.push(RecalledMemoryItem {
                id: cand.id,
                layer,
                content: cand.content,
                timestamp_ms,
                score: cand.score,
                importance: cand.score_components.importance,
                source_ref,
                score_components: Some(cand.score_components),
            });
        }

        Ok(MemoryRecallResult {
            items: final_items,
            total_candidates,
            governance_filtered,
            total_chars,
            retrieval_status: Some(status),
        })
    }

    /// Persist turn writeback entry into Working and Episodic layers.
    pub fn writeback(&self, entry: &MemoryWritebackEntry) -> Result<String, MemoryError> {
        let episode_id = format!("ep-{}", uuid::Uuid::new_v4());
        let timestamp_secs = if let Some(ms) = entry.timestamp_ms {
            ms / 1000
        } else {
            chrono::Utc::now().timestamp()
        };

        let episode = Episode {
            id: episode_id.clone(),
            timestamp: timestamp_secs,
            role: entry.role.clone(),
            content: entry.content.clone(),
            session_id: entry.session_id.clone(),
        };

        // 1. Update working memory ring buffer
        {
            let mut working_lock = self.working.lock().expect("working memory mutex");
            let ring = working_lock
                .entry(entry.session_id.clone())
                .or_insert_with(|| VecDeque::with_capacity(WORKING_RING_BUFFER_CAP));
            if ring.len() >= WORKING_RING_BUFFER_CAP {
                ring.pop_front();
            }
            ring.push_back(episode.clone());
        }

        // 2. Persist to storage backend
        self.backend
            .put_episode(&episode)
            .map_err(|e| MemoryError::Invalid(e.to_string()))?;

        self.backend
            .put_episode_metadata(
                &episode_id,
                serde_json::json!({
                    "scope": entry.scope,
                    "provenance": entry.provenance,
                    "layer": "episodic",
                    "content_hash": crate::canonical::vector::content_hash(&entry.content),
                }),
            )
            .map_err(|e| MemoryError::Invalid(e.to_string()))?;

        Ok(episode_id)
    }

    /// Compile a structured closed-world prompt overlay from a recall query.
    pub fn compile_prompt_overlay(
        &self,
        query: &MemoryRecallQuery,
    ) -> Result<Option<String>, MemoryError> {
        let recall_result = self.recall(query)?;
        Ok(self
            .compiler
            .compile(&recall_result, &query.session_id, query.max_chars))
    }

    /// Generate a bounded continuity state compression for a session.
    pub fn compress_continuity(
        &self,
        session_id: &str,
        max_summary_chars: usize,
    ) -> Result<ContinuityState, MemoryError> {
        let episodes = self
            .backend
            .recent_episodes(session_id, 50)
            .map_err(|e| MemoryError::Invalid(e.to_string()))?;
        Ok(self
            .compressor
            .compress(session_id, &episodes, max_summary_chars))
    }

    /// Run background or idle memory consolidation job for a session.
    pub fn run_consolidation(&self, session_id: &str) -> Result<ConsolidationReport, MemoryError> {
        let episodes = self
            .backend
            .recent_episodes(session_id, 100)
            .map_err(|e| MemoryError::Invalid(e.to_string()))?;
        Ok(self.consolidation.consolidate(session_id, &episodes))
    }

    /// Forget an episode via the governance sidecar.
    pub fn forget_episode(
        &self,
        episode_id: &str,
        reason: Option<&str>,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        let result = self
            .governance
            .forget_episode(episode_id, reason, expected_rev)?;
        {
            let mut working_lock = self.working.lock().expect("working memory mutex");
            for ring in working_lock.values_mut() {
                ring.retain(|ep| ep.id != episode_id);
            }
        }
        Ok(result)
    }

    /// Protect an episode from automatic purging or forgetting.
    pub fn protect_episode(
        &self,
        episode_id: &str,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        self.governance.protect_episode(episode_id, expected_rev)
    }

    /// Unprotect an episode.
    pub fn unprotect_episode(
        &self,
        episode_id: &str,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        self.governance.unprotect_episode(episode_id, expected_rev)
    }

    /// Update an episode's content override.
    pub fn update_episode_content(
        &self,
        episode_id: &str,
        new_content: &str,
        updated_by: Option<&str>,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        let result = self.governance.update_episode_content(
            episode_id,
            new_content,
            updated_by,
            expected_rev,
        )?;
        if let Ok(Some(mut meta)) = self.backend.get_episode_metadata(episode_id) {
            let new_hash = crate::canonical::vector::content_hash(new_content);
            if let Some(obj) = meta.as_object_mut() {
                obj.insert(
                    "content_hash".to_string(),
                    serde_json::Value::String(new_hash),
                );
                obj.remove("vector");
                let _ = self.backend.put_episode_metadata(episode_id, meta);
            }
        }
        Ok(result)
    }

    fn episode_scope_metadata(&self, episode: &Episode) -> (MemoryScope, MemoryProvenance) {
        let fallback_scope = MemoryScope::Session {
            session_id: episode.session_id.clone(),
        };
        let fallback_provenance = MemoryProvenance {
            source_session: Some(episode.session_id.clone()),
            ..MemoryProvenance::default()
        };
        let Ok(Some(metadata)) = self.backend.get_episode_metadata(&episode.id) else {
            return (fallback_scope, fallback_provenance);
        };
        let scope = metadata
            .get("scope")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or(fallback_scope);
        let provenance = metadata
            .get("provenance")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or(fallback_provenance);
        (scope, provenance)
    }
}

fn block_on_future<F>(f: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(f))
            }
            _ => std::thread::scope(|s| {
                s.spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("local runtime")
                        .block_on(f)
                })
                .join()
                .expect("thread join")
            }),
        },
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("local runtime")
            .block_on(f),
    }
}
