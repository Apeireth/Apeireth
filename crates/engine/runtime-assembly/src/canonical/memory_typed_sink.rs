//! Canonical adapters for typed memory projections.
//!
//! This boundary deliberately requires explicit identity for principal-scoped writes. Missing
//! stores or identity produce `Skipped`, never an apparent successful persistence.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::Timestamp;
use apeireth_memory::{
    Commitment, CommitmentCandidate, CommitmentKind, CommitmentSignal, CommitmentStatus,
    MemoryError, MemoryMaterializationOutcome, MemoryProvenance, MemoryScope,
    MemoryTypedMaterializationSink, PersonaProfileDelta, PersonaProfileStore, RelationCandidate,
    SqliteCommitmentStore, SqliteTemporalGraphStore, TemporalGraphFact,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

/// Concrete production sink for commitment, persona, and temporal-relation projections.
#[derive(Clone, Default)]
pub struct CanonicalMemoryTypedSink {
    pub commitments: Option<Arc<SqliteCommitmentStore>>,
    pub persona: Option<Arc<dyn PersonaProfileStore>>,
    pub relations: Option<Arc<SqliteTemporalGraphStore>>,
    /// Explicit persona and subject identity. Persona writes are skipped when either is absent.
    pub persona_id: Option<String>,
    pub subject_id: Option<String>,
}

impl CanonicalMemoryTypedSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_commitments(mut self, store: Arc<SqliteCommitmentStore>) -> Self {
        self.commitments = Some(store);
        self
    }
    pub fn with_persona_store(mut self, store: Arc<dyn PersonaProfileStore>) -> Self {
        self.persona = Some(store);
        self
    }
    pub fn with_relations(mut self, store: Arc<SqliteTemporalGraphStore>) -> Self {
        self.relations = Some(store);
        self
    }
    pub fn with_identity(
        mut self,
        persona_id: impl Into<String>,
        subject_id: impl Into<String>,
    ) -> Self {
        self.persona_id = Some(persona_id.into());
        self.subject_id = Some(subject_id.into());
        self
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
fn error(error: impl ToString) -> MemoryError {
    MemoryError::Other(error.to_string())
}
fn stable_id(prefix: &str, values: &[&str]) -> String {
    let mut h = Sha256::new();
    h.update(prefix.as_bytes());
    for value in values {
        h.update((value.len() as u64).to_le_bytes());
        h.update(value.as_bytes());
    }
    format!("{prefix}-{:x}", h.finalize())
}
fn scope_json(scope: &MemoryScope) -> serde_json::Value {
    serde_json::to_value(scope).unwrap_or_else(|_| serde_json::json!({}))
}
fn provenance_json(value: &MemoryProvenance) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({}))
}

