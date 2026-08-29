//! End-to-end coverage for the owned turn-scoped module invoker handle.
//!
//! Proves, through the real canonical turn machinery, that
//! `ModuleContext::invoker_handle()` hands out an owned handle which:
//! - shares one `ModuleTurnState` with the borrowed `ctx.invoker()` accessor
//!   and with clones of itself inside the same turn;
//! - is isolated across turns (fresh budget and fresh trace per turn);
//! - is isolated across concurrently executing sessions;
//! - still fails closed on governance Deny / RequireApproval with zero
//!   provider calls and no hidden approval;
//! - plugs into the organ `LlmFactory` bridge without being retained as
//!   persistent module state (the probe module below holds no handle).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId, TraceId};
use apeireth_governance::{Decision, GovernanceHook, GovernanceRequest};
use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use apeireth_runtime::canonical::{
    AgentModule, HookPoint, InvokerLlmFactory, ModuleContext, ModuleError, ModuleInvocationError,
    ModuleInvocationRequest, ModuleManifest, ModuleOutcome, Runtime, TurnOutcome, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

// ---------------------------------------------------------------------
// Scripted provider
// ---------------------------------------------------------------------

struct ScriptedProvider {
    id: CapabilityId,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
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
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let content = request
            .messages
            .last()
            .map(|m| apeireth_protocol::canonical::ContentPart::join_text(&m.content))
            .unwrap_or_default();
        Ok(NormalizedResponse {
            id: format!("response-{}", self.call_count()),
            model: request.model.clone(),
            content: format!("echo: {content}"),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<ScriptedProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<ScriptedProvider>) -> Arc<Self> {
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

// ---------------------------------------------------------------------
// Governance hooks
// ---------------------------------------------------------------------

/// Serves a scripted sequence of decisions (Allow past the end) and captures
/// every request's (session, trace) so turn identity can be audited.
struct SequenceHook {
    decisions: Mutex<Vec<Decision>>,
    served: AtomicUsize,
    captures: Mutex<Vec<(SessionId, TraceId)>>,
}

impl SequenceHook {
    fn new(decisions: Vec<Decision>) -> Arc<Self> {
        Arc::new(Self {
            decisions: Mutex::new(decisions),
            served: AtomicUsize::new(0),
            captures: Mutex::new(Vec::new()),
        })
    }

    fn captures(&self) -> Vec<(SessionId, TraceId)> {
        self.captures.lock().unwrap().clone()
    }
}

#[async_trait]
impl GovernanceHook for SequenceHook {
    fn name(&self) -> &str {
        "sequence-hook"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        self.captures
            .lock()
            .unwrap()
            .push((request.session, request.trace));
        let index = self.served.fetch_add(1, Ordering::SeqCst);
        let mut decisions = self.decisions.lock().unwrap();
        match decisions.get(index) {
            Some(decision) => decision.clone(),
            None => Decision::Allow,
        }
    }
}

// ---------------------------------------------------------------------
// Probe module: drives handle scenarios from inside a real hook
// ---------------------------------------------------------------------

/// What the hook should do when it fires.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ProbeStep {
    /// Borrowed accessor call, then owned handle call, then cloned handle
    /// call — with a turn budget of 2 the third must be refused.
    BorrowedHandleClone,
    /// Two handle calls (exhausts a budget of 2).
    ExhaustBudget,
    /// One handle call.
    OneCall,
    /// Route one completion through `InvokerLlmFactory` over the handle.
    BridgeOneCall,
}

struct ProbeModule {
    manifest: ModuleManifest,
    steps: Mutex<Vec<ProbeStep>>,
    /// Result records, one string per attempted side-call.
    records: Mutex<Vec<String>>,
}

impl ProbeModule {
    fn new(steps: Vec<ProbeStep>) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new("probe.invoker_handle", "Invoker handle probe"),
            steps: Mutex::new(steps),
            records: Mutex::new(Vec::new()),
        })
    }

    fn records(&self) -> Vec<String> {
        self.records.lock().unwrap().clone()
    }
}

