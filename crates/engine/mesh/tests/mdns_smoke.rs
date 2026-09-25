//! Real mDNS smoke: platform integration, deliberately distinct from the
//! deterministic [`apeireth_mesh::FakeDiscoveryBackend`] semantics tests.
//!
//! Bounded observation window (no sleep-gambling): the advertiser publishes a
//! record and the browser must observe it within the deadline. If the host
//! refuses multicast entirely the failure names the platform limitation — it
//! is never silently ignored.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::NodeId;
use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::{
    AdvertisedPresence, DiscoveredPeer, DiscoveryBackend, MdnsDiscovery, MeshDiscoveryService,
};
use apeireth_mesh_protocol::KeyFingerprint;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mdns_smoke_advertised_presence_is_observed_on_the_local_network() {
    let identity_a = NodeId::new("node-a").unwrap();
    let identity_b = NodeId::new("node-b").unwrap();
    let fingerprint = KeyFingerprint::of_public_key(&[7u8; 32]);

    // Node B advertises (explicit opt-in — this call is the consent).
    let advertiser = MdnsDiscovery::advertiser(
        "smoke-b",
        AdvertisedPresence::generic(
            identity_b.clone(),
            "127.0.0.1:7443".to_string(),
            fingerprint.clone(),
        ),
    );
    advertiser.start().await.expect("advertiser starts");

    // Node A browses and records observations (its own id filters self).
    let browser = MdnsDiscovery::browser("smoke-a");
    let service = MeshDiscoveryService::new(identity_a, browser.clone());
    browser.set_sink(service.observation_sink());
    browser.start().await.expect("browser starts");

    // Bounded observation window with periodic checks.
    let mut observed: Vec<DiscoveredPeer> = Vec::new();
    for _ in 0..25 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        observed = service.snapshot();
        if observed
            .iter()
            .any(|peer| peer.node_hint.as_ref() == Some(&identity_b))
        {
            break;
        }
    }

    browser.stop().await.ok();
    advertiser.stop().await.ok();

    assert!(
        observed
            .iter()
            .any(|peer| peer.node_hint.as_ref() == Some(&identity_b)),
        "PLATFORM LIMITATION if this fails: multicast observation did not \
         arrive within 5s on this host (semantic coverage lives in the \
         FakeDiscoveryBackend tests); observed: {observed:?}"
    );
}
