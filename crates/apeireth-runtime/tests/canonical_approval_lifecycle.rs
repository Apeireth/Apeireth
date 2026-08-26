//! M2C-A deterministic approval/resume lifecycle proof.
//!
//! The runtime, plugin manager, registry, governance pipeline, provider router,
//! and session store are the real implementations. Only the provider and tools
//! are test fixtures.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{
    GovernancePipeline, Permission, PermissionGovernanceHook, PermissionPolicy,
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
    ApprovalDecision, ApprovalResolution, ApprovalStatus, InMemorySessionStore, Runtime,
    TurnOutcome, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";
const TOOL_A: &str = "allowed_a";
const TOOL_B: &str = "approval_b";
const TOOL_C: &str = "allowed_c";

// ---------------------------------------------------------------------------
// Counting tools
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Count {
    A,
    B,
    C,
}

impl Count {
    fn as_str(self) -> &'static str {
        match self {
            Self::A => TOOL_A,
            Self::B => TOOL_B,
            Self::C => TOOL_C,
        }
    }
}

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
            description: Some("counts invocations".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!(self.name))
    }
}

struct CountingPlugin {
    manifest: PluginManifest,
    invocations_a: Arc<AtomicUsize>,
    invocations_b: Arc<AtomicUsize>,
    invocations_c: Arc<AtomicUsize>,
}

