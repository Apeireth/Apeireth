//! W3.3 SG-16: 痛觉评估在真实派发调用点上的接线证据 (默认关).
//!
//! 这里跑的是真实运行时: 插件管理、能力注册表、治理、provider 路由、会话、
//! agent 循环都是生产实现; 只有两个边缘被替换 —— 脚本化 provider 与一个总是
//! 失败的工具。目的不是证明真实网络工具失败 (未手测), 而是证明:
//!
//! 1. 串行派发调用点 (`execute.rs` `dispatch_one_tool` → `record_call_completed`)
//!    在开关开时记录一次评估, 关时观测面全空, 且两种模式的 `ToolFailed` 事件与
//!    trace 逐字节一致 (评估不污染事实流);
//! 2. 并行窗口正常提交调用点 (同函数, 每个 `SlotOutcome::Completed` 各一次)
//!    对每笔失败各记一次评估, 同因归入同组。
//!
//! 评估只记录建议 (`Retry` / `Diagnose`), 不改变本回合的工具结果、错误文本或
//! 重试行为 —— 两次运行都走完 provider 第二轮并正常收束。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_orchestration::nociception::{FailureClass, Recommendation};
use apeireth_orchestration::recovery_protocol::HealingPhase;
use apeireth_orchestration::{CallScheduler, SafetyRule, SafetyWhitelist, SchedulerConfig};
use apeireth_organ::nociception::{
    BioPainSnapshot, BioRecoveryPhaseCounts, BioRecoverySnapshot, NociceptionObserver,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool,
    NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{Runtime, SessionEventKind, TraceEvent, TurnRequest};
use async_trait::async_trait;
use serde_json::json;

const MODEL: &str = "fake-model-1";
const FLAKY_TOOL: &str = "flaky";
const POISON_TOOL: &str = "poison";
const RECOVERING_TOOL: &str = "recovering";
const PAIN_NOW_MS: i64 = 1_700_000_000_000;

// ---------------------------------------------------------------------------
// 脚本化 provider
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Step {
    CallTools(Vec<ToolCall>),
    Say(&'static str),
}

struct FakeProvider {
    id: CapabilityId,
    script: Vec<Step>,
    calls: Mutex<usize>,
}

impl FakeProvider {
    fn new(script: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").expect("provider id"),
            script,
            calls: Mutex::new(0),
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
            ModelDescriptor::new(ModelId::new(MODEL).expect("model id"), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = {
            let mut calls = self.calls.lock().expect("provider calls");
            let index = *calls;
            *calls += 1;
            index
        };
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
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        match step {
            Step::CallTools(calls) => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: calls.clone(),
                ..base
            }),
            Step::Say(text) => Ok(NormalizedResponse {
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
                PluginId::new("vendor.fake").expect("plugin id"),
                "1.0.0",
                "Scripted completions",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Scripted completions",
            )
            .expect("declare provider"),
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
// 总是失败的工具
// ---------------------------------------------------------------------------

struct FlakyTool {
    id: CapabilityId,
}

#[async_trait]
impl ToolCapability for FlakyTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: FLAKY_TOOL.to_string(),
            description: Some("Always fails with a retryable upstream error".to_string()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::retryable_error(&call.id, "upstream timeout")
    }
}

struct FlakyPlugin {
    manifest: PluginManifest,
}

impl FlakyPlugin {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.flaky").expect("plugin id"),
                "1.0.0",
                "A tool that always fails",
            )
            .declare_capability(
                CapabilityId::new("tool.flaky").expect("capability id"),
                CapabilityKind::Tool,
                "Always fails",
            )
            .expect("declare tool"),
        })
    }
}

#[async_trait]
impl Plugin for FlakyPlugin {
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
        vec![Arc::new(FlakyTool {
            id: CapabilityId::new("tool.flaky").expect("capability id"),
        })]
    }
}

// ---------------------------------------------------------------------------
// 再生案例用的脚本化工具: 永久失败 / 先失败后成功
// ---------------------------------------------------------------------------

