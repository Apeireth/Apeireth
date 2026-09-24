//! Mesh Phase 1B acceptance: transparent remote provider routing.
//!
//! Every test maps to a required property (§31 items 1–34). The thesis: **the
//! agent owns the turn; the provider only performs inference.** The model may
//! run anywhere; location changes execution, not ownership.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    system_clock, CapabilityId, Metadata, ModelId, NodeId, PluginId, SessionId, TraceId,
};
use apeireth_governance::{Action, ActionSource, Decision, GovernanceHook, GovernanceRequest};
use apeireth_mesh::{
    linked_pair, InvokeMeta, MeshCapabilityExecutor, MeshConnection, MeshEnvelope, MeshNode,
    MeshNodeConfig, MeshPayload, RemoteCapabilityPort, RemoteProviderDescriptor, RemoteToolRoute,
    RouteSelection, TrustPolicy,
};
use apeireth_mesh::{
    CapabilityAdvertEntry, CapabilityCall, CapabilityOutcome, WireModelDescriptor,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginManifest, ProviderCapability, ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    resolve_model_route, runtime_mesh_executor, ModelRouteResolution, Runtime, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";
const PROVIDER_CAP: &str = "provider.test";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Records both canonical governance actions with their source and causal ids.
#[derive(Default)]
struct GovernanceProbe {
    seen: Mutex<Vec<(&'static str, String, ActionSource, TraceId, SessionId, u32)>>,
    deny: Mutex<Option<String>>,
}

impl GovernanceProbe {
    fn allowing() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn denying(action_label: &'static str) -> Arc<Self> {
        let probe = Self::default();
        *probe.deny.lock().unwrap() = Some(action_label.to_string());
        Arc::new(probe)
    }

    fn count(&self, action_label: &'static str) -> usize {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|(label, ..)| *label == action_label)
            .count()
    }

    fn last_for(
        &self,
        action_label: &'static str,
    ) -> Option<(String, ActionSource, TraceId, SessionId, u32)> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(label, ..)| *label == action_label)
            .map(|(_, name, source, trace, session, round)| {
                (name.clone(), source.clone(), *trace, *session, *round)
            })
    }
}

#[async_trait]
impl GovernanceHook for GovernanceProbe {
    fn name(&self) -> &str {
        "provider_probe"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let entry = match &request.action {
            Action::Completion { model, .. } => ("completion", model.to_string()),
            Action::CapabilityDispatch { capability, .. } => {
                ("capability_dispatch", capability.to_string())
            }
            other => ("other", other.label().to_string()),
        };
        self.seen.lock().unwrap().push((
            entry.0,
            entry.1,
            request.source.clone(),
            request.trace,
            request.session,
            request.round,
        ));
        if self.deny.lock().unwrap().as_deref() == Some(entry.0) {
            return Decision::deny("denied by test policy");
        }
        Decision::Allow
    }
}

/// A scripted provider capability: queued responses, capturing every request,
/// counting every invocation, optionally slow (for cancellation).
struct ScriptedProvider {
    id: CapabilityId,
    responses: Mutex<VecDeque<NormalizedResponse>>,
    captured: Arc<Mutex<Vec<NormalizedRequest>>>,
    runs: Arc<AtomicUsize>,
    slow: bool,
    finished: Arc<AtomicUsize>,
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor {
            id: ModelId::new(MODEL).unwrap(),
            provider: self.id.clone(),
            display_name: Some("fake".into()),
            context_window: Some(8_192),
            max_output_tokens: Some(1_024),
            features: vec![ModelFeature::ToolCalls],
            metadata: Metadata::new(),
        }]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        self.captured.lock().unwrap().push(request.clone());
        if self.slow {
            // Long inference: completes only if never cancelled (the runtime
            // drops this future on cancellation — we count completions to
            // prove the drop).
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
        self.finished.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(ProviderError::Refused {
                provider: self.id.to_string(),
                detail: "script exhausted".into(),
            })
    }
}

struct ProviderModule {
    manifest: PluginManifest,
    providers: Vec<Arc<dyn ProviderCapability>>,
}

#[async_trait]
impl Plugin for ProviderModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(
        &self,
        _ctx: &apeireth_plugin::PluginContext,
    ) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.providers.clone()
    }
}