impl CountingPlugin {
    fn new() -> Arc<Self> {
        let manifest = PluginManifest::new(
            PluginId::new("builtin.counting").unwrap(),
            "1.0.0",
            "Counting tools for approval lifecycle tests",
        )
        .declare_capability(
            CapabilityId::new("tool.allowed_a").unwrap(),
            CapabilityKind::Tool,
            "Allowed A",
        )
        .unwrap()
        .declare_capability(
            CapabilityId::new("tool.approval_b").unwrap(),
            CapabilityKind::Tool,
            "Approval B",
        )
        .unwrap()
        .declare_capability(
            CapabilityId::new("tool.allowed_c").unwrap(),
            CapabilityKind::Tool,
            "Allowed C",
        )
        .unwrap();

        Arc::new(Self {
            manifest,
            invocations_a: Arc::new(AtomicUsize::new(0)),
            invocations_b: Arc::new(AtomicUsize::new(0)),
            invocations_c: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn count(&self, count: Count) -> usize {
        match count {
            Count::A => self.invocations_a.load(Ordering::SeqCst),
            Count::B => self.invocations_b.load(Ordering::SeqCst),
            Count::C => self.invocations_c.load(Ordering::SeqCst),
        }
    }
}

#[async_trait]
impl Plugin for CountingPlugin {
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
        vec![
            Arc::new(CountingTool {
                id: CapabilityId::new("tool.allowed_a").unwrap(),
                name: TOOL_A,
                invocations: Arc::clone(&self.invocations_a),
            }),
            Arc::new(CountingTool {
                id: CapabilityId::new("tool.approval_b").unwrap(),
                name: TOOL_B,
                invocations: Arc::clone(&self.invocations_b),
            }),
            Arc::new(CountingTool {
                id: CapabilityId::new("tool.allowed_c").unwrap(),
                name: TOOL_C,
                invocations: Arc::clone(&self.invocations_c),
            }),
        ]
    }
}

// ---------------------------------------------------------------------------
// Scripted provider
// ---------------------------------------------------------------------------

enum ProviderStep {
    ToolCalls(Vec<ToolCall>),
    Say(&'static str),
}

struct FakeProvider {
    id: CapabilityId,
    steps: Vec<ProviderStep>,
    calls: AtomicUsize,
}

impl FakeProvider {
    fn new(id: &str, steps: Vec<ProviderStep>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            steps,
            calls: AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
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
        let step = self.steps.get(index).expect("provider script exhausted");
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
            ProviderStep::ToolCalls(calls) => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: calls.clone(),
                ..base
            }),
            ProviderStep::Say(text) => Ok(NormalizedResponse {
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
                PluginId::new("builtin.fake_provider").unwrap(),
                "1.0.0",
                "Scripted provider for approval tests",
            )
            .declare_capability(
                CapabilityId::new("provider.fake").unwrap(),
                CapabilityKind::Provider,
                "Scripted provider",
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

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn fixed_clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn three_tool_calls() -> Vec<ToolCall> {
    vec![
        ToolCall {
            id: "call_a".into(),
            name: TOOL_A.into(),
            arguments: serde_json::json!({}),
        },
        ToolCall {
            id: "call_b".into(),
            name: TOOL_B.into(),
            arguments: serde_json::json!({ "command": "echo approved" }),
        },
        ToolCall {
            id: "call_c".into(),
            name: TOOL_C.into(),
            arguments: serde_json::json!({}),
        },
    ]
}

async fn build_runtime_with(
    store: Arc<dyn apeireth_runtime::canonical::SessionStore>,
    counting: Arc<CountingPlugin>,
    provider: Arc<FakeProvider>,
) -> Runtime {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.allowed_a".into()));
    policy.grant(Permission::ExecuteTool("tool.approval_b".into()));
    policy.grant(Permission::ExecuteTool("tool.allowed_c".into()));
    policy.require_approval_for("tool.approval_b");

    Runtime::builder()
        .with_clock(fixed_clock())
        .with_session_store(store)
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(counting.clone())
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .unwrap()
}

async fn build_runtime(
    store: Arc<dyn apeireth_runtime::canonical::SessionStore>,
) -> (Runtime, Arc<CountingPlugin>, Arc<FakeProvider>) {
    let counting = CountingPlugin::new();
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            ProviderStep::ToolCalls(three_tool_calls()),
            ProviderStep::Say("all done"),
        ],
    );

    let runtime = build_runtime_with(store, counting.clone(), provider.clone()).await;
    (runtime, counting, provider)
}

fn pending_view(outcome: &TurnOutcome) -> apeireth_runtime::canonical::PendingApprovalView {
    match outcome {
        TurnOutcome::PendingApproval(view) => view.clone(),
        TurnOutcome::Completed(_) => panic!("expected PendingApproval"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn requiring_approval_returns_pending_outcome_and_tool_is_not_invoked() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, counting, provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();

    let view = pending_view(&outcome);
    assert_eq!(view.tool_name, TOOL_B);
    assert_eq!(view.capability_id.as_str(), "tool.approval_b");
    assert_eq!(view.tool_call.id, "call_b");
    assert!(!view.operation_fingerprint.is_empty());
    assert_eq!(counting.count(Count::A), 1, "A executes before the pause");
    assert_eq!(counting.count(Count::B), 0, "B waits for approval");
    assert_eq!(
        counting.count(Count::C),
        0,
        "C does not execute before approval"
    );
    assert_eq!(
        provider.call_count(),
        1,
        "provider is not re-queried while paused"
    );

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert_eq!(stored.active_approval_id, Some(view.approval_id));
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Pending
    );
}

#[tokio::test]
async fn approve_executes_frozen_tool_and_continues_everything_once() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, counting, provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();

    let response = match resolution {
        ApprovalResolution::Resumed(TurnOutcome::Completed(response)) => response,
        other => panic!("expected Resumed(Completed), got {other:?}"),
    };

    assert_eq!(response.text, "all done");
    assert_eq!(counting.count(Count::A), 1, "A executes once");
    assert_eq!(
        counting.count(Count::B),
        1,
        "B executes once after approval"
    );
    assert_eq!(counting.count(Count::C), 1, "C executes once after B");
    assert_eq!(provider.call_count(), 2, "provider continues after resume");

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert_eq!(stored.active_approval_id, None);
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Consumed
    );
}

#[tokio::test]
async fn reject_skips_tool_and_continues_remaining_calls() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, counting, provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

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

    let response = match resolution {
        ApprovalResolution::Resumed(TurnOutcome::Completed(response)) => response,
        other => panic!("expected Resumed(Completed), got {other:?}"),
    };

    assert_eq!(response.text, "all done");
    assert_eq!(counting.count(Count::A), 1);
    assert_eq!(
        counting.count(Count::B),
        0,
        "rejected tool must not execute"
    );
    assert_eq!(counting.count(Count::C), 1, "C still executes after reject");
    assert_eq!(provider.call_count(), 2);

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert_eq!(stored.active_approval_id, None);
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Rejected
    );
    assert_eq!(
        stored
            .approvals
            .get(&view.approval_id)
            .unwrap()
            .human_reason,
        Some("not now".into())
    );
}

#[tokio::test]
async fn double_approve_executes_only_once() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, counting, _provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    let first = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(matches!(
        first,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));

    let second = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    match second {
        ApprovalResolution::AlreadyResolved { status } => {
            assert_eq!(status, ApprovalStatus::Consumed)
        }
        other => panic!("expected AlreadyResolved(Consumed), got {other:?}"),
    }

    assert_eq!(counting.count(Count::B), 1);
}