/// Marker for the isolated side-call request bodies.
fn side_call() -> ModuleInvocationRequest {
    ModuleInvocationRequest::isolated("probe system", "probe input")
}

fn record(
    module: &ProbeModule,
    label: &str,
    result: &Result<apeireth_runtime::canonical::ModuleInvocationResponse, ModuleInvocationError>,
) {
    let line = match result {
        Ok(_) => format!("{label}:ok"),
        Err(ModuleInvocationError::BudgetExceeded { .. }) => format!("{label}:budget_exceeded"),
        Err(ModuleInvocationError::Denied { .. }) => format!("{label}:denied"),
        Err(ModuleInvocationError::ApprovalRequired { .. }) => {
            format!("{label}:approval_required")
        }
        Err(ModuleInvocationError::RecursionLimit { .. }) => format!("{label}:recursion_limit"),
        Err(other) => format!("{label}:error:{other}"),
    };
    module.records.lock().unwrap().push(line);
}

#[async_trait]
impl AgentModule for ProbeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart {
            return Ok(ModuleOutcome::continue_());
        }
        let step = if self.steps.lock().unwrap().is_empty() {
            None
        } else {
            Some(self.steps.lock().unwrap().remove(0))
        };
        match step {
            Some(ProbeStep::BorrowedHandleClone) => {
                let handle = ctx.invoker_handle();
                record(self, "borrowed", &ctx.invoker().invoke(side_call()).await);
                record(self, "handle", &handle.invoke(side_call()).await);
                record(self, "clone", &handle.clone().invoke(side_call()).await);
            }
            Some(ProbeStep::ExhaustBudget) => {
                let handle = ctx.invoker_handle();
                record(self, "one", &handle.invoke(side_call()).await);
                record(self, "two", &handle.invoke(side_call()).await);
            }
            Some(ProbeStep::OneCall) => {
                let handle = ctx.invoker_handle();
                record(self, "solo", &handle.invoke(side_call()).await);
            }
            Some(ProbeStep::BridgeOneCall) => {
                // The owned handle moves into the organ LlmFactory bridge and
                // is dropped when this hook returns. The probe module holds
                // no handle field: nothing here persists into module state.
                let factory: Arc<dyn LlmFactory> =
                    Arc::new(InvokerLlmFactory::new(ctx.invoker_handle()));
                let mut instance = factory
                    .spawn(SubagentRole::Reviewer, MODEL)
                    .await
                    .map_err(|e| ModuleError::Message(e.to_string()))?;
                let reply = instance
                    .complete(apeireth_plugin::llm_factory::CompletionRequest {
                        system_prompt: "bridge system".into(),
                        messages: vec![apeireth_plugin::llm_factory::CompletionMessage {
                            role: "user".into(),
                            content: "bridge input".into(),
                        }],
                        temperature: 0.0,
                        tools: vec![],
                        max_tokens: None,
                    })
                    .await
                    .map_err(|e| ModuleError::Message(e.to_string()))?;
                self.records
                    .lock()
                    .unwrap()
                    .push(format!("bridge:ok:{}", reply.message.content));
            }
            None => {}
        }
        Ok(ModuleOutcome::continue_())
    }
}

async fn build_runtime(
    provider: Arc<ScriptedProvider>,
    governance: Arc<SequenceHook>,
    max_invocations: usize,
    module: Arc<ProbeModule>,
) -> Runtime {
    Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(governance)
        .with_max_module_invocations(max_invocations)
        .with_plugin(ProviderPlugin::new(provider))
        .with_module(module)
        .build()
        .await
        .unwrap()
}

// ---------------------------------------------------------------------
// A. cloned handles in the same turn share one ModuleTurnState
// ---------------------------------------------------------------------

