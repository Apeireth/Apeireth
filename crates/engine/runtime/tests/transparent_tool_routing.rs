//! Mesh Phase 1A acceptance: transparent remote tool routing.
//!
//! Every test maps to a required property (§26 items 1–25; §11 governance
//! counts; §14 main acceptance). The thesis under test: **the model does not
//! know where a tool runs; the runtime resolves where execution happens; the
//! mesh transports the invocation; the remote module still owns the
//! implementation.**

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    system_clock, CapabilityId, Metadata, ModelId, ModuleId, NodeId, SessionId, TraceId,
};
use apeireth_governance::{
    Action, ActionSource, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionPolicy,
};
use apeireth_mesh::{
    linked_pair, InvokeMeta, ManagerExecutor, MeshConnection, MeshEnvelope, MeshNode,
    MeshNodeConfig, MeshPayload, ModuleManagerView, RemoteCapabilityPort, RemoteToolDescriptor,
    TrustPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Module, ModuleManager, ModuleManifest, ProviderCapability, ProviderError,
    ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool,
    NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{runtime_mesh_executor, Runtime, TurnRequest};
use async_trait::async_trait;

const ECHO_CAP: &str = "tool.mesh.echo";
const TEST_MODULE: &str = "builtin.mesh-test";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Counts governance judgements and optionally denies — the single canonical
/// decision point on each side, instrumented for exactly-once assertions.
struct CountingHook {
    seen: Mutex<Vec<(String, ActionSource, TraceId, SessionId, u32)>>,
    deny: Option<String>,
    granted: Vec<String>,
}

impl CountingHook {
    fn allowing(capability: &str) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            deny: None,
            granted: vec![capability.to_string()],
        })
    }

    fn denying(capability: &str) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            deny: Some(capability.to_string()),
            granted: Vec::new(),
        })
    }

    fn dispatch_count(&self) -> usize {
        self.seen.lock().unwrap().len()
    }

    fn last(&self) -> Option<(String, ActionSource, TraceId, SessionId, u32)> {
        self.seen.lock().unwrap().last().cloned()
    }
}

#[async_trait]
impl GovernanceHook for CountingHook {
    fn name(&self) -> &str {
        "counting_hook"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        if let Action::CapabilityDispatch { capability, .. } = &request.action {
            self.seen.lock().unwrap().push((
                capability.to_string(),
                request.source.clone(),
                request.trace,
                request.session,
                request.round,
            ));
            if self.deny.as_deref() == Some(capability.as_str()) {
                return Decision::deny("denied by test policy");
            }
            if self.granted.iter().any(|g| g == capability.as_str()) {
                return Decision::Allow;
            }
            return Decision::deny("not granted");
        }
        Decision::Allow
    }
}

/// The `builtin.mesh-test` echo capability: returns its arguments verbatim.
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
            description: Some("mesh echo test capability".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

/// A module owning zero or more tools (optionally dynamic, for the MCP shape).
struct ToolModule {
    manifest: ModuleManifest,
    tools: Mutex<Vec<Arc<dyn ToolCapability>>>,
    dynamic: bool,
}

#[async_trait]
impl Module for ToolModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.lock().unwrap().clone()
    }
    async fn add_tool(&self, tool: Arc<dyn ToolCapability>) -> apeireth_plugin::PluginResult<()> {
        self.tools.lock().unwrap().push(tool);
        Ok(())
    }
    async fn remove_tool(&self, id: &CapabilityId) -> apeireth_plugin::PluginResult<bool> {
        let mut tools = self.tools.lock().unwrap();
        let before = tools.len();
        tools.retain(|tool| tool.id() != id);
        Ok(tools.len() != before)
    }
}

