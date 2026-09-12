//! Canonical Memory Plane scope, provenance, ranking, and provider contracts.
//!
//! These types are deliberately independent of Runtime, providers, and
//! credentials. Assembly owns concrete adapters; Memory owns the semantics.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use apeireth_core::kernel::Timestamp;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Visibility boundary for a memory record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum MemoryScope {
    /// Shared across all users and personas in the installation.
    Global,
    /// Visible to one user.
    User { user_id: String },
    /// Visible to one persona for one user.
    Persona { persona_id: String, user_id: String },
    /// Visible to one project.
    Project { project_id: String },
    /// Visible only to one session.
    Session { session_id: String },
}

/// Query specification for storage backends that can retrieve candidate episodes
/// according to explicit visibility scopes, rather than restricting only to
/// the active session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCandidateQuery {
    pub source_session: Option<String>,
    pub visible_scopes: Vec<MemoryScope>,
    pub limit: usize,
    pub as_of_ms: Option<i64>,
}

/// A storage backend capable of evaluating candidate visibility across scopes.
///
/// This moves candidate generation from `WHERE session_id = ?` into a true
/// scope-aware query at the storage layer.
pub trait ScopedMemoryBackend: Send + Sync {
    fn query_candidates(
        &self,
        query: &MemoryCandidateQuery,
    ) -> Result<Vec<apeireth_core::Episode>, Box<dyn std::error::Error + Send + Sync>>;
}

impl MemoryScope {
    /// A conservative default for legacy session-linked episodes.
    pub fn session(session_id: impl Into<String>) -> Self {
        Self::Session {
            session_id: session_id.into(),
        }
    }

    /// Stable display label for projections and diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::User { .. } => "user",
            Self::Persona { .. } => "persona",
            Self::Project { .. } => "project",
            Self::Session { .. } => "session",
        }
    }

    /// Whether this record is visible through the supplied explicit scope.
    /// Matching is exact for scoped identities; Global is only visible when
    /// Global is explicitly present in the query.
    pub fn is_visible_in(&self, visible_scopes: &[Self]) -> bool {
        visible_scopes.iter().any(|visible| match (self, visible) {
            (Self::Global, Self::Global) => true,
            (Self::User { user_id: a }, Self::User { user_id: b }) => a == b,
            (
                Self::Persona {
                    persona_id: a_persona,
                    user_id: a_user,
                },
                Self::Persona {
                    persona_id: b_persona,
                    user_id: b_user,
                },
            ) => a_persona == b_persona && a_user == b_user,
            (Self::Project { project_id: a }, Self::Project { project_id: b }) => a == b,
            (Self::Session { session_id: a }, Self::Session { session_id: b }) => a == b,
            _ => false,
        })
    }
}

impl fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => f.write_str("global"),
            Self::User { user_id } => write!(f, "user:{user_id}"),
            Self::Persona {
                persona_id,
                user_id,
            } => write!(f, "persona:{persona_id}@{user_id}"),
            Self::Project { project_id } => write!(f, "project:{project_id}"),
            Self::Session { session_id } => write!(f, "session:{session_id}"),
        }
    }
}

/// Safe provenance attached to every extracted or written memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub source: String,
    pub source_session: Option<String>,
    pub source_trace: Option<String>,
    pub source_request: Option<String>,
}

impl Default for MemoryProvenance {
    fn default() -> Self {
        Self {
            source: "runtime".to_string(),
            source_session: None,
            source_trace: None,
            source_request: None,
        }
    }
}

/// Explainable components of a deterministic retrieval score.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub semantic: f64,
    pub lexical: f64,
    pub importance: f64,
    pub recency: f64,
    pub activation: f64,
    pub continuity: f64,
    pub confidence: f64,
    /// Relevance supplied by the knowledge graph or relational layer.
    #[serde(default)]
    pub graph: f64,
    /// A residual/coverage signal favouring information not already represented.
    #[serde(default)]
    pub novelty: f64,
}