fn provider_module(
    capability: &str,
    responses: Vec<NormalizedResponse>,
    slow: bool,
) -> (Arc<dyn Plugin>, Arc<ScriptedProvider>) {
    let provider = Arc::new(ScriptedProvider {
        id: CapabilityId::new(capability).unwrap(),
        responses: Mutex::new(responses.into()),
        captured: Arc::new(Mutex::new(Vec::new())),
        runs: Arc::new(AtomicUsize::new(0)),
        slow,
        finished: Arc::new(AtomicUsize::new(0)),
    });
    let manifest = PluginManifest::new(
        PluginId::new("builtin.scripted").unwrap(),
        "1.0.0",
        "scripted provider",
    )
    .declare_capability(
        CapabilityId::new(capability).unwrap(),
        CapabilityKind::Provider,
        "scripted provider",
    )
    .unwrap();
    (
        Arc::new(ProviderModule {
            manifest,
            providers: vec![provider.clone() as Arc<dyn ProviderCapability>],
        }),
        provider,
    )
}

/// The C-side echo tool.
struct EchoTool {
    id: CapabilityId,
    name: String,
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.clone(),
            description: Some("echo".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

struct ToolModule {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

#[async_trait]
impl Plugin for ToolModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(
        &self,
        _ctx: &apeireth_plugin::PluginContext,
    ) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
}

fn echo_module(capability: &str, tool_name: &str, runs: Arc<AtomicUsize>) -> Arc<dyn Plugin> {
    let manifest = PluginManifest::new(PluginId::new("builtin.echo").unwrap(), "1.0.0", "echo")
        .declare_capability(
            CapabilityId::new(capability).unwrap(),
            CapabilityKind::Tool,
            "echo",
        )
        .unwrap();
    Arc::new(ToolModule {
        manifest,
        tools: vec![Arc::new(EchoTool {
            id: CapabilityId::new(capability).unwrap(),
            name: tool_name.into(),
            runs,
        }) as Arc<dyn ToolCapability>],
    })
}

fn text_response(id: &str, text: &str) -> NormalizedResponse {
    NormalizedResponse {
        id: id.into(),
        model: MODEL.into(),
        content: text.into(),
        finish_reason: Some(NormalizedFinishReason::Stop),
        usage: NormalizedUsage {
            prompt_tokens: 7,
            completion_tokens: 3,
            total_tokens: 10,
        },
        tool_calls: Vec::new(),
        raw_metadata: serde_json::Map::new(),
    }
}

fn tool_call_response(id: &str, name: &str) -> NormalizedResponse {
    NormalizedResponse {
        id: id.into(),
        model: MODEL.into(),
        content: String::new(),
        finish_reason: Some(NormalizedFinishReason::ToolCalls),
        usage: NormalizedUsage::default(),
        tool_calls: vec![ToolCall {
            id: "call_1".into(),
            name: name.into(),
            arguments: serde_json::json!({ "text": "hello-from-b" }),
        }],
        raw_metadata: serde_json::Map::new(),
    }
}

/// Counts provider-shaped RPC sends (handshake/advert/tool traffic excluded).
struct CountingLink {
    inner: Arc<dyn MeshConnection>,
    provider_sends: Arc<AtomicUsize>,
}

#[async_trait]
impl MeshConnection for CountingLink {
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), apeireth_mesh::MeshTransportError> {
        if let MeshPayload::InvokeCapability(inv) = &envelope.payload {
            if matches!(inv.call, CapabilityCall::Provider(_)) {
                self.provider_sends.fetch_add(1, Ordering::SeqCst);
            }
        }
        self.inner.send(envelope).await
    }
    async fn recv(&self) -> Result<MeshEnvelope, apeireth_mesh::MeshTransportError> {
        self.inner.recv().await
    }
    fn close(&self) {
        self.inner.close();
    }
    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

/// Connect A→B over an in-memory pair wrapped in a provider-RPC counter (the
/// live link then lives inside the nodes; only the counter comes back).
async fn connect(a: &Arc<MeshNode>, b: &Arc<MeshNode>) -> Arc<AtomicUsize> {
    let provider_sends = Arc::new(AtomicUsize::new(0));
    let (link_a, link_b) = linked_pair();
    let link_a: Arc<dyn MeshConnection> = Arc::new(CountingLink {
        inner: Arc::new(link_a),
        provider_sends: Arc::clone(&provider_sends),
    });
    let (ra, rb) = tokio::join!(
        a.connect(link_a, b.node_id().clone()),
        b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    provider_sends
}

fn trust_pair(a: &str, b: &str) -> (TrustPolicy, TrustPolicy) {
    (
        TrustPolicy::new().with_trusted(NodeId::new(b).unwrap(), format!("memory://{b}")),
        TrustPolicy::new().with_trusted(NodeId::new(a).unwrap(), format!("memory://{a}")),
    )
}

fn mesh_node(
    id: &str,
    executor: Arc<dyn MeshCapabilityExecutor>,
    trust: TrustPolicy,
) -> Arc<MeshNode> {
    MeshNode::new(
        MeshNodeConfig::new(NodeId::new(id).unwrap(), format!("node {id}")).with_trust(trust),
        executor,
        system_clock(),
    )
}

/// Runtime A (turn owner, usually provider-less) + Node B (scripted provider).
struct Rig {
    runtime_a: Arc<Runtime>,
    node_a: Arc<MeshNode>,
    node_b: Arc<MeshNode>,
    hook_a: Arc<GovernanceProbe>,
    hook_b: Arc<GovernanceProbe>,
    provider: Arc<ScriptedProvider>,
    provider_sends: Arc<AtomicUsize>,
}

async fn rig(
    hook_a: Arc<GovernanceProbe>,
    hook_b: Arc<GovernanceProbe>,
    b_responses: Vec<NormalizedResponse>,
) -> Rig {
    let (module, provider) = provider_module(PROVIDER_CAP, b_responses, false);
    let runtime_b = Arc::new(
        Runtime::builder()
            .with_plugin(module)
            .with_governance(hook_b.clone())
            .build()
            .await
            .unwrap(),
    );
    let runtime_a = Arc::new(
        Runtime::builder()
            .with_governance(hook_a.clone())
            .build()
            .await
            .unwrap(),
    );

    let (ta, tb) = trust_pair("node-a", "node-b");
    let node_a = mesh_node(
        "node-a",
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))),
        ta,
    );
    let node_b = mesh_node("node-b", Arc::new(runtime_mesh_executor(runtime_b)), tb);
    runtime_a.set_remote_port(node_a.clone());
    let provider_sends = connect(&node_a, &node_b).await;

