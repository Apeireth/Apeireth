//! 受控文件写入 (`tool.apply_patch`) 的三档授权语义 + 审批卡浮出链回归 (内测整改)。
//!
//! 同一把生产补丁工具在三档会话预设下必须产生三种结局:
//! `read_only` 拒绝 / `standard` 冻结待批 (审批卡浮出) / `full` 放行且真落盘。
//! 审批卡链: 冻结即挂起 —— `TurnOutcome::PendingApproval` 带出待批视图
//! (工具名/能力 id/命令文本), 人工批准后恢复执行、真写盘; 拒绝则零落盘。
//! 单调边界不随档位放宽: full 档的工作区外补丁照拒 (错误帧)。

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
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, InMemorySessionStore, PermissionPreset, Runtime,
    SessionStore, TurnOutcome, TurnRequest,
};
use apeireth_runtime_assembly::PermissionPresetGovernanceHook;
use apeireth_tools_canonical::{
    apply_patch_capability, authorized_file_write_policy, ObservedGate, APPLY_PATCH_CAPABILITY_ID,
    APPLY_PATCH_TOOL_NAME,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

fn add_patch(path: &str, body: &str) -> String {
    format!("*** Begin Patch\n*** Add File: {path}\n+{body}\n*** End Patch")
}

fn apply_patch_call(patch: String) -> ToolCall {
    ToolCall {
        id: "call_patch_1".into(),
        name: APPLY_PATCH_TOOL_NAME.into(),
        arguments: serde_json::json!({ "patch": patch }),
    }
}

struct FakeProvider {
    id: CapabilityId,
    calls: std::sync::atomic::AtomicUsize,
    tool_call: ToolCall,
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
        use std::sync::atomic::Ordering;
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
                tool_calls: vec![self.tool_call.clone()],
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

/// 会话预设 + 生产写入工具的真实装配: 治理链 = 授权策略 (grant +
/// require_approval 标记, 即「每次写入都要人批」档) 外裹会话预设钩子。
async fn build_runtime(
    root: std::path::PathBuf,
    store: Arc<dyn SessionStore>,
    tool_call: ToolCall,
) -> (Runtime, Arc<FakeProvider>) {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool(APPLY_PATCH_CAPABILITY_ID.into()));
    policy.require_approval_for(APPLY_PATCH_CAPABILITY_ID);

    let pipeline = GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy)));
    let governance = Arc::new(PermissionPresetGovernanceHook::new(
        Arc::new(pipeline),
        store.clone(),
    ));

    let tool = apply_patch_capability(
        root,
        Arc::new(ObservedGate::new()),
        authorized_file_write_policy(),
    );
    let provider = Arc::new(FakeProvider {
        id: CapabilityId::new("provider.fake").unwrap(),
        calls: std::sync::atomic::AtomicUsize::new(0),
        tool_call,
    });

    let runtime = Runtime::builder()
        .with_session_store(store)
        .with_governance(governance)
        .with_capability(tool)
        .with_plugin(Arc::new(ProviderPlugin {
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
            provider: provider.clone(),
        }))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();
    (runtime, provider)
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

/// read_only 档: 写入工具拒绝即帧, 零落盘、零审批实体。
#[tokio::test]
async fn read_only_preset_refuses_the_file_write_tool() {
    let root = tempfile::tempdir().unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        apply_patch_call(add_patch("out.txt", "hello")),
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::ReadOnly).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "write a file"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    assert!(
        !root.path().join("out.txt").exists(),
        "read_only 下写入绝不落盘"
    );
    assert_eq!(
        provider.calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "拒绝后模型仍可继续收束回合"
    );
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(
        stored.approvals.is_empty(),
        "read_only 拒绝不铸造审批实体: {:?}",
        stored.approvals
    );
}

/// standard 档: 冻结即挂起 —— 审批卡浮出 (待批视图带工具名/能力 id/命令文本),
/// 批准后恢复执行且真落盘。
#[tokio::test]
async fn standard_preset_freezes_the_write_at_a_pending_approval_card() {
    let root = tempfile::tempdir().unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        apply_patch_call(add_patch("out.txt", "hello from approved write")),
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Standard).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "write a file"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("standard 档的写入必须冻结待批");
    };
    // 审批卡载荷: 本地审批面板 / 聊天内「等待批准」帧消费同一视图。
    assert_eq!(view.capability_id.as_str(), APPLY_PATCH_CAPABILITY_ID);
    assert_eq!(view.tool_name, APPLY_PATCH_TOOL_NAME);
    assert_eq!(view.tool_call.id, "call_patch_1");
    assert!(
        !view.command_text.trim().is_empty(),
        "审批卡必须有可读命令文本: {:?}",
        view.command_text
    );
    assert!(
        !view.arguments_summary.trim().is_empty(),
        "审批卡必须有参数摘要: {:?}",
        view.arguments_summary
    );
    assert!(
        !view.governance_reason.trim().is_empty(),
        "审批卡必须带出治理理由: {:?}",
        view.governance_reason
    );
    assert!(!root.path().join("out.txt").exists(), "冻结期间绝不落盘");
    assert_eq!(
        provider.calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "挂起期间不得再唤 provider"
    );

    // 人工批准 → 恢复执行 → 真落盘。
    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("批准后必须恢复并收束回合");
    };
    assert_eq!(response.text, "done");
    assert_eq!(
        std::fs::read_to_string(root.path().join("out.txt")).unwrap(),
        "hello from approved write",
        "批准后写入必须真落盘"
    );
}

/// standard 档拒绝: 零落盘, 回合仍可收束 (审批四态闭合的拒绝态)。
#[tokio::test]
async fn standard_preset_rejection_never_writes() {
    let root = tempfile::tempdir().unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, _provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        apply_patch_call(add_patch("out.txt", "must not land")),
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Standard).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "write a file"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("standard 档的写入必须冻结待批");
    };
    let resolution = runtime
        .resolve_approval(
            session,
            view.approval_id,
            ApprovalDecision::Reject { reason: None },
        )
        .await
        .unwrap();
    assert!(matches!(
        resolution,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));
    assert!(!root.path().join("out.txt").exists(), "拒绝后零落盘");
}

/// full 档: 写入放行且**真落盘**, 不再逐次审批 (授权映射漏接时的病灶面)。
#[tokio::test]
async fn full_preset_releases_the_write_and_it_lands() {
    let root = tempfile::tempdir().unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, _provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        apply_patch_call(add_patch("out.txt", "hello full release")),
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Full).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "write a file"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    assert_eq!(
        std::fs::read_to_string(root.path().join("out.txt")).unwrap(),
        "hello full release",
        "full 档写入必须真落盘"
    );
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(
        stored.approvals.is_empty(),
        "full 档不挂起审批: {:?}",
        stored.approvals
    );
}

/// full 档照穿单调边界: 工作区外补丁拒绝即帧, 零落盘 (放行从不绕过硬边界)。
#[tokio::test]
async fn full_preset_still_refuses_workspace_escapes() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, _provider) = build_runtime(
        root.clone(),
        store,
        apply_patch_call(add_patch("../escape.txt", "pwned")),
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Full).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "write a file"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    assert!(
        !base.path().join("escape.txt").exists() && !root.join("escape.txt").exists(),
        "工作区外/越界补丁在 full 档也绝不落盘"
    );
}
