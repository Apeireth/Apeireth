//! P3-6 回归: 已批准 / 预批的工具调用**仍然过治理链** ⇒ 热撤销立即生效。
//!
//! 旧行为 (缺陷): `preapproved` 分支 (人批准之后的恢复派发) 只复核冻结审批的
//! 身份 / 指纹 / 动作绑定, **完全不评估治理** ⇒ 面板上的热撤销
//! (`PermissionPolicy::revoke`) 对「已经批准过」的调用**无效** —— 撤销之后
//! 再恢复, 照样执行。
//!
//! 新行为: preapproved 分支与普通路径**同样评估治理**, 但把「需要人工审批」
//! 这一档**降级为「已批准」** (人已经对这一次拍过板, 不再二次索要审批) ⇒
//!
//! ① 未被撤销的已批准调用**照旧执行** (不得因修复而功能回归);
//! ② 被热撤销的已批准调用**不再放行** (拒绝即帧, 零执行, 留审计行)。
//!
//! 夹具口径与 `canonical_approval_lifecycle.rs` 一致: 真实 runtime / 治理链 /
//! 会话库, 只有 provider 与工具是夹具。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{
    GovernancePipeline, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedUsage,
    ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, InMemorySessionStore, Runtime, SessionEventKind,
    SessionStore, TurnOutcome, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";
/// 能力 id (治理授权面用的名字)。
const TOOL_CAPABILITY: &str = "tool.approval_b";
/// 模型侧声明名 (模型在 tool_calls 里点的名字)。
const TOOL_NAME: &str = "approval_b";

// ---------------------------------------------------------------------------
// 夹具: 计数工具 (记录真实执行次数)
// ---------------------------------------------------------------------------

struct CountingTool {
    id: CapabilityId,
    name: &'static str,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for CountingTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> apeireth_protocol::canonical::NormalizedTool {
        apeireth_protocol::canonical::NormalizedTool {
            name: self.name.into(),
            description: Some("counts invocations".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!(self.name))
    }
}

struct CountingPlugin {
    manifest: PluginManifest,
    invocations: Arc<AtomicUsize>,
}

