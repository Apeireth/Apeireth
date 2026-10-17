//! git 写分级 (`tool.git_write`) 的会话预设三档语义 + L2 永远人批 + 审批卡
//! 载荷回归 (D-P1「git 写分级上架」)。
//!
//! 同一把分级工具在三档会话预设下必须产生三种结局: `read_only` 拒绝 /
//! `standard` 冻结待批 (审批卡带分级标注浮出) / `full` 放行 L1 且真落地。
//! **L2 出网永不因预设放行** (`full` 不放, 免审记忆也不跳) —— 人批 push 仍走
//! IM 审批卡同链, 但每次都得人批。

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
use apeireth_runtime_assembly::{
    roster_from_config, PermissionPresetGovernanceHook, ProductionBackends, ProductionModules,
    ProductionModulesConfig,
};
use apeireth_tools_canonical::{
    authorized_git_write_policy, git_write_capability, GitWriteReleaseTier,
    GitWriteTierApprovalHook, GIT_WRITE_CAPABILITY_ID, GIT_WRITE_TOOL_NAME,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

fn git_write_call(command: &str) -> ToolCall {
    ToolCall {
        id: "call_git_write_1".into(),
        name: GIT_WRITE_TOOL_NAME.into(),
        arguments: serde_json::json!({ "command": command }),
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
                prompt_cache_hit_tokens: None,
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

/// 会话预设 + 生产分级工具的真实装配: 治理链 = 授权策略 (grant) + 分级风险
/// 钩子 (L1 默认人批 / L2 永远人批 / L3 拒绝即帧), 外裹会话预设钩子。
async fn build_runtime(
    root: std::path::PathBuf,
    store: Arc<dyn SessionStore>,
    tool_call: ToolCall,
    release: GitWriteReleaseTier,
) -> (Runtime, Arc<FakeProvider>) {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool(GIT_WRITE_CAPABILITY_ID.into()));

    let pipeline = GovernancePipeline::new()
        .with(Arc::new(PermissionGovernanceHook::new(policy)))
        .with(Arc::new(GitWriteTierApprovalHook::new(release)));
    let governance = Arc::new(PermissionPresetGovernanceHook::new(
        Arc::new(pipeline),
        store.clone(),
    ));

    let tool = git_write_capability(root, authorized_git_write_policy());
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

/// read_only 档: git 写工具拒绝即帧, 零落盘、零审批实体。
#[tokio::test]
async fn read_only_preset_refuses_the_git_write_tool() {
    let root = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    assert!(
        run(&["init", "-q"])
            && run(&["config", "user.email", "d-p1@test.local"])
            && run(&["config", "user.name", "D-P1 Test"])
    );
    std::fs::write(root.path().join("a.txt"), "hello").unwrap();

    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        git_write_call("git add a.txt && git commit -m \"must not land\""),
        GitWriteReleaseTier::RequireApprovalEveryWrite,
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::ReadOnly).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "commit the change"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    // 没有提交产生 (仓库里没有 commit)。
    let log = std::process::Command::new("git")
        .arg("-C")
        .arg(root.path())
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&log.stdout).trim().is_empty(),
        "read_only 下绝不产生提交"
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

/// standard 档: L1 冻结待批 —— 审批卡载荷带命令文本 + 分级标注;
/// 批准后恢复执行、真产生提交。
#[tokio::test]
async fn standard_preset_freezes_l1_at_a_tier_annotated_approval_card() {
    let root = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    assert!(
        run(&["init", "-q"])
            && run(&["config", "user.email", "d-p1@test.local"])
            && run(&["config", "user.name", "D-P1 Test"])
    );
    std::fs::write(root.path().join("a.txt"), "hello").unwrap();

    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        git_write_call("git add a.txt && git commit -m \"approved commit\""),
        GitWriteReleaseTier::RequireApprovalEveryWrite,
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Standard).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "commit the change"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("standard 档的 L1 必须冻结待批");
    };
    // 审批卡载荷: 命文文本 + 分级标注 (本地面板 / IM 审批卡消费同一视图)。
    assert_eq!(view.capability_id.as_str(), GIT_WRITE_CAPABILITY_ID);
    assert_eq!(view.tool_name, GIT_WRITE_TOOL_NAME);
    assert!(
        view.command_text.contains("git add a.txt") && view.command_text.contains("git commit -m"),
        "审批卡必须有可读命令文本: {:?}",
        view.command_text
    );
    assert!(
        !view.arguments_summary.trim().is_empty(),
        "审批卡必须有参数摘要: {:?}",
        view.arguments_summary
    );
    assert!(
        view.governance_reason.contains("L1"),
        "审批卡必须带分级标注: {:?}",
        view.governance_reason
    );
    assert!(
        view.governance_reason.contains("reversible-local"),
        "审批卡分级标注必须可读: {:?}",
        view.governance_reason
    );
    // 冻结期间不落地。
    let log = |root: &std::path::Path| {
        String::from_utf8_lossy(
            &std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["log", "--oneline"])
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string()
    };
    assert!(log(root.path()).is_empty(), "冻结期间绝不产生提交");
    assert_eq!(
        provider.calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "挂起期间不得再唤 provider"
    );

    // 人工批准 → 恢复执行 → 真产生提交。
    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("批准后必须恢复并收束回合");
    };
    assert_eq!(response.text, "done");
    assert!(
        log(root.path()).contains("approved commit"),
        "批准后必须真产生提交"
    );
}

