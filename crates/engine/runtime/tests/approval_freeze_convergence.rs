//! Pending-approval freeze convergence: the approval gate collapses the turn's
//! round loop instead of spinning it.
//!
//! Deterministic end-to-end proof through the canonical runtime, governance
//! pipeline, plugin registry and an in-process scripted provider (no network).
//! The behaviours pinned here:
//!
//! 1. a tool proposal frozen for human approval collapses the round loop at
//!    once — the pending state carries the generated text and the real rounds
//!    consumed, and no further round budget is burned;
//! 2. approving resumes the frozen call and continues the remaining flow;
//! 3. rejecting closes per the existing rejection semantics (the model may
//!    recover);
//! 4. a repeated proposal of the same pending-approval operation in the same
//!    turn never re-enters the freeze loop: it is the same pending item and the
//!    turn collapses directly (one approval, one action);
//! 5. the budget knobs parse, clamp, and fall back to their defaults;
//! 6. a round-limit failure reports the real budget numbers;
//! 7. a pending close outranks a round-limit close in the same round;
//! 8. the default budgets (8 rounds / 16 tool calls) are unchanged.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId, Timestamp, VirtualClock};
use apeireth_governance::{Permission, PermissionGovernanceHook, PermissionPolicy};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::execute::MAX_TOOL_CALLS_PER_ROUND;
use apeireth_runtime::canonical::{
    parse_tool_call_limit, parse_turn_round_limit, ApprovalDecision, ApprovalResolution,
    ApprovalStatus, Runtime, RuntimeError, SessionEventKind, TurnOutcome, TurnRequest,
    DEFAULT_MAX_ROUNDS, MAX_TOOL_CALL_LIMIT, MAX_TURN_ROUNDS, MIN_TOOL_CALL_LIMIT, MIN_TURN_ROUNDS,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

/// One scripted provider response.
enum Step {
    /// A response with text and no tool calls.
    Say(&'static str),
    /// A response with text plus tool calls.
    ToolCalls {
        /// The model's visible text for the round.
        content: &'static str,
        /// The proposed tool calls.
        calls: Vec<ToolCall>,
    },
}

struct ScriptedProvider {
    id: CapabilityId,
    steps: Vec<Step>,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    fn new(steps: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            steps,
            calls: AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
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
            .steps
            .get(index)
            .unwrap_or_else(|| panic!("provider script exhausted at call {index}"));
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        Ok(match step {
            Step::Say(text) => NormalizedResponse {
                content: (*text).to_string(),
                ..base
            },
            Step::ToolCalls { content, calls } => NormalizedResponse {
                content: (*content).to_string(),
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                tool_calls: calls.clone(),
                ..base
            },
        })
    }
}

/// A counting demo tool whose every call needs approval under the test policy.
struct DemoTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for DemoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "demo".into(),
            description: Some("counts invocations".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({ "ran": true }))
    }
}

/// A second tool that runs without an approval gate, used to drive a genuinely
/// diverging turn into the round limit.
struct NoteTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for NoteTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "note".into(),
            description: Some("counts invocations".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({ "noted": true }))
    }
}

struct TestPlugin {
    manifest: PluginManifest,
    provider: Option<Arc<ScriptedProvider>>,
    tool_invocations: Option<Arc<AtomicUsize>>,
}

impl TestPlugin {
    fn provider(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.scripted_provider").unwrap(),
                "1.0.0",
                "scripted provider for approval-freeze convergence tests",
            )
            .declare_capability(
                CapabilityId::new("provider.fake").unwrap(),
                CapabilityKind::Provider,
                "scripted provider",
            )
            .unwrap(),
            provider: Some(provider),
            tool_invocations: None,
        })
    }

    fn tools(invocations: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.demo_tool").unwrap(),
                "1.0.0",
                "approval-gated demo tool plus an ungated note tool",
            )
            .declare_capability(
                CapabilityId::new("tool.demo").unwrap(),
                CapabilityKind::Tool,
                "counts invocations",
            )
            .unwrap()
            .declare_capability(
                CapabilityId::new("tool.note").unwrap(),
                CapabilityKind::Tool,
                "counts invocations",
            )
            .unwrap(),
            provider: None,
            tool_invocations: Some(invocations),
        })
    }
}

