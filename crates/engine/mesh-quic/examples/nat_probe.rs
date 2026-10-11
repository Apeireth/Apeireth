//! Namespace NAT probe: one binary, three roles, used by
//! `scripts/test-mesh-nat.sh`.
//!
//! `observer` mode answers endpoint-observation probes (what the rendezvous
//! server does in production). `peer` mode binds the socket a peer would use,
//! asks the observer what public address it sees, and then hands that same
//! socket to a dual-role QUIC endpoint — proving the observed port is the port
//! QUIC would use for both dialing and accepting.
//!
//! `punch` mode is the traversal half: it runs the **production** connection
//! path end to end between two namespaced peers —
//!
//! ```text
//! bind one UDP socket -> observe (server-reflexive candidate)
//!   -> sign presence with that candidate -> exchange presences
//!   -> MeshNode::connect_punched (PunchCoordinator -> simultaneous open
//!      over the one socket -> pinned QUIC handshake -> convergence)
//!   -> remote tool.mesh.echo invocation, result content checked
//! ```
//!
//! There is no test-only shortcut in that chain: the coordination state, the
//! punch driver, the handshake and the convergence are the ones
//! `MeshNode::connect_punched` drives in production. What the harness still
//! does *not* prove by itself is that any real NAT behaves like the nft
//! masquerade in the script — that is what running it on Linux establishes.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::{system_clock, CapabilityId, Clock, PluginId, TraceId};
use apeireth_governance::GovernanceHook;
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::relay::signed_presence_with_candidates;
use apeireth_mesh::{
    InvokeMeta, ManagerExecutor, MeshNode, MeshNodeConfig, PluginManagerView, RouteSelection,
    TrustPolicy,
};
use apeireth_mesh_protocol::observe::{decode_probe, encode_reply, OBSERVE_DATAGRAM_BYTES};
use apeireth_mesh_protocol::rendezvous::{
    canonicalize_candidates, EndpointCandidate, EndpointCandidateKind, PresenceRecord,
    VerifiedPresence,
};
use apeireth_mesh_protocol::KeyFingerprint;
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::PreparedMeshEndpoint;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManager, PluginManifest, ToolCapability,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;

/// Rendezvous audience the harness signs presence for.
const AUDIENCE: &str = "apeireth-nat-harness";
const ECHO_CAP: &str = "tool.mesh.echo";
const TEST_MODULE: &str = "builtin.mesh-test";
/// The simultaneous-open window: both peers must punch inside it.
const WINDOW_MS: u64 = 10_000;
/// How long a peer waits for the other side's presence file to appear.
const PRESENCE_WAIT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("observer") => observer(&args[1]).await,
        Some("peer") => peer(&args[1], &args[2]).await,
        Some("punch") => {
            let code = punch(&args[1], &args[2], &args[3], &args[4], &args[5]).await;
            std::process::exit(code);
        }
        _ => {
            eprintln!("usage: nat_probe observer <bind-addr>");
            eprintln!("       nat_probe peer <bind-addr> <observer-addr>");
            eprintln!("       nat_probe punch <bind-addr> <observer-addr> <workdir> <role> <peer-role>");
            std::process::exit(2);
        }
    }
}

/// Answer observation probes forever, printing the address it bound.
async fn observer(bind: &str) {
    let bind: SocketAddr = bind.parse().expect("observer bind address");
    let socket = tokio::net::UdpSocket::bind(bind)
        .await
        .expect("bind observer");
    println!(
        "OBSERVER={}",
        socket.local_addr().expect("observer local addr")
    );
    let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
    loop {
        let Ok((n, from)) = socket.recv_from(&mut buf).await else {
            return;
        };
        let Some(nonce) = decode_probe(&buf[..n]) else {
            continue;
        };
        let _ = socket.send_to(&encode_reply(&nonce, from), from).await;
    }
}

/// Observe from the socket, then hand that same socket to a dual-role endpoint.
async fn peer(bind: &str, observer: &str) {
    let bind: SocketAddr = bind.parse().expect("peer bind address");
    let observer: SocketAddr = observer.parse().expect("observer address");
    let identity = Arc::new(NodeIdentity::generate().expect("identity"));

    let prepared = PreparedMeshEndpoint::bind(bind).await.expect("bind");
    let local = prepared.local_addr().expect("local addr");
    println!("LOCAL={local}");

    match prepared.observe(observer, Duration::from_secs(3)).await {
        Ok(observed) => println!("OBSERVED={observed}"),
        Err(error) => println!("OBSERVED_FAILED={error}"),
    }

    let endpoint = prepared
        .into_quic_peer_endpoint(Arc::clone(&identity), &[], QuicTransportConfig::default())
        .expect("dual-role endpoint");
    println!(
        "QUIC_LOCAL={}",
        endpoint.local_addr().expect("quic local addr")
    );
    endpoint.close();
}

