use std::sync::Arc;

use crate::{
    MemoryGovernanceError, MemoryGovernanceStore, SqliteCommitmentStore, SqliteMemoryStore,
    SqlitePersonaProfileStore, SqliteTemporalGraphStore,
};
use apeireth_plugin::preference::PreferenceStore;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UniversalForgetOutcome {
    pub episode: bool,
    pub preference: bool,
    pub commitment: bool,
    pub temporal: bool,
    pub persona: bool,
}

/// Existing-store composition boundary for universal forget eligibility.
/// Append-only typed stores retain audit/history rows and expose terminal or
/// tombstone state to recall rather than being physically deleted.
#[derive(Clone)]
pub struct UniversalForgetFacade {
    pub episodes: Arc<SqliteMemoryStore>,
    pub preferences: Option<Arc<dyn PreferenceStore>>,
    pub commitments: Option<Arc<SqliteCommitmentStore>>,
    pub persona: Option<Arc<SqlitePersonaProfileStore>>,
    pub temporal: Option<Arc<SqliteTemporalGraphStore>>,
}

impl UniversalForgetFacade {
    pub fn new(episodes: Arc<SqliteMemoryStore>) -> Self {
        Self {
            episodes,
            preferences: None,
            commitments: None,
            persona: None,
            temporal: None,
        }
    }

    pub fn is_episode_forgotten(&self, id: &str) -> bool {
        self.episodes
            .get_governed(id)
            .ok()
            .flatten()
            .is_some_and(|state| state.status.as_str() == "forgotten")
    }

    pub fn forget_episode(
        &self,
        id: &str,
        reason: Option<&str>,
        revision: i64,
    ) -> Result<UniversalForgetOutcome, MemoryGovernanceError> {
        self.episodes.forget_episode(id, reason, revision)?;
        Ok(UniversalForgetOutcome {
            episode: true,
            ..Default::default()
        })
    }
}
