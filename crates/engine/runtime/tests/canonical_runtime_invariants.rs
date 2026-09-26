//! End-to-end proof of the runtime invariant registry at the dispatch
//! boundary.
//!
//! The runtime, plugin manager, capability registry, governance pipeline,
//! provider router, session store, and agent loop are the real
//! implementations; only the two edges are substituted: a scripted provider
//! and a counting tool.
//!
//! What is proved here:
//! - a duplicate side effect under one request is detected and attributed;
//! - log-only records the violation for audit and lets the turn continue;
//! - fail-fast blocks the duplicate before it executes;
//! - a disabled invariant stays silent;
//! - a healthy turn produces zero invariant noise.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_orchestration::runtime_invariants::{
    first_batch_auditor, InvariantAuditor, InvariantMode, INV_A_NO_DOUBLE_SIDE_EFFECT,
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
    InMemorySessionStore, Runtime, RuntimeError, SessionEventKind, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ToolSpec {
    call_id: &'static str,
    tool: &'static str,
    arguments: serde_json::Value,
}

#[derive(Clone)]
enum Scripted {
    /// One assistant round carrying several tool calls.
    CallTools(Vec<ToolSpec>),
    Say(&'static str),
}

struct FakeProvider {
    id: CapabilityId,
    script: Vec<Scripted>,
    calls: AtomicUsize,
    seen: Mutex<Vec<NormalizedRequest>>,
}

impl FakeProvider {
    fn new(id: &str, script: Vec<Scripted>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
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
        self.seen.lock().unwrap().push(request.clone());
        let step = self.script.get(index).unwrap_or_else(|| {
            panic!(
                "{} called {} times, script has {} steps",
                self.id,
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
            Scripted::CallTools(specs) => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: specs
                    .iter()
                    .map(|spec| ToolCall {
                        id: spec.call_id.to_string(),
                        name: spec.tool.to_string(),
                        arguments: spec.arguments.clone(),
                    })
                    .collect(),
                ..base
            }),
            Scripted::Say(text) => Ok(NormalizedResponse {
                content: (*text).to_string(),
                ..base
            }),
        }
    }
}

/// A tool that records every invocation (the observable side effect).
struct EffectfulTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for EffectfulTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "effectful".into(),
            description: Some("one side effect per invocation".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({"done": true}))
    }
}

struct ToolPlugin {
    manifest: PluginManifest,
    tool: Arc<EffectfulTool>,
    invocations: Arc<AtomicUsize>,
}

impl ToolPlugin {
    fn new(plugin_id: &str) -> (Arc<Self>, Arc<AtomicUsize>) {
        let invocations = Arc::new(AtomicUsize::new(0));
        let tool = Arc::new(EffectfulTool {
            id: CapabilityId::new("tool.effectful").unwrap(),
            invocations: Arc::clone(&invocations),
        });
        let plugin = Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new(plugin_id).unwrap(),
                "1.0.0",
                "effectful tool provider",
            )
            .declare_capability(
                CapabilityId::new("tool.effectful").unwrap(),
                CapabilityKind::Tool,
                "effectful tool",
            )
            .unwrap(),
            tool,
            invocations: Arc::clone(&invocations),
        });
        (plugin, invocations)
    }
}

#[async_trait]
impl Plugin for ToolPlugin {
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
        vec![Arc::clone(&self.tool) as Arc<dyn ToolCapability>]
    }
}

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
                "scripted completions",
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

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn spec(call_id: &'static str) -> ToolSpec {
    ToolSpec {
        call_id,
        tool: "effectful",
        arguments: serde_json::json!({ "op": "publish" }),
    }
}

/// One round whose two tool calls name the same call id: a degenerate
/// provider output that re-executes one side effect under one request.
fn duplicate_side_effect_round() -> Scripted {
    Scripted::CallTools(vec![spec("call_dup"), spec("call_dup")])
}

async fn runtime_with(
    script: Vec<Scripted>,
    auditor: Option<Arc<InvariantAuditor>>,
) -> (Runtime, Arc<AtomicUsize>) {
    let provider = FakeProvider::new("provider.fake", script);
    let (tool_plugin, invocations) = ToolPlugin::new("plugin.tool");
    let mut builder = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("plugin.provider", provider))
        .with_plugin(tool_plugin)
        .with_default_model(MODEL)
        .with_max_rounds(8)
        .with_session_store(Arc::new(InMemorySessionStore::new()));
    if let Some(auditor) = auditor {
        builder = builder.with_invariant_auditor(auditor);
    }
    (builder.build().await.unwrap(), invocations)
}

