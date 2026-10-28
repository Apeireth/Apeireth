//! End-to-end coverage for the canonical cognitive-module ABI.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    AgentId, CapabilityId, ModelId, PluginId, RequestId, SessionId, TraceId,
};
use apeireth_governance::{
    Action, AllowAll, Decision, DenyCapabilities, GovernanceHook, GovernancePipeline,
    GovernanceRequest, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::agents::{AgentConfig, AgentOwner, AgentStatus, AgentTaskOrigin};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    MessageRole, ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedMessage,
    NormalizedRequest, NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall,
    ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::instances::{AgentHostPolicy, ModuleAgentGrant};
use apeireth_runtime::canonical::module::{ModuleTurnState, RuntimeModuleInvoker};
use apeireth_runtime::canonical::quota::{CostRateMicros, CostRateTable};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, BehaviorModule, HookPoint, InvocationOrigin,
    ModuleContext, ModuleError, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleManifest, ModuleOutcome, Runtime, RuntimeError,
    SessionEventKind, TurnOutcome, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

struct InvocationEvidence {
    manifest: ModuleManifest,
    requests: Vec<ModuleInvocationRequest>,
    results: Mutex<Vec<Result<ModuleInvocationResponse, ModuleInvocationError>>>,
    hooks: Mutex<Vec<HookPoint>>,
}
impl InvocationEvidence {
    fn new(requests: usize) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new("module.invoker_evidence", "invoker evidence"),
            requests: (0..requests)
                .map(|_| {
                    ModuleInvocationRequest::isolated("private system", "private task")
                        .with_model(MODEL)
                })
                .collect(),
            results: Mutex::new(vec![]),
            hooks: Mutex::new(vec![]),
        })
    }
    fn definition(&self) -> PluginId {
        PluginId::new(format!("compat.{}", self.manifest.id)).unwrap()
    }
    fn capability(&self) -> CapabilityId {
        apeireth_runtime::canonical::behavior_adapter::registered_behavior_capability(
            &self.manifest.id,
        )
        .unwrap()
    }
}
#[async_trait]
impl BehaviorModule for InvocationEvidence {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.hooks.lock().unwrap().push(hook);
        if hook == HookPoint::TurnStart {
            for request in &self.requests {
                let result = ctx.invoker_handle().invoke(request.clone()).await;
                self.results.lock().unwrap().push(result);
            }
        }
        Ok(ModuleOutcome::continue_())
    }
}

fn opaque_side_response() -> NormalizedResponse {
    let call = ToolCall {
        id: "duplicate-wire-id".into(),
        name: "echo".into(),
        arguments: serde_json::json!({}),
    };
    NormalizedResponse {
        id: "original-side-id".into(),
        model: "reported-model".into(),
        content: "private answer".into(),
        finish_reason: Some(NormalizedFinishReason::ToolCalls),
        usage: NormalizedUsage {
            prompt_tokens: 7,
            completion_tokens: 3,
            total_tokens: 10,
            prompt_cache_hit_tokens: Some(2),
        },
        // An opaque response is not an executable batch, even with duplicate IDs.
        tool_calls: vec![call.clone(), call],
        raw_metadata: serde_json::from_value(serde_json::json!({"wire": {"retained": true}}))
            .unwrap(),
    }
}

#[derive(Debug)]
struct EvidenceRates;
impl CostRateTable for EvidenceRates {
    fn rate_for(&self, _: &str) -> Option<CostRateMicros> {
        Some(CostRateMicros {
            input_micros_per_1k_tokens: 1000,
            output_micros_per_1k_tokens: 2000,
        })
    }
}