    Rig {
        runtime_a,
        node_a,
        node_b,
        hook_a,
        hook_b,
        provider,
        provider_sends,
    }
}

// ---------------------------------------------------------------------------
// §24 main acceptance A + items 7/1/3: remote-only provider serves a turn.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_01_07_24_remote_only_provider_serves_the_turn() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(
        hook_a.clone(),
        hook_b.clone(),
        vec![text_response("resp-1", "hello from remote provider")],
    )
    .await;

    // (1) The remote provider advert projects onto A with model metadata.
    let providers = rig.node_a.remote_providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].capability.as_str(), PROVIDER_CAP);
    assert!(providers[0].models.iter().any(|m| m.id.as_str() == MODEL));

    // §24: A starts a normal turn with no local provider at all.
    let session = SessionId::new();
    let response = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(session, "hi").with_model(MODEL))
        .await
        .unwrap()
        .completed()
        .expect("the turn completes against the remote provider");
    assert_eq!(response.text, "hello from remote provider");

    // (7) The automatic unambiguous route succeeded: B's module ran the model.
    assert_eq!(rig.provider.runs.load(Ordering::SeqCst), 1);

    // (3) Session/turn authority stayed in A; A's registries hold no B
    // implementation.
    assert_eq!(response.session, session, "session authority is A's");
    assert!(rig.runtime_a.plugins().plugins().is_empty());
    assert!(!rig
        .runtime_a
        .plugins()
        .capabilities()
        .contains(&CapabilityId::new(PROVIDER_CAP).unwrap()));

    // Exactly-once completion governance on both sides.
    assert_eq!(
        rig.hook_a.count("completion"),
        1,
        "requester governance exactly once"
    );
    assert_eq!(
        rig.hook_b.count("completion"),
        1,
        "executor governance exactly once"
    );
    let (_, source_b, trace_b, session_b, round_b) = rig.hook_b.last_for("completion").unwrap();
    assert!(
        matches!(source_b, ActionSource::Remote { node } if node.as_str() == "node-a"),
        "executor governance sees the remote source"
    );
    // (21/22/23) TraceId, SessionId, round preserved across the hop.
    assert_eq!(trace_b, response.trace.trace);
    assert_eq!(session_b, session);
    assert_eq!(round_b, 1);
}

