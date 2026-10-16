use std::sync::Arc;

use apeireth_memory::MemoryGovernanceStore;
use apeireth_memory::PersonaProfileStore;
use apeireth_memory::{
    MemoryCandidate, MemoryLayerKind, MemoryProvenance, MemoryRecallQuery, MemoryScope,
    ScoreComponents, SqliteCommitmentStore, SqliteMemoryStore, SqlitePersonaProfileStore,
    SqliteTemporalGraphStore, TemporalGraphQuery, TypedMemoryRecallSource, TypedRecallIdentity,
};
use apeireth_plugin::memory_backend::MemoryBackend;

/// Assembly adapter that exposes existing typed SQLite stores to the unified
/// coordinator ranking path. It owns no persistence and never infers identity.
#[derive(Clone)]
pub struct SqliteTypedMemoryRecallSource {
    pub episodes: Option<Arc<SqliteMemoryStore>>,
    pub commitments: Option<Arc<SqliteCommitmentStore>>,
    pub persona: Option<Arc<SqlitePersonaProfileStore>>,
    pub relations: Option<Arc<SqliteTemporalGraphStore>>,
    pub profile: Option<apeireth_memory::PersonaMemoryProfile>,
}

impl SqliteTypedMemoryRecallSource {
    pub fn new() -> Self {
        Self {
            episodes: None,
            commitments: None,
            persona: None,
            relations: None,
            profile: None,
        }
    }
    pub fn with_episodes(mut self, store: Arc<SqliteMemoryStore>) -> Self {
        self.episodes = Some(store);
        self
    }
    pub fn with_commitments(mut self, store: Arc<SqliteCommitmentStore>) -> Self {
        self.commitments = Some(store);
        self
    }
    pub fn with_persona(mut self, store: Arc<SqlitePersonaProfileStore>) -> Self {
        self.persona = Some(store);
        self
    }
    pub fn with_profile(mut self, profile: apeireth_memory::PersonaMemoryProfile) -> Self {
        self.profile = Some(profile);
        self
    }
    pub fn with_relations(mut self, store: Arc<SqliteTemporalGraphStore>) -> Self {
        self.relations = Some(store);
        self
    }
}
impl Default for SqliteTypedMemoryRecallSource {
    fn default() -> Self {
        Self::new()
    }
}

fn scope_from_json(value: Option<&str>, identity: &TypedRecallIdentity) -> MemoryScope {
    value
        .and_then(|raw| serde_json::from_str::<MemoryScope>(raw).ok())
        .unwrap_or_else(|| MemoryScope::User {
            user_id: identity.subject_id.clone(),
        })
}
fn provenance(source: &str, source_ref: Option<String>) -> MemoryProvenance {
    MemoryProvenance {
        source: source.into(),
        source_request: source_ref,
        ..Default::default()
    }
}

impl TypedMemoryRecallSource for SqliteTypedMemoryRecallSource {
    fn candidates(
        &self,
        query: &MemoryRecallQuery,
        identity: &TypedRecallIdentity,
        now_ms: i64,
    ) -> Result<Vec<MemoryCandidate>, Box<dyn std::error::Error + Send + Sync>> {
        let mut out = Vec::new();
        if let Some(store) = &self.episodes {
            for governed in store
                .governed_recent_episodes(&query.session_id, query.limit.saturating_mul(4).max(8))?
                .into_iter()
                .filter(|item| item.status.as_str() != "forgotten")
            {
                let episode = governed.episode;
                out.push(MemoryCandidate {
                    id: format!("typed:episode:{}", episode.id),
                    layer: "episodic".into(),
                    scope: MemoryScope::Session {
                        session_id: episode.session_id.clone(),
                    },
                    content: episode.content,
                    score: 0.0,
                    score_components: ScoreComponents {
                        recency: 1.0,
                        importance: 0.5,
                        confidence: 0.5,
                        ..Default::default()
                    },
                    provenance: provenance("typed_episode", Some(episode.id)),
                });
            }
        }
        if query.layers.contains(&MemoryLayerKind::Semantic) {
            if let Some(store) = &self.commitments {
                for item in store.list_active_for_subject(
                    &identity.subject_id,
                    None,
                    query.as_of_ms.or(Some(now_ms)),
                )? {
                    out.push(MemoryCandidate {
                        id: format!("typed:commitment:{}", item.id),
                        layer: "semantic".into(),
                        scope: scope_from_json(item.scope.as_deref(), identity),
                        content: format!("Active commitment: {}", item.text),
                        score: 0.0,
                        score_components: ScoreComponents {
                            importance: item.confidence,
                            confidence: item.confidence,
                            recency: 1.0,
                            ..Default::default()
                        },
                        provenance: provenance("typed_commitment", item.source_episode),
                    });
                }
            }
            if let Some(profile) = &self.profile {
                let mut parts = Vec::new();
                if !profile.portrait.is_empty() {
                    parts.push(profile.portrait.clone());
                }
                parts.extend(profile.traits.iter().cloned());
                parts.extend(profile.known_facts.iter().cloned());
                parts.extend(profile.shared_experiences.iter().cloned());
                if !parts.is_empty()
                    && profile.persona_id == identity.persona_id
                    && profile.subject_id == identity.subject_id
                {
                    out.push(MemoryCandidate {
                        id: format!(
                            "typed:persona:{}:{}",
                            identity.persona_id, identity.subject_id
                        ),
                        layer: "semantic".into(),
                        scope: MemoryScope::Persona {
                            persona_id: identity.persona_id.clone(),
                            user_id: identity.subject_id.clone(),
                        },
                        content: format!("Persona profile: {}", parts.join("; ")),
                        score: 0.0,
                        score_components: ScoreComponents {
                            importance: 0.9,
                            confidence: 0.9,
                            recency: 1.0,
                            ..Default::default()
                        },
                        provenance: provenance("typed_persona", None),
                    });
                }
            }
        }
        if query.layers.contains(&MemoryLayerKind::Relational) {
            if let Some(store) = &self.relations {
                let facts = store.query(
                    &TemporalGraphQuery::new(query.as_of_ms.unwrap_or(now_ms))
                        .subject(&identity.subject_id)
                        .limit(query.limit.saturating_mul(4).max(8)),
                )?;
                for fact in facts {
                    out.push(MemoryCandidate {
                        id: format!("typed:relation:{}", fact.id),
                        layer: "relational".into(),
                        scope: serde_json::from_value(fact.scope.clone()).unwrap_or_else(|_| {
                            MemoryScope::User {
                                user_id: identity.subject_id.clone(),
                            }
                        }),
                        content: format!(
                            "{} {} {}",
                            fact.subject_id, fact.predicate, fact.object_id
                        ),
                        score: 0.0,
                        score_components: ScoreComponents {
                            importance: fact.confidence,
                            confidence: fact.confidence,
                            recency: 1.0,
                            graph: 1.0,
                            ..Default::default()
                        },
                        provenance: provenance("typed_temporal_graph", fact.source_episode_id),
                    });
                }
            }
        }
        Ok(out)
    }
}