#[async_trait]
impl Plugin for TestPlugin {
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
        self.tool_invocations
            .as_ref()
            .map(|invocations| {
                vec![
                    Arc::new(DemoTool {
                        id: CapabilityId::new("tool.demo").unwrap(),
                        invocations: Arc::clone(invocations),
                    }) as Arc<dyn ToolCapability>,
                    Arc::new(NoteTool {
                        id: CapabilityId::new("tool.note").unwrap(),
                        invocations: Arc::clone(invocations),
                    }) as Arc<dyn ToolCapability>,
                ]
            })
            .unwrap_or_default()
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.provider
            .as_ref()
            .map(|provider| vec![Arc::clone(provider) as Arc<dyn ProviderCapability>])
            .unwrap_or_default()
    }
}

fn frozen_clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn call(id: &str, args: serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "demo".into(),
        arguments: args,
    }
}

fn echo_hi(id: &str) -> ToolCall {
    call(id, serde_json::json!({ "command": "echo hi" }))
}

fn note(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "note".into(),
        arguments: serde_json::json!({ "text": "spin" }),
    }
}

/// Build the runtime under test: demo tool gated behind human approval, the
/// scripted provider, and `max_rounds` (0 = the default budget).
async fn build_runtime(
    provider: Arc<ScriptedProvider>,
    invocations: Arc<AtomicUsize>,
    max_rounds: u32,
) -> Runtime {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.demo".into()));
    policy.grant(Permission::ExecuteTool("tool.note".into()));
    policy.require_approval_for("tool.demo");
    let mut builder = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations))
        .with_plugin(TestPlugin::provider(provider))
        .with_default_model(MODEL);
    if max_rounds > 0 {
        builder = builder.with_max_rounds(max_rounds);
    }
    builder.build().await.unwrap()
}

/// ① 提议工具 → 冻结 → 回合立即收束为待批准: 轮耗 = 1, 非 422, 文本保留。
#[tokio::test]
async fn a_frozen_proposal_collapses_the_round_loop_at_once() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls {
        content: "即将执行命令",
        calls: vec![echo_hi("call-1")],
    }]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 0).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run it"))
        .await
        .expect("a pending close is not an error");

    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("expected PendingApproval");
    };
    // 轮耗 = 1: 提议它的那一轮, 一轮都不多烧。
    assert_eq!(view.rounds_used, 1, "轮耗必须是提议工具的那一轮");
    // 保留已生成文本: 待批准态把模型已说的话原样带出来。
    assert!(
        view.generated_text.contains("即将执行命令"),
        "generated text must be preserved: {:?}",
        view.generated_text
    );
    assert_eq!(view.tool_name, "demo");
    assert_eq!(view.tool_call.id, "call-1");
    // 回合循环立即收束: provider 不再被叫。
    assert_eq!(
        provider.call_count(),
        1,
        "the round loop must collapse at once"
    );
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "nothing executes pre-approval"
    );

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(stored.active_approval_id, Some(view.approval_id));
    assert_eq!(stored.approvals.len(), 1);
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Pending
    );
}

/// ② 批准 → 恢复执行冻结调用并续跑剩余流程。
#[tokio::test]
async fn approving_resumes_the_frozen_call_and_finishes_the_turn() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            calls: vec![echo_hi("call-1")],
        },
        Step::Say("done"),
    ]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 0).await;
    let session = SessionId::new();

    let TurnOutcome::PendingApproval(view) = runtime
        .execute_outcome(TurnRequest::new(session, "run it"))
        .await
        .unwrap()
    else {
        panic!("expected PendingApproval");
    };
    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("expected Resumed(Completed)");
    };
    assert_eq!(response.text, "done");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "one approval, one action"
    );
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Consumed
    );
    assert_eq!(stored.active_approval_id, None);
}

