//! Real Runtime capability ownership, native cleanup and protocol integration.
use apeireth_core::kernel::{AgentId, CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::{AllowAll, DenyCapabilities};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCaller, ToolCapability, ToolConcurrency, ToolExecutionContext,
    ToolExecutionPolicy, ToolModule,
};
use apeireth_protocol::{
    adapters::{
        AnthropicMessagesAdapter, GeminiAdapter, OpenAiChatAdapter, OpenAiResponsesAdapter,
    },
    canonical::{
        validate_closed_tool_context, ContentPart, ModelDescriptor, ModelFeature,
        NormalizedRequest, NormalizedResponse, NormalizedTool, ToolCall, ToolEventSourceV1,
        ToolResult,
    },
    ProtocolAdapter,
};
use apeireth_runtime::canonical::{
    dispatcher::{DispatchLimits, ToolScope},
    CapabilityProvider, Runtime, RuntimeBuilder, SessionEventKind, TurnRequest,
};
use apeireth_runtime_assembly::{FilesystemModule, ProcessModule, ShellModule};
use apeireth_tools_canonical::{external::ExifTool, TrustedShellConfig};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{BTreeSet, VecDeque};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tempfile::{tempdir, TempDir};

fn pid(id: &str) -> PluginId {
    PluginId::new(id).unwrap()
}
fn call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_fixture".into(),
        name: name.into(),
        arguments,
    }
}
struct Provider {
    id: CapabilityId,
    script: Mutex<VecDeque<Vec<ToolCall>>>,
    requests: Mutex<Vec<NormalizedRequest>>,
}
impl Provider {
    fn new(script: Vec<Vec<ToolCall>>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.capability_fixture").unwrap(),
            script: Mutex::new(script.into()),
            requests: Mutex::new(vec![]),
        })
    }
    fn results(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .flat_map(|request| request.messages.iter())
            .filter(|message| message.role == apeireth_protocol::canonical::MessageRole::Tool)
            .map(|message| ContentPart::join_text(&message.content))
            .collect()
    }
}
#[async_trait]
impl ProviderCapability for Provider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new("fixture").unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        validate_closed_tool_context(&request.messages)
            .expect("real Runtime history must be closed");
        let mut wire = request.clone();
        wire.max_tokens = Some(128);
        for adapter in [
            Box::new(OpenAiChatAdapter::new()) as Box<dyn ProtocolAdapter>,
            Box::new(OpenAiResponsesAdapter::new()),
            Box::new(AnthropicMessagesAdapter::new()),
            Box::new(GeminiAdapter::new()),
        ] {
            adapter
                .adapt_request(&wire)
                .expect("actual Runtime messages must project legally");
        }
        self.requests.lock().unwrap().push(request.clone());
        let mut response = NormalizedResponse::text("fixture", &request.model, "done");
        response.tool_calls = self.script.lock().unwrap().pop_front().unwrap_or_default();
        Ok(response)
    }
}
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<Provider>,
}
#[async_trait]
impl Plugin for ProviderPlugin {
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
fn builder(provider: Arc<Provider>) -> RuntimeBuilder {
    let manifest = PluginManifest::new(pid("provider.fixture"), "1.0.0", "fixture")
        .declare_capability(provider.id.clone(), CapabilityKind::Provider, "fixture")
        .unwrap();
    Runtime::builder()
        .with_default_model("fixture")
        .with_governance(Arc::new(AllowAll))
        .with_plugin(Arc::new(ProviderPlugin { manifest, provider }))
}
async fn facts(
    runtime: &Runtime,
    session: SessionId,
) -> Vec<apeireth_protocol::canonical::ToolEventV1> {
    runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .unwrap()
        .events
        .iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::ToolEventRecorded { fact } => Some(fact.clone()),
            _ => None,
        })
        .collect()
}
fn shell_config(root: &TempDir) -> TrustedShellConfig {
    // Explicit host opt-out only for harmless offline fixtures on platforms
    // without the desktop sandbox. Product sandbox defaults stay enabled.
    TrustedShellConfig::new(root.path()).with_sandbox(false)
}
fn harmless_shell() -> &'static str {
    if cfg!(windows) {
        "echo capability_fixture"
    } else {
        "printf capability_fixture"
    }
}

#[tokio::test]
async fn minimal_kernel_chats_without_any_business_module_or_tool() {
    let provider = Provider::new(vec![]);
    let runtime = builder(provider.clone()).build().await.unwrap();
    assert!(runtime.tools().is_empty());
    assert!(runtime.tool_declarations().is_empty());
    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    assert_eq!(response.text, "done");
    assert!(provider.requests.lock().unwrap()[0].tools.is_empty());
}

