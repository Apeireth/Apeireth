//! Canonical adapters for typed memory projections.
//!
//! This boundary deliberately requires explicit identity for principal-scoped writes. Missing
//! stores or identity produce `Skipped`, never an apparent successful persistence.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::Timestamp;
use apeireth_memory::{
    Commitment, CommitmentCandidate, CommitmentKind, CommitmentSignal, CommitmentStatus,
    MemoryError, MemoryMaterializationOutcome, MemoryProvenance, MemoryScope, MemoryTypedCandidate,
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
    if let Ok(at) = RECOVERY_OPERATION.try_with(|op| op.at_ms) {
        return at;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

struct RecoveryInvocation {
    id: String,
    at_ms: i64,
    context: serde_json::Value,
}
tokio::task_local! { static RECOVERY_OPERATION: RecoveryInvocation; }
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

fn source_episode(fallback: Option<String>) -> Option<String> {
    RECOVERY_OPERATION
        .try_with(|op| op.context["source_episode_id"].as_str().map(str::to_owned))
        .ok()
        .flatten()
        .or(fallback)
}

#[async_trait]
impl MemoryTypedMaterializationSink for CanonicalMemoryTypedSink {
    fn supports_recovery_journal(
        &self,
        journal: &apeireth_memory::writeback_recovery::SqliteWritebackJournal,
    ) -> bool {
        self.commitments
            .as_ref()
            .is_none_or(|store| journal.shares_pool(store.pool()))
            && self
                .relations
                .as_ref()
                .is_none_or(|store| journal.shares_pool(store.pool()))
            && self
                .persona
                .as_ref()
                .is_none_or(|store| store.supports_recovery_journal(journal))
    }
    fn supports_durable_recovery(&self) -> bool {
        true
    }

    async fn prepare_recovery(
        &self,
        candidate: &MemoryTypedCandidate,
    ) -> Result<serde_json::Value, MemoryError> {
        match candidate {
            MemoryTypedCandidate::Persona { .. } => {
                let (Some(store), Some(persona), Some(subject)) =
                    (&self.persona, &self.persona_id, &self.subject_id)
                else {
                    return Ok(serde_json::Value::Null);
                };
                let revision = store
                    .get_profile(persona, subject)
                    .await
                    .map_err(error)?
                    .map_or(0, |p| p.revision);
                Ok(serde_json::json!({"persona_revision":revision}))
            }
            MemoryTypedCandidate::Commitment { candidate, signal }
                if !matches!(signal, CommitmentSignal::Create) =>
            {
                let Some(store) = &self.commitments else {
                    return Ok(serde_json::Value::Null);
                };
                let subject =
                    self.subject_id
                        .as_deref()
                        .or_else(|| match &candidate.scope {
                            MemoryScope::User { user_id }
                            | MemoryScope::Persona { user_id, .. } => Some(user_id.as_str()),
                            _ => None,
                        });
                let Some(subject) = subject else {
                    return Ok(serde_json::Value::Null);
                };
                let active = store
                    .list_active_for_subject(subject, None, None)
                    .map_err(error)?;
                Ok(
                    match active.into_iter().find(|c| c.text == candidate.content) {
                        Some(c) => {
                            serde_json::json!({"commitment_id":c.id,"commitment_revision":c.revision})
                        }
                        None => serde_json::json!({"commitment_absent":true}),
                    },
                )
            }
            _ => Ok(serde_json::Value::Null),
        }
    }

    async fn materialize_recovery(
        &self,
        candidate: &MemoryTypedCandidate,
        operation_id: &str,
        at_ms: i64,
        frozen_context: &serde_json::Value,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        RECOVERY_OPERATION
            .scope(
                RecoveryInvocation {
                    id: operation_id.to_owned(),
                    at_ms,
                    context: frozen_context.clone(),
                },
                async {
                    match candidate {
                        MemoryTypedCandidate::Commitment { candidate, signal } => {
                            self.materialize_commitment(candidate, signal).await
                        }
                        MemoryTypedCandidate::Persona { delta } => {
                            self.materialize_persona(delta).await
                        }
                        MemoryTypedCandidate::Relation { candidate } => {
                            self.materialize_relation(candidate).await
                        }
                    }
                },
            )
            .await
    }
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
                    source_episode: source_episode(candidate.provenance.source_session.clone()),
                    deadline: None,
                    confidence: candidate.confidence,
                    supersedes_id: None,
                    retracted_at_ms: None,
                    completed_at_ms: None,
                    created_at: at,
                    updated_at: at,
                    revision: 0,
                };
                if let Some(existing) = store.get(&commitment.id).map_err(error)? {
                    if RECOVERY_OPERATION.try_with(|_| ()).is_ok()
                        && existing.scope != commitment.scope
                    {
                        return Err(error("commitment replay scope conflict"));
                    }
                    return Ok(MemoryMaterializationOutcome::Skipped {
                        reason: "commitment already exists".into(),
                    });
                }
                if let Ok(operation) = RECOVERY_OPERATION.try_with(|op| op.id.clone()) {
                    store
                        .create_recovery(&operation, commitment)
                        .await
                        .map_err(error)?;
                } else {
                    store.create(commitment).await.map_err(error)?;
                }
                Ok(MemoryMaterializationOutcome::Applied)
            }
            CommitmentSignal::Complete | CommitmentSignal::Cancel => {
                let status = if matches!(signal, CommitmentSignal::Complete) {
                    CommitmentStatus::Completed
                } else {
                    CommitmentStatus::Cancelled
                };
                let frozen = RECOVERY_OPERATION.try_with(|op| op.context.clone()).ok();
                if frozen
                    .as_ref()
                    .is_some_and(|c| c["commitment_absent"] == true)
                {
                    return Ok(MemoryMaterializationOutcome::Skipped {
                        reason: "matching active commitment was absent when planned".into(),
                    });
                }
                let active = store
                    .list_active_for_subject(subject, None, None)
                    .map_err(error)?;
                let current = if let Some(context) = frozen {
                    let id = context["commitment_id"]
                        .as_str()
                        .ok_or_else(|| error("frozen commitment identity missing"))?;
                    let current = store
                        .get(id)
                        .map_err(error)?
                        .ok_or_else(|| error("frozen commitment no longer exists"))?;
                    if current.subject_id != subject {
                        return Err(error("commitment subject conflict"));
                    }
                    if current.status == status {
                        return Ok(MemoryMaterializationOutcome::Skipped {
                            reason: "commitment transition already applied".into(),
                        });
                    }
                    if Some(current.revision) != context["commitment_revision"].as_i64() {
                        return Err(error("frozen commitment revision changed"));
                    }
                    Some(current)
                } else {
                    active
                        .into_iter()
                        .find(|item| item.text == candidate.content)
                };
                let Some(current) = current else {
                    return Ok(MemoryMaterializationOutcome::Skipped {
                        reason: "matching active commitment not found".into(),
                    });
                };
                if let Ok(operation) = RECOVERY_OPERATION.try_with(|op| op.id.clone()) {
                    store
                        .transition_recovery(&operation, &current, status, at)
                        .await
                        .map_err(error)?;
                } else {
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
                }
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
        let recovery = RECOVERY_OPERATION
            .try_with(|op| (op.id.clone(), op.context["persona_revision"].as_u64()))
            .ok();
        let timestamp = Timestamp::from_epoch_millis(now_ms())
            .ok_or_else(|| error("invalid materialization timestamp"))?;
        if let Some((id, revision)) = recovery {
            store
                .apply_delta_once(
                    &id,
                    persona_id,
                    subject_id,
                    revision.ok_or_else(|| error("frozen persona revision missing"))?,
                    delta.clone(),
                    timestamp,
                )
                .await
                .map_err(error)?;
        } else {
            store
                .apply_delta(persona_id, subject_id, revision, delta.clone(), timestamp)
                .await
                .map_err(error)?;
        }
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
        let values = [
            relation_key.as_str(),
            candidate.object_id.as_str(),
            candidate.content.as_str(),
        ];
        let id = match RECOVERY_OPERATION.try_with(|op| op.id.clone()).ok() {
            Some(operation) => stable_id(
                "memory-fact",
                &[values[0], values[1], values[2], &operation],
            ),
            None => stable_id("memory-fact", &values),
        };
        let fact = TemporalGraphFact {
            id,
            relation_key,
            subject_id: candidate.subject_id.clone(),
            predicate: candidate.predicate.clone(),
            object_id: candidate.object_id.clone(),
            valid_from_ms: at,
            valid_until_ms: None,
            believed_at_ms: at,
            source_episode_id: source_episode(candidate.provenance.source_session.clone()),
            scope: scope_json(&candidate.scope),
            provenance: provenance_json(&candidate.provenance),
            confidence: candidate.confidence,
            revision: 0,
            supersedes_id: None,
            retracted_at_ms: None,
            created_at_ms: at,
        };
        if let Ok(operation) = RECOVERY_OPERATION.try_with(|op| op.id.clone()) {
            store
                .append_recovery(&operation, fact)
                .await
                .map_err(error)?;
        } else {
            store.append(fact).await.map_err(error)?;
        }
        Ok(MemoryMaterializationOutcome::Applied)
    }
}

