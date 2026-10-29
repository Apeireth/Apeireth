//! Tests proving bounded, private-transcript SubLoops owned by modules and strictly governed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::{
    HookPoint, Module, ModuleContext, ModuleManifest, ModuleOutcome, PromptOverlay, Runtime,
    SubLoopError, SubLoopResult, SubLoopSpec, TurnRequest,
};
use async_trait::async_trait;

// S3: grant just the selected internal callback; unrelated tools remain denied.
fn behavior_grant(id: &str) -> Arc<dyn GovernanceHook> {
    let mut policy = apeireth_governance::PermissionPolicy::new();
    policy.grant(apeireth_governance::Permission::ExecuteTool(format!(
        "tool.behavior.{id}"
    )));
    Arc::new(apeireth_governance::PermissionGovernanceHook::new(policy))
}

#[derive(Clone)]
struct MockInstrumentedTool {
    id: CapabilityId,
    name: String,
    invocations: Arc<AtomicUsize>,
    contexts: Arc<Mutex<Vec<apeireth_plugin::ToolExecutionContext>>>,
}

impl MockInstrumentedTool {
    fn new(id: &str, name: &str) -> (Self, Arc<AtomicUsize>) {
        let invocations = Arc::new(AtomicUsize::new(0));
        (
            Self {
                id: CapabilityId::new(id).unwrap(),
                name: name.to_string(),
                invocations: Arc::clone(&invocations),
                contexts: Arc::new(Mutex::new(vec![])),
            },
            invocations,
        )
    }
}

#[async_trait]
impl ToolCapability for MockInstrumentedTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.clone(),
            description: Some(format!("Instrumented tool {}", self.name)),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&apeireth_plugin::FrozenInvocation>,
        context: &apeireth_plugin::ToolExecutionContext,
    ) -> ToolResult {
        self.contexts.lock().unwrap().push(context.clone());
        self.invoke(call).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({ "executed": self.name }))
    }
}

#[derive(Clone)]
enum ScriptStep {
    CallTool {
        call_id: &'static str,
        tool: &'static str,
        arguments: serde_json::Value,
    },
    Say(&'static str),
    DelayAndSay(Duration, &'static str),
    Reply(NormalizedResponse),
    Fail,
}

struct ScriptedProvider {
    id: CapabilityId,
    script: Vec<ScriptStep>,
    calls: AtomicUsize,
    requests: Mutex<Vec<NormalizedRequest>>,
}

impl ScriptedProvider {
    fn new(id: &str, script: Vec<ScriptStep>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: AtomicUsize::new(0),
            requests: Mutex::new(vec![]),
        })
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new("subloop-model").unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.requests.lock().unwrap().push(request.clone());
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("called beyond script at step {index}"));

        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 5,
                completion_tokens: 5,
                total_tokens: 10,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };

        match step {
            ScriptStep::Fail => Err(ProviderError::Network {
                provider: self.id.to_string(),
                detail: "offline transient failure".into(),
            }),
            ScriptStep::Reply(response) => Ok(response.clone()),
            ScriptStep::CallTool {
                call_id,
                tool,
                arguments,
            } => Ok(NormalizedResponse {
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                tool_calls: vec![ToolCall {
                    id: (*call_id).to_string(),
                    name: (*tool).to_string(),
                    arguments: arguments.clone(),
                }],
                ..base
            }),
            ScriptStep::Say(text) => Ok(NormalizedResponse {
                content: (*text).to_string(),
                ..base
            }),
            ScriptStep::DelayAndSay(delay, text) => {
                tokio::time::sleep(*delay).await;
                Ok(NormalizedResponse {
                    content: (*text).to_string(),
                    ..base
                })
            }
        }
    }
}

struct ScriptedProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<ScriptedProvider>,
}