#[tokio::test]
async fn standalone_invoker_uses_owned_single_shot_task_and_preserves_the_entire_response() {
    let expected = opaque_side_response();
    let provider = FakeProvider::new(vec![Reply::Response(Box::new(expected.clone()))]);
    let module = InvocationEvidence::new(0);
    let tool = EchoPlugin::new();
    let runtime = Arc::new(
        Runtime::builder()
            .with_plugin(ProviderPlugin::new(provider.clone()))
            .with_plugin(tool.clone())
            .with_registered_behavior(module.clone())
            .with_governance(Arc::new(AllowAll))
            .build()
            .await
            .unwrap(),
    );
    let origin = AgentTaskOrigin {
        agent: AgentId::new(),
        session: SessionId::new(),
        request: RequestId::new(),
        trace: TraceId::new(),
    };
    let invoker = RuntimeModuleInvoker::new(
        runtime.clone(),
        origin.clone(),
        None,
        MODEL,
        module.manifest.id.as_str(),
        Arc::new(ModuleTurnState::new(1)),
        0,
    );
    let response = invoker
        .invoke(
            ModuleInvocationRequest::isolated("private system", "private task").with_model(MODEL),
        )
        .await
        .unwrap();
    assert_eq!(response.response, expected);
    assert_eq!(response.served_by, provider.id);
    assert_eq!(provider.call_count(), 1);
    assert_eq!(tool.calls.load(Ordering::SeqCst), 0);
    assert!(module.hooks.lock().unwrap().is_empty());
    assert!(provider.request(0).tools.is_empty());
    assert_eq!(
        provider.request(0).messages,
        vec![
            NormalizedMessage::system("private system"),
            NormalizedMessage::user("private task")
        ]
    );
    let children = runtime.agent_instances();
    assert_eq!(children.len(), 1);
    let child = &children[0];
    assert_ne!(child.id, origin.agent);
    assert_ne!(child.session, origin.session);
    assert_eq!(child.status, AgentStatus::Stopped);
    assert_eq!(child.usage.requests, 1);
    assert_eq!(child.usage.input_tokens, 7);
    assert_eq!(
        child.owner,
        AgentOwner::Module {
            definition: module.definition(),
            instance: runtime.plugins().instance_id(&module.definition()).unwrap()
        }
    );
    let private = runtime
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert!(private.events.iter().any(|e| matches!(&e.event, SessionEventKind::AgentTaskStarted { parent_request, parent_trace, .. } if *parent_request == origin.request && *parent_trace == origin.trace)));
    assert!(matches!(
        invoker
            .clone()
            .invoke(ModuleInvocationRequest::isolated("private system", "retry"))
            .await,
        Err(ModuleInvocationError::BudgetExceeded { limit: 1 })
    ));
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn registered_invoker_usage_belongs_to_each_child_and_real_owner_with_parent_provenance() {
    let expected = opaque_side_response();
    let mut unknown_cache = expected.clone();
    unknown_cache.usage.prompt_cache_hit_tokens = None;
    let provider = FakeProvider::new(vec![
        Reply::Response(Box::new(expected.clone())),
        Reply::Response(Box::new(unknown_cache.clone())),
        Reply::Text("parent answer"),
    ]);
    let module = InvocationEvidence::new(2);
    let tool = EchoPlugin::new();
    let mut policy = AgentHostPolicy {
        concurrent_turns: 1,
        ..Default::default()
    };
    policy.modules.insert(
        module.definition(),
        ModuleAgentGrant {
            max_instances: 2,
            ..Default::default()
        },
    );
    let host = Runtime::builder()
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(tool.clone())
        .with_registered_behavior(module.clone())
        .with_governance(Arc::new(AllowAll))
        .with_default_model(MODEL)
        .with_cost_rates(Arc::new(EvidenceRates))
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![module.capability(), CapabilityId::new("tool.echo").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        parent.run("parent input".into()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(reply.text, "parent answer");
    assert_eq!(provider.call_count(), 3);
    assert_eq!(tool.calls.load(Ordering::SeqCst), 0);
    assert_eq!(module.hooks.lock().unwrap().len(), 5);
    let results = module.results.lock().unwrap();
    assert_eq!(results[0].as_ref().unwrap().response, expected);
    assert_eq!(results[1].as_ref().unwrap().response, unknown_cache);
    assert!(results
        .iter()
        .all(|r| r.as_ref().unwrap().served_by == provider.id));
    drop(results);
    let kernel = host.kernel().await;
    let public = kernel
        .sessions()
        .load(&parent.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(public.messages.len(), 2);
    assert_eq!(text(&public.messages[0]), "parent input");
    assert_eq!(text(&public.messages[1]), "parent answer");
    let parent_trace = public
        .events
        .iter()
        .find(|e| e.request == reply.request)
        .unwrap()
        .trace;
    let owner = AgentOwner::Module {
        definition: module.definition(),
        instance: kernel.plugins().instance_id(&module.definition()).unwrap(),
    };
    let children: Vec<_> = host
        .instances()
        .into_iter()
        .filter(|a| a.id != parent.id())
        .collect();
    assert_eq!(children.len(), 2);
    for child in children {
        assert_eq!(child.owner, owner);
        assert_ne!(child.session, public.id);
        assert_eq!(child.status, AgentStatus::Stopped);
        assert_eq!(child.usage.requests, 1);
        assert_eq!(child.usage.tool_calls, 0);
        let facts = host.requests(child.id).unwrap();
        assert_eq!(facts.len(), 1);
        let fact = &facts[0];
        assert_eq!(fact.agent, child.id);
        assert_eq!(fact.owner, owner);
        assert_ne!(fact.request, reply.request);
        assert_eq!(fact.provider, provider.id);
        assert_eq!(fact.estimated_cost_micros, Some(13));
        assert!(!fact.failed && !fact.cancelled);
        let private = kernel
            .sessions()
            .load(&child.session)
            .await
            .unwrap()
            .unwrap();
        assert!(private.events.iter().any(|e| e.request == fact.request && matches!(&e.event, SessionEventKind::AgentTaskStarted { agent, owner: actual_owner, parent_agent, parent_session, parent_request, parent_trace: trace } if *agent == child.id && actual_owner == &owner && *parent_agent == parent.id() && *parent_session == public.id && *parent_request == reply.request && *trace == parent_trace)));
        assert!(!private
            .events
            .iter()
            .any(|e| matches!(e.event, SessionEventKind::ToolEventRecorded { .. })));
    }
    let usage = host.usage(Some(&owner));
    assert_eq!(usage.requests, 2);
    assert_eq!(usage.input_tokens, 14);
    assert_eq!(usage.output_tokens, 6);
    assert_eq!(usage.estimated_cost_micros, 26);
    assert_eq!(usage.cached_tokens, 2);
    assert_eq!(usage.unknown_cache_requests, 1);
    assert_eq!(usage.complete_cache_count(), None);
    assert_eq!(usage.unreported_usage_requests, 0);
    assert_eq!(parent.snapshot().usage.requests, 1);
    assert_eq!(parent.snapshot().usage.input_tokens, 1);
    assert_eq!(host.usage(None).requests, 3);
    assert_eq!(host.usage(None).estimated_cost_micros, 29);
}

#[tokio::test]
async fn invoker_provider_failure_and_timeout_retain_child_facts_without_recursive_error_hooks() {
    let provider = FakeProvider::new(vec![
        Reply::Fail,
        Reply::Delay,
        Reply::Text("parent answer"),
    ]);
    let module = InvocationEvidence::new(2);
    let mut policy = AgentHostPolicy::default();
    let mut grant = ModuleAgentGrant::default();
    grant.budget.turn_timeout_ms = 40;
    policy.modules.insert(module.definition(), grant);
    let host = Runtime::builder()
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_registered_behavior(module.clone())
        .with_governance(Arc::new(AllowAll))
        .with_default_model(MODEL)
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![module.capability()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    parent.run("parent".into()).await.unwrap();
    let results = module.results.lock().unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0]
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("side request failed"));
    assert!(results[1]
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("deadline"));
    drop(results);
    assert_eq!(
        module.hooks.lock().unwrap().len(),
        5,
        "child error hooks must also be suppressed"
    );
    let children: Vec<_> = host
        .instances()
        .into_iter()
        .filter(|a| a.id != parent.id())
        .collect();
    assert_eq!(children.len(), 2);
    let usage = host.usage(Some(&children[0].owner));
    assert_eq!(usage.requests, 2);
    assert_eq!(usage.provider_errors, 1);
    assert_eq!(usage.cancellations, 1);
    assert_eq!(usage.unreported_usage_requests, 2);
    assert_eq!(usage.unknown_cache_requests, 2);
    assert_eq!(usage.complete_cost_estimate(), None);
    for child in children {
        assert_eq!(child.status, AgentStatus::Stopped);
        let facts = host.requests(child.id).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].agent, child.id);
        assert_eq!(facts[0].owner, child.owner);
        assert!(facts[0].failed || facts[0].cancelled);
        assert_eq!(facts[0].reported, None);
    }
    assert_eq!(host.usage(None).requests, 3);
}

#[tokio::test]
async fn invoker_preserves_missing_usage_and_refuses_further_budgeted_owner_requests() {
    let unreported = NormalizedResponse::text("no-usage", MODEL, "private answer");
    let provider = FakeProvider::new(vec![
        Reply::Response(Box::new(unreported.clone())),
        Reply::Text("parent answer"),
    ]);
    let module = InvocationEvidence::new(2);
    let mut policy = AgentHostPolicy::default();
    policy
        .modules
        .insert(module.definition(), ModuleAgentGrant::default());
    let host = Runtime::builder()
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_registered_behavior(module.clone())
        .with_governance(Arc::new(AllowAll))
        .with_default_model(MODEL)
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![module.capability()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    parent.run("parent".into()).await.unwrap();
    let results = module.results.lock().unwrap();
    assert_eq!(results[0].as_ref().unwrap().response, unreported);
    assert!(results[1]
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("usage is unavailable"));
    drop(results);
    let child = host
        .instances()
        .into_iter()
        .find(|a| a.id != parent.id())
        .unwrap();
    let usage = host.usage(Some(&child.owner));
    assert_eq!(usage.requests, 1);
    assert_eq!(usage.unreported_usage_requests, 1);
    assert_eq!(usage.complete_token_count(), None);
    assert_eq!(usage.complete_cost_estimate(), None);
    assert_eq!(usage.complete_cache_count(), None);
    assert_eq!(provider.call_count(), 2);
    assert_eq!(host.usage(None).requests, 2);
}

#[tokio::test]
async fn invoker_and_subloop_share_the_same_turn_admission_budget() {
    struct MixedCalls(ModuleManifest);
    #[async_trait]
    impl BehaviorModule for MixedCalls {
        fn manifest(&self) -> &ModuleManifest {
            &self.0
        }
        async fn on_hook(
            &self,
            hook: HookPoint,
            ctx: &ModuleContext<'_>,
        ) -> Result<ModuleOutcome, ModuleError> {
            if hook == HookPoint::TurnStart {
                ctx.invoker()
                    .invoke(ModuleInvocationRequest::isolated("side", "first"))
                    .await?;
                ctx.subloop()
                    .spawn(apeireth_runtime::canonical::subloop::SubLoopSpec::single_turn("second"))
                    .await
                    .unwrap();
                let error = ctx
                    .invoker_handle()
                    .clone()
                    .invoke(ModuleInvocationRequest::isolated("side", "third"))
                    .await
                    .unwrap_err();
                assert!(matches!(
                    error,
                    ModuleInvocationError::BudgetExceeded { limit: 2 }
                ));
            }
            Ok(ModuleOutcome::continue_())
        }
    }
    let provider = FakeProvider::new(vec![
        Reply::Text("one"),
        Reply::Text("two"),
        Reply::Text("parent"),
    ]);
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_max_module_invocations(2)
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_module(Arc::new(MixedCalls(ModuleManifest::new(
            "module.mixed",
            "mixed",
        ))))
        .with_governance(Arc::new(AllowAll))
        .build()
        .await
        .unwrap();
    assert_eq!(
        runtime
            .execute(TurnRequest::new(SessionId::new(), "parent"))
            .await
            .unwrap()
            .text,
        "parent"
    );
    assert_eq!(provider.call_count(), 3);
    assert_eq!(
        runtime
            .agent_instances()
            .iter()
            .map(|a| a.usage.requests)
            .sum::<u64>(),
        2
    );
}

#[derive(Clone)]
enum Reply {
    Text(&'static str),
    Tool,
    TwoTools,
    Response(Box<NormalizedResponse>),
    Fail,
    Delay,
}

struct FakeProvider {
    id: CapabilityId,
    replies: Vec<Reply>,
    calls: AtomicUsize,
    requests: Mutex<Vec<NormalizedRequest>>,
}

impl FakeProvider {
    fn new(replies: Vec<Reply>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            replies,
            calls: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn request(&self, index: usize) -> NormalizedRequest {
        self.requests.lock().unwrap()[index].clone()
    }
}

#[async_trait]
impl ProviderCapability for FakeProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.requests.lock().unwrap().push(request.clone());
        let reply = self.replies.get(index).unwrap_or_else(|| {
            panic!(
                "provider called {} times, only {} replies exist",
                index + 1,
                self.replies.len()
            )
        });
        let response = NormalizedResponse {
            id: format!("response-{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            // Scripted normal calls report usage so owner budgets remain usable.
            usage: NormalizedUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        Ok(match reply {
            Reply::Response(response) => *response.clone(),
            Reply::Fail => {
                return Err(ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail: "side request failed".into(),
                })
            }
            Reply::Delay => {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                panic!("side request must be cancelled");
            }
            Reply::Text(text) => NormalizedResponse {
                content: (*text).into(),
                ..response
            },
            Reply::Tool => NormalizedResponse {
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                tool_calls: vec![ToolCall {
                    id: "call-1".into(),
                    name: "echo".into(),
                    arguments: serde_json::json!({}),
                }],
                ..response
            },
            Reply::TwoTools => NormalizedResponse {
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                tool_calls: vec![
                    ToolCall {
                        id: "call-1".into(),
                        name: "echo".into(),
                        arguments: serde_json::json!({}),
                    },
                    ToolCall {
                        id: "call-2".into(),
                        name: "echo".into(),
                        arguments: serde_json::json!({}),
                    },
                ],
                ..response
            },
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.fake_provider").unwrap(),
                "1.0.0",
                "fake provider",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "fake provider",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

struct EchoPlugin {
    manifest: PluginManifest,
    calls: Arc<AtomicUsize>,
}

impl EchoPlugin {
    fn new() -> Arc<Self> {
        let calls = Arc::new(AtomicUsize::new(0));
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.echo").unwrap(),
                "1.0.0",
                "echo tool",
            )
            .declare_capability(
                CapabilityId::new("tool.echo").unwrap(),
                CapabilityKind::Tool,
                "echo tool",
            )
            .unwrap(),
            calls,
        })
    }
}

#[async_trait]
impl Plugin for EchoPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::new(EchoTool {
            id: CapabilityId::new("tool.echo").unwrap(),
            calls: Arc::clone(&self.calls),
        })]
    }
}

struct EchoTool {
    id: CapabilityId,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "echo".into(),
            description: Some("echo".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!("tool-result"))
    }
}