/// A static echo module: `builtin.mesh-test` owning one echo tool.
async fn echo_module(
    module_id: &str,
    capability: &str,
    tool_name: &str,
    runs: Arc<AtomicUsize>,
) -> Arc<dyn Module> {
    let mut manager = ModuleManager::new();
    let manifest = ModuleManifest::system(module_id, "1.0.0", "builtin mesh test module")
        .declare_capability(
            CapabilityId::new(capability).unwrap(),
            CapabilityKind::Tool,
            "echo across the mesh",
        )
        .unwrap();
    let module: Arc<dyn Module> = Arc::new(ToolModule {
        manifest,
        tools: Mutex::new(vec![Arc::new(EchoTool {
            id: CapabilityId::new(capability).unwrap(),
            name: tool_name.into(),
            runs,
        }) as Arc<dyn ToolCapability>]),
        dynamic: false,
    });
    manager.register(module.clone()).unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();
    module
}

/// A dynamic-ownership module (the MCP shape), registered and started empty.
async fn dynamic_module(module_id: &str) -> (Arc<dyn Module>, Arc<ToolModule>) {
    let mut manager = ModuleManager::new();
    let manifest = ModuleManifest::system(module_id, "1.0.0", "dynamic tools")
        .with_capability_ownership(apeireth_plugin::CapabilityOwnership::Dynamic);
    let module = Arc::new(ToolModule {
        manifest,
        tools: Mutex::new(Vec::new()),
        dynamic: true,
    });
    manager.register(module.clone() as Arc<dyn Module>).unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();
    (module.clone() as Arc<dyn Module>, module)
}

/// A live module-manager holder behind the mesh execution port — used where a
/// test must mutate the canonical index after the nodes are connected (the
/// dynamic/MCP shape). Real `ModuleManager`, real governance; only the runtime
/// shell is omitted because `Runtime::register_dynamic_tool` takes `&mut self`
/// by design.
struct Holder {
    manager: std::sync::RwLock<ModuleManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
}

impl ModuleManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&ModuleManager) -> R) -> R {
        f(&self.manager.read().unwrap())
    }
    fn governance(&self) -> &Arc<dyn GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn apeireth_core::kernel::Clock> {
        &self.clock
    }
}

/// A scripted provider: fixed responses, capturing every request it serves.
struct ScriptedProvider {
    id: CapabilityId,
    responses: Mutex<VecDeque<NormalizedResponse>>,
    captured: Arc<Mutex<Vec<NormalizedRequest>>>,
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor {
            id: ModelId::new("fake-model-1").unwrap(),
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
        self.captured.lock().unwrap().push(request.clone());
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
    manifest: ModuleManifest,
    provider: Arc<ScriptedProvider>,
}

#[async_trait]
impl Module for ProviderModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.provider.clone() as Arc<dyn ProviderCapability>]
    }
}

fn scripted_provider_module(
    responses: Vec<NormalizedResponse>,
) -> (Arc<dyn Module>, Arc<Mutex<Vec<NormalizedRequest>>>) {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(ScriptedProvider {
        id: CapabilityId::new("provider.fake").unwrap(),
        responses: Mutex::new(responses.into()),
        captured: Arc::clone(&captured),
    });
    let manifest = ModuleManifest::system("builtin.scripted", "1.0.0", "scripted provider")
        .declare_capability(
            CapabilityId::new("provider.fake").unwrap(),
            CapabilityKind::Provider,
            "scripted provider",
        )
        .unwrap();
    (Arc::new(ProviderModule { manifest, provider }), captured)
}

fn tool_call_response(call_id: &str, name: &str) -> NormalizedResponse {
    NormalizedResponse {
        id: "resp-1".into(),
        model: "fake-model-1".into(),
        content: String::new(),
        finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls),
        usage: NormalizedUsage::default(),
        tool_calls: vec![ToolCall {
            id: call_id.into(),
            name: name.into(),
            arguments: serde_json::json!({ "text": "hello-from-a" }),
        }],
        raw_metadata: serde_json::Map::new(),
    }
}

fn text_response(text: &str) -> NormalizedResponse {
    NormalizedResponse {
        id: "resp-2".into(),
        model: "fake-model-1".into(),
        content: text.into(),
        finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
        usage: NormalizedUsage::default(),
        tool_calls: Vec::new(),
        raw_metadata: serde_json::Map::new(),
    }
}

