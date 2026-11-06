//! Single-main profile switching is serialized separately from model execution.
use super::*;
use crate::canonical::profiles::ProfileConfig;
impl AgentRuntime {
    pub async fn activate_profile(
        &self,
        profile: &ProfileConfig,
    ) -> AgentResult<Arc<dyn AgentInstance>> {
        let config = profile.agent_config()?;
        let mut active = self.inner.profile_switch.lock().await;
        if let Some(previous) = active.as_ref() {
            previous.stop().await?;
            previous.destroy().await?;
        }
        // Clear before creation: a failed activation never leaves two main roles.
        *active = None;
        let agent = self
            .spawn_agent(format!("profile:{}", profile.id), config)
            .await?;
        *active = Some(agent.clone());
        Ok(agent)
    }
    pub async fn deactivate_profile(&self) -> AgentResult<()> {
        let mut active = self.inner.profile_switch.lock().await;
        if let Some(previous) = active.as_ref() {
            previous.destroy().await?;
        }
        *active = None;
        Ok(())
    }
}
