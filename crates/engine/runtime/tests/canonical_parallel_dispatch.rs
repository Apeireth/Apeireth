//! Tool-dispatch-face proof of the parallel call classification scheduler.
//!
//! The runtime under test is real end to end; only the two edges are scripted:
//! a provider that emits several tool calls in one round, and a gated tool that
//! records its start/end order. What is proved here is the dispatch surface's
//! classification contract: explicit allow rules open overlap, everything else
//! stays serial, unsafe calls form barriers, results commit in model order, and
//! an approval pause never dispatches anything twice.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{Action, AllowAll, Decision, GovernanceHook, GovernanceRequest};
use apeireth_orchestration::{CallScheduler, SafetyRule, SafetyWhitelist, SchedulerConfig};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    MessageRole, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, InMemorySessionStore, Runtime, TurnOutcome, TurnRequest,
};
use async_trait::async_trait;
use serde_json::{json, Value};

const MODEL: &str = "fake-model-1";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// One scripted provider step: either several tool calls in one round or a
/// final answer.
#[derive(Clone)]
enum ScriptStep {
    CallTools(Vec<(String, String, Value)>),
    Say(String),
}

/// A provider that replays a script and records every request it received.
struct FakeProvider {
    id: CapabilityId,
    script: Vec<ScriptStep>,
    calls: Mutex<usize>,
    seen: Mutex<Vec<NormalizedRequest>>,
}

impl FakeProvider {
    fn new(id: &str, script: Vec<ScriptStep>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: Mutex::new(0),
            seen: Mutex::new(Vec::new()),
        })
    }

    fn request(&self, index: usize) -> NormalizedRequest {
        self.seen.lock().unwrap()[index].clone()
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
        let index = {
            let mut calls = self.calls.lock().unwrap();
            let index = *calls;
            *calls += 1;
            index
        };
        self.seen.lock().unwrap().push(request.clone());

        let step = self.script.get(index).unwrap_or_else(|| {
            panic!(
                "provider called {} times, script has {} steps",
                index + 1,
                self.script.len()
            )
        });

        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
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

        match step {
            ScriptStep::CallTools(calls) => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: calls
                    .iter()
                    .map(|(id, tool, arguments)| ToolCall {
                        id: id.clone(),
                        name: tool.clone(),
                        arguments: arguments.clone(),
                    })
                    .collect(),
                ..base
            }),
            ScriptStep::Say(text) => Ok(NormalizedResponse {
                content: text.clone(),
                ..base
            }),
        }
    }
}

/// A plugin providing a scripted provider.
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
}

