//! LAN discovery: observation only, never trust.
//!
//! # Discovery tells you a node may exist
//!
//! A [`DiscoveredPeer`] is the record "I saw something claiming to be an
//! Apeireth node here" — nothing more. It carries *hints* (a claimed
//! [`NodeId`], an advertised fingerprint) that help a user recognise a device
//! and match what the pairing ceremony later proves. No discovery record can
//! become a [`crate::KnownPeer`]; only the pairing ceremony's bilateral user
//! confirmation does that.
//!
//! # Outside the runtime entirely
//!
//! Discovered peers never enter the peer/connection list, never produce
//! capability projections, never reach the tool or provider route tables, and
//! never initiate capability RPC. Discovery is connectivity assistance for
//! enrollment; it is not a route source.
//!
//! # Lifecycle and hygiene
//!
//! A [`MeshDiscoveryService`] owns its backend task: `start` / `stop` /
//! `snapshot`, and dropping it releases everything. Records dedup
//! deterministically (by node hint + endpoint), the local node is never listed
//! (no pairing with yourself), entries that stop being announced become stale
//! and disappear from the snapshot — while persisted trust remains untouched.
//! Advertisement is **opt-in**; installing Apeireth never broadcasts device
//! presence by default.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::NodeId;
use apeireth_mesh_protocol::KeyFingerprint;
use serde::{Deserialize, Serialize};

/// How a peer was observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMethod {
    /// mDNS/DNS-SD on the local network.
    Mdns,
    /// A test or manual backend.
    Synthetic,
}

/// "I saw something claiming to be a node here." Hints only — never trust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    /// The claimed node identity. A hint: anyone can claim anything in an
    /// unauthenticated broadcast; the pairing ceremony proves possession.
    pub node_hint: Option<NodeId>,
    /// Where it claims to listen.
    pub endpoint: String,
    /// The claimed key fingerprint. A hint for UI matching only — the
    /// ceremony binds the real key.
    pub advertised_fingerprint: Option<KeyFingerprint>,
    /// Display name chosen by the peer (never defaulted to an OS username).
    pub display_name: Option<String>,
    /// How it was observed.
    pub discovered_via: DiscoveryMethod,
    /// Last observation time (epoch millis, observer clock).
    pub last_seen_ms: u64,
}

/// Deterministic dedup key: node hint + endpoint. Twelve mDNS events for one
/// device collapse to one snapshot row.
impl DiscoveredPeer {
    fn key(&self) -> (Option<String>, String) {
        (
            self.node_hint.as_ref().map(|node| node.to_string()),
            self.endpoint.clone(),
        )
    }
}

/// The pluggable observation backend. Real mDNS is one implementation; tests
/// use [`FakeDiscoveryBackend`] so CI never depends on multicast.
#[async_trait::async_trait]
pub trait DiscoveryBackend: Send + Sync {
    /// Begin observing. Observations are delivered by the backend through the
    /// sink given at construction.
    async fn start(&self) -> Result<(), String>;
    /// Stop observing and release sockets/tasks.
    async fn stop(&self) -> Result<(), String>;
}

/// Sink a backend pushes observations into.
pub type DiscoverySink = Arc<dyn Fn(DiscoveredPeer) + Send + Sync>;

/// Deterministic backend for tests and embedding without multicast.
#[derive(Default)]
pub struct FakeDiscoveryBackend {
    sink: Mutex<Option<DiscoverySink>>,
    running: Mutex<bool>,
}

impl FakeDiscoveryBackend {
    /// An idle fake backend; attach a sink with [`Self::set_sink`].
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Install the observation sink.
    pub fn set_sink(&self, sink: DiscoverySink) {
        *self.sink.lock().unwrap_or_else(|p| p.into_inner()) = Some(sink);
    }