/// 审批卡脱敏: remote URL 凭据不入审批卡 (命令文本 + 摘要 + 理由全查),
/// 且 L2 分级标注同卡浮出 (手机上批 push 的同一张卡)。
#[tokio::test]
async fn approval_card_redacts_remote_url_credentials_and_labels_l2() {
    let root = tempfile::tempdir().unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, _provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        git_write_call("git remote add origin https://user:tok123456@host.example/repo.git"),
        GitWriteReleaseTier::RequireApprovalEveryWrite,
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Standard).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "add the remote"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("L2 必须冻结待批");
    };
    assert!(
        view.governance_reason.contains("L2"),
        "审批卡必须带 L2 分级标注: {:?}",
        view.governance_reason
    );
    for surface in [
        &view.command_text,
        &view.arguments_summary,
        &view.governance_reason,
    ] {
        assert!(!surface.contains("tok123456"), "{surface:?}");
        assert!(!surface.contains("user:"), "{surface:?}");
    }
    assert!(
        view.command_text.contains("[REDACTED_URL_CREDENTIAL]"),
        "命令文本应留脱敏标记: {:?}",
        view.command_text
    );
}

/// full 档: L1 放行且真落地 (不再逐次审批)。
#[tokio::test]
async fn full_preset_releases_l1_and_the_commit_lands() {
    let root = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    assert!(
        run(&["init", "-q"])
            && run(&["config", "user.email", "d-p1@test.local"])
            && run(&["config", "user.name", "D-P1 Test"])
    );
    std::fs::write(root.path().join("a.txt"), "hello").unwrap();

    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, _provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        git_write_call("git add a.txt && git commit -m \"full release\""),
        GitWriteReleaseTier::RequireApprovalEveryWrite,
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Full).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "commit the change"))
        .await
        .unwrap();
    assert_eq!(completed_text(&outcome), "done");
    let log_output = std::process::Command::new("git")
        .arg("-C")
        .arg(root.path())
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&log_output.stdout);
    assert!(log.contains("full release"), "full 档 L1 必须真落地: {log}");
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(
        stored.approvals.is_empty(),
        "full 档不挂起审批: {:?}",
        stored.approvals
    );
}

/// L2 永远人批 (①): `full` 档也不放 —— push 冻结待批, 不自动放行。
#[tokio::test]
async fn l2_never_auto_passes_under_the_full_preset() {
    let root = tempfile::tempdir().unwrap();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (runtime, _provider) = build_runtime(
        root.path().to_path_buf(),
        store,
        git_write_call("git push origin main"),
        GitWriteReleaseTier::RequireApprovalEveryWrite,
    )
    .await;
    let session = SessionId::new();
    set_preset(&runtime, session, PermissionPreset::Full).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "push the branch"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("L2 在 full 档也必须冻结待批 (L2 永不因预设放行)");
    };
    assert!(
        view.governance_reason.contains("L2")
            && view.governance_reason.contains("never auto-passes"),
        "L2 分级标注必须写明永不自动放行: {:?}",
        view.governance_reason
    );
}

