//! End-to-end proof of the repeated-identical-call advisory channel.
//!
//! The runtime, plugin manager, capability registry, governance pipeline,
//! provider router, session store, and agent loop are the real
//! implementations; only the two edges are substituted: a scripted provider
//! instead of a network call and a counting tool instead of a real one.
//!
//! What is proved here:
//! - the advisory is pure advice: every repeated call still executes;
//! - the reminder rides along as additional context on the tool-result
//!   message, after the untouched result rendering;
//! - streaks are consecutive and cleared by a user message;
//! - the exclusion list is transparent to counting and clearing.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_orchestration::repetition_advisory::{RepetitionPolicy, RESULT_CONTEXT_MARKER};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, ModelFeature, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, TurnRequest};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Scripted {
    CallTool {
        call_id: &'static str,
        tool: &'static str,
        arguments: serde_json::Value,
    },
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

/// A tool that records every invocation and answers with one stable payload.
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

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.into(),
            description: Some("counting tool".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({"ok": true}))
    }
}

struct ToolPlugin {
    manifest: PluginManifest,
    tools: Vec<Arc<CountingTool>>,
    invocations: Arc<AtomicUsize>,
}

impl ToolPlugin {
    /// Register one plugin exposing one counting tool per name.
    fn new(plugin_id: &str, tool_names: &[&'static str]) -> (Arc<Self>, Arc<AtomicUsize>) {
        let invocations = Arc::new(AtomicUsize::new(0));
        let mut manifest = PluginManifest::new(
            PluginId::new(plugin_id).unwrap(),
            "1.0.0",
            "counting tool provider",
        );
        let mut tools = Vec::new();
        for (index, name) in tool_names.iter().enumerate() {
            let id = CapabilityId::new(format!("tool.counting{index}")).unwrap();
            manifest = manifest
                .declare_capability(id.clone(), CapabilityKind::Tool, "counting tool")
                .unwrap();
            tools.push(Arc::new(CountingTool {
                id,
                name,
                invocations: Arc::clone(&invocations),
            }));
        }
        let plugin = Arc::new(Self {
            manifest,
            tools,
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
        self.tools
            .iter()
            .map(|tool| Arc::clone(tool) as Arc<dyn ToolCapability>)
            .collect()
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

/// The transported tool-result messages of one session, in order.
async fn tool_result_texts(runtime: &Runtime, session: SessionId) -> Vec<String> {
    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session is persisted");
    stored
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .map(message_text)
        .collect()
}

fn identical_call(call_id: &'static str) -> Scripted {
    Scripted::CallTool {
        call_id,
        tool: "counting",
        arguments: serde_json::json!({ "query": "same", "page": 1 }),
    }
}

async fn runtime_with(
    script: Vec<Scripted>,
    policy: RepetitionPolicy,
) -> (Runtime, Arc<AtomicUsize>) {
    let provider = FakeProvider::new("provider.fake", script);
    let (tool_plugin, invocations) = ToolPlugin::new("plugin.tool", &["counting", "clock"]);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("plugin.provider", provider))
        .with_plugin(tool_plugin)
        .with_repetition_policy(policy)
        .with_default_model(MODEL)
        .with_max_rounds(32)
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .build()
        .await
        .unwrap();
    (runtime, invocations)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn repeated_calls_execute_fully_and_carry_advisory_context() {
    let script = vec![
        identical_call("call_1"),
        identical_call("call_2"),
        identical_call("call_3"),
        identical_call("call_4"),
        identical_call("call_5"),
        Scripted::Say("done"),
    ];
    let (runtime, invocations) = runtime_with(script, RepetitionPolicy::default()).await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "loop"))
        .await
        .unwrap();

    assert_eq!(
        invocations.load(Ordering::SeqCst),
        5,
        "the advisory channel must never intercept execution"
    );

    let texts = tool_result_texts(&runtime, session).await;
    assert_eq!(texts.len(), 5);
    for (index, text) in texts.iter().enumerate() {
        let occurrence = index + 1;
        assert!(
            text.starts_with(r#"{"ok":true}"#),
            "the result rendering must stay first and intact: {text}"
        );
        match occurrence {
            3 | 5 => {
                let marker_at = text.find(RESULT_CONTEXT_MARKER).unwrap_or_else(|| {
                    panic!("occurrence {occurrence} must carry the advisory: {text}")
                });
                assert!(
                    marker_at > 0,
                    "the advisory is appended after the result: {text}"
                );
            }
            _ => assert!(
                !text.contains(RESULT_CONTEXT_MARKER),
                "occurrence {occurrence} must stay silent: {text}"
            ),
        }
    }
}

#[tokio::test]
async fn the_largest_threshold_carries_the_detailed_advisory() {
    let script = vec![
        identical_call("call_1"),
        identical_call("call_2"),
        identical_call("call_3"),
        identical_call("call_4"),
        identical_call("call_5"),
        identical_call("call_6"),
        identical_call("call_7"),
        identical_call("call_8"),
        Scripted::Say("done"),
    ];
    let (runtime, invocations) = runtime_with(script, RepetitionPolicy::default()).await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "deep loop"))
        .await
        .unwrap();
    assert_eq!(invocations.load(Ordering::SeqCst), 8);

    let texts = tool_result_texts(&runtime, session).await;
    let detailed = &texts[7];
    assert!(detailed.contains("已尝试次数：8"), "{detailed}");
    assert!(detailed.contains("参数预览"), "{detailed}");
    assert!(
        detailed.contains("换一个角度") && detailed.contains("向用户询问"),
        "the detailed advisory suggests changing angle or asking the user: {detailed}"
    );
    assert!(texts[2].contains("连续重复 3 次"), "{}", texts[2]);
    assert!(texts[4].contains("连续重复 5 次"), "{}", texts[4]);
}

#[tokio::test]
async fn a_user_message_clears_the_streak_across_turns() {
    let script = vec![
        identical_call("call_1"),
        identical_call("call_2"),
        Scripted::Say("first turn"),
        identical_call("call_3"),
        identical_call("call_4"),
        Scripted::Say("second turn"),
    ];
    let (runtime, invocations) = runtime_with(script, RepetitionPolicy::default()).await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "turn one"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "turn two"))
        .await
        .unwrap();

