//! Mesh Phase 0 semantic acceptance tests (in-memory transport).
//!
//! Each test maps to one required property of the Phase 0 slice. Together they
//! prove the thesis: **remote execution is an extension of the existing
//! Module/Capability system, not a second runtime architecture** — dispatch
//! lands in the executing node's `ModuleManager` under its canonical
//! governance, adverts are read-only projections that own nothing, and the
//! network only ever changes where execution happens.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use apeireth_core::kernel::{
    system_clock, CapabilityId, Clock, ModuleId, NodeId, SessionId, Timestamp, TraceId,
};
use apeireth_governance::{Action, ActionSource, Decision, GovernanceHook, GovernanceRequest};
use apeireth_mesh::{
    linked_pair, CapabilityCatalogEntry, Hello, HelloAck, InvokeMeta, ManagerExecutor,
    MeshCapabilityExecutor, MeshConnection, MeshEnvelope, MeshError, MeshErrorCode, MeshExecError,
    MeshInvokeContext, MeshNode, MeshNodeConfig, MeshPayload, ModuleManagerView, NodeDescriptor,
    PeerState, RouteSelection, TrustPolicy, MESH_PROTOCOL_VERSION,
};
use apeireth_plugin::{CapabilityKind, Module, ModuleManager, ModuleManifest, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use tokio::sync::Notify;

// ---------------------------------------------------------------------------
// Harness: a canonical ModuleManager + governance behind the mesh port.
// ---------------------------------------------------------------------------

/// Records what the executing capability actually saw.
#[derive(Default)]
struct ExecRecorder {
    capability_ids: Mutex<Vec<String>>,
    count: AtomicUsize,
}

/// The `builtin.mesh-test` echo capability: returns its arguments verbatim.
struct EchoTool {
    id: CapabilityId,
    name: String,
    recorder: Arc<ExecRecorder>,
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
        self.recorder.count.fetch_add(1, Ordering::SeqCst);
        self.recorder
            .capability_ids
            .lock()
            .unwrap()
            .push(self.id.to_string());
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

/// A module owning the echo tool (or nothing, for dynamic-ownership tests).
struct MeshTestModule {
    manifest: ModuleManifest,
    tools: RwLock<Vec<Arc<dyn ToolCapability>>>,
    dynamic: bool,
}

#[async_trait]
impl Module for MeshTestModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.read().unwrap().clone()
    }
    async fn add_tool(&self, tool: Arc<dyn ToolCapability>) -> apeireth_plugin::PluginResult<()> {
        self.tools.write().unwrap().push(tool);
        Ok(())
    }
    async fn remove_tool(&self, id: &CapabilityId) -> apeireth_plugin::PluginResult<bool> {
        let mut tools = self.tools.write().unwrap();
        let before = tools.len();
        tools.retain(|tool| tool.id() != id);
        Ok(tools.len() != before)
    }
}

/// Canonical governance that records every judgement (and optionally denies a
/// named capability) — the executing node's single decision point.
struct RecordingGovernance {
    seen: Mutex<Vec<(String, ActionSource)>>,
    traces: Mutex<Vec<TraceId>>,
    deny_capability: Option<String>,
}

impl RecordingGovernance {
    fn allow_all() -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            traces: Mutex::new(Vec::new()),
            deny_capability: None,
        })
    }

    fn denying(capability: &str) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            traces: Mutex::new(Vec::new()),
            deny_capability: Some(capability.to_string()),
        })
    }

    fn judgements(&self) -> Vec<(String, ActionSource)> {
        self.seen.lock().unwrap().clone()
    }

    fn traces_seen(&self) -> Vec<TraceId> {
        self.traces.lock().unwrap().clone()
    }
}

#[async_trait]
impl GovernanceHook for RecordingGovernance {
    fn name(&self) -> &str {
        "recording_governance"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        if let Action::CapabilityDispatch { capability, .. } = &request.action {
            self.seen
                .lock()
                .unwrap()
                .push((capability.to_string(), request.source.clone()));
            self.traces.lock().unwrap().push(request.trace);
            if self.deny_capability.as_deref() == Some(capability.as_str()) {
                return Decision::deny("denied by test policy");
            }
        }
        Decision::Allow
    }
}

