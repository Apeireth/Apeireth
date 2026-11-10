//! Real Runtime integration: no shell, network service, or domain implementation.
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::{
    system_clock, CapabilityId, Lifecycle, ModelId, PluginId, SessionId, TaskId, TraceId,
};
use apeireth_governance::{Action, AllowAll, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::hooks::{HookContextV1, HookToolRequest};
use apeireth_plugin::{
    CapabilityKind, FrozenInvocation, NoCredentials, Plugin, PluginContext, PluginError,
    PluginManager, PluginManifest, PluginResult, ProviderCapability, ProviderError, ToolCapability,
};
use apeireth_protocol::adapters::{
    AnthropicMessagesAdapter, GeminiAdapter, OpenAiChatAdapter, OpenAiResponsesAdapter,
};
use apeireth_protocol::canonical::*;
use apeireth_protocol::ProtocolAdapter;
use apeireth_runtime::canonical::dispatcher::{DispatchLimits, ToolScope};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, Runtime, SessionEventKind, TurnOutcome, TurnRequest,
};
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{Barrier, Notify};

fn pid(id: &str) -> PluginId {
    PluginId::new(id).unwrap()
}
fn context() -> PluginContext {
    PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new())
}
fn call(name: &str) -> ToolCall {
    ToolCall {
        id: format!("model_{name}"),
        name: name.into(),
        arguments: json!({"value": 7}),
    }
}
fn auto(name: &str, execution: TaskId) -> HookToolRequest {
    HookToolRequest {
        execution,
        call: call(name),
    }
}

#[derive(Clone)]
enum ToolMode {
    Echo,
    Pending(Arc<Notify>),
    WrongId,
    Panic,
    Large,
    LargeEvent,
}
type Seen = (ToolCall, Option<FrozenInvocation>, Option<SessionId>);
struct TestTool {
    id: CapabilityId,
    name: String,
    seen: Mutex<Vec<Seen>>,
    mode: ToolMode,
}
impl TestTool {
    fn new(name: &str) -> Arc<Self> {
        Self::mode(name, ToolMode::Echo)
    }
    fn mode(name: &str, mode: ToolMode) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(format!("tool.{name}")).unwrap(),
            name: name.into(),
            seen: Mutex::new(vec![]),
            mode,
        })
    }
}
#[async_trait]
impl ToolCapability for TestTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(&self.name).with_parameters(json!({"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}).as_object().unwrap().clone())
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invoke_in_session(call, None, None).await
    }
    fn freeze_invocation(&self, _: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        Ok(Some(FrozenInvocation::new(
            json!({"bound":"original"}),
            json!({"display":"redacted"}),
        )))
    }
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.seen
            .lock()
            .unwrap()
            .push((call.clone(), frozen.cloned(), session));
        match &self.mode {
            ToolMode::Echo => ToolResult::ok(&call.id, call.arguments.clone()),
            ToolMode::Pending(entered) => {
                entered.notify_one();
                std::future::pending().await
            }
            ToolMode::WrongId => ToolResult::ok("unrelated", json!(0)),
            ToolMode::Panic => panic!("fixture tool panic"),
            ToolMode::Large => ToolResult::ok(&call.id, json!("x".repeat(1024))),
            ToolMode::LargeEvent => ToolResult::ok(&call.id, json!("x".repeat(300_000))),
        }
    }
}

#[derive(Clone)]
enum Callback {
    Calls(Vec<HookToolRequest>),
    Barrier(Arc<Barrier>),
    Pending(Arc<Notify>),
    Fail,
    Panic,
}
struct TestModule {
    manifest: PluginManifest,
    tools: Vec<Arc<TestTool>>,
    hooks: Vec<(ModuleHookV1, Callback)>,
    lifecycle: Arc<Mutex<Vec<String>>>,
    fail_init: bool,
    fail_start: bool,
    fail_stop: bool,
}
impl TestModule {
    fn new(id: &str, tools: Vec<Arc<TestTool>>, hooks: Vec<(ModuleHookV1, Callback)>) -> Self {
        let mut manifest = PluginManifest::new(pid(id), "1.0.0", "integration fixture");
        for tool in &tools {
            manifest = manifest
                .declare_capability(tool.id.clone(), CapabilityKind::Tool, "fixture")
                .unwrap();
        }
        Self {
            manifest,
            tools,
            hooks,
            lifecycle: Arc::new(Mutex::new(vec![])),
            fail_init: false,
            fail_start: false,
            fail_stop: false,
        }
    }
    fn record(&self, phase: &str) {
        self.lifecycle
            .lock()
            .unwrap()
            .push(format!("{}:{phase}", self.manifest.id));
    }
}
#[async_trait]
impl Plugin for TestModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        self.record("initialize");
        if self.fail_init {
            Err(PluginError::init_failed(
                self.manifest.id.clone(),
                "fixture",
            ))
        } else {
            Ok(())
        }
    }
    async fn start(&self, _: &PluginContext) -> PluginResult<()> {
        self.record("start");
        if self.fail_start {
            Err(PluginError::init_failed(
                self.manifest.id.clone(),
                "fixture start",
            ))
        } else {
            Ok(())
        }
    }
    async fn shutdown(&self) -> PluginResult<()> {
        self.record("shutdown");
        if self.fail_stop {
            Err(PluginError::shutdown_failed(
                self.manifest.id.clone(),
                "fixture cleanup",
            ))
        } else {
            Ok(())
        }
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools
            .iter()
            .cloned()
            .map(|t| t as Arc<dyn ToolCapability>)
            .collect()
    }
    fn hooks(&self) -> Vec<ModuleHookV1> {
        self.hooks.iter().map(|(point, _)| *point).collect()
    }
    async fn on_hook(&self, ctx: &HookContextV1) -> PluginResult<Vec<HookToolRequest>> {
        match self
            .hooks
            .iter()
            .find(|(point, _)| *point == ctx.hook)
            .map(|(_, callback)| callback)
            .unwrap()
        {
            Callback::Calls(calls) => Ok(calls.clone()),
            Callback::Barrier(barrier) => {
                barrier.wait().await;
                Ok(vec![])
            }
            Callback::Pending(entered) => {
                entered.notify_one();
                std::future::pending().await
            }
            Callback::Fail => Err(PluginError::init_failed(
                self.manifest.id.clone(),
                "fixture callback",
            )),
            Callback::Panic => panic!("fixture callback panic"),
        }
    }
}

