//! Deterministic end-to-end proof of the canonical agent loop.
//!
//! Nothing here is mocked at the seam under test. The runtime, plugin manager,
//! capability registry, governance pipeline, provider router, session store, and
//! agent loop are all the real implementations. Only the two *edges* are
//! substituted: a scripted provider instead of a network call, and a calculator
//! instead of a real tool. That is what makes the test both honest and
//! deterministic — there is no sleep, no clock read, and no network anywhere.
//!
//! The chain being proved:
//!
//! ```text
//!   "calculate 1+1"
//!        -> provider round 1  -> ToolCall(calculator, {a:1,b:1})
//!        -> governance        -> allow
//!        -> capability lookup -> tool.calculator
//!        -> plugin dispatch   -> ToolResult("2")
//!        -> provider round 2  -> "The answer is 2."
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{
    AllowAll, Decision, DenyCapabilities, GovernanceHook, GovernancePipeline, GovernanceRequest,
    MaxRounds,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    ExecutionTrace, InMemorySessionStore, Runtime, RuntimeError, SessionEventKind, TraceEvent,
    TurnRequest,
};
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
    /// Fail.
    Fail(ProviderError),
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

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
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
            Scripted::Fail(e) => Err(e.clone()),
        }
    }
}

/// The calculator tool.
struct Calculator {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for Calculator {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "calculator".into(),
            description: Some("Add two integers".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        let (Some(a), Some(b)) = (
            call.arguments.get("a").and_then(serde_json::Value::as_i64),
            call.arguments.get("b").and_then(serde_json::Value::as_i64),
        ) else {
            return ToolResult::permanent_error(&call.id, "expected integer fields a and b");
        };
        ToolResult::ok(&call.id, serde_json::json!((a + b).to_string()))
    }
}

/// A plugin providing the calculator.
struct CalculatorPlugin {
    manifest: PluginManifest,
    invocations: Arc<AtomicUsize>,
}

impl CalculatorPlugin {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.calculator").unwrap(),
                "1.0.0",
                "Arithmetic",
            )
            .declare_capability(
                CapabilityId::new("tool.calculator").unwrap(),
                CapabilityKind::Tool,
                "Add two integers",
            )
            .unwrap(),
            invocations: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn invocation_count(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Plugin for CalculatorPlugin {
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
        vec![Arc::new(Calculator {
            id: CapabilityId::new("tool.calculator").unwrap(),
            invocations: Arc::clone(&self.invocations),
        })]
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
                "Fake vendor",
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

/// A clock that never advances, so every timestamp is identical and the test
/// cannot depend on wall time.
fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

/// The script from the specification: ask for the calculator, then answer.
fn calculator_script() -> Vec<Scripted> {
    vec![
        Scripted::CallTool {
            call_id: "call_1",
            tool: "calculator",
            arguments: serde_json::json!({ "a": 1, "b": 1 }),
        },
        Scripted::Say("The answer is 2."),
    ]
}

/// Extract a message's text, whatever content parts it holds.
fn message_text(message: &apeireth_protocol::canonical::NormalizedMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(text.as_str()),
            ContentPart::ImageUrl { .. } => None,
        })
        .collect()
}

struct DenyEveryCompletion;

#[async_trait]
impl GovernanceHook for DenyEveryCompletion {
    fn name(&self) -> &str {
        "governance.input.safety"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match &request.action {
            apeireth_governance::Action::Completion { .. } => {
                Decision::deny("input blocked by deterministic policy")
            }
            _ => Decision::Allow,
        }
    }
}

struct ApproveEveryCompletion;

#[async_trait]
impl GovernanceHook for ApproveEveryCompletion {
    fn name(&self) -> &str {
        "governance.input.approval"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match &request.action {
            apeireth_governance::Action::Completion { .. } => {
                Decision::require_approval("a human must approve this input")
            }
            _ => Decision::Allow,
        }
    }
}

// ---------------------------------------------------------------------------
// The specified end-to-end case
// ---------------------------------------------------------------------------

#[tokio::test]
async fn minimal_tool_call_agent_loop_closes_end_to_end() {
    let provider = FakeProvider::new("provider.fake", calculator_script());
    let calculator = CalculatorPlugin::new();
    let store = Arc::new(InMemorySessionStore::new());

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store.clone())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(calculator.clone())
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");