/// Holder behind the mesh execution port: one canonical manager + governance.
struct Holder {
    manager: RwLock<ModuleManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn Clock>,
    recorder: Arc<ExecRecorder>,
}

impl ModuleManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&ModuleManager) -> R) -> R {
        f(&self.manager.read().unwrap())
    }
    fn governance(&self) -> &Arc<dyn GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

/// A node's local world: the canonical manager (with `builtin.mesh-test`) and
/// its mesh presence.
struct TestNode {
    node: Arc<MeshNode>,
    holder: Arc<Holder>,
    recorder: Arc<ExecRecorder>,
}

fn echo_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "mesh_echo".into(),
        arguments: serde_json::json!({ "text": "hello-mesh" }),
    }
}

fn meta(trace: TraceId) -> InvokeMeta {
    InvokeMeta::new(SessionId::new(), trace, 1)
}

const ECHO_CAP: &str = "tool.mesh.echo";
const TEST_MODULE: &str = "builtin.mesh-test";

/// Build a node whose manager owns one `builtin.mesh-test` module exporting
/// `tool.mesh.echo`.
async fn node_with_echo(
    id: &str,
    governance: Arc<dyn GovernanceHook>,
) -> (Arc<MeshNode>, Arc<Holder>) {
    node_with_echo_and_trust(id, governance, TrustPolicy::new()).await
}

async fn node_with_echo_and_trust(
    id: &str,
    governance: Arc<dyn GovernanceHook>,
    trust: TrustPolicy,
) -> (Arc<MeshNode>, Arc<Holder>) {
    let recorder = Arc::new(ExecRecorder::default());
    let mut manager = ModuleManager::new();
    let manifest = ModuleManifest::system(TEST_MODULE, "1.0.0", "builtin mesh test module")
        .declare_capability(
            CapabilityId::new(ECHO_CAP).unwrap(),
            CapabilityKind::Tool,
            "echo across the mesh",
        )
        .unwrap();
    let module: Arc<dyn Module> = Arc::new(MeshTestModule {
        manifest,
        tools: RwLock::new(vec![Arc::new(EchoTool {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            name: "mesh_echo".into(),
            recorder: Arc::clone(&recorder),
        }) as Arc<dyn ToolCapability>]),
        dynamic: false,
    });
    manager.register(module).unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();

    let holder = Arc::new(Holder {
        manager: RwLock::new(manager),
        governance,
        clock: system_clock(),
        recorder,
    });
    let node_id = NodeId::new(id).unwrap();
    let config = MeshNodeConfig::new(node_id, format!("node {id}")).with_trust(trust);
    let node = MeshNode::new(
        config,
        Arc::new(ManagerExecutor::new(Arc::clone(&holder))),
        system_clock(),
    );
    (node, holder)
}