impl ScriptedProviderPlugin {
    fn new(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.mock").unwrap(),
                "1.0.0",
                "Mock provider plugin",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "Scripted completions",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ScriptedProviderPlugin {
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

// ---------------------------------------------------------------------------
// 1. Basic SubLoop execution on private transcript
// ---------------------------------------------------------------------------

struct BasicSubLoopModule {
    manifest: ModuleManifest,
    allowed_tool: Arc<dyn ToolCapability>,
}

impl BasicSubLoopModule {
    fn new(allowed_tool: Arc<dyn ToolCapability>) -> Self {
        Self {
            manifest: ModuleManifest::new("module.subloop.basic", "Basic SubLoop Module"),
            allowed_tool,
        }
    }
}

#[async_trait]
impl Module for BasicSubLoopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            let spec = SubLoopSpec {
                max_rounds: 3,
                allowed_capabilities: vec![self.allowed_tool.id().clone()],
                timeout: None,
                messages: vec![NormalizedMessage::user("run subtask")],
                system_prompt: Some("You are a subloop helper.".into()),
                model: Some("subloop-model".into()),
            };

            let result = ctx
                .subloop()
                .spawn(spec)
                .await
                .map_err(|e| apeireth_runtime::ModuleError::Message(e.to_string()))?;

            assert_eq!(result.text, "subloop finished successfully");
            assert_eq!(result.rounds, 2);
            assert_eq!(result.tool_results.len(), 1);
            assert!(result.tool_results[0].is_ok());

            return Ok(
                ModuleOutcome::continue_().with_prompt_overlay(PromptOverlay::system(format!(
                    "SubLoop computed: {}",
                    result.text
                ))),
            );
        }

        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn subloop_runs_on_private_transcript_with_strict_capability_allowlist() {
    let (tool, tool_invocations) = MockInstrumentedTool::new("tool.allowed", "allowed_tool");
    let script = vec![
        ScriptStep::CallTool {
            call_id: "subloop_call_1",
            tool: "allowed_tool",
            arguments: serde_json::json!({}),
        },
        ScriptStep::Say("subloop finished successfully"),
        ScriptStep::Say("Main answer complete"),
    ];

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = Arc::new(BasicSubLoopModule::new(Arc::new(tool)));

    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_capability(module.allowed_tool.clone())
        .with_module(module)
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_default_model("subloop-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let session_id = SessionId::new();
    let req = TurnRequest::new(session_id, "Hello from user").with_model("subloop-model");

    let response = runtime.execute(req).await.expect("turn executes");
    assert_eq!(response.text, "Main answer complete");
    assert_eq!(tool_invocations.load(Ordering::SeqCst), 1);

    // Verify main session transcript: does NOT have subloop private messages!
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load completes")
        .expect("session exists");
    for msg in &session.messages {
        for part in &msg.content {
            if let apeireth_protocol::canonical::ContentPart::Text { text } = part {
                assert_ne!(
                    text, "run subtask",
                    "SubLoop private prompt must not be written to main session"
                );
                assert_ne!(
                    text, "subloop finished successfully",
                    "SubLoop intermediate content must not be in main session"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Hostile provider attempting denied tool in SubLoop
// ---------------------------------------------------------------------------

struct HostileSubLoopModule {
    manifest: ModuleManifest,
    allowed_tool: Arc<dyn ToolCapability>,
    denied_tool: Arc<dyn ToolCapability>,
}

#[async_trait]
impl Module for HostileSubLoopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            let spec = SubLoopSpec {
                max_rounds: 3,
                allowed_capabilities: vec![self.allowed_tool.id().clone()], // denied_tool is NOT allowed
                timeout: None,
                messages: vec![NormalizedMessage::user("subloop prompt")],
                system_prompt: None,
                model: Some("subloop-model".into()),
            };

            let result = ctx
                .subloop()
                .spawn(spec)
                .await
                .map_err(|e| apeireth_runtime::ModuleError::Message(e.to_string()))?;

            assert_eq!(result.rounds, 2);
            assert_eq!(result.tool_results.len(), 1);
            // Tool call was rejected because it's not in allowlist
            assert!(!result.tool_results[0].is_ok());

            return Ok(ModuleOutcome::continue_());
        }

        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn hostile_provider_attempting_denied_capability_is_blocked_and_invoke_counter_is_zero() {
    let (allowed_tool, allowed_invocations) =
        MockInstrumentedTool::new("tool.allowed", "allowed_tool");
    let (denied_tool, denied_invocations) = MockInstrumentedTool::new("tool.denied", "denied_tool");

    let script = vec![
        // Hostile provider calls denied_tool despite it not being in declaration
        ScriptStep::CallTool {
            call_id: "hostile_call_1",
            tool: "denied_tool",
            arguments: serde_json::json!({}),
        },
        ScriptStep::Say("SubLoop handled denial cleanly"),
        ScriptStep::Say("Main turn completed"),
    ];

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = Arc::new(HostileSubLoopModule {
        manifest: ModuleManifest::new("module.hostile", "Hostile Tester Module"),
        allowed_tool: Arc::new(allowed_tool),
        denied_tool: Arc::new(denied_tool),
    });

    let mut runtime = Runtime::builder()
        .with_governance(behavior_grant("module.hostile"))
        .with_plugin(plugin)
        .with_capability(module.allowed_tool.clone())
        .with_capability(module.denied_tool.clone())
        .with_module(module)
        .with_default_model("subloop-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let req = TurnRequest::new(SessionId::new(), "hello").with_model("subloop-model");
    let response = runtime.execute(req).await.expect("turn executes");

    assert_eq!(response.text, "Main turn completed");
    // Invariant: denied_tool::invoke was NEVER executed
    assert_eq!(
        denied_invocations.load(Ordering::SeqCst),
        0,
        "Denied capability must never be invoked"
    );
    assert_eq!(allowed_invocations.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------------------
// 3. Global governance deny remains authoritative inside SubLoop
// ---------------------------------------------------------------------------

struct CustomDenyGovernance;

#[async_trait]
impl GovernanceHook for CustomDenyGovernance {
    fn name(&self) -> &str {
        "custom_deny"
    }

    async fn evaluate(&self, req: &GovernanceRequest<'_>) -> Decision {
        match req.action {
            Action::CapabilityDispatch { capability, .. } => {
                if capability.as_str() == "tool.globally_denied" {
                    Decision::Deny {
                        reason: "globally blocked by security policy".into(),
                    }
                } else {
                    Decision::Allow
                }
            }
            _ => Decision::Allow,
        }
    }
}

struct GovernanceSubLoopModule {
    manifest: ModuleManifest,
    tool: Arc<dyn ToolCapability>,
}

#[async_trait]
impl Module for GovernanceSubLoopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            let spec = SubLoopSpec {
                max_rounds: 3,
                allowed_capabilities: vec![self.tool.id().clone()], // SubLoop allowlist says YES
                timeout: None,
                messages: vec![NormalizedMessage::user("subloop prompt")],
                system_prompt: None,
                model: Some("subloop-model".into()),
            };

            let result = ctx
                .subloop()
                .spawn(spec)
                .await
                .map_err(|e| apeireth_runtime::ModuleError::Message(e.to_string()))?;

            assert_eq!(result.rounds, 2);
            assert_eq!(result.tool_results.len(), 1);
            // Must be permanently denied by governance
            assert!(!result.tool_results[0].is_ok());
            assert!(result.tool_results[0]
                .render()
                .contains("refused by governance"));

            return Ok(ModuleOutcome::continue_());
        }

        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn globally_denied_capability_remains_denied_inside_subloop() {
    let (tool, tool_invocations) =
        MockInstrumentedTool::new("tool.globally_denied", "globally_denied_tool");

    let script = vec![
        ScriptStep::CallTool {
            call_id: "gov_call_1",
            tool: "globally_denied_tool",
            arguments: serde_json::json!({}),
        },
        ScriptStep::Say("SubLoop handled governance refusal"),
        ScriptStep::Say("Main turn completed cleanly"),
    ];

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = Arc::new(GovernanceSubLoopModule {
        manifest: ModuleManifest::new("module.gov.test", "Governance Tester Module"),
        tool: Arc::new(tool),
    });

    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_capability(module.tool.clone())
        .with_module(module)
        .with_governance(Arc::new(CustomDenyGovernance))
        .with_default_model("subloop-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let req = TurnRequest::new(SessionId::new(), "hello").with_model("subloop-model");
    let response = runtime.execute(req).await.expect("turn executes");

    assert_eq!(response.text, "Main turn completed cleanly");
    // Tool::invoke was NEVER called because governance blocked it before invocation
    assert_eq!(
        tool_invocations.load(Ordering::SeqCst),
        0,
        "ToolCapability::invoke must not be called when governance denies"
    );
}

// ---------------------------------------------------------------------------
// 4. Capability requiring interactive approval fails cleanly in SubLoop
// ---------------------------------------------------------------------------

struct ApprovalRequiredGovernance;

#[async_trait]
impl GovernanceHook for ApprovalRequiredGovernance {
    fn name(&self) -> &str {
        "approval_required"
    }

    async fn evaluate(&self, req: &GovernanceRequest<'_>) -> Decision {
        match req.action {
            // S3 Host grant covers only this callback. Every other capability
            // keeps the original fixture's interactive-approval requirement.
            Action::CapabilityDispatch { capability, .. }
                if capability.as_str() == "tool.behavior.module.appr.test" =>
            {
                Decision::Allow
            }
            Action::CapabilityDispatch { .. } => Decision::RequireApproval {
                reason: "sensitive action needs human confirmation".into(),
            },
            _ => Decision::Allow,
        }
    }
}

struct ApprovalSubLoopModule {
    manifest: ModuleManifest,
    tool: Arc<dyn ToolCapability>,
}

#[async_trait]
impl Module for ApprovalSubLoopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            let spec = SubLoopSpec {
                max_rounds: 3,
                allowed_capabilities: vec![self.tool.id().clone()],
                timeout: None,
                messages: vec![NormalizedMessage::user("subloop prompt")],
                system_prompt: None,
                model: Some("subloop-model".into()),
            };

            let result = ctx
                .subloop()
                .spawn(spec)
                .await
                .map_err(|e| apeireth_runtime::ModuleError::Message(e.to_string()))?;

            assert_eq!(result.rounds, 2);
            assert_eq!(result.tool_results.len(), 1);
            assert!(!result.tool_results[0].is_ok());
            assert!(result.tool_results[0]
                .render()
                .contains("requires interactive approval which is not permitted in subloops"));

            return Ok(ModuleOutcome::continue_());
        }

        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn effectful_capability_requiring_approval_fails_cleanly_in_subloop() {
    let (tool, tool_invocations) =
        MockInstrumentedTool::new("tool.approval_required", "approval_tool");

    let script = vec![
        ScriptStep::CallTool {
            call_id: "appr_call_1",
            tool: "approval_tool",
            arguments: serde_json::json!({}),
        },
        ScriptStep::Say("SubLoop handled approval requirement cleanly"),
        ScriptStep::Say("Main turn completed cleanly"),
    ];

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = Arc::new(ApprovalSubLoopModule {
        manifest: ModuleManifest::new("module.appr.test", "Approval Tester Module"),
        tool: Arc::new(tool),
    });

    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_capability(module.tool.clone())
        .with_module(module)
        .with_governance(Arc::new(ApprovalRequiredGovernance))
        .with_default_model("subloop-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let session_id = SessionId::new();
    let req = TurnRequest::new(session_id, "hello").with_model("subloop-model");
    let response = runtime.execute(req).await.expect("turn executes");

    assert_eq!(response.text, "Main turn completed cleanly");
    assert_eq!(
        tool_invocations.load(Ordering::SeqCst),
        0,
        "Tool requiring interactive approval must not be invoked inside SubLoop"
    );

    // Assert no pending approval was created in the session
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load")
        .expect("session");
    assert!(session.active_approval_id.is_none());
    for child in runtime.agent_instances() {
        let private = runtime
            .sessions()
            .load(&child.session)
            .await
            .unwrap()
            .unwrap();
        assert!(private.active_approval_id.is_none());
        assert!(
            private.approvals.is_empty(),
            "isolated tool cannot leave hidden approvals"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. SubLoop timeout enforcement across whole execution
// ---------------------------------------------------------------------------

struct TimeoutSubLoopModule {
    manifest: ModuleManifest,
}

#[async_trait]
impl Module for TimeoutSubLoopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            let spec = SubLoopSpec {
                max_rounds: 3,
                allowed_capabilities: Vec::new(),
                timeout: Some(Duration::from_millis(50)), // Strict 50ms timeout
                messages: vec![NormalizedMessage::user("slow request")],
                system_prompt: None,
                model: Some("subloop-model".into()),
            };

            let err = ctx
                .subloop()
                .spawn(spec)
                .await
                .expect_err("SubLoop must time out");

            match err {
                SubLoopError::Timeout => {}
                other => panic!("expected SubLoopError::Timeout, got {other:?}"),
            }

            return Ok(ModuleOutcome::continue_()
                .with_prompt_overlay(PromptOverlay::system("SubLoop timed out as expected")));
        }

        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn subloop_timeout_enforces_overall_execution_deadline() {
    let script = vec![
        // Provider takes 300ms to respond, exceeding the 50ms SubLoop timeout
        ScriptStep::DelayAndSay(Duration::from_millis(300), "delayed response"),
        // Main loop turn response
        ScriptStep::Say("Main turn completed after subloop timeout"),
    ];

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = Arc::new(TimeoutSubLoopModule {
        manifest: ModuleManifest::new("module.timeout.test", "Timeout Tester Module"),
    });

    let mut runtime = Runtime::builder()
        .with_governance(behavior_grant("module.timeout.test"))
        .with_plugin(plugin)
        .with_module(module)
        .with_default_model("subloop-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let session_id = SessionId::new();
    let req = TurnRequest::new(session_id, "hello").with_model("subloop-model");
    let response = runtime.execute(req).await.expect("turn executes");

    assert_eq!(response.text, "Main turn completed after subloop timeout");

    // Primary session is intact and does not contain partial subloop messages
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load")
        .expect("session");
    for msg in &session.messages {
        for part in &msg.content {
            if let apeireth_protocol::canonical::ContentPart::Text { text } = part {
                assert_ne!(text, "slow request");
                assert_ne!(text, "delayed response");
            }
        }
    }
}

struct DenyCompletions;

#[async_trait]
impl GovernanceHook for DenyCompletions {
    fn name(&self) -> &str {
        "deny_completions"
    }

    async fn evaluate(&self, req: &GovernanceRequest<'_>) -> Decision {
        match req.action {
            Action::Completion { .. } => Decision::Deny {
                reason: "completions blocked".into(),
            },
            _ => Decision::Allow,
        }
    }
}

struct IsolatedCompletionSubLoopModule {
    manifest: ModuleManifest,
    error: Arc<std::sync::Mutex<Option<String>>>,
}

#[async_trait]
impl Module for IsolatedCompletionSubLoopModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            let err = ctx
                .subloop()
                .spawn(SubLoopSpec::single_turn("side work"))
                .await
                .expect_err("subloop completion must fail closed");
            *self.error.lock().unwrap() = Some(err.to_string());
            return Ok(ModuleOutcome::continue_());
        }
        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn global_completion_deny_blocks_subloop_and_does_not_call_provider() {
    let provider = ScriptedProvider::new("provider.mock", vec![ScriptStep::Say("must not run")]);
    let error = Arc::new(std::sync::Mutex::new(None));
    let module = Arc::new(IsolatedCompletionSubLoopModule {
        manifest: ModuleManifest::new("module.completion.deny", "completion deny"),
        error: Arc::clone(&error),
    });
    let mut runtime = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(Arc::clone(&provider)))
        .with_module(module)
        .with_governance(Arc::new(DenyCompletions))
        .with_default_model("subloop-model")
        .build()
        .await
        .expect("runtime builds");

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello").with_model("subloop-model"))
        .await
        .expect_err("main completion is also denied");
    assert!(err.to_string().contains("completions blocked"), "{err}");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let side = error
        .lock()
        .unwrap()
        .clone()
        .expect("subloop error captured");
    assert!(
        side.contains("refused by governance") || side.contains("completions blocked"),
        "{side}"
    );
}

// S1 evidence runs real instances and the existing Kernel/Dispatcher, with an
// offline Provider only at the network boundary.
use apeireth_plugin::agents::{AgentConfig, AgentErrorCode, AgentOwner, AgentStatus};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, SessionEventKind};

struct RecordingSubLoops {
    manifest: ModuleManifest,
    specs: Vec<SubLoopSpec>,
    results: Mutex<Vec<Result<SubLoopResult, SubLoopError>>>,
    starts: AtomicUsize,
}
impl RecordingSubLoops {
    fn new(id: &str, specs: Vec<SubLoopSpec>) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new(id, "Subtask evidence"),
            specs,
            results: Mutex::new(vec![]),
            starts: AtomicUsize::new(0),
        })
    }
}
#[async_trait]
impl Module for RecordingSubLoops {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook == HookPoint::TurnStart {
            self.starts.fetch_add(1, Ordering::SeqCst);
            for spec in &self.specs {
                let result = ctx.subloop().spawn(spec.clone()).await;
                self.results.lock().unwrap().push(result);
            }
        }
        Ok(ModuleOutcome::continue_())
    }
}
fn behavior_id(module: &RecordingSubLoops) -> PluginId {
    PluginId::new(format!("compat.{}", module.manifest.id)).unwrap()
}
fn behavior_tool(module: &RecordingSubLoops) -> CapabilityId {
    apeireth_runtime::canonical::behavior_adapter::registered_behavior_capability(
        &module.manifest.id,
    )
    .unwrap()
}
fn child_spec(tool: &dyn ToolCapability) -> SubLoopSpec {
    SubLoopSpec::single_turn("private task")
        .with_max_rounds(2)
        .with_allowed_capability(tool.id().clone())
}
fn granted_policy(module: &RecordingSubLoops, tools: Vec<CapabilityId>) -> AgentHostPolicy {
    let mut policy = AgentHostPolicy {
        concurrent_turns: 1,
        ..Default::default()
    };
    policy.modules.insert(
        behavior_id(module),
        ModuleAgentGrant {
            tools: tools.into_iter().collect(),
            max_instances: 4,
            ..Default::default()
        },
    );
    policy
}

#[tokio::test]
async fn registered_parent_subloop_uses_independent_instance_and_durable_paired_facts_with_one_turn_slot(
) {
    let (tool, count) = MockInstrumentedTool::new("tool.evidence", "evidence");
    let module = RecordingSubLoops::new("module.evidence", vec![child_spec(&tool)]);
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "child_call",
                tool: "evidence",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("child answer"),
            ScriptStep::Say("parent answer"),
        ],
    );
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_capability(Arc::new(tool.clone()))
        .with_registered_behavior(module.clone())
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_default_model("subloop-model")
        .build_agent_runtime(granted_policy(&module, vec![tool.id().clone()]))
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![behavior_tool(&module), tool.id().clone()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(2), parent.run("parent input".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reply.text, "parent answer");
    assert_eq!(
        module.starts.load(Ordering::SeqCst),
        1,
        "child must suppress automatic hooks"
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let child = host
        .instances()
        .into_iter()
        .find(|a| a.id != parent.id())
        .unwrap();
    assert_ne!(child.session, parent.snapshot().session);
    assert_eq!(child.status, AgentStatus::Stopped);
    assert!(
        matches!(&child.owner, AgentOwner::Module { definition, .. } if definition == &behavior_id(&module))
    );
    assert_eq!(child.usage.requests, 2);
    assert_eq!(tool.contexts.lock().unwrap()[0].agent, child.id);
    assert_eq!(
        tool.contexts.lock().unwrap()[0].session,
        Some(child.session)
    );
    assert_eq!(child.usage.tool_calls, 1);
    assert_eq!(parent.snapshot().usage.requests, 1);
    assert_eq!(
        host.usage(None).requests,
        3,
        "actual requests are counted once"
    );
    let kernel = host.kernel().await;
    let private = kernel
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    let public = kernel
        .sessions()
        .load(&parent.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    assert!(!public
        .messages
        .iter()
        .any(
            |m| apeireth_protocol::canonical::ContentPart::join_text(&m.content)
                .contains("private task")
                || apeireth_protocol::canonical::ContentPart::join_text(&m.content)
                    .contains("child answer")
        ));
    assert!(private.events.iter().any(|e| matches!(&e.event, SessionEventKind::AgentTaskStarted {
        agent, parent_agent, parent_session, parent_request, ..
    } if *agent == child.id && *parent_agent == parent.id() && *parent_session == public.id && *parent_request == reply.request)));
    let facts: Vec<_> = private
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SessionEventKind::ToolEventRecorded { fact } => Some((e.request, fact)),
            _ => None,
        })
        .collect();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].1.call().id, "child_call");
    assert!(facts[0].1.result().is_ok());
    assert_ne!(facts[0].0, reply.request);
    apeireth_protocol::canonical::validate_closed_tool_context(&private.provider_view()).unwrap();
    let results = module.results.lock().unwrap();
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.rounds, 2);
    assert_eq!(result.usage.total_tokens, 20);
    assert_eq!(result.usage.prompt_cache_hit_tokens, None);
    assert_eq!(result.tool_results.len(), 1);
}

#[tokio::test]
async fn subloop_scope_is_intersection_of_allowlist_parent_scope_and_host_grant() {
    for parent_allows in [false, true] {
        let (tool, count) = MockInstrumentedTool::new("tool.restricted", "restricted");
        let module = RecordingSubLoops::new("module.scope", vec![child_spec(&tool)]);
        let provider = ScriptedProvider::new(
            "provider.mock",
            vec![
                ScriptStep::CallTool {
                    call_id: "forbidden",
                    tool: "restricted",
                    arguments: serde_json::json!({}),
                },
                ScriptStep::Say("denied safely"),
                ScriptStep::Say("main"),
            ],
        );
        let policy = granted_policy(
            &module,
            if parent_allows {
                vec![]
            } else {
                vec![tool.id().clone()]
            },
        );
        let host = Runtime::builder()
            .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
            .with_capability(Arc::new(tool.clone()))
            .with_registered_behavior(module.clone())
            .with_governance(Arc::new(apeireth_governance::AllowAll))
            .with_default_model("subloop-model")
            .build_agent_runtime(policy)
            .await
            .unwrap();
        let mut tools = vec![behavior_tool(&module)];
        if parent_allows {
            tools.push(tool.id().clone());
        }
        let parent = host
            .spawn_agent(
                "main",
                AgentConfig {
                    tools,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        parent.run("input".into()).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(provider.requests.lock().unwrap()[0].tools.is_empty());
        let results = module.results.lock().unwrap();
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.tool_results.len(), 1);
        assert!(!result.tool_results[0].is_ok());
    }
}

#[tokio::test]
async fn subloops_share_turn_budget_across_modules_handles_and_new_children_and_reset_next_turn() {
    let a = RecordingSubLoops::new("module.budget.a", vec![SubLoopSpec::single_turn("first")]);
    let b = RecordingSubLoops::new(
        "module.budget.b",
        vec![
            SubLoopSpec::single_turn("second"),
            SubLoopSpec::single_turn("retry"),
        ],
    );
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::Say("side one"),
            ScriptStep::Say("main one"),
            ScriptStep::Say("side two"),
            ScriptStep::Say("main two"),
        ],
    );
    let mut callback_policy = apeireth_governance::PermissionPolicy::new();
    for id in ["module.budget.a", "module.budget.b"] {
        callback_policy.grant(apeireth_governance::Permission::ExecuteTool(format!(
            "tool.behavior.{id}"
        )));
    }
    let mut runtime = Runtime::builder()
        .with_governance(Arc::new(
            apeireth_governance::PermissionGovernanceHook::new(callback_policy),
        ))
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_module(a.clone())
        .with_module(b.clone())
        .with_default_model("subloop-model")
        .with_max_module_invocations(1)
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "one"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "two"))
        .await
        .unwrap();
    assert!(a.results.lock().unwrap().iter().all(Result::is_ok));
    assert_eq!(b.results.lock().unwrap().len(), 4);
    assert!(b
        .results
        .lock()
        .unwrap()
        .iter()
        .all(|r| matches!(r, Err(SubLoopError::BudgetExceeded { limit: 1 }))));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 4);
    assert_eq!(
        runtime
            .agent_instances()
            .iter()
            .map(|a| a.usage.requests)
            .sum::<u64>(),
        2
    );
}

