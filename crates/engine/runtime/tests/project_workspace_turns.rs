use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    CapabilityKind, FrozenInvocation, Plugin, PluginContext, PluginManifest, PluginResult,
    ProviderCapability, ProviderError, ToolCapability, ToolExecutionContext,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, ToolCall, ToolResult,
};
use apeireth_runtime::canonical::workspace::{
    FileWorkspaceRegistry, WorkspaceCreateRequest, WorkspacePermissionScope, WorkspaceRegistry,
};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, ProjectWorkspaceBinding, Runtime, RuntimeError, Session,
    TurnOutcome, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "project-workspace-test";
const READ: &str = "project_files_read";
const WRITE: &str = "project_files_write";
const AMBIENT: &str = "ambient_tool";

struct Script {
    manifest: PluginManifest,
    id: CapabilityId,
    steps: Mutex<VecDeque<Vec<&'static str>>>,
    catalogs: Mutex<Vec<Vec<String>>>,
}

impl Script {
    fn new(steps: impl IntoIterator<Item = Vec<&'static str>>) -> Arc<Self> {
        let id = CapabilityId::new("provider.project_workspace_test").unwrap();
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.project_workspace").unwrap(),
                "1.0.0",
                "Project workspace turn tests",
            )
            .declare_capability(id.clone(), CapabilityKind::Provider, "Scripted provider")
            .unwrap(),
            id,
            steps: Mutex::new(steps.into_iter().collect()),
            catalogs: Mutex::default(),
        })
    }
}

struct ScriptPlugin(Arc<Script>);

#[async_trait]
impl Plugin for ScriptPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.0.manifest
    }

    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.0.clone()]
    }
}

#[async_trait]
impl ProviderCapability for Script {
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
        let mut catalogs = self.catalogs.lock().unwrap();
        catalogs.push(request.tools.iter().map(|tool| tool.name.clone()).collect());
        let mut response =
            NormalizedResponse::text(format!("reply-{}", catalogs.len()), MODEL, "done");
        for tool in self
            .steps
            .lock()
            .unwrap()
            .pop_front()
            .expect("script exhausted")
        {
            response.tool_calls.push(ToolCall {
                id: format!("{}-{tool}", catalogs.len()),
                name: tool.into(),
                arguments: serde_json::json!({}),
            });
        }
        if !response.tool_calls.is_empty() {
            response.finish_reason = Some(NormalizedFinishReason::ToolCalls);
        }
        Ok(response)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Invocation {
    name: &'static str,
    workspace: Option<ProjectWorkspaceBinding>,
    approved: bool,
}

type Invocations = Arc<Mutex<Vec<Invocation>>>;

struct Recorder {
    id: CapabilityId,
    name: &'static str,
    invoked: Invocations,
}

#[async_trait]
impl ToolCapability for Recorder {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(self.name)
    }

    async fn invoke(&self, _: &ToolCall) -> ToolResult {
        panic!("the dispatcher must supply a tool execution context")
    }

    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        let workspace = context
            .authorization
            .host_context
            .as_deref()
            .and_then(|value| value.downcast_ref::<ProjectWorkspaceBinding>())
            .copied();
        self.invoked.lock().unwrap().push(Invocation {
            name: self.name,
            workspace,
            approved: context.authorization.approved,
        });
        ToolResult::ok(&call.id, serde_json::json!({"executed": true}))
    }
}

fn create(
    registry: &FileWorkspaceRegistry,
    permission: WorkspacePermissionScope,
) -> ProjectWorkspaceBinding {
    let workspace = registry
        .create_desktop_workspace("Same name", permission, 1)
        .unwrap();
    ProjectWorkspaceBinding {
        workspace_id: workspace.workspace_id,
        revision: workspace.revision,
    }
}