async fn runtime(provider: Arc<FakeProvider>, modules: Vec<Arc<dyn BehaviorModule>>) -> Runtime {
    let mut builder = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider));
    for module in modules {
        builder = builder.with_module(module);
    }
    builder.build().await.unwrap()
}

fn text(message: &NormalizedMessage) -> String {
    apeireth_protocol::canonical::ContentPart::join_text(&message.content)
}

struct RecordingModule {
    manifest: ModuleManifest,
    hooks: Arc<Mutex<Vec<HookPoint>>>,
}

impl RecordingModule {
    fn new(id: &str, hooks: Arc<Mutex<Vec<HookPoint>>>) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new(id, id),
            hooks,
        })
    }
}

#[async_trait]
impl BehaviorModule for RecordingModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.hooks.lock().unwrap().push(hook);
        Ok(ModuleOutcome::continue_())
    }
}

struct OverlayModule {
    manifest: ModuleManifest,
    marker: String,
    first_turn: AtomicBool,
}

impl OverlayModule {
    fn new(id: &str, marker: &str) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new(id, id),
            marker: marker.into(),
            first_turn: AtomicBool::new(true),
        })
    }
}

#[async_trait]
impl BehaviorModule for OverlayModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeModelCall && self.first_turn.swap(false, Ordering::SeqCst) {
            return Ok(ModuleOutcome::continue_().with_system_overlay(self.marker.clone()));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct RetryModule {
    manifest: ModuleManifest,
    calls: AtomicUsize,
}

impl RetryModule {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new("module.retry", "retry"),
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl BehaviorModule for RetryModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeFinalCommit && self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(ModuleOutcome::retry("revise this candidate"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct StopModule {
    manifest: ModuleManifest,
}

#[async_trait]
impl BehaviorModule for StopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeFinalCommit {
            return Ok(ModuleOutcome::stop("rejected by module"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct ApprovedToolStopModule {
    manifest: ModuleManifest,
    calls: AtomicUsize,
}

#[async_trait]
impl BehaviorModule for ApprovedToolStopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeToolCall && self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
            return Ok(ModuleOutcome::stop("module vetoed approved dispatch"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct SideCallModule {
    manifest: ModuleManifest,
    side_text: Arc<Mutex<Option<String>>>,
    side_tool_calls: Arc<Mutex<Option<usize>>>,
    hook_count: Arc<AtomicUsize>,
    origin: Arc<Mutex<Option<InvocationOrigin>>>,
}

#[async_trait]
impl BehaviorModule for SideCallModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.hook_count.fetch_add(1, Ordering::SeqCst);
        if hook != HookPoint::BeforeFinalCommit {
            return Ok(ModuleOutcome::continue_());
        }
        let candidate = ctx.candidate().ok_or(ModuleError::MissingCandidate)?;
        *self.origin.lock().unwrap() = Some(ctx.invocation.origin.clone());
        let result = ctx
            .invoker()
            .invoke(ModuleInvocationRequest::isolated(
                "SIDE_SYSTEM_MARKER",
                &candidate.content,
            ))
            .await?;
        *self.side_text.lock().unwrap() = Some(result.text().to_string());
        *self.side_tool_calls.lock().unwrap() = Some(result.response.tool_calls.len());
        Ok(ModuleOutcome::continue_())
    }
}

struct FailingModule {
    manifest: ModuleManifest,
    saw_error: Arc<AtomicBool>,
}

#[async_trait]
impl BehaviorModule for FailingModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::OnError {
            self.saw_error.store(true, Ordering::SeqCst);
            return Ok(ModuleOutcome::continue_());
        }
        Err(ModuleError::Message("module failed".into()))
    }
}

struct BudgetModule {
    manifest: ModuleManifest,
    budget_error: Arc<Mutex<Option<ModuleInvocationError>>>,
}

#[async_trait]
impl BehaviorModule for BudgetModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeFinalCommit {
            for _ in 0..3 {
                if let Err(error) = ctx
                    .invoker()
                    .invoke(ModuleInvocationRequest::isolated("side", "input"))
                    .await
                {
                    *self.budget_error.lock().unwrap() = Some(error);
                }
            }
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct ToolHookModule {
    manifest: ModuleManifest,
    hooks: Arc<Mutex<Vec<HookPoint>>>,
}

struct RetryOverlayModule {
    manifest: ModuleManifest,
    before_model_calls: AtomicUsize,
    final_checks: AtomicUsize,
}

struct BeforeModelRetryModule {
    manifest: ModuleManifest,
    calls: AtomicUsize,
}

#[async_trait]
impl BehaviorModule for BeforeModelRetryModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeModelCall && self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(ModuleOutcome::retry("retry before provider"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

#[async_trait]
impl BehaviorModule for RetryOverlayModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeModelCall {
            let call = self.before_model_calls.fetch_add(1, Ordering::SeqCst);
            return Ok(ModuleOutcome::continue_().with_system_overlay(format!("overlay-{call}")));
        }
        if hook == HookPoint::BeforeFinalCommit
            && self.final_checks.fetch_add(1, Ordering::SeqCst) == 0
        {
            return Ok(ModuleOutcome::retry("revise"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct BeforeToolRetryModule {
    manifest: ModuleManifest,
    calls: AtomicUsize,
}

#[async_trait]
impl BehaviorModule for BeforeToolRetryModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeToolCall && self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(ModuleOutcome::retry("regenerate without this tool call"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct AfterToolRetryModule {
    manifest: ModuleManifest,
    calls: AtomicUsize,
}

#[async_trait]
impl BehaviorModule for AfterToolRetryModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::AfterToolResult && self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(ModuleOutcome::retry("retry after the first result"));
        }
        Ok(ModuleOutcome::continue_())
    }
}

struct OutcomeModule {
    manifest: ModuleManifest,
    order: Arc<Mutex<Vec<String>>>,
    outcome: ModuleOutcome,
}

#[async_trait]
impl BehaviorModule for OutcomeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeFinalCommit {
            self.order.lock().unwrap().push(self.manifest.id.clone());
            return Ok(self.outcome.clone());
        }
        Ok(ModuleOutcome::continue_())
    }
}

#[async_trait]
impl BehaviorModule for ToolHookModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.hooks.lock().unwrap().push(hook);
        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn zero_modules_keep_the_existing_canonical_behavior() {
    let provider = FakeProvider::new(vec![Reply::Text("answer")]);
    let runtime = runtime(Arc::clone(&provider), Vec::new()).await;
    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap();
    assert_eq!(response.text, "answer");
    assert_eq!(response.rounds, 1);
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn hooks_fire_and_overlays_are_ordered_transient_and_not_persisted() {
    let provider = FakeProvider::new(vec![Reply::Text("first"), Reply::Text("second")]);
    let a = OverlayModule::new("module.a", "A_MARKER");
    let b = OverlayModule::new("module.b", "B_MARKER");
    let runtime = runtime(
        Arc::clone(&provider),
        vec![
            Arc::clone(&a) as Arc<dyn BehaviorModule>,
            Arc::clone(&b) as Arc<dyn BehaviorModule>,
        ],
    )
    .await;
    let session_id = SessionId::new();

    runtime
        .execute(TurnRequest::new(session_id, "one"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session_id, "two"))
        .await
        .unwrap();

    let first = provider.request(0);
    let first_texts: Vec<String> = first.messages.iter().map(text).collect();
    assert_eq!(first_texts, ["one", "A_MARKER", "B_MARKER"]); // 缓存友好组装：史在前、叠加层在尾
    let second_texts: Vec<String> = provider.request(1).messages.iter().map(text).collect();
    assert_eq!(second_texts, ["one", "first", "two"]);

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    let persisted: Vec<String> = session.messages.iter().map(text).collect();
    assert_eq!(persisted, ["one", "first", "two", "second"]);
    assert!(!persisted.iter().any(|message| message.contains("MARKER")));
}

#[tokio::test]
async fn overlays_are_recomputed_for_each_provider_retry() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate 1"), Reply::Text("candidate 2")]);
    let module = Arc::new(RetryOverlayModule {
        manifest: ModuleManifest::new("module.retry_overlay", "retry overlay"),
        before_model_calls: AtomicUsize::new(0),
        final_checks: AtomicUsize::new(0),
    });
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;

    runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap();

    let first: Vec<String> = provider.request(0).messages.iter().map(text).collect();
    assert_eq!(first, ["question", "overlay-0"]); // 缓存友好组装：史在前、叠加层在尾
    let second: Vec<String> = provider.request(1).messages.iter().map(text).collect();
    assert_eq!(second, ["question", "overlay-1", "revise"]); // 史→叠→重试脚手架
    assert!(
        !second.iter().any(|message| message == "candidate 1"),
        "rejected candidate must not be replayed to the provider"
    );
}

#[tokio::test]
async fn before_model_retry_consumes_one_logical_round_without_double_provider_calling() {
    let provider = FakeProvider::new(vec![Reply::Text("answer")]);
    let module = Arc::new(BeforeModelRetryModule {
        manifest: ModuleManifest::new("module.before_model_retry", "before model retry"),
        calls: AtomicUsize::new(0),
    });
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap();

    assert_eq!(response.rounds, 2);
    assert_eq!(provider.call_count(), 1);
    let request: Vec<String> = provider.request(0).messages.iter().map(text).collect();
    assert_eq!(request, ["question", "retry before provider"]);
}

#[tokio::test]
async fn lifecycle_hooks_include_turn_start_model_final_and_after_turn() {
    let provider = FakeProvider::new(vec![Reply::Text("answer")]);
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let module = RecordingModule::new("module.recorder", Arc::clone(&hooks));
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;
    runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap();
    let hooks = hooks.lock().unwrap().clone();
    assert_eq!(
        hooks,
        [
            HookPoint::TurnStart,
            HookPoint::BeforeModelCall,
            HookPoint::AfterModelResponse,
            HookPoint::BeforeFinalCommit,
            HookPoint::AfterTurn,
        ]
    );
}

#[tokio::test]
async fn before_final_commit_retry_uses_the_same_round_budget_and_transient_scaffolding() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate 1"), Reply::Text("candidate 2")]);
    let module = RetryModule::new();
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;
    let session_id = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session_id, "question"))
        .await
        .unwrap();

    assert_eq!(response.text, "candidate 2");
    assert_eq!(response.rounds, 2);
    let retry_request = provider.request(1);
    let retry_texts: Vec<String> = retry_request.messages.iter().map(text).collect();
    assert_eq!(retry_texts, ["question", "revise this candidate"]);
    assert!(
        !retry_texts.iter().any(|message| message == "candidate 1"),
        "rejected candidate must not be replayed to the provider"
    );
    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    let persisted: Vec<String> = session.messages.iter().map(text).collect();
    assert_eq!(persisted, ["question", "candidate 2"]);
    assert!(!persisted
        .iter()
        .any(|message| message.contains("candidate 1")));
    assert!(!persisted.iter().any(|message| message.contains("revise")));
}

#[tokio::test]
async fn before_final_commit_stop_rejects_without_committing_the_candidate() {
    let provider = FakeProvider::new(vec![Reply::Text("must not commit")]);
    let module = Arc::new(StopModule {
        manifest: ModuleManifest::new("module.stop", "stop"),
    });
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;
    let session_id = SessionId::new();
    let error = runtime
        .execute(TurnRequest::new(session_id, "question"))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::ModuleStopped { ref reason, .. } if reason == "rejected by module"
    ));
    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(
        session.messages.iter().map(text).collect::<Vec<_>>(),
        ["question"]
    );
    assert!(session.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::ExecutionFailed { phase, .. } if phase == "module_stop"
    )));
}

#[tokio::test]
async fn directive_precedence_is_strongest_first_and_all_modules_run() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate")]);
    let order = Arc::new(Mutex::new(Vec::new()));
    let modules: Vec<Arc<dyn BehaviorModule>> = vec![
        Arc::new(OutcomeModule {
            manifest: ModuleManifest::new("module.continue", "continue"),
            order: Arc::clone(&order),
            outcome: ModuleOutcome::continue_(),
        }),
        Arc::new(OutcomeModule {
            manifest: ModuleManifest::new("module.retry", "retry"),
            order: Arc::clone(&order),
            outcome: ModuleOutcome::retry("retry"),
        }),
        Arc::new(OutcomeModule {
            manifest: ModuleManifest::new("module.stop.first", "stop first"),
            order: Arc::clone(&order),
            outcome: ModuleOutcome::stop("stop first"),
        }),
        Arc::new(OutcomeModule {
            manifest: ModuleManifest::new("module.stop.second", "stop second"),
            order: Arc::clone(&order),
            outcome: ModuleOutcome::stop("stop second"),
        }),
    ];
    let runtime = runtime(Arc::clone(&provider), modules).await;

    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::ModuleStopped { ref module_id, ref reason }
            if module_id == "module.stop.first" && reason == "stop first"
    ));
    assert_eq!(
        order.lock().unwrap().as_slice(),
        [
            "module.continue",
            "module.retry",
            "module.stop.first",
            "module.stop.second",
        ]
    );
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn runtime_errors_are_observable_through_on_error_without_being_swallowed() {
    let provider = FakeProvider::new(vec![Reply::Text("never reached")]);
    let saw_error = Arc::new(AtomicBool::new(false));
    let module = Arc::new(FailingModule {
        manifest: ModuleManifest::new("module.failing", "failing"),
        saw_error: Arc::clone(&saw_error),
    });
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;
    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::Module { .. }));
    assert!(saw_error.load(Ordering::SeqCst));
    assert_eq!(provider.call_count(), 0);
}