/// Connect two nodes over a fresh in-memory pair: A dials, B accepts.
async fn connect_pair(
    a: &Arc<MeshNode>,
    b: &Arc<MeshNode>,
) -> (Arc<dyn MeshConnection>, Arc<dyn MeshConnection>) {
    let (link_a, link_b) = linked_pair();
    let link_a: Arc<dyn MeshConnection> = Arc::new(link_a);
    let link_b: Arc<dyn MeshConnection> = Arc::new(link_b);
    let (ra, rb) = tokio::join!(
        a.connect(Arc::clone(&link_a), b.node_id().clone()),
        b.accept(Arc::clone(&link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    (link_a, link_b)
}

fn mutual_trust(a: &str, b: &str) -> (TrustPolicy, TrustPolicy) {
    let node_a = NodeId::new(a).unwrap();
    let node_b = NodeId::new(b).unwrap();
    (
        TrustPolicy::new().with_trusted(node_b.clone(), format!("memory://{b}")),
        TrustPolicy::new().with_trusted(node_a, format!("memory://{a}")),
    )
}

// ---------------------------------------------------------------------------
// 1. Two NodeIds never confuse.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_01_two_node_ids_are_distinct_and_identity_confusion_terminates_the_link() {
    let node_a = NodeId::new("node-a").unwrap();
    let node_b = NodeId::new("node-b").unwrap();
    assert_ne!(node_a, node_b);

    // As map keys they cannot collide.
    let mut map = std::collections::BTreeMap::new();
    map.insert(node_a.clone(), 1);
    map.insert(node_b.clone(), 2);
    assert_eq!(map.len(), 2);

    // An envelope claiming a source other than the handshaken identity is
    // refused and terminates the connection.
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a_mesh, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b_mesh, _) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;

    let (mine, theirs) = linked_pair();
    let mine: Arc<dyn MeshConnection> = Arc::new(mine);
    let accept_task = tokio::spawn({
        let node = Arc::clone(&node_b_mesh);
        let link: Arc<dyn MeshConnection> = Arc::new(theirs);
        async move { node.accept(link).await }
    });

    // Handshake as node-a…
    let hello = MeshEnvelope::send(
        node_a.clone(),
        node_b.clone(),
        TraceId::new(),
        MeshPayload::Hello(Hello {
            node: NodeDescriptor::new(node_a.clone(), "a", MESH_PROTOCOL_VERSION, "2.0.0"),
        }),
    );
    mine.send(hello).await.unwrap();
    assert!(matches!(
        mine.recv().await.unwrap().payload,
        MeshPayload::HelloAck(HelloAck { accepted: true, .. })
    ));
    assert_eq!(accept_task.await.unwrap().unwrap(), node_a);
    // Drain the post-handshake advert before probing with a confused envelope.
    assert!(matches!(
        mine.recv().await.unwrap().payload,
        MeshPayload::CapabilityAdvert(_)
    ));

    // …then smuggle an envelope from a third identity.
    let confused = MeshEnvelope::send(
        NodeId::new("node-c").unwrap(),
        node_b.clone(),
        TraceId::new(),
        MeshPayload::Heartbeat(apeireth_mesh::Heartbeat { sent_at_ms: 0 }),
    );
    mine.send(confused).await.unwrap();
    // B terminates the link: the far end observes the close.
    assert!(
        mine.recv().await.is_err(),
        "identity confusion must close the link"
    );
    let _ = node_a_mesh;
}

// ---------------------------------------------------------------------------
// 2. Protocol version mismatch fails closed.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_02_protocol_version_mismatch_fails_closed_without_downgrading() {
    let (tb, _) = mutual_trust("node-a", "node-b");
    let (node_b_mesh, _) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;

    let (mine, theirs) = linked_pair();
    let mine: Arc<dyn MeshConnection> = Arc::new(mine);
    let accept_task = tokio::spawn({
        let node = Arc::clone(&node_b_mesh);
        let link: Arc<dyn MeshConnection> = Arc::new(theirs);
        async move { node.accept(link).await }
    });

    let mut bad_descriptor = NodeDescriptor::new(
        NodeId::new("node-a").unwrap(),
        "a",
        MESH_PROTOCOL_VERSION,
        "2.0.0",
    );
    bad_descriptor.protocol_version = MESH_PROTOCOL_VERSION + 99;
    let hello = MeshEnvelope::send(
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
        TraceId::new(),
        MeshPayload::Hello(Hello {
            node: bad_descriptor,
        }),
    );
    mine.send(hello).await.unwrap();

    // The initiator is told, explicitly…
    let ack = mine.recv().await.unwrap();
    match ack.payload {
        MeshPayload::HelloAck(HelloAck {
            accepted: false,
            reason: Some(reason),
            ..
        }) => assert!(reason.contains("version"), "{reason}"),
        other => panic!("expected explicit rejection, got {other:?}"),
    }
    // …the accept side fails closed…
    let result = accept_task.await.unwrap();
    assert!(result.is_err(), "version mismatch must fail closed");
    // …and the link is closed rather than downgraded.
    assert!(mine.recv().await.is_err());
}