async fn runtime(
    registry: Arc<FileWorkspaceRegistry>,
    script: &Arc<Script>,
) -> (Runtime, Invocations) {
    let invoked = Invocations::default();
    let mut builder = Runtime::builder()
        .with_workspace_registry(Some(registry))
        .with_governance(Arc::new(AllowAll))
        .with_plan_mode_enabled(true)
        .with_default_model(MODEL)
        .with_plugin(Arc::new(ScriptPlugin(script.clone())));
    for name in [READ, WRITE, AMBIENT] {
        builder = builder.with_tool(Arc::new(Recorder {
            id: CapabilityId::new(format!("tool.{name}")).unwrap(),
            name,
            invoked: invoked.clone(),
        }));
    }
    (builder.build().await.unwrap(), invoked)
}

#[tokio::test]
async fn a_project_advertises_and_dispatches_only_its_local_authorized_tools() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(FileWorkspaceRegistry::open(dir.path()).unwrap());
    let a = create(&registry, WorkspacePermissionScope::Read);
    let b = create(&registry, WorkspacePermissionScope::Read);
    let shared = registry
        .create(WorkspaceCreateRequest::managed(
            dir.path(),
            "Old shared root",
            WorkspacePermissionScope::Read,
            1,
        ))
        .unwrap();
    let script = Script::new([
        vec![AMBIENT, WRITE, "exit_plan_mode", READ],
        vec![],
        vec![READ, AMBIENT],
        vec![],
    ]);
    let (runtime, invoked) = runtime(registry, &script).await;
    let scoped = SessionId::new();
    assert!(matches!(
        runtime
            .execute(
                TurnRequest::new(scoped, "shared root").with_project_workspace(
                    ProjectWorkspaceBinding {
                        workspace_id: shared.workspace_id,
                        revision: shared.revision,
                    },
                )
            )
            .await,
        Err(RuntimeError::ProjectWorkspaceRejected { .. })
    ));
    assert!(!runtime.sessions().contains(&scoped).await.unwrap());
    runtime
        .execute(TurnRequest::new(scoped, "project turn").with_project_workspace(a))
        .await
        .unwrap();
    let stored = runtime.sessions().load(&scoped).await.unwrap().unwrap();
    assert_eq!(stored.project_workspace_binding(), Some(a));
    let recovered: Session =
        serde_json::from_value(serde_json::to_value(&stored).unwrap()).unwrap();
    assert_eq!(recovered.project_workspace_binding(), Some(a));

    for bypass in [
        TurnRequest::new(scoped, "ambient bypass"),
        TurnRequest::new(scoped, "wrong workspace").with_project_workspace(b),
    ] {
        assert!(matches!(
            runtime.execute(bypass).await,
            Err(RuntimeError::ProjectWorkspaceRejected { .. })
        ));
    }
    assert_eq!(
        runtime
            .sessions()
            .load(&scoped)
            .await
            .unwrap()
            .unwrap()
            .revision,
        stored.revision
    );
    assert_eq!(script.catalogs.lock().unwrap().len(), 2);

    let legacy = SessionId::new();
    runtime
        .execute(TurnRequest::new(legacy, "legacy turn"))
        .await
        .unwrap();
    assert!(matches!(
        runtime
            .execute(TurnRequest::new(legacy, "bind the old transcript").with_project_workspace(a))
            .await,
        Err(RuntimeError::ProjectWorkspaceRejected { .. })
    ));
    let catalogs = script.catalogs.lock().unwrap();
    assert_eq!(catalogs[0], [READ]);
    assert_eq!(catalogs[1], [READ]);
    assert!(catalogs[2].iter().any(|name| name == AMBIENT));
    assert!(!catalogs[2].iter().any(|name| name == READ || name == WRITE));
    assert_eq!(
        &*invoked.lock().unwrap(),
        &[
            Invocation {
                name: READ,
                workspace: Some(a),
                approved: false
            },
            Invocation {
                name: AMBIENT,
                workspace: None,
                approved: false
            },
        ]
    );
}