#[tokio::test]
async fn unregistered_shell_returns_paired_structured_error_without_fallback() {
    let root = tempdir().unwrap();
    let marker = root.path().join("should_not_exist");
    let command = if cfg!(windows) {
        format!("echo forbidden > \"{}\"", marker.display())
    } else {
        format!("printf forbidden > '{}'", marker.display())
    };
    let provider = Provider::new(vec![vec![call("shell", json!({"command":command}))]]);
    let runtime = builder(provider.clone()).build().await.unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "try missing shell"))
        .await
        .unwrap();
    let results = provider.results();
    assert_eq!(
        serde_json::from_str::<Value>(
            results[0]
                .strip_prefix("error: ")
                .expect("legacy error rendering prefix")
        )
        .unwrap()["code"],
        "unregistered"
    );
    assert!(!root.path().join("should_not_exist").exists());
    assert_eq!(provider.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn shell_module_executes_once_through_registry_context_and_v1_projection() {
    let root = tempdir().unwrap();
    let provider = Provider::new(vec![vec![call(
        "shell",
        json!({"command":harmless_shell()}),
    )]]);
    let runtime = builder(provider.clone())
        .with_tool_module(&ShellModule::new(shell_config(&root)))
        .unwrap()
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    assert_eq!(runtime.tools().len(), 1);
    assert_eq!(
        runtime.capability_registry().len(),
        1,
        "plugin and host tools share one executable catalog"
    );
    runtime
        .execute(TurnRequest::new(session, "run harmless fixture"))
        .await
        .unwrap();
    let events = facts(&runtime, session).await;
    assert_eq!(events.len(), 1);
    assert!(events[0].result().is_ok(), "{:?}", events[0].result());
    assert!(events[0].result().render().contains("capability_fixture"));
    assert!(
        matches!(events[0].source(),ToolEventSourceV1::Model{module_id:Some(owner)} if owner==&pid("module.tool.shell"))
    );
    assert_eq!(provider.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn stopped_and_unregistered_shell_rejects_cached_execution_handles() {
    let root = tempdir().unwrap();
    let provider = Provider::new(vec![vec![call(
        "shell",
        json!({"command":harmless_shell()}),
    )]]);
    let mut runtime = builder(provider.clone())
        .with_tool_module(&ShellModule::new(shell_config(&root)))
        .unwrap()
        .build()
        .await
        .unwrap();
    let cached = runtime.tools().pop().unwrap();
    runtime
        .stop_module(&pid("module.tool.shell"))
        .await
        .unwrap();
    runtime
        .stop_module(&pid("module.tool.shell"))
        .await
        .unwrap();
    assert!(runtime.tools().is_empty());
    let result = cached
        .invoke(&call("shell", json!({"command":harmless_shell()})))
        .await;
    assert!(!result.is_ok());
    assert!(result.render().contains("stopping"));
    runtime
        .unregister_module(&pid("module.tool.shell"))
        .unwrap();
    assert!(runtime
        .unregister_module(&pid("module.tool.shell"))
        .is_err());
    runtime
        .execute(TurnRequest::new(SessionId::new(), "missing after unload"))
        .await
        .unwrap();
    assert!(provider.results()[0].contains("unregistered"));
}

#[tokio::test]
async fn shell_permission_denial_and_scope_denial_do_not_execute() {
    let root = tempdir().unwrap();
    for deny_by_scope in [false, true] {
        let provider = Provider::new(vec![vec![call(
            "shell",
            json!({"command":harmless_shell()}),
        )]]);
        let mut build = builder(provider.clone())
            .with_tool_module(&ShellModule::new(shell_config(&root)))
            .unwrap();
        if !deny_by_scope {
            build = build.with_governance(Arc::new(
                DenyCapabilities::new().deny(CapabilityId::new("tool.shell").unwrap()),
            ));
        }
        let runtime = build.build().await.unwrap();
        let session = SessionId::new();
        if deny_by_scope {
            runtime
                .tool_dispatcher()
                .set_scope(session, ToolScope::Only(BTreeSet::new()));
        }
        runtime
            .execute(TurnRequest::new(session, "denied fixture"))
            .await
            .unwrap();
        let events = facts(&runtime, session).await;
        assert_eq!(events.len(), 1);
        assert!(!events[0].result().is_ok());
        assert!(!events[0].result().render().contains("stdout"));
        if deny_by_scope {
            assert!(runtime.tool_declarations_for_session(session).is_empty());
        }
    }
}

#[tokio::test]
async fn shell_output_is_bounded_and_reports_truncation() {
    let root = tempdir().unwrap();
    let provider = Provider::new(vec![vec![call(
        "shell",
        json!({"command":harmless_shell()}),
    )]]);
    let module = ShellModule::new(shell_config(&root).with_output_bounds(4, 4));
    let runtime = builder(provider)
        .with_tool_module(&module)
        .unwrap()
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "bounded output"))
        .await
        .unwrap();
    let events = facts(&runtime, session).await;
    assert!(events[0].result().is_ok());
    let output: Value = serde_json::from_str(&events[0].result().render()).unwrap();
    assert_eq!(output["stdout"].as_str().unwrap().len(), 4);
    assert_eq!(output["stdout_truncated"], true);
}

#[tokio::test]
async fn filesystem_is_independently_registered_and_schema_validated() {
    let root = tempdir().unwrap();
    std::fs::write(root.path().join("fixture.txt"), "owned file").unwrap();
    let provider = Provider::new(vec![
        vec![call(
            "filesystem",
            json!({"operation":"read","path":"fixture.txt"}),
        )],
        vec![call(
            "filesystem",
            json!({"operation":17,"path":"fixture.txt"}),
        )],
    ]);
    let runtime = builder(provider.clone())
        .with_tool_module(&FilesystemModule::new(root.path()))
        .unwrap()
        .build()
        .await
        .unwrap();
    assert_eq!(runtime.tools().len(), 1);
    assert_eq!(runtime.tools()[0].declaration().name, "filesystem");
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "read"))
        .await
        .unwrap();
    let events = facts(&runtime, session).await;
    assert_eq!(events.len(), 2);
    assert!(events[0].result().render().contains("owned file"));
    assert!(events[1].result().render().contains("invalid_arguments"));
    assert!(events.iter().all(|event| matches!(event.source(),ToolEventSourceV1::Model{module_id:Some(owner)} if owner==&pid("module.tool.filesystem"))));
}