impl ScoreComponents {
    pub fn weighted(self, config: &MemoryRankingConfig) -> f64 {
        self.semantic * config.semantic_weight
            + self.lexical * config.lexical_weight
            + self.importance * config.importance_weight
            + self.recency * config.recency_weight
            + self.activation * config.activation_weight
            + self.continuity * config.continuity_weight
            + self.confidence * config.confidence_weight
            + self.graph * config.graph_weight
            + self.novelty * config.novelty_weight
    }
}

/// Centralized ranking weights. No retrieval stage should own magic weights.
///
/// `diversity_lambda` is the control for a later MMR selection stage: zero
/// means relevance-only selection and one means diversity-only selection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryRankingConfig {
    pub semantic_weight: f64,
    pub lexical_weight: f64,
    pub importance_weight: f64,
    pub recency_weight: f64,
    pub activation_weight: f64,
    pub continuity_weight: f64,
    pub confidence_weight: f64,
    pub graph_weight: f64,
    pub novelty_weight: f64,
    pub diversity_lambda: f64,
}

impl Default for MemoryRankingConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.25,
            lexical_weight: 0.20,
            importance_weight: 0.15,
            recency_weight: 0.10,
            activation_weight: 0.10,
            continuity_weight: 0.05,
            confidence_weight: 0.05,
            graph_weight: 0.05,
            novelty_weight: 0.05,
            diversity_lambda: 0.20,
        }
    }
}

/// Structured, serializable policy shared by callers performing memory recall.
///
/// The empty default visibility set is intentional: callers must opt into the
/// scopes they want to expose (closed-world recall).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecallPolicy {
    pub visible_scopes: Vec<MemoryScope>,
    pub layers: Vec<crate::layers::MemoryLayerKind>,
    pub limit: usize,
    pub max_chars: usize,
    pub proactive_budget: usize,
    pub min_score: f64,
    pub as_of_ms: Option<i64>,
    pub token_budget: usize,
}

impl Default for RecallPolicy {
    fn default() -> Self {
        Self {
            visible_scopes: Vec::new(),
            layers: vec![
                crate::layers::MemoryLayerKind::Working,
                crate::layers::MemoryLayerKind::Episodic,
                crate::layers::MemoryLayerKind::Semantic,
                crate::layers::MemoryLayerKind::Relational,
            ],
            limit: 8,
            max_chars: 4_000,
            proactive_budget: 2,
            min_score: 0.10,
            as_of_ms: None,
            token_budget: 1_024,
        }
    }
}

impl RecallPolicy {
    /// Construct a policy with the supplied visibility boundary.
    #[must_use]
    pub fn new(visible_scopes: Vec<MemoryScope>) -> Self {
        Self {
            visible_scopes,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_visible_scopes(mut self, visible_scopes: Vec<MemoryScope>) -> Self {
        self.visible_scopes = visible_scopes;
        self
    }

    #[must_use]
    pub fn with_visible_scope(mut self, scope: MemoryScope) -> Self {
        if !self.visible_scopes.contains(&scope) {
            self.visible_scopes.push(scope);
        }
        self
    }

    #[must_use]
    pub fn with_layers(mut self, layers: Vec<crate::layers::MemoryLayerKind>) -> Self {
        self.layers = layers;
        self
    }

    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    #[must_use]
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars.max(128);
        self
    }

    #[must_use]
    pub fn with_proactive_budget(mut self, proactive_budget: usize) -> Self {
        self.proactive_budget = proactive_budget;
        self
    }

    #[must_use]
    pub fn with_min_score(mut self, min_score: f64) -> Self {
        self.min_score = min_score;
        self
    }

    #[must_use]
    pub fn with_as_of_ms(mut self, as_of_ms: i64) -> Self {
        self.as_of_ms = Some(as_of_ms);
        self
    }

    #[must_use]
    pub fn without_as_of(mut self) -> Self {
        self.as_of_ms = None;
        self
    }

    #[must_use]
    pub fn with_token_budget(mut self, token_budget: usize) -> Self {
        self.token_budget = token_budget.max(1);
        self
    }

    #[must_use]
    pub fn with_as_of(mut self, as_of_ms: Option<i64>) -> Self {
        self.as_of_ms = as_of_ms;
        self
    }
}

/// A retrieval candidate with enough safe metadata to explain its ranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub id: String,
    pub layer: String,
    pub scope: MemoryScope,
    pub content: String,
    pub score: f64,
    pub score_components: ScoreComponents,
    pub provenance: MemoryProvenance,
}

