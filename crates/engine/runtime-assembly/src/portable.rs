//! Platform-neutral assembly over the same managed AgentRuntime and kernel.
//! Hosts inject providers, module definitions, permissions and event sinks through
//! RuntimeBuilder. No business capability is registered implicitly.
use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::agents::{
    AgentConfig, AgentError, AgentInstance, AgentReply, AgentSnapshot, AgentUsage,
};
use apeireth_protocol::canonical::ToolEventV1;
use apeireth_runtime::canonical::profiles::ProfileConfig;
use apeireth_runtime::{
    AgentHostPolicy, AgentRuntime, RuntimeBuilder, RuntimeError, SessionEventKind,
};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum AssemblyError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Agent(#[from] AgentError),
}
struct MainAgent {
    agent: Arc<dyn AgentInstance>,
    tools: Vec<CapabilityId>,
}
/// One main role at a time. Module-owned agents remain managed by AgentRuntime.
pub struct PortableHost {
    runtime: AgentRuntime,
    main: tokio::sync::RwLock<MainAgent>,
    switching: tokio::sync::Mutex<()>,
}
impl PortableHost {
    /// Register the host's selected providers/modules on builder before assembly.
    pub async fn assemble(
        builder: RuntimeBuilder,
        policy: AgentHostPolicy,
        config: AgentConfig,
    ) -> Result<Self, AssemblyError> {
        let runtime = builder.build_agent_runtime(policy).await?;
        let tools = config.tools.clone();
        let agent = match runtime.spawn_agent("main", config).await {
            Ok(agent) => agent,
            Err(error) => {
                let _ = runtime.shutdown().await;
                return Err(error.into());
            }
        };
        Ok(Self {
            runtime,
            main: tokio::sync::RwLock::new(MainAgent { agent, tools }),
            switching: tokio::sync::Mutex::new(()),
        })
    }
    pub fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }
    pub async fn snapshot(&self) -> AgentSnapshot {
        self.main.read().await.agent.snapshot()
    }
    /// This is the explicit main-instance scope, not all registered tools.
    pub async fn tool_scope(&self) -> Vec<CapabilityId> {
        self.main.read().await.tools.clone()
    }
    pub async fn submit(&self, message: impl Into<String>) -> Result<AgentReply, AssemblyError> {
        let agent = self.main.read().await.agent.clone();
        Ok(agent.run(message.into()).await?)
    }
    /// Cancellation stops this main instance. Bind a profile to create a fresh one.
    pub async fn cancel(&self) -> Result<(), AssemblyError> {
        let agent = self.main.read().await.agent.clone();
        Ok(agent.stop().await?)
    }
    pub async fn bind_profile(
        &self,
        profile: &ProfileConfig,
    ) -> Result<AgentSnapshot, AssemblyError> {
        let config = profile.agent_config()?;
        let _switch = self.switching.lock().await;
        let old = self.main.read().await.agent.clone();
        old.stop().await?;
        old.destroy().await?;
        let tools = config.tools.clone();
        let agent = self
            .runtime
            .spawn_agent(format!("profile:{}", profile.id), config)
            .await?;
        let snapshot = agent.snapshot();
        *self.main.write().await = MainAgent { agent, tools };
        Ok(snapshot)
    }
    /// Durable V1 observations for the current main session, including audit-only facts.
    pub async fn tool_events(&self) -> Result<Vec<ToolEventV1>, AssemblyError> {
        let session = self.snapshot().await.session;
        let kernel = self.runtime.kernel().await;
        let stored = kernel.sessions().load(&session).await?;
        Ok(stored
            .into_iter()
            .flat_map(|session| session.events)
            .filter_map(|entry| match entry.event {
                SessionEventKind::ToolEventRecorded { fact } => Some(fact),
                _ => None,
            })
            .collect())
    }
    pub fn usage(&self) -> AgentUsage {
        self.runtime.usage(None)
    }
    pub async fn shutdown(&self) -> Result<(), AssemblyError> {
        Ok(self.runtime.shutdown().await?)
    }
}