// ---------------------------------------------------------------------------
// 3. An untrusted peer cannot invoke capabilities.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_03_an_untrusted_peer_cannot_invoke_anything() {
    // B trusts nobody: A's handshake is refused before any capability flows.
    let (node_b_mesh, _) = node_with_echo("node-b", RecordingGovernance::allow_all()).await;
    let (ta, tb) = (
        TrustPolicy::new().with_trusted(NodeId::new("node-b").unwrap(), "memory://node-b"),
        TrustPolicy::new(), // A is unknown to B
    );
    let (node_a_mesh, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let _ = tb;

    let (link_a, link_b) = linked_pair();
    let link_a: Arc<dyn MeshConnection> = Arc::new(link_a);
    let link_b: Arc<dyn MeshConnection> = Arc::new(link_b);
    let (ra, rb) = tokio::join!(
        node_a_mesh.connect(Arc::clone(&link_a), node_b_mesh.node_id().clone()),
        node_b_mesh.accept(link_b),
    );
    assert!(
        ra.is_err() || rb.is_err(),
        "an unknown peer must not connect"
    );
    // No connection means no invocation, whatever the capability set says.
    let err = node_a_mesh
        .invoke_tool(
            &RouteSelection::Remote(node_b_mesh.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_1"),
            meta(TraceId::new()),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        MeshError::PeerUnavailable { .. } | MeshError::RouteUnavailable { .. }
    ));

    // An explicitly untrusted (known-but-refused) peer is refused the same way.
    let policy =
        TrustPolicy::new().with_untrusted(NodeId::new("node-c").unwrap(), "memory://node-c");
    assert!(!policy.is_trusted(&NodeId::new("node-c").unwrap()));
    assert!(policy.authorize(&NodeId::new("node-c").unwrap()).is_err());
}

// ---------------------------------------------------------------------------
// 4. A disconnected peer's capabilities are not dispatchable.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_04_a_disconnected_peers_capabilities_are_not_dispatchable() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, holder_b) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;
    connect_pair(&node_a, &node_b).await;

    // Wait for the advert to land.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(node_a
        .projection(node_b.node_id())
        .iter()
        .any(|cap| cap.id.as_str() == ECHO_CAP));

    node_a.disconnect(node_b.node_id());
    let err = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_1"),
            meta(TraceId::new()),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, MeshError::PeerUnavailable { .. }), "{err}");
    assert_eq!(holder_b.recorder.count.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------------------
// 5 + 20 + 21. Adverts create no second implementation ownership; local
// registries hold no remote implementations; the capability index stores
// identity only.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_05_20_21_an_advert_is_a_projection_and_never_an_implementation() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, holder_a) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, _holder_b) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let projection = node_a.projection(node_b.node_id());
    assert!(!projection.is_empty());
    // A projection entry is metadata: its Debug shape carries no implementation
    // object of any kind.
    for entry in &projection {
        let dumped = format!("{entry:?}");
        for forbidden in ["Arc", "dyn ", "fn(", "closure"] {
            assert!(
                !dumped.contains(forbidden),
                "projection leaked {forbidden:?}"
            );
        }
    }

    // (20) A's ModuleRegistry contains exactly its own module — never B's
    // `builtin.mesh-test` implementation object.
    let manager = holder_a.manager.read().unwrap();
    let local_ids: Vec<String> = manager.modules().ids().map(|id| id.to_string()).collect();
    assert_eq!(local_ids, vec![TEST_MODULE.to_string()]);
    assert_eq!(
        manager
            .modules()
            .get(&ModuleId::new(TEST_MODULE).unwrap())
            .unwrap()
            .manifest()
            .id
            .as_str(),
        TEST_MODULE
    );

    // (21) The capability index maps identity to owner, and every entry is
    // locally owned. The remote id exists only inside the routing projection.
    for id in manager.capabilities().ids() {
        let owner = manager.capabilities().owner(id).unwrap();
        assert_eq!(
            owner.as_str(),
            TEST_MODULE,
            "index holds local ownership only"
        );
    }
    assert!(node_a
        .remote_capabilities()
        .iter()
        .any(|cap| cap.id.as_str() == ECHO_CAP));
}