#[tokio::test]
async fn isolated_side_calls_use_the_provider_without_tools_or_recursive_hooks() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate"), Reply::Text("side verdict")]);
    let side_text = Arc::new(Mutex::new(None));
    let side_tool_calls = Arc::new(Mutex::new(None));
    let hook_count = Arc::new(AtomicUsize::new(0));
    let origin = Arc::new(Mutex::new(None));
    let module = Arc::new(SideCallModule {
        manifest: ModuleManifest::new("module.side", "side"),
        side_text: Arc::clone(&side_text),
        side_tool_calls: Arc::clone(&side_tool_calls),
        hook_count: Arc::clone(&hook_count),
        origin: Arc::clone(&origin),
    });
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;
    let session_id = SessionId::new();
    runtime
        .execute(TurnRequest::new(session_id, "question"))
        .await
        .unwrap();

    assert_eq!(provider.call_count(), 2);
    assert_eq!(side_text.lock().unwrap().as_deref(), Some("side verdict"));
    assert_eq!(*side_tool_calls.lock().unwrap(), Some(0));
    let side_request = provider.request(1);
    assert!(side_request.tools.is_empty());
    assert_eq!(text(&side_request.messages[0]), "SIDE_SYSTEM_MARKER");
    assert_eq!(text(&side_request.messages[1]), "candidate");
    assert_eq!(hook_count.load(Ordering::SeqCst), 5);
    assert!(matches!(
        origin.lock().unwrap().as_ref(),
        Some(InvocationOrigin::UserTurn)
    ));
    assert_eq!(
        runtime
            .sessions()
            .load_or_create(session_id)
            .await
            .unwrap()
            .messages
            .len(),
        2
    );
}