struct PoisonTool {
    id: CapabilityId,
}

#[async_trait]
impl ToolCapability for PoisonTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: POISON_TOOL.to_string(),
            description: Some("Always fails with a permanent (systematic) error".to_string()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::permanent_error(&call.id, "invalid argument")
    }
}

/// 首次调用永久失败, 之后成功: 同一能力先开案例、再以成功收尾。
struct RecoveringTool {
    id: CapabilityId,
    invocations: AtomicUsize,
}

#[async_trait]
impl ToolCapability for RecoveringTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: RECOVERING_TOOL.to_string(),
            description: Some("Fails once, then succeeds".to_string()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let invocation = self.invocations.fetch_add(1, Ordering::SeqCst);
        if invocation == 0 {
            ToolResult::permanent_error(&call.id, "invalid argument")
        } else {
            ToolResult::ok(&call.id, json!({ "recovered": true }))
        }
    }
}

struct ScriptedToolPlugin {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

impl ScriptedToolPlugin {
    fn new() -> Arc<Self> {
        let poison = CapabilityId::new("tool.poison").expect("capability id");
        let recovering = CapabilityId::new("tool.recovering").expect("capability id");
        let manifest = PluginManifest::new(
            PluginId::new("builtin.scripted").expect("plugin id"),
            "1.0.0",
            "Scripted permanent-failure and recovering tools",
        )
        .declare_capability(
            poison.clone(),
            CapabilityKind::Tool,
            "Always fails permanently",
        )
        .expect("declare poison")
        .declare_capability(
            recovering.clone(),
            CapabilityKind::Tool,
            "Fails once then succeeds",
        )
        .expect("declare recovering");
        Arc::new(Self {
            manifest,
            tools: vec![
                Arc::new(PoisonTool { id: poison }),
                Arc::new(RecoveringTool {
                    id: recovering,
                    invocations: AtomicUsize::new(0),
                }),
            ],
        })
    }
}

#[async_trait]
impl Plugin for ScriptedToolPlugin {
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
        self.tools.clone()
    }
}

// ---------------------------------------------------------------------------
// 组装
// ---------------------------------------------------------------------------

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(PAIN_NOW_MS)
            .expect("fixed timestamp")
            .as_datetime(),
    ))
}

fn now_ms() -> u64 {
    u64::try_from(PAIN_NOW_MS).expect("fixed clock is non-negative")
}

fn tool_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        name: FLAKY_TOOL.to_string(),
        arguments: json!({}),
    }
}

fn named_tool_call(name: &str, id: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        name: name.to_string(),
        arguments: json!({}),
    }
}

/// 观察者只在开关开启时构造并安装; 未开启 = 内核零接线。
fn pain_observer(bio_pain: bool) -> Option<Arc<NociceptionObserver>> {
    bio_pain.then(|| Arc::new(NociceptionObserver::new()))
}

fn pain_snapshot(observer: &Option<Arc<NociceptionObserver>>) -> BioPainSnapshot {
    observer
        .as_ref()
        .map(|observer| observer.snapshot())
        .unwrap_or_default()
}

async fn build_scripted_runtime(
    script: Vec<Step>,
    bio_pain: bool,
) -> (Runtime, Option<Arc<NociceptionObserver>>) {
    let provider = FakeProvider::new(script);
    let observer = pain_observer(bio_pain);
    let mut builder = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider))
        .with_plugin(ScriptedToolPlugin::new())
        .with_default_model(MODEL);
    if let Some(observer) = &observer {
        builder = builder.with_tool_completion_observer(observer.clone());
    }
    let runtime = builder.build().await.expect("runtime builds");
    (runtime, observer)
}