#[async_trait]
impl MemoryTypedMaterializationSink for CanonicalMemoryTypedSink {
    async fn materialize_commitment(
        &self,
        candidate: &CommitmentCandidate,
        signal: &CommitmentSignal,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        let Some(store) = &self.commitments else {
            return Ok(MemoryMaterializationOutcome::Skipped {
                reason: "commitment store is not configured".into(),
            });
        };
        // Commitments are principal-owned; scope is not an identity substitute.
        let subject = self
            .subject_id
            .as_deref()
            .or_else(|| match &candidate.scope {
                MemoryScope::User { user_id } => Some(user_id.as_str()),
                MemoryScope::Persona { user_id, .. } => Some(user_id.as_str()),
                _ => None,
            });
        let Some(subject) = subject else {
            return Ok(MemoryMaterializationOutcome::Skipped {
                reason: "commitment identity is not explicit".into(),
            });
        };
        let at = now_ms();
        match signal {
            CommitmentSignal::Create => {
                let commitment = Commitment {
                    id: stable_id("memory-commitment", &[subject, &candidate.content]),
                    subject_id: subject.into(),
                    kind: CommitmentKind::Shared,
                    status: CommitmentStatus::Active,
                    text: candidate.content.clone(),
                    scope: Some(
                        serde_json::to_string(&scope_json(&candidate.scope)).map_err(error)?,
                    ),
                    provenance: Some(
                        serde_json::to_string(&provenance_json(&candidate.provenance))
                            .map_err(error)?,
                    ),
                    source_episode: candidate.provenance.source_session.clone(),
                    deadline: None,
                    confidence: candidate.confidence,
                    supersedes_id: None,
                    retracted_at_ms: None,
                    completed_at_ms: None,
                    created_at: at,
                    updated_at: at,
                    revision: 0,
                };
                if store.get(&commitment.id).map_err(error)?.is_some() {
                    return Ok(MemoryMaterializationOutcome::Skipped {
                        reason: "commitment already exists".into(),
                    });
                }
                store.create(commitment).await.map_err(error)?;
                Ok(MemoryMaterializationOutcome::Applied)
            }
            CommitmentSignal::Complete | CommitmentSignal::Cancel => {
                let active = store
                    .list_active_for_subject(subject, None, None)
                    .map_err(error)?;
                let Some(current) = active
                    .into_iter()
                    .find(|item| item.text == candidate.content)
                else {
                    return Ok(MemoryMaterializationOutcome::Skipped {
                        reason: "matching active commitment not found".into(),
                    });
                };
                let status = if matches!(signal, CommitmentSignal::Complete) {
                    CommitmentStatus::Completed
                } else {
                    CommitmentStatus::Cancelled
                };
                store
                    .transition_for_subject(
                        subject,
                        &current.id,
                        current.revision,
                        status,
                        at,
                        Some("typed memory materialization"),
                    )
                    .await
                    .map_err(error)?;
                Ok(MemoryMaterializationOutcome::Applied)
            }
        }
    }

    async fn materialize_persona(
        &self,
        delta: &PersonaProfileDelta,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        let (Some(store), Some(persona_id), Some(subject_id)) = (
            &self.persona,
            self.persona_id.as_deref(),
            self.subject_id.as_deref(),
        ) else {
            return Ok(MemoryMaterializationOutcome::Skipped {
                reason: "persona store or explicit identity is not configured".into(),
            });
        };
        let revision = store
            .get_profile(persona_id, subject_id)
            .await
            .map_err(error)?
            .map_or(0, |p| p.revision);
        let timestamp = Timestamp::from_epoch_millis(now_ms())
            .ok_or_else(|| error("invalid materialization timestamp"))?;
        store
            .apply_delta(persona_id, subject_id, revision, delta.clone(), timestamp)
            .await
            .map_err(error)?;
        Ok(MemoryMaterializationOutcome::Applied)
    }

    async fn materialize_relation(
        &self,
        candidate: &RelationCandidate,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        let Some(store) = &self.relations else {
            return Ok(MemoryMaterializationOutcome::Skipped {
                reason: "relation store is not configured".into(),
            });
        };
        if candidate.subject_id.is_empty()
            || candidate.predicate.is_empty()
            || candidate.object_id.is_empty()
        {
            return Ok(MemoryMaterializationOutcome::Skipped {
                reason: "relation candidate fields are empty".into(),
            });
        }
        let at = now_ms();
        let relation_key = stable_id(
            "memory-relation",
            &[&candidate.subject_id, &candidate.predicate],
        );
        let fact = TemporalGraphFact {
            id: stable_id(
                "memory-fact",
                &[&relation_key, &candidate.object_id, &candidate.content],
            ),
            relation_key,
            subject_id: candidate.subject_id.clone(),
            predicate: candidate.predicate.clone(),
            object_id: candidate.object_id.clone(),
            valid_from_ms: at,
            valid_until_ms: None,
            believed_at_ms: at,
            source_episode_id: candidate.provenance.source_session.clone(),
            scope: scope_json(&candidate.scope),
            provenance: provenance_json(&candidate.provenance),
            confidence: candidate.confidence,
            revision: 0,
            supersedes_id: None,
            retracted_at_ms: None,
            created_at_ms: at,
        };
        store.append(fact).await.map_err(error)?;
        Ok(MemoryMaterializationOutcome::Applied)
    }
}