#[tokio::test]
async fn concurrent_approve_executes_only_once() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, counting, _provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    let runtime = Arc::new(runtime);
    let first = {
        let runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            runtime
                .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
                .await
                .unwrap()
        })
    };
    let second = {
        let runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            runtime
                .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
                .await
                .unwrap()
        })
    };

    let (first, second) = tokio::join!(first, second);
    let first = first.unwrap();
    let second = second.unwrap();

    let completed = |resolution: ApprovalResolution| {
        matches!(
            resolution,
            ApprovalResolution::Resumed(TurnOutcome::Completed(_))
        )
    };
    let already = |resolution: ApprovalResolution| {
        matches!(
            resolution,
            ApprovalResolution::AlreadyResolved {
                status: ApprovalStatus::Consumed
            }
        )
    };

    assert!(
        (completed(first.clone()) && already(second.clone()))
            || (already(first) && completed(second)),
        "one resolution must claim and complete; the other must observe Consumed"
    );
    assert_eq!(counting.count(Count::B), 1, "exactly one execution");
}

#[tokio::test]
async fn reject_then_approve_does_not_execute() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, counting, _provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    let rejection = runtime
        .resolve_approval(
            session,
            view.approval_id,
            ApprovalDecision::Reject { reason: None },
        )
        .await
        .unwrap();
    assert!(matches!(
        rejection,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));

    let approval = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(matches!(
        approval,
        ApprovalResolution::AlreadyResolved {
            status: ApprovalStatus::Rejected
        }
    ));
    assert_eq!(counting.count(Count::B), 0);
}

#[tokio::test]
async fn expired_approval_does_not_execute() {
    let store = Arc::new(InMemorySessionStore::new());
    let clock = fixed_clock();
    let counting = CountingPlugin::new();
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            ProviderStep::ToolCalls(three_tool_calls()),
            ProviderStep::Say("all done"),
        ],
    );

    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.allowed_a".into()));
    policy.grant(Permission::ExecuteTool("tool.approval_b".into()));
    policy.grant(Permission::ExecuteTool("tool.allowed_c".into()));
    policy.require_approval_for("tool.approval_b");

    let runtime = Runtime::builder()
        .with_clock(clock.clone())
        .with_session_store(store.clone())
        .with_governance(Arc::new(
            GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy))),
        ))
        .with_plugin(counting.clone())
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .with_approval_ttl(5_000)
        .build()
        .await
        .unwrap();

    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    // Advance the virtual clock beyond the 5s approval TTL.
    clock.advance(chrono::Duration::seconds(6));

    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(matches!(resolution, ApprovalResolution::Expired));
    assert_eq!(counting.count(Count::B), 0);

    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .expect("session persisted");
    assert_eq!(
        stored.approvals.get(&view.approval_id).unwrap().status,
        ApprovalStatus::Expired
    );
}

#[tokio::test]
async fn reopen_pending_approval_survives_runtime_rebuild() {
    let store: Arc<dyn apeireth_runtime::canonical::SessionStore> =
        Arc::new(InMemorySessionStore::new());
    let counting = CountingPlugin::new();
    let provider = FakeProvider::new(
        "provider.fake",
        vec![
            ProviderStep::ToolCalls(three_tool_calls()),
            ProviderStep::Say("all done"),
        ],
    );

    let (session, approval_id) = {
        let runtime = build_runtime_with(store.clone(), counting.clone(), provider.clone()).await;
        let session = SessionId::new();
        let outcome = runtime
            .execute_outcome(TurnRequest::new(session, "run three tools"))
            .await
            .unwrap();
        let view = pending_view(&outcome);
        (session, view.approval_id)
    };

    // Rebuild a fresh runtime over the same store with the same provider and
    // tool plugins. The provider call counter is shared, so the continuation
    // sees the post-tool transcript and the provider answers instead of
    // re-issuing the original tool calls.
    let runtime = build_runtime_with(store.clone(), counting.clone(), provider.clone()).await;
    let resolution = runtime
        .resolve_approval(session, approval_id, ApprovalDecision::Approve)
        .await
        .unwrap();

    assert!(matches!(
        resolution,
        ApprovalResolution::Resumed(TurnOutcome::Completed(_))
    ));
    assert_eq!(counting.count(Count::B), 1);
}

#[tokio::test]
async fn new_turn_while_pending_is_blocked() {
    let store = Arc::new(InMemorySessionStore::new());
    let (runtime, _counting, _provider) = build_runtime(store.clone()).await;
    let session = SessionId::new();

    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "run three tools"))
        .await
        .unwrap();
    let view = pending_view(&outcome);

    let err = runtime
        .execute_outcome(TurnRequest::new(session, "conflicting new turn"))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            apeireth_runtime::canonical::RuntimeError::SessionApprovalPending { .. }
        ),
        "expected SessionApprovalPending, got {err}"
    );
    assert_eq!(view.approval_id.to_string().len(), 36);
}