#[tokio::test]
async fn side_call_tool_responses_are_never_dispatched() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate"), Reply::Tool]);
    let side_text = Arc::new(Mutex::new(None));
    let side_tool_calls = Arc::new(Mutex::new(None));
    let hook_count = Arc::new(AtomicUsize::new(0));
    let origin = Arc::new(Mutex::new(None));
    let module = Arc::new(SideCallModule {
        manifest: ModuleManifest::new("module.side_tool_response", "side tool response"),
        side_text,
        side_tool_calls: Arc::clone(&side_tool_calls),
        hook_count,
        origin,
    });
    let tool_plugin = EchoPlugin::new();
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_plugin(tool_plugin.clone())
        .with_module(module)
        .build()
        .await
        .unwrap();

    runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap();

    assert_eq!(*side_tool_calls.lock().unwrap(), Some(1));
    assert_eq!(tool_plugin.calls.load(Ordering::SeqCst), 0);
    assert!(provider.request(1).tools.is_empty());
}

#[tokio::test]
async fn side_call_budget_is_shared_and_explicit() {
    let provider = FakeProvider::new(vec![
        Reply::Text("candidate"),
        Reply::Text("side"),
        Reply::Text("side"),
    ]);
    let budget_error = Arc::new(Mutex::new(None));
    let module = Arc::new(BudgetModule {
        manifest: ModuleManifest::new("module.budget", "budget"),
        budget_error: Arc::clone(&budget_error),
    });
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_max_module_invocations(2)
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(module)
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap();
    assert_eq!(provider.call_count(), 3);
    assert!(matches!(
        *budget_error.lock().unwrap(),
        Some(ModuleInvocationError::BudgetExceeded { limit: 2 })
    ));
}