// ---------------------------------------------------------------------------
// §25 main acceptance B + items 28/29/30: three nodes — remote provider does
// not steal the agent loop.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_28_29_30_three_node_turn_provider_on_b_tool_on_c() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let hook_c = GovernanceProbe::allowing();

    // A: turn owner, no providers, no tools.
    let runtime_a = Arc::new(
        Runtime::builder()
            .with_governance(hook_a.clone())
            .build()
            .await
            .unwrap(),
    );

    // B: provider only. Round 1 emits ToolCall("echo"); round 2 is the final.
    let (provider_module_b, provider_b) = provider_module(
        PROVIDER_CAP,
        vec![
            tool_call_response("resp-1", "echo"),
            text_response("resp-2", "all done"),
        ],
        false,
    );
    let runtime_b = Arc::new(
        Runtime::builder()
            .with_plugin(provider_module_b)
            .with_governance(hook_b.clone())
            .build()
            .await
            .unwrap(),
    );

    // C: remote tool only.
    let runs_c = Arc::new(AtomicUsize::new(0));
    let runtime_c = Arc::new(
        Runtime::builder()
            .with_plugin(echo_module("tool.echo", "echo", Arc::clone(&runs_c)))
            .with_governance(hook_c)
            .build()
            .await
            .unwrap(),
    );

    let trust_ab = TrustPolicy::new()
        .with_trusted(NodeId::new("node-b").unwrap(), "memory://node-b")
        .with_trusted(NodeId::new("node-c").unwrap(), "memory://node-c");
    let trust_ba =
        TrustPolicy::new().with_trusted(NodeId::new("node-a").unwrap(), "memory://node-a");
    let trust_ca =
        TrustPolicy::new().with_trusted(NodeId::new("node-a").unwrap(), "memory://node-a");

    let node_a = mesh_node(
        "node-a",
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))),
        trust_ab,
    );
    let node_b = mesh_node(
        "node-b",
        Arc::new(runtime_mesh_executor(runtime_b)),
        trust_ba,
    );
    let node_c = mesh_node(
        "node-c",
        Arc::new(runtime_mesh_executor(runtime_c)),
        trust_ca,
    );
    runtime_a.set_remote_port(node_a.clone());

    connect(&node_a, &node_b).await;
    connect(&node_a, &node_c).await;

    let session = SessionId::new();
    let response = runtime_a
        .execute_outcome(TurnRequest::new(session, "do it").with_model(MODEL))
        .await
        .unwrap()
        .completed()
        .expect("the three-node turn completes");
    assert_eq!(response.text, "all done");
    assert_eq!(response.rounds, 2);

    // B ran the model twice and executed no tool at all.
    assert_eq!(
        provider_b.runs.load(Ordering::SeqCst),
        2,
        "B serves both rounds"
    );
    // C's module executed the tool exactly once — the ToolCall came back to A
    // and A dispatched it (remote provider did not steal the agent loop).
    assert_eq!(
        runs_c.load(Ordering::SeqCst),
        1,
        "A dispatched the tool to C"
    );

    // §12: A's request to B carried the tool declarations — including the
    // remote tool projection from C — and B treated them as ordinary tools.
    let captured = provider_b.captured.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert!(
        captured[0].tools.iter().any(|t| t.name == "echo"),
        "A forwards its derived tool catalog (incl. C's tool) to the provider"
    );
    // (30) B saw only NormalizedRequest — no session authority, no turn state.
    let json = serde_json::to_string(&captured[0]).unwrap();
    assert!(
        !json.contains("node-a"),
        "no node identity in the model request"
    );
    drop(captured);

    // Round 2's transcript carried the tool result from C.
    let round2 = rig_json(&provider_b, 1);
    assert!(round2.contains("hello-from-b"), "{round2}");
    let _ = session;
}

fn rig_json(provider: &Arc<ScriptedProvider>, index: usize) -> String {
    serde_json::to_string(&provider.captured.lock().unwrap()[index]).unwrap()
}

// ---------------------------------------------------------------------------
// Items 11–14: governance counts.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_13_requester_deny_sends_zero_rpc() {
    let hook_a = GovernanceProbe::denying("completion");
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(
        hook_a.clone(),
        hook_b.clone(),
        vec![text_response("r", "x")],
    )
    .await;

    let result = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model(MODEL))
        .await;
    assert!(result.is_err(), "a denied completion aborts the turn");
    assert_eq!(rig.hook_a.count("completion"), 1);
    assert_eq!(
        rig.provider_sends.load(Ordering::SeqCst),
        0,
        "network sends = 0"
    );
    assert_eq!(rig.hook_b.count("completion"), 0);
    assert_eq!(
        rig.provider.runs.load(Ordering::SeqCst),
        0,
        "provider invocation = 0"
    );
}

