//! Plan mode as a collaborative state projection, end to end through the real
//! turn chain.
//!
//! Nothing at the seam under test is mocked: the runtime, session store,
//! governance pipeline, provider router, and agent loop are the real
//! implementations. Only the two edges are substituted — a scripted provider
//! instead of a network call, and a frozen clock.
//!
//! What is proved here:
//! - a parked posture switch lands at the pre-step of the next accepted turn
//!   and never flips the projection inside a running turn;
//! - the resident `exit_plan_mode` control queues the switch without changing
//!   the current turn's prompt projection;
//! - switching alters the prompt projection and nothing else (tool catalog and
//!   governance verdicts are invariant);
//! - a rebuilt runtime recovers the folded posture from the session event log;
//! - a runtime that never engages the mechanism is unchanged.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{Action, AllowAll, Decision, GovernanceHook, GovernanceRequest};
use apeireth_orchestration::plan_mode::{
    PlanMode, PlanModeEvent, PlanModeLedger, PlanModeRequestOutcome, PLAN_MODE_BEHAVIOR_CONVENTION,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::{
    InMemorySessionStore, Runtime, SessionEventKind, TurnRequest, PLAN_MODE_EXIT_TOOL_NAME,
};
use apeireth_runtime::SessionStore;
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// What a scripted provider should do on a given round.
#[derive(Clone)]
enum Scripted {
    /// Ask for a tool.
    CallTool {
        call_id: &'static str,
        tool: &'static str,
        arguments: serde_json::Value,
    },
    /// Answer with text.
    Say(&'static str),
}

/// A provider that replays a script and records every request it received.
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
            Scripted::CallTool {
                call_id,
                tool,
                arguments,
            } => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: (*call_id).to_string(),
                    name: (*tool).to_string(),
                    arguments: arguments.clone(),
                }],
                ..base
            }),
            Scripted::Say(text) => Ok(NormalizedResponse {
                content: (*text).to_string(),
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
                "Scripted provider fixture",
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

/// A clock that never advances, so nothing depends on wall time.
fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

/// Extract a message's text, whatever content parts it holds.
fn message_text(message: &NormalizedMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(text.as_str()),
            ContentPart::ImageUrl { .. } => None,
        })
        .collect()
}

/// Whether one recorded provider request carries the planning convention.
fn request_has_plan_convention(request: &NormalizedRequest) -> bool {
    request
        .messages
        .iter()
        .any(|message| message_text(message).contains(PLAN_MODE_BEHAVIOR_CONVENTION))
}

/// The plan-mode ledger entries journaled in one stored session.
async fn plan_mode_events(store: &InMemorySessionStore, session: SessionId) -> Vec<PlanModeEvent> {
    store.load(&session).await.unwrap().unwrap().plan_mode_log()
}

async fn enabled_runtime(store: Arc<InMemorySessionStore>, provider: Arc<FakeProvider>) -> Runtime {
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider))
        .with_default_model(MODEL)
        .with_plan_mode_enabled(true)
        .build()
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 切换延迟落账: a switch requested between turns is journaled as intent only
/// (nothing posted yet) and lands at the pre-step of the next accepted turn,
/// where the prompt projection finally changes.
#[tokio::test]
async fn plan_mode_switch_lands_at_the_next_accepted_turn_pre_step() {
    let store = Arc::new(InMemorySessionStore::new());
    let provider = FakeProvider::new(
        "provider.fake",
        vec![Scripted::Say("one"), Scripted::Say("two")],
    );
    let runtime = enabled_runtime(store.clone(), provider.clone()).await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "first turn"))
        .await
        .unwrap();
    assert!(
        !request_has_plan_convention(&provider.request(0)),
        "before any switch the provider request is unchanged"
    );

    assert_eq!(
        runtime
            .request_plan_mode_switch(session, PlanMode::Planning)
            .await
            .unwrap(),
        PlanModeRequestOutcome::Queued
    );
    // Parked as intent, not posted: the projected posture has not changed.
    let parked = PlanModeLedger::from_events(plan_mode_events(&store, session).await).projection();
    assert_eq!(parked.mode, PlanMode::Executing, "nothing posted yet");
    assert_eq!(parked.pending_switch, Some(PlanMode::Planning));

    runtime
        .execute(TurnRequest::new(session, "second turn"))
        .await
        .unwrap();
    assert!(
        request_has_plan_convention(&provider.request(1)),
        "the next accepted turn runs under the landed posture"
    );
    let landed = PlanModeLedger::from_events(plan_mode_events(&store, session).await).projection();
    assert_eq!(landed.mode, PlanMode::Planning);
    assert_eq!(landed.state_version, 1);
    assert_eq!(landed.pending_switch, None);
}