/// ③ 拒绝 → 既有拒绝语义收束 (合成拒绝结果交还模型, 模型可恢复)。
#[tokio::test]
async fn rejecting_closes_per_the_existing_rejection_semantics() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            calls: vec![echo_hi("call-1")],
        },
        Step::Say("understood, skipping the command"),
    ]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 0).await;
    let session = SessionId::new();

    let TurnOutcome::PendingApproval(view) = runtime
        .execute_outcome(TurnRequest::new(session, "run it"))
        .await
        .unwrap()
    else {
        panic!("expected PendingApproval");
    };
    let resolution = runtime
        .resolve_approval(
            session,
            view.approval_id,
            ApprovalDecision::Reject {
                reason: Some("not now".into()),
            },
        )
        .await
        .unwrap();
    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("expected Resumed(Completed)");
    };
    assert_eq!(response.text, "understood, skipping the command");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "a rejected call never runs"
    );
}

/// ④ 同回合重复提议同一待批工具不循环 (批准后重提): 不再冻结、不再批、不再烧轮。
#[tokio::test]
async fn a_same_turn_repeat_after_approve_collapses_without_looping() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "第一次提议",
            calls: vec![echo_hi("call-1")],
        },
        Step::ToolCalls {
            content: "再跑一次",
            calls: vec![echo_hi("call-2")],
        },
    ]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 0).await;
    let session = SessionId::new();

    let TurnOutcome::PendingApproval(view) = runtime
        .execute_outcome(TurnRequest::new(session, "run it"))
        .await
        .unwrap()
    else {
        panic!("expected PendingApproval");
    };
    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();

    // 视为同一待批准项, 直接收束: 回合到此为止, 不再有第二张审批卡。
    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("the repeat must collapse the turn directly, not re-freeze");
    };
    assert!(
        response.text.contains("第一次提议") && response.text.contains("再跑一次"),
        "generated text must be preserved across the collapse: {:?}",
        response.text
    );
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "one approval authorizes one action; the repeat never executes"
    );
    assert_eq!(
        provider.call_count(),
        2,
        "no spin: two model rounds, then close"
    );
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.approvals.len(),
        1,
        "the repeat is the same pending item, not a second one"
    );
    assert!(stored.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::ApprovalReentryCollapsed { approval_id, .. } if *approval_id == view.approval_id
    )));
}

/// ④ 同回合重复提议同一待批工具不循环 (拒绝后重提): 决议维持效力, 回合直接收束。
#[tokio::test]
async fn a_same_turn_repeat_after_reject_collapses_without_looping() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            calls: vec![echo_hi("call-1")],
        },
        Step::ToolCalls {
            content: "让我再试一次",
            calls: vec![echo_hi("call-2")],
        },
    ]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 0).await;
    let session = SessionId::new();

    let TurnOutcome::PendingApproval(view) = runtime
        .execute_outcome(TurnRequest::new(session, "run it"))
        .await
        .unwrap()
    else {
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
        panic!("the repeat must collapse the turn directly, not re-freeze");
    };
    assert!(
        response.text.contains("让我再试一次"),
        "generated text must be preserved: {:?}",
        response.text
    );
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "a rejected operation never runs"
    );
    assert_eq!(provider.call_count(), 2, "no spin");
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.approvals.len(),
        1,
        "no second approval item for the same operation in one turn"
    );
}