struct ContextTool {
    id: CapabilityId,
    name: String,
    policy: ToolExecutionPolicy,
    contexts: Mutex<Vec<ToolExecutionContext>>,
    active: AtomicUsize,
    maximum: AtomicUsize,
    barrier: Option<Arc<tokio::sync::Barrier>>,
}
impl ContextTool {
    fn new(policy: ToolExecutionPolicy) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("tool.context_fixture").unwrap(),
            name: "context_fixture".into(),
            policy,
            contexts: Mutex::new(vec![]),
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
            barrier: None,
        })
    }
}
#[async_trait]
impl ToolCapability for ContextTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(&self.name).with_parameters(
        json!({"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}).as_object().unwrap().clone())
    }
    fn execution_policy(&self) -> ToolExecutionPolicy {
        self.policy.clone()
    }
    async fn invoke(&self, _: &ToolCall) -> ToolResult {
        panic!("canonical context was lost")
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&apeireth_plugin::FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        self.contexts.lock().unwrap().push(context.clone());
        assert_eq!(
            apeireth_plugin::tool_execution::native_execution_context()
                .unwrap()
                .deadline,
            context.deadline
        );
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.maximum.fetch_max(active, Ordering::SeqCst);
        if let Some(barrier) = &self.barrier {
            tokio::time::timeout(Duration::from_secs(3), barrier.wait())
                .await
                .expect("independent sessions must execute tools concurrently");
        }
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert_eq!(
            apeireth_plugin::tool_execution::native_execution_context()
                .unwrap()
                .deadline,
            context.deadline
        );
        self.active.fetch_sub(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone()).with_name(&call.name)
    }
}

#[tokio::test]
async fn agent_call_contexts_are_isolated_and_tool_concurrency_is_local() {
    for (policy, expected) in [(ToolConcurrency::Serial, 1), (ToolConcurrency::Parallel, 2)] {
        let mut tool = ContextTool::new(ToolExecutionPolicy {
            concurrency: policy,
            ..Default::default()
        });
        if policy == ToolConcurrency::Parallel {
            Arc::get_mut(&mut tool).unwrap().barrier = Some(Arc::new(tokio::sync::Barrier::new(2)));
        }
        let module = ToolModule::new(pid("module.context"), vec![tool.clone()]).unwrap();
        let provider = Provider::new(vec![
            vec![call("context_fixture", json!({"value":1}))],
            vec![call("context_fixture", json!({"value":2}))],
        ]);
        let runtime = builder(provider)
            .with_plugin(Arc::new(module))
            .build()
            .await
            .unwrap();
        let a = SessionId::new();
        let b = SessionId::new();
        let agent_a = AgentId::new();
        let agent_b = AgentId::new();
        runtime.tool_dispatcher().set_agent(a, agent_a);
        runtime.tool_dispatcher().set_agent(b, agent_b);
        let (left, right) = tokio::join!(
            runtime.execute(TurnRequest::new(a, "one")),
            runtime.execute(TurnRequest::new(b, "two"))
        );
        left.unwrap();
        right.unwrap();
        assert_eq!(tool.maximum.load(Ordering::SeqCst), expected);
        let contexts = tool.contexts.lock().unwrap();
        assert_eq!(contexts.len(), 2);
        assert!(contexts
            .iter()
            .any(|c| c.session == Some(a) && c.agent == agent_a));
        assert!(contexts
            .iter()
            .any(|c| c.session == Some(b) && c.agent == agent_b));
        assert!(contexts.iter().all(|c| c.turn.is_some()
            && c.trace.is_some()
            && c.caller == ToolCaller::Model
            && c.module == pid("module.context")));
        assert_ne!(contexts[0].turn, contexts[1].turn);
    }
}

#[tokio::test]
async fn catalog_rejects_dynamic_shadowing_and_cancels_unregistered_legacy_handles() {
    let tool = ContextTool::new(Default::default());
    let runtime = builder(Provider::new(vec![]))
        .with_plugin(Arc::new(
            ToolModule::new(pid("module.context"), vec![tool.clone()]).unwrap(),
        ))
        .build()
        .await
        .unwrap();
    assert!(runtime
        .capability_registry()
        .register("dynamic.fixture", tool.clone())
        .is_err());
    let dynamic = ContextTool::new(Default::default());
    let runtime = builder(Provider::new(vec![])).build().await.unwrap();
    runtime
        .capability_registry()
        .register("dynamic.fixture", dynamic.clone())
        .unwrap();
    let cached = runtime.tools().pop().unwrap();
    assert!(
        runtime
            .capability_registry()
            .unregister_and_drain(dynamic.id())
            .await
    );
    let result = cached
        .invoke(&call("context_fixture", json!({"value":3})))
        .await;
    assert!(!result.is_ok());
    assert!(dynamic.contexts.lock().unwrap().is_empty());
}

#[tokio::test]
async fn registered_output_schema_failure_is_a_correlated_tool_error() {
    let tool = ContextTool::new(ToolExecutionPolicy {
        output_schema: Some(json!({"type":"integer"})),
        ..Default::default()
    });
    let provider = Provider::new(vec![vec![call("context_fixture", json!({"value":1}))]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            ToolModule::new(pid("module.context"), vec![tool]).unwrap(),
        ))
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "bad output"))
        .await
        .unwrap();
    assert!(provider.results()[0].contains("invalid_result"));
}

