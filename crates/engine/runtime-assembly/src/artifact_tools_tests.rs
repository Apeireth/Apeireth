use super::*;
use apeireth_core::kernel::{system_clock, AgentId, PluginId, RequestId, SessionId, TraceId};
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_plugin::{ToolAuthorization, ToolCaller, ToolCancellation};

fn context(call: &ToolCall) -> ToolExecutionContext {
    ToolExecutionContext {
        agent: AgentId::new(),
        principal: None,
        session: Some(SessionId::new()),
        turn: Some(RequestId::new()),
        trace: Some(TraceId::new()),
        call_id: call.id.clone(),
        caller: ToolCaller::Model,
        module: PluginId::new("artifact.test").unwrap(),
        module_instance: None,
        internal_review: None,
        message_support: false,
        message_route: None,
        delegated_tools: None,
        delegation: None,
        tool_scope: None,
        authorization: ToolAuthorization::default(),
        cancellation: ToolCancellation::default(),
        deadline: Instant::now() + Duration::from_secs(30),
        module_tasks: ModuleTaskScope::new(),
        resource_lease: None,
    }
}

async fn setup() -> (tempfile::TempDir, Arc<dyn ArtifactService>, ToolCall) {
    let directory = tempfile::tempdir().unwrap();
    let service: Arc<dyn ArtifactService> = Arc::new(
        crate::sqlite_artifacts::SqliteArtifactService::open_with_sealer(directory.path(), None)
            .await
            .unwrap(),
    );
    let store = service
        .capabilities(ArtifactAccess::local_human())
        .await
        .unwrap()
        .store_id;
    let call = ToolCall {
        id: "managed-call-1".into(),
        name: "artifact_write".into(),
        arguments: json!({"action":"create","payload":{"store_id":store,"operation_id":uuid::Uuid::new_v4().to_string(),"work_id":"local-work-2","title":"共同方案","media_type":"text/plain","content":"起草😀\r\n","conversation_session_id":null}}),
    };
    (directory, service, call)
}

#[tokio::test]
async fn governed_tool_and_human_read_share_one_authoritative_version() {
    let (_directory, service, call) = setup().await;
    let tool = ArtifactTool::write(service.clone());
    assert!(matches!(
        tool.execution_policy().permission,
        ToolPermissionPolicy::RequireApproval
    ));
    let mut ctx = context(&call);
    assert!(!tool.invoke(&call).await.is_ok());
    assert!(!tool.invoke_with_context(&call, None, &ctx).await.is_ok());
    ctx.authorization.approved = true;
    let response = tool.invoke_with_context(&call, None, &ctx).await;
    assert!(response.is_ok(), "{}", response.render());
    let value: Value = serde_json::from_str(&response.render()).unwrap();
    let receipt: ArtifactReceipt = serde_json::from_value(value["result"].clone()).unwrap();
    assert_eq!(
        receipt.revision.as_ref().unwrap().author.kind,
        ArtifactAuthorKind::Agent
    );
    assert_eq!(
        receipt.revision.as_ref().unwrap().author.id,
        ctx.agent.to_string()
    );
    assert_eq!(
        receipt
            .revision
            .as_ref()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .request_id,
        ctx.turn.unwrap().to_string()
    );
    let read = service
        .read(
            ArtifactAccess::local_human(),
            receipt.store_id,
            receipt.artifact.artifact_id,
            None,
        )
        .await
        .unwrap();
    assert_eq!(read.revision.content, "起草😀\r\n");
}

#[tokio::test]
async fn missing_turn_cancelled_and_forged_actor_never_write() {
    let (_directory, service, call) = setup().await;
    let tool = ArtifactTool::write(service.clone());
    let mut ctx = context(&call);
    ctx.authorization.approved = true;
    ctx.turn = None;
    assert!(!tool.invoke_with_context(&call, None, &ctx).await.is_ok());
    ctx.turn = Some(RequestId::new());
    ctx.cancellation.cancel();
    assert!(!tool.invoke_with_context(&call, None, &ctx).await.is_ok());
    let mut injected = call.clone();
    injected.arguments["payload"]["author"] = json!({"kind":"human","id":"owner"});
    let mut active = context(&injected);
    active.authorization.approved = true;
    assert!(!tool
        .invoke_with_context(&injected, None, &active)
        .await
        .is_ok());
    let store = service
        .capabilities(ArtifactAccess::local_human())
        .await
        .unwrap()
        .store_id;
    assert!(service
        .list(
            ArtifactAccess::local_human(),
            ArtifactListQuery {
                store_id: store,
                work_id: "local-work-2".into(),
                after: None,
                limit: None
            }
        )
        .await
        .unwrap()
        .artifacts
        .is_empty());
}

#[tokio::test]
async fn active_user_table_revokes_both_tool_and_human_surface() {
    let (directory, service, call) = setup().await;
    let read = ArtifactTool::read(service.clone());
    let cap = ToolCall {
        id: call.id,
        name: "artifact_read".into(),
        arguments: json!({"action":"capabilities","payload":{}}),
    };
    assert!(read
        .invoke_with_context(&cap, None, &context(&cap))
        .await
        .is_ok());
    std::fs::write(directory.path().join("users.json"), "{}").unwrap();
    assert!(!read
        .invoke_with_context(&cap, None, &context(&cap))
        .await
        .is_ok());
    assert_eq!(
        service
            .capabilities(ArtifactAccess::local_human())
            .await
            .unwrap_err(),
        ArtifactError::ScopeUnavailable
    );
}

#[tokio::test]
async fn full_or_remembered_preset_never_removes_artifact_human_approval_floor() {
    use crate::canonical::PermissionPresetGovernanceHook;
    use apeireth_governance::{
        Action, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionGovernanceHook,
        PermissionPolicy,
    };
    use apeireth_runtime::canonical::{
        InMemorySessionStore, PermissionPreset, Session, SessionStore,
    };
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let capability = CapabilityId::new(ARTIFACT_WRITE_CAPABILITY).unwrap();
    let mut permissions = PermissionPolicy::new();
    permissions.grant(Permission::ExecuteTool(ARTIFACT_WRITE_CAPABILITY.into()));
    permissions.require_approval_for(ARTIFACT_WRITE_CAPABILITY);
    let hook = PermissionPresetGovernanceHook::new(
        Arc::new(PermissionGovernanceHook::new(permissions)),
        store.clone(),
    );
    let arguments = json!({"action":"commit","payload":{}});
    for preset in [
        PermissionPreset::ReadOnly,
        PermissionPreset::Standard,
        PermissionPreset::Full,
    ] {
        let id = SessionId::new();
        let mut session = Session::new(id, system_clock().as_ref());
        session.settings.permission_preset = preset;
        session.settings.approval_remember = true;
        store.save(&session).await.unwrap();
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            id,
            TraceId::new(),
            1,
        );
        let before = hook.evaluate(&request).await;
        if matches!(preset, PermissionPreset::ReadOnly) {
            assert!(matches!(before, Decision::Deny { .. }));
        } else {
            assert!(matches!(before, Decision::RequireApproval { .. }));
            hook.approval_resolved(&id, &capability);
            assert!(matches!(
                hook.evaluate(&request).await,
                Decision::RequireApproval { .. }
            ));
        }
    }
}
