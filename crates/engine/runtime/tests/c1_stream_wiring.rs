//! C2-3 端到端证据: 生产回合把结果接到 C1 权威流。
//!
//! 这里不 mock 被测的缝: 运行时、插件管理、能力注册、治理、provider 路由、
//! 会话存储与 C1 注册表都是真实现。只替换两个边界: 脚本化 provider 与一个
//! 需要审批的最小工具。四个事实各用一条流钉住:
//!
//! 1. 流式回合的每个 delta 实时入流, 终帧恰好一次且为 completed;
//! 2. 停在审批时终帧为 approval, 审批 id/摘要/过期时刻同源;
//! 3. 回合失败时终帧为 failed 且带真因;
//! 4. 已停止的回合即使最终完成, 终帧仍为 stopped (停止优先);
//! 5. 非流式回合即使显式带 request id 也不注册 C1 流。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, RequestId, SessionId, Timestamp, VirtualClock,
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
use apeireth_protocol::canonical_stream::{C1FinalFrame, C1Frame, C1StreamState};
use apeireth_runtime::canonical::canonical_stream::{global_registry, ReplayOutcome, StreamHandle};
use apeireth_runtime::canonical::turn_cancellation::{self, StopOutcome};
use apeireth_runtime::canonical::{Runtime, TurnOutcome, TurnRequest};
use async_trait::async_trait;
use tokio::sync::Notify;
use tokio::time::timeout;

const MODEL: &str = "fake-model-1";
const TOOL: &str = "gated";

// ---------------------------------------------------------------------------
// Fixtures (边界替身)
// ---------------------------------------------------------------------------

enum Scripted {
    Say(&'static str),
    CallTool(&'static str),
}

/// 脚本化 provider: 可发多段 delta, 也可在发完后阻塞等放行。
struct StreamProvider {
    id: CapabilityId,
    script: Vec<Scripted>,
    deltas: Vec<&'static str>,
    calls: AtomicUsize,
    started: Option<Arc<Notify>>,
    release: Option<Arc<Notify>>,
}

impl StreamProvider {
    fn new(script: Vec<Scripted>, deltas: Vec<&'static str>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            script,
            deltas,
            calls: AtomicUsize::new(0),
            started: None,
            release: None,
        })
    }

    fn gated(
        script: Vec<Scripted>,
        deltas: Vec<&'static str>,
        started: Arc<Notify>,
        release: Arc<Notify>,
    ) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            script,
            deltas,
            calls: AtomicUsize::new(0),
            started: Some(started),
            release: Some(release),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProviderCapability for StreamProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls)
                .with_feature(ModelFeature::Streaming),
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
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        Ok(match step {
            Scripted::Say(text) => NormalizedResponse {
                content: (*text).into(),
                ..base
            },
            Scripted::CallTool(tool) => NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: "call_1".into(),
                    name: (*tool).into(),
                    arguments: serde_json::json!({}),
                }],
                ..base
            },
        })
    }

    async fn complete_streaming(
        &self,
        request: &NormalizedRequest,
        on_delta: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<NormalizedResponse, ProviderError> {
        for delta in &self.deltas {
            on_delta((*delta).to_string());
        }
        if let Some(started) = &self.started {
            started.notify_one();
        }
        if let Some(release) = &self.release {
            release.notified().await;
        }
        self.complete(request).await
    }
}

struct GatedTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
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

struct FixturePlugin {
    manifest: PluginManifest,
    provider: Arc<StreamProvider>,
    invocations: Arc<AtomicUsize>,
}