#[cfg(unix)]
fn fake_process(root: &TempDir, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = root.path().join("fixture-program");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[cfg(unix)]
#[tokio::test]
async fn process_module_executes_existing_structured_adapter_without_shell_module() {
    let root = tempdir().unwrap();
    std::fs::write(root.path().join("fixture.bin"), "fixture").unwrap();
    let binary = fake_process(&root, "printf '[{\"FileName\":\"fixture.bin\"}]'");
    let tool = Arc::new(ExifTool::new(root.path()).with_binary(binary));
    let module = ProcessModule::with_capabilities(vec![tool]);
    let provider = Provider::new(vec![vec![call(
        "ext.exiftool",
        json!({"path":"fixture.bin"}),
    )]]);
    let mut runtime = builder(provider)
        .with_tool_module(&module)
        .unwrap()
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "metadata"))
        .await
        .unwrap();
    let events = facts(&runtime, session).await;
    assert!(events[0].result().is_ok(), "{:?}", events[0].result());
    assert!(
        matches!(events[0].source(),ToolEventSourceV1::Model{module_id:Some(owner)} if owner==&pid("module.tool.process"))
    );
    assert!(!runtime
        .tool_declarations()
        .iter()
        .any(|tool| tool.name == "shell"));
    runtime
        .stop_module(&pid("module.tool.process"))
        .await
        .unwrap();
    runtime
        .unregister_module(&pid("module.tool.process"))
        .unwrap();
    assert!(runtime.tools().is_empty());
}

