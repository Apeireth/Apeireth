//! End-to-end coverage for the canonical cognitive-module ABI.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::{
    DenyCapabilities, GovernancePipeline, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    MessageRole, ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedMessage,
    NormalizedRequest, NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall,
    ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    AgentModule, ApprovalDecision, ApprovalResolution, HookPoint, InvocationOrigin, ModuleContext,
    ModuleError, ModuleInvocationError, ModuleInvocationRequest, ModuleManifest, ModuleOutcome,
    Runtime, RuntimeError, SessionEventKind, TurnOutcome, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

#[derive(Clone)]
enum Reply {
    Text(&'static str),
    Tool,
    TwoTools,
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
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        Ok(match reply {
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

async fn runtime(provider: Arc<FakeProvider>, modules: Vec<Arc<dyn AgentModule>>) -> Runtime {
    let mut builder = Runtime::builder()
        .with_default_model(MODEL)
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
impl AgentModule for RecordingModule {
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
impl AgentModule for OverlayModule {
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
impl AgentModule for RetryModule {
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
impl AgentModule for StopModule {
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
impl AgentModule for ApprovedToolStopModule {
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
impl AgentModule for SideCallModule {
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
impl AgentModule for FailingModule {
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
impl AgentModule for BudgetModule {
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
impl AgentModule for BeforeModelRetryModule {
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
impl AgentModule for RetryOverlayModule {
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
impl AgentModule for BeforeToolRetryModule {
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
impl AgentModule for AfterToolRetryModule {
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
impl AgentModule for OutcomeModule {
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
impl AgentModule for ToolHookModule {
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
            Arc::clone(&a) as Arc<dyn AgentModule>,
            Arc::clone(&b) as Arc<dyn AgentModule>,
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
    assert_eq!(first_texts, ["A_MARKER", "B_MARKER", "one"]);
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
    assert_eq!(first, ["overlay-0", "question"]);
    let second: Vec<String> = provider.request(1).messages.iter().map(text).collect();
    assert_eq!(second, ["overlay-1", "question", "candidate 1", "revise"]);
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
    assert_eq!(
        retry_texts,
        ["question", "candidate 1", "revise this candidate"]
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
    let modules: Vec<Arc<dyn AgentModule>> = vec![
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
        RuntimeError::RoundLimitExceeded { limit: 1 }
    ));
    assert_eq!(provider.call_count(), 1);
}
