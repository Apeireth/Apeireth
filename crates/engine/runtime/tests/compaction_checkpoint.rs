//! Compress checkpoints in the runtime: the trigger, the fold, and the
//! zero-damage guarantee.
//!
//! The chain being proved:
//!
//! ```text
//!   long transcript crosses the shared overflow trigger
//!        -> one summary call through the injected generator
//!        -> checkpoint recorded under its marker pair (transcript untouched)
//!        -> provider messages derived by folding: span replaced by summary
//! ```
//!
//! and the two refuse paths: no generator installed, or the summary call
//! failing — both leave the session exactly as it was and send the full
//! transcript.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, RequestId, SessionId, Timestamp, TraceId, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_orchestration::compaction_checkpoint::{
    SummaryError, SummaryGenerator, SummaryRequest, SummarySections,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedMessage,
    NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{
    InMemorySessionStore, Runtime, Session, SessionEventKind, SessionStore, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-compaction";
const IDENTITY: &str = "system identity safety preamble";
const TURN_ONE: &str = "TURN-ONE-SENTINEL";
const TURN_TWO: &str = "TURN-TWO-SENTINEL";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A provider that records every request it received and answers `ok`.
struct Recorder {
    id: CapabilityId,
    context_window: u32,
    requests: Mutex<Vec<NormalizedRequest>>,
}

impl Recorder {
    fn new(context_window: u32) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            context_window,
            requests: Mutex::new(Vec::new()),
        })
    }

    fn request_text(&self, index: usize) -> String {
        self.requests.lock().unwrap()[index]
            .messages
            .iter()
            .map(|message| ContentPart::join_text(&message.content))
            .collect::<Vec<_>>()
            .join("\n---\n")
    }
}

#[async_trait]
impl ProviderCapability for Recorder {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls)
                .with_context_window(self.context_window),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(NormalizedResponse {
            id: "response-0".into(),
            model: request.model.clone(),
            content: "ok".into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<Recorder>,
}

impl ProviderPlugin {
    fn new(provider: Arc<Recorder>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.fake_provider").unwrap(),
                "1.0.0",
                "fake provider",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "fake provider",
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

/// A scripted summary generator: counts calls and answers a fixed result (or a
/// fixed failure).
struct ScriptedSummary {
    calls: AtomicUsize,
    answer: Result<String, SummaryError>,
}

impl ScriptedSummary {
    fn answers(text: String) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            answer: Ok(text),
        })
    }

    fn fails(error: SummaryError) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            answer: Err(error),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl SummaryGenerator for ScriptedSummary {
    async fn generate(&self, _request: &SummaryRequest) -> Result<String, SummaryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.answer.clone()
    }
}

fn small_summary(keys: &str) -> String {
    SummarySections {
        keys: keys.into(),
        ..SummarySections::default()
    }
    .render()
}

fn big_turn_input(sentinel: &str) -> String {
    format!("{sentinel}-{}", "X".repeat(4_000))
}

async fn runtime_with(
    provider: Arc<Recorder>,
    generator: Option<Arc<dyn SummaryGenerator>>,
    store: Arc<InMemorySessionStore>,
) -> Runtime {
    let mut builder = Runtime::builder()
        .with_default_model(MODEL)
        .with_session_store(store)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider));
    if let Some(generator) = generator {
        builder = builder.with_summary_generator(generator);
    }
    builder.build().await.unwrap()
}

/// Run the two-turn fixture: turn one plants a long past, turn two arrives
/// over the trigger and is the turn whose provider request gets folded.
async fn run_two_big_turns(runtime: &Runtime, session: SessionId) {
    runtime
        .execute(TurnRequest::new(session, big_turn_input(TURN_ONE)).with_system(IDENTITY))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, big_turn_input(TURN_TWO)))
        .await
        .unwrap();
}