#[tokio::test]
async fn cloned_handles_in_same_turn_share_one_budget() {
    let provider = ScriptedProvider::new();
    let governance = SequenceHook::new(Vec::new());
    let module = ProbeModule::new(vec![ProbeStep::BorrowedHandleClone]);
    let runtime = build_runtime(provider.clone(), governance, 2, module.clone()).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "turn one"))
        .await
        .unwrap();
    assert!(matches!(outcome, TurnOutcome::Completed(_)));

    // The borrowed accessor, the owned handle and its clone all drew from the
    // same budget of 2: the third call is refused, not given a fresh budget.
    assert_eq!(
        module.records(),
        vec![
            "borrowed:ok".to_string(),
            "handle:ok".to_string(),
            "clone:budget_exceeded".to_string(),
        ]
    );
    // Two side-calls plus the turn's own main round reached the provider;
    // the refused third side-call did not.
    assert_eq!(provider.call_count(), 3);
}

// ---------------------------------------------------------------------
// B. different turns do NOT share budget
// ---------------------------------------------------------------------

#[tokio::test]
async fn different_turns_have_independent_budgets() {
    let provider = ScriptedProvider::new();
    let governance = SequenceHook::new(Vec::new());
    let module = ProbeModule::new(vec![ProbeStep::ExhaustBudget, ProbeStep::OneCall]);
    let runtime = build_runtime(provider.clone(), governance, 2, module.clone()).await;

    let session = SessionId::new();
    runtime
        .execute_outcome(TurnRequest::new(session, "turn one"))
        .await
        .unwrap();
    runtime
        .execute_outcome(TurnRequest::new(session, "turn two"))
        .await
        .unwrap();

    // Turn one exhausted its budget of 2; turn two started fresh.
    assert_eq!(
        module.records(),
        vec![
            "one:ok".to_string(),
            "two:ok".to_string(),
            "solo:ok".to_string(),
        ]
    );
}

// ---------------------------------------------------------------------
// C. session/trace context does not cross turns
// ---------------------------------------------------------------------

#[tokio::test]
async fn side_call_context_is_scoped_to_its_own_turn() {
    let provider = ScriptedProvider::new();
    let governance = SequenceHook::new(Vec::new());
    let module = ProbeModule::new(vec![ProbeStep::OneCall, ProbeStep::OneCall]);
    let runtime = build_runtime(provider.clone(), governance.clone(), 8, module.clone()).await;

    let session = SessionId::new();
    runtime
        .execute_outcome(TurnRequest::new(session, "turn one"))
        .await
        .unwrap();
    runtime
        .execute_outcome(TurnRequest::new(session, "turn two"))
        .await
        .unwrap();

    // Four governed completions in order: turn-one side-call, turn-one main
    // round, turn-two side-call, turn-two main round. Side-call requests must
    // carry their own turn's trace, never another turn's.
    let captures = governance.captures();
    assert_eq!(captures.len(), 4, "got {captures:?}");
    assert!(
        captures.iter().all(|(sid, _)| *sid == session),
        "every governed call belongs to the one session"
    );
    let trace_one = captures[0].1;
    let trace_two = captures[2].1;
    assert_eq!(captures[1].1, trace_one, "turn one shares one trace");
    assert_eq!(captures[3].1, trace_two, "turn two shares one trace");
    assert_ne!(
        trace_one, trace_two,
        "a second turn must never inherit the first turn's trace"
    );
}

// ---------------------------------------------------------------------
// D. concurrent sessions remain independent
// ---------------------------------------------------------------------

#[tokio::test]
async fn concurrent_sessions_do_not_share_budget_or_context() {
    let provider = ScriptedProvider::new();
    let governance = SequenceHook::new(Vec::new());
    // Both sessions run the same single-call step concurrently with a budget
    // of 1: any cross-session budget sharing would fail one of them.
    let module = ProbeModule::new(vec![ProbeStep::OneCall, ProbeStep::OneCall]);
    let runtime = build_runtime(provider.clone(), governance.clone(), 1, module.clone()).await;

    let session_a = SessionId::new();
    let session_b = SessionId::new();
    let (ra, rb) = tokio::join!(
        runtime.execute_outcome(TurnRequest::new(session_a, "hello a")),
        runtime.execute_outcome(TurnRequest::new(session_b, "hello b")),
    );
    assert!(matches!(ra.unwrap(), TurnOutcome::Completed(_)));
    assert!(matches!(rb.unwrap(), TurnOutcome::Completed(_)));

    // Both side-calls succeeded: one per session, each with its own budget.
    assert_eq!(
        module.records(),
        vec!["solo:ok".to_string(), "solo:ok".to_string()]
    );
    let captures = governance.captures();
    let sessions: std::collections::BTreeSet<SessionId> =
        captures.iter().map(|(sid, _)| *sid).collect();
    let traces: std::collections::BTreeSet<TraceId> =
        captures.iter().map(|(_, tid)| *tid).collect();
    assert_eq!(
        sessions,
        std::collections::BTreeSet::from([session_a, session_b]),
        "exactly the two executed sessions, no fabricated ids"
    );
    assert_eq!(
        traces.len(),
        2,
        "two concurrent turns carry two distinct traces"
    );
}