    let session_id = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session_id, "calculate 1+1"))
        .await
        .expect("the turn completes");

    // --- the six required assertions ------------------------------------

    // 1. the provider was invoked twice
    assert_eq!(
        provider.call_count(),
        2,
        "the loop must go back to the provider after the tool runs"
    );
    assert_eq!(outcome.trace.provider_invocations(), 2);
    assert_eq!(outcome.rounds, 2);

    // 2. the tool was invoked exactly once
    assert_eq!(calculator.invocation_count(), 1);
    assert_eq!(outcome.trace.capability_dispatches(), 1);
    assert_eq!(
        outcome
            .trace
            .dispatches_of(&CapabilityId::new("tool.calculator").unwrap()),
        1
    );

    // 3. the tool result was actually handed back to the provider
    let second_request = provider.request(1);
    let tool_messages: Vec<_> = second_request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .collect();
    assert_eq!(tool_messages.len(), 1, "round 2 must carry the tool result");
    assert_eq!(
        tool_messages[0].tool_call_id.as_deref(),
        Some("call_1"),
        "the result must stay correlated to the call that asked for it"
    );
    assert_eq!(
        message_text(tool_messages[0]),
        "2",
        "the provider must see the computed value"
    );

    // The assistant's tool-call message must precede the result, or the
    // provider is answering a question it never asked.
    let assistant_index = second_request
        .messages
        .iter()
        .position(|m| !m.tool_calls.is_empty())
        .expect("round 2 must carry the assistant's tool-call message");
    let tool_index = second_request
        .messages
        .iter()
        .position(|m| m.role == MessageRole::Tool)
        .unwrap();
    assert!(
        assistant_index < tool_index,
        "the call must precede its result in the transcript"
    );

    // 4. the same session was used throughout
    assert_eq!(outcome.session, session_id);
    assert_eq!(store.len().await, 1, "no second session was created");
    let persisted = runtime
        .sessions()
        .load_or_create(session_id)
        .await
        .expect("session persisted");
    // user, assistant(tool_calls), tool result, assistant(final)
    assert_eq!(persisted.len(), 4);
    assert_eq!(persisted.messages[0].role, MessageRole::User);
    assert_eq!(persisted.messages[1].role, MessageRole::Assistant);
    assert_eq!(persisted.messages[2].role, MessageRole::Tool);
    assert_eq!(persisted.messages[3].role, MessageRole::Assistant);
    assert_eq!(message_text(&persisted.messages[3]), "The answer is 2.");

    // 5. one trace covers the whole turn
    let trace_id = outcome.trace.trace;
    assert_eq!(outcome.trace.session, session_id);
    assert_eq!(outcome.trace.request, outcome.request);
    assert_eq!(
        outcome.trace.completed_rounds(),
        Some(2),
        "the trace must record that the turn closed"
    );
    // Every event belongs to this turn; the trace is the correlation unit.
    assert!(!outcome.trace.entries.is_empty());
    assert_eq!(outcome.trace.trace, trace_id);

    // 6. the final response is correct
    assert_eq!(outcome.text, "The answer is 2.");
    assert_eq!(outcome.served_by.as_str(), "provider.fake");
    assert_eq!(outcome.usage.total_tokens, 15);
}