async fn build_runtime(
    script: Vec<Step>,
    bio_pain: bool,
    parallel: bool,
) -> (Runtime, Option<Arc<NociceptionObserver>>) {
    let provider = FakeProvider::new(script);
    let observer = pain_observer(bio_pain);
    let mut builder = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider))
        .with_plugin(FlakyPlugin::new())
        .with_default_model(MODEL);
    if let Some(observer) = &observer {
        builder = builder.with_tool_completion_observer(observer.clone());
    }
    if parallel {
        let scheduler = CallScheduler::new(
            Arc::new(SafetyWhitelist::new().allow(SafetyRule::for_tool(FLAKY_TOOL))),
            SchedulerConfig {
                max_parallel: 2,
                wind_down: Duration::from_millis(300),
            },
        )
        .expect("scheduler config");
        builder = builder.with_call_scheduler(Arc::new(scheduler));
    }
    let runtime = builder.build().await.expect("runtime builds");
    (runtime, observer)
}

fn failed_completions(outcome: &apeireth_runtime::canonical::TurnResponse) -> usize {
    outcome
        .trace
        .events()
        .filter(|event| {
            matches!(
                event,
                TraceEvent::CapabilityCompleted {
                    succeeded: false,
                    ..
                }
            )
        })
        .count()
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

/// 串行派发调用点: 开关关 → 观测面全空; 开 → 一次评估。两种模式下失败证据
/// (会话事件 + trace) 逐字节一致。
#[tokio::test]
async fn the_serial_call_site_records_pain_only_when_the_flag_is_on() {
    let mut serialized_traces = Vec::new();
    let mut serialized_events = Vec::new();

    for enabled in [false, true] {
        let (runtime, observer) = build_runtime(
            vec![
                Step::CallTools(vec![tool_call("call_1")]),
                Step::Say("I saw the failure."),
            ],
            enabled,
            false,
        )
        .await;
        let session_id = SessionId::new();
        let outcome = runtime
            .execute(TurnRequest::new(session_id, "try the flaky tool"))
            .await
            .expect("the turn completes despite the tool failure");

        assert_eq!(
            failed_completions(&outcome),
            1,
            "失败仍是本回合的一次完成事实"
        );
        assert_eq!(outcome.text, "I saw the failure.");
        let persisted = runtime
            .sessions()
            .load_or_create(session_id)
            .await
            .expect("session persists");
        assert!(
            persisted.events.iter().any(|event| matches!(
                &event.event,
                SessionEventKind::ToolFailed {
                    capability: Some(capability),
                    tool_call_id,
                    error,
                    ..
                } if capability.as_str() == "tool.flaky"
                    && tool_call_id == "call_1"
                    && error == "error: upstream timeout"
            )),
            "契约事件仍在: {enabled}"
        );

        // 两次运行的身份 (trace/session/request id) 必然不同, 可比的是事实本身:
        // 轨迹条目 (时刻 + 事件) 与会话事件载荷。
        serialized_traces
            .push(serde_json::to_string(&outcome.trace.entries).expect("trace entries serialize"));
        let event_payloads: Vec<&SessionEventKind> =
            persisted.events.iter().map(|entry| &entry.event).collect();
        serialized_events.push(serde_json::to_string(&event_payloads).expect("events serialize"));

        let snapshot = pain_snapshot(&observer);
        assert_eq!(snapshot.enabled, enabled);
        if enabled {
            assert_eq!(snapshot.assessed, 1);
            assert_eq!(snapshot.observations.len(), 1);
            assert_eq!(snapshot.observations[0].class, FailureClass::Transient);
            assert_eq!(snapshot.observations[0].recommended, Recommendation::Retry);
            assert_eq!(snapshot.groups.len(), 1);
            assert_eq!(snapshot.groups[0].hits, 1);
            assert_eq!(
                snapshot.recovery.active_cases, 0,
                "Retry 不建再生案例 (首次瞬时失败)"
            );
            assert_eq!(snapshot.recovery.closed_cases, 0);
        } else {
            assert_eq!(snapshot.assessed, 0, "默认关不评估");
            assert_eq!(snapshot.repaired, 0);
            assert!(snapshot.observations.is_empty());
            assert!(snapshot.groups.is_empty());
            assert_eq!(
                snapshot.recovery,
                BioRecoverySnapshot::default(),
                "默认关再生案例账逐字段为空"
            );
        }
    }

    assert_eq!(
        serialized_traces[0], serialized_traces[1],
        "开关不得改变 trace 一个字节"
    );
    assert_eq!(
        serialized_events[0], serialized_events[1],
        "开关不得改变会话事件一个字节"
    );
}

/// 并行窗口正常提交调用点: 同一轮两笔失败各记一次评估, 同因归入同组。
#[tokio::test]
async fn the_parallel_window_call_site_records_one_assessment_per_failed_call() {
    let (runtime, observer) = build_runtime(
        vec![
            Step::CallTools(vec![tool_call("call_a"), tool_call("call_b")]),
            Step::Say("Both failed."),
        ],
        true,
        true,
    )
    .await;
    let session_id = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session_id, "try two flaky calls"))
        .await
        .expect("the turn completes");

    assert_eq!(failed_completions(&outcome), 2, "并行窗口两笔失败各一轨迹");
    let persisted = runtime
        .sessions()
        .load_or_create(session_id)
        .await
        .expect("session persists");
    let failed_events = persisted
        .events
        .iter()
        .filter(|event| matches!(&event.event, SessionEventKind::ToolFailed { .. }))
        .count();
    assert_eq!(failed_events, 2, "两笔失败各一条会话事件");

    let snapshot = pain_snapshot(&observer);
    assert_eq!(snapshot.assessed, 2);
    assert_eq!(snapshot.observations.len(), 2);
    assert_eq!(
        snapshot.groups.len(),
        1,
        "同能力同原因 (同错误文本) 归入一组"
    );
    assert_eq!(snapshot.groups[0].hits, 2);
    assert_eq!(snapshot.observations[0].recommended, Recommendation::Retry);
    assert_eq!(snapshot.observations[1].recommended, Recommendation::Retry);
    assert!(
        !snapshot.observations[1].chronic,
        "两次未达慢性阈值, 评估不升级"
    );
    assert_eq!(
        snapshot.recovery.active_cases, 0,
        "并行窗口的 Retry 同样不建再生案例"
    );
    assert_eq!(snapshot.recovery.closed_cases, 0);
}