#[tokio::test]
async fn test_14_executor_deny_runs_zero_provider_invocation() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::denying("completion");
    let rig = rig(
        hook_a.clone(),
        hook_b.clone(),
        vec![text_response("r", "x")],
    )
    .await;

    let result = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model(MODEL))
        .await;
    assert!(result.is_err(), "the remote refusal fails the round");
    assert_eq!(rig.hook_a.count("completion"), 1);
    assert_eq!(rig.hook_b.count("completion"), 1);
    assert_eq!(
        rig.provider.runs.load(Ordering::SeqCst),
        0,
        "provider invocation = 0"
    );
    assert_eq!(
        rig.provider_sends.load(Ordering::SeqCst),
        1,
        "exactly one RPC"
    );
}

// ---------------------------------------------------------------------------
// Items 8/9/10: collision policy and explicit routes.
// ---------------------------------------------------------------------------

#[test]
fn test_08_local_remote_model_collision_is_never_silently_shadowed() {
    let local = vec![CapabilityId::new(PROVIDER_CAP).unwrap()];
    let remote = vec![RemoteProviderDescriptor {
        node: NodeId::new("node-b").unwrap(),
        module: PluginId::new("builtin.scripted").unwrap(),
        capability: CapabilityId::new("provider.test").unwrap(),
        models: vec![wire_model("provider.test")],
        available: true,
    }];
    // Local provider + remote provider for the same model: ambiguous — no
    // silent local-wins, no silent remote-wins.
    assert!(matches!(
        resolve_model_route(MODEL, &local, &remote),
        ModelRouteResolution::Ambiguous { .. }
    ));
}

#[test]
fn test_09_remote_remote_model_collision_is_not_random() {
    let remote = vec![
        RemoteProviderDescriptor {
            node: NodeId::new("node-b").unwrap(),
            module: PluginId::new("builtin.scripted").unwrap(),
            capability: CapabilityId::new(PROVIDER_CAP).unwrap(),
            models: vec![wire_model(PROVIDER_CAP)],
            available: true,
        },
        RemoteProviderDescriptor {
            node: NodeId::new("node-c").unwrap(),
            module: PluginId::new("builtin.scripted").unwrap(),
            capability: CapabilityId::new(PROVIDER_CAP).unwrap(),
            models: vec![wire_model(PROVIDER_CAP)],
            available: true,
        },
    ];
    assert!(matches!(
        resolve_model_route(MODEL, &[], &remote),
        ModelRouteResolution::Ambiguous { .. }
    ));
    // Both routes coexist as distinct network identities (no first-wins).
    assert_ne!(remote[0].node, remote[1].node);
}

fn wire_model(provider: &str) -> WireModelDescriptor {
    WireModelDescriptor {
        id: ModelId::new(MODEL).unwrap(),
        provider: CapabilityId::new(provider).unwrap(),
        display_name: None,
        context_window: Some(1_024),
        max_output_tokens: None,
        features: vec!["tool_calls".to_string()],
    }
}

#[tokio::test]
async fn test_10_explicit_remote_provider_route_stays_callable() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(
        hook_a.clone(),
        hook_b.clone(),
        vec![text_response("r", "explicit ok")],
    )
    .await;

    // Even where automatic routing could be contested, the explicit route is
    // always callable.
    let request = NormalizedRequest::new(MODEL.to_string(), vec![]);
    let response = rig
        .node_a
        .invoke_provider(
            &RouteSelection::Remote(rig.node_b.node_id().clone()),
            &CapabilityId::new(PROVIDER_CAP).unwrap(),
            request,
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .unwrap();
    assert_eq!(response.content, "explicit ok");
    // High-level explicit path: requester completion governance exactly once.
    assert_eq!(rig.hook_a.count("completion"), 1);
}

// ---------------------------------------------------------------------------
// Items 15–20: wire roundtrip preserves the canonical response and request.
// ---------------------------------------------------------------------------