/// Counts InvokeCapability sends (handshake/advert traffic excluded).
struct CountingLink {
    inner: Arc<dyn MeshConnection>,
    invoke_sends: Arc<AtomicUsize>,
}

#[async_trait]
impl MeshConnection for CountingLink {
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), apeireth_mesh::MeshTransportError> {
        if matches!(envelope.payload, MeshPayload::InvokeCapability(_)) {
            self.invoke_sends.fetch_add(1, Ordering::SeqCst);
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

struct Rig {
    runtime_a: Arc<Runtime>,
    runtime_b: Arc<Runtime>,
    node_a: Arc<MeshNode>,
    node_b: Arc<MeshNode>,
    hook_a: Arc<CountingHook>,
    hook_b: Arc<CountingHook>,
    runs_b: Arc<AtomicUsize>,
    captured_a: Arc<Mutex<Vec<NormalizedRequest>>>,
    invoke_sends: Arc<AtomicUsize>,
}

/// Build connected A (scripted provider + port) and B (echo module) runtimes.
async fn rig(tool_name: &str, hook_a: Arc<CountingHook>, hook_b: Arc<CountingHook>) -> Rig {
    let runs_b = Arc::new(AtomicUsize::new(0));
    let runtime_b = Arc::new(
        Runtime::builder()
            .with_module(echo_module(TEST_MODULE, ECHO_CAP, tool_name, Arc::clone(&runs_b)).await)
            .with_governance(hook_b.clone())
            .build()
            .await
            .unwrap(),
    );
    let (provider_module, captured_a) = scripted_provider_module(vec![
        tool_call_response("call_1", tool_name),
        text_response("final answer"),
    ]);
    let runtime_a = Arc::new(
        Runtime::builder()
            .with_module(provider_module)
            .with_governance(hook_a.clone())
            .build()
            .await
            .unwrap(),
    );

    let node_a_id = NodeId::new("node-a").unwrap();
    let node_b_id = NodeId::new("node-b").unwrap();
    let node_a = MeshNode::new(
        MeshNodeConfig::new(node_a_id.clone(), "laptop")
            .with_trust(TrustPolicy::new().with_trusted(node_b_id.clone(), "memory://node-b")),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(node_b_id.clone(), "workstation")
            .with_trust(TrustPolicy::new().with_trusted(node_a_id, "memory://node-a")),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_b))),
        system_clock(),
    );
    runtime_a.set_remote_port(node_a.clone());

    let invoke_sends = Arc::new(AtomicUsize::new(0));
    let (link_a, link_b) = linked_pair();
    let link_a: Arc<dyn MeshConnection> = Arc::new(CountingLink {
        inner: Arc::new(link_a),
        invoke_sends: Arc::clone(&invoke_sends),
    });
    let (ra, rb) = tokio::join!(
        node_a.connect(link_a, node_b_id),
        node_b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    Rig {
        runtime_a,
        runtime_b,
        node_a,
        node_b,
        hook_a,
        hook_b,
        runs_b,
        captured_a,
        invoke_sends,
    }
}