#[tokio::test]
async fn tool_hooks_run_around_governed_dispatch() {
    let provider = FakeProvider::new(vec![Reply::Tool, Reply::Text("done")]);
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let module = Arc::new(ToolHookModule {
        manifest: ModuleManifest::new("module.tools", "tools"),
        hooks: Arc::clone(&hooks),
    });
    let tool_plugin = EchoPlugin::new();
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(
            DenyCapabilities::new().deny(CapabilityId::new("tool.echo").unwrap()),
        ))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_plugin(tool_plugin.clone())
        .with_module(module)
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "use tool"))
        .await
        .unwrap();
    let hooks = hooks.lock().unwrap().clone();
    assert!(hooks.contains(&HookPoint::BeforeToolCall));
    assert!(hooks.contains(&HookPoint::AfterToolResult));
    assert_eq!(
        tool_plugin.calls.load(Ordering::SeqCst),
        0,
        "module Continue cannot bypass governance denial"
    );
}

#[tokio::test]
async fn before_tool_retry_closes_the_tool_transcript_before_the_next_provider_call() {
    let provider = FakeProvider::new(vec![Reply::Tool, Reply::Text("done")]);
    let module = Arc::new(BeforeToolRetryModule {
        manifest: ModuleManifest::new("module.before_tool_retry", "before tool retry"),
        calls: AtomicUsize::new(0),
    });
    let runtime = runtime(Arc::clone(&provider), vec![module]).await;
    let session_id = SessionId::new();

    let response = runtime
        .execute(TurnRequest::new(session_id, "use tool"))
        .await
        .unwrap();
    assert_eq!(response.rounds, 2);

    let retry_request = provider.request(1);
    assert_eq!(retry_request.messages[1].role, MessageRole::Assistant);
    assert_eq!(retry_request.messages[1].tool_calls.len(), 1);
    assert_eq!(retry_request.messages[2].role, MessageRole::Tool);
    assert_eq!(
        retry_request.messages[2].tool_call_id.as_deref(),
        Some("call-1")
    );
    assert!(text(&retry_request.messages[2]).contains("skipped"));
    assert_eq!(retry_request.messages[3].role, MessageRole::User);
    assert_eq!(
        text(&retry_request.messages[3]),
        "regenerate without this tool call"
    );

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(session.messages[2].role, MessageRole::Tool);
    assert_eq!(session.messages.len(), 4);
}