/// Rechecks the extraction boundary before a private typed store is touched.
pub(crate) struct ProfileTypedSink<'a> {
    pub inner: &'a dyn MemoryTypedMaterializationSink,
    pub scope: MemoryScope,
    pub provenance: MemoryProvenance,
    pub cancellation: apeireth_plugin::ToolCancellation,
}
impl ProfileTypedSink<'_> {
    fn check(&self, scope: &MemoryScope) -> Result<(), MemoryError> {
        if self.cancellation.is_cancelled() || scope != &self.scope {
            return Err(MemoryError::Invalid(
                "private typed materialization scope denied".into(),
            ));
        }
        Ok(())
    }

    fn recovery_candidate(
        &self,
        candidate: &MemoryTypedCandidate,
    ) -> Result<MemoryTypedCandidate, MemoryError> {
        let mut candidate = candidate.clone();
        match &mut candidate {
            MemoryTypedCandidate::Commitment { candidate, .. } => {
                self.check(&candidate.scope)?;
                candidate.provenance = self.provenance.clone();
            }
            MemoryTypedCandidate::Persona { delta } => {
                self.check(&self.scope)?;
                delta.provenance = self.provenance.clone();
            }
            MemoryTypedCandidate::Relation { candidate } => {
                self.check(&candidate.scope)?;
                candidate.provenance = self.provenance.clone();
            }
        }
        Ok(candidate)
    }
}
#[async_trait]
impl MemoryTypedMaterializationSink for ProfileTypedSink<'_> {
    fn supports_recovery_journal(
        &self,
        journal: &apeireth_memory::writeback_recovery::SqliteWritebackJournal,
    ) -> bool {
        self.inner.supports_recovery_journal(journal)
    }
    fn supports_durable_recovery(&self) -> bool {
        self.inner.supports_durable_recovery()
    }
    async fn prepare_recovery(
        &self,
        candidate: &MemoryTypedCandidate,
    ) -> Result<serde_json::Value, MemoryError> {
        self.inner
            .prepare_recovery(&self.recovery_candidate(candidate)?)
            .await
    }
    async fn materialize_recovery(
        &self,
        candidate: &MemoryTypedCandidate,
        operation_id: &str,
        at_ms: i64,
        context: &serde_json::Value,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        self.inner
            .materialize_recovery(
                &self.recovery_candidate(candidate)?,
                operation_id,
                at_ms,
                context,
            )
            .await
    }
    async fn materialize_commitment(
        &self,
        candidate: &CommitmentCandidate,
        signal: &CommitmentSignal,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        self.check(&candidate.scope)?;
        let mut candidate = candidate.clone();
        candidate.provenance = self.provenance.clone();
        self.inner.materialize_commitment(&candidate, signal).await
    }
    async fn materialize_persona(
        &self,
        delta: &PersonaProfileDelta,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        self.check(&self.scope)?;
        let mut delta = delta.clone();
        delta.provenance = self.provenance.clone();
        self.inner.materialize_persona(&delta).await
    }
    async fn materialize_relation(
        &self,
        candidate: &RelationCandidate,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        self.check(&candidate.scope)?;
        let mut candidate = candidate.clone();
        candidate.provenance = self.provenance.clone();
        self.inner.materialize_relation(&candidate).await
    }
}