// ---------------------------------------------------------------------------
// §14 main acceptance + items 7/8/9/10/11/12: a real model response drives a
// transparent remote tool round to completion.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_07_08_09_10_11_12_full_turn_invokes_the_remote_tool_transparently() {
    let hook_a = CountingHook::allowing(ECHO_CAP);
    let hook_b = CountingHook::allowing(ECHO_CAP);
    let rig = rig("mesh_echo", hook_a.clone(), hook_b.clone()).await;

    // (1) The remote-only tool is in the model catalog as an ordinary tool.
    let catalog = rig.runtime_a.tool_declarations();
    assert!(catalog.iter().any(|t| t.name == "mesh_echo"));
    // The model sees an ordinary declaration — no node identity in it.
    let declaration = catalog
        .iter()
        .find(|t| t.name == "mesh_echo")
        .expect("remote tool appears in the model catalog");
    assert_eq!(declaration.name, "mesh_echo");

    // A real turn: provider round 1 emits ToolCall("mesh_echo")… (§14)
    let session = SessionId::new();
    let response = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(session, "hello").with_model("fake-model-1"))
        .await
        .unwrap()
        .completed()
        .expect("the turn completes with a remote tool round");
    assert_eq!(response.text, "final answer");
    assert_eq!(response.rounds, 2, "one tool round + one final round");

    // …the canonical dispatch resolved the remote route, requester governance
    // ran once, the mesh carried the call, Node B governed and executed…
    assert_eq!(
        rig.runs_b.load(Ordering::SeqCst),
        1,
        "B's module executed once"
    );

    // (9) The ToolResult entered the next provider round.
    let captured = rig.captured_a.lock().unwrap();
    assert_eq!(captured.len(), 2, "two provider rounds");
    let round2_json = serde_json::to_string(&captured[1]).unwrap();
    assert!(
        round2_json.contains("hello-from-a"),
        "the remote ToolResult must reach the second provider round: {round2_json}"
    );

    // The provider never receives network-specific metadata; the model never
    // receives a NodeId.
    for request in captured.iter() {
        let json = serde_json::to_string(request).unwrap();
        for forbidden in [
            "node-a",
            "node-b",
            "RemoteToolRoute",
            "MeshEnvelope",
            "NodeId",
        ] {
            assert!(
                !json.contains(forbidden),
                "provider request leaked {forbidden}: {json}"
            );
        }
        assert!(
            request.tools.iter().any(|t| t.name == "mesh_echo"),
            "the model sees the ordinary tool declaration in every round"
        );
    }
    drop(captured);

    // (13) Requester governance exactly once, with remote-never-rechecked
    // semantics; executor governance exactly once.
    assert_eq!(
        rig.hook_a.dispatch_count(),
        1,
        "requester governance exactly once"
    );
    assert_eq!(
        rig.hook_b.dispatch_count(),
        1,
        "executor governance exactly once"
    );

    // (10/11/12) TraceId, SessionId and round survive the hop unchanged.
    let trace = response.trace.trace;
    let (_, source_b, trace_b, session_b, round_b) = rig.hook_b.last().unwrap();
    assert_eq!(trace_b, trace, "TraceId preserved end-to-end");
    assert_eq!(session_b, session, "SessionId preserved");
    assert_eq!(round_b, 1, "round preserved");
    assert!(
        matches!(source_b, ActionSource::Remote { node } if node.as_str() == "node-a"),
        "executor governance sees the remote source"
    );
    // The turn trace equals the invoke trace equals the remote execution trace.
    assert_eq!(
        rig.hook_a.last().unwrap().2,
        trace,
        "requester shares the turn trace"
    );
}

// ---------------------------------------------------------------------------
// §11 governance counts (items 13/14/15): one count per node per invocation.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_13_14_requester_deny_sends_no_rpc() {
    let hook_a = CountingHook::denying(ECHO_CAP);
    let hook_b = CountingHook::allowing(ECHO_CAP);
    let rig = rig("mesh_echo", hook_a.clone(), hook_b.clone()).await;

    let response = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model("fake-model-1"))
        .await
        .unwrap()
        .completed()
        .unwrap();

    assert_eq!(rig.hook_a.dispatch_count(), 1, "A governance calls = 1");
    assert_eq!(
        rig.invoke_sends.load(Ordering::SeqCst),
        0,
        "network sends = 0"
    );
    assert_eq!(rig.hook_b.dispatch_count(), 0, "B never saw the call");
    assert_eq!(
        rig.runs_b.load(Ordering::SeqCst),
        0,
        "implementation calls = 0"
    );
    // The model learns why via an ordinary tool error.
    let round2 = serde_json::to_string(&rig.captured_a.lock().unwrap()[1]).unwrap();
    assert!(round2.contains("refused by governance"), "{round2}");
    assert_eq!(response.text, "final answer");
}