/// 回合中途翻转被拦: the resident `exit_plan_mode` control queues the switch
/// while the current turn keeps its prompt projection unchanged — the landing
/// happens at the next accepted turn's pre-step, never mid-turn.
#[tokio::test]
async fn the_resident_exit_tool_queues_the_switch_without_flipping_mid_turn() {
    let store = Arc::new(InMemorySessionStore::new());
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            Scripted::CallTool {
                call_id: "call_exit",
                tool: PLAN_MODE_EXIT_TOOL_NAME,
                arguments: serde_json::json!({ "plan": "方案：分三步实施。" }),
            },
            Scripted::Say("done"),
            Scripted::Say("later"),
        ],
    );
    let runtime = enabled_runtime(store.clone(), provider.clone()).await;
    let session = SessionId::new();

    runtime
        .request_plan_mode_switch(session, PlanMode::Planning)
        .await
        .unwrap();

    runtime
        .execute(TurnRequest::new(session, "make a plan and exit"))
        .await
        .unwrap();

    assert!(
        request_has_plan_convention(&provider.request(0)),
        "the turn starts in planning posture"
    );
    assert!(
        request_has_plan_convention(&provider.request(1)),
        "the round after the exit call still runs under the same projection"
    );
    // The exit control answered the model with a queued switch, in-band.
    let round_two = provider.request(1);
    let tool_result_seen = round_two
        .messages
        .iter()
        .any(|message| message_text(message).contains("下一被接受回合"));
    assert!(
        tool_result_seen,
        "the model is told the switch lands at the next accepted turn"
    );
    // Mid-turn: the switch is parked, not posted.
    let mid_turn =
        PlanModeLedger::from_events(plan_mode_events(&store, session).await).projection();
    assert_eq!(mid_turn.mode, PlanMode::Planning, "no mid-turn flip");
    assert_eq!(mid_turn.pending_switch, Some(PlanMode::Executing));

    runtime
        .execute(TurnRequest::new(session, "next turn"))
        .await
        .unwrap();
    assert!(
        !request_has_plan_convention(&provider.request(2)),
        "the next accepted turn runs under the landed executing posture"
    );
    let landed = PlanModeLedger::from_events(plan_mode_events(&store, session).await).projection();
    assert_eq!(landed.mode, PlanMode::Executing);
    assert_eq!(landed.state_version, 2);
}

/// 执法独立: switching postures changes the prompt projection and nothing
/// else — the tool catalog is identical (the exit control stays resident in
/// both) and the governance verdict for the same action is identical.
#[tokio::test]
async fn a_mode_switch_leaves_the_tool_catalog_and_governance_verdicts_untouched() {
    struct RecordingVerdict;
    #[async_trait]
    impl GovernanceHook for RecordingVerdict {
        fn name(&self) -> &str {
            "recording_verdict"
        }
        async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
            Decision::Allow
        }
    }

    let store = Arc::new(InMemorySessionStore::new());
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("ok")]);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store.clone())
        .with_governance(Arc::new(RecordingVerdict))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_default_model(MODEL)
        .with_plan_mode_enabled(true)
        .build()
        .await
        .unwrap();
    let session = SessionId::new();

    let catalog_before = runtime.tool_declarations();
    assert!(
        catalog_before
            .iter()
            .any(|tool| tool.name == PLAN_MODE_EXIT_TOOL_NAME),
        "the exit control is resident"
    );

    let action = Action::CapabilityDispatch {
        capability: &CapabilityId::new("tool.demo").unwrap(),
        arguments: &serde_json::json!({}),
    };
    let verdict_before = runtime
        .governance()
        .evaluate(&GovernanceRequest::new(
            action,
            session,
            apeireth_core::kernel::TraceId::new(),
            1,
        ))
        .await;

    runtime
        .request_plan_mode_switch(session, PlanMode::Planning)
        .await
        .unwrap();

    let catalog_after = runtime.tool_declarations();
    assert_eq!(
        catalog_before, catalog_after,
        "a switch never changes the tool catalog"
    );
    let verdict_after = runtime
        .governance()
        .evaluate(&GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &CapabilityId::new("tool.demo").unwrap(),
                arguments: &serde_json::json!({}),
            },
            session,
            apeireth_core::kernel::TraceId::new(),
            1,
        ))
        .await;
    assert_eq!(
        verdict_before, verdict_after,
        "a switch never changes a governance verdict"
    );
}

/// resume 恢复: a rebuilt runtime (fresh process shape over the same store)
/// recovers the folded posture from the persisted event stream alone.
#[tokio::test]
async fn resuming_a_session_rebuilds_the_folded_mode_from_the_event_stream() {
    let store = Arc::new(InMemorySessionStore::new());
    let first = FakeProvider::new(
        "provider.fake",
        vec![Scripted::Say("one"), Scripted::Say("two")],
    );
    let runtime = enabled_runtime(store.clone(), first.clone()).await;
    let session = SessionId::new();

    runtime
        .request_plan_mode_switch(session, PlanMode::Planning)
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "planning turn"))
        .await
        .unwrap();
    assert!(request_has_plan_convention(&first.request(0)));

    // Rebuild: a different runtime instance over the same persisted session.
    let second = FakeProvider::new("provider.fake-2", vec![Scripted::Say("three")]);
    let rebuilt = enabled_runtime(store.clone(), second.clone()).await;
    rebuilt
        .execute(TurnRequest::new(session, "after restart"))
        .await
        .unwrap();
    assert!(
        request_has_plan_convention(&second.request(0)),
        "the folded posture is rebuilt from the event stream alone"
    );
    let folded = PlanModeLedger::from_events(plan_mode_events(&store, session).await).projection();
    assert_eq!(folded.mode, PlanMode::Planning);
    assert_eq!(folded.state_version, 1);
}

/// 默认行为零变化: a runtime that never enables the mechanism has no resident
/// exit control, projects nothing into the provider request, and journals no
/// plan-mode entries at all.
#[tokio::test]
async fn a_runtime_without_the_plan_mode_mechanism_is_unchanged() {
    let store = Arc::new(InMemorySessionStore::new());
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("plain")]);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store.clone())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "ordinary turn"))
        .await
        .unwrap();

    assert!(!request_has_plan_convention(&provider.request(0)));
    assert!(
        !runtime
            .tool_declarations()
            .iter()
            .any(|tool| tool.name == PLAN_MODE_EXIT_TOOL_NAME),
        "no extra tool surface by default"
    );
    assert!(
        plan_mode_events(&store, session).await.is_empty(),
        "no plan-mode entries are journaled"
    );
    assert!(
        runtime
            .request_plan_mode_switch(session, PlanMode::Planning)
            .await
            .is_err(),
        "the control surface refuses rather than pretending"
    );
}