/// The compaction entries of one stored session, flattened to `kind:marker`.
fn compaction_events(stored: &Session) -> Vec<String> {
    stored
        .events
        .iter()
        .filter_map(|event| match &event.event {
            SessionEventKind::CompactionStarted { marker } => Some(format!("start:{marker}")),
            SessionEventKind::CompactionCheckpoint { marker, .. } => {
                Some(format!("checkpoint:{marker}"))
            }
            SessionEventKind::CompactionClosed { marker } => Some(format!("end:{marker}")),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 超窗触发接入 + 替换
// ---------------------------------------------------------------------------

/// Over the trigger with a summary generator installed: exactly one summary
/// call runs, the checkpoint lands under a closed marker pair, and the provider
/// request sees the folded view — the compressed span replaced by the summary,
/// the retained tail verbatim, the transcript untouched.
#[tokio::test]
async fn over_trigger_compacts_and_the_provider_sees_the_folded_view() {
    let provider = Recorder::new(1_000);
    let store = Arc::new(InMemorySessionStore::new());
    let generator = ScriptedSummary::answers(small_summary("THE-KEY-FACT"));
    let generator_handle: Arc<dyn SummaryGenerator> = generator.clone();
    let runtime = runtime_with(
        Arc::clone(&provider),
        Some(generator_handle),
        Arc::clone(&store),
    )
    .await;
    let session = SessionId::new();

    run_two_big_turns(&runtime, session).await;

    assert_eq!(generator.call_count(), 1, "一次压缩只调一次摘要");
    let folded_request = provider.request_text(1);
    assert!(
        folded_request.contains("## 意图"),
        "派生视图必须带摘要: {folded_request}"
    );
    assert!(folded_request.contains("THE-KEY-FACT"));
    assert!(
        !folded_request.contains(TURN_ONE),
        "被压缩区间不得再出现在派生视图里"
    );
    assert!(folded_request.contains(TURN_TWO), "尾部逐字保留");

    // The marker pair closes around exactly one checkpoint.
    let stored = store.load(&session).await.unwrap().unwrap();
    let events = compaction_events(&stored);
    assert_eq!(events.len(), 3, "start + checkpoint + end: {events:?}");
    for (event, kind) in events.iter().zip(["start", "checkpoint", "end"]) {
        assert!(
            event.starts_with(&format!("{kind}:")),
            "事件顺序必须是 start -> checkpoint -> end: {events:?}"
        );
    }
    let opened = events[0].split_once(':').map(|(_, marker)| marker);
    let closed = events[2].split_once(':').map(|(_, marker)| marker);
    assert_eq!(opened, closed, "标记对必须同 marker");

    // 留档: the transcript itself still holds every original message
    // (system, turn-1 input, turn-1 answer, turn-2 input, turn-2 answer).
    assert_eq!(stored.messages.len(), 5, "原消息一条不少");
    assert!(
        ContentPart::join_text(&stored.messages[1].content).contains(TURN_ONE),
        "留档原文可回放"
    );
    assert_eq!(
        stored.provider_view(),
        store.load(&session).await.unwrap().unwrap().provider_view(),
        "同一事件流重放出同一视图"
    );
}

/// The summary call failing (provider unavailable or bad output) changes
/// nothing: the provider still sees the full transcript and the session has no
/// compaction events at all.
#[tokio::test]
async fn summary_failure_leaves_the_session_untouched() {
    let provider = Recorder::new(1_000);
    let store = Arc::new(InMemorySessionStore::new());
    let generator = ScriptedSummary::fails(SummaryError::Unavailable("offline".into()));
    let generator_handle: Arc<dyn SummaryGenerator> = generator.clone();
    let runtime = runtime_with(
        Arc::clone(&provider),
        Some(generator_handle),
        Arc::clone(&store),
    )
    .await;
    let session = SessionId::new();

    run_two_big_turns(&runtime, session).await;

    assert_eq!(generator.call_count(), 1);
    let request = provider.request_text(1);
    assert!(request.contains(TURN_ONE), "失败必须不压缩: {request}");
    assert!(request.contains(TURN_TWO));

    let stored = store.load(&session).await.unwrap().unwrap();
    assert_eq!(stored.messages.len(), 5);
    assert!(
        compaction_events(&stored).is_empty(),
        "失败不得留下任何 checkpoint 事件"
    );
}

/// Under the trigger nothing is compacted and the summary generator is never
/// called: normal sessions see zero impact.
#[tokio::test]
async fn under_trigger_turns_are_untouched() {
    let provider = Recorder::new(1_000);
    let store = Arc::new(InMemorySessionStore::new());
    let generator = ScriptedSummary::answers(small_summary("irrelevant"));
    let generator_handle: Arc<dyn SummaryGenerator> = generator.clone();
    let runtime = runtime_with(
        Arc::clone(&provider),
        Some(generator_handle),
        Arc::clone(&store),
    )
    .await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "hi").with_system(IDENTITY))
        .await
        .unwrap();

    assert_eq!(generator.call_count(), 0, "未超窗不得调摘要");
    assert!(provider.request_text(0).contains("hi"));
    let stored = store.load(&session).await.unwrap().unwrap();
    assert!(compaction_events(&stored).is_empty());
}

/// No summary generator installed: the derived view is the transcript itself,
/// whatever its size.
#[tokio::test]
async fn a_session_without_a_summary_generator_is_never_compacted() {
    let provider = Recorder::new(1_000);
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime_with(Arc::clone(&provider), None, Arc::clone(&store)).await;
    let session = SessionId::new();

    run_two_big_turns(&runtime, session).await;

    let request = provider.request_text(1);
    assert!(request.contains(TURN_ONE), "无摘要器 = 原文直发");
    assert!(request.contains(TURN_TWO));
    let stored = store.load(&session).await.unwrap().unwrap();
    assert!(compaction_events(&stored).is_empty());
}

// ---------------------------------------------------------------------------
// checkpoint 落盘往返
// ---------------------------------------------------------------------------

/// Checkpoint events survive a session-store round trip and a JSON round trip
/// (the exact shape a durable store persists), and replaying the fold over the
/// reloaded session yields the same view.
#[tokio::test]
async fn checkpoint_events_round_trip_through_the_session_store() {
    let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ));
    let store = InMemorySessionStore::new();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    for text in ["first", "second", "third"] {
        session.append(NormalizedMessage::user(text), clock.as_ref());
    }
    let (request, trace) = (RequestId::new(), TraceId::new());
    let marker = "compaction/test-1".to_string();
    session.record(
        request,
        trace,
        SessionEventKind::CompactionStarted {
            marker: marker.clone(),
        },
        clock.as_ref(),
    );
    session.record(
        request,
        trace,
        SessionEventKind::CompactionCheckpoint {
            start_seq: 0,
            end_seq: 2,
            summary: small_summary("kept"),
            marker: marker.clone(),
        },
        clock.as_ref(),
    );
    session.record(
        request,
        trace,
        SessionEventKind::CompactionClosed {
            marker: marker.clone(),
        },
        clock.as_ref(),
    );
    store.save(&session).await.unwrap();

    let reloaded = store.load(&session.id).await.unwrap().unwrap();
    assert_eq!(
        compaction_events(&reloaded),
        vec![
            "start:compaction/test-1".to_string(),
            "checkpoint:compaction/test-1".to_string(),
            "end:compaction/test-1".to_string(),
        ]
    );

    // A durable store persists the session as JSON; round trip that shape.
    let json = serde_json::to_string(&reloaded).unwrap();
    let from_disk: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(from_disk.messages, reloaded.messages);
    assert_eq!(from_disk.events, reloaded.events);
    assert_eq!(
        from_disk.provider_view(),
        reloaded.provider_view(),
        "重放同一事件流必须得到同一视图"
    );
    assert_eq!(
        from_disk.provider_view().len(),
        2,
        "摘要 + 尾部: [summary, third]"
    );
}
