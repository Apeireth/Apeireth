//! `apeireth mesh …` — the minimal enrollment surface (Phase 2B).
//!
//! ```text
//! apeireth mesh discover              opt-in LAN scan (explicit command = consent)
//! apeireth mesh pair <endpoint> <node>
//! apeireth mesh peers                 list persisted KnownPeers
//! apeireth mesh forget <node>         remove a peer's trust (never your own key)
//! ```
//!
//! The CLI is an adapter over the pairing/discovery state machines — those
//! stay headless-friendly for future desktop/mobile UIs. Confirmation **fails
//! closed**: no interactive answer means rejection, never a default yes.

use std::sync::Arc;

use apeireth_core::kernel::NodeId;
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::{
    pair_outbound, ConfirmDecision, DiscoveryBackend, FakeDiscoveryBackend, MdnsDiscovery,
    MeshDiscoveryService, PairingOptions, TrustStore,
};
use apeireth_mesh_quic::{QuicMeshTransport, QuicTransportConfig};

/// Filesystem locations the mesh CLI persists to.
#[derive(Debug, Clone)]
pub struct MeshPaths {
    /// Node identity key file (private key; 0600 on Unix).
    pub identity: String,
    /// Stable local node id file.
    pub node_id: String,
    /// Known-peer trust store file.
    pub trust: String,
}

impl Default for MeshPaths {
    fn default() -> Self {
        Self {
            identity: "mesh-identity.key".to_string(),
            node_id: "mesh-node.id".to_string(),
            trust: "mesh-trust.json".to_string(),
        }
    }
}

/// Load the stable local node id, generating and persisting on first use.
pub fn load_or_generate_node_id(path: &str) -> Result<NodeId, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let text = text.trim();
            if text.is_empty() {
                return Err("node id file is empty".to_string());
            }
            NodeId::new(text).map_err(|e| e.to_string())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let node = NodeId::generate();
            std::fs::write(path, node.as_str()).map_err(|e| e.to_string())?;
            Ok(node)
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Run one `mesh` subcommand; the returned text is the command's output.
pub async fn dispatch_mesh(args: &[String], paths: &MeshPaths) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("peers") => list_peers(paths),
        Some("forget") => forget_peer(paths, args.get(1)),
        Some("discover") => discover(paths).await,
        Some("pair") => pair(paths, args.get(1), args.get(2)).await,
        _ => Err(
            "usage: apeireth mesh <discover|pair <endpoint> <node>|peers|forget <node>>"
                .to_string(),
        ),
    }
}

fn list_peers(paths: &MeshPaths) -> Result<String, String> {
    let store = TrustStore::load_or_new(&paths.trust).map_err(|e| e.to_string())?;
    let peers = store.known_peers();
    if peers.is_empty() {
        return Ok("no paired peers".to_string());
    }
    let mut out = String::new();
    for peer in peers {
        out.push_str(&format!(
            "{}  endpoint={}  fingerprint={}  trust={:?}\n",
            peer.node,
            peer.endpoint,
            peer.key_fingerprint
                .map(|fp| fp.to_string())
                .unwrap_or_else(|| "-".to_string()),
            peer.trust
        ));
    }
    Ok(out)
}

fn forget_peer(paths: &MeshPaths, node: Option<&String>) -> Result<String, String> {
    let node = node.ok_or_else(|| "usage: apeireth mesh forget <node>".to_string())?;
    let node = NodeId::new(node).map_err(|e| e.to_string())?;
    let store = TrustStore::load_or_new(&paths.trust).map_err(|e| e.to_string())?;
    if store.forget(&node).map_err(|e| e.to_string())? {
        Ok(format!("forgot {node} (local identity untouched)"))
    } else {
        Err(format!("{node} is not a paired peer"))
    }
}

/// Opt-in LAN scan: running this command *is* the consent to browse. Device
/// advertisement is a separate, equally explicit opt-in (never automatic).
async fn discover(paths: &MeshPaths) -> Result<String, String> {
    let my_node = load_or_generate_node_id(&paths.node_id)?;
    let backend = MdnsDiscovery::browser("apeireth-cli");
    let service = MeshDiscoveryService::new(my_node, backend.clone() as Arc<dyn DiscoveryBackend>);
    backend.set_sink(service.observation_sink());
    service.start().await.map_err(|e| e)?;

    // Bounded observation window, then a snapshot. No hard-coded sleep
    // gambling on multicast timing: the window is explicit.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let peers = service.snapshot();
    service.stop().await.ok();

    if peers.is_empty() {
        return Ok("no peers discovered on the local network".to_string());
    }
    let mut out = String::new();
    for peer in peers {
        out.push_str(&format!(
            "{}  endpoint={}  fingerprint-hint={}  paired={}  last-seen={}\n",
            peer.node_hint
                .map(|node| node.to_string())
                .unwrap_or_else(|| "-".to_string()),
            peer.endpoint,
            peer.advertised_fingerprint
                .map(|fp| fp.to_string())
                .unwrap_or_else(|| "-".to_string()),
            "no (discovery never grants trust)",
            peer.last_seen_ms,
        ));
    }
    Ok(out)
}

async fn pair(
    paths: &MeshPaths,
    endpoint: Option<&String>,
    node: Option<&String>,
) -> Result<String, String> {
    let endpoint =
        endpoint.ok_or_else(|| "usage: apeireth mesh pair <endpoint> <node>".to_string())?;
    let node = node.ok_or_else(|| "usage: apeireth mesh pair <endpoint> <node>".to_string())?;
    let node = NodeId::new(node).map_err(|e| e.to_string())?;

    let identity = NodeIdentity::load_or_generate(&paths.identity).map_err(|e| e.to_string())?;
    let my_node = load_or_generate_node_id(&paths.node_id)?;
    let store = TrustStore::load_or_new(&paths.trust).map_err(|e| e.to_string())?;

    // NodeIdentity is deliberately not Clone (it holds a private key): the
    // transport gets its own handle loaded from the same file.
    let transport_identity = NodeIdentity::load(&paths.identity).map_err(|e| e.to_string())?;
    let transport = QuicMeshTransport::new_for_pairing(
        "127.0.0.1:0"
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?,
        Arc::new(transport_identity),
        QuicTransportConfig::default(),
    )
    .map_err(|e| e.to_string())?;
    let link = transport
        .connect(endpoint)
        .await
        .map_err(|e| e.to_string())?;

    let paired = pair_outbound(
        link,
        &identity,
        &my_node,
        node,
        endpoint.clone(),
        &store,
        confirm_on_stdin,
        PairingOptions::default(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(format!(
        "paired {} (fingerprint {})",
        paired.node, paired.key_fingerprint
    ))
}

/// Interactive confirmation. **Fails closed**: anything other than an
/// explicit `y`/`yes` — including EOF and non-interactive stdin — rejects.
fn confirm_on_stdin(sas: &str) -> ConfirmDecision {
    println!("Pairing code: {sas}");
    println!("Verify the same code is shown on the other device.");
    print!("Confirm? [y/N] ");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    match std::io::stdin().read_line(&mut answer) {
        Ok(0) | Err(_) => ConfirmDecision::Reject,
        Ok(_) => {
            let answer = answer.trim().to_ascii_lowercase();
            if answer == "y" || answer == "yes" {
                ConfirmDecision::Confirm
            } else {
                ConfirmDecision::Reject
            }
        }
    }
}

// Keep the fake backend name referenced so the discovery test surface and the
// CLI build against one module set.
#[allow(dead_code)]
fn _fake_backend_available() -> FakeDiscoveryBackend {
    FakeDiscoveryBackend::default()
}
