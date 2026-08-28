//! Tests proving bounded, private-transcript SubLoops owned by modules and strictly governed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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

#[derive(Clone)]
struct MockInstrumentedTool {
    id: CapabilityId,
    name: String,
    invocations: Arc<AtomicUsize>,
}

impl MockInstrumentedTool {
    fn new(id: &str, name: &str) -> (Self, Arc<AtomicUsize>) {
        let invocations = Arc::new(AtomicUsize::new(0));
        (
            Self {
                id: CapabilityId::new(id).unwrap(),
                name: name.to_string(),
                invocations: Arc::clone(&invocations),
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
}

struct ScriptedProvider {
    id: CapabilityId,
    script: Vec<ScriptStep>,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    fn new(id: &str, script: Vec<ScriptStep>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: AtomicUsize::new(0),
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
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };

        match step {
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

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.allowed_tool.clone()]
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
        .with_module(module)
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

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.allowed_tool.clone(), self.denied_tool.clone()]
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
        .with_plugin(plugin)
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

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
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

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
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