#[cfg(target_os = "linux")]
async fn wait_for_pid(path: &std::path::Path) -> u32 {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(pid) = text.trim().parse() {
                    return pid;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("fixture child starts")
}
#[cfg(target_os = "linux")]
async fn assert_dead(pid: u32) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                Err(_) => return,
                Ok(stat)
                    if stat
                        .rsplit(')')
                        .next()
                        .unwrap()
                        .trim_start()
                        .starts_with('Z') =>
                {
                    return
                }
                _ => tokio::time::sleep(Duration::from_millis(5)).await,
            }
        }
    })
    .await
    .expect("contained process must be dead after module drain");
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn module_cancel_and_caller_abort_reap_shell_and_descendant_before_stop_returns() {
    for abort_caller in [false, true] {
        let root = tempdir().unwrap();
        let command =
            "printf '%s' \"$$\" > parent.pid; sleep 20 & printf '%s' \"$!\" > child.pid; wait";
        let provider = Provider::new(vec![vec![call("shell", json!({"command":command}))]]);
        let runtime = Arc::new(
            builder(provider)
                .with_tool_module(&ShellModule::new(shell_config(&root).with_file_write(true)))
                .unwrap()
                .build()
                .await
                .unwrap(),
        );
        let owned = runtime.clone();
        let task = tokio::spawn(async move {
            owned
                .execute(TurnRequest::new(SessionId::new(), "cancellable shell"))
                .await
        });
        let parent = wait_for_pid(&root.path().join("parent.pid")).await;
        let child = wait_for_pid(&root.path().join("child.pid")).await;
        if abort_caller {
            task.abort();
        } else {
            runtime.cancel_module(&pid("module.tool.shell")).unwrap();
        }
        let _ = task.await;
        let mut runtime = Arc::try_unwrap(runtime).expect("turn released runtime");
        tokio::time::timeout(
            Duration::from_secs(3),
            runtime.stop_module(&pid("module.tool.shell")),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            runtime
                .plugins()
                .task_scope(&pid("module.tool.shell"))
                .unwrap()
                .active_tasks(),
            0
        );
        assert_dead(parent).await;
        assert_dead(child).await;
    }
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shell_timeout_terminates_native_tree_even_when_dispatch_deadline_wins() {
    let root = tempdir().unwrap();
    let command =
        "printf '%s' \"$$\" > parent.pid; sleep 20 & printf '%s' \"$!\" > child.pid; wait";
    let provider = Provider::new(vec![vec![call("shell", json!({"command":command}))]]);
    let mut runtime = builder(provider)
        .with_dispatch_limits(DispatchLimits {
            timeout: Duration::from_millis(150),
            ..Default::default()
        })
        .with_tool_module(&ShellModule::new(shell_config(&root).with_file_write(true)))
        .unwrap()
        .build()
        .await
        .unwrap();
    let start = Instant::now();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "bounded shell"))
        .await
        .unwrap();
    assert!(start.elapsed() < Duration::from_secs(3));
    let parent = wait_for_pid(&root.path().join("parent.pid")).await;
    let child = wait_for_pid(&root.path().join("child.pid")).await;
    runtime
        .stop_module(&pid("module.tool.shell"))
        .await
        .unwrap();
    assert_dead(parent).await;
    assert_dead(child).await;
    assert_eq!(facts(&runtime, session).await.len(), 1);
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn process_module_native_work_cancels_and_drains_with_existing_tool_abi() {
    let root = tempdir().unwrap();
    std::fs::write(root.path().join("fixture.bin"), "fixture").unwrap();
    let script = format!(
        "printf '%s' \"$$\" > '{}'; sleep 20",
        root.path().join("parent.pid").display()
    );
    let binary = fake_process(&root, &script);
    let module = ProcessModule::with_capabilities(vec![Arc::new(
        ExifTool::new(root.path()).with_binary(binary),
    )]);
    let provider = Provider::new(vec![vec![call(
        "ext.exiftool",
        json!({"path":"fixture.bin"}),
    )]]);
    let runtime = Arc::new(
        builder(provider)
            .with_tool_module(&module)
            .unwrap()
            .build()
            .await
            .unwrap(),
    );
    let owned = runtime.clone();
    let task = tokio::spawn(async move {
        owned
            .execute(TurnRequest::new(SessionId::new(), "cancellable process"))
            .await
    });
    let parent = wait_for_pid(&root.path().join("parent.pid")).await;
    runtime.cancel_module(&pid("module.tool.process")).unwrap();
    task.await.unwrap().unwrap();
    let mut runtime = Arc::try_unwrap(runtime).expect("turn released runtime");
    tokio::time::timeout(
        Duration::from_secs(3),
        runtime.stop_module(&pid("module.tool.process")),
    )
    .await
    .unwrap()
    .unwrap();
    assert_dead(parent).await;
}