#[tokio::test]
async fn the_trace_orders_the_chain_exactly_as_specified() {
    let provider = FakeProvider::new("provider.fake", calculator_script());
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider))
        .with_plugin(CalculatorPlugin::new())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "calculate 1+1"))
        .await
        .unwrap();

    let shape: Vec<String> = outcome
        .trace
        .events()
        .map(|e| match e {
            TraceEvent::ProviderInvoked { round, .. } => format!("provider_invoked:{round}"),
            TraceEvent::ProviderSucceeded { round, .. } => format!("provider_succeeded:{round}"),
            TraceEvent::ProviderFailed { round, .. } => format!("provider_failed:{round}"),
            TraceEvent::GovernanceEvaluated { action, round, .. } => {
                format!("governance:{action}:{round}")
            }
            TraceEvent::CapabilityDispatched { round, .. } => format!("dispatched:{round}"),
            TraceEvent::CapabilityCompleted { round, .. } => format!("completed:{round}"),
            TraceEvent::CapabilityUnavailable { round, .. } => format!("unavailable:{round}"),
            TraceEvent::TurnCompleted { rounds } => format!("turn_completed:{rounds}"),
            // `TraceEvent` is non_exhaustive; a new variant appearing in this
            // chain should fail loudly rather than be silently dropped.
            other => panic!("unhandled trace event: {other:?}"),
        })
        .collect();

    assert_eq!(
        shape,
        vec![
            "governance:completion:1",
            "provider_invoked:1",
            "provider_succeeded:1",
            "governance:capability_dispatch:1",
            "dispatched:1",
            "completed:1",
            "governance:completion:2",
            "provider_invoked:2",
            "provider_succeeded:2",
            "turn_completed:2",
        ],
        "the loop must run governance -> provider -> governance -> dispatch -> provider"
    );
}

#[tokio::test]
async fn the_turn_output_carries_no_raw_reasoning() {
    let provider = FakeProvider::new("provider.fake", calculator_script());
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider))
        .with_plugin(CalculatorPlugin::new())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "calculate 1+1"))
        .await
        .unwrap();

    // The debugging contract is the structured trace, and it must be
    // serializable without carrying model-authored reasoning text.
    let json = serde_json::to_string(&outcome.trace).unwrap();
    for forbidden in ["reasoning", "chain_of_thought", "cot", "thinking"] {
        assert!(!json.contains(forbidden), "found {forbidden:?} in {json}");
    }
    let _: ExecutionTrace = serde_json::from_str(&json).unwrap();
}

// ---------------------------------------------------------------------------
// The loop's failure behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_governance_denial_stops_the_tool_but_not_the_turn() {
    let provider = FakeProvider::new("provider.fake", calculator_script());
    let calculator = CalculatorPlugin::new();

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(GovernancePipeline::new().with(Arc::new(
            DenyCapabilities::new().deny(CapabilityId::new("tool.calculator").unwrap()),
        ))))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(calculator.clone())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "calculate 1+1"))
        .await
        .expect("a refused tool must not fail the turn");

    assert_eq!(
        calculator.invocation_count(),
        0,
        "a denied capability must never reach its plugin"
    );
    assert_eq!(outcome.trace.capability_dispatches(), 0);
    assert_eq!(provider.call_count(), 2, "the turn continues to round 2");
    let decision = outcome
        .trace
        .events()
        .find(|event| {
            matches!(
                event,
                TraceEvent::GovernanceEvaluated { action, .. }
                    if action == "capability_dispatch"
            )
        })
        .expect("tool governance decision is traced");
    assert!(matches!(
        decision,
        TraceEvent::GovernanceEvaluated {
            hook,
            owner: None,
            decision,
            reason: Some(reason),
            round: 1,
            ..
        } if hook == "deny_capabilities"
            && decision == "deny"
            && reason.contains("tool.calculator")
    ));

    // The model is told why, so it can try something else.
    let second_request = provider.request(1);
    let refusal = second_request
        .messages
        .iter()
        .find(|m| m.role == MessageRole::Tool)
        .expect("the refusal is reported as a tool result");
    let text = message_text(refusal);
    assert!(text.contains("refused by governance"), "{text}");
    assert!(text.contains("tool.calculator"), "{text}");
}

