//! Explicit desktop Host grants and the default managed main composition.
use apeireth_plugin::agents::{AgentBudget, AgentConfig};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, Runtime, RuntimeBuilder};

impl super::ProductionModules {
    /// Select the existing desktop catalog for module tasks. Governance and the
    /// parent session scope still apply; registering a module does not call this.
    pub fn desktop_agent_policy(&self) -> Result<AgentHostPolicy, String> {
        let mut tools = self
            .capabilities()
            .iter()
            .map(|tool| tool.id().clone())
            .collect::<std::collections::BTreeSet<_>>();
        for module in self.modules() {
            tools.insert(
                apeireth_runtime::canonical::behavior_adapter::registered_behavior_capability(
                    &module.manifest().id,
                )?,
            );
        }
        let mut policy = AgentHostPolicy::default();
        for module in self.modules() {
            let definition =
                apeireth_runtime::canonical::behavior_adapter::registered_behavior_owner(
                    &module.manifest().id,
                )?;
            policy.modules.insert(
                definition,
                ModuleAgentGrant {
                    tools: tools.clone(),
                    max_instances: policy.max_instances,
                    max_depth: apeireth_runtime::canonical::module::DEFAULT_MAX_INVOCATION_DEPTH,
                    budget: policy.max_budget.clone(),
                    ..Default::default()
                },
            );
        }
        // Desktop already has explicit per-session quota settings. Preserve its
        // lifetime semantics rather than imposing a small portable task budget.
        policy.max_budget = desktop_main_budget();
        Ok(policy)
    }
}
fn desktop_main_budget() -> AgentBudget {
    AgentBudget {
        max_requests: u64::MAX,
        max_tokens: u64::MAX,
        max_tool_calls: u64::MAX,
        max_cost_micros: None,
        turn_timeout_ms: u64::MAX,
    }
}
/// Same registry, providers, sessions and loop; all rich entries delegate to main.
/// Private Profile mappings remain an explicit, independent Host choice.
pub async fn build_desktop_main_runtime(
    builder: RuntimeBuilder,
    policy: AgentHostPolicy,
) -> Result<Runtime, apeireth_runtime::RuntimeError> {
    let budget = policy.max_budget.clone();
    let host = builder.build_agent_runtime(policy).await?;
    let tools = {
        let kernel = host.kernel().await;
        kernel
            .capability_registry()
            .capabilities()
            .iter()
            .map(|tool| tool.id().clone())
            .chain(
                kernel
                    .capability_registry()
                    .controls()
                    .iter()
                    .map(|tool| tool.id().clone()),
            )
            .collect()
    };
    host.into_main_runtime_with_session_quotas(AgentConfig {
        tools,
        budget,
        ..Default::default()
    })
    .await
}