#[tokio::test]
async fn duplicate_model_call_id_is_rejected_before_any_effect_and_kernel_recovers() {
    let tool = ContextTool::new(Default::default());
    let duplicate = call("context_fixture", json!({"value":1}));
    let provider = Provider::new(vec![vec![duplicate.clone(), duplicate]]);
    let runtime = builder(provider)
        .with_plugin(Arc::new(
            ToolModule::new(pid("module.context"), vec![tool.clone()]).unwrap(),
        ))
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    let failure = runtime
        .execute(TurnRequest::new(session, "duplicate identities"))
        .await
        .unwrap_err();
    assert!(failure.to_string().contains("duplicate tool call ID"));
    assert!(tool.contexts.lock().unwrap().is_empty());
    let saved = runtime.sessions().load(&session).await.unwrap().unwrap();
    validate_closed_tool_context(&saved.messages).unwrap();
    assert_eq!(
        runtime
            .execute(TurnRequest::new(session, "continue safely"))
            .await
            .unwrap()
            .text,
        "done"
    );
}

#[tokio::test]
async fn required_capabilities_are_scope_restrictions_not_implicit_grants() {
    let tool = ContextTool::new(ToolExecutionPolicy {
        required_capabilities: vec![CapabilityId::new("tool.other").unwrap()],
        ..Default::default()
    });
    let provider = Provider::new(vec![vec![call("context_fixture", json!({"value":1}))]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            ToolModule::new(pid("module.context"), vec![tool.clone()]).unwrap(),
        ))
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime.tool_dispatcher().set_scope(
        session,
        ToolScope::Only([tool.id.clone()].into_iter().collect()),
    );
    runtime
        .execute(TurnRequest::new(session, "missing required scope"))
        .await
        .unwrap();
    assert!(tool.contexts.lock().unwrap().is_empty());
    assert!(provider.results()[0].contains("scope_denied"));
}

struct HookContextModule {
    inner: ToolModule,
    execution: apeireth_core::kernel::TaskId,
}
#[async_trait]
impl Plugin for HookContextModule {
    fn manifest(&self) -> &PluginManifest {
        self.inner.manifest()
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.inner.tools()
    }
    fn hooks(&self) -> Vec<apeireth_protocol::canonical::ModuleHookV1> {
        vec![apeireth_protocol::canonical::ModuleHookV1::BeforeModelCall]
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
    async fn on_hook(
        &self,
        _: &apeireth_plugin::hooks::HookContextV1,
    ) -> PluginResult<Vec<apeireth_plugin::hooks::HookToolRequest>> {
        Ok(vec![apeireth_plugin::hooks::HookToolRequest {
            execution: self.execution,
            call: call("context_fixture", json!({"value":9})),
        }])
    }
}
#[tokio::test]
async fn automatic_hook_preserves_module_caller_authorization_and_scope_snapshot() {
    let tool = ContextTool::new(Default::default());
    let module = HookContextModule {
        inner: ToolModule::new(pid("module.context_hook"), vec![tool.clone()]).unwrap(),
        execution: apeireth_core::kernel::TaskId::new(),
    };
    let runtime = builder(Provider::new(vec![]))
        .with_plugin(Arc::new(module))
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime.tool_dispatcher().set_scope(
        session,
        ToolScope::Only([tool.id.clone()].into_iter().collect()),
    );
    let mut request = TurnRequest::new(session, "automatic fixture");
    request.security_context = Some(apeireth_governance::TurnSecurityContext::new(
        "intent_fixture",
        "trace_fixture",
    ));
    runtime.execute(request).await.unwrap();
    let contexts = tool.contexts.lock().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(
        contexts[0].caller,
        ToolCaller::Module(pid("module.context_hook"))
    );
    assert_eq!(
        contexts[0].authorization.intent_id.as_deref(),
        Some("intent_fixture")
    );
    assert_eq!(contexts[0].tool_scope, Some(vec![tool.id.clone()]));
    drop(contexts);
    assert!(
        matches!(facts(&runtime,session).await[0].source(),ToolEventSourceV1::Module{module_id,..} if module_id==&pid("module.context_hook"))
    );
}

#[tokio::test]
async fn registered_approval_policy_cannot_be_bypassed_by_allow_all_governance() {
    use apeireth_runtime::canonical::{ApprovalDecision, ApprovalResolution, TurnOutcome};
    let tool = ContextTool::new(ToolExecutionPolicy {
        permission: apeireth_plugin::ToolPermissionPolicy::RequireApproval,
        ..Default::default()
    });
    let provider = Provider::new(vec![vec![call("context_fixture", json!({"value":4}))]]);
    let runtime = builder(provider)
        .with_plugin(Arc::new(
            ToolModule::new(pid("module.context"), vec![tool.clone()]).unwrap(),
        ))
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "approval fixture"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("tool policy must require approval")
    };
    assert!(tool.contexts.lock().unwrap().is_empty());
    let resumed = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(matches!(
        resumed,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));
    let contexts = tool.contexts.lock().unwrap();
    assert_eq!(contexts.len(), 1);
    assert!(contexts[0].authorization.approved);
}