/// ⑦ 待批准与轮上限同轮时上报待批准 (错误帧不再是 turn_not_converged)。
#[tokio::test]
async fn a_pending_close_outranks_the_round_limit_close() {
    let invocations = Arc::new(AtomicUsize::new(0));
    // 一轮里两个都需审批的调用: 第二个的冻结落在轮预算已耗尽的那一轮。
    let provider = ScriptedProvider::new(vec![Step::ToolCalls {
        content: "",
        calls: vec![
            call("call-a", serde_json::json!({ "command": "echo a" })),
            call("call-b", serde_json::json!({ "command": "echo b" })),
        ],
    }]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 1).await;
    let session = SessionId::new();

    // 第一次冻结: 轮预算已耗尽 (轮耗 1 = 上限 1), 上报待批准而不是轮上限失败。
    let TurnOutcome::PendingApproval(first) = runtime
        .execute_outcome(TurnRequest::new(session, "run both"))
        .await
        .expect("the pending close must win over the exhausted round budget")
    else {
        panic!("expected PendingApproval");
    };
    assert_eq!(first.rounds_used, 1);

    // 同轮的第二个冻结: 同样上报待批准 (202 语义), 不是 turn_not_converged。
    let resolution = runtime
        .resolve_approval(session, first.approval_id, ApprovalDecision::Approve)
        .await
        .expect("the pending close must win over the round-limit close");
    let ApprovalResolution::Resumed(second) = resolution else {
        panic!("expected Resumed(PendingApproval)");
    };
    let TurnOutcome::PendingApproval(second) = second else {
        panic!("expected the second freeze to be reported as pending");
    };
    assert_eq!(second.tool_call.id, "call-b");
    assert_eq!(invocations.load(Ordering::SeqCst), 1);

    // 续跑剩余流程走到真实轮上限时, 错误帧带真实数字 (⑥)。
    let error = runtime
        .resolve_approval(session, second.approval_id, ApprovalDecision::Approve)
        .await
        .expect_err("the budget is genuinely exhausted after both approvals");
    let RuntimeError::RoundLimitExceeded {
        limit,
        rounds,
        pending_tool,
    } = error
    else {
        panic!("expected RoundLimitExceeded, got {error}");
    };
    assert_eq!(limit, 1);
    assert_eq!(rounds, 1, "已耗轮数必须是真实数字");
    assert_eq!(pending_tool.as_deref(), Some("demo"), "当时待批准工具名");
    assert_eq!(invocations.load(Ordering::SeqCst), 2);
}

/// ⑥ 轮上限错误报真实数字: limit / 已耗轮数 / 当时待批准工具名 (若有)。
#[tokio::test]
async fn the_round_limit_error_reports_real_budget_numbers() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            calls: vec![echo_hi("call-1")],
        },
        Step::ToolCalls {
            content: "",
            calls: vec![note("call-2")],
        },
        Step::ToolCalls {
            content: "",
            calls: vec![note("call-3")],
        },
    ]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 2).await;
    let session = SessionId::new();

    let TurnOutcome::PendingApproval(view) = runtime
        .execute_outcome(TurnRequest::new(session, "run it"))
        .await
        .unwrap()
    else {
        panic!("expected PendingApproval");
    };
    // 拒绝后续跑: 模型换了个工具原地打转, 真实轮上限在下一轮触发。
    let error = runtime
        .resolve_approval(
            session,
            view.approval_id,
            ApprovalDecision::Reject { reason: None },
        )
        .await
        .expect_err("a genuinely diverging turn hits the round limit");
    let RuntimeError::RoundLimitExceeded {
        limit,
        rounds,
        pending_tool,
    } = error
    else {
        panic!("expected RoundLimitExceeded, got {error}");
    };
    assert_eq!(limit, 2);
    assert_eq!(rounds, 2, "已耗轮数必须是真实数字");
    assert_eq!(
        pending_tool.as_deref(),
        Some("demo"),
        "当时待批准工具名 (若有)"
    );
    let message = RuntimeError::RoundLimitExceeded {
        limit,
        rounds,
        pending_tool,
    }
    .to_string();
    assert!(
        message.contains("turn did not converge within 2 rounds"),
        "{message}"
    );
}