    /// Inject one observation, as if the network reported it.
    pub fn emit(&self, peer: DiscoveredPeer) {
        if let Some(sink) = self.sink.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            sink(peer);
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryBackend for FakeDiscoveryBackend {
    async fn start(&self) -> Result<(), String> {
        *self.running.lock().unwrap_or_else(|p| p.into_inner()) = true;
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        *self.running.lock().unwrap_or_else(|p| p.into_inner()) = false;
        Ok(())
    }
}

/// The lifecycle owner of one discovery session. Opt-in: nothing observes
/// until [`MeshDiscoveryService::start`] is called, and [`Self::stop`] (or
/// drop) releases the backend.
pub struct MeshDiscoveryService {
    local_node: NodeId,
    backend: Arc<dyn DiscoveryBackend>,
    peers: Mutex<BTreeMap<(Option<String>, String), DiscoveredPeer>>,
    running: Mutex<bool>,
}

impl MeshDiscoveryService {
    /// A service over `backend`, excluding `local_node` from its own
    /// observations (never pair with yourself).
    pub fn new(local_node: NodeId, backend: Arc<dyn DiscoveryBackend>) -> Arc<Self> {
        Arc::new(Self {
            local_node,
            backend,
            peers: Mutex::new(BTreeMap::new()),
            running: Mutex::new(false),
        })
    }

    /// The sink backends push into (self-filtered, deduped, snapshot-stamped).
    pub fn observation_sink(self: &Arc<Self>) -> DiscoverySink {
        let service = Arc::clone(self);
        Arc::new(move |peer: DiscoveredPeer| service.observe(peer))
    }

    /// Record one observation. Self-observations are dropped; repeats dedup
    /// by `(node hint, endpoint)` and refresh `last_seen_ms`.
    pub fn observe(&self, peer: DiscoveredPeer) {
        if peer.node_hint.as_ref() == Some(&self.local_node) {
            return; // never list yourself
        }
        let mut peers = self.peers.lock().unwrap_or_else(|p| p.into_inner());
        peers.insert(peer.key(), peer);
    }

    /// The current snapshot, in stable key order.
    pub fn snapshot(&self) -> Vec<DiscoveredPeer> {
        self.peers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Remove entries not seen since `cutoff_ms`. Discovery presence is
    /// ephemeral; persisted trust ([`crate::TrustStore`]) is a different
    /// store and is never touched here.
    pub fn prune_stale(&self, cutoff_ms: u64) {
        self.peers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|_, peer| peer.last_seen_ms >= cutoff_ms);
    }

    /// Start observing.
    pub async fn start(&self) -> Result<(), String> {
        self.backend.start().await?;
        *self.running.lock().unwrap_or_else(|p| p.into_inner()) = true;
        Ok(())
    }

    /// Stop observing and release the backend.
    pub async fn stop(&self) -> Result<(), String> {
        self.backend.stop().await?;
        *self.running.lock().unwrap_or_else(|p| p.into_inner()) = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(node: &str, endpoint: &str, seen: u64) -> DiscoveredPeer {
        DiscoveredPeer {
            node_hint: Some(NodeId::new(node).unwrap()),
            endpoint: endpoint.to_string(),
            advertised_fingerprint: Some(KeyFingerprint::of_public_key(&[1; 32])),
            display_name: Some("Apeireth node".into()),
            discovered_via: DiscoveryMethod::Synthetic,
            last_seen_ms: seen,
        }
    }

    #[tokio::test]
    async fn duplicates_collapse_self_is_filtered_and_stale_prunes() {
        let backend = FakeDiscoveryBackend::new();
        let service = MeshDiscoveryService::new(NodeId::new("node-a").unwrap(), backend.clone());
        backend.set_sink(service.observation_sink());
        backend.start().await.unwrap();

        // Twelve events for one device → one row (§20).
        for _ in 0..12 {
            backend.emit(peer("node-b", "10.0.0.2:7443", 100));
        }
        assert_eq!(service.snapshot().len(), 1);

        // Yourself never appears (§21).
        backend.emit(peer("node-a", "10.0.0.1:7443", 100));
        assert_eq!(service.snapshot().len(), 1);

        // Stale entries leave the snapshot; trust is a different store (§22).
        service.prune_stale(200);
        assert!(service.snapshot().is_empty());
        backend.stop().await.unwrap();
    }
}
