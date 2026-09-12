//! Session-level `permission_preset` semantics.
//!
//! One dangerous write/execute tool (`tool.shell`) is evaluated under the same
//! governance policy in three presets and must produce three different
//! outcomes: refuse (`read_only`), require approval (`standard`), allow without
//! approval (`full`). The enforcement hook lives in the production assembly
//! crate so the execution core remains capability-generic.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::{
    GovernancePipeline, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool,
    NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    InMemorySessionStore, PermissionPreset, Runtime, SessionStore, TurnOutcome, TurnRequest,
};
use apeireth_runtime_assembly::PermissionPresetGovernanceHook;
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

struct DangerousTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for DangerousTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("shell")
            .with_description("executes a shell command")
            .with_parameters(ToolParameters::new())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!("executed"))
    }
}

struct FakeProvider {
    id: CapabilityId,
    calls: AtomicUsize,
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
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        if index == 0 {
            Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: "call_shell".into(),
                    name: "shell".into(),
                    arguments: serde_json::json!({ "command": "echo hi" }),
                }],
                ..base
            })
        } else {
            Ok(NormalizedResponse {
                content: "done".into(),
                ..base
            })
        }
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
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
        vec![self.provider.clone()]
    }
}

fn provider_plugin(provider: Arc<FakeProvider>) -> Arc<ProviderPlugin> {
    Arc::new(ProviderPlugin {
        manifest: PluginManifest::new(
            PluginId::new("builtin.fake_provider").unwrap(),
            "1.0.0",
            "scripted provider",
        )
        .declare_capability(
            CapabilityId::new("provider.fake").unwrap(),
            CapabilityKind::Provider,
            "Scripted provider",
        )
        .unwrap(),
        provider,
    })
}

async fn build_runtime(
    store: Arc<dyn SessionStore>,
    invocations: Arc<AtomicUsize>,
) -> (Runtime, Arc<AtomicUsize>) {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.shell".into()));
    policy.require_approval_for("tool.shell");

    let pipeline = GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy)));
    let governance = Arc::new(PermissionPresetGovernanceHook::new(
        Arc::new(pipeline),
        store.clone(),
    ));

    let tool = Arc::new(DangerousTool {
        id: CapabilityId::new("tool.shell").unwrap(),
        invocations: invocations.clone(),
    });

    let runtime = Runtime::builder()
        .with_session_store(store)
        .with_governance(governance)
        .with_capability(tool)
        .with_plugin(provider_plugin(Arc::new(FakeProvider {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
        })))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();
    (runtime, invocations)
}

async fn set_preset(runtime: &Runtime, session: SessionId, preset: PermissionPreset) {
    let mut loaded = runtime.sessions().load_or_create(session).await.unwrap();
    loaded.settings.permission_preset = preset;
    runtime.sessions().save(&loaded).await.unwrap();
}

fn completed_text(outcome: &TurnOutcome) -> String {
    match outcome {
        TurnOutcome::Completed(response) => response.text.clone(),
        TurnOutcome::PendingApproval(_) => panic!("expected Completed"),
    }
}

#[tokio::test]
async fn read_only_refuses_dangerous_tool_with_chinese_explanation() {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let invocations = Arc::new(AtomicUsize::new(0));
    let (runtime, invocations) = build_runtime(store, invocations).await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::ReadOnly).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run shell"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "read_only must refuse the tool"
    );

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(stored.approvals.is_empty(), "read_only must not mint approval");
    let denied = stored.events.iter().find(|event| {
        matches!(&event.event, apeireth_runtime::canonical::SessionEventKind::GovernanceDenied { hook, reason, .. } if hook == "permission_preset" && reason.contains("只读"))
    });
    assert!(denied.is_some(), "expected a Chinese read_only denial event");
}

#[tokio::test]
async fn standard_requires_approval_for_dangerous_tool() {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let invocations = Arc::new(AtomicUsize::new(0));
    let (runtime, invocations) = build_runtime(store, invocations).await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Standard).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run shell"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("expected PendingApproval");
    };
    assert_eq!(view.tool_name, "shell");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "standard must wait for approval"
    );
    assert!(
        view.command_text.starts_with("shell:") && view.command_text.contains("echo hi"),
        "{:?}",
        view.command_text
    );
    assert!(
        view.arguments_summary.contains("echo hi"),
        "{:?}",
        view.arguments_summary
    );
}

#[tokio::test]
async fn full_allows_dangerous_tool_without_approval() {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let invocations = Arc::new(AtomicUsize::new(0));
    let (runtime, invocations) = build_runtime(store, invocations).await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Full).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run shell"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "full must dispatch without approval"
    );

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(stored.approvals.is_empty(), "full must not mint approval");
}