#[tokio::test]
async fn subloop_preserves_normalized_input_model_system_round_errors_and_saturating_usage() {
    let (tool, _) = MockInstrumentedTool::new("tool.usage", "usage");
    let mut spec = child_spec(&tool)
        .with_system_prompt("private system")
        .with_model("subloop-model");
    spec.messages = vec![
        NormalizedMessage::assistant("prior answer"),
        NormalizedMessage::user("follow up"),
    ];
    let zero = spec.clone().with_max_rounds(0);
    let nonconverging = spec.clone().with_max_rounds(1);
    let module = RecordingSubLoops::new("module.normalized", vec![zero, nonconverging, spec]);
    let response = |id: &str, tool_call: bool| NormalizedResponse {
        id: id.into(),
        model: "subloop-model".into(),
        content: if tool_call {
            String::new()
        } else {
            "done".into()
        },
        finish_reason: Some(if tool_call {
            NormalizedFinishReason::ToolCalls
        } else {
            NormalizedFinishReason::Stop
        }),
        usage: NormalizedUsage {
            prompt_tokens: u32::MAX,
            completion_tokens: u32::MAX,
            total_tokens: u32::MAX,
            prompt_cache_hit_tokens: Some(u32::MAX),
        },
        tool_calls: if tool_call {
            vec![ToolCall {
                id: id.into(),
                name: "usage".into(),
                arguments: serde_json::json!({}),
            }]
        } else {
            vec![]
        },
        raw_metadata: serde_json::Map::new(),
    };
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::Reply(response("limit_call", true)),
            ScriptStep::Reply(response("usage_call", true)),
            ScriptStep::Reply(response("final", false)),
            ScriptStep::Say("parent"),
        ],
    );
    let mut policy = AgentHostPolicy::default();
    policy.max_budget.max_tokens = u64::MAX;
    // The old Host fallback had an unbounded side budget in this saturation
    // fixture. S3 uses the real module owner: grant that same test budget and
    // only its actual tool explicitly, without changing production limits.
    policy.modules.insert(
        PluginId::new("compat.module.normalized").unwrap(),
        ModuleAgentGrant {
            tools: [tool.id().clone()].into(),
            max_instances: 3,
            budget: apeireth_plugin::agents::AgentBudget {
                max_tokens: u64::MAX,
                ..policy.max_budget.clone()
            },
            ..Default::default()
        },
    );
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_capability(Arc::new(tool))
        .with_module(module.clone())
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_default_model("subloop-model")
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![CapabilityId::new("tool.behavior.module.normalized").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    parent.run("parent".into()).await.unwrap();
    let results = module.results.lock().unwrap();
    assert!(matches!(
        results[0],
        Err(SubLoopError::RoundLimitReached { rounds: 0 })
    ));
    assert!(matches!(
        results[1],
        Err(SubLoopError::RoundLimitReached { rounds: 1 })
    ));
    let result = results[2].as_ref().unwrap();
    assert_eq!(result.rounds, 2);
    assert_eq!(result.usage.total_tokens, u32::MAX);
    assert_eq!(result.usage.prompt_cache_hit_tokens, Some(u32::MAX));
    let requests = provider.requests.lock().unwrap();
    assert_eq!(
        requests[0]
            .messages
            .iter()
            .map(|m| apeireth_protocol::canonical::ContentPart::join_text(&m.content))
            .collect::<Vec<_>>(),
        vec!["private system", "prior answer", "follow up"]
    );
    assert_eq!(requests[0].model, "subloop-model");
    assert_eq!(
        host.instances()
            .iter()
            .filter(|a| a.id != parent.id())
            .map(|a| a.usage.requests)
            .sum::<u64>(),
        3,
        "nonconverging requests retain their usage"
    );
}

