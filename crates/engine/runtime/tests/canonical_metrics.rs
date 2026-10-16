//! G-P1 埋点采集的端到端证据: 真回合 → 收束埋点 → 指标注册表读数。
//!
//! 这里不 mock 被测的缝: 运行时、插件管理、能力注册、治理、provider 路由、
//! 会话存储都是真实现, 只在两个边界做替身 (脚本化 provider 与一个最小工具),
//! 保证确定性 (无 sleep、无网络)。钉住四件事:
//!
//! 1. 回合收束处记一拍, 成功/失败/待批分别计数;
//! 2. 成功拍的 usage 计量与回合收束计量同源 (聊天头遥测同一数据源);
//! 3. 待批拍与审批续跑再收束的拍各记各的 (与收束事件同口径);
//! 4. 埋点零侵入: execute.rs 的指标埋点保持最小面 (两个收束点)。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{
    AllowAll, Decision, GovernanceHook, GovernancePipeline, GovernanceRequest, Permission,
    PermissionGovernanceHook, PermissionPolicy,
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
    ApprovalDecision, ApprovalResolution, InMemorySessionStore, Runtime, TurnOutcome, TurnRequest,
};

const MODEL: &str = "fake-model-1";
const TOOL: &str = "gated";

// ---------------------------------------------------------------------------
// Fixtures (边界替身)
// ---------------------------------------------------------------------------

/// 脚本化 provider: 按剧本回合同的响应。
enum Scripted {
    Say(&'static str),
    CallTool {
        call_id: &'static str,
        tool: &'static str,
    },
}

struct FakeProvider {
    id: CapabilityId,
    script: Vec<Scripted>,
    calls: AtomicUsize,
}

impl FakeProvider {
    fn new(id: &str, script: Vec<Scripted>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait::async_trait]
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
        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("provider script exhausted at call {}", index + 1));
        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            // turn-telemetry 同源计量: 固定 usage, 断言可精确钉住。
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        Ok(match step {
            Scripted::Say(text) => NormalizedResponse {
                content: (*text).into(),
                ..base
            },
            Scripted::CallTool { call_id, tool } => NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: (*call_id).into(),
                    name: (*tool).into(),
                    arguments: serde_json::json!({}),
                }],
                ..base
            },
        })
    }
}

/// 需要人工审批才能派发的最小工具。
struct GatedTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl ToolCapability for GatedTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(TOOL)
            .with_description("gated tool")
            .with_parameters(ToolParameters::new())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!("ok"))
    }
}

/// 一个插件同时供 provider 与工具 (装配面最小化)。
struct FakePlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
    invocations: Arc<AtomicUsize>,
}

impl FakePlugin {
    fn new(provider: Arc<FakeProvider>) -> Arc<Self> {
        let manifest = PluginManifest::new(
            PluginId::new("vendor.fake").unwrap(),
            "1.0.0",
            "metrics fixture",
        )
        .declare_capability(provider.id().clone(), CapabilityKind::Provider, "scripted")
        .unwrap()
        .declare_capability(
            CapabilityId::new("tool.gated").unwrap(),
            CapabilityKind::Tool,
            "gated tool",
        )
        .unwrap();
        Arc::new(Self {
            manifest,
            provider,
            invocations: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn tool_invocations(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl Plugin for FakePlugin {
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

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::new(GatedTool {
            id: CapabilityId::new("tool.gated").unwrap(),
            invocations: Arc::clone(&self.invocations),
        })]
    }
}

/// 完成治理一律否决 (失败收束的确定性来源)。
struct DenyEveryCompletion;

#[async_trait::async_trait]
impl GovernanceHook for DenyEveryCompletion {
    fn name(&self) -> &str {
        "governance.metrics.fixture"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match &request.action {
            apeireth_governance::Action::Completion { .. } => Decision::deny("blocked for metrics"),
            _ => Decision::Allow,
        }
    }
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

/// 治理策略: 工具已授权, 但派发前必须人工审批 (待批收束的确定性来源)。
fn approval_policy() -> PermissionPolicy {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.gated".into()));
    policy.require_approval_for("tool.gated");
    policy
}

async fn runtime_with(
    provider: Arc<FakeProvider>,
    governance: Arc<dyn GovernanceHook>,
) -> (Runtime, Arc<FakePlugin>) {
    let plugin = FakePlugin::new(provider);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(governance)
        .with_plugin(plugin.clone())
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .expect("runtime");
    (runtime, plugin)
}

// ---------------------------------------------------------------------------
// 1. 成功收束: 一拍, usage 计量同源
// ---------------------------------------------------------------------------

#[tokio::test]
async fn completed_turn_records_one_success_beat_with_turn_telemetry_usage() {
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("done")]);
    let (runtime, plugin) = runtime_with(provider, Arc::new(AllowAll)).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "hello"))
        .await
        .expect("turn");
    assert!(matches!(outcome, TurnOutcome::Completed(_)));

