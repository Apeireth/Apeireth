//! One high-level request, and the whole simultaneous-open dance underneath it.
//!
//! Nothing in this test calls `connect` on the other peer by hand. The test
//! states the intent once — "A wants B" — and the coordinator, the driver, and
//! the convergence rule do the rest: both peers dial, both accept, and however
//! many authenticated connections that produces collapses to exactly one.
//!
//! Loopback only. This proves the coordination path, not NAT traversal.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::punch::{punch, PunchCoordinator};
use apeireth_mesh::relay::{now_ms, signed_presence_with_candidates};
use apeireth_mesh::{Direction, MeshListener, MeshTransport};
use apeireth_mesh_protocol::rendezvous::{
    canonicalize_candidates, EndpointCandidate, EndpointCandidateKind, VerifiedPresence,
};
use apeireth_mesh_protocol::NodeId;
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::QuicPeerEndpoint;

const WINDOW_MS: u64 = 10_000;

/// A real signed-and-verified presence advertising one host candidate.
fn presence_for(
    identity: &NodeIdentity,
    node: &NodeId,
    session_id: u64,
    addr: SocketAddr,
) -> VerifiedPresence {
    let candidates = canonicalize_candidates(vec![EndpointCandidate::new(
        EndpointCandidateKind::Host,
        addr,
    )
    .unwrap()])
    .unwrap();
    let record = signed_presence_with_candidates(
        identity,
        node.clone(),
        "relay.example".to_string(),
        session_id,
        600_000,
        true,
        candidates,
    )
    .unwrap();
    VerifiedPresence::verify(record, now_ms(), "relay.example").unwrap()
}

#[tokio::test]
async fn one_request_makes_both_peers_punch_and_leaves_one_connection() {
    let a_id = Arc::new(NodeIdentity::generate().unwrap());
    let b_id = Arc::new(NodeIdentity::generate().unwrap());
    let (a_node, b_node) = (
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
    );

    // Each peer has one dual-role endpoint, and pins exactly the other.
    let a_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&a_id),
            &[b_id.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b_peer = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&b_id),
            &[a_id.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );

    // Each holds the other's current, verified reachability.
    let a_presence = presence_for(&a_id, &a_node, 11, a_peer.local_addr().unwrap());
    let b_presence = presence_for(&b_id, &b_node, 22, b_peer.local_addr().unwrap());

    let coordinator = Arc::new(PunchCoordinator::new(8));

    // B is the receiving side: it does not dial anything, it waits to be told.
    let b_waiting = {
        let coordinator = Arc::clone(&coordinator);
        let (a, b) = (a_node.clone(), b_node.clone());
        tokio::spawn(async move {
            coordinator
                .wait_for_session(&a, &b, Duration::from_secs(5))
                .await
        })
    };

    // The one high-level act: A asks for B.
    let now = now_ms();
    let requested = coordinator
        .request(&a_node, &b_node, 11, 22, now, WINDOW_MS)
        .expect("punch requested");

    let b_session = b_waiting
        .await
        .unwrap()
        .expect("B must be woken by the coordination")
        .oriented_for(&b_node);
    assert_eq!(
        requested.id(),
        b_session.id(),
        "both peers must act on the same attempt"
    );
    assert_eq!(b_session.local_presence(), 22);
    assert_eq!(b_session.remote_presence(), 11);

    // Both peers act on their own orientation, on their own one endpoint.
    let a_dial: Arc<dyn MeshTransport> = a_peer.clone();
    let a_listen: Arc<dyn MeshListener> = a_peer.clone();
    let b_dial: Arc<dyn MeshTransport> = b_peer.clone();
    let b_listen: Arc<dyn MeshListener> = b_peer.clone();

    let (a_result, b_result) = tokio::join!(
        punch(&requested, a_dial, a_listen, &b_presence, now),
        punch(&b_session, b_dial, b_listen, &a_presence, now),
    );
    let a_outcome = a_result.expect("A's punch produces a connection");
    let b_outcome = b_result.expect("B's punch produces a connection");

    // A is the lower identity, so A keeps its outbound and B keeps its inbound:
    // two ends of one physical connection, chosen independently.
    assert_eq!(a_outcome.kept_direction, Direction::Outbound);
    assert_eq!(b_outcome.kept_direction, Direction::Inbound);
    assert!(!a_outcome.kept.is_closed(), "A's survivor stays open");
    assert!(!b_outcome.kept.is_closed(), "B's survivor stays open");
    assert!(a_outcome.physical_links >= 1);
    assert!(b_outcome.physical_links >= 1);

    a_outcome.kept.close();
    b_outcome.kept.close();
    a_peer.close();
    b_peer.close();
}