struct MockProvider {
    id: CapabilityId,
    requests: Mutex<Vec<NormalizedRequest>>,
    script: Mutex<VecDeque<Vec<ToolCall>>>,
}
impl MockProvider {
    fn new(script: Vec<Vec<ToolCall>>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.registry_test").unwrap(),
            requests: Mutex::new(vec![]),
            script: Mutex::new(script.into()),
        })
    }
}
#[async_trait]
impl ProviderCapability for MockProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new("registry-test").unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls)
                .with_context_window(1024),
        ]
    }
    async fn complete(&self, req: &NormalizedRequest) -> Result<NormalizedResponse, ProviderError> {
        validate_closed_tool_context(&req.messages)
            .expect("Runtime must send a closed conversation");
        let mut wire = req.clone();
        wire.max_tokens = Some(128);
        for adapter in [
            Box::new(OpenAiChatAdapter::new()) as Box<dyn ProtocolAdapter>,
            Box::new(OpenAiResponsesAdapter::new()),
            Box::new(AnthropicMessagesAdapter::new()),
            Box::new(GeminiAdapter::new()),
        ] {
            adapter
                .adapt_request(&wire)
                .expect("actual Runtime request must project through each protocol");
        }
        self.requests.lock().unwrap().push(req.clone());
        let mut response = NormalizedResponse::text("fixture-response", &req.model, "done");
        response.tool_calls = self.script.lock().unwrap().pop_front().unwrap_or_default();
        Ok(response)
    }
}
struct ProviderModule {
    manifest: PluginManifest,
    provider: Arc<MockProvider>,
}
impl ProviderModule {
    fn new(provider: Arc<MockProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(pid("provider.fixture"), "1.0.0", "fixture")
                .declare_capability(provider.id.clone(), CapabilityKind::Provider, "fixture")
                .unwrap(),
            provider,
        })
    }
}
#[async_trait]
impl Plugin for ProviderModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.provider.clone()]
    }
}
async fn runtime(
    modules: Vec<Arc<TestModule>>,
    provider: Arc<MockProvider>,
    limits: DispatchLimits,
) -> Runtime {
    let mut builder = Runtime::builder()
        .with_default_model("registry-test")
        .with_plugin(ProviderModule::new(provider))
        .with_governance(Arc::new(AllowAll))
        .with_dispatch_limits(limits);
    for module in modules {
        builder = builder.with_plugin(module);
    }
    builder.build().await.unwrap()
}

#[tokio::test]
async fn registered_module_injects_and_answers_model_in_a_real_turn() {
    let tool = TestTool::new("fixture_echo");
    let execution = TaskId::new();
    let module = Arc::new(TestModule::new(
        "module.echo",
        vec![tool.clone()],
        vec![(
            ModuleHookV1::BeforeModelCall,
            Callback::Calls(vec![
                auto("fixture_echo", execution),
                auto("fixture_echo", execution),
            ]),
        )],
    ));
    let provider = MockProvider::new(vec![vec![call("fixture_echo")]]);
    let runtime = runtime(vec![module], provider.clone(), DispatchLimits::default()).await;
    let id = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(id, "hello"))
        .await
        .unwrap();
    assert_eq!(response.text, "done");
    assert_eq!(response.rounds, 2);
    let seen = tool.seen.lock().unwrap();
    assert_eq!(seen.len(), 2, "one automatic execution, one model execution despite duplicate hook requests and another round");
    assert!(seen.iter().all(|(_, _, session)| *session == Some(id)));
    drop(seen);
    let requests = provider.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let module_message = requests[0]
        .messages
        .iter()
        .find(|m| m.role == MessageRole::Tool)
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&ContentPart::join_text(&module_message.content)).unwrap();
    assert_eq!(payload["origin"], "module");
    assert_eq!(payload["module_id"], "module.echo");
    assert_eq!(payload["payload"]["value"]["value"], 7);
    assert!(requests[0]
        .messages
        .iter()
        .filter(|m| m.role != MessageRole::Tool)
        .all(|m| !ContentPart::join_text(&m.content).contains("\"origin\"")));
    drop(requests);
    let session = runtime.sessions().load(&id).await.unwrap().unwrap();
    let facts = session
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SessionEventKind::ToolEventRecorded { fact } => Some(fact),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(facts.len(), 2);
    assert!(facts
        .iter()
        .any(|e| matches!(e.source(), ToolEventSourceV1::Model { module_id: Some(_) })));
    let restored: apeireth_runtime::canonical::Session =
        serde_json::from_value(serde_json::to_value(&session).unwrap()).unwrap();
    assert_eq!(restored.provider_view(), session.provider_view());
}