impl CountingPlugin {
    fn new() -> Arc<Self> {
        let manifest = PluginManifest::new(
            PluginId::new("builtin.counting").unwrap(),
            "1.0.0",
            "Counting tool for the hot-revoke regression",
        )
        .declare_capability(
            CapabilityId::new(TOOL_CAPABILITY).unwrap(),
            CapabilityKind::Tool,
            "Approval B",
        )
        .unwrap();
        Arc::new(Self {
            manifest,
            invocations: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn invocations(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Plugin for CountingPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::new(CountingTool {
            id: CapabilityId::new(TOOL_CAPABILITY).unwrap(),
            name: TOOL_NAME,
            invocations: Arc::clone(&self.invocations),
        })]
    }
}

// ---------------------------------------------------------------------------
// 夹具: 脚本化 provider
// ---------------------------------------------------------------------------

enum ProviderStep {
    ToolCalls(Vec<ToolCall>),
    Say(&'static str),
}

struct FakeProvider {
    id: CapabilityId,
    steps: Vec<ProviderStep>,
    calls: AtomicUsize,
}

impl FakeProvider {
    fn new(steps: Vec<ProviderStep>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            steps,
            calls: AtomicUsize::new(0),
        })
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
        let step = self.steps.get(index).expect("provider script exhausted");
        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        match step {
            ProviderStep::ToolCalls(calls) => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: calls.clone(),
                ..base
            }),
            ProviderStep::Say(text) => Ok(NormalizedResponse {
                content: (*text).to_string(),
                ..base
            }),
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
                "Scripted provider for the hot-revoke regression",
            )
            .declare_capability(
                CapabilityId::new("provider.fake").unwrap(),
                CapabilityKind::Provider,
                "Fake provider",
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

// ---------------------------------------------------------------------------
// 装配
// ---------------------------------------------------------------------------

fn fixed_clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn tool_call() -> ToolCall {
    ToolCall {
        id: "call_b".into(),
        name: TOOL_NAME.into(),
        arguments: serde_json::json!({ "command": "echo approved" }),
    }
}

/// 装配一个 runtime: 治理链 = **共享 live 策略** (授权面 `ExecuteTool` +
/// 「每次都要人批」标记), 策略句柄外借 ⇒ 测试可随时热撤销。
async fn build_runtime() -> (Runtime, Arc<CountingPlugin>, Arc<Mutex<PermissionPolicy>>) {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool(TOOL_CAPABILITY.to_string()));
    policy.require_approval_for(TOOL_CAPABILITY);
    let policy = Arc::new(Mutex::new(policy));

    let counting = CountingPlugin::new();
    let provider = FakeProvider::new(vec![
        ProviderStep::ToolCalls(vec![tool_call()]),
        ProviderStep::Say("all done"),
    ]);

    let runtime = Runtime::builder()
        .with_clock(fixed_clock())
        .with_session_store(store)
        .with_governance(Arc::new(GovernancePipeline::new().with(Arc::new(
            PermissionGovernanceHook::new_shared(Arc::clone(&policy)),
        ))))
        .with_plugin(counting.clone())
        .with_plugin(ProviderPlugin::new(provider))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap();
    (runtime, counting, policy)
}

fn pending_view(outcome: &TurnOutcome) -> apeireth_runtime::canonical::PendingApprovalView {
    match outcome {
        TurnOutcome::PendingApproval(view) => view.clone(),
        TurnOutcome::Completed(_) => panic!("expected PendingApproval"),
    }
}

// ---------------------------------------------------------------------------
// 用例
// ---------------------------------------------------------------------------

/// ① 热撤销后, **已批准**的调用不再放行 (P3-6 的正面口径)。
///
/// 路径: 首次派发 ⇒ 治理要人批 ⇒ 人批准 ⇒ 恢复派发 (preapproved 分支)。
/// 撤销动作发生在「人已批准、尚未恢复派发」之间 ⇒ 恢复派发必须被治理拒绝,
/// 工具**零执行**, 且会话里留治理拒绝行。
#[tokio::test]
async fn hot_revoked_grant_stops_the_already_approved_call() {
    let (runtime, counting, policy) = build_runtime().await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run the approval tool"))
        .await
        .unwrap();
    let view = pending_view(&outcome);
    assert_eq!(counting.invocations(), 0, "冻结待批期间不得执行工具");

    // 热撤销: 共享 live 策略上撤掉这个能力的授权 (面板 `grants/revoke` 同款)。
    {
        let mut live = policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(
            live.revoke(&Permission::ExecuteTool(TOOL_CAPABILITY.to_string())),
            "热撤销必须真的撤掉那份授权"
        );
    }

    // 人批准 ⇒ 恢复派发走 preapproved 分支 ⇒ 治理拒绝 ⇒ 拒绝即帧。
    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(
        matches!(
            resolution,
            ApprovalResolution::Resumed(TurnOutcome::Completed(_))
        ),
        "撤销后不得再次挂起索要审批 (拒绝即帧, 回合照常收束): {resolution:?}"
    );

    assert_eq!(
        counting.invocations(),
        0,
        "热撤销后, 已批准的调用**不得**再放行执行"
    );

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    let denial = stored
        .events
        .iter()
        .find(|event| matches!(&event.event, SessionEventKind::GovernanceDenied { .. }))
        .expect("热撤销后的派发必须留治理拒绝行");
    match &denial.event {
        SessionEventKind::GovernanceDenied { hook, reason, .. } => {
            assert_eq!(hook, "permission_governance");
            assert!(
                reason.contains("not permitted"),
                "拒绝理由必须如实说明授权已被撤: {reason}"
            );
        }
        other => panic!("expected governance denial, got {other:?}"),
    }
}

/// ② 未被撤销的已批准调用**照旧执行** (修复不得变成功能回归)。
///
/// 「需要人工审批」这一档在 preapproved 分支上被**降级为「已批准」** ——
/// 人已经拍板过这一次, 不得二次索要审批, 也不得把调用卡住。
#[tokio::test]
async fn approved_call_without_revocation_still_runs_without_a_second_prompt() {
    let (runtime, counting, _policy) = build_runtime().await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run the approval tool"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(
        matches!(
            resolution,
            ApprovalResolution::Resumed(TurnOutcome::Completed(_))
        ),
        "已批准的调用必须恢复执行, 不得二次索要审批: {resolution:?}"
    );
    assert_eq!(counting.invocations(), 1, "已批准的调用必须真执行一次");

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert!(
        !stored
            .events
            .iter()
            .any(|event| matches!(&event.event, SessionEventKind::GovernanceDenied { .. })),
        "未撤销时不得出现治理拒绝行"
    );
}