/// Recorded invariant violations of one session, in order.
async fn recorded_violations(runtime: &Runtime, session: SessionId) -> Vec<(String, String)> {
    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session is persisted");
    stored
        .events
        .iter()
        .filter_map(|event| match &event.event {
            SessionEventKind::InvariantViolation {
                invariant, module, ..
            } => Some((invariant.clone(), module.clone())),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn log_only_records_the_duplicate_side_effect_and_the_turn_continues() {
    let script = vec![duplicate_side_effect_round(), Scripted::Say("done")];
    let (runtime, invocations) = runtime_with(
        script,
        Some(Arc::new(first_batch_auditor(InvariantMode::LogOnly))),
    )
    .await;
    let session = SessionId::new();

    let outcome = runtime
        .execute(TurnRequest::new(session, "run the operation"))
        .await
        .expect("log-only never blocks the turn");

    assert_eq!(
        invocations.load(Ordering::SeqCst),
        2,
        "log-only observes and records; it does not intercept execution"
    );
    assert_eq!(outcome.text, "done");

    let violations = recorded_violations(&runtime, session).await;
    assert_eq!(
        violations,
        vec![(
            INV_A_NO_DOUBLE_SIDE_EFFECT.to_string(),
            "approval".to_string()
        )],
        "the duplicate is recorded once, with its module attribution"
    );
}

#[tokio::test]
async fn fail_fast_blocks_the_duplicate_side_effect_before_it_executes() {
    let script = vec![duplicate_side_effect_round(), Scripted::Say("done")];
    let (runtime, invocations) = runtime_with(
        script,
        Some(Arc::new(first_batch_auditor(InvariantMode::FailFast))),
    )
    .await;
    let session = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session, "run the operation"))
        .await
        .expect_err("fail-fast must block the duplicate");
    assert!(
        matches!(&error, RuntimeError::InvariantBlocked { invariant, .. }
            if invariant == INV_A_NO_DOUBLE_SIDE_EFFECT),
        "{error}"
    );
    assert!(
        error.to_string().contains(INV_A_NO_DOUBLE_SIDE_EFFECT),
        "{error}"
    );
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "the duplicate side effect must never execute"
    );
}

#[tokio::test]
async fn a_disabled_invariant_stays_silent_at_the_dispatch_boundary() {
    let script = vec![duplicate_side_effect_round(), Scripted::Say("done")];
    let auditor = first_batch_auditor(InvariantMode::LogOnly);
    assert!(auditor.set_enabled(INV_A_NO_DOUBLE_SIDE_EFFECT, false));
    let (runtime, invocations) = runtime_with(script, Some(Arc::new(auditor))).await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "run the operation"))
        .await
        .expect("a disabled invariant cannot block or record");

    assert_eq!(invocations.load(Ordering::SeqCst), 2);
    assert!(
        recorded_violations(&runtime, session).await.is_empty(),
        "the disabled invariant must stay silent"
    );
}

#[tokio::test]
async fn a_healthy_turn_produces_zero_invariant_noise() {
    let script = vec![
        Scripted::CallTools(vec![
            spec("call_1"),
            ToolSpec {
                call_id: "call_2",
                tool: "effectful",
                arguments: serde_json::json!({ "op": "draft" }),
            },
        ]),
        Scripted::Say("done"),
    ];
    let (runtime, invocations) = runtime_with(
        script,
        Some(Arc::new(first_batch_auditor(InvariantMode::LogOnly))),
    )
    .await;
    let session = SessionId::new();

    let outcome = runtime
        .execute(TurnRequest::new(session, "run two different operations"))
        .await
        .expect("a healthy turn completes");

    assert_eq!(outcome.text, "done");
    assert_eq!(invocations.load(Ordering::SeqCst), 2);
    assert!(
        recorded_violations(&runtime, session).await.is_empty(),
        "distinct operations under one request are ordinary tool chains"
    );
}