struct NativeDrainTool {
    id: CapabilityId,
    started: Arc<tokio::sync::Notify>,
    drained: Arc<AtomicUsize>,
}
#[async_trait]
impl ToolCapability for NativeDrainTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "native_drain".into(),
            description: None,
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, _: &ToolCall) -> ToolResult {
        panic!("context required")
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&apeireth_plugin::FrozenInvocation>,
        context: &apeireth_plugin::ToolExecutionContext,
    ) -> ToolResult {
        let lease = context
            .module_tasks
            .lease_with_cancellation(context.cancellation.clone())
            .unwrap();
        let cancellation = context.cancellation.clone();
        let started = self.started.clone();
        let drained = self.drained.clone();
        tokio::task::spawn_blocking(move || {
            started.notify_one();
            while !cancellation.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            // Cancellation acknowledgement and native cleanup are distinct.
            std::thread::sleep(Duration::from_millis(20));
            drained.fetch_add(1, Ordering::SeqCst);
            drop(lease);
        })
        .await
        .unwrap();
        ToolResult::ok(&call.id, serde_json::json!("drained"))
    }
}

#[tokio::test]
async fn subloop_timeout_returns_only_after_native_tool_cancellation_drains() {
    let tool = Arc::new(NativeDrainTool {
        id: CapabilityId::new("tool.native").unwrap(),
        started: Arc::new(tokio::sync::Notify::new()),
        drained: Arc::new(AtomicUsize::new(0)),
    });
    let module = RecordingSubLoops::new(
        "module.native",
        vec![child_spec(tool.as_ref()).with_timeout(Duration::from_millis(50))],
    );
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "native",
                tool: "native_drain",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("parent"),
        ],
    );
    let mut runtime = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider))
        .with_capability(tool.clone())
        .with_module(module.clone())
        .with_default_model("subloop-model")
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .build()
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        runtime.execute(TurnRequest::new(SessionId::new(), "parent")),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        module.results.lock().unwrap()[0],
        Err(SubLoopError::Timeout)
    ));
    assert_eq!(tool.drained.load(Ordering::SeqCst), 1);
    let child = runtime.agent_instances().pop().unwrap();
    assert_eq!(child.status, AgentStatus::Stopped);
    assert_eq!(child.usage.requests, 1);
    let private = runtime
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert_interrupted_checkpoint(&private, child.id, &["native"]);
}