#[tokio::test]
async fn pending_model_batch_defers_hook_pairs_until_all_results_close() {
    let model_tool = TestTool::new("main_echo");
    let auto_tool = TestTool::new("auto_echo");
    let module = Arc::new(TestModule::new(
        "module.batch",
        vec![model_tool, auto_tool.clone()],
        vec![
            (
                ModuleHookV1::BeforeToolCall,
                Callback::Calls(vec![auto("auto_echo", TaskId::new())]),
            ),
            (
                ModuleHookV1::AfterToolCall,
                Callback::Calls(vec![auto("auto_echo", TaskId::new())]),
            ),
        ],
    ));
    let mut second = call("main_echo");
    second.id = "second".into();
    let provider = MockProvider::new(vec![vec![call("main_echo"), second]]);
    let runtime = runtime(vec![module], provider.clone(), DispatchLimits::default()).await;
    runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    assert_eq!(
        auto_tool.seen.lock().unwrap().len(),
        2,
        "no recursive hook execution"
    );
    let requests = provider.requests.lock().unwrap();
    let messages = &requests[1].messages;
    let model = messages
        .iter()
        .position(|m| m.tool_calls.len() == 2)
        .unwrap();
    assert_eq!(messages[model + 1].role, MessageRole::Tool);
    assert_eq!(messages[model + 2].role, MessageRole::Tool);
    assert_eq!(
        messages[model + 3].role,
        MessageRole::Assistant,
        "automatic pairs appear after both model results"
    );
}

#[tokio::test]
async fn lifecycle_is_dependency_ordered_split_and_unregister_removes_tools() {
    let log = Arc::new(Mutex::new(vec![]));
    let mut a = TestModule::new("module.a", vec![TestTool::new("owned")], vec![]);
    a.lifecycle = log.clone();
    let mut b = TestModule::new("module.b", vec![], vec![]);
    b.manifest = b.manifest.depends_on(pid("module.a"));
    b.lifecycle = log.clone();
    let mut registry = PluginManager::new();
    registry.register(Arc::new(b)).unwrap();
    registry.register(Arc::new(a)).unwrap();
    registry
        .initialize(&pid("module.a"), &context())
        .await
        .unwrap();
    assert_eq!(
        registry.state(&pid("module.a")).unwrap(),
        Lifecycle::Initialized
    );
    assert!(registry.active_tools().is_empty());
    registry.start(&pid("module.a"), &context()).await.unwrap();
    registry
        .initialize(&pid("module.b"), &context())
        .await
        .unwrap();
    registry.start(&pid("module.b"), &context()).await.unwrap();
    assert!(registry.stop(&pid("module.a")).await.is_err());
    assert!(registry.unregister(&pid("module.a")).is_err());
    assert!(registry.shutdown_all().await.is_empty());
    registry.unregister(&pid("module.b")).unwrap();
    registry.unregister(&pid("module.a")).unwrap();
    assert!(registry.records().is_empty());
    assert!(registry.tool_by_name("owned").is_none());
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "module.a:initialize",
            "module.a:start",
            "module.b:initialize",
            "module.b:start",
            "module.b:shutdown",
            "module.a:shutdown"
        ]
    );
}

#[tokio::test]
async fn cycle_missing_dependency_duplicates_and_tool_name_conflicts_fail_before_start() {
    let mut a = TestModule::new("module.a", vec![], vec![]);
    a.manifest = a.manifest.depends_on(pid("module.b"));
    let mut b = TestModule::new("module.b", vec![], vec![]);
    b.manifest = b.manifest.depends_on(pid("module.a"));
    let a = Arc::new(a);
    let mut registry = PluginManager::new();
    registry.register(a.clone()).unwrap();
    assert!(matches!(
        registry.resolve_order(),
        Err(PluginError::MissingDependency { .. })
    ));
    registry.register(Arc::new(b)).unwrap();
    assert!(matches!(
        registry.start_all(&context()).await,
        Err(PluginError::DependencyCycle(_))
    ));
    assert!(a.lifecycle.lock().unwrap().is_empty());
    assert!(registry.register(a).is_err());
    let mut registry = PluginManager::new();
    registry
        .register(Arc::new(TestModule::new(
            "module.first",
            vec![TestTool::new("collision")],
            vec![],
        )))
        .unwrap();
    assert!(registry
        .register(Arc::new(TestModule::new(
            "module.second",
            vec![TestTool::new("collision")],
            vec![]
        )))
        .is_err());
    assert_eq!(registry.plugins().len(), 1);
}