/// Explicit principal identity used by typed durable-memory recall.
/// A session identifier is deliberately not a substitute for either field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedRecallIdentity {
    pub persona_id: String,
    pub subject_id: String,
}

/// Provider-neutral source of typed durable-memory candidates.
/// Implementations return candidates for the coordinator's existing ranking path.
pub trait TypedMemoryRecallSource: Send + Sync {
    fn candidates(
        &self,
        query: &crate::layers::MemoryRecallQuery,
        identity: &TypedRecallIdentity,
        now_ms: i64,
    ) -> Result<Vec<MemoryCandidate>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Embedding adapter boundary. Memory never constructs HTTP clients or reads
/// credentials; Assembly injects an implementation when semantic recall is
/// available.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    fn model_id(&self) -> &str {
        "unknown"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbeddingError {
    #[error("embedding provider unavailable: {0}")]
    Unavailable(String),
    #[error("embedding provider returned invalid vector: {0}")]
    InvalidVector(String),
}

/// Explicit no-op semantic provider. Lexical retrieval remains available.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for NoEmbeddingProvider {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::Unavailable("not configured".to_string()))
    }
}

/// Optional second-stage reranking adapter.
#[async_trait]
pub trait MemoryReranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<MemoryCandidate>,
        token_budget: usize,
    ) -> Vec<MemoryCandidate>;
}

/// Deterministic default reranker; it preserves the already computed order.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicReranker;

#[async_trait]
impl MemoryReranker for DeterministicReranker {
    async fn rerank(
        &self,
        _query: &str,
        candidates: Vec<MemoryCandidate>,
        _token_budget: usize,
    ) -> Vec<MemoryCandidate> {
        candidates
    }
}

/// Persona profile kept separate from generic memory records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaMemoryProfile {
    pub persona_id: String,
    pub subject_id: String,
    pub portrait: String,
    pub traits: Vec<String>,
    pub known_facts: Vec<String>,
    pub shared_experiences: Vec<String>,
    pub revision: u64,
    pub provenance: MemoryProvenance,
    pub updated_at: Timestamp,
}

/// Optimistic, provenance-bearing profile patch. It never replaces the whole
/// profile supplied by a model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersonaProfileDelta {
    pub portrait_replace: Option<String>,
    pub traits_add: Vec<String>,
    pub traits_remove: Vec<String>,
    pub facts_add: Vec<String>,
    pub facts_update: Vec<(String, String)>,
    pub experiences_add: Vec<String>,
    pub provenance: MemoryProvenance,
}