#[tokio::test]
async fn parent_stop_cancels_inflight_subloop_and_drains_native_leases_without_stopping_provider() {
    let tool = Arc::new(NativeDrainTool {
        id: CapabilityId::new("tool.native").unwrap(),
        started: Arc::new(tokio::sync::Notify::new()),
        drained: Arc::new(AtomicUsize::new(0)),
    });
    let (first, count) = MockInstrumentedTool::new("tool.first", "first");
    let module = RecordingSubLoops::new(
        "module.parent_cancel",
        vec![child_spec(tool.as_ref())
            .with_max_rounds(3)
            .with_allowed_capability(first.id().clone())],
    );
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "reused",
                tool: "first",
                arguments: serde_json::json!({}),
            },
            ScriptStep::CallTool {
                call_id: "reused",
                tool: "native_drain",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("sibling still runs"),
        ],
    );
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider))
        .with_capability(tool.clone())
        .with_capability(Arc::new(first.clone()))
        .with_registered_behavior(module.clone())
        .with_default_model("subloop-model")
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .build_agent_runtime(granted_policy(
            &module,
            vec![tool.id().clone(), first.id().clone()],
        ))
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![
                    behavior_tool(&module),
                    tool.id().clone(),
                    first.id().clone(),
                ],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let running_parent = parent.clone();
    let task = tokio::spawn(async move { running_parent.run("run".into()).await });
    tokio::time::timeout(Duration::from_secs(2), tool.started.notified())
        .await
        .unwrap();
    parent.stop().await.unwrap();
    assert_eq!(tool.drained.load(Ordering::SeqCst), 1);
    assert!(matches!(task.await.unwrap(), Err(e) if e.code == AgentErrorCode::Cancelled));
    assert!(host
        .instances()
        .iter()
        .all(|a| a.status == AgentStatus::Stopped));
    let child = host
        .instances()
        .into_iter()
        .find(|a| a.id != parent.id())
        .unwrap();
    let private = host
        .kernel()
        .await
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert_interrupted_checkpoint(&private, child.id, &["reused"]);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(child.usage.tool_calls, 2);
    assert_eq!(private.events.iter().filter(|e| matches!(&e.event,
        SessionEventKind::ToolEventRecorded { fact } if fact.call().id == "reused" && fact.result().is_ok())).count(), 1);
    parent.stop().await.unwrap();
    let again = host
        .kernel()
        .await
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&private).unwrap(),
        serde_json::to_value(&again).unwrap()
    );
    let sibling = host
        .spawn_agent("other", AgentConfig::default())
        .await
        .unwrap();
    assert_eq!(
        sibling.run("hello".into()).await.unwrap().text,
        "sibling still runs"
    );
}