#[tokio::test]
async fn boot_start_failure_rolls_back_including_partially_initialized_module() {
    for fail_initialize in [true, false] {
        let log = Arc::new(Mutex::new(vec![]));
        let mut a = TestModule::new("module.a", vec![], vec![]);
        a.lifecycle = log.clone();
        let mut b = TestModule::new("module.b", vec![], vec![]);
        b.lifecycle = log.clone();
        b.manifest = b.manifest.depends_on(pid("module.a"));
        b.fail_init = fail_initialize;
        b.fail_start = !fail_initialize;
        let mut registry = PluginManager::new();
        registry.register(Arc::new(a)).unwrap();
        registry.register(Arc::new(b)).unwrap();
        assert!(registry.start_all_or_rollback(&context()).await.is_err());
        assert_eq!(
            registry.state(&pid("module.a")).unwrap(),
            Lifecycle::Stopped
        );
        assert_eq!(registry.state(&pid("module.b")).unwrap(), Lifecycle::Failed);
        assert_eq!(registry.errors(&pid("module.b")).unwrap().len(), 1);
        let log = log.lock().unwrap();
        assert_eq!(
            &log[log.len() - 2..],
            ["module.b:shutdown", "module.a:shutdown"]
        );
    }
}

#[tokio::test]
async fn shutdown_failure_does_not_abandon_independent_modules() {
    let mut a = TestModule::new("module.a", vec![], vec![]);
    a.fail_stop = true;
    let b = Arc::new(TestModule::new("module.b", vec![], vec![]));
    let mut registry = PluginManager::new();
    registry.register(Arc::new(a)).unwrap();
    registry.register(b.clone()).unwrap();
    registry.start_all(&context()).await.unwrap();
    assert_eq!(registry.shutdown_all().await.len(), 1);
    assert_eq!(
        registry.state(&pid("module.b")).unwrap(),
        Lifecycle::Stopped
    );
    assert!(registry.unregister(&pid("module.a")).is_err());
    assert!(b
        .lifecycle
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.ends_with("shutdown")));
}