#[tokio::test]
async fn test_15_executor_deny_invokes_no_implementation() {
    let hook_a = CountingHook::allowing(ECHO_CAP);
    let hook_b = CountingHook::denying(ECHO_CAP);
    let rig = rig("mesh_echo", hook_a.clone(), hook_b.clone()).await;

    let response = rig
        .runtime_a
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model("fake-model-1"))
        .await
        .unwrap()
        .completed()
        .unwrap();

    assert_eq!(rig.hook_a.dispatch_count(), 1, "A governance calls = 1");
    assert_eq!(rig.hook_b.dispatch_count(), 1, "B governance calls = 1");
    assert_eq!(
        rig.runs_b.load(Ordering::SeqCst),
        0,
        "implementation calls = 0"
    );
    assert_eq!(
        rig.invoke_sends.load(Ordering::SeqCst),
        1,
        "exactly one RPC"
    );
    let round2 = serde_json::to_string(&rig.captured_a.lock().unwrap()[1]).unwrap();
    assert!(round2.contains("refused by governance"), "{round2}");
    assert_eq!(response.text, "final answer");
}

// ---------------------------------------------------------------------------
// Items 2/3/19/20: availability and freshness of the automatic route.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_02_03_19_20_availability_freshness_and_reconnect() {
    let hook_a = CountingHook::allowing(ECHO_CAP);
    let hook_b = CountingHook::allowing(ECHO_CAP);
    let rig = rig("mesh_echo", hook_a, hook_b).await;
    let cap = CapabilityId::new(ECHO_CAP).unwrap();

    assert!(rig
        .runtime_a
        .tool_declarations()
        .iter()
        .any(|t| t.name == "mesh_echo"));

    // (2/19) An unreachable peer removes the automatic route immediately.
    rig.node_a.disconnect(rig.node_b.node_id());
    assert!(
        !rig.runtime_a
            .tool_declarations()
            .iter()
            .any(|t| t.name == "mesh_echo"),
        "a tool that cannot run must not be offered to the model"
    );

    // (20) Reconnect restores it — from the fresh advert, not a stale cache.
    let (link_a, link_b) = linked_pair();
    let (ra, rb) = tokio::join!(
        rig.node_a
            .connect(Arc::new(link_a), rig.node_b.node_id().clone()),
        rig.node_b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(rig
        .runtime_a
        .tool_declarations()
        .iter()
        .any(|t| t.name == "mesh_echo"));
    let result = rig
        .node_a
        .invoke_tool(
            &apeireth_mesh::RouteSelection::Remote(rig.node_b.node_id().clone()),
            &cap,
            ToolCall {
                id: "call_re".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({}),
            },
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await;
    assert!(result.is_ok());

    // (3) A removed remote tool disappears from the next derived catalog.
    rig.node_a.disconnect(rig.node_b.node_id());
    let (link_a, link_b) = linked_pair();
    let (ra, rb) = tokio::join!(
        rig.node_a
            .connect(Arc::new(link_a), rig.node_b.node_id().clone()),
        rig.node_b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(rig
        .runtime_a
        .tool_declarations()
        .iter()
        .any(|t| t.name == "mesh_echo"));
    // (3) Removal-propagation is asserted in the dynamic test below, where a
    // canonical unregister is possible; static modules cannot lose exports.
}

// ---------------------------------------------------------------------------
// Items 4/5: dynamic MCP tools flow through; ghost bag tools never do.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_04_05_dynamic_tools_advertised_ghost_bag_never() {
    let hook_b = CountingHook::allowing("tool.mcp.late");
    let runs = Arc::new(AtomicUsize::new(0));

    // Node B: a real ModuleManager with the MCP-shaped dynamic owner.
    let (_keep, mcp) = dynamic_module("module.mcp").await;
    let mut manager = ModuleManager::new();
    manager.register(_keep).unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();
    let holder = Arc::new(Holder {
        manager: std::sync::RwLock::new(manager),
        governance: hook_b,
        clock: system_clock(),
    });

    // Node A: a real Runtime whose model catalog is under test.
    let runtime_a = Arc::new(Runtime::builder().build().await.unwrap());

    let node_a_id = NodeId::new("node-a").unwrap();
    let node_b_id = NodeId::new("node-b").unwrap();
    let node_a = MeshNode::new(
        MeshNodeConfig::new(node_a_id.clone(), "laptop")
            .with_trust(TrustPolicy::new().with_trusted(node_b_id.clone(), "memory://node-b")),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))),
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(node_b_id.clone(), "workstation")
            .with_trust(TrustPolicy::new().with_trusted(node_a_id, "memory://node-a")),
        Arc::new(ManagerExecutor::new(holder.clone())),
        system_clock(),
    );
    runtime_a.set_remote_port(node_a.clone());

    // (4) A dynamically admitted tool flows: canonical index → advert →
    // projection → model catalog.
    let tool: Arc<dyn ToolCapability> = Arc::new(EchoTool {
        id: CapabilityId::new("tool.mcp.late").unwrap(),
        name: "late".into(),
        runs: Arc::clone(&runs),
    });
    holder
        .manager
        .write()
        .unwrap()
        .register_dynamic_tool(&ModuleId::new("module.mcp").unwrap(), tool)
        .await
        .unwrap();

    let (link_a, link_b) = linked_pair();
    let (ra, rb) = tokio::join!(
        node_a.connect(Arc::new(link_a), node_b_id),
        node_b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(
        runtime_a
            .tool_declarations()
            .iter()
            .any(|t| t.name == "late"),
        "a dynamically admitted remote tool must reach the model catalog"
    );

    // (5) A ghost tool filled straight into the module bag (the bypass
    // foot-gun) is never advertised and never in the model catalog.
    mcp.tools.lock().unwrap().push(Arc::new(EchoTool {
        id: CapabilityId::new("tool.mcp.ghost").unwrap(),
        name: "ghost".into(),
        runs: Arc::new(AtomicUsize::new(0)),
    }) as Arc<dyn ToolCapability>);
    node_b.publish_advert().await;
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(
        !runtime_a
            .tool_declarations()
            .iter()
            .any(|t| t.name == "ghost"),
        "a bag-only ghost tool must never reach the model catalog"
    );

    // (3) Removal through the canonical path disappears from the next catalog.
    holder
        .manager
        .write()
        .unwrap()
        .unregister_dynamic_tool(
            &ModuleId::new("module.mcp").unwrap(),
            &CapabilityId::new("tool.mcp.late").unwrap(),
        )
        .await
        .unwrap();
    node_b.publish_advert().await;
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(
        !runtime_a
            .tool_declarations()
            .iter()
            .any(|t| t.name == "late"),
        "a removed remote tool must vanish from the model catalog"
    );
}