// ---------------------------------------------------------------------------
// punch mode: the traversal half, through the production connection path.
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// One node's local world for the harness: the canonical manager owning
/// `builtin.mesh-test` / `tool.mesh.echo`, behind the mesh execution port.
struct Holder {
    manager: Mutex<PluginManager>,
    governance: Arc<dyn GovernanceHook>,
    clock: Arc<dyn Clock>,
    echo_runs: Arc<AtomicUsize>,
}

impl PluginManagerView for Holder {
    fn with_manager<R>(&self, f: impl FnOnce(&PluginManager) -> R) -> R {
        f(&self.manager.lock().unwrap())
    }
    fn governance(&self) -> &Arc<dyn GovernanceHook> {
        &self.governance
    }
    fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

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

struct EchoModule {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

#[async_trait]
impl Plugin for EchoModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
    async fn initialize(&self, _ctx: &PluginContext) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
}

async fn node_with_echo(id: &str, peer_pin: KeyFingerprint, peer: &str) -> (Arc<MeshNode>, Arc<Holder>) {
    let runs = Arc::new(AtomicUsize::new(0));
    let manifest = PluginManifest::new(
        PluginId::new(TEST_MODULE).unwrap(),
        "1.0.0",
        "builtin mesh test module",
    )
    .declare_capability(
        CapabilityId::new(ECHO_CAP).unwrap(),
        CapabilityKind::Tool,
        "echo across the mesh",
    )
    .unwrap();
    let module: Arc<dyn Plugin> = Arc::new(EchoModule {
        manifest,
        tools: vec![Arc::new(EchoTool {
            id: CapabilityId::new(ECHO_CAP).unwrap(),
            runs: Arc::clone(&runs),
        }) as Arc<dyn ToolCapability>],
    });
    let mut manager = PluginManager::new();
    manager.register(module).unwrap();
    let ctx = PluginContext::new(
        system_clock(),
        Arc::new(apeireth_plugin::NoCredentials),
        TraceId::new(),
    );
    manager.start_all(&ctx).await.unwrap();

    let holder = Arc::new(Holder {
        manager: Mutex::new(manager),
        governance: Arc::new(apeireth_governance::AllowAll),
        clock: system_clock(),
        echo_runs: runs,
    });
    let node_id = apeireth_mesh::NodeId::new(id).unwrap();
    let trust = TrustPolicy::new().with_trusted_pinned(
        apeireth_mesh::NodeId::new(peer).unwrap(),
        "harness",
        peer_pin,
    );
    let node = MeshNode::new(
        MeshNodeConfig::new(node_id, format!("node {id}")).with_trust(trust),
        Arc::new(ManagerExecutor::new(Arc::clone(&holder))),
        system_clock(),
    );
    (node, holder)
}

/// Write `record` to `workdir/presence-<role>.json` atomically, so a peer
/// polling for the file never reads a half-written record.
fn write_presence(workdir: &str, role: &str, record: &PresenceRecord) -> std::io::Result<()> {
    let path = std::path::Path::new(workdir).join(format!("presence-{role}.json"));
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Wait for the peer's presence file and verify it. Fail closed: a missing,
/// malformed, wrongly-audienced, expired, or badly-signed record is an error,
/// never a dial target.
fn wait_peer_presence(workdir: &str, peer_role: &str) -> Result<VerifiedPresence, String> {
    let path = std::path::Path::new(workdir).join(format!("presence-{peer_role}.json"));
    let deadline = std::time::Instant::now() + PRESENCE_WAIT;
    while std::time::Instant::now() < deadline {
        if path.exists() {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let record: PresenceRecord =
                serde_json::from_str(&text).map_err(|e| format!("peer presence is not a valid record: {e}"))?;
            return VerifiedPresence::verify(record, now_ms(), AUDIENCE)
                .map_err(|e| format!("peer presence failed verification: {e}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "peer presence file {} did not appear within {:?}",
        path.display(),
        PRESENCE_WAIT
    ))
}

/// The traversal half: observe, sign, exchange, punch, authenticate, invoke.
///
/// Returns the process exit code: 0 only when every step succeeded.
async fn punch(bind: &str, observer: &str, workdir: &str, role: &str, peer_role: &str) -> i32 {
    let bind: SocketAddr = bind.parse().expect("peer bind address");
    let observer: SocketAddr = observer.parse().expect("observer address");
    let identity = Arc::new(NodeIdentity::generate().expect("identity"));
    let my_node = apeireth_mesh::NodeId::new(role).unwrap();
    let peer_node = apeireth_mesh::NodeId::new(peer_role).unwrap();

    // Bind once. This exact socket observes, punches, and then serves QUIC.
    let prepared = match PreparedMeshEndpoint::bind(bind).await {
        Ok(prepared) => prepared,
        Err(error) => {
            println!("PUNCH_RESULT=fail:bind:{error}");
            return 1;
        }
    };
    println!("LOCAL={}", prepared.local_addr().unwrap());

    // The server-reflexive candidate: what an outsider sees for THIS socket.
    let observed = match prepared.observe(observer, Duration::from_secs(3)).await {
        Ok(observed) => observed,
        Err(error) => {
            println!("PUNCH_RESULT=fail:observe:{error}");
            return 1;
        }
    };
    println!("OBSERVED={observed}");

    // Sign it. The signature proves key possession; it grants no trust, and
    // the address still has to survive the pinned handshake below.
    let session_id = now_ms();
    let candidates = match canonicalize_candidates(vec![EndpointCandidate::new(
        EndpointCandidateKind::ServerReflexive,
        observed,
    )
    .unwrap()]) {
        Ok(candidates) => candidates,
        Err(error) => {
            println!("PUNCH_RESULT=fail:candidate:{error}");
            return 1;
        }
    };
    let record = match signed_presence_with_candidates(
        &identity,
        my_node.clone(),
        AUDIENCE.to_string(),
        session_id,
        120_000,
        true,
        candidates,
    ) {
        Ok(record) => record,
        Err(error) => {
            println!("PUNCH_RESULT=fail:sign:{error}");
            return 1;
        }
    };
    if let Err(error) = write_presence(workdir, role, &record) {
        println!("PUNCH_RESULT=fail:presence-write:{error}");
        return 1;
    }

    // The peer's current, verified reachability — the only thing the dial
    // plan is ever built from.
    let peer_presence = match wait_peer_presence(workdir, peer_role) {
        Ok(presence) => presence,
        Err(error) => {
            println!("PUNCH_RESULT=fail:peer-presence:{error}");
            return 1;
        }
    };
    let peer_pin = KeyFingerprint::of_public_key(&peer_presence.record().public_key);
    println!("PEER_PRESENCE_SESSION={}", peer_presence.session_id());

    // The one socket becomes the dual-role QUIC endpoint: nothing re-binds.
    let endpoint = match prepared.into_quic_peer_endpoint(
        Arc::clone(&identity),
        std::slice::from_ref(&peer_pin),
        QuicTransportConfig::default(),
    ) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            println!("PUNCH_RESULT=fail:endpoint:{error}");
            return 1;
        }
    };
    println!("QUIC_LOCAL={}", endpoint.local_addr().unwrap());

    let (node, holder) = node_with_echo(role, peer_pin, peer_role).await;

    // The production entry point: trust gate, presence gate, bounded
    // simultaneous open on the one socket, pinned handshake, convergence.
    let connected = node
        .connect_punched(
            peer_node.clone(),
            &peer_presence,
            session_id,
            Arc::new(endpoint),
            WINDOW_MS,
        )
        .await;
    match connected {
        Ok(peer) => {
            println!("PUNCH_RESULT=ok");
            println!("CONNECTED_PEER={peer}");
        }
        Err(error) => {
            println!("PUNCH_RESULT=fail:punch:{error}");
            return 1;
        }
    }

    let links: Vec<_> = node
        .peers()
        .into_iter()
        .filter(|(id, _, _)| id == &peer_node)
        .collect();
    println!("LINK_COUNT={}", links.len());
    println!(
        "PEER_STATE={:?}",
        links.first().map(|(_, state, _)| state).unwrap_or(&apeireth_mesh::PeerState::Closed)
    );

    // One side drives a real remote invocation; the other side simply stays
    // up long enough for it to land. The result *content* is the check.
    if role == "node-a" {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let result = node
            .invoke_tool(
                &RouteSelection::Remote(peer_node),
                &CapabilityId::new(ECHO_CAP).unwrap(),
                ToolCall {
                    id: "call-nat-e2e".into(),
                    name: "mesh_echo".into(),
                    arguments: serde_json::json!({ "e2e": "nat-harness-punch" }),
                },
                InvokeMeta::new(apeireth_core::kernel::SessionId::new(), TraceId::new(), 1),
            )
            .await;
        match result {
            Ok(outcome) => {
                println!("E2E_RENDER={}", outcome.render());
                println!("E2E_EXEC_COUNT={}", holder.echo_runs.load(Ordering::SeqCst));
                if outcome.render() == r#"{"e2e":"nat-harness-punch"}"# {
                    println!("E2E_RESULT=ok");
                    return 0;
                }
                println!("E2E_RESULT=fail:content");
                1
            }
            Err(error) => {
                println!("E2E_RESULT=fail:invoke:{error}");
                1
            }
        }
    } else {
        // The receiving side: keep the process (and the link) up while the
        // invocation lands, then report the connection evidence.
        tokio::time::sleep(Duration::from_secs(2)).await;
        let executed = holder.echo_runs.load(Ordering::SeqCst);
        println!("E2E_EXEC_COUNT={executed}");
        println!("E2E_RESULT=ok");
        0
    }
}
