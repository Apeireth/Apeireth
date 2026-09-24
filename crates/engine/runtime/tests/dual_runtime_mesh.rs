//! Phase 0 main acceptance: two full Apeireth runtimes, one remote call.
//!
//! ```text
//! Runtime A ── MeshNode A ──(in-memory link)── MeshNode B ── Runtime B
//!     │                                                            │
//!     └── invoke tool.mesh.echo (remote route) ──► governance B ──► ModuleManager B
//!                                                    │
//!                                          module builtin.mesh-test
//!                                          capability tool.mesh.echo
//! ```
//!
//! This test runs against real [`Runtime`] instances (not test doubles) to
//! prove the thesis end to end: **remote execution is an extension of the
//! existing Module/Capability system, not a second runtime architecture.**

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use apeireth_core::kernel::{system_clock, CapabilityId, ModuleId, NodeId, SessionId, TraceId};
use apeireth_governance::{
    ActionSource, GovernanceRequest, Permission, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_mesh::{InvokeMeta, RouteSelection, TrustPolicy};
use apeireth_plugin::{CapabilityKind, Module, ModuleManifest, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_runtime::canonical::{
    runtime_mesh_executor, MeshCapabilityExecutor, MeshNode, MeshNodeConfig, Runtime,
};
use async_trait::async_trait;

const ECHO_CAP: &str = "tool.mesh.echo";
const TEST_MODULE: &str = "builtin.mesh-test";

/// Records what Node B's governance judged, for source/trace assertions.
#[derive(Default)]
struct GovernanceProbe {
    seen: Mutex<Vec<(String, ActionSource, TraceId)>>,
}

impl GovernanceProbe {
    fn hook(self: &Arc<Self>) -> apeireth_governance::GovernancePipeline {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool(ECHO_CAP.to_string()));
        apeireth_governance::GovernancePipeline::new()
            .with(Arc::new(ProbeHook(Arc::clone(self))))
            .with(Arc::new(PermissionGovernanceHook::new(policy)))
    }
}

struct ProbeHook(Arc<GovernanceProbe>);

#[async_trait]
impl apeireth_governance::GovernanceHook for ProbeHook {
    fn name(&self) -> &str {
        "probe_hook"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> apeireth_governance::Decision {
        if let apeireth_governance::Action::CapabilityDispatch { capability, .. } = &request.action
        {
            self.0.seen.lock().unwrap().push((
                capability.to_string(),
                request.source.clone(),
                request.trace,
            ));
        }
        apeireth_governance::Decision::Allow
    }
}

/// The `builtin.mesh-test` echo capability, counting real executions.
struct EchoTool {
    id: CapabilityId,
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "mesh_echo".into(),
            description: Some("builtin mesh acceptance echo".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

struct MeshTestModule {
    manifest: ModuleManifest,
    tools: RwLock<Vec<Arc<dyn ToolCapability>>>,
}

#[async_trait]
impl Module for MeshTestModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.read().unwrap().clone()
    }
}

fn echo_module(runs: Arc<AtomicUsize>) -> Arc<dyn Module> {
    let manifest = ModuleManifest::system(TEST_MODULE, "1.0.0", "builtin mesh acceptance module")
        .declare_capability(
            CapabilityId::new(ECHO_CAP).unwrap(),
            CapabilityKind::Tool,
            "echo across the mesh",
        )
        .unwrap();
    Arc::new(MeshTestModule {
        manifest,
        tools: RwLock::new(vec![Arc::new(EchoTool {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            runs,
        }) as Arc<dyn ToolCapability>]),
    })
}

#[tokio::test]
async fn two_runtimes_exchange_a_capability_call_over_the_mesh() {
    // ---- Runtime B: owns `builtin.mesh-test` / `tool.mesh.echo`. -----------
    let runs = Arc::new(AtomicUsize::new(0));
    let probe_b = Arc::new(GovernanceProbe::default());
    let runtime_b = Arc::new(
        Runtime::builder()
            .with_module(echo_module(Arc::clone(&runs)))
            .with_governance(Arc::new(probe_b.hook()))
            .build()
            .await
            .expect("runtime B builds"),
    );

    // ---- Runtime A: no local echo; it will reach B's. --------------------
    let mut policy_a = PermissionPolicy::new();
    policy_a.grant(Permission::ExecuteTool(ECHO_CAP.to_string()));
    let runtime_a = Arc::new(
        Runtime::builder()
            .with_governance(Arc::new(PermissionGovernanceHook::new(policy_a)))
            .build()
            .await
            .expect("runtime A builds"),
    );

    // ---- Mesh presence: semantics over the in-memory transport. ----------
    let node_a_id = NodeId::new("node-a").unwrap();
    let node_b_id = NodeId::new("node-b").unwrap();
    let trust_a = TrustPolicy::new().with_trusted(node_b_id.clone(), "memory://node-b");
    let trust_b = TrustPolicy::new().with_trusted(node_a_id.clone(), "memory://node-a");

    let node_a = MeshNode::new(
        MeshNodeConfig::new(node_a_id.clone(), "laptop").with_trust(trust_a),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))) as Arc<dyn MeshCapabilityExecutor>,
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(node_b_id.clone(), "workstation").with_trust(trust_b),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_b))) as Arc<dyn MeshCapabilityExecutor>,
        system_clock(),
    );

    let (link_a, link_b) = apeireth_mesh::linked_pair();
    let link_a: Arc<dyn apeireth_mesh::MeshConnection> = Arc::new(link_a);
    let link_b: Arc<dyn apeireth_mesh::MeshConnection> = Arc::new(link_b);
    let (ra, rb) = tokio::join!(
        node_a.connect(link_a, node_b_id.clone()),
        node_b.accept(link_b),
    );
    assert_eq!(ra.unwrap(), node_b_id);
    assert_eq!(rb.unwrap(), node_a_id);

    // A receives B's capability projection: identity and owner, no more.
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let projection = node_a.projection(&node_b_id);
    let echo_projection = projection
        .iter()
        .find(|cap| cap.id.as_str() == ECHO_CAP)
        .expect("B's tool.mesh.echo must be projected onto A");
    assert_eq!(echo_projection.owner.module.as_str(), TEST_MODULE);
    assert_eq!(echo_projection.owner.node, node_b_id);

    // A's own runtime never gained an implementation: its module registry
    // still holds exactly nothing.
    assert_eq!(runtime_a.module_manager().modules().len(), 0);
    assert!(!runtime_a
        .module_manager()
        .capabilities()
        .contains(&CapabilityId::new(ECHO_CAP).unwrap()));

    // ---- The first real remote call. --------------------------------------
    let trace = TraceId::new();
    let call = ToolCall {
        id: "call_remote_1".into(),
        name: "mesh_echo".into(),
        arguments: serde_json::json!({ "hello": "from-a" }),
    };
    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b_id.clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            call,
            InvokeMeta::new(SessionId::new(), trace, 1),
        )
        .await
        .expect("the remote call returns a canonical ToolResult");

    // The canonical result came back to A, correlated and intact.
    assert!(result.is_ok());
    assert_eq!(result.tool_call_id, "call_remote_1");
    assert_eq!(result.render(), r#"{"hello":"from-a"}"#);

    // Node B's ModuleManager-owned module actually executed it.
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // Node B's governance judged it — with the caller's trace and the remote
    // source context. Request-side (A) and execution-side (B) both consulted
    // the canonical governance hook; neither side grew a mesh policy engine.
    let judgements = probe_b.seen.lock().unwrap().clone();
    assert_eq!(judgements.len(), 1);
    assert_eq!(judgements[0].0, ECHO_CAP);
    assert!(
        matches!(&judgements[0].1, ActionSource::Remote { node } if *node == node_a_id),
        "execution-side governance must see the remote source: {:?}",
        judgements[0].1
    );
    assert_eq!(judgements[0].2, trace, "TraceId crosses the hop unchanged");

    // And the local selection still executes locally — the network changed
    // only where execution happened.
    let local_err = node_a
        .invoke_tool(
            &RouteSelection::Local,
            &CapabilityId::new(ECHO_CAP).unwrap(),
            ToolCall {
                id: "call_local_1".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({}),
            },
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await;
    assert!(
        local_err.is_err(),
        "A has no local tool.mesh.echo and must say so"
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1, "B ran exactly once");
}

#[tokio::test]
async fn a_remote_call_keeps_the_canonical_capability_id_end_to_end() {
    let runs = Arc::new(AtomicUsize::new(0));
    let mut policy_b = PermissionPolicy::new();
    policy_b.grant(Permission::ExecuteTool(ECHO_CAP.to_string()));
    let runtime_b = Arc::new(
        Runtime::builder()
            .with_module(echo_module(Arc::clone(&runs)))
            .with_governance(Arc::new(PermissionGovernanceHook::new(policy_b)))
            .build()
            .await
            .unwrap(),
    );
    let runtime_a = Arc::new(Runtime::builder().build().await.unwrap());

    let node_a_id = NodeId::new("node-a").unwrap();
    let node_b_id = NodeId::new("node-b").unwrap();
    let node_a = MeshNode::new(
        MeshNodeConfig::new(node_a_id.clone(), "laptop")
            .with_trust(TrustPolicy::new().with_trusted(node_b_id.clone(), "memory://node-b")),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_a))) as Arc<dyn MeshCapabilityExecutor>,
        system_clock(),
    );
    let node_b = MeshNode::new(
        MeshNodeConfig::new(node_b_id.clone(), "workstation")
            .with_trust(TrustPolicy::new().with_trusted(node_a_id.clone(), "memory://node-a")),
        Arc::new(runtime_mesh_executor(Arc::clone(&runtime_b))) as Arc<dyn MeshCapabilityExecutor>,
        system_clock(),
    );

    let (link_a, link_b) = apeireth_mesh::linked_pair();
    let (ra, rb) = tokio::join!(
        node_a.connect(Arc::new(link_a), node_b_id.clone()),
        node_b.accept(Arc::new(link_b)),
    );
    ra.unwrap();
    rb.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    // Node B's registry proves the canonical dispatch went through the
    // ModuleManager: the owner of `tool.mesh.echo` is `builtin.mesh-test`, and
    // the tool resolves by its canonical CapabilityId.
    let owner = runtime_b
        .module_manager()
        .capabilities()
        .owner(&CapabilityId::new(ECHO_CAP).unwrap())
        .unwrap()
        .clone();
    assert_eq!(owner, ModuleId::new(TEST_MODULE).unwrap());

    let result = node_a
        .invoke_tool(
            &RouteSelection::Remote(node_b_id.clone()),
            &CapabilityId::new(ECHO_CAP).unwrap(),
            ToolCall {
                id: "call_id_check".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "n": 1 }),
            },
            InvokeMeta::new(SessionId::new(), TraceId::new(), 1),
        )
        .await
        .unwrap();
    assert_eq!(result.render(), r#"{"n":1}"#);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}
