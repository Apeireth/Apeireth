//! Simultaneous open, and the convergence of the connections it produces.
//!
//! Two peers that both hold each other's current reachability, and both have a
//! dual-role endpoint, will each dial *and* accept. That can leave two
//! authenticated QUIC connections between the same pair. This suite proves that
//! both ends independently keep the *same* one and close the other.
//!
//! Loopback only. This is convergence plumbing — not NAT evidence.

use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::{keeps_connection, Direction};
use apeireth_mesh_protocol::NodeId;
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::QuicPeerEndpoint;

/// One dual-role endpoint that pins exactly `trusted`.
fn endpoint(identity: &Arc<NodeIdentity>, trusted: &NodeIdentity) -> Arc<QuicPeerEndpoint> {
    Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(identity),
            &[trusted.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn two_simultaneous_connections_converge_to_one_on_both_ends() {
    let a_id = Arc::new(NodeIdentity::generate().unwrap());
    let b_id = Arc::new(NodeIdentity::generate().unwrap());
    // The tie-break identity is the *authenticated* one: in a real session this
    // is the NodeId proved by the handshake, not anything the endpoint carries.
    let (a_node, b_node) = (
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
    );
    let a = endpoint(&a_id, &b_id);
    let b = endpoint(&b_id, &a_id);
    let (a_addr, b_addr) = (
        a.local_addr().unwrap().to_string(),
        b.local_addr().unwrap().to_string(),
    );

    // Both peers dial and accept at the same time, on their one endpoint.
    let a_dial = tokio::spawn({
        let a = Arc::clone(&a);
        async move { a.connect(&b_addr).await }
    });
    let b_dial = tokio::spawn({
        let b = Arc::clone(&b);
        async move { b.connect(&a_addr).await }
    });
    let a_accept = tokio::spawn({
        let a = Arc::clone(&a);
        async move { a.accept().await }
    });
    let b_accept = tokio::spawn({
        let b = Arc::clone(&b);
        async move { b.accept().await }
    });

    let a_out = a_dial.await.unwrap().expect("A outbound");
    let b_in = b_accept.await.unwrap().expect("B inbound");
    let b_out = b_dial.await.unwrap().expect("B outbound");
    let a_in = a_accept.await.unwrap().expect("A inbound");

    // Two physical connections exist at once. This is the race.
    assert!(!a_out.is_closed() && !a_in.is_closed());
    assert!(!b_out.is_closed() && !b_in.is_closed());

    // Each end picks a survivor using only stable identity + observed direction.
    let a_keeps_outbound = keeps_connection(&a_node, &b_node, Direction::Outbound);
    let b_keeps_inbound = keeps_connection(&b_node, &a_node, Direction::Inbound);
    assert_eq!(
        a_keeps_outbound, b_keeps_inbound,
        "both ends must name the same physical connection as the survivor"
    );
    assert!(
        a_keeps_outbound,
        "in this pair the lower identity dialed, so that connection wins"
    );

    let (a_kept, a_dropped) = if a_keeps_outbound {
        (&a_out, &a_in)
    } else {
        (&a_in, &a_out)
    };
    let (b_kept, b_dropped) = if b_keeps_inbound {
        (&b_in, &b_out)
    } else {
        (&b_out, &b_in)
    };

    a_dropped.close();
    b_dropped.close();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(a_dropped.is_closed(), "the loser closes on A's side");
    assert!(b_dropped.is_closed(), "the loser closes on B's side");
    assert!(!a_kept.is_closed(), "the survivor stays open on A's side");
    assert!(!b_kept.is_closed(), "the survivor stays open on B's side");

    a.close();
    b.close();
}

#[tokio::test]
async fn a_one_sided_late_start_still_converges() {
    // B does not dial at all: only one physical connection can exist, and the
    // rule must still name it rather than drop the only link there is.
    let a_id = Arc::new(NodeIdentity::generate().unwrap());
    let b_id = Arc::new(NodeIdentity::generate().unwrap());
    let (a_node, b_node) = (
        NodeId::new("node-a").unwrap(),
        NodeId::new("node-b").unwrap(),
    );
    let a = endpoint(&a_id, &b_id);
    let b = endpoint(&b_id, &a_id);
    let b_addr = b.local_addr().unwrap().to_string();

    let accepting = tokio::spawn({
        let b = Arc::clone(&b);
        async move { b.accept().await }
    });
    let a_out = a.connect(&b_addr).await.expect("A dials B");
    let b_in = tokio::time::timeout(Duration::from_secs(10), accepting)
        .await
        .expect("accept settles")
        .expect("no panic")
        .expect("B accepts");

    assert!(keeps_connection(&a_node, &b_node, Direction::Outbound));
    assert!(!a_out.is_closed() && !b_in.is_closed());
    a.close();
    b.close();
}
