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
        // `rt.module_messages`: 桌面 Host **显式武装**受限模块消息管道（2026-11-07 接线）。
        //
        // 缺口（补前）：本组合根从不设置 `message_policy` ⇒ 模块间消息端口恒为
        // `Disabled` ⇒ ⇒ 「owner 认证 + 有界」的那条管道在桌面产品里**根本不存在**
        // （只有测试在装它）。**没动** `production.rs` / CLI 组合根 ⇒ 装配面零尺寸增长。
        //
        // ⭐ 零行为变化（这是本条能默认开的前提）：`ModuleMessagePolicy::default()`
        // 只有**有界限额**（in_flight 16 / payload 64 KiB / hops 16 / 幂等 128 / 5s），
        // **零授权**（`grants` 空）⇒ ⇒ 没有任何模块被授权发/收 ⇒ **零流量、零副作用**。
        // 授权仍须 Host 显式加 `ModuleMessageGrant`（投递不授予权限）。
        //
        // ⭐ 认证语义仍是「owner 认证」：`AgentHostPolicy.modules` 的 grant 与本管道
        // grant 是**两张表**；管道只按 `message_policy.grants` 放行。
        policy.message_policy =
            Some(apeireth_runtime::canonical::module_messages::ModuleMessagePolicy::default());
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