// ---------------------------------------------------------------------------
// Items 16/17/18: collision policy — ambiguous names never reach the model,
// explicit routes still work.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_16_local_remote_name_collision_is_ambiguous() {
    let hook_a = CountingHook::allowing(ECHO_CAP);
    let hook_b = CountingHook::allowing(ECHO_CAP);
    let rig = rig("mesh_echo", hook_a.clone(), hook_b.clone()).await;

    // A local tool claiming the same model-facing name as B's remote tool.
    let local_runs = Arc::new(AtomicUsize::new(0));
    let local_manifest = ModuleManifest::system("builtin.local", "1.0.0", "local echo")
        .declare_capability(
            CapabilityId::new("tool.local.echo").unwrap(),
            CapabilityKind::Tool,
            "local echo",
        )
        .unwrap();
    let local_module: Arc<dyn Module> = Arc::new(ToolModule {
        manifest: local_manifest,
        tools: Mutex::new(vec![Arc::new(EchoTool {
            id: CapabilityId::new("tool.local.echo").unwrap(),
            name: "mesh_echo".into(),
            runs: local_runs.clone(),
        }) as Arc<dyn ToolCapability>]),
        dynamic: false,
    });
    // Register through a fresh manager view: the runtime is already built, so
    // assert at the route-table level (the same table dispatch consults).
    let mut manager = ModuleManager::new();
    manager.register(local_module).unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();

    let local_views: Vec<apeireth_runtime::canonical::LocalToolView> = manager
        .active_tools()
        .into_iter()
        .map(|tool| apeireth_runtime::canonical::LocalToolView {
            owner: manager.capabilities().owner(tool.id()).unwrap().clone(),
            capability_id: tool.id().clone(),
            declaration: tool.declaration(),
        })
        .collect();
    let remote = rig.node_a.remote_tools();
    let table = apeireth_runtime::canonical::ToolRouteTable::derive(local_views, remote);

    // (16) Ambiguous — excluded from the model catalog, resolvable only via
    // explicit routes (18).
    assert!(table.is_ambiguous("mesh_echo"));
    assert_eq!(
        table.model_catalog().len(),
        0,
        "ambiguous names never reach the model"
    );
    assert!(matches!(
        table.resolve("mesh_echo"),
        Err(apeireth_runtime::canonical::ToolRouteError::Ambiguous { .. })
    ));
    // (18) Explicit route still works while the name is contested.
    let result = rig
        .node_a
        .invoke_tool(
            &apeireth_mesh::RouteSelection::Remote(rig.node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            ToolCall {
                id: "call_explicit".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "n": 1 }),
            },
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await;
    assert!(result.is_ok(), "explicit routes remain callable");
    assert_eq!(
        local_runs.load(Ordering::SeqCst),
        0,
        "nothing ran implicitly"
    );
}