// ---------------------------------------------------------------------------
// 再生案例 (W3.4 SG-17): 真实失败链上的开案/收尾 + 默认关零变化
// ---------------------------------------------------------------------------

fn recovery_script() -> Vec<Step> {
    vec![
        Step::CallTools(vec![named_tool_call(RECOVERING_TOOL, "call_recover_1")]),
        Step::Say("the first call failed"),
        Step::CallTools(vec![named_tool_call(RECOVERING_TOOL, "call_recover_2")]),
        Step::Say("the second call recovered"),
    ]
}

/// 同一能力先永久失败 (开案) 再成功 (收尾); 开关关时案例账逐字段为空, 且两模式
/// 的 trace 与会话事件载荷逐字节一致。
#[tokio::test]
async fn the_recovery_case_opens_on_a_failure_and_closes_after_a_success() {
    let mut serialized_traces: Vec<Vec<String>> = Vec::new();
    let mut serialized_events: Vec<Vec<String>> = Vec::new();

    for enabled in [false, true] {
        let (runtime, observer) = build_scripted_runtime(recovery_script(), enabled).await;
        let mut turn_traces = Vec::new();
        let mut turn_events = Vec::new();

        for (turn, prompt) in ["first call", "second call"].iter().enumerate() {
            let session_id = SessionId::new();
            let outcome = runtime
                .execute(TurnRequest::new(session_id, *prompt))
                .await
                .expect("the turn completes");
            let persisted = runtime
                .sessions()
                .load_or_create(session_id)
                .await
                .expect("session persists");
            let event_payloads: Vec<&SessionEventKind> =
                persisted.events.iter().map(|entry| &entry.event).collect();
            turn_traces
                .push(serde_json::to_string(&outcome.trace.entries).expect("trace serializes"));
            turn_events.push(serde_json::to_string(&event_payloads).expect("events serialize"));

            let snapshot = pain_snapshot(&observer);
            if let Some(observer) = &observer {
                assert_eq!(
                    observer.recovery_snapshot(),
                    snapshot.recovery,
                    "两个只读访问器同源"
                );
            }
            if !enabled {
                assert_eq!(
                    snapshot.recovery,
                    BioRecoverySnapshot::default(),
                    "默认关: 再生案例账逐字段为空 (turn {turn})"
                );
                continue;
            }
            if turn == 0 {
                assert_eq!(snapshot.recovery.active_cases, 1);
                assert_eq!(snapshot.recovery.closed_cases, 0);
                let case = &snapshot.recovery.cases[0];
                assert_eq!(case.capability, "tool.recovering");
                assert_eq!(
                    case.phase,
                    HealingPhase::Inflammation,
                    "系统性失败 → Diagnose → 炎症期"
                );
                assert_eq!(case.started_ms, now_ms(), "注入时钟决定开案时间");
                assert_eq!(snapshot.recovery.phases.inflammation, 1);
            } else {
                assert_eq!(
                    snapshot.recovery.active_cases, 0,
                    "成功收尾后移出活跃账 (不泄漏)"
                );
                assert_eq!(snapshot.recovery.closed_cases, 1);
                assert_eq!(snapshot.recovery.phases, BioRecoveryPhaseCounts::default());
            }
        }

        serialized_traces.push(turn_traces);
        serialized_events.push(turn_events);
    }

    assert_eq!(
        serialized_traces[0], serialized_traces[1],
        "开关不得改变 trace 一个字节"
    );
    assert_eq!(
        serialized_events[0], serialized_events[1],
        "开关不得改变会话事件载荷一个字节"
    );
}