    assert_eq!(invocations.load(Ordering::SeqCst), 4);
    let texts = tool_result_texts(&runtime, session).await;
    assert_eq!(texts.len(), 4);
    for text in &texts {
        assert!(
            !text.contains(RESULT_CONTEXT_MARKER),
            "the second turn's user message must clear the streak: {text}"
        );
    }
}

#[tokio::test]
async fn excluded_tools_neither_count_nor_clear_a_streak() {
    let excluded = Scripted::CallTool {
        call_id: "call_clock",
        tool: "clock",
        arguments: serde_json::json!({ "query": "same", "page": 1 }),
    };
    let script = vec![
        identical_call("call_1"),
        identical_call("call_2"),
        excluded.clone(),
        excluded.clone(),
        identical_call("call_3"),
        Scripted::Say("done"),
    ];
    let policy = RepetitionPolicy::default()
        .with_thresholds(vec![3])
        .with_excluded_tools(vec!["clock".to_string()]);
    let (runtime, invocations) = runtime_with(script, policy).await;
    let session = SessionId::new();

    runtime
        .execute(TurnRequest::new(session, "loop with excluded noise"))
        .await
        .unwrap();

    // 3 identical target calls + 2 excluded calls: everything executed.
    assert_eq!(invocations.load(Ordering::SeqCst), 5);

    let texts = tool_result_texts(&runtime, session).await;
    assert_eq!(texts.len(), 5);
    assert!(
        !texts[0].contains(RESULT_CONTEXT_MARKER)
            && !texts[1].contains(RESULT_CONTEXT_MARKER)
            && !texts[2].contains(RESULT_CONTEXT_MARKER)
            && !texts[3].contains(RESULT_CONTEXT_MARKER),
        "excluded calls stay silent and clear nothing: {texts:?}"
    );
    assert!(
        texts[4].contains(RESULT_CONTEXT_MARKER),
        "the target streak survives the excluded calls and reaches the threshold: {}",
        texts[4]
    );
}