#[tokio::test]
async fn a_hallucinated_tool_name_is_reported_to_the_model() {
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            Scripted::CallTool {
                call_id: "call_1",
                tool: "nonexistent_tool",
                arguments: serde_json::json!({}),
            },
            Scripted::Say("Sorry, I cannot do that."),
        ],
    );
    let calculator = CalculatorPlugin::new();

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(calculator.clone())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let outcome = runtime
        .execute(TurnRequest::new(
            SessionId::new(),
            "do something impossible",
        ))
        .await
        .expect("an unknown tool must not fail the turn");

    assert_eq!(calculator.invocation_count(), 0);
    assert_eq!(outcome.text, "Sorry, I cannot do that.");
    assert!(outcome.trace.events().any(|e| matches!(
        e,
        TraceEvent::CapabilityUnavailable { requested, .. } if requested == "nonexistent_tool"
    )));

    let reported = message_text(
        provider
            .request(1)
            .messages
            .iter()
            .find(|m| m.role == MessageRole::Tool)
            .expect("the miss is reported as a tool result"),
    );
    assert!(reported.contains("nonexistent_tool"), "{reported}");
    assert!(
        reported.contains("calculator"),
        "the model should be told what it *can* call: {reported}"
    );
}

#[tokio::test]
async fn a_tool_that_fails_keeps_the_turn_alive() {
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            Scripted::CallTool {
                call_id: "call_1",
                tool: "calculator",
                arguments: serde_json::json!({ "a": "not a number" }),
            },
            Scripted::Say("That input was not valid."),
        ],
    );
    let calculator = CalculatorPlugin::new();
    let store = Arc::new(InMemorySessionStore::new());

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(calculator.clone())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let session_id = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session_id, "calculate nonsense"))
        .await
        .unwrap();

    assert_eq!(calculator.invocation_count(), 1, "the tool did run");
    assert!(outcome.trace.events().any(|e| matches!(
        e,
        TraceEvent::CapabilityCompleted {
            succeeded: false,
            ..
        }
    )));
    assert_eq!(outcome.text, "That input was not valid.");

    let persisted = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert!(persisted.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::ToolFailed {
            capability: Some(capability),
            tool_call_id,
            error,
            round: 1,
        } if capability.as_str() == "tool.calculator"
            && tool_call_id == "call_1"
            && error.contains("integer fields")
    )));
}

#[tokio::test]
async fn a_completion_denial_persists_the_attempt_and_decision() {
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("must not run")]);
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(store)
        .with_governance(Arc::new(DenyEveryCompletion))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();
    let session_id = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session_id, "blocked input"))
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::Denied { .. }));
    assert_eq!(provider.call_count(), 0);

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(session.messages.len(), 1);
    assert_eq!(message_text(&session.messages[0]), "blocked input");
    assert!(session.revision >= 3);
    let denial = session
        .events
        .iter()
        .find(|event| matches!(&event.event, SessionEventKind::GovernanceDenied { .. }))
        .expect("denial is durable");
    match &denial.event {
        SessionEventKind::GovernanceDenied {
            hook,
            action,
            reason,
            round,
        } => {
            assert_eq!(hook, "governance.input.safety");
            assert_eq!(action, "completion");
            assert!(reason.contains("blocked"));
            assert_eq!(*round, 1);
        }
        other => panic!("expected governance denial, got {other:?}"),
    }
    assert_eq!(session.events[0].trace, denial.trace);
}

