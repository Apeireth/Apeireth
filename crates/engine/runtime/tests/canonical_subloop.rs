//! Tests proving bounded, private-transcript SubLoops owned by modules.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, RequestId, SessionId, TraceId};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::{
    HookPoint, Module, ModuleContext, ModuleManifest, ModuleOutcome, Runtime, SubLoopSpec,
    TurnRequest,
};
use async_trait::async_trait;

struct MockAllowedTool;

#[async_trait]
impl ToolCapability for MockAllowedTool {
    fn id(&self) -> &CapabilityId {
        static ID: std::sync::OnceLock<CapabilityId> = std::sync::OnceLock::new();
        ID.get_or_init(|| CapabilityId::new("tool.allowed").unwrap())
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "allowed_tool".into(),
            description: Some("An allowed tool".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::json!({ "allowed": true }))
    }
}

struct MockDeniedTool;

#[async_trait]
impl ToolCapability for MockDeniedTool {
    fn id(&self) -> &CapabilityId {
        static ID: std::sync::OnceLock<CapabilityId> = std::sync::OnceLock::new();
        ID.get_or_init(|| CapabilityId::new("tool.denied").unwrap())
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "denied_tool".into(),
            description: Some("A denied tool".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::json!({ "denied": true }))
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

/// Module that invokes a bounded SubLoop on TurnStart and injects the structured outcome.
struct SubLoopTesterModule {
    manifest: ModuleManifest,
    allowed_tool: Arc<dyn ToolCapability>,
    denied_tool: Arc<dyn ToolCapability>,
}

impl SubLoopTesterModule {
    fn new() -> Self {
        Self {
            manifest: ModuleManifest::new("module.subloop.tester", "SubLoop Tester Module"),
            allowed_tool: Arc::new(MockAllowedTool),
            denied_tool: Arc::new(MockDeniedTool),
        }
    }
}

#[async_trait]
impl Module for SubLoopTesterModule {
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
                allowed_capabilities: vec![self.allowed_tool.id().clone()], // Only allowed_tool in allowlist
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

            return Ok(ModuleOutcome::continue_().with_prompt_overlay(
                apeireth_runtime::PromptOverlay::system(format!(
                    "SubLoop computed: {}",
                    result.text
                )),
            ));
        }

        Ok(ModuleOutcome::continue_())
    }
}

#[tokio::test]
async fn subloop_runs_on_private_transcript_with_strict_capability_allowlist() {
    // Script:
    // Step 0: SubLoop calls allowed_tool
    // Step 1: SubLoop finishes with "subloop finished successfully"
    // Step 2: Main loop model turn sees overlay and answers "Main answer complete"
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
    let module = Arc::new(SubLoopTesterModule::new());

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

struct SubLoopDeniedTesterModule {
    manifest: ModuleManifest,
    allowed_tool: Arc<dyn ToolCapability>,
    denied_tool: Arc<dyn ToolCapability>,
}

impl SubLoopDeniedTesterModule {
    fn new() -> Self {
        Self {
            manifest: ModuleManifest::new(
                "module.subloop.denied_tester",
                "SubLoop Denied Tester Module",
            ),
            allowed_tool: Arc::new(MockAllowedTool),
            denied_tool: Arc::new(MockDeniedTool),
        }
    }
}

#[async_trait]
impl Module for SubLoopDeniedTesterModule {
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
                allowed_capabilities: vec![self.allowed_tool.id().clone()], // denied_tool is NOT in allowlist
                timeout: None,
                messages: vec![NormalizedMessage::user("try calling denied tool")],
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
async fn subloop_with_allowed_and_denied_tools() {
    let script = vec![
        ScriptStep::CallTool {
            call_id: "subloop_call_denied",
            tool: "denied_tool",
            arguments: serde_json::json!({}),
        },
        ScriptStep::Say("SubLoop handled denial cleanly"),
        ScriptStep::Say("Main turn completed"),
    ];

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = Arc::new(SubLoopDeniedTesterModule::new());

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
}