#[tokio::test]
async fn approval_resume_preserves_binding_and_a_revoked_revision_cannot_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(FileWorkspaceRegistry::open(dir.path()).unwrap());
    let a = create(&registry, WorkspacePermissionScope::ApprovalWrite);
    let b = create(&registry, WorkspacePermissionScope::ApprovalWrite);
    let script = Script::new([
        vec![WRITE],
        vec![],
        vec![WRITE],
        vec![WRITE],
        vec![AMBIENT],
        vec![],
    ]);
    let (runtime, invoked) = runtime(registry.clone(), &script).await;

    let start = |session, binding| {
        TurnRequest::new(session, "write this file").with_project_workspace(binding)
    };
    let allowed = SessionId::new();
    let TurnOutcome::PendingApproval(pending) =
        runtime.execute_outcome(start(allowed, a)).await.unwrap()
    else {
        panic!("a project write must require human approval");
    };
    assert!(invoked.lock().unwrap().is_empty());
    // Rebuild the persisted approval by serializing the open session before resolution.
    let stored = runtime.sessions().load(&allowed).await.unwrap().unwrap();
    let restored: Session = serde_json::from_value(serde_json::to_value(&stored).unwrap()).unwrap();
    runtime.sessions().save(&restored).await.unwrap();
    assert!(matches!(
        runtime
            .resolve_approval(allowed, pending.approval_id, ApprovalDecision::Approve)
            .await
            .unwrap(),
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));

    let revoked = SessionId::new();
    let TurnOutcome::PendingApproval(pending) =
        runtime.execute_outcome(start(revoked, b)).await.unwrap()
    else {
        panic!("a project write must require human approval");
    };
    registry.close(&b.workspace_id, b.revision, 2).unwrap();
    assert!(matches!(
        runtime
            .resolve_approval(revoked, pending.approval_id, ApprovalDecision::Approve)
            .await
            .unwrap(),
        ApprovalResolution::Expired
    ));
    assert!(matches!(
        runtime.execute(start(revoked, b)).await,
        Err(RuntimeError::ProjectWorkspaceRejected { .. })
    ));
    assert!(runtime
        .verify_project_workspace_session(revoked, b)
        .await
        .is_err());
    let rejected = SessionId::new();
    let TurnOutcome::PendingApproval(pending) =
        runtime.execute_outcome(start(rejected, a)).await.unwrap()
    else {
        panic!("a project write must require human approval");
    };
    assert!(matches!(
        runtime
            .resolve_approval(
                rejected,
                pending.approval_id,
                ApprovalDecision::Reject { reason: None }
            )
            .await
            .unwrap(),
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));
    let catalogs = script.catalogs.lock().unwrap();
    assert_eq!(catalogs.len(), 6);
    for catalog in catalogs.iter() {
        assert_eq!(catalog, &[READ, WRITE]);
    }
    assert_eq!(
        &*invoked.lock().unwrap(),
        &[Invocation {
            name: WRITE,
            workspace: Some(a),
            approved: true
        },]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn competing_project_claims_for_a_blank_session_bind_exactly_once() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(FileWorkspaceRegistry::open(dir.path()).unwrap());
    let a = create(&registry, WorkspacePermissionScope::Read);
    let b = create(&registry, WorkspacePermissionScope::Read);
    let script = Script::new([vec![]]);
    let (runtime, _) = runtime(registry, &script).await;
    let runtime = Arc::new(runtime);
    let session_id = SessionId::new();
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let claim = |binding| {
        let runtime = runtime.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            runtime
                .execute(TurnRequest::new(session_id, "claim").with_project_workspace(binding))
                .await
        })
    };
    let (left, right) = tokio::join!(claim(a), claim(b));
    let winning_binding = match (left.unwrap(), right.unwrap()) {
        (Ok(_), Err(RuntimeError::ProjectWorkspaceRejected { .. })) => a,
        (Err(RuntimeError::ProjectWorkspaceRejected { .. }), Ok(_)) => b,
        _ => panic!("expected one accepted claim"),
    };
    runtime
        .verify_project_workspace_session(session_id, winning_binding)
        .await
        .unwrap();
    assert_eq!(script.catalogs.lock().unwrap().len(), 1);
}