impl ProviderPlugin {
    fn new(plugin_id: &str, provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new(plugin_id).unwrap(),
                "1.0.0",
                "Scripted completions",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Scripted completions",
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

/// A tool whose every invocation logs its start and end, then answers with the
/// call id. `delay_ms` stretches the invocation so overlap is observable.
struct GatedTool {
    id: CapabilityId,
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl GatedTool {
    fn new(name: &'static str, log: Arc<Mutex<Vec<String>>>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(format!("tool.{name}")).unwrap(),
            name,
            log,
        })
    }
}

#[async_trait]
impl ToolCapability for GatedTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.into(),
            description: Some("Gated instrumented call".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let label = call
            .arguments
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        self.log.lock().unwrap().push(format!("start:{label}"));
        let delay = call
            .arguments
            .get("delay_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if delay > 0 {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        self.log.lock().unwrap().push(format!("end:{label}"));
        ToolResult::ok(&call.id, json!({ "done": label })).with_name(self.name)
    }
}

/// Governance that pauses exactly the calls flagged `needs_approval`.
struct ApproveFlaggedCalls;

#[async_trait]
impl GovernanceHook for ApproveFlaggedCalls {
    fn name(&self) -> &str {
        "test.flagged-approval"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match &request.action {
            Action::CapabilityDispatch { arguments, .. }
                if arguments.get("needs_approval").and_then(Value::as_bool) == Some(true) =>
            {
                Decision::require_approval("flagged for a human decision")
            }
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

fn call(label: &str, delay_ms: u64) -> (String, String, Value) {
    (
        format!("call_{label}"),
        "gated".into(),
        json!({ "label": label, "delay_ms": delay_ms }),
    )
}

fn flagged_call(label: &str) -> (String, String, Value) {
    (
        format!("call_{label}"),
        "gated".into(),
        json!({ "label": label, "delay_ms": 5, "needs_approval": true }),
    )
}

/// The tool result message ids in transcript order.
async fn result_ids(runtime: &Runtime, session_id: SessionId) -> Vec<String> {
    let session = runtime.sessions().load(&session_id).await.unwrap().unwrap();
    session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .map(|message| message.tool_call_id.clone().unwrap_or_default())
        .collect()
}

fn at(log: &[String], needle: &str) -> usize {
    log.iter()
        .position(|entry| entry == needle)
        .unwrap_or_else(|| panic!("{needle} not in event log: {log:?}"))
}

fn par_scheduler(max_parallel: usize) -> Arc<CallScheduler> {
    Arc::new(
        CallScheduler::new(
            Arc::new(SafetyWhitelist::new().allow(SafetyRule::for_tool("gated"))),
            SchedulerConfig {
                max_parallel,
                wind_down: Duration::from_millis(300),
            },
        )
        .unwrap(),
    )
}

async fn build_runtime(
    script: Vec<ScriptStep>,
    log: Arc<Mutex<Vec<String>>>,
    governance: Option<Arc<dyn GovernanceHook>>,
    scheduler: Option<Arc<CallScheduler>>,
    extra_tools: Vec<Arc<dyn ToolCapability>>,
) -> (Runtime, Arc<FakeProvider>) {
    let provider = FakeProvider::new("provider.fake", script);
    let mut builder = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(governance.unwrap_or_else(|| Arc::new(AllowAll)))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_tool(GatedTool::new("gated", log))
        .with_default_model(MODEL);
    for tool in extra_tools {
        builder = builder.with_tool(tool);
    }
    if let Some(scheduler) = scheduler {
        builder = builder.with_call_scheduler(scheduler);
    }
    let runtime = builder.build().await.unwrap();
    (runtime, provider)
}

// ---------------------------------------------------------------------------
// Classification at the dispatch face
// ---------------------------------------------------------------------------

#[tokio::test]
async fn parallel_window_commits_results_in_model_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    // Completion order is forced to c3, c2, c1 (倒序) by the delays.
    let script = vec![
        ScriptStep::CallTools(vec![call("c1", 80), call("c2", 40), call("c3", 5)]),
        ScriptStep::Say("done".into()),
    ];
    let (runtime, _provider) = build_runtime(
        script,
        Arc::clone(&log),
        None,
        Some(par_scheduler(3)),
        Vec::new(),
    )
    .await;

    let session_id = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session_id, "run three"))
        .await
        .unwrap();
    assert_eq!(response.text, "done");

    // 结果提交保持模型序: 乱序完成也按序交付。
    assert_eq!(
        result_ids(&runtime, session_id).await,
        vec!["call_c1", "call_c2", "call_c3"]
    );

    let log = log.lock().unwrap().clone();
    // 派发重叠: c3 的完成早于 c1/c2 的完成, 且三条都已启动。
    assert_eq!(
        (
            at(&log, "end:c3") < at(&log, "end:c2"),
            at(&log, "end:c2") < at(&log, "end:c1")
        ),
        (true, true),
        "完成次序被延迟强制为倒序: {log:?}"
    );
    assert!(
        at(&log, "start:c1") < at(&log, "end:c3") && at(&log, "start:c2") < at(&log, "end:c3"),
        "并行窗口内必须真重叠: {log:?}"
    );
}

#[tokio::test]
async fn without_a_scheduler_dispatch_stays_strictly_serial() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let script = vec![
        ScriptStep::CallTools(vec![call("c1", 20), call("c2", 5)]),
        ScriptStep::Say("done".into()),
    ];
    let (runtime, _provider) =
        build_runtime(script, Arc::clone(&log), None, None, Vec::new()).await;

    let session_id = SessionId::new();
    runtime
        .execute(TurnRequest::new(session_id, "run two"))
        .await
        .unwrap();

    assert_eq!(
        result_ids(&runtime, session_id).await,
        vec!["call_c1", "call_c2"]
    );
    assert_eq!(
        *log.lock().unwrap(),
        vec!["start:c1", "end:c1", "start:c2", "end:c2"],
        "未安装调度器即一切调用严格串行"
    );
}