// ---------------------------------------------------------------------------
// 6. (NodeId, ModuleId) distinguishes same-named modules on different nodes.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_06_the_same_module_id_on_two_nodes_stays_distinguishable() {
    // A trusts both B and C; B and C both trust A.
    let trust_a = TrustPolicy::new()
        .with_trusted(NodeId::new("node-b").unwrap(), "memory://node-b")
        .with_trusted(NodeId::new("node-c").unwrap(), "memory://node-c");
    let trust_b =
        TrustPolicy::new().with_trusted(NodeId::new("node-a").unwrap(), "memory://node-a");
    let trust_c =
        TrustPolicy::new().with_trusted(NodeId::new("node-a").unwrap(), "memory://node-a");

    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), trust_a).await;
    let (node_b, _) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), trust_b).await;
    let (node_c, _) =
        node_with_echo_and_trust("node-c", RecordingGovernance::allow_all(), trust_c).await;
    connect_pair(&node_a, &node_b).await;
    connect_pair(&node_a, &node_c).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let cap = CapabilityId::new(ECHO_CAP).unwrap();
    let b_route = node_a
        .resolve(&cap, &RouteSelection::Remote(node_b.node_id().clone()))
        .unwrap();
    let c_route = node_a
        .resolve(&cap, &RouteSelection::Remote(node_c.node_id().clone()))
        .unwrap();
    assert_eq!(b_route.owner, c_route.owner, "same module id on both nodes");
    assert_ne!(
        b_route.located_owner().unwrap(),
        c_route.located_owner().unwrap(),
        "(node, module) must distinguish them"
    );
}

// ---------------------------------------------------------------------------
// 7. A local capability is never silently overridden by a remote one.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_07_a_remote_advert_never_overrides_the_same_local_capability() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    // Both A and B own `tool.mesh.echo` — the exact collision case.
    let (node_a, holder_a) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, holder_b) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let cap = CapabilityId::new(ECHO_CAP).unwrap();
    let routes = node_a.routes_for(&cap);
    assert_eq!(
        routes.len(),
        2,
        "both routes coexist in separate namespaces"
    );

    // Explicit local selection: A's own module runs, B's never does.
    let result = node_a
        .invoke_tool(
            &RouteSelection::Local,
            &cap,
            echo_call("call_local"),
            meta(TraceId::new()),
        )
        .await
        .unwrap();
    assert!(result.is_ok());
    assert_eq!(holder_a.recorder.count.load(Ordering::SeqCst), 1);
    assert_eq!(holder_b.recorder.count.load(Ordering::SeqCst), 0);

    // Explicit remote selection: B's module runs.
    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &cap,
            echo_call("call_remote"),
            meta(TraceId::new()),
        )
        .await
        .unwrap();
    assert!(result.is_ok());
    assert_eq!(holder_a.recorder.count.load(Ordering::SeqCst), 1);
    assert_eq!(holder_b.recorder.count.load(Ordering::SeqCst), 1);
}

// ---------------------------------------------------------------------------
// 8 + 9 + 12 + 14. Remote call preserves CapabilityId, lands in B's
// ModuleManager, returns the result, and keeps the TraceId.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_08_09_12_14_remote_call_round_trip_identity_trace_and_result() {
    let governance_b = RecordingGovernance::allow_all();
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _holder_a) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, holder_b) = node_with_echo_and_trust(
        "node-b",
        governance_b.clone() as Arc<dyn GovernanceHook>,
        tb,
    )
    .await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let trace = TraceId::new();
    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_42"),
            meta(trace),
        )
        .await
        .unwrap();

    // (12) The result comes back to A, correlated and intact.
    assert_eq!(result.tool_call_id, "call_42");
    assert!(result.is_ok());
    assert_eq!(result.render(), r#"{"text":"hello-mesh"}"#);

    // (8) The CapabilityId survived the hop unchanged…
    // (9) …and execution ran inside B's ModuleManager-owned module.
    assert_eq!(holder_b.recorder.count.load(Ordering::SeqCst), 1);
    assert_eq!(
        holder_b.recorder.capability_ids.lock().unwrap()[0],
        ECHO_CAP
    );

    // (14) The TraceId crossed the network unchanged: B's governance judged
    // the dispatch inside the caller's causal span.
    assert_eq!(governance_b.traces_seen(), vec![trace]);
}

// ---------------------------------------------------------------------------
// 10. Remote execution passes Node B's governance, with remote source.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_10_remote_execution_passes_node_b_governance_with_remote_source() {
    let governance_b = RecordingGovernance::allow_all();
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, _holder_b) = node_with_echo_and_trust(
        "node-b",
        governance_b.clone() as Arc<dyn GovernanceHook>,
        tb,
    )
    .await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_gov"),
            meta(TraceId::new()),
        )
        .await
        .unwrap();

    let judgements = governance_b.judgements();
    assert_eq!(
        judgements.len(),
        1,
        "Node B governance must see the dispatch"
    );
    assert_eq!(judgements[0].0, ECHO_CAP);
    assert!(
        matches!(&judgements[0].1, ActionSource::Remote { node } if node == node_a.node_id()),
        "the source context must name the calling node: {:?}",
        judgements[0].1
    );
}