    let aggregate = runtime
        .metrics()
        .session_aggregate(session)
        .expect("beat recorded");
    assert_eq!(aggregate.turns, 1);
    assert_eq!(aggregate.succeeded, 1);
    assert_eq!(aggregate.failed, 0);
    assert_eq!(aggregate.pending_approval, 0);
    assert_eq!(aggregate.latency_ms.count, 1, "收束拍必有延迟值");
    assert_eq!(aggregate.input_tokens.sum, 10, "usage 计量同源: prompt 列");
    assert_eq!(
        aggregate.output_tokens.sum, 5,
        "usage 计量同源: completion 列"
    );
    assert_eq!(aggregate.rounds.sum, 1);
    assert_eq!(aggregate.tool_calls.sum, 0);
    assert_eq!(
        aggregate.cache_hit_tokens, None,
        "计量源无缓存列: 未知不冒充 0"
    );
    assert_eq!(aggregate.cost_micros, None, "未计价: 可选列如实空");
    assert_eq!(plugin.tool_invocations(), 0);

    let global = runtime.metrics().global_aggregate();
    assert_eq!(global.turns, 1);
    assert_eq!(global.succeeded, 1);
}

// ---------------------------------------------------------------------------
// 2. 失败收束: 一拍, 无计量不冒充 0
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failed_turn_records_one_failure_beat_without_fabricated_usage() {
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("must not run")]);
    let (runtime, _plugin) = runtime_with(provider, Arc::new(DenyEveryCompletion)).await;
    let session = SessionId::new();

    let error = runtime
        .execute_outcome(TurnRequest::new(session, "blocked"))
        .await
        .expect_err("completion denied");
    assert!(!error.to_string().is_empty(), "失败帧必须带真因");

    let aggregate = runtime
        .metrics()
        .session_aggregate(session)
        .expect("beat recorded");
    assert_eq!(aggregate.turns, 1);
    assert_eq!(aggregate.failed, 1);
    assert_eq!(aggregate.succeeded, 0);
    assert_eq!(aggregate.input_tokens.count, 0, "失败拍无 usage 计量");
    assert_eq!(aggregate.latency_ms.count, 1);
    assert_eq!(runtime.metrics().global_aggregate().failed, 1);
}

// ---------------------------------------------------------------------------
// 3. 待批收束 + 审批续跑再收束: 两拍各记各的 (与收束事件同口径)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pending_beat_and_resumed_completion_beat_count_separately() {
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            Scripted::CallTool {
                call_id: "call_1",
                tool: TOOL,
            },
            Scripted::Say("done"),
        ],
    );
    let (runtime, plugin) = runtime_with(
        provider,
        Arc::new(
            GovernancePipeline::new()
                .with(Arc::new(PermissionGovernanceHook::new(approval_policy()))),
        ),
    )
    .await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run the gated tool"))
        .await
        .expect("turn");
    let TurnOutcome::PendingApproval(view) = &outcome else {
        panic!("expected a pending approval, got {outcome:?}");
    };

    let after_pause = runtime
        .metrics()
        .session_aggregate(session)
        .expect("pending beat");
    assert_eq!(after_pause.turns, 1);
    assert_eq!(after_pause.pending_approval, 1);
    assert_eq!(after_pause.succeeded, 0);
    assert_eq!(
        after_pause.input_tokens.count, 0,
        "待批拍收束处无 usage 计量"
    );
    assert_eq!(
        after_pause.rounds.sum,
        u64::from(view.rounds_used),
        "待批拍的轮数列自收束视图"
    );

    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .expect("resolution");
    assert!(matches!(
        resolution,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));

    let after_resume = runtime
        .metrics()
        .session_aggregate(session)
        .expect("resumed beat");
    assert_eq!(after_resume.turns, 2, "续跑再收束补一拍, 不吞不重");
    assert_eq!(after_resume.pending_approval, 1);
    assert_eq!(after_resume.succeeded, 1);
    assert_eq!(
        after_resume.input_tokens.count, 1,
        "续跑成功拍带 usage 计量"
    );
    assert_eq!(after_resume.input_tokens.sum, 10);
    assert_eq!(after_resume.tool_calls.sum, 1, "一次工具派发");
    assert_eq!(plugin.tool_invocations(), 1, "冻结操作真实执行一次");

    let global = runtime.metrics().global_aggregate();
    assert_eq!(global.turns, 2);
    assert_eq!(
        global.succeeded + global.pending_approval + global.failed,
        2
    );
}

// ---------------------------------------------------------------------------
// 4. 埋点零侵入: execute.rs 的指标埋点保持最小面
// ---------------------------------------------------------------------------

/// 观测线在多线共享面 (execute.rs) 上只允许两个收束点各一行调用;埋点
/// 越铺越大必须先过批, 不许顺手加。
#[test]
fn execute_metrics_instrumentation_stays_two_minimal_sites() {
    let source = include_str!("../src/canonical/execute.rs");
    let calls = source.matches("self.metrics").count();
    assert_eq!(
        calls, 2,
        "指标埋点只允许两个收束点 (回合收束 / 审批续跑收束) 各一行调用"
    );
    assert!(
        source.contains(".record_outcome("),
        "回合收束点必须走 record_outcome"
    );
    assert!(
        source.contains(".record_resolution("),
        "审批续跑收束点必须走 record_resolution"
    );
}