#[tokio::test]
async fn after_tool_retry_closes_remaining_calls_without_mixing_feedback_into_results() {
    let provider = FakeProvider::new(vec![Reply::TwoTools, Reply::Text("done")]);
    let tool_plugin = EchoPlugin::new();
    let module = Arc::new(AfterToolRetryModule {
        manifest: ModuleManifest::new("module.after_tool_retry", "after tool retry"),
        calls: AtomicUsize::new(0),
    });
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_plugin(tool_plugin.clone())
        .with_module(module)
        .build()
        .await
        .unwrap();
    let session_id = SessionId::new();

    let response = runtime
        .execute(TurnRequest::new(session_id, "use both tools"))
        .await
        .unwrap();
    assert_eq!(response.rounds, 2);
    assert_eq!(tool_plugin.calls.load(Ordering::SeqCst), 1);

    let retry_request = provider.request(1);
    assert_eq!(retry_request.messages[1].tool_calls.len(), 2);
    assert_eq!(retry_request.messages[2].role, MessageRole::Tool);
    assert_eq!(
        retry_request.messages[2].tool_call_id.as_deref(),
        Some("call-1")
    );
    assert_eq!(retry_request.messages[3].role, MessageRole::Tool);
    assert_eq!(
        retry_request.messages[3].tool_call_id.as_deref(),
        Some("call-2")
    );
    assert!(text(&retry_request.messages[3]).contains("remaining tool calls skipped"));
    assert_eq!(retry_request.messages[4].role, MessageRole::User);
    assert_eq!(
        text(&retry_request.messages[4]),
        "retry after the first result"
    );
}