// ---------------------------------------------------------------------------
// 11. A denied remote invocation never executes the capability.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_11_a_denied_remote_invocation_never_executes() {
    let governance_b = RecordingGovernance::denying(ECHO_CAP);
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, holder_b) = node_with_echo_and_trust(
        "node-b",
        governance_b.clone() as Arc<dyn GovernanceHook>,
        tb,
    )
    .await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let err = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_denied"),
            meta(TraceId::new()),
        )
        .await
        .unwrap_err();

    match err {
        MeshError::Remote {
            code, capability, ..
        } => {
            assert_eq!(code, MeshErrorCode::Denied);
            assert_eq!(capability.unwrap().as_str(), ECHO_CAP);
        }
        other => panic!("expected a mapped remote denial, got {other}"),
    }
    assert_eq!(
        holder_b.recorder.count.load(Ordering::SeqCst),
        0,
        "a denied invocation must never reach the capability"
    );
}

// ---------------------------------------------------------------------------
// 13. Remote errors map onto the caller's error surface.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_13_remote_errors_map_onto_the_caller_error_surface() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, holder_b) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // Stop B's module behind a still-live projection: the executing side now
    // reports NotAvailable and the caller sees a mapped remote error.
    holder_b.manager.write().unwrap().shutdown_all().await;

    let err = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_gone"),
            meta(TraceId::new()),
        )
        .await
        .unwrap_err();
    match err {
        MeshError::Remote { code, .. } => {
            assert_eq!(code, MeshErrorCode::NotAvailable, "{err}");
        }
        other => panic!("expected a mapped remote error, got {other}"),
    }
}

// ---------------------------------------------------------------------------
// 15. Cancellation propagates to the executing node.
// ---------------------------------------------------------------------------

/// Executor that runs until its cancel flag is set — the minimal "slow
/// capability" to observe cooperative cancellation.
struct SlowExecutor {
    started: Arc<Notify>,
}

#[async_trait]
impl MeshCapabilityExecutor for SlowExecutor {
    fn catalog(&self) -> Vec<CapabilityCatalogEntry> {
        Vec::new()
    }

