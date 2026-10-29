//! Platform-neutral, Host-authorized configuration. Loading never activates an Agent.
use apeireth_core::kernel::{NamespaceId, ProfileId};
use apeireth_plugin::agents::{
    AgentConfig, AgentError, AgentErrorCode, AgentPrincipal, AgentResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub id: ProfileId,
    pub namespace: NamespaceId,
    pub label: String,
    pub agent: AgentConfig,
}
impl ProfileConfig {
    pub fn principal(&self) -> AgentPrincipal {
        AgentPrincipal {
            profile: self.id,
            namespace: self.namespace,
        }
    }
    /// Namespace and owner fields cannot be supplied by a model or inherited parent.
    pub fn agent_config(&self) -> AgentResult<AgentConfig> {
        if self.label.len() > 256
            || self.agent.parent.is_some()
            || self
                .agent
                .principal
                .as_ref()
                .is_some_and(|p| p != &self.principal())
        {
            return Err(AgentError::new(
                AgentErrorCode::InvalidConfig,
                "invalid profile binding",
            ));
        }
        let mut agent = self.agent.clone();
        agent.principal = Some(self.principal());
        Ok(agent)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRecord {
    pub config: ProfileConfig,
    pub revision: u64,
}
/// Host storage port. `None` inserts; `Some(revision)` updates using CAS.
#[async_trait]
pub trait ProfileStore: Send + Sync {
    async fn load(&self, id: ProfileId) -> AgentResult<Option<ProfileRecord>>;
    async fn save(
        &self,
        config: ProfileConfig,
        expected: Option<u64>,
    ) -> AgentResult<ProfileRecord>;
    async fn list(&self) -> AgentResult<Vec<ProfileRecord>>;
}