#[tokio::test]
async fn each_provider_round_consumes_shared_budget_and_keeps_partial_tool_facts() {
    let (tool, count) = MockInstrumentedTool::new("tool.round_budget", "round_budget");
    let module = RecordingSubLoops::new(
        "module.round_budget",
        vec![child_spec(&tool), SubLoopSpec::single_turn("retry")],
    );
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "partial",
                tool: "round_budget",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("parent"),
        ],
    );
    let mut runtime = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_capability(Arc::new(tool))
        .with_module(module.clone())
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_default_model("subloop-model")
        .with_max_module_invocations(1)
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "parent"))
        .await
        .unwrap();
    assert!(module
        .results
        .lock()
        .unwrap()
        .iter()
        .all(|r| matches!(r, Err(SubLoopError::BudgetExceeded { limit: 1 }))));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    let child = runtime
        .agent_instances()
        .into_iter()
        .find(|a| a.usage.requests == 1)
        .unwrap();
    let private = runtime
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    let facts: Vec<_> = private
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SessionEventKind::ToolEventRecorded { fact } => Some(fact),
            _ => None,
        })
        .collect();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].call().id, "partial");
    assert!(facts[0].result().is_ok());
    apeireth_protocol::canonical::validate_closed_tool_context(&private.provider_view()).unwrap();
    assert_eq!(child.usage.tool_calls, 1);
    assert_eq!(child.usage.input_tokens, 5);
}

#[tokio::test]
async fn provider_fallback_attempt_also_reserves_the_same_task_budget() {
    use apeireth_plugin::agents::{AgentExecutionMode, AgentTask};
    let a = ScriptedProvider::new("provider.a", vec![ScriptStep::Fail]);
    let b = ScriptedProvider::new("provider.b", vec![ScriptStep::Fail]);
    let second = Arc::new(ScriptedProviderPlugin {
        manifest: PluginManifest::new(PluginId::new("vendor.second").unwrap(), "1.0.0", "Fallback")
            .declare_capability(
                b.id.clone(),
                CapabilityKind::Provider,
                "Fallback completions",
            )
            .unwrap(),
        provider: b.clone(),
    });
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(a.clone()))
        .with_plugin(second)
        .with_default_model("subloop-model")
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = host
        .spawn_agent("task", AgentConfig::default())
        .await
        .unwrap();
    let gate = Arc::new(apeireth_runtime::canonical::module::ModuleTurnState::new(1));
    let error = agent
        .execute(AgentTask {
            messages: vec![NormalizedMessage::user("private")],
            system: None,
            model: None,
            max_rounds: 1,
            timeout: None,
            mode: AgentExecutionMode::Isolated,
            request_gate: Some(gate.clone()),
            origin: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, AgentErrorCode::BudgetExceeded);
    assert!(apeireth_plugin::agents::AgentRequestGate::reserve(gate.as_ref()).is_err());
    assert_eq!(
        a.calls.load(Ordering::SeqCst) + b.calls.load(Ordering::SeqCst),
        1
    );
    let facts = host.requests(agent.id()).unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(agent.snapshot().usage.provider_errors, 1);
    assert_eq!(agent.snapshot().usage.requests, 1);
    agent.stop().await.unwrap();
}

