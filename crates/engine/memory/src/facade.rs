//! Minimal production-callable facade for durable memory mutations.
//!
//! This module intentionally contains no new persistence logic. It composes the
//! existing SQLite commitment and persona stores so callers can perform the
//! two mutations that need a small, stable boundary without wiring runtime
//! assembly or depending on store internals.

use apeireth_core::kernel::Timestamp;

use crate::commitments::{Commitment, CommitmentError, CommitmentStatus, SqliteCommitmentStore};
use crate::persona_store_sqlite::SqlitePersonaProfileStore;
use crate::scope::{PersonaMemoryProfile, PersonaProfileDelta, PersonaProfileStore};

/// Small durable mutation boundary for commitment candidates and persona deltas.
#[derive(Clone)]
pub struct MemoryMutationFacade {
    commitments: SqliteCommitmentStore,
    persona_profiles: SqlitePersonaProfileStore,
}

impl MemoryMutationFacade {
    /// Compose the facade from the existing durable stores.
    pub fn new(
        commitments: SqliteCommitmentStore,
        persona_profiles: SqlitePersonaProfileStore,
    ) -> Self {
        Self {
            commitments,
            persona_profiles,
        }
    }

    /// Create a candidate using the canonical commitment store.
    pub async fn create_commitment_candidate(
        &self,
        candidate: Commitment,
    ) -> Result<Commitment, CommitmentError> {
        self.commitments.create(candidate).await
    }

    /// Complete a candidate with subject-scoped optimistic concurrency.
    pub async fn complete_commitment_candidate(
        &self,
        subject_id: &str,
        commitment_id: &str,
        expected_revision: i64,
        completed_at_ms: i64,
        reason: Option<&str>,
    ) -> Result<Commitment, CommitmentError> {
        self.commitments
            .transition_for_subject(
                subject_id,
                commitment_id,
                expected_revision,
                CommitmentStatus::Completed,
                completed_at_ms,
                reason,
            )
            .await
    }

    /// Apply a persona delta only when `expected_revision` still matches.
    pub async fn apply_persona_delta_cas(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String> {
        self.persona_profiles
            .apply_delta(persona_id, subject_id, expected_revision, delta, updated_at)
            .await
    }

    pub fn commitments(&self) -> &SqliteCommitmentStore {
        &self.commitments
    }

    pub fn persona_profiles(&self) -> &SqlitePersonaProfileStore {
        &self.persona_profiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitments::{CommitmentKind, CommitmentStatus};
    use crate::scope::{MemoryProvenance, PersonaProfileStore};
    use apeireth_storage::SqliteConnectionPool;

    fn candidate() -> Commitment {
        Commitment {
            id: "candidate-1".into(),
            subject_id: "subject-1".into(),
            kind: CommitmentKind::Shared,
            status: CommitmentStatus::Active,
            text: "follow up".into(),
            scope: None,
            provenance: None,
            source_episode: None,
            deadline: None,
            confidence: 0.8,
            supersedes_id: None,
            retracted_at_ms: None,
            completed_at_ms: None,
            created_at: 10,
            updated_at: 10,
            revision: 0,
        }
    }

    #[tokio::test]
    async fn facade_composes_commitment_lifecycle_and_persona_cas() {
        let commitment_store = SqliteCommitmentStore::in_memory().await.unwrap();
        let persona_pool = SqliteConnectionPool::in_memory().await.unwrap();
        let persona_store = SqlitePersonaProfileStore::new(persona_pool);
        persona_store.ensure_schema().await.unwrap();
        let facade = MemoryMutationFacade::new(commitment_store, persona_store);

        let created = facade
            .create_commitment_candidate(candidate())
            .await
            .unwrap();
        assert_eq!(created.revision, 0);
        let completed = facade
            .complete_commitment_candidate("subject-1", "candidate-1", 0, 20, Some("done"))
            .await
            .unwrap();
        assert_eq!(completed.status, CommitmentStatus::Completed);
        assert_eq!(completed.revision, 1);
        assert!(matches!(
            facade
                .complete_commitment_candidate("subject-1", "candidate-1", 0, 21, None)
                .await,
            Err(CommitmentError::Cas { .. })
        ));

        let delta = PersonaProfileDelta {
            traits_add: vec!["careful".into()],
            provenance: MemoryProvenance::default(),
            ..PersonaProfileDelta::default()
        };
        let timestamp = Timestamp::from_epoch_millis(30).unwrap();
        let profile = facade
            .apply_persona_delta_cas("persona-1", "subject-1", 0, delta, timestamp)
            .await
            .unwrap();
        assert_eq!(profile.revision, 1);
        assert_eq!(profile.traits, vec!["careful"]);
        let conflict = facade
            .apply_persona_delta_cas(
                "persona-1",
                "subject-1",
                0,
                PersonaProfileDelta::default(),
                timestamp,
            )
            .await;
        assert!(conflict.is_err());
    }
}