#[tokio::test]
async fn test_17_remote_remote_name_collision_is_ambiguous() {
    let hook_a = CountingHook::allowing(ECHO_CAP);
    let hook_b = CountingHook::allowing(ECHO_CAP);
    let hook_c = CountingHook::allowing("tool.mesh.search");
    let rig = rig("mesh_echo", hook_a, hook_b).await;

    // Node C advertises a *different* capability under the same tool name.
    let runs_c = Arc::new(AtomicUsize::new(0));
    let runtime_c = Arc::new(
        Runtime::builder()
            .with_module(
                echo_module("builtin.search", "tool.mesh.search", "mesh_echo", runs_c).await,
            )
            .with_governance(hook_c)
            .build()
            .await
            .unwrap(),
    );
    let node_c_id = NodeId::new("node-c").unwrap();
    let node_c = MeshNode::new(
        MeshNodeConfig::new(node_c_id.clone(), "c").with_trust(
            TrustPolicy::new().with_trusted(NodeId::new("node-a").unwrap(), "memory://node-a"),
        ),
        Arc::new(runtime_mesh_executor(runtime_c)),
        system_clock(),
    );
    // A must trust C: rebuild is unnecessary for the route-table assertion —
    // drive the projection through a direct table derive.
    let remote = vec![
        RemoteToolDescriptor {
            node: rig.node_b.node_id().clone(),
            module: ModuleId::new(TEST_MODULE).unwrap(),
            capability: CapabilityId::new(ECHO_CAP).unwrap(),
            declaration: NormalizedTool {
                name: "mesh_echo".into(),
                description: Some("b".into()),
                parameters: ToolParameters::new(),
                strict: false,
            },
            available: true,
        },
        RemoteToolDescriptor {
            node: node_c_id,
            module: ModuleId::new("builtin.search").unwrap(),
            capability: CapabilityId::new("tool.mesh.search").unwrap(),
            declaration: NormalizedTool {
                name: "mesh_echo".into(),
                description: Some("c".into()),
                parameters: ToolParameters::new(),
                strict: false,
            },
            available: true,
        },
    ];
    let table = apeireth_runtime::canonical::ToolRouteTable::derive(vec![], remote);
    assert!(
        table.is_ambiguous("mesh_echo"),
        "remote/remote collision is ambiguous"
    );
    assert_eq!(
        table.model_catalog().len(),
        0,
        "the model never sees a random winner"
    );
    assert!(matches!(
        table.resolve("mesh_echo"),
        Err(apeireth_runtime::canonical::ToolRouteError::Ambiguous { .. })
    ));
    let _ = node_c;
}