#[tokio::test]
async fn normalized_task_port_preserves_full_response_and_reports_task_usage_separately() {
    use apeireth_plugin::agents::{AgentExecutionMode, AgentTask};
    let response = NormalizedResponse {
        id: "original-response".into(),
        model: "subloop-model".into(),
        content: "answer".into(),
        finish_reason: Some(NormalizedFinishReason::Stop),
        usage: NormalizedUsage {
            prompt_tokens: 3,
            completion_tokens: 4,
            total_tokens: 7,
            prompt_cache_hit_tokens: Some(2),
        },
        tool_calls: vec![],
        raw_metadata: serde_json::Map::from_iter([(
            "opaque".into(),
            serde_json::json!({"retained": true}),
        )]),
    };
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::Reply(response.clone()),
            ScriptStep::Reply(response.clone()),
        ],
    );
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_default_model("subloop-model")
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = host
        .spawn_agent("task", AgentConfig::default())
        .await
        .unwrap();
    let mut ids = vec![];
    for text in ["one", "two"] {
        let reply = agent
            .execute(AgentTask {
                messages: vec![NormalizedMessage::user(text)],
                system: None,
                model: None,
                max_rounds: 1,
                timeout: None,
                mode: AgentExecutionMode::Conversation,
                request_gate: None,
                origin: None,
            })
            .await
            .unwrap();
        assert_eq!(reply.response, response);
        assert_eq!(reply.served_by, provider.id);
        assert_eq!(reply.usage.total_tokens, 7);
        assert_eq!(reply.usage.prompt_cache_hit_tokens, Some(2));
        assert_eq!(reply.rounds, 1);
        ids.push(reply.request);
    }
    assert_ne!(ids[0], ids[1]);
    assert_eq!(agent.snapshot().usage.requests, 2);
    assert_eq!(agent.snapshot().usage.input_tokens, 6);
    assert_eq!(provider.requests.lock().unwrap()[1].messages.len(), 3);
    agent.stop().await.unwrap();
}

struct RequireCompletionApproval;
#[async_trait]
impl GovernanceHook for RequireCompletionApproval {
    fn name(&self) -> &str {
        "completion_approval"
    }
    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        if matches!(request.action, Action::Completion { .. }) {
            Decision::RequireApproval {
                reason: "explicit approval required".into(),
            }
        } else {
            Decision::Allow
        }
    }
}
#[tokio::test]
async fn isolated_completion_approval_fails_closed_without_creating_pending_approvals() {
    let module =
        RecordingSubLoops::new("module.approval", vec![SubLoopSpec::single_turn("private")]);
    let provider = ScriptedProvider::new("provider.mock", vec![]);
    let mut runtime = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_module(module.clone())
        .with_default_model("subloop-model")
        .with_governance(Arc::new(RequireCompletionApproval))
        .build()
        .await
        .unwrap();
    assert!(runtime
        .execute(TurnRequest::new(SessionId::new(), "parent"))
        .await
        .is_err());
    assert!(matches!(
        module.results.lock().unwrap()[0],
        Err(SubLoopError::ApprovalRequired { .. })
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let child = runtime.agent_instances().pop().unwrap();
    let session = runtime
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert!(session.active_approval_id.is_none());
    assert!(session.approvals.is_empty());
    assert_eq!(child.status, AgentStatus::Stopped);
}

#[tokio::test]
async fn registered_owner_stop_cancels_and_drains_child_without_stopping_parent() {
    let tool = Arc::new(NativeDrainTool {
        id: CapabilityId::new("tool.native").unwrap(),
        started: Arc::new(tokio::sync::Notify::new()),
        drained: Arc::new(AtomicUsize::new(0)),
    });
    let module = RecordingSubLoops::new("module.owner_cancel", vec![child_spec(tool.as_ref())]);
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "native",
                tool: "native_drain",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("parent survives"),
            ScriptStep::Say("next turn"),
        ],
    );
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider))
        .with_capability(tool.clone())
        .with_registered_behavior(module.clone())
        .with_default_model("subloop-model")
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .build_agent_runtime(granted_policy(&module, vec![tool.id().clone()]))
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![behavior_tool(&module), tool.id().clone()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let run_parent = parent.clone();
    let task = tokio::spawn(async move { run_parent.run("run".into()).await });
    tokio::time::timeout(Duration::from_secs(2), tool.started.notified())
        .await
        .unwrap();
    host.stop_module(&behavior_id(&module)).await.unwrap();
    assert_eq!(tool.drained.load(Ordering::SeqCst), 1);
    assert_eq!(task.await.unwrap().unwrap().text, "parent survives");
    let child = host
        .instances()
        .into_iter()
        .find(|a| a.id != parent.id())
        .unwrap();
    assert_eq!(child.status, AgentStatus::Stopped);
    assert!(
        matches!(child.owner, AgentOwner::Module { definition, instance }
        if definition == behavior_id(&module) && instance == host.kernel().await.plugins().instance_id(&definition).unwrap())
    );
    let private = host
        .kernel()
        .await
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert_interrupted_checkpoint(&private, child.id, &["native"]);
    assert_eq!(parent.run("again".into()).await.unwrap().text, "next turn");
    assert_eq!(module.starts.load(Ordering::SeqCst), 1);
}

fn assert_interrupted_checkpoint(
    session: &apeireth_runtime::canonical::session::Session,
    agent: apeireth_core::kernel::AgentId,
    pending: &[&str],
) {
    let facts: Vec<_> = session
        .events
        .iter()
        .filter(|e| matches!(e.event, SessionEventKind::AgentTaskInterrupted { .. }))
        .collect();
    assert_eq!(
        facts.len(),
        1,
        "explicit drain publishes one interruption fact"
    );
    assert!(
        matches!(&facts[0].event, SessionEventKind::AgentTaskInterrupted {
        agent: actual, pending_tool_calls
    } if *actual == agent && pending_tool_calls == pending)
    );
    assert!(session.events.iter().any(|e| e.request == facts[0].request && e.trace == facts[0].trace
        && matches!(e.event, SessionEventKind::AgentTaskStarted { agent: actual, .. } if actual == agent)));
    assert!(!session.events.iter().any(|e| e.request == facts[0].request
        && matches!(e.event, SessionEventKind::TurnCompleted { .. })));
    let reloaded: apeireth_runtime::canonical::session::Session =
        serde_json::from_value(serde_json::to_value(session).unwrap()).unwrap();
    assert_eq!(
        session.log, reloaded.log,
        "reader preserves interruption and provenance"
    );
}