impl FixturePlugin {
    fn new(provider: Arc<StreamProvider>) -> Arc<Self> {
        let manifest = PluginManifest::new(
            PluginId::new("vendor.fake").unwrap(),
            "1.0.0",
            "C2-3 stream wiring fixture",
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
}

#[async_trait]
impl Plugin for FixturePlugin {
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

struct DenyEveryCompletion;

#[async_trait]
impl GovernanceHook for DenyEveryCompletion {
    fn name(&self) -> &str {
        "governance.c2_3.fixture"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match &request.action {
            apeireth_governance::Action::Completion { .. } => Decision::deny("blocked for C2-3"),
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

/// 工具已授权, 但派发前必须人工审批。
fn approval_policy() -> PermissionPolicy {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.gated".into()));
    policy.require_approval_for("tool.gated");
    policy
}

async fn runtime_with(
    provider: Arc<StreamProvider>,
    governance: Arc<dyn GovernanceHook>,
) -> Runtime {
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(governance)
        .with_plugin(FixturePlugin::new(provider))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .expect("runtime")
}

async fn await_handle(session: SessionId, request: RequestId) -> StreamHandle {
    timeout(Duration::from_secs(5), async {
        loop {
            if let Some(handle) =
                global_registry().handle(&session.to_string(), &request.to_string())
            {
                break handle;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("C1 stream was registered")
}

fn replay_window(handle: &StreamHandle) -> Vec<C1Frame> {
    match global_registry().replay(handle, 0).expect("replay") {
        ReplayOutcome::Window(frames) => frames,
        ReplayOutcome::ResyncRequired {
            oldest_available_sequence,
        } => panic!("unexpected resync at {oldest_available_sequence}"),
    }
}

fn only_final(frames: &[C1Frame]) -> &C1FinalFrame {
    assert_eq!(
        frames.iter().filter(|frame| frame.is_final()).count(),
        1,
        "exactly one final frame: {frames:?}"
    );
    match frames.last() {
        Some(C1Frame::Final(frame)) => frame,
        other => panic!("last frame must be final, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 1. 流式完成: 实时 delta + 恰好一帧 completed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn streaming_turn_publishes_deltas_and_one_completed_final() {
    let provider = StreamProvider::new(vec![Scripted::Say("Hello")], vec!["Hel", "lo"]);
    let runtime = Arc::new(runtime_with(provider, Arc::new(AllowAll)).await);
    let session = SessionId::new();
    let request = RequestId::new();

    let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let seen_by_sink = Arc::clone(&seen);
    let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |delta| {
        seen_by_sink.lock().unwrap().push(delta);
    });

    let turn = {
        let runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            runtime
                .execute_outcome_streaming(
                    TurnRequest::new(session, "say hello").with_request(request),
                    sink,
                )
                .await
        })
    };
    let handle = await_handle(session, request).await;
    let outcome = turn.await.expect("turn task").expect("turn outcome");
    assert!(matches!(outcome, TurnOutcome::Completed(_)));
    assert_eq!(*seen.lock().unwrap(), vec!["Hel", "lo"]);

    let frames = replay_window(&handle);
    assert_eq!(
        frames.iter().map(C1Frame::sequence).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    let C1Frame::Delta(first) = &frames[0] else {
        panic!("first frame must be a delta: {frames:?}");
    };
    assert_eq!(first.delta, "Hel");
    let C1Frame::Delta(second) = &frames[1] else {
        panic!("second frame must be a delta: {frames:?}");
    };
    assert_eq!(second.delta, "lo");
    let final_frame = only_final(&frames);
    assert_eq!(final_frame.state, C1StreamState::Completed);
    assert_eq!(final_frame.text.as_deref(), Some("Hello"));
    let usage = final_frame.usage.as_ref().expect("usage");
    assert_eq!((usage.prompt_tokens, usage.completion_tokens), (10, 5));
    assert_eq!(usage.total_tokens, 15);

    let snapshot = global_registry()
        .stream_snapshot(&handle)
        .expect("snapshot");
    assert!(snapshot.finalized);
    assert_eq!(snapshot.buffered_frames, 3);
    assert_eq!(snapshot.next_sequence, 4);

    let mut subscription = global_registry().subscribe(&handle).expect("subscribe");
    assert_eq!(subscription.recv().await, Some(frames[2].clone()));
    assert!(subscription.recv().await.is_none());

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session");
    assert_eq!(stored.messages.len(), 2, "the turn ran exactly once");
}

// ---------------------------------------------------------------------------
// 2. 停在审批: 终帧 approval 携带同源审批视图
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pending_approval_publishes_one_approval_final() {
    let provider = StreamProvider::new(vec![Scripted::CallTool(TOOL)], Vec::new());
    let governance = Arc::new(
        GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(approval_policy()))),
    );
    let runtime = runtime_with(provider, governance).await;
    let session = SessionId::new();
    let request = RequestId::new();

    let turn = {
        let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_delta| {});
        tokio::spawn(async move {
            runtime
                .execute_outcome_streaming(
                    TurnRequest::new(session, "run gated").with_request(request),
                    sink,
                )
                .await
        })
    };
    let handle = await_handle(session, request).await;
    let outcome = turn.await.expect("turn task").expect("turn outcome");
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("expected a pending approval");
    };

    let frames = replay_window(&handle);
    let final_frame = only_final(&frames);
    assert_eq!(final_frame.state, C1StreamState::Approval);
    assert!(final_frame.usage.is_none());
    let approvals = final_frame.approvals.as_ref().expect("approvals");
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].approval_id, view.approval_id.to_string());
    assert_eq!(
        approvals[0].expires_at_ms,
        Some(view.expires_at.epoch_millis())
    );
    let expected_digest =
        (!view.action_binding_digest.is_empty()).then(|| view.action_binding_digest.clone());
    assert_eq!(approvals[0].action_binding_digest, expected_digest);
    let expected_text = (!view.generated_text.is_empty()).then(|| view.generated_text.clone());
    assert_eq!(final_frame.text, expected_text);
}

// ---------------------------------------------------------------------------
// 3. 回合失败: 终帧 failed 带真因, provider 未被调用
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failed_turn_publishes_failed_final() {
    let provider = StreamProvider::new(vec![Scripted::Say("must not run")], Vec::new());
    let runtime = runtime_with(provider.clone(), Arc::new(DenyEveryCompletion)).await;
    let session = SessionId::new();
    let request = RequestId::new();

    let turn = {
        let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_delta| {});
        tokio::spawn(async move {
            runtime
                .execute_outcome_streaming(
                    TurnRequest::new(session, "blocked").with_request(request),
                    sink,
                )
                .await
        })
    };
    let handle = await_handle(session, request).await;
    let error = turn
        .await
        .expect("turn task")
        .expect_err("completion denied");
    assert!(error.to_string().contains("blocked for C2-3"), "{error}");
    assert_eq!(provider.call_count(), 0, "denied before the wire");

    let frames = replay_window(&handle);
    let final_frame = only_final(&frames);
    assert_eq!(final_frame.state, C1StreamState::Failed);
    let frame_error = final_frame.error.as_deref().expect("error");
    assert!(frame_error.contains("blocked for C2-3"), "{frame_error}");
    assert!(final_frame.text.is_none());
    assert!(final_frame.usage.is_none());
}

// ---------------------------------------------------------------------------
// 4. 已停止: 回合最终完成, 终帧仍为 stopped (停止优先)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stopped_turn_publishes_stopped_final() {
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let provider = StreamProvider::gated(
        vec![Scripted::Say("late")],
        vec!["par"],
        Arc::clone(&started),
        Arc::clone(&release),
    );
    let runtime = Arc::new(runtime_with(provider, Arc::new(AllowAll)).await);
    let session = SessionId::new();
    let request = RequestId::new();

    let turn = {
        let runtime = Arc::clone(&runtime);
        let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_delta| {});
        tokio::spawn(async move {
            runtime
                .execute_outcome_streaming(
                    TurnRequest::new(session, "stop me").with_request(request),
                    sink,
                )
                .await
        })
    };
    let handle = await_handle(session, request).await;
    timeout(Duration::from_secs(5), started.notified())
        .await
        .expect("provider reached the gate");
    assert!(
        matches!(
            turn_cancellation::stop(session, request),
            StopOutcome::Stopped(_)
        ),
        "the stop must land on the live turn"
    );
    assert!(
        turn_cancellation::stop_summary(session, request).is_some(),
        "stop summary is the fact the final frame maps"
    );
    release.notify_one();

    let outcome = turn.await.expect("turn task").expect("turn outcome");
    assert!(
        matches!(outcome, TurnOutcome::Completed(_)),
        "stop does not abort model generation; the turn still completes"
    );
    let frames = replay_window(&handle);
    assert_eq!(frames.len(), 2, "one delta and one stopped final");
    let final_frame = only_final(&frames);
    assert_eq!(final_frame.state, C1StreamState::Stopped);
    assert!(final_frame.text.is_none());
    assert!(final_frame.error.is_none());
    assert!(final_frame.usage.is_none());
}

// ---------------------------------------------------------------------------
// 5. 非流式: 显式 request id 也不注册 C1 流
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_streaming_turn_with_explicit_request_id_registers_no_stream() {
    let provider = StreamProvider::new(vec![Scripted::Say("plain")], Vec::new());
    let runtime = runtime_with(provider, Arc::new(AllowAll)).await;
    let session = SessionId::new();
    let request = RequestId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "no stream").with_request(request))
        .await
        .expect("turn");
    assert!(matches!(outcome, TurnOutcome::Completed(_)));
    assert!(
        global_registry()
            .handle(&session.to_string(), &request.to_string())
            .is_none(),
        "the C1 stream is a streaming-turn fact, not a request-id fact"
    );
}
