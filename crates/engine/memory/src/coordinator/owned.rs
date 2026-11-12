//! Stable-id additive persistence for module-owned, explicitly scoped memory.
use super::*;
impl MemoryCoordinator {
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