#[tokio::test]
async fn pipeline_preserves_context_and_execution_policy_through_legacy_stages() {
    let tool = ContextTool::new(ToolExecutionPolicy {
        output_schema: Some(json!({"type":"object"})),
        concurrency: ToolConcurrency::Serial,
        ..Default::default()
    });
    let pipelined = Arc::new(apeireth_tools_canonical::PipelinedCapability::new(
        tool.clone(),
        Arc::new(apeireth_tools_canonical::ToolExecutionPipeline::new()),
    ));
    let provider = Provider::new(vec![vec![call("context_fixture", json!({"value":5}))]]);
    let runtime = builder(provider)
        .with_plugin(Arc::new(
            ToolModule::new(pid("module.pipeline"), vec![pipelined]).unwrap(),
        ))
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "pipeline fixture"))
        .await
        .unwrap();
    assert!(facts(&runtime, session).await[0].result().is_ok());
    let contexts = tool.contexts.lock().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].session, Some(session));
    assert_eq!(contexts[0].module, pid("module.pipeline"));
    assert!(contexts[0].turn.is_some());
}

#[tokio::test]
async fn shell_can_be_replaced_without_modifying_the_runtime_loop() {
    use apeireth_core::kernel::{system_clock, TraceId};
    let first = tempdir().unwrap();
    let second = tempdir().unwrap();
    let command = if cfg!(windows) { "cd" } else { "pwd" };
    let provider = Provider::new(vec![vec![call("shell", json!({"command":command}))]]);
    let mut runtime = builder(provider)
        .with_tool_module(&ShellModule::new(shell_config(&first)))
        .unwrap()
        .build()
        .await
        .unwrap();
    let previous = runtime.tools().pop().unwrap();
    runtime
        .stop_module(&pid("module.tool.shell"))
        .await
        .unwrap();
    runtime
        .unregister_module(&pid("module.tool.shell"))
        .unwrap();
    let replacement = ShellModule::new(shell_config(&second))
        .tool_module()
        .unwrap();
    let context = PluginContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    runtime
        .register_module(Arc::new(replacement), &context)
        .await
        .unwrap();
    assert!(!previous
        .invoke(&call("shell", json!({"command":command})))
        .await
        .is_ok());
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "replacement fixture"))
        .await
        .unwrap();
    let events = facts(&runtime, session).await;
    assert!(events[0].result().is_ok());
    let output: Value = serde_json::from_str(&events[0].result().render()).unwrap();
    let reported = output["stdout"].as_str().unwrap().trim();
    assert_eq!(
        std::fs::canonicalize(reported).unwrap(),
        std::fs::canonicalize(second.path()).unwrap()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn process_failure_and_independent_permission_denial_leave_filesystem_healthy() {
    let root = tempdir().unwrap();
    std::fs::write(root.path().join("fixture.bin"), "fixture").unwrap();
    for denied in [false, true] {
        let binary = fake_process(&root, "printf invalid_json");
        let process = ProcessModule::with_capabilities(vec![Arc::new(
            ExifTool::new(root.path()).with_binary(binary),
        )]);
        let provider = Provider::new(vec![
            vec![call("ext.exiftool", json!({"path":"fixture.bin"}))],
            vec![call(
                "filesystem",
                json!({"operation":"read","path":"fixture.bin"}),
            )],
        ]);
        let mut build = builder(provider)
            .with_tool_module(&process)
            .unwrap()
            .with_tool_module(&FilesystemModule::new(root.path()))
            .unwrap();
        if denied {
            build = build.with_governance(Arc::new(
                DenyCapabilities::new().deny(CapabilityId::new("tool.external.exiftool").unwrap()),
            ));
        }
        let runtime = build.build().await.unwrap();
        let session = SessionId::new();
        runtime
            .execute(TurnRequest::new(session, "failure isolation fixture"))
            .await
            .unwrap();
        let events = facts(&runtime, session).await;
        assert_eq!(events.len(), 2);
        assert!(!events[0].result().is_ok());
        assert!(events[1].result().is_ok());
        if denied {
            assert!(events[0].result().render().contains("not permitted"));
        }
        assert!(events[1].result().render().contains("fixture"));
    }
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normally_completed_shell_cleans_background_descendants_and_inherited_pipes() {
    let root = tempdir().unwrap();
    let provider = Provider::new(vec![vec![call(
        "shell",
        json!({"command":"sleep 20 & printf '%s' \"$!\" > child.pid"}),
    )]]);
    let mut runtime = builder(provider)
        .with_tool_module(&ShellModule::new(shell_config(&root).with_file_write(true)))
        .unwrap()
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    tokio::time::timeout(
        Duration::from_secs(3),
        runtime.execute(TurnRequest::new(session, "background cleanup fixture")),
    )
    .await
    .unwrap()
    .unwrap();
    let child = wait_for_pid(&root.path().join("child.pid")).await;
    runtime
        .stop_module(&pid("module.tool.shell"))
        .await
        .unwrap();
    assert_dead(child).await;
    assert!(facts(&runtime, session).await[0].result().is_ok());
}

struct DynamicPair(Vec<Arc<dyn ToolCapability>>);
impl CapabilityProvider for DynamicPair {
    fn id(&self) -> &str {
        "host.dynamic.pair"
    }
    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.0.clone()
    }
}
#[tokio::test]
async fn unloading_one_legacy_dynamic_tool_does_not_cancel_its_batch_peer() {
    let first = ContextTool::new(Default::default());
    let mut second = ContextTool::new(Default::default());
    let configured = Arc::get_mut(&mut second).unwrap();
    configured.id = CapabilityId::new("tool.context_other").unwrap();
    configured.name = "context_other".into();
    let provider = Provider::new(vec![vec![call("context_other", json!({"value":8}))]]);
    let runtime = builder(provider).build().await.unwrap();
    runtime
        .capability_registry()
        .register_provider(&DynamicPair(vec![first.clone(), second.clone()]))
        .unwrap();
    let old = runtime
        .tools()
        .into_iter()
        .find(|tool| tool.id() == first.id())
        .unwrap();
    assert!(
        runtime
            .capability_registry()
            .unregister_and_drain(first.id())
            .await
    );
    assert_eq!(runtime.tools().len(), 1);
    runtime
        .execute(TurnRequest::new(SessionId::new(), "remaining peer"))
        .await
        .unwrap();
    assert_eq!(second.contexts.lock().unwrap().len(), 1);
    assert!(!old
        .invoke(&call("context_fixture", json!({"value":1})))
        .await
        .is_ok());
    assert!(first.contexts.lock().unwrap().is_empty());
}

struct NativeSerialTool {
    id: CapabilityId,
    active: Arc<AtomicUsize>,
    maximum: Arc<AtomicUsize>,
    entered: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl ToolCapability for NativeSerialTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("native_serial")
    }
    fn execution_policy(&self) -> ToolExecutionPolicy {
        ToolExecutionPolicy {
            concurrency: ToolConcurrency::Serial,
            timeout: Some(Duration::from_millis(80)),
            ..Default::default()
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let active = self.active.clone();
        let maximum = self.maximum.clone();
        let entered = self.entered.clone();
        let result = apeireth_tools_canonical::process::run_native(move || {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(count, Ordering::SeqCst);
            entered.notify_one();
            // Uninterruptible native work must drain while retaining its permit.
            std::thread::sleep(Duration::from_millis(300));
            active.fetch_sub(1, Ordering::SeqCst);
        })
        .await;
        match result {
            Ok(()) => ToolResult::ok(&call.id, json!({})),
            Err(error) => ToolResult::permanent_error(&call.id, error.to_string()),
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_serial_permit_survives_async_timeout_until_worker_has_drained() {
    let maximum = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(tokio::sync::Notify::new());
    let tool = Arc::new(NativeSerialTool {
        id: CapabilityId::new("tool.native_serial").unwrap(),
        active: active.clone(),
        maximum: maximum.clone(),
        entered: entered.clone(),
    });
    let provider = Provider::new(vec![
        vec![call("native_serial", json!({}))],
        vec![call("native_serial", json!({}))],
    ]);
    let runtime = Arc::new(
        builder(provider)
            .with_plugin(Arc::new(
                ToolModule::new(pid("module.native_serial"), vec![tool]).unwrap(),
            ))
            .build()
            .await
            .unwrap(),
    );
    let owned = runtime.clone();
    let first = tokio::spawn(async move {
        owned
            .execute(TurnRequest::new(SessionId::new(), "first native call"))
            .await
    });
    entered.notified().await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    runtime
        .execute(TurnRequest::new(SessionId::new(), "second native call"))
        .await
        .unwrap();
    first.await.unwrap().unwrap();
    let mut runtime = Arc::try_unwrap(runtime).unwrap();
    runtime
        .stop_module(&pid("module.native_serial"))
        .await
        .unwrap();
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(
        maximum.load(Ordering::SeqCst),
        1,
        "native effects must not overlap after an async timeout releases its future"
    );
}