/// ⑤ 预算取值: 越界钳制、非法值回默认。
#[test]
fn budget_values_clamp_and_fall_back_to_defaults() {
    assert_eq!(parse_turn_round_limit(None), DEFAULT_MAX_ROUNDS);
    assert_eq!(parse_turn_round_limit(Some("")), DEFAULT_MAX_ROUNDS);
    assert_eq!(parse_turn_round_limit(Some("  ")), DEFAULT_MAX_ROUNDS);
    assert_eq!(parse_turn_round_limit(Some("nope")), DEFAULT_MAX_ROUNDS);
    assert_eq!(parse_turn_round_limit(Some("2")), 2);
    assert_eq!(
        parse_turn_round_limit(Some("0")),
        MIN_TURN_ROUNDS,
        "越界钳制"
    );
    assert_eq!(
        parse_turn_round_limit(Some("999")),
        MAX_TURN_ROUNDS,
        "越界钳制"
    );
    assert_eq!(
        parse_turn_round_limit(Some("-7")),
        MIN_TURN_ROUNDS,
        "越界钳制"
    );

    assert_eq!(parse_tool_call_limit(None), MAX_TOOL_CALLS_PER_ROUND);
    assert_eq!(parse_tool_call_limit(Some("x")), MAX_TOOL_CALLS_PER_ROUND);
    assert_eq!(parse_tool_call_limit(Some("4")), 4);
    assert_eq!(parse_tool_call_limit(Some("0")), MIN_TOOL_CALL_LIMIT);
    assert_eq!(parse_tool_call_limit(Some("9999")), MAX_TOOL_CALL_LIMIT);
}

/// ⑤/④ 配置字段也显式: 调小单轮工具调用上限会截断批次 (预算生效)。
#[tokio::test]
async fn a_configured_tool_call_limit_truncates_the_batch() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let calls: Vec<ToolCall> = (0..4)
        .map(|index| {
            call(
                &format!("call-{index}"),
                serde_json::json!({ "command": "echo x" }),
            )
        })
        .collect();
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls { content: "", calls },
        Step::Say("done"),
    ]);
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.demo".into()));
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations.clone()))
        .with_plugin(TestPlugin::provider(provider.clone()))
        .with_default_model(MODEL)
        .with_max_tool_calls_per_round(2)
        .build()
        .await
        .unwrap();

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "run four"))
        .await
        .unwrap();
    assert_eq!(response.text, "done");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        2,
        "the configured per-round tool-call limit truncates the batch"
    );
}

/// ⑧ 默认预算不变 (8 / 16 基线零回归)。
#[tokio::test]
async fn default_budgets_are_unchanged() {
    assert_eq!(DEFAULT_MAX_ROUNDS, 8);
    assert_eq!(MAX_TOOL_CALLS_PER_ROUND, 16);

    let invocations = Arc::new(AtomicUsize::new(0));
    // 模型每轮都提议一个普通调用并原地打转: 默认预算下第 9 轮起点触发上限。
    let steps: Vec<Step> = (0..9)
        .map(|index| Step::ToolCalls {
            content: "",
            calls: vec![call(
                &format!("call-{index}"),
                serde_json::json!({ "command": "echo spin" }),
            )],
        })
        .collect();
    let provider = ScriptedProvider::new(steps);
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.demo".into()));
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations.clone()))
        .with_plugin(TestPlugin::provider(provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();
    assert_eq!(runtime.config().max_rounds, DEFAULT_MAX_ROUNDS);
    assert_eq!(
        runtime.config().max_tool_calls_per_round,
        MAX_TOOL_CALLS_PER_ROUND
    );

    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "spin"))
        .await
        .expect_err("a genuinely diverging turn hits the default round limit");
    let RuntimeError::RoundLimitExceeded { limit, rounds, .. } = error else {
        panic!("expected RoundLimitExceeded, got {error}");
    };
    assert_eq!(limit, 8);
    assert_eq!(rounds, 8);
    assert_eq!(
        provider.call_count(),
        8,
        "the limit is enforced, not exceeded"
    );
}