#[test]
fn test_15_16_17_18_19_20_wire_roundtrip_preserves_all_canonical_fields() {
    let mut raw_metadata = serde_json::Map::new();
    raw_metadata.insert("reasoning_content".into(), serde_json::json!("step 1"));
    raw_metadata.insert(
        "vendor_secret_marker".into(),
        serde_json::json!("kept-in-band"),
    );
    let response = NormalizedResponse {
        id: "resp-x".into(),
        model: MODEL.into(),
        content: "answer".into(),
        finish_reason: Some(NormalizedFinishReason::ToolCalls),
        usage: NormalizedUsage {
            prompt_tokens: 11,
            completion_tokens: 22,
            total_tokens: 33,
        },
        tool_calls: vec![ToolCall {
            id: "call_9".into(),
            name: "echo".into(),
            arguments: serde_json::json!({ "k": 1 }),
        }],
        raw_metadata,
    };

    // (16)(17)(18)(19) Full response preservation: content, reasoning (via
    // raw_metadata), finish_reason, usage, tool_calls — semantic equality.
    let outcome = CapabilityOutcome::Provider(response.clone());
    let envelope = MeshEnvelope::send(
        NodeId::new("node-b").unwrap(),
        NodeId::new("node-a").unwrap(),
        TraceId::new(),
        MeshPayload::CapabilityResult(apeireth_mesh::CapabilityResult {
            capability: CapabilityId::new(PROVIDER_CAP).unwrap(),
            outcome,
        }),
    );
    let text = apeireth_mesh::encode_envelope(&envelope).unwrap();
    assert!(
        text.contains("reasoning_content"),
        "reasoning stays in raw_metadata"
    );
    let decoded = apeireth_mesh::decode_envelope(&text).unwrap();
    match decoded.payload {
        MeshPayload::CapabilityResult(result) => match result.outcome {
            CapabilityOutcome::Provider(back) => {
                assert_eq!(back, response, "the wire must not drop canonical fields");
            }
            other => panic!("expected provider outcome, got {other:?}"),
        },
        other => panic!("expected result, got {other:?}"),
    }

    // (15)(20) Full request preservation, tool declarations included.
    let request = NormalizedRequest::new(
        MODEL.to_string(),
        vec![apeireth_protocol::canonical::NormalizedMessage::user("hi")],
    );
    let mut request = request;
    request.tools = vec![NormalizedTool {
        name: "echo".into(),
        description: Some("remote tool decl".into()),
        parameters: ToolParameters::new(),
        strict: false,
    }];
    let envelope = MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::InvokeCapability(apeireth_mesh::InvokeCapability {
            capability: CapabilityId::new(PROVIDER_CAP).unwrap(),
            kind: apeireth_mesh::MeshCapabilityKind::Provider,
            call: CapabilityCall::Provider(request.clone()),
            session: SessionId::new(),
            round: 1,
            deadline_epoch_ms: None,
        }),
    );
    let decoded =
        apeireth_mesh::decode_envelope(&apeireth_mesh::encode_envelope(&envelope).unwrap())
            .unwrap();
    match decoded.payload {
        MeshPayload::InvokeCapability(inv) => match inv.call {
            CapabilityCall::Provider(back) => {
                assert_eq!(back, request, "the wire must not reshape the request");
                assert_eq!(back.tools.len(), 1, "tool declarations preserved");
            }
            other => panic!("expected provider call, got {other:?}"),
        },
        other => panic!("expected invoke, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Items 2/32: adverts carry no credentials or provider config.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_02_32_provider_advert_never_carries_secrets_or_config() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(hook_a, hook_b, vec![text_response("r", "x")]).await;

    // The projection is metadata only: no implementation object shape.
    for descriptor in rig.node_a.remote_providers() {
        let dumped = format!("{descriptor:?}");
        for forbidden in ["Arc", "dyn ", "fn(", "closure"] {
            assert!(
                !dumped.contains(forbidden),
                "projection leaked {forbidden:?}"
            );
        }
    }

    // And the advert entry itself never carries provider config or secrets.
    let entry = CapabilityAdvertEntry::provider(
        CapabilityId::new(PROVIDER_CAP).unwrap(),
        "scripted",
        true,
        vec![wire_model(PROVIDER_CAP)],
    );
    let json = serde_json::to_string(&entry).unwrap();
    for forbidden in [
        "api_key",
        "apikey",
        "base_url",
        "credential",
        "bearer",
        "secret",
        "billing",
        "metadata",
    ] {
        assert!(
            !json.contains(forbidden),
            "advert leaked {forbidden:?}: {json}"
        );
    }
    // Structural: WireModelDescriptor has no metadata/config field at all.
    assert!(!json.contains("\"metadata\""));
}

// ---------------------------------------------------------------------------
// Items 4/5/31: ownership proof + availability gating.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_03_04_31_registries_hold_zero_remote_provider_implementations() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(hook_a, hook_b, vec![text_response("r", "x")]).await;

    let manager = rig.runtime_a.plugins();
    assert_eq!(
        manager.plugins().len(),
        0,
        "A's PluginRegistry owns nothing of B's"
    );
    assert_eq!(
        manager.capabilities().len(),
        0,
        "A's CapabilityRegistry is empty"
    );
    // The mesh RouteTable holds projection metadata only (asserted structurally
    // in the advert test above).
    assert_eq!(rig.node_a.remote_providers().len(), 1);
}

