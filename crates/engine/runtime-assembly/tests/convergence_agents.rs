#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_plugin::agents::*;
use apeireth_plugin::{
    CapabilityKind, FrozenInvocation, Plugin, PluginContext, PluginManifest, PluginResult,
    ToolCapability, ToolExecutionContext,
};
use apeireth_protocol::canonical::{ContentPart, NormalizedTool, ToolCall, ToolResult};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use support::*;

#[tokio::test]
async fn independent_agents_run_provider_requests_concurrently_and_keep_history_usage_scopes_separate(
) {
    let provider = Provider::new(vec![]);
    provider.synchronize(2);
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let first = runtime
        .spawn_agent("main", AgentConfig::default())
        .await
        .unwrap();
    let second = runtime
        .spawn_agent("background", AgentConfig::default())
        .await
        .unwrap();
    assert_ne!(first.id(), second.id());
    assert_ne!(first.snapshot().session, second.snapshot().session);
    let (a, b) = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            first.run("first_private".into()),
            second.run("second_private".into())
        )
    })
    .await
    .expect("provider requests must overlap");
    assert_eq!(a.unwrap().text, "done");
    assert_eq!(b.unwrap().text, "done");
    for req in provider.requests() {
        let text = req
            .messages
            .iter()
            .map(|m| ContentPart::join_text(&m.content))
            .collect::<String>();
        assert_ne!(
            text.contains("first_private"),
            text.contains("second_private")
        );
        assert!(req.tools.is_empty());
    }
    assert_eq!(first.snapshot().usage.input_tokens, 10);
    assert_eq!(second.snapshot().usage.input_tokens, 10);
    assert_eq!(first.snapshot().usage.cached_tokens, 3);
    assert_eq!(runtime.usage(None).requests, 2);
    assert_eq!(runtime.usage(None).input_tokens, 20);
    assert_eq!(runtime.usage(None).output_tokens, 10);
    assert_eq!(runtime.requests(first.id()).unwrap().len(), 1);
    assert_eq!(runtime.requests(first.id()).unwrap()[0].agent, first.id());
    first.pause().unwrap();
    assert_eq!(
        first.run("paused".into()).await.unwrap_err().code,
        AgentErrorCode::Paused
    );
    first.resume().unwrap();
    first.destroy().await.unwrap();
    assert_eq!(first.snapshot().status, AgentStatus::Destroyed);
    assert!(runtime
        .kernel()
        .await
        .sessions()
        .load(&first.snapshot().session)
        .await
        .unwrap()
        .is_none());
    assert_eq!(second.run("still_alive".into()).await.unwrap().text, "done");
    runtime.shutdown().await.unwrap();
    assert_eq!(second.snapshot().status, AgentStatus::Stopped);
}
#[tokio::test]
async fn cancelling_one_instance_does_not_shutdown_shared_provider_or_other_instance() {
    let provider = Provider::new(vec![]);
    let _hold = provider.hold_next();
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let blocked = runtime
        .spawn_agent("blocked", AgentConfig::default())
        .await
        .unwrap();
    let peer = runtime
        .spawn_agent("peer", AgentConfig::default())
        .await
        .unwrap();
    let running = blocked.clone();
    let task = tokio::spawn(async move { running.run("blocked".into()).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while provider.requests().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    blocked.stop().await.unwrap();
    assert_eq!(
        task.await.unwrap().unwrap_err().code,
        AgentErrorCode::Cancelled
    );
    assert_eq!(blocked.snapshot().usage.cancellations, 1);
    assert_eq!(blocked.snapshot().status, AgentStatus::Stopped);
    assert_eq!(peer.run("peer".into()).await.unwrap().text, "done");
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn budgets_refuse_a_new_provider_request_without_double_charging() {
    let provider = Provider::new(vec![]);
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "budget",
            AgentConfig {
                budget: AgentBudget {
                    max_requests: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("one".into()).await.unwrap();
    assert_eq!(
        agent.run("two".into()).await.unwrap_err().code,
        AgentErrorCode::BudgetExceeded
    );
    assert_eq!(provider.requests().len(), 1);
    assert_eq!(agent.snapshot().usage.requests, 1);
    assert_eq!(agent.snapshot().usage.input_tokens, 10);
    assert_eq!(runtime.requests(agent.id()).unwrap().len(), 1);
    runtime.shutdown().await.unwrap();
}
struct AgentModule {
    manifest: PluginManifest,
    tool: Arc<SpawnTool>,
}
struct SpawnTool {
    id: CapabilityId,
    agents: Mutex<Option<Arc<dyn ModuleAgents>>>,
}
#[async_trait]
impl ToolCapability for SpawnTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("spawn_child")
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::permanent_error(&call.id, "explicit execution context required")
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&FrozenInvocation>,
        ctx: &ToolExecutionContext,
    ) -> ToolResult {
        let agents = self.agents.lock().unwrap().clone().unwrap();
        let child = agents
            .spawn_agent(AgentConfig {
                parent: Some(ctx.agent),
                principal: ctx.principal.clone(),
                ..Default::default()
            })
            .await
            .unwrap();
        let result = child.run("child_private".into()).await.unwrap();
        ToolResult::ok(&call.id, json!({"agent":child.id(),"text":result.text}))
            .with_name(&call.name)
    }
}
#[async_trait]
impl Plugin for AgentModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
    fn uses_tool_events_v1(&self) -> bool {
        true
    }
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        *self.tool.agents.lock().unwrap() = ctx.agents.clone();
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
}
fn agent_module() -> Arc<AgentModule> {
    let tool = Arc::new(SpawnTool {
        id: CapabilityId::new("tool.spawn_child").unwrap(),
        agents: Mutex::new(None),
    });
    Arc::new(AgentModule {
        manifest: PluginManifest::new(pid("module.agents"), "1.0.0", "agent module")
            .declare_capability(tool.id.clone(), CapabilityKind::Tool, "child")
            .unwrap(),
        tool,
    })
}
#[tokio::test]
async fn a_module_creates_an_independent_real_loop_with_owned_usage_and_stop_propagation() {
    let module = agent_module();
    let provider = Provider::new(vec![vec![call("spawn_child", json!({}))], vec![], vec![]]);
    let mut policy = AgentHostPolicy::default();
    policy
        .modules
        .insert(pid("module.agents"), ModuleAgentGrant::default());
    let runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let main = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![CapabilityId::new("tool.spawn_child").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        main.run("parent_private".into()).await.unwrap().text,
        "done"
    );
    let child = runtime
        .instances()
        .into_iter()
        .find(|a| matches!(a.owner, AgentOwner::Module { .. }))
        .unwrap();
    assert_ne!(child.id, main.id());
    assert_ne!(child.session, main.snapshot().session);
    assert_eq!(child.usage.requests, 1);
    assert_eq!(main.snapshot().usage.requests, 2);
    assert_eq!(runtime.usage(None).requests, 3);
    let requests = provider.requests();
    let child_text = requests[1]
        .messages
        .iter()
        .map(|m| ContentPart::join_text(&m.content))
        .collect::<String>();
    assert!(child_text.contains("child_private"));
    assert!(!child_text.contains("parent_private"));
    assert!(requests[1].tools.is_empty());
    assert!(!facts(&*runtime.kernel().await, main.snapshot().session)
        .await
        .is_empty());
    let api = module.tool.agents.lock().unwrap().clone().unwrap();
    assert_eq!(
        api.spawn_agent(AgentConfig {
            parent: Some(child.id),
            ..Default::default()
        })
        .await
        .err()
        .unwrap()
        .code,
        AgentErrorCode::LimitExceeded
    );
    runtime
        .unregister_module(&pid("module.agents"))
        .await
        .unwrap();
    assert_eq!(
        runtime.snapshot(child.id).unwrap().status,
        AgentStatus::Stopped
    );
    assert_eq!(
        api.spawn_agent(AgentConfig::default())
            .await
            .err()
            .unwrap()
            .code,
        AgentErrorCode::OwnerStopped
    );
    assert_eq!(
        main.run("main_still_works".into()).await.unwrap().text,
        "done"
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn module_budget_survives_child_replacement_and_creation_does_not_grant_tools_or_namespaces()
{
    let module = agent_module();
    let provider = Provider::new(vec![]);
    let budget = AgentBudget {
        max_requests: 1,
        ..Default::default()
    };
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.agents"),
        ModuleAgentGrant {
            budget: budget.clone(),
            ..Default::default()
        },
    );
    let runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let api = module.tool.agents.lock().unwrap().clone().unwrap();
    assert_eq!(
        api.spawn_agent(AgentConfig {
            tools: vec![CapabilityId::new("tool.spawn_child").unwrap()],
            ..Default::default()
        })
        .await
        .err()
        .unwrap()
        .code,
        AgentErrorCode::ScopeDenied
    );
    assert_eq!(
        api.spawn_agent(AgentConfig {
            principal: Some(AgentPrincipal {
                profile: apeireth_core::kernel::ProfileId::new(),
                namespace: apeireth_core::kernel::NamespaceId::new()
            }),
            ..Default::default()
        })
        .await
        .err()
        .unwrap()
        .code,
        AgentErrorCode::NamespaceDenied
    );
    let first = api
        .spawn_agent(AgentConfig {
            budget: budget.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    first.run("first".into()).await.unwrap();
    first.stop().await.unwrap();
    let next = api
        .spawn_agent(AgentConfig {
            budget,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        next.run("second".into()).await.unwrap_err().code,
        AgentErrorCode::BudgetExceeded
    );
    assert_eq!(runtime.usage(Some(&first.snapshot().owner)).requests, 1);
    assert_eq!(provider.requests().len(), 1);
    runtime.shutdown().await.unwrap();
}
#[derive(Debug)]
struct Rates;
impl apeireth_runtime::CostRateTable for Rates {
    fn rate_for(&self, _: &str) -> Option<apeireth_runtime::CostRateMicros> {
        Some(apeireth_runtime::CostRateMicros {
            input_micros_per_1k_tokens: 1000,
            output_micros_per_1k_tokens: 2000,
        })
    }
}
#[tokio::test]
async fn estimated_cost_and_cached_usage_sum_once_and_missing_usage_remains_unknown() {
    let provider = Provider::new(vec![]);
    let runtime = builder(provider.clone())
        .with_cost_rates(Arc::new(Rates))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let a = runtime
        .spawn_agent("first", AgentConfig::default())
        .await
        .unwrap();
    let b = runtime
        .spawn_agent("second", AgentConfig::default())
        .await
        .unwrap();
    a.run("a".into()).await.unwrap();
    b.run("b".into()).await.unwrap();
    assert_eq!(a.snapshot().usage.estimated_cost_micros, 20);
    assert_eq!(runtime.usage(None).estimated_cost_micros, 40);
    assert_eq!(runtime.usage(None).cached_tokens, 6);
    assert_eq!(runtime.usage(None).unpriced_requests, 0);
    provider.set_usage(Default::default());
    let unknown = runtime
        .spawn_agent("unknown", AgentConfig::default())
        .await
        .unwrap();
    unknown.run("unknown".into()).await.unwrap();
    assert_eq!(unknown.snapshot().usage.unreported_usage_requests, 1);
    assert!(runtime.requests(unknown.id()).unwrap()[0]
        .reported
        .is_none());
    assert!(runtime.requests(unknown.id()).unwrap()[0]
        .estimated_cost_micros
        .is_none());
    assert_eq!(
        unknown.run("again".into()).await.unwrap_err().code,
        AgentErrorCode::BudgetExceeded
    );
    assert_eq!(provider.requests().len(), 3);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn configured_provider_limit_is_shared_without_serializing_the_runtime() {
    let provider = Provider::new(vec![]);
    let hold = provider.hold_next();
    let mut policy = AgentHostPolicy::default();
    policy.provider_concurrency = 1;
    let runtime = builder(provider.clone())
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let first = runtime
        .spawn_agent("first", AgentConfig::default())
        .await
        .unwrap();
    let second = runtime
        .spawn_agent("second", AgentConfig::default())
        .await
        .unwrap();
    let a = first.clone();
    let one = tokio::spawn(async move { a.run("one".into()).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while provider.requests().len() != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let b = second.clone();
    let two = tokio::spawn(async move { b.run("two".into()).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while second.snapshot().status != AgentStatus::Running {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(provider.requests().len(), 1);
    hold.notify_one();
    one.await.unwrap().unwrap();
    two.await.unwrap().unwrap();
    assert_eq!(provider.requests().len(), 2);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn dropping_the_last_host_handle_cancels_owned_agent_work() {
    let provider = Provider::new(vec![]);
    let _hold = provider.hold_next();
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent("drop", AgentConfig::default())
        .await
        .unwrap();
    let a = agent.clone();
    let task = tokio::spawn(async move { a.run("inflight".into()).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while provider.requests().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(runtime);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .code,
        AgentErrorCode::Cancelled
    );
    assert_eq!(agent.snapshot().status, AgentStatus::Stopped);
}

#[tokio::test]
async fn managed_minimal_kernel_records_v1_unregistered_result_without_installing_any_business_tool(
) {
    let provider = Provider::new(vec![
        vec![call("shell", json!({"command":"exit 99"}))],
        vec![],
    ]);
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent("minimal", AgentConfig::default())
        .await
        .unwrap();
    assert!(runtime.kernel().await.tools().is_empty());
    assert_eq!(agent.run("missing".into()).await.unwrap().text, "done");
    let rows = facts(&*runtime.kernel().await, agent.snapshot().session).await;
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].result().is_ok());
    assert!(provider
        .results()
        .iter()
        .any(|result| result.contains("unregistered")));
    assert_eq!(agent.snapshot().usage.tool_calls, 0);
    runtime.shutdown().await.unwrap();
}
#[cfg(target_os = "linux")]
#[tokio::test]
async fn agent_stop_waits_for_native_tool_cleanup_owned_by_both_agent_and_module() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("pid");
    let command = format!("echo $$ > '{}'; sleep 30", marker.display());
    let provider = Provider::new(vec![vec![call("shell", json!({"command":command}))]]);
    let shell = apeireth_runtime_assembly::ShellModule::new(
        apeireth_tools_canonical::TrustedShellConfig::new(root.path())
            .with_sandbox(false)
            .with_file_write(true),
    );
    let runtime = builder(provider)
        .with_tool_module(&shell)
        .unwrap()
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "native",
            AgentConfig {
                tools: vec![CapabilityId::new("tool.shell").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let a = agent.clone();
    let task = tokio::spawn(async move { a.run("native".into()).await });
    let process_id = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(value) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = value.trim().parse::<u32>() {
                    break pid;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    agent.stop().await.unwrap();
    assert_eq!(
        task.await.unwrap().unwrap_err().code,
        AgentErrorCode::Cancelled
    );
    assert_eq!(agent.snapshot().usage.tool_calls, 1);
    assert_eq!(agent.snapshot().usage.cancellations, 1);
    assert!(
        !std::path::Path::new(&format!("/proc/{process_id}")).exists(),
        "stop must reap the native leader before reporting stopped"
    );
    assert_eq!(
        runtime
            .kernel()
            .await
            .plugins()
            .task_scope(&pid("module.tool.shell"))
            .unwrap()
            .active_tasks(),
        0
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn provider_retry_after_is_respected_and_failed_usage_is_not_invented() {
    let provider = Provider::new(vec![]);
    provider.fail_next(apeireth_plugin::ProviderError::RateLimited {
        provider: "fixture".into(),
        retry_after_ms: 120,
    });
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent("retry", AgentConfig::default())
        .await
        .unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        agent.run("limited".into()).await.unwrap_err().code,
        AgentErrorCode::Provider
    );
    assert_eq!(agent.snapshot().usage.errors, 1);
    assert_eq!(agent.snapshot().usage.unreported_usage_requests, 1);
    assert!(agent.snapshot().usage.complete_token_count().is_none());
    assert!(agent.snapshot().usage.complete_cost_estimate().is_none());
    agent.resume().unwrap();
    agent.run("retry".into()).await.unwrap();
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert_eq!(provider.requests().len(), 2);
    assert_eq!(agent.snapshot().usage.requests, 2);
    assert_eq!(agent.snapshot().usage.input_tokens, 10);
    runtime.shutdown().await.unwrap();
}
struct AutonomousModule {
    manifest: PluginManifest,
    done: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl Plugin for AutonomousModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        let agents = ctx.agents.clone().expect("managed host port");
        let done = self.done.clone();
        let scope = ctx.tasks.clone();
        let lease = scope.lease().unwrap();
        tokio::spawn(async move {
            let _owner = lease;
            let _ = scope
                .run(async move {
                    agents.wait_ready().await.unwrap();
                    let child = agents.spawn_agent(AgentConfig::default()).await.unwrap();
                    child.run("autonomous_owned_start".into()).await.unwrap();
                    child.destroy().await.unwrap();
                    done.notify_one();
                })
                .await;
        });
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
}
#[tokio::test]
async fn a_module_starts_its_own_real_agent_after_assembly_without_a_host_task_or_tool_call() {
    let done = Arc::new(tokio::sync::Notify::new());
    let provider = Provider::new(vec![]);
    let module = Arc::new(AutonomousModule {
        manifest: PluginManifest::new(pid("module.autonomous"), "1.0.0", "autonomous owned agent"),
        done: done.clone(),
    });
    let mut policy = AgentHostPolicy::default();
    policy
        .modules
        .insert(pid("module.autonomous"), ModuleAgentGrant::default());
    let runtime = builder(provider.clone())
        .with_plugin(module)
        .build_agent_runtime(policy)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), done.notified())
        .await
        .unwrap();
    let instances = runtime.instances();
    assert_eq!(instances.len(), 1);
    assert!(
        matches!(&instances[0].owner,AgentOwner::Module{definition,..} if definition==&pid("module.autonomous"))
    );
    assert_eq!(instances[0].status, AgentStatus::Destroyed);
    assert_eq!(instances[0].usage.requests, 1);
    assert!(provider.requests()[0].tools.is_empty());
    runtime.shutdown().await.unwrap();
}