    async fn execute_tool(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        _call: &ToolCall,
    ) -> Result<ToolResult, MeshExecError> {
        self.started.notify_one();
        for _ in 0..2_000 {
            if ctx.cancelled.is_cancelled() {
                return Err(MeshExecError::Cancelled {
                    capability: capability.clone(),
                });
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        Err(MeshExecError::Internal {
            capability: capability.clone(),
            message: "cancellation never arrived".into(),
        })
    }

    async fn authorize(
        &self,
        _ctx: &MeshInvokeContext,
        _capability: &CapabilityId,
        _arguments: &serde_json::Value,
    ) -> Result<(), MeshExecError> {
        Ok(())
    }

    async fn execute_provider(
        &self,
        _ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        _request: &apeireth_protocol::canonical::NormalizedRequest,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, MeshExecError> {
        Err(MeshExecError::Internal {
            capability: capability.clone(),
            message: "provider execution not used in this test".into(),
        })
    }

    async fn authorize_completion(
        &self,
        _ctx: &MeshInvokeContext,
        _model: &str,
        _message_count: usize,
    ) -> Result<(), MeshExecError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_15_cancellation_propagates_to_the_executing_node() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;

    // Node B executes through the slow executor (advertised via its catalog).
    let started = Arc::new(Notify::new());
    let node_b = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-b").unwrap(), "b").with_trust(tb),
        Arc::new(SlowExecutorWithAdvert {
            inner: SlowExecutor {
                started: Arc::clone(&started),
            },
        }),
        system_clock(),
    );

    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let handle = node_a
        .start_call(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_slow"),
            meta(TraceId::new()),
        )
        .await
        .unwrap();
    started.notified().await;

    node_a.cancel(handle.message_id).await;
    let err = handle.outcome().await.unwrap_err();
    assert!(matches!(err, MeshError::Cancelled), "{err}");
}

/// Wraps the slow executor with one advertised capability so routing resolves.
struct SlowExecutorWithAdvert {
    inner: SlowExecutor,
}

#[async_trait]
impl MeshCapabilityExecutor for SlowExecutorWithAdvert {
    fn catalog(&self) -> Vec<CapabilityCatalogEntry> {
        vec![CapabilityCatalogEntry {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            kind: apeireth_mesh::MeshCapabilityKind::Tool,
            owner: ModuleId::new(TEST_MODULE).unwrap(),
            description: "slow".into(),
            available: true,
            declaration: Some(NormalizedTool {
                name: "mesh_echo".into(),
                description: Some("slow".into()),
                parameters: ToolParameters::new(),
                strict: false,
            }),
            models: None,
        }]
    }

    async fn execute_tool(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        call: &ToolCall,
    ) -> Result<ToolResult, MeshExecError> {
        self.inner.execute_tool(ctx, capability, call).await
    }

    async fn authorize(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        arguments: &serde_json::Value,
    ) -> Result<(), MeshExecError> {
        self.inner.authorize(ctx, capability, arguments).await
    }

    async fn execute_provider(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        request: &apeireth_protocol::canonical::NormalizedRequest,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, MeshExecError> {
        self.inner.execute_provider(ctx, capability, request).await
    }

    async fn authorize_completion(
        &self,
        ctx: &MeshInvokeContext,
        model: &str,
        message_count: usize,
    ) -> Result<(), MeshExecError> {
        self.inner
            .authorize_completion(ctx, model, message_count)
            .await
    }
}

// ---------------------------------------------------------------------------
// 16. Heartbeat timeout makes routes unavailable (without losing identity).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_16_heartbeat_timeout_makes_the_route_unavailable() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, _) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    assert_eq!(node_a.peer_state(node_b.node_id()), Some(PeerState::Active));

    // Deterministic silence: sweep far past the timeout with no traffic.
    let now = Timestamp::from_clock(system_clock().as_ref())
        .epoch_millis()
        .max(0) as u64;
    node_a.sweep_heartbeats(now + 60_000);
    assert_eq!(
        node_a.peer_state(node_b.node_id()),
        Some(PeerState::Unreachable)
    );

    let err = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_late"),
            meta(TraceId::new()),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, MeshError::PeerUnavailable { .. }), "{err}");

    // The projection and the known-peer identity survive: nothing was
    // permanently lost.
    assert!(node_a
        .projection(node_b.node_id())
        .iter()
        .any(|cap| cap.id.as_str() == ECHO_CAP));
}