#[tokio::test]
async fn an_approval_requirement_persists_the_attempt_and_pause() {
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("must not run")]);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(ApproveEveryCompletion))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();
    let session_id = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session_id, "approval input"))
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::ApprovalRequired { .. }));
    assert_eq!(provider.call_count(), 0);

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(message_text(&session.messages[0]), "approval input");
    assert!(session.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::CompletionApprovalRequired {
            hook,
            action,
            reason,
            round: 1,
        } if hook == "governance.input.approval"
            && action == "completion"
            && reason.contains("human")
    )));
    assert!(
        session
            .events
            .iter()
            .all(|event| !matches!(&event.event, SessionEventKind::ApprovalRequired { .. })),
        "completion approval must not mint a ghost capability approval id"
    );
}

#[tokio::test]
async fn a_turn_that_never_converges_hits_the_round_limit() {
    // Always asks for the tool, never answers.
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            Scripted::CallTool {
                call_id: "call_1",
                tool: "calculator",
                arguments: serde_json::json!({ "a": 1, "b": 1 }),
            };
            4
        ],
    );
    let calculator = CalculatorPlugin::new();

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(calculator.clone())
        .with_default_model(MODEL)
        .with_max_rounds(3)
        .build()
        .await
        .unwrap();

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "loop forever"))
        .await
        .unwrap_err();

    match err {
        RuntimeError::RoundLimitExceeded { limit } => assert_eq!(limit, 3),
        other => panic!("expected RoundLimitExceeded, got {other}"),
    }
    assert_eq!(
        provider.call_count(),
        3,
        "the limit is enforced, not exceeded"
    );
    assert_eq!(calculator.invocation_count(), 3);
}

#[tokio::test]
async fn a_governance_hook_can_bound_the_loop_before_the_structural_limit() {
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            Scripted::CallTool {
                call_id: "call_1",
                tool: "calculator",
                arguments: serde_json::json!({ "a": 1, "b": 1 }),
            };
            8
        ],
    );

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(MaxRounds::new(2))),
        ))
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(CalculatorPlugin::new())
        .with_default_model(MODEL)
        .with_max_rounds(8)
        .build()
        .await
        .unwrap();

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "loop forever"))
        .await
        .unwrap_err();

    match err {
        RuntimeError::Denied { hook, reason } => {
            assert_eq!(hook, "max_rounds");
            assert!(reason.contains('2'), "{reason}");
        }
        other => panic!("expected Denied, got {other}"),
    }
    assert_eq!(
        provider.call_count(),
        2,
        "governance stopped the turn before the structural limit"
    );
}

// ---------------------------------------------------------------------------
// Routing and session continuity through the loop
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_loop_falls_back_to_a_second_provider_and_records_why() {
    let flaky = FakeProvider::new(
        "provider.flaky",
        vec![Scripted::Fail(ProviderError::RateLimited {
            provider: "provider.flaky".into(),
            retry_after_ms: 50,
        })],
    );
    let healthy = FakeProvider::new(
        "provider.healthy",
        vec![Scripted::Say("Served by the backup.")],
    );

    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_plugin(ProviderPlugin::new("vendor.flaky", flaky.clone()))
        .with_plugin(ProviderPlugin::new("vendor.healthy", healthy.clone()))
        .with_fallback_order(vec![
            CapabilityId::new("provider.flaky").unwrap(),
            CapabilityId::new("provider.healthy").unwrap(),
        ])
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();

    assert_eq!(outcome.text, "Served by the backup.");
    assert_eq!(outcome.served_by.as_str(), "provider.healthy");
    assert_eq!(flaky.call_count(), 1);
    assert_eq!(healthy.call_count(), 1);

    let failure = outcome
        .trace
        .events()
        .find_map(|e| match e {
            TraceEvent::ProviderFailed {
                provider,
                retryable,
                ..
            } => Some((provider.clone(), *retryable)),
            _ => None,
        })
        .expect("the skipped provider must appear in the trace");
    assert_eq!(failure.0.as_str(), "provider.flaky");
    assert!(failure.1, "the fallback happened because it was retryable");
}