#[tokio::test]
async fn parent_cancel_preserves_child_commits_and_cancelled_provider_fact_without_recharging() {
    let (tool, count) = MockInstrumentedTool::new("tool.partial", "partial");
    let module = RecordingSubLoops::new("module.partial_cancel", vec![child_spec(&tool)]);
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "committed",
                tool: "partial",
                arguments: serde_json::json!({}),
            },
            ScriptStep::DelayAndSay(Duration::from_secs(30), "must not commit"),
            ScriptStep::Say("sibling"),
        ],
    );
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_capability(Arc::new(tool.clone()))
        .with_behavior(module.clone())
        .with_default_model("subloop-model")
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .build_agent_runtime(granted_policy(&module, vec![tool.id().clone()]))
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![behavior_tool(&module), tool.id().clone()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let running = parent.clone();
    let task = tokio::spawn(async move { running.run("parent".into()).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while provider.calls.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    parent.stop().await.unwrap();
    assert!(matches!(task.await.unwrap(), Err(e) if e.code == AgentErrorCode::Cancelled));
    let child = host
        .instances()
        .into_iter()
        .find(|a| a.id != parent.id())
        .unwrap();
    let session = host
        .kernel()
        .await
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert_interrupted_checkpoint(&session, child.id, &[]);
    apeireth_protocol::canonical::validate_closed_tool_context(&session.provider_view()).unwrap();
    assert_eq!(session.events.iter().filter(|e| matches!(&e.event,
        SessionEventKind::ToolEventRecorded { fact } if fact.call().id == "committed" && fact.result().is_ok())).count(), 1);
    assert!(!session.messages.iter().any(|m| {
        apeireth_protocol::canonical::ContentPart::join_text(&m.content).contains("must not commit")
    }));
    let facts = host.requests(child.id).unwrap();
    assert_eq!(facts.len(), 2);
    assert!(facts[0].reported.is_some() && !facts[0].cancelled);
    assert!(facts[1].reported.is_none() && facts[1].cancelled && !facts[1].failed);
    assert_eq!(facts[0].request, facts[1].request);
    assert_eq!(child.usage.requests, 2);
    assert_eq!(child.usage.cancellations, 1);
    assert_eq!(child.usage.tool_calls, 1);
    assert_eq!(host.usage(Some(&child.owner)).requests, 2);
    assert_eq!(host.usage(None).requests, 2);
    assert_eq!(parent.snapshot().usage.requests, 0);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    host.shutdown().await.unwrap();
    let after = host
        .kernel()
        .await
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.log, after.log,
        "shutdown does not duplicate interruption facts"
    );
}

#[tokio::test]
async fn managed_child_reused_wire_ids_keep_distinct_facts_and_one_meter_per_execution() {
    use apeireth_runtime::canonical::events::{event_sink, RuntimeEvent};
    let (tool, count) = MockInstrumentedTool::new("tool.audit", "audit");
    let module = RecordingSubLoops::new(
        "module.audit",
        vec![child_spec(&tool).with_max_rounds(3), child_spec(&tool)],
    );
    let provider = ScriptedProvider::new(
        "provider.mock",
        vec![
            ScriptStep::CallTool {
                call_id: "reused",
                tool: "audit",
                arguments: serde_json::json!({}),
            },
            ScriptStep::CallTool {
                call_id: "reused",
                tool: "audit",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("private one"),
            ScriptStep::CallTool {
                call_id: "reused",
                tool: "audit",
                arguments: serde_json::json!({}),
            },
            ScriptStep::Say("private two"),
            ScriptStep::Say("public"),
        ],
    );
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let host = Runtime::builder()
        .with_plugin(ScriptedProviderPlugin::new(provider.clone()))
        .with_capability(Arc::new(tool.clone()))
        .with_module(module.clone())
        .with_default_model("subloop-model")
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_event_sink(event_sink(move |event| {
            captured.lock().unwrap().push(event)
        }))
        .build_agent_runtime(granted_policy(&module, vec![tool.id().clone()]))
        .await
        .unwrap();
    let parent = host
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![behavior_tool(&module), tool.id().clone()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        parent.run("public input".into()).await.unwrap().text,
        "public"
    );
    let children: Vec<_> = host
        .instances()
        .into_iter()
        .filter(|a| a.id != parent.id())
        .collect();
    assert_eq!(children.len(), 2);
    let mut execution_ids = std::collections::BTreeSet::new();
    let mut request_ids = std::collections::BTreeSet::new();
    for child in &children {
        let session = host
            .kernel()
            .await
            .sessions()
            .load(&child.session)
            .await
            .unwrap()
            .unwrap();
        let reloaded: apeireth_runtime::canonical::session::Session =
            serde_json::from_value(serde_json::to_value(&session).unwrap()).unwrap();
        assert_eq!(session.log, reloaded.log);
        apeireth_protocol::canonical::validate_closed_tool_context(&reloaded.provider_view())
            .unwrap();
        assert!(!session
            .events
            .iter()
            .any(|e| matches!(e.event, SessionEventKind::AgentTaskInterrupted { .. })));
        let mut facts = 0;
        for entry in &session.events {
            if let SessionEventKind::ToolEventRecorded { fact } = &entry.event {
                assert_eq!(fact.call().id, "reused");
                assert!(
                    execution_ids.insert(fact.call_id()),
                    "wire id is not an execution identity"
                );
                facts += 1;
            }
        }
        assert_eq!(child.usage.tool_calls, facts);
        let requests = host.requests(child.id).unwrap();
        assert_eq!(requests.len() as u64, child.usage.requests);
        for fact in requests {
            assert_eq!(fact.agent, child.id);
            assert_eq!(fact.owner, child.owner);
            assert!(request_ids.insert(fact.id));
            assert!(!fact.failed && !fact.cancelled);
        }
    }
    assert_eq!(execution_ids.len(), 3);
    assert_eq!(count.load(Ordering::SeqCst), 3);
    assert_eq!(
        host.usage(None).requests,
        provider.calls.load(Ordering::SeqCst) as u64
    );
    assert_eq!(host.usage(None).requests, 6);
    assert_eq!(host.usage(Some(&children[0].owner)).requests, 5);
    assert_eq!(parent.snapshot().usage.requests, 1);
    // Every fixed Hook callback is separately executed, including empty audit
    // observations. Count its durable facts rather than just TurnStart's output.
    let public = host
        .kernel()
        .await
        .sessions()
        .load(&parent.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    let callbacks = public
        .events
        .iter()
        .filter(|e| matches!(e.event, SessionEventKind::ToolEventRecorded { .. }))
        .count() as u64;
    assert!(callbacks > 0);
    assert_eq!(parent.snapshot().usage.tool_calls, callbacks);
    assert_eq!(host.usage(None).tool_calls, callbacks + 3);
    for event in events.lock().unwrap().iter() {
        let session = match event {
            RuntimeEvent::TurnStarted { session, .. }
            | RuntimeEvent::TurnCompleted { session, .. }
            | RuntimeEvent::TurnFailed { session, .. }
            | RuntimeEvent::Trace { session, .. }
            | RuntimeEvent::ApprovalRequired { session, .. } => session,
        };
        assert_eq!(
            *session,
            parent.snapshot().session,
            "private child emits no frontend frames"
        );
    }
    assert!(!events.lock().unwrap().is_empty());
    host.shutdown().await.unwrap();
}