// ---------------------------------------------------------------------------
// 17. Reconnect restores the capability projection.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_17_reconnect_restores_the_capability_projection() {
    let (ta, tb) = mutual_trust("node-a", "node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), ta).await;
    let (node_b, _) =
        node_with_echo_and_trust("node-b", RecordingGovernance::allow_all(), tb).await;
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(node_a
        .projection(node_b.node_id())
        .iter()
        .any(|cap| cap.id.as_str() == ECHO_CAP));

    // Lose the connection entirely (and, for good measure, forget the stale
    // projection the way an operator reset would).
    node_a.disconnect(node_b.node_id());
    node_b.disconnect(node_a.node_id());

    // Reconnect over a fresh pair: handshake + advert restore the projection.
    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(node_a
        .projection(node_b.node_id())
        .iter()
        .any(|cap| cap.id.as_str() == ECHO_CAP));

    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b.node_id().clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_again"),
            meta(TraceId::new()),
        )
        .await
        .unwrap();
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// 18 + 19. A dynamic (MCP-style) capability is advertised, and its removal
// disappears from the projection — with no bag bypass.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_18_19_dynamic_capabilities_advertise_and_removals_propagate() {
    let recorder = Arc::new(ExecRecorder::default());
    let mut manager = ModuleManager::new();
    // A dynamic owner with no declared capabilities — the MCP shape.
    let manifest = ModuleManifest::system("module.mcp", "1.0.0", "mcp tools")
        .with_capability_ownership(apeireth_plugin::CapabilityOwnership::Dynamic);
    let mcp_module = Arc::new(MeshTestModule {
        manifest,
        tools: RwLock::new(Vec::new()),
        dynamic: true,
    });
    manager
        .register(Arc::clone(&mcp_module) as Arc<dyn Module>)
        .unwrap();
    let ctx = apeireth_plugin::ModuleInitContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();

    let holder = Arc::new(Holder {
        manager: RwLock::new(manager),
        governance: RecordingGovernance::allow_all(),
        clock: system_clock(),
        recorder,
    });

    let trust_a =
        TrustPolicy::new().with_trusted(NodeId::new("node-b").unwrap(), "memory://node-b");
    let (node_a, _) =
        node_with_echo_and_trust("node-a", RecordingGovernance::allow_all(), trust_a).await;
    let trust_b =
        TrustPolicy::new().with_trusted(NodeId::new("node-a").unwrap(), "memory://node-a");
    let node_b = MeshNode::new(
        MeshNodeConfig::new(NodeId::new("node-b").unwrap(), "b").with_trust(trust_b),
        Arc::new(ManagerExecutor::new(Arc::clone(&holder))),
        system_clock(),
    );

    // Admit the dynamic tool through the canonical manager path only — never
    // by filling any private bag.
    let tool: Arc<dyn ToolCapability> = Arc::new(EchoTool {
        id: CapabilityId::new("tool.mcp.late").unwrap(),
        name: "late".into(),
        recorder: Arc::new(ExecRecorder::default()),
    });
    holder
        .manager
        .write()
        .unwrap()
        .register_dynamic_tool(&ModuleId::new("module.mcp").unwrap(), tool)
        .await
        .unwrap();

    connect_pair(&node_a, &node_b).await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // (18) The dynamically admitted capability is advertised.
    assert!(node_a
        .projection(node_b.node_id())
        .iter()
        .any(|cap| cap.id.as_str() == "tool.mcp.late"));

    // A tool sitting in the module's bag but never admitted by the manager is
    // not network-visible: the canonical indexed view is the only surface.
    // This direct bag fill is exactly the historical bypass foot-gun.
    mcp_module.tools.write().unwrap().push(Arc::new(EchoTool {
        id: CapabilityId::new("tool.mcp.ghost").unwrap(),
        name: "ghost".into(),
        recorder: Arc::new(ExecRecorder::default()),
    }) as Arc<dyn ToolCapability>);
    node_b.publish_advert().await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(
        !node_a
            .projection(node_b.node_id())
            .iter()
            .any(|cap| cap.id.as_str() == "tool.mcp.ghost"),
        "a bag-only tool must never be advertised"
    );

    // (19) Removal through the canonical path disappears from the projection.
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
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(
        !node_a
            .projection(node_b.node_id())
            .iter()
            .any(|cap| cap.id.as_str() == "tool.mcp.late"),
        "a removed capability must vanish from the projection"
    );
}

// ---------------------------------------------------------------------------
// 22. Local dispatch through the same path has no regression.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_22_local_dispatch_through_the_canonical_path_still_works() {
    let governance = RecordingGovernance::allow_all();
    let (node_a, holder_a) =
        node_with_echo("node-a", governance.clone() as Arc<dyn GovernanceHook>).await;

    let result = node_a
        .invoke_tool(
            &RouteSelection::Local,
            &CapabilityId::new(ECHO_CAP).unwrap(),
            echo_call("call_local_plain"),
            meta(TraceId::new()),
        )
        .await
        .unwrap();
    assert!(result.is_ok());
    assert_eq!(holder_a.recorder.count.load(Ordering::SeqCst), 1);

    // Governance saw a *local* dispatch.
    let judgements = governance.judgements();
    assert!(matches!(judgements[0].1, ActionSource::Local));

    // Direct canonical dispatch (what the agent loop does) is unchanged.
    let tool = holder_a
        .manager
        .read()
        .unwrap()
        .tool(&CapabilityId::new(ECHO_CAP).unwrap())
        .unwrap();
    let result = tool.invoke(&echo_call("call_direct")).await;
    assert!(result.is_ok());
    assert_eq!(holder_a.recorder.count.load(Ordering::SeqCst), 2);
}