#[tokio::test]
async fn hook_callbacks_run_concurrently_but_commit_in_dependency_order() {
    let barrier = Arc::new(Barrier::new(2));
    let a = Arc::new(TestModule::new(
        "module.a",
        vec![],
        vec![(
            ModuleHookV1::BeforeModelCall,
            Callback::Barrier(barrier.clone()),
        )],
    ));
    let mut b = TestModule::new(
        "module.b",
        vec![],
        vec![(ModuleHookV1::BeforeModelCall, Callback::Barrier(barrier))],
    );
    b.manifest = b.manifest.depends_on(pid("module.a"));
    let runtime = runtime(
        vec![Arc::new(b), a],
        MockProvider::new(vec![]),
        DispatchLimits::default(),
    )
    .await;
    let id = SessionId::new();
    tokio::time::timeout(
        Duration::from_secs(2),
        runtime.execute(TurnRequest::new(id, "hello")),
    )
    .await
    .unwrap()
    .unwrap();
    let session = runtime.sessions().load(&id).await.unwrap().unwrap();
    let owners = session
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SessionEventKind::RegisteredHook {
                module,
                hook: ModuleHookV1::BeforeModelCall,
                status,
            } => {
                assert_eq!(status, "completed");
                Some(module.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(owners, vec!["module.a", "module.b"]);
}

#[tokio::test]
async fn hook_timeout_error_panic_and_cancellation_are_isolated_and_drained() {
    for (callback, expected, cancel) in [
        (Callback::Pending(Arc::new(Notify::new())), "timeout", false),
        (Callback::Fail, "callback_error", false),
        (Callback::Panic, "panicked", false),
        (
            Callback::Pending(Arc::new(Notify::new())),
            "cancelled",
            true,
        ),
    ] {
        let entered = match &callback {
            Callback::Pending(n) => Some(n.clone()),
            _ => None,
        };
        let module = Arc::new(TestModule::new(
            "module.fail",
            vec![],
            vec![(ModuleHookV1::BeforeModelCall, callback)],
        ));
        let limits = DispatchLimits {
            hook_timeout: if cancel {
                Duration::from_secs(3)
            } else {
                Duration::from_millis(30)
            },
            ..Default::default()
        };
        let runtime = Arc::new(runtime(vec![module], MockProvider::new(vec![]), limits).await);
        let id = SessionId::new();
        let task_runtime = runtime.clone();
        let task =
            tokio::spawn(async move { task_runtime.execute(TurnRequest::new(id, "hello")).await });
        if cancel {
            entered.unwrap().notified().await;
            runtime.cancel_module(&pid("module.fail")).unwrap();
        }
        assert_eq!(task.await.unwrap().unwrap().text, "done");
        assert_eq!(
            runtime
                .plugins()
                .task_scope(&pid("module.fail"))
                .unwrap()
                .active_tasks(),
            0
        );
        let session = runtime.sessions().load(&id).await.unwrap().unwrap();
        assert!(session.events.iter().any(
            |e| matches!(&e.event,SessionEventKind::RegisteredHook {status,..} if status==expected)
        ));
    }
}

#[tokio::test]
async fn dispatcher_schema_scope_and_governance_refusals_do_not_invoke() {
    for mode in [0, 1, 2, 3] {
        let tool = TestTool::new("restricted");
        let mut proposed = call("restricted");
        if mode == 0 {
            proposed.arguments = json!({"value":"wrong"});
        }
        if mode == 3 {
            proposed.arguments = json!({"value":7,"unexpected":true});
        }
        let provider = MockProvider::new(vec![vec![proposed]]);
        let module = Arc::new(TestModule::new(
            "module.restricted",
            vec![tool.clone()],
            if mode == 3 {
                vec![(ModuleHookV1::BeforeModelCall, Callback::Calls(vec![]))]
            } else {
                vec![]
            },
        ));
        let runtime = if mode == 2 {
            Runtime::builder()
                .with_default_model("registry-test")
                .with_plugin(ProviderModule::new(provider.clone()))
                .with_plugin(module)
                .build()
                .await
                .unwrap()
        } else {
            runtime(vec![module], provider.clone(), DispatchLimits::default()).await
        };
        let id = SessionId::new();
        if mode == 1 {
            runtime
                .tool_dispatcher()
                .set_scope(id, ToolScope::Only(BTreeSet::new()));
            assert!(runtime.tool_declarations_for_session(id).is_empty());
        }
        runtime
            .execute(TurnRequest::new(id, "hello"))
            .await
            .unwrap();
        assert!(tool.seen.lock().unwrap().is_empty());
        let requests = provider.requests.lock().unwrap();
        assert!(
            ContentPart::join_text(&requests[1].messages.last().unwrap().content)
                .contains("error:")
        );
    }
}

#[tokio::test]
async fn automatic_hook_cannot_bypass_governance_or_own_another_modules_tool() {
    for other_owner in [false, true] {
        let tool = TestTool::new("protected");
        let owned = Arc::new(TestModule::new(
            "module.owner",
            vec![tool.clone()],
            if !other_owner {
                vec![(
                    ModuleHookV1::BeforeModelCall,
                    Callback::Calls(vec![auto("protected", TaskId::new())]),
                )]
            } else {
                vec![]
            },
        ));
        let mut builder = Runtime::builder()
            .with_default_model("registry-test")
            .with_plugin(ProviderModule::new(MockProvider::new(vec![])))
            .with_plugin(owned);
        if other_owner {
            builder = builder
                .with_governance(Arc::new(AllowAll))
                .with_plugin(Arc::new(TestModule::new(
                    "module.attacker",
                    vec![],
                    vec![(
                        ModuleHookV1::BeforeModelCall,
                        Callback::Calls(vec![auto("protected", TaskId::new())]),
                    )],
                )));
        }
        let runtime = builder.build().await.unwrap();
        runtime
            .execute(TurnRequest::new(SessionId::new(), "hello"))
            .await
            .unwrap();
        assert!(tool.seen.lock().unwrap().is_empty());
    }
}

struct ApprovalPolicy;
#[async_trait]
impl GovernanceHook for ApprovalPolicy {
    fn name(&self) -> &str {
        "fixture_approval"
    }
    async fn evaluate(&self, req: &GovernanceRequest<'_>) -> Decision {
        match req.action {
            Action::Completion { .. } => Decision::Allow,
            _ => Decision::RequireApproval {
                reason: "fixture approval".into(),
            },
        }
    }
}

#[tokio::test]
async fn legacy_adapter_preserves_frozen_payload_and_session_on_approval_resume() {
    let tool = TestTool::new("legacy");
    let provider = MockProvider::new(vec![vec![call("legacy")]]);
    let id = SessionId::new();
    let runtime = Runtime::builder()
        .with_default_model("registry-test")
        .with_governance(Arc::new(ApprovalPolicy))
        .with_tool(tool.clone())
        .with_plugin(ProviderModule::new(provider))
        .build()
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(pending) = runtime
        .execute_outcome(TurnRequest::new(id, "hello"))
        .await
        .unwrap()
    else {
        panic!("must pause")
    };
    let result = runtime
        .resolve_approval(id, pending.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(matches!(
        result,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));
    let seen = tool.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].2, Some(id));
    assert_eq!(
        seen[0].1.as_ref().unwrap().payload,
        json!({"bound":"original"})
    );
}

#[tokio::test]
async fn automatic_approval_is_a_tool_error_without_hidden_pending_state() {
    let tool = TestTool::new("approval");
    let id = SessionId::new();
    let module = Arc::new(TestModule::new(
        "module.approval",
        vec![tool.clone()],
        vec![(
            ModuleHookV1::BeforeModelCall,
            Callback::Calls(vec![auto("approval", TaskId::new())]),
        )],
    ));
    let runtime = Runtime::builder()
        .with_default_model("registry-test")
        .with_governance(Arc::new(ApprovalPolicy))
        .with_plugin(module)
        .with_plugin(ProviderModule::new(MockProvider::new(vec![])))
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(id, "hello"))
        .await
        .unwrap();
    assert!(tool.seen.lock().unwrap().is_empty());
    assert!(runtime.pending_approvals(id).await.unwrap().is_empty());
}