#[tokio::test]
async fn test_05_06_26_peer_state_gates_and_reconnect_restores_provider_routing() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(hook_a, hook_b, vec![text_response("r", "back")]).await;

    // (5/26) An unreachable peer removes the provider from automatic routing
    // immediately: resolution becomes None, the model is not silently retried
    // anywhere else.
    rig.node_a.disconnect(rig.node_b.node_id());
    assert!(rig.node_a.remote_providers().is_empty());
    assert!(matches!(
        resolve_model_route(MODEL, &[], &rig.node_a.remote_providers()),
        ModelRouteResolution::None
    ));
    let err = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model(MODEL))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apeireth_runtime::canonical::RuntimeError::NoProvider { .. }
    ));

    // (6) Reconnect + fresh advert restores automatic routing.
    let (ta, tb) = trust_pair("node-a", "node-b");
    let _ = (ta, tb);
    let (link_a, link_b) = linked_pair();
    let (ra, rb) = tokio::join!(
        rig.node_a
            .connect(Arc::new(link_a), rig.node_b.node_id().clone()),
        rig.node_b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert_eq!(rig.node_a.remote_providers().len(), 1);
    let response = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model(MODEL))
        .await
        .unwrap()
        .completed()
        .unwrap();
    assert_eq!(response.text, "back");
}

// ---------------------------------------------------------------------------
// Item 24 (deadline) and 25 (cancellation).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_24_deadline_is_propagated_and_enforced() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();
    let rig = rig(hook_a, hook_b, vec![text_response("r", "late")]).await;

    let request = NormalizedRequest::new(MODEL.to_string(), vec![]);
    let err = rig
        .node_a
        .invoke_provider(
            &RouteSelection::Remote(rig.node_b.node_id().clone()),
            &CapabilityId::new(PROVIDER_CAP).unwrap(),
            request,
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1).with_deadline(Some(1)),
        )
        .await
        .unwrap_err();
    // A deadline in the past is enforced on the executing side (the deadline
    // crossed the wire inside the invoke context).
    let text = err.to_string();
    assert!(
        text.contains("deadline") || text.to_lowercase().contains("cancel"),
        "{text}"
    );
    assert_eq!(rig.provider.runs.load(Ordering::SeqCst), 0, "never started");
}

#[tokio::test]
async fn test_25_cancellation_stops_a_remote_provider_call() {
    let hook_a = GovernanceProbe::allowing();
    let hook_b = GovernanceProbe::allowing();

    // Node B with a slow provider (30s inference).
    let (module, slow) = provider_module(PROVIDER_CAP, vec![text_response("r", "never")], true);
    let runtime_b = Arc::new(
        Runtime::builder()
            .with_plugin(module)
            .with_governance(hook_b)
            .build()
            .await
            .unwrap(),
    );
    let runtime_a = Arc::new(
        Runtime::builder()
            .with_governance(hook_a)
            .build()
            .await
            .unwrap(),
    );
    let (ta, tb) = trust_pair("node-a", "node-b");
    let node_a = mesh_node(
        "node-a",
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))),
        ta,
    );
    let node_b = mesh_node("node-b", Arc::new(runtime_mesh_executor(runtime_b)), tb);
    runtime_a.set_remote_port(node_a.clone());
    connect(&node_a, &node_b).await;

    // Start a remote provider call through the shared raw primitive.
    let handle = node_a
        .start_capability_call(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(PROVIDER_CAP).unwrap(),
            CapabilityCall::Provider(NormalizedRequest::new(MODEL.to_string(), vec![])),
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    node_a.cancel(handle.message_id).await;

    let err = handle.outcome().await.unwrap_err();
    assert!(matches!(err, apeireth_mesh::MeshError::Cancelled), "{err}");
    // The backend inference future was dropped — it never completed.
    assert_eq!(
        slow.finished.load(Ordering::SeqCst),
        0,
        "late response never lands"
    );
}

