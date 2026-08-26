//! End-to-end Trusted Shell approval/resume proof through the canonical
//! runtime, governance, plugin registry, and real ProcessExecutor-backed shell
//! tool. Only harmless platform-native echo commands are executed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::{
    GovernancePipeline, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, Runtime, TurnOutcome, TurnRequest,
};
use apeireth_tools_canonical::{BuiltinToolsOptions, BuiltinToolsPlugin, TrustedShellConfig};
use async_trait::async_trait;
use tempfile::tempdir;

const MODEL: &str = "fake-model-1";

struct FakeProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    first_tool_call: Option<ToolCall>,
}

impl FakeProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
            first_tool_call: None,
        })
    }

    fn with_first_tool_call(call: ToolCall) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
            first_tool_call: Some(call),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
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
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };

        if index == 0 {
            if let Some(call) = &self.first_tool_call {
                return Ok(NormalizedResponse {
                    finish_reason: Some(
                        apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                    ),
                    tool_calls: vec![call.clone()],
                    ..base
                });
            }

            #[cfg(windows)]
            let command = "echo shell_approval_e2e_ok";
            #[cfg(not(windows))]
            let command = "printf 'shell_approval_e2e_ok'";

            Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: "call_shell_1".into(),
                    name: "shell".into(),
                    arguments: serde_json::json!({ "command": command }),
                }],
                ..base
            })
        } else {
            Ok(NormalizedResponse {
                content: "shell done".into(),
                ..base
            })
        }
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
                "Scripted provider for shell approval e2e",
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

#[tokio::test]
async fn trusted_shell_pending_approval_then_approve_executes_once_and_continues() {
    let tmp = tempdir().unwrap();
    let provider = FakeProvider::new();
    let shell_config = TrustedShellConfig::new(tmp.path().to_path_buf());
    let tools_plugin = BuiltinToolsPlugin::with_options(
        tmp.path().to_path_buf(),
        BuiltinToolsOptions {
            shell: Some(shell_config),
            fetch: None,
        },
    );

    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.shell".into()));
    policy.require_approval_for("tool.shell");

    let runtime = Runtime::builder()
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(Arc::new(tools_plugin))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "please run a shell command"))
        .await
        .unwrap();

    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("expected PendingApproval");
    };

    assert_eq!(view.capability_id.as_str(), "tool.shell");
    assert_eq!(view.tool_name, "shell");
    assert_eq!(view.tool_call.id, "call_shell_1");
    assert!(view
        .tool_call
        .arguments
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap()
        .contains("shell_approval_e2e_ok"));
    assert_eq!(
        provider.call_count(),
        1,
        "provider must not be recalled while paused"
    );

    let effective = view
        .effective_invocation
        .as_ref()
        .expect("shell freezes invocation");
    assert_eq!(effective["environment_mode"], "explicit_minimal");
    let expected_cwd = tmp
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(
        effective["cwd"].as_str().unwrap().contains(&expected_cwd)
            || expected_cwd.contains(effective["cwd"].as_str().unwrap()),
        "effective cwd {:?} should match canonical workspace root {expected_cwd:?}",
        effective["cwd"]
    );
    assert_eq!(effective["filesystem_isolation"], "Unsupported");
    assert_eq!(effective["network_isolation"], "Unsupported");

    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();

    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("expected Resumed(Completed)");
    };
    assert_eq!(response.text, "shell done");
    assert_eq!(provider.call_count(), 2);

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert_eq!(stored.active_approval_id, None);
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        apeireth_runtime::canonical::ApprovalStatus::Consumed
    );
}

#[tokio::test]
async fn trusted_shell_reject_never_executes_and_model_recovers() {
    let tmp = tempdir().unwrap();
    let provider = FakeProvider::new();
    let tools_plugin = BuiltinToolsPlugin::with_options(
        tmp.path().to_path_buf(),
        BuiltinToolsOptions {
            shell: Some(TrustedShellConfig::new(tmp.path().to_path_buf())),
            fetch: None,
        },
    );

    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.shell".into()));
    policy.require_approval_for("tool.shell");

    let runtime = Runtime::builder()
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(Arc::new(tools_plugin))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "please run a shell command"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("expected PendingApproval");
    };

    let resolution = runtime
        .resolve_approval(
            session,
            view.approval_id,
            ApprovalDecision::Reject { reason: None },
        )
        .await
        .unwrap();

    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("expected Resumed(Completed)");
    };
    assert_eq!(response.text, "shell done");
    assert_eq!(
        provider.call_count(),
        2,
        "model gets a chance to recover after rejection"
    );
}

#[tokio::test]
async fn invalid_shell_request_never_creates_pending_approval() {
    let tmp = tempdir().unwrap();
    let invalid_call = ToolCall {
        id: "call_shell_invalid".into(),
        name: "shell".into(),
        arguments: serde_json::json!({ "command": "echo hi", "cwd": "missing_dir" }),
    };
    let provider = FakeProvider::with_first_tool_call(invalid_call);
    let tools_plugin = BuiltinToolsPlugin::with_options(
        tmp.path().to_path_buf(),
        BuiltinToolsOptions {
            shell: Some(TrustedShellConfig::new(tmp.path().to_path_buf())),
            fetch: None,
        },
    );

    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.shell".into()));
    policy.require_approval_for("tool.shell");

    let runtime = Runtime::builder()
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(Arc::new(tools_plugin))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(
            session,
            "please run an invalid shell command",
        ))
        .await
        .unwrap();

    match outcome {
        TurnOutcome::Completed(response) => assert_eq!(response.text, "shell done"),
        TurnOutcome::PendingApproval(view) => {
            panic!("invalid shell must not create PendingApproval: {view:?}")
        }
    }

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert!(
        stored.approvals.is_empty(),
        "invalid shell request must not mint a pending approval"
    );
    assert!(stored.events.iter().all(|event| !matches!(
        &event.event,
        apeireth_runtime::canonical::SessionEventKind::ApprovalRequired { .. }
    )));
}