#[tokio::test]
async fn tool_timeout_cancellation_panic_correlation_and_result_limits_share_adapter() {
    for (mode, code, cancel) in [
        (ToolMode::Pending(Arc::new(Notify::new())), "timeout", false),
        (ToolMode::Pending(Arc::new(Notify::new())), "stopping", true),
        (ToolMode::Panic, "panicked", false),
        (ToolMode::WrongId, "invalid_result", false),
        (ToolMode::Large, "result_limit", false),
    ] {
        let entered = match &mode {
            ToolMode::Pending(n) => Some(n.clone()),
            _ => None,
        };
        let tool = TestTool::mode("bounded", mode);
        let module = Arc::new(TestModule::new("module.bounded", vec![tool], vec![]));
        let limits = DispatchLimits {
            timeout: if cancel {
                Duration::from_secs(3)
            } else {
                Duration::from_millis(30)
            },
            max_result_bytes: 512,
            ..Default::default()
        };
        let provider = MockProvider::new(vec![vec![call("bounded")]]);
        let runtime = Arc::new(runtime(vec![module], provider.clone(), limits).await);
        let task_runtime = runtime.clone();
        let task = tokio::spawn(async move {
            task_runtime
                .execute(TurnRequest::new(SessionId::new(), "hello"))
                .await
        });
        if cancel {
            entered.unwrap().notified().await;
            runtime.cancel_module(&pid("module.bounded")).unwrap();
        }
        task.await.unwrap().unwrap();
        let requests = provider.requests.lock().unwrap();
        assert!(
            ContentPart::join_text(&requests[1].messages.last().unwrap().content).contains(code)
        );
        runtime
            .plugins()
            .task_scope(&pid("module.bounded"))
            .unwrap()
            .close_and_wait()
            .await;
    }
}

#[tokio::test]
async fn task_scope_cancels_owned_background_work_before_shutdown_and_unregister() {
    let mut registry = PluginManager::new();
    registry
        .register(Arc::new(TestModule::new("module.task", vec![], vec![])))
        .unwrap();
    registry.start_all(&context()).await.unwrap();
    let scope = registry.task_scope(&pid("module.task")).unwrap();
    let task_scope = scope.clone();
    let entered = Arc::new(Notify::new());
    let signal = entered.clone();
    let task = tokio::spawn(async move {
        task_scope
            .run(async move {
                signal.notify_one();
                std::future::pending::<()>().await
            })
            .await
    });
    entered.notified().await;
    registry.stop(&pid("module.task")).await.unwrap();
    assert!(task.await.unwrap().is_err());
    assert_eq!(scope.active_tasks(), 0);
    assert!(scope.run(async {}).await.is_err());
    registry.unregister(&pid("module.task")).unwrap();
}