/// Persistence boundary for persona profiles. Implementations may be backed
/// by SQLite or another governed store; the trait never accepts a raw model
/// replacement without a revision check.
#[async_trait]
pub trait PersonaProfileStore: Send + Sync {
    async fn get_profile(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Option<PersonaMemoryProfile>, String>;
    async fn apply_delta(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String>;
}

/// Process-local profile store used for deterministic tests and embeddings.
/// Production callers should inject a durable implementation.
#[derive(Debug, Default)]
pub struct InMemoryPersonaProfileStore {
    profiles: Mutex<BTreeMap<(String, String), PersonaMemoryProfile>>,
}

#[async_trait]
impl PersonaProfileStore for InMemoryPersonaProfileStore {
    async fn get_profile(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Option<PersonaMemoryProfile>, String> {
        Ok(self
            .profiles
            .lock()
            .map_err(|_| "persona profile store poisoned".to_string())?
            .get(&(persona_id.to_string(), subject_id.to_string()))
            .cloned())
    }

    async fn apply_delta(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String> {
        let mut profiles = self
            .profiles
            .lock()
            .map_err(|_| "persona profile store poisoned".to_string())?;
        let key = (persona_id.to_string(), subject_id.to_string());
        let profile = profiles.entry(key).or_insert_with(|| PersonaMemoryProfile {
            persona_id: persona_id.to_string(),
            subject_id: subject_id.to_string(),
            portrait: String::new(),
            traits: Vec::new(),
            known_facts: Vec::new(),
            shared_experiences: Vec::new(),
            revision: 0,
            provenance: MemoryProvenance::default(),
            updated_at,
        });
        profile.apply_delta(&delta, expected_revision, updated_at)?;
        Ok(profile.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_includes_graph_and_novelty_components() {
        let components = ScoreComponents {
            graph: 2.0,
            novelty: 3.0,
            ..ScoreComponents::default()
        };
        let config = MemoryRankingConfig {
            graph_weight: 0.25,
            novelty_weight: 0.50,
            ..MemoryRankingConfig::default()
        };

        assert!((components.weighted(&config) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recall_policy_defaults_are_bounded_and_structured() {
        let policy = RecallPolicy::default();

        assert!(policy.visible_scopes.is_empty());
        assert_eq!(policy.layers.len(), 4);
        assert_eq!(policy.limit, 8);
        assert_eq!(policy.max_chars, 4_000);
        assert_eq!(policy.proactive_budget, 2);
        assert_eq!(policy.min_score, 0.10);
        assert_eq!(policy.as_of_ms, None);
        assert_eq!(policy.token_budget, 1_024);
    }

    #[test]
    fn recall_policy_builders_clamp_hard_budgets() {
        let policy = RecallPolicy::default()
            .with_limit(0)
            .with_max_chars(0)
            .with_token_budget(0)
            .with_min_score(0.4)
            .with_as_of_ms(123);

        assert_eq!(policy.limit, 1);
        assert_eq!(policy.max_chars, 128);
        assert_eq!(policy.token_budget, 1);
        assert_eq!(policy.min_score, 0.4);
        assert_eq!(policy.as_of_ms, Some(123));
    }
}

impl PersonaMemoryProfile {
    /// Apply a delta only when its expected revision still matches.
    pub fn apply_delta(
        &mut self,
        delta: &PersonaProfileDelta,
        expected_revision: u64,
        updated_at: Timestamp,
    ) -> Result<(), String> {
        if self.revision != expected_revision {
            return Err(format!(
                "persona profile revision conflict: expected {expected_revision}, current {}",
                self.revision
            ));
        }
        if let Some(portrait) = &delta.portrait_replace {
            self.portrait = portrait.clone();
        }
        for trait_name in &delta.traits_remove {
            self.traits.retain(|value| value != trait_name);
        }
        for trait_name in &delta.traits_add {
            if !self.traits.contains(trait_name) {
                self.traits.push(trait_name.clone());
            }
        }
        for fact in &delta.facts_add {
            if !self.known_facts.contains(fact) {
                self.known_facts.push(fact.clone());
            }
        }
        for (old, new) in &delta.facts_update {
            if let Some(fact) = self.known_facts.iter_mut().find(|fact| *fact == old) {
                *fact = new.clone();
            } else if !self.known_facts.contains(new) {
                self.known_facts.push(new.clone());
            }
        }
        for experience in &delta.experiences_add {
            if !self.shared_experiences.contains(experience) {
                self.shared_experiences.push(experience.clone());
            }
        }
        self.revision = self.revision.saturating_add(1);
        self.provenance = delta.provenance.clone();
        self.updated_at = updated_at;
        Ok(())
    }
}