#[tokio::test]
async fn rejected_approval_still_emits_after_tool_result() {
    let provider = FakeProvider::new(vec![Reply::Tool, Reply::Text("done")]);
    let tool_plugin = EchoPlugin::new();
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let module = Arc::new(ToolHookModule {
        manifest: ModuleManifest::new("module.approval_hooks", "approval hooks"),
        hooks: Arc::clone(&hooks),
    });
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.echo".into()));
    policy.require_approval_for("tool.echo");
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_plugin(tool_plugin.clone())
        .with_module(module)
        .build()
        .await
        .unwrap();
    let session_id = SessionId::new();

    let pending = match runtime
        .execute_outcome(TurnRequest::new(session_id, "use tool"))
        .await
        .unwrap()
    {
        TurnOutcome::PendingApproval(view) => view,
        TurnOutcome::Completed(_) => panic!("expected approval pause"),
    };
    let resolution = runtime
        .resolve_approval(
            session_id,
            pending.approval_id,
            ApprovalDecision::Reject {
                reason: Some("not now".into()),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        resolution,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));
    assert_eq!(tool_plugin.calls.load(Ordering::SeqCst), 0);
    assert!(hooks.lock().unwrap().contains(&HookPoint::AfterToolResult));
}

#[tokio::test]
async fn module_veto_of_a_claimed_approval_clears_the_active_pause_fail_closed() {
    let provider = FakeProvider::new(vec![Reply::Tool]);
    let tool_plugin = EchoPlugin::new();
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.echo".into()));
    policy.require_approval_for("tool.echo");
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_plugin(tool_plugin.clone())
        .with_module(Arc::new(ApprovedToolStopModule {
            manifest: ModuleManifest::new("module.approved_stop", "approved stop"),
            calls: AtomicUsize::new(0),
        }))
        .build()
        .await
        .unwrap();
    let session_id = SessionId::new();

    let pending = match runtime
        .execute_outcome(TurnRequest::new(session_id, "use tool"))
        .await
        .unwrap()
    {
        TurnOutcome::PendingApproval(view) => view,
        TurnOutcome::Completed(_) => panic!("expected approval pause"),
    };
    let error = runtime
        .resolve_approval(session_id, pending.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::ModuleStopped { .. }));
    assert_eq!(tool_plugin.calls.load(Ordering::SeqCst), 0);

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(session.active_approval_id, None);
    assert_eq!(
        session.approvals.get(&pending.approval_id).unwrap().status,
        apeireth_runtime::canonical::ApprovalStatus::Interrupted
    );
}

#[tokio::test]
async fn retry_cannot_extend_the_structural_round_limit() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate")]);
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_max_rounds(1)
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_module(RetryModule::new())
        .build()
        .await
        .unwrap();
    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::RoundLimitExceeded { limit: 1, .. }
    ));
    assert_eq!(provider.call_count(), 1);
}

struct DenyCompletions;

#[async_trait]
impl GovernanceHook for DenyCompletions {
    fn name(&self) -> &str {
        "deny_completions"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match request.action {
            Action::Completion { .. } => Decision::deny("completions blocked"),
            _ => Decision::Allow,
        }
    }
}

#[tokio::test]
async fn isolated_module_invoker_obeys_completion_governance() {
    let provider = FakeProvider::new(vec![Reply::Text("must not run")]);
    let side_text = Arc::new(Mutex::new(None));
    let module = Arc::new(SideCallModule {
        manifest: ModuleManifest::new("module.side_denied", "side denied"),
        side_text: Arc::clone(&side_text),
        side_tool_calls: Arc::new(Mutex::new(None)),
        hook_count: Arc::new(AtomicUsize::new(0)),
        origin: Arc::new(Mutex::new(None)),
    });
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(DenyCompletions))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(module)
        .build()
        .await
        .unwrap();
    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("completions blocked"), "{error}");
    assert_eq!(provider.call_count(), 0);
    assert!(side_text.lock().unwrap().is_none());
}

/// Propagates a budget-exhaustion error with `?`, exactly like the judge
/// module does when its isolated side-call hits the per-turn budget.
struct PropagatingBudgetModule {
    manifest: ModuleManifest,
}

#[async_trait]
impl BehaviorModule for PropagatingBudgetModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::BeforeFinalCommit {
            // 1st call fits the budget; 2nd exhausts it and propagates.
            let _ = ctx
                .invoker()
                .invoke(ModuleInvocationRequest::isolated("side", "first"))
                .await?;
            ctx.invoker()
                .invoke(ModuleInvocationRequest::isolated("side", "second"))
                .await?;
        }
        Ok(ModuleOutcome::continue_())
    }
}

/// Regression for the 2026-09-19 real-machine failure: when a module (judge)
/// propagates `BudgetExceeded`, the canonical loop used to abort the WHOLE
/// turn with "module … budget exceeded" — the user lost the candidate answer.
/// The fix degrades the module to Continue for the pass, commits the
/// candidate, and records the skip as a session event.
#[tokio::test]
async fn budget_exhaustion_degrades_the_module_instead_of_aborting_the_turn() {
    let provider = FakeProvider::new(vec![Reply::Text("candidate"), Reply::Text("side")]);
    let module = Arc::new(PropagatingBudgetModule {
        manifest: ModuleManifest::new("module.budget_propagate", "budget propagate"),
    });
    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_max_module_invocations(1)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(module)
        .build()
        .await
        .unwrap();

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "question"))
        .await
        .expect("the turn must survive budget exhaustion");
    assert_eq!(response.text, "candidate");
    // main candidate + one successful side-call; the exhausting call never ran.
    assert_eq!(provider.call_count(), 2);
}