#[tokio::test]
async fn session_start_is_once_idle_is_per_turn_and_session_end_is_explicit() {
    let points = [
        ModuleHookV1::SessionStart,
        ModuleHookV1::BeforeModelCall,
        ModuleHookV1::AfterModelCall,
        ModuleHookV1::OnIdle,
        ModuleHookV1::SessionEnd,
    ];
    let module = Arc::new(TestModule::new(
        "module.session",
        vec![],
        points
            .iter()
            .map(|p| (*p, Callback::Calls(vec![])))
            .collect(),
    ));
    let runtime = runtime(
        vec![module],
        MockProvider::new(vec![]),
        DispatchLimits::default(),
    )
    .await;
    let id = SessionId::new();
    for _ in 0..2 {
        runtime
            .execute(TurnRequest::new(id, "hello"))
            .await
            .unwrap();
    }
    runtime.end_session(id, None).await.unwrap();
    runtime.end_session(id, None).await.unwrap();
    assert!(runtime
        .execute(TurnRequest::new(id, "closed"))
        .await
        .is_err());
    let session = runtime.sessions().load(&id).await.unwrap().unwrap();
    let hooks = session
        .events
        .iter()
        .filter_map(|e| match e.event {
            SessionEventKind::RegisteredHook { hook, .. } => Some(hook),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        hooks,
        vec![
            points[0], points[1], points[2], points[3], points[1], points[2], points[3], points[4]
        ]
    );
}

#[tokio::test]
async fn runtime_registration_stop_and_unregister_use_the_existing_manager() {
    let mut runtime = runtime(vec![], MockProvider::new(vec![]), DispatchLimits::default()).await;
    let tool = TestTool::new("hot_tool");
    let module = Arc::new(TestModule::new("module.hot", vec![tool], vec![]));
    runtime.register_module(module, &context()).await.unwrap();
    assert!(runtime
        .tools()
        .iter()
        .any(|t| t.declaration().name == "hot_tool"));
    runtime.stop_module(&pid("module.hot")).await.unwrap();
    assert!(runtime.tools().is_empty());
    runtime.unregister_module(&pid("module.hot")).unwrap();
    assert!(runtime.plugins().state(&pid("module.hot")).is_err());
}

#[tokio::test]
async fn kernel_has_no_shell_without_registration_even_if_model_requests_it() {
    let provider = MockProvider::new(vec![vec![call("shell")]]);
    let runtime = runtime(vec![], provider.clone(), DispatchLimits::default()).await;
    assert!(runtime.tool_declarations().is_empty());
    runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    let requests = provider.requests.lock().unwrap();
    assert!(
        ContentPart::join_text(&requests[1].messages.last().unwrap().content)
            .contains("no tool named")
    );
}

#[tokio::test]
async fn conflicting_execution_identity_and_hook_call_limit_reject_before_effects() {
    for conflict in [true, false] {
        let tool = TestTool::new("atomic");
        let execution = TaskId::new();
        let first = auto("atomic", execution);
        let mut second = first.clone();
        if conflict {
            second.call.arguments = json!({"value":99});
        }
        let module = Arc::new(TestModule::new(
            "module.atomic",
            vec![tool.clone()],
            vec![(
                ModuleHookV1::BeforeModelCall,
                Callback::Calls(vec![first, second]),
            )],
        ));
        let limits = DispatchLimits {
            max_hook_calls: if conflict { 16 } else { 1 },
            ..Default::default()
        };
        let runtime = runtime(vec![module], MockProvider::new(vec![]), limits).await;
        let id = SessionId::new();
        runtime
            .execute(TurnRequest::new(id, "hello"))
            .await
            .unwrap();
        assert!(tool.seen.lock().unwrap().is_empty());
        let expected = if conflict {
            "invalid_batch"
        } else {
            "call_limit"
        };
        let session = runtime.sessions().load(&id).await.unwrap().unwrap();
        assert!(session.events.iter().any(
            |e| matches!(&e.event,SessionEventKind::RegisteredHook {status,..} if status==expected)
        ));
    }
}

#[tokio::test]
async fn one_failed_hook_does_not_skip_a_healthy_module() {
    let tool = TestTool::new("healthy");
    let failing = Arc::new(TestModule::new(
        "module.a",
        vec![],
        vec![(ModuleHookV1::BeforeModelCall, Callback::Fail)],
    ));
    let healthy = Arc::new(TestModule::new(
        "module.b",
        vec![tool.clone()],
        vec![(
            ModuleHookV1::BeforeModelCall,
            Callback::Calls(vec![auto("healthy", TaskId::new())]),
        )],
    ));
    let runtime = runtime(
        vec![failing, healthy],
        MockProvider::new(vec![]),
        DispatchLimits::default(),
    )
    .await;
    runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    assert_eq!(tool.seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn automatic_after_tool_work_is_a_barrier_for_the_parallel_scheduler() {
    use apeireth_orchestration::call_scheduler::{
        CallScheduler, SafetyRule, SafetyWhitelist, SchedulerConfig,
    };
    let tool = TestTool::new("parallel");
    let helper = TestTool::new("parallel_helper");
    let module = Arc::new(TestModule::new(
        "module.parallel",
        vec![tool, helper.clone()],
        vec![(
            ModuleHookV1::AfterToolCall,
            Callback::Calls(vec![auto("parallel_helper", TaskId::new())]),
        )],
    ));
    let mut second = call("parallel");
    second.id = "parallel_second".into();
    let provider = MockProvider::new(vec![vec![call("parallel"), second]]);
    let scheduler = Arc::new(
        CallScheduler::new(
            Arc::new(SafetyWhitelist::new().allow(SafetyRule::for_tool("parallel"))),
            SchedulerConfig {
                max_parallel: 2,
                wind_down: Duration::from_millis(200),
            },
        )
        .unwrap(),
    );
    let runtime = Runtime::builder()
        .with_default_model("registry-test")
        .with_governance(Arc::new(AllowAll))
        .with_plugin(module)
        .with_plugin(ProviderModule::new(provider.clone()))
        .with_call_scheduler(scheduler)
        .build()
        .await
        .unwrap();
    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    let stages = response
        .trace
        .entries
        .iter()
        .filter_map(|entry| match &entry.event {
            apeireth_runtime::canonical::TraceEvent::CapabilityDispatched {
                capability, ..
            } if capability.as_str() == "tool.parallel" => Some("dispatch"),
            apeireth_runtime::canonical::TraceEvent::CapabilityCompleted { capability, .. }
                if capability.as_str() == "tool.parallel" =>
            {
                Some("complete")
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(stages, vec!["dispatch", "complete", "dispatch", "complete"]);

    assert_eq!(helper.seen.lock().unwrap().len(), 1);
    assert_eq!(provider.requests.lock().unwrap().len(), 2);
}

struct OfflineSummary;
#[async_trait]
impl apeireth_orchestration::compaction_checkpoint::SummaryGenerator for OfflineSummary {
    async fn generate(
        &self,
        _: &apeireth_orchestration::compaction_checkpoint::SummaryRequest,
    ) -> Result<String, apeireth_orchestration::compaction_checkpoint::SummaryError> {
        Err(
            apeireth_orchestration::compaction_checkpoint::SummaryError::Unavailable(
                "fixture".into(),
            ),
        )
    }
}

#[tokio::test]
async fn before_compaction_fires_at_an_actual_due_checkpoint() {
    let module = Arc::new(TestModule::new(
        "module.compact",
        vec![],
        vec![(ModuleHookV1::BeforeCompaction, Callback::Calls(vec![]))],
    ));
    let runtime = Runtime::builder()
        .with_default_model("registry-test")
        .with_governance(Arc::new(AllowAll))
        .with_plugin(module)
        .with_plugin(ProviderModule::new(MockProvider::new(vec![])))
        .with_summary_generator(Arc::new(OfflineSummary))
        .build()
        .await
        .unwrap();
    let id = SessionId::new();
    runtime
        .execute(TurnRequest::new(id, "x".repeat(50_000)))
        .await
        .unwrap();
    let session = runtime.sessions().load(&id).await.unwrap().unwrap();
    assert!(session.events.iter().any(|e|matches!(&e.event,SessionEventKind::RegisteredHook {hook:ModuleHookV1::BeforeCompaction,status,..} if status=="completed")));
}

#[tokio::test]
async fn cached_provider_handle_is_cancelled_with_its_module() {
    let provider = MockProvider::new(vec![]);
    let mut runtime = runtime(vec![], provider.clone(), DispatchLimits::default()).await;
    let cached = runtime.plugins().active_providers().pop().unwrap();
    assert!(runtime.shutdown().await.is_empty());
    assert!(cached
        .complete(&NormalizedRequest::new(
            "registry-test",
            vec![NormalizedMessage::user("hello")]
        ))
        .await
        .is_err());
    assert!(provider.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn module_output_budget_keeps_a_correlated_error_after_execution() {
    let tool = TestTool::mode("big_event", ToolMode::LargeEvent);
    let module = Arc::new(TestModule::new(
        "module.big",
        vec![tool.clone()],
        vec![(
            ModuleHookV1::BeforeModelCall,
            Callback::Calls(vec![auto("big_event", TaskId::new())]),
        )],
    ));
    let provider = MockProvider::new(vec![]);
    let runtime = runtime(vec![module], provider.clone(), DispatchLimits::default()).await;
    runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    assert_eq!(tool.seen.lock().unwrap().len(), 1);
    let requests = provider.requests.lock().unwrap();
    let result = requests[0]
        .messages
        .iter()
        .find(|m| m.role == MessageRole::Tool)
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&ContentPart::join_text(&result.content)).unwrap();
    assert_eq!(payload["payload"]["status"], "error");
    assert_eq!(payload["payload"]["retryable"], false);
}

#[tokio::test]
async fn pending_event_count_stops_new_effects_and_records_the_limit() {
    let tool = TestTool::new("many_events");
    let module = Arc::new(TestModule::new(
        "module.many",
        vec![tool.clone()],
        vec![(
            ModuleHookV1::BeforeModelCall,
            Callback::Calls(
                (0..65)
                    .map(|_| auto("many_events", TaskId::new()))
                    .collect(),
            ),
        )],
    ));
    let runtime = runtime(
        vec![module],
        MockProvider::new(vec![]),
        DispatchLimits {
            max_hook_calls: 128,
            ..Default::default()
        },
    )
    .await;
    let id = SessionId::new();
    runtime
        .execute(TurnRequest::new(id, "hello"))
        .await
        .unwrap();
    assert_eq!(tool.seen.lock().unwrap().len(), 64);
    let session = runtime.sessions().load(&id).await.unwrap().unwrap();
    assert!(session.events.iter().any(
        |e| matches!(&e.event,SessionEventKind::RegisteredHook {status,..} if status=="event_limit")
    ));
}

struct LegacyStartRetry(apeireth_runtime::canonical::ModuleManifest);
#[async_trait]
impl apeireth_runtime::canonical::BehaviorModule for LegacyStartRetry {
    fn manifest(&self) -> &apeireth_runtime::canonical::ModuleManifest {
        &self.0
    }
    async fn on_hook(
        &self,
        hook: apeireth_runtime::canonical::HookPoint,
        _: &apeireth_runtime::canonical::ModuleContext<'_>,
    ) -> Result<apeireth_runtime::canonical::ModuleOutcome, apeireth_runtime::canonical::ModuleError>
    {
        use apeireth_runtime::canonical::{HookPoint, ModuleOutcome};
        Ok(if hook == HookPoint::TurnStart {
            ModuleOutcome::retry("legacy retry feedback")
        } else {
            ModuleOutcome::continue_()
        })
    }
}

#[tokio::test]
async fn session_start_precedes_model_when_legacy_turn_start_requests_retry() {
    use apeireth_runtime::canonical::ModuleManifest;
    let module = Arc::new(TestModule::new(
        "module.start_retry",
        vec![],
        vec![
            (ModuleHookV1::SessionStart, Callback::Calls(vec![])),
            (ModuleHookV1::BeforeModelCall, Callback::Calls(vec![])),
        ],
    ));
    let runtime = Runtime::builder()
        .with_default_model("registry-test")
        .with_governance(Arc::new(AllowAll))
        .with_module(Arc::new(LegacyStartRetry(ModuleManifest::new(
            "legacy.retry",
            "legacy retry",
        ))))
        .with_plugin(module)
        .with_plugin(ProviderModule::new(MockProvider::new(vec![])))
        .build()
        .await
        .unwrap();
    let id = SessionId::new();
    runtime
        .execute(TurnRequest::new(id, "hello"))
        .await
        .unwrap();
    let session = runtime.sessions().load(&id).await.unwrap().unwrap();
    let points = session
        .events
        .iter()
        .filter_map(|e| match e.event {
            SessionEventKind::RegisteredHook {
                ref module, hook, ..
            } if module == &pid("module.start_retry") => Some(hook),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        points,
        vec![ModuleHookV1::SessionStart, ModuleHookV1::BeforeModelCall]
    );
}