// ---------------------------------------------------------------------------
// Items 6/21: no-mesh and local behavior are unchanged.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_06_21_no_mesh_runtime_is_byte_equivalent() {
    let hook_a = CountingHook::allowing("tool.local.echo");
    let runs = Arc::new(AtomicUsize::new(0));
    let manifest = ModuleManifest::system("builtin.local", "1.0.0", "local echo")
        .declare_capability(
            CapabilityId::new("tool.local.echo").unwrap(),
            CapabilityKind::Tool,
            "local echo",
        )
        .unwrap();
    let local_module: Arc<dyn Module> = Arc::new(ToolModule {
        manifest,
        tools: Mutex::new(vec![Arc::new(EchoTool {
            id: CapabilityId::new("tool.local.echo").unwrap(),
            name: "local_echo".into(),
            runs: Arc::clone(&runs),
        }) as Arc<dyn ToolCapability>]),
        dynamic: false,
    });
    let (provider_module, captured) = scripted_provider_module(vec![
        tool_call_response("call_1", "local_echo"),
        text_response("done"),
    ]);
    let runtime = Arc::new(
        Runtime::builder()
            .with_module(local_module)
            .with_module(provider_module)
            .with_governance(hook_a.clone())
            .build()
            .await
            .unwrap(),
    );

    // (21) Without a port the catalog is byte-identical to the local list.
    assert_eq!(
        runtime.tool_declarations(),
        runtime.module_manager().tool_declarations(),
        "no-mesh tool_declarations must be byte-equivalent"
    );

    // (6) Local dispatch through the turn loop works exactly as before.
    let response = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "hi").with_model("fake-model-1"))
        .await
        .unwrap()
        .completed()
        .unwrap();
    assert_eq!(response.text, "done");
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    let round2 = serde_json::to_string(&captured.lock().unwrap()[1]).unwrap();
    assert!(round2.contains("hello-from-a"), "{round2}");
    assert_eq!(hook_a.dispatch_count(), 1);
}

// ---------------------------------------------------------------------------
// Items 22/23/24: ownership proof — projections are metadata only.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_22_23_24_registries_own_zero_remote_implementations() {
    let hook_a = CountingHook::allowing(ECHO_CAP);
    let hook_b = CountingHook::allowing(ECHO_CAP);
    let rig = rig("mesh_echo", hook_a, hook_b).await;

    // (24) Remote descriptors are metadata — no implementation object shape.
    for descriptor in rig.node_a.remote_tools() {
        let dumped = format!("{descriptor:?}");
        for forbidden in ["Arc", "dyn ", "fn(", "closure"] {
            assert!(
                !dumped.contains(forbidden),
                "projection leaked {forbidden:?}"
            );
        }
    }

    // (22/23) A's canonical registries hold A's modules and capabilities only.
    let manager = rig.runtime_a.module_manager();
    let module_ids: Vec<String> = manager.modules().ids().map(|id| id.to_string()).collect();
    assert!(
        module_ids.iter().all(|id| id != TEST_MODULE),
        "A's ModuleRegistry must not hold B's module: {module_ids:?}"
    );
    for id in manager.capabilities().ids() {
        assert_ne!(
            id.as_str(),
            ECHO_CAP,
            "A's CapabilityRegistry must not index the remote capability"
        );
    }
}

// ---------------------------------------------------------------------------
// Item 25: the turn loop depends on the port, never on transport types.
// ---------------------------------------------------------------------------

#[test]
fn test_25_turn_loop_has_no_transport_types() {
    let execute = include_str!("../src/canonical/execute.rs");
    let tool_route = include_str!("../src/canonical/tool_route.rs");
    let runtime_src = include_str!("../src/canonical/runtime.rs");
    for (name, source) in [
        ("execute.rs", execute),
        ("tool_route.rs", tool_route),
        ("runtime.rs", runtime_src),
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