/// L2 永远人批 (②): `approval_remember` 免审记忆也不越过地板 —— 同一 push
/// 批过一次, 下一次仍要人批。
#[tokio::test]
async fn l2_approval_memory_never_skips_the_human() {
    use apeireth_core::kernel::TraceId;
    use apeireth_governance::{Action, GovernanceHook, GovernanceRequest};

    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let session = SessionId::new();
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool(GIT_WRITE_CAPABILITY_ID.into()));
    let pipeline = GovernancePipeline::new()
        .with(Arc::new(PermissionGovernanceHook::new(policy)))
        .with(Arc::new(GitWriteTierApprovalHook::new(
            GitWriteReleaseTier::RequireApprovalEveryWrite,
        )));
    let hook = PermissionPresetGovernanceHook::new(Arc::new(pipeline), store.clone());

    let clock = apeireth_core::kernel::system_clock();
    let mut stored = apeireth_runtime::canonical::Session::new(session, clock.as_ref());
    stored.settings.permission_preset = PermissionPreset::Standard;
    stored.settings.approval_remember = true;
    store.save(&stored).await.unwrap();

    let capability = CapabilityId::new(GIT_WRITE_CAPABILITY_ID).unwrap();
    let arguments = serde_json::json!({ "command": "git push origin main" });
    let request = || {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            session,
            TraceId::new(),
            1,
        )
    };

    // 第一次: 待批 → 人批 → 记忆种入 (session, capability, 授权对象;
    // E-P2 起第三键是族集/精确操作指纹, 见 permission_preset.rs::authorization_key)。
    let first = hook.evaluate_verbose(&request()).await;
    assert!(
        matches!(
            first.decision,
            apeireth_governance::Decision::RequireApproval { .. }
        ),
        "第一次必须待批: {:?}",
        first.decision
    );
    hook.approval_resolved(&session, &capability);

    // 第二次: 同参数同会话 —— L2 地板在, 免审记忆不得跳人。
    let second = hook.evaluate_verbose(&request()).await;
    assert!(
        matches!(
            second.decision,
            apeireth_governance::Decision::RequireApproval { .. }
        ),
        "L2 永远人批, 免审记忆不得越过地板: {:?}",
        second.decision
    );
}

/// 装配接线: 主开关开 = 注册可见; 默认关 / 无工作区根 = 不注册;
/// 自省名册照实报告两个开关的生效值。
#[tokio::test]
async fn git_write_registers_only_when_enabled_and_reports_roster_switches() {
    let clock = apeireth_core::kernel::system_clock();
    let base = || ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    };
    let backends = || ProductionBackends {
        workspace_root: Some(std::path::PathBuf::from(".")),
        ..ProductionBackends::default()
    };

    let mut config = base();
    config.git_write = true;
    config.git_write_auto_pass_l1 = true;
    let modules = ProductionModules::build(config.clone(), backends(), clock.clone())
        .expect("assembly builds");
    let ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect();
    assert!(
        ids.iter().any(|id| id == GIT_WRITE_CAPABILITY_ID),
        "git 写工具必须注册: {ids:?}"
    );

    // 默认关 (opt-in): 不注册。
    let off = ProductionModules::build(base(), backends(), clock.clone()).expect("assembly builds");
    assert!(off
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != GIT_WRITE_CAPABILITY_ID));

    // 无工作区根 = 注册条件不成立。
    let mut no_root = base();
    no_root.git_write = true;
    let without_root =
        ProductionModules::build(no_root, ProductionBackends::default(), clock.clone())
            .expect("assembly builds");
    assert!(without_root
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != GIT_WRITE_CAPABILITY_ID));

    // 自省名册: 两个开关照实报告 (子档依赖主开关)。
    let roster = roster_from_config(&config, true, false, false, true, &[]);
    let git_rows: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("git_write"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        git_rows,
        vec![("git_write", true), ("git_write_auto_pass_l1", true)]
    );
    let mut sub_only = base();
    sub_only.git_write_auto_pass_l1 = true;
    let roster = roster_from_config(&sub_only, true, false, false, true, &[]);
    let git_rows: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("git_write"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        git_rows,
        vec![("git_write", false), ("git_write_auto_pass_l1", false)]
    );
}