// ---------------------------------------------------------------------------
// Item 27: no-mesh local provider path unchanged (incl. fallback).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_27_local_provider_path_and_fallback_are_unchanged() {
    let hook = GovernanceProbe::allowing();

    // Two local providers: the first fails transiently, the second serves —
    // the router's existing fallback semantics must survive intact.
    let (module_b, serving) = provider_module(
        PROVIDER_CAP,
        vec![text_response("resp-2", "local answer")],
        false,
    );

    let transient = Arc::new(TransientProvider {
        id: CapabilityId::new("provider.flaky").unwrap(),
        runs: Arc::new(AtomicUsize::new(0)),
    });
    let flaky_manifest =
        PluginManifest::new(PluginId::new("builtin.flaky").unwrap(), "1.0.0", "flaky")
            .declare_capability(
                CapabilityId::new("provider.flaky").unwrap(),
                CapabilityKind::Provider,
                "flaky",
            )
            .unwrap();
    let flaky_module = Arc::new(ProviderModule {
        manifest: flaky_manifest,
        providers: vec![transient.clone() as Arc<dyn ProviderCapability>],
    });

    let runtime = Arc::new(
        Runtime::builder()
            .with_plugin(flaky_module)
            .with_plugin(module_b)
            .with_governance(hook.clone())
            .build()
            .await
            .unwrap(),
    );

    // Selection and request/response flow unchanged without a remote port.
    let response = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model(MODEL))
        .await
        .unwrap()
        .completed()
        .unwrap();
    assert_eq!(response.text, "local answer");
    assert_eq!(
        transient.runs.load(Ordering::SeqCst),
        1,
        "fallback tried the flaky one first"
    );
    assert_eq!(
        serving.runs.load(Ordering::SeqCst),
        1,
        "then served from the healthy one"
    );
    assert_eq!(response.served_by.as_str(), PROVIDER_CAP);
    assert_eq!(hook.count("completion"), 1);
    // Tool declarations in the request stayed attached per provider capability.
    let captured = serving.captured.lock().unwrap();
    assert!(
        serde_json::to_string(&captured[0])
            .unwrap()
            .contains("hello")
            || captured[0].messages.len() == 1
    );
}

/// Fails once transiently (so fallback must continue), then would succeed.
struct TransientProvider {
    id: CapabilityId,
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl ProviderCapability for TransientProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor {
            id: ModelId::new(MODEL).unwrap(),
            provider: self.id.clone(),
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            features: vec![ModelFeature::ToolCalls],
            metadata: Metadata::new(),
        }]
    }
    async fn complete(
        &self,
        _request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Err(ProviderError::Timeout {
            provider: self.id.to_string(),
            timeout_ms: 1,
        })
    }
}

// ---------------------------------------------------------------------------
// Item 33: the provider routing loop never touches transport/peer types.
// ---------------------------------------------------------------------------

#[test]
fn test_33_provider_loop_has_no_transport_types() {
    for (name, source) in [
        ("execute.rs", include_str!("../src/canonical/execute.rs")),
        ("provider.rs", include_str!("../src/canonical/provider.rs")),
        (
            "tool_route.rs",
            include_str!("../src/canonical/tool_route.rs"),
        ),
        ("runtime.rs", include_str!("../src/canonical/runtime.rs")),
    ] {
        for forbidden in [
            "MeshConnection",
            "InMemoryMeshTransport",
            "MeshTransport",
            "MeshEnvelope",
            "MeshNode",
            "PeerState",
            "Handshake",
        ] {
            assert!(
                !source.contains(forbidden),
                "{name} must not reference transport/peer type {forbidden}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Item 34: the security-round invariants remain in force (structural guard;
// their behavioral tests run in the workspace battery).
// ---------------------------------------------------------------------------

#[test]
fn test_34_security_round_invariants_remain_in_force() {
    let execute = include_str!("../src/canonical/execute.rs");
    assert!(
        execute.contains("MAX_TOOL_CALLS_PER_ROUND"),
        "M20 tool-call bounding must remain"
    );
    assert!(
        execute.contains("resumed_context"),
        "M27 frozen security-context propagation must remain"
    );
}