#[tokio::test]
async fn consecutive_turns_share_one_session_transcript() {
    let provider = FakeProvider::new(
        "provider.fake",
        vec![Scripted::Say("first"), Scripted::Say("second")],
    );
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let session_id = SessionId::new();
    let first = runtime
        .execute(TurnRequest::new(session_id, "one").with_system("You are terse."))
        .await
        .unwrap();
    let second = runtime
        .execute(TurnRequest::new(session_id, "two").with_system("You are terse."))
        .await
        .unwrap();

    assert_eq!(first.session, second.session);
    assert_ne!(
        first.trace.trace, second.trace.trace,
        "each turn gets its own trace"
    );

    // The second request sees the first exchange.
    let second_request = provider.request(1);
    assert_eq!(
        second_request
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::System)
            .count(),
        1,
        "a system prompt must be seeded once, not re-added on every turn"
    );
    let texts: Vec<String> = second_request.messages.iter().map(message_text).collect();
    assert_eq!(texts, ["You are terse.", "one", "first", "two"]);
}

#[tokio::test]
async fn the_model_is_offered_exactly_the_active_tools() {
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("ok")]);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
        .with_plugin(CalculatorPlugin::new())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();

    let first_request = provider.request(0);
    let names: Vec<&str> = first_request
        .tools
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["calculator"]);
}

#[tokio::test]
async fn a_runtime_with_no_provider_fails_with_a_legible_error() {
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_plugin(CalculatorPlugin::new())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap_err();

    match err {
        RuntimeError::NoProvider { model, available } => {
            assert_eq!(model, MODEL);
            assert_eq!(available, "none");
        }
        other => panic!("expected NoProvider, got {other}"),
    }
}

#[tokio::test]
async fn a_failed_turn_still_preserves_the_users_message() {
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();

    let session_id = SessionId::new();
    assert!(runtime
        .execute(TurnRequest::new(session_id, "hello"))
        .await
        .is_err());

    let session = runtime.sessions().load_or_create(session_id).await.unwrap();
    assert_eq!(
        session.len(),
        1,
        "a retry should continue the conversation, not lose the user's turn"
    );
    assert_eq!(message_text(&session.messages[0]), "hello");
    let failure = session
        .events
        .iter()
        .find(|event| matches!(&event.event, SessionEventKind::ProviderFailed { .. }))
        .expect("provider failure is durable");
    assert!(matches!(
        &failure.event,
        SessionEventKind::ProviderFailed { error, round: 1 }
            if error.contains("no provider")
    ));
    assert_eq!(session.events[0].trace, failure.trace);
}

#[tokio::test]
async fn a_turn_without_a_model_is_rejected_rather_than_guessing() {
    let provider = FakeProvider::new("provider.fake", vec![Scripted::Say("ok")]);
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_plugin(ProviderPlugin::new("vendor.fake", provider))
        .build()
        .await
        .unwrap();

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap_err();
    assert!(matches!(err, RuntimeError::Misconfigured(_)), "{err}");
}

#[tokio::test]
async fn the_whole_turn_is_reproducible() {
    // Same inputs, same frozen clock, twice: everything except the generated
    // ids must be identical. If the loop ever grows a dependency on wall time
    // or iteration order, this stops holding.
    async fn run() -> (String, Vec<String>, usize) {
        let provider = FakeProvider::new("provider.fake", calculator_script());
        let runtime = Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new("vendor.fake", provider.clone()))
            .with_plugin(CalculatorPlugin::new())
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap();

        let outcome = runtime
            .execute(TurnRequest::new(SessionId::new(), "calculate 1+1"))
            .await
            .unwrap();

        let shape: Vec<String> = outcome
            .trace
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{}:{:?}",
                    e.at.epoch_millis(),
                    std::mem::discriminant(&e.event)
                )
            })
            .collect();
        (outcome.text, shape, provider.call_count())
    }

    let a = run().await;
    let b = run().await;
    assert_eq!(a, b, "the turn must be deterministic");
}