/// 跨回合连续同因失败: 只开一个案例, 每次失败推进一期 (炎症→增生→重塑)。
#[tokio::test]
async fn repeated_wired_failures_advance_one_case_without_duplicating_it() {
    let script = vec![
        Step::CallTools(vec![named_tool_call(POISON_TOOL, "call_poison_1")]),
        Step::Say("first failure"),
        Step::CallTools(vec![named_tool_call(POISON_TOOL, "call_poison_2")]),
        Step::Say("second failure"),
        Step::CallTools(vec![named_tool_call(POISON_TOOL, "call_poison_3")]),
        Step::Say("third failure"),
    ];
    let (runtime, observer) = build_scripted_runtime(script, true).await;
    let expected = [
        HealingPhase::Inflammation,
        HealingPhase::Proliferation,
        HealingPhase::Remodeling,
    ];
    for (turn, expected_phase) in expected.iter().enumerate() {
        let outcome = runtime
            .execute(TurnRequest::new(
                SessionId::new(),
                format!("turn {turn} tries the poison tool"),
            ))
            .await
            .expect("the turn completes");
        assert_eq!(failed_completions(&outcome), 1);
        let snapshot = pain_snapshot(&observer);
        assert_eq!(
            snapshot.recovery.active_cases, 1,
            "同因多次失败不得重复开案例 (turn {turn})"
        );
        assert_eq!(
            snapshot.recovery.cases[0].phase, *expected_phase,
            "turn {turn} 阶段推进"
        );
        assert_eq!(snapshot.recovery.closed_cases, 0, "失败驱动不收尾");
        assert_eq!(snapshot.recovery.rejected_transitions, 0);
    }
    let snapshot = pain_snapshot(&observer);
    assert_eq!(snapshot.recovery.cases[0].capability, "tool.poison");
    assert_eq!(snapshot.assessed, 3, "三次失败各一次痛觉评估");
    assert_eq!(snapshot.groups.len(), 1, "同因归入同一痛觉组");
}