#[tokio::test]
async fn an_unconfigured_whitelist_keeps_every_call_serial() {
    // 判定器默认失败关闭: 装了调度器但没显式放行, 也不得重叠。
    let log = Arc::new(Mutex::new(Vec::new()));
    let script = vec![
        ScriptStep::CallTools(vec![call("c1", 20), call("c2", 5)]),
        ScriptStep::Say("done".into()),
    ];
    let blank = Arc::new(
        CallScheduler::new(
            Arc::new(SafetyWhitelist::new()),
            SchedulerConfig {
                max_parallel: 4,
                wind_down: Duration::from_millis(300),
            },
        )
        .unwrap(),
    );
    let (runtime, _provider) =
        build_runtime(script, Arc::clone(&log), None, Some(blank), Vec::new()).await;

    let session_id = SessionId::new();
    runtime
        .execute(TurnRequest::new(session_id, "run two"))
        .await
        .unwrap();

    assert_eq!(
        *log.lock().unwrap(),
        vec!["start:c1", "end:c1", "start:c2", "end:c2"],
        "空白名单失败关闭: 调用仍严格串行"
    );
}

#[tokio::test]
async fn an_unsafe_call_forms_a_barrier_between_parallel_windows() {
    let log = Arc::new(Mutex::new(Vec::new()));
    // [safe, safe, unsafe, safe, safe]: unsafe 调用独占执行, 前后成屏障。
    let script = vec![
        ScriptStep::CallTools(vec![
            call("s1", 40),
            call("s2", 40),
            (
                "call_x".into(),
                "plain".into(),
                json!({ "label": "x", "delay_ms": 40 }),
            ),
            call("s3", 40),
            call("s4", 40),
        ]),
        ScriptStep::Say("done".into()),
    ];
    let (runtime, _provider) = build_runtime(
        script,
        Arc::clone(&log),
        None,
        Some(par_scheduler(2)),
        vec![GatedTool::new("plain", Arc::clone(&log)) as Arc<dyn ToolCapability>],
    )
    .await;

    let session_id = SessionId::new();
    runtime
        .execute(TurnRequest::new(session_id, "mixed stream"))
        .await
        .unwrap();

    assert_eq!(
        result_ids(&runtime, session_id).await,
        vec!["call_s1", "call_s2", "call_x", "call_s3", "call_s4"]
    );

    let log = log.lock().unwrap().clone();
    // 同窗口内重叠。
    assert!(at(&log, "start:s2") < at(&log, "end:s1"), "{log:?}");
    assert!(at(&log, "start:s4") < at(&log, "end:s3"), "{log:?}");
    // 未放行调用与前后窗口互斥 (屏障)。
    assert!(
        at(&log, "end:s1") < at(&log, "start:x") && at(&log, "end:s2") < at(&log, "start:x"),
        "屏障前必须排空: {log:?}"
    );
    assert!(
        at(&log, "end:x") < at(&log, "start:s3") && at(&log, "end:x") < at(&log, "start:s4"),
        "屏障后才可再派发: {log:?}"
    );
}

#[tokio::test]
async fn an_approval_inside_a_window_pauses_without_double_dispatch() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let script = vec![
        ScriptStep::CallTools(vec![call("a1", 5), flagged_call("a2")]),
        ScriptStep::Say("done".into()),
    ];
    let (runtime, _provider) = build_runtime(
        script,
        Arc::clone(&log),
        Some(Arc::new(ApproveFlaggedCalls)),
        Some(par_scheduler(2)),
        Vec::new(),
    )
    .await;

    let session_id = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session_id, "run with one flagged"))
        .await
        .unwrap();
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("the flagged call must pause for a human decision");
    };
    assert_eq!(view.tool_call.id, "call_a2");

    // 窗口在暂停点收窄: a1 已执行并提交, a2 未被启动。
    assert_eq!(result_ids(&runtime, session_id).await, vec!["call_a1"]);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["start:a1", "end:a1"],
        "暂停的调用在批准前不得执行"
    );

    match runtime
        .resolve_approval(session_id, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap()
    {
        ApprovalResolution::Resumed(TurnOutcome::Completed(response)) => {
            assert_eq!(response.text, "done");
        }
        other => panic!("approval resume must complete the turn: {other:?}"),
    }

    // 恢复后按模型序补齐 a2, 且两个调用各执行恰好一次 (不双跑)。
    assert_eq!(
        result_ids(&runtime, session_id).await,
        vec!["call_a1", "call_a2"]
    );
    let log = log.lock().unwrap().clone();
    assert_eq!(
        log.iter()
            .filter(|entry| entry.as_str() == "start:a1" || entry.as_str() == "end:a1")
            .count(),
        2,
        "a1 不得因恢复而重跑: {log:?}"
    );
    assert_eq!(
        log.iter()
            .filter(|entry| entry.as_str() == "start:a2" || entry.as_str() == "end:a2")
            .count(),
        2,
        "a2 执行恰好一次: {log:?}"
    );
    assert!(at(&log, "start:a2") > at(&log, "end:a1"), "{log:?}");
}