// ---------------------------------------------------------------------
// E. governance deny: zero provider calls, fail closed
// ---------------------------------------------------------------------

#[tokio::test]
async fn deny_turn_scoped_handle_call_never_reaches_provider() {
    let provider = ScriptedProvider::new();
    // Evaluation order in one turn: TurnStart side-call first, main round
    // second. Deny the side-call; the main round is then denied too, which is
    // fine — the probe only asserts the side-call was refused fail-closed.
    let governance = SequenceHook::new(vec![
        Decision::deny("side-calls disabled"),
        Decision::deny("main disabled"),
    ]);
    let module = ProbeModule::new(vec![ProbeStep::OneCall]);
    let runtime = build_runtime(provider.clone(), governance, 8, module.clone()).await;

    let result = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "denied turn"))
        .await;
    assert!(result.is_err(), "a denied main round fails the turn");

    assert_eq!(
        module.records(),
        vec!["solo:denied".to_string()],
        "the side-call surfaced the governance denial, fail closed"
    );
    assert_eq!(
        provider.call_count(),
        0,
        "no provider call may happen under Deny"
    );
}

// ---------------------------------------------------------------------
// F. RequireApproval: zero provider calls for the side-call, no hidden approval
// ---------------------------------------------------------------------

#[tokio::test]
async fn require_approval_handle_call_creates_no_hidden_approval() {
    let provider = ScriptedProvider::new();
    // The side-call gets RequireApproval (fail closed); the main round is
    // allowed so the turn itself completes without any approval record.
    let governance = SequenceHook::new(vec![
        Decision::require_approval("escalation needed"),
        Decision::Allow,
    ]);
    let module = ProbeModule::new(vec![ProbeStep::OneCall]);
    let runtime = build_runtime(provider.clone(), governance, 8, module.clone()).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "approval turn"))
        .await
        .unwrap();
    assert!(
        matches!(outcome, TurnOutcome::Completed(_)),
        "no approval may be pending: the side-call cannot mint one"
    );

    assert_eq!(
        module.records(),
        vec!["solo:approval_required".to_string()],
        "the side-call surfaced RequireApproval as a fail-closed error"
    );
    assert_eq!(
        provider.call_count(),
        1,
        "only the main round reached the provider; the side-call never did"
    );
}

// ---------------------------------------------------------------------
// Bridge compatibility through the owned handle
// ---------------------------------------------------------------------

#[tokio::test]
async fn organ_llm_factory_bridge_works_over_the_owned_handle() {
    let provider = ScriptedProvider::new();
    let governance = SequenceHook::new(Vec::new());
    let module = ProbeModule::new(vec![ProbeStep::BridgeOneCall]);
    let runtime = build_runtime(provider.clone(), governance, 8, module.clone()).await;

    let outcome = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "bridge turn"))
        .await
        .unwrap();
    assert!(matches!(outcome, TurnOutcome::Completed(_)));

    let records = module.records();
    assert_eq!(records.len(), 1);
    assert!(
        records[0].starts_with("bridge:ok:echo: "),
        "the bridge returned the scripted provider text, got {:?}",
        records[0]
    );
    assert!(
        records[0].contains("bridge input"),
        "the organ request body travelled through the canonical invoker"
    );
    // One side-call through the bridge + one main round.
    assert_eq!(provider.call_count(), 2);
}
