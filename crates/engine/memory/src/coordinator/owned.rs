//! Stable-id additive persistence for module-owned, explicitly scoped memory.
use super::*;
impl MemoryCoordinator {
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
