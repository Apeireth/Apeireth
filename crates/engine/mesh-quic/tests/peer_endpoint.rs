//! A QUIC endpoint in both roles on one socket.
//!
//! Simultaneous open needs a peer that can send the first packet and answer
//! one, on the same mapping. These tests prove the two roles really do share
//! one endpoint and that unpinned peers are still refused. Loopback only: this
//! is plumbing, not NAT evidence.

use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::QuicPeerEndpoint;

#[tokio::test]
async fn a_dial_and_an_accept_meet_on_one_endpoint() {
    let a_id = Arc::new(NodeIdentity::generate().unwrap());
    let b_id = Arc::new(NodeIdentity::generate().unwrap());
    // Each trusts exactly the other.
    let a = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&a_id),
            &[b_id.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&b_id),
            &[a_id.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b_addr = b.local_addr().unwrap().to_string();

    let accepting = tokio::spawn({
        let b = Arc::clone(&b);
        async move { b.accept().await }
    });
    let dialed = a
        .connect(&b_addr)
        .await
        .expect("A dials B over its own socket");
    let accepted = tokio::time::timeout(Duration::from_secs(10), accepting)
        .await
        .expect("the accept task finishes")
        .expect("no panic")
        .expect("B accepts the inbound connection");

    assert!(!dialed.is_closed());
    assert!(!accepted.is_closed());
    a.close();
    b.close();
}

#[tokio::test]
async fn an_unpinned_peer_cannot_complete_the_handshake() {
    let a_id = Arc::new(NodeIdentity::generate().unwrap());
    let b_id = Arc::new(NodeIdentity::generate().unwrap());
    let stranger = NodeIdentity::generate().unwrap();
    // A pins somebody else's key, so B is not a peer A will talk to.
    let a = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&a_id),
            &[stranger.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let b = Arc::new(
        QuicPeerEndpoint::bind(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&b_id),
            &[a_id.fingerprint().clone()],
            QuicTransportConfig::default(),
        )
        .unwrap(),
    );
    let a_addr = a.local_addr().unwrap().to_string();

    let accepting = tokio::spawn({
        let a = Arc::clone(&a);
        async move { a.accept().await }
    });
    let dialed = b.connect(&a_addr).await;

    // The refusal lands on the server side. In TLS 1.3 the client can believe
    // the handshake finished before the server has checked its certificate, so
    // a client-side `Ok` is not evidence of admission. That asymmetry is
    // exactly why the mesh-level trust check above this boundary is mandatory:
    // an accepted link still has to prove a pinned fingerprint before it
    // becomes a peer connection.
    let accepted = tokio::time::timeout(Duration::from_secs(10), accepting)
        .await
        .expect("the accepting side settles")
        .expect("no panic");
    assert!(
        accepted.is_err(),
        "the server must refuse a client whose key it does not pin"
    );
    drop(dialed);
    a.close();
    b.close();
}

#[tokio::test]
async fn closing_the_endpoint_ends_the_accept_loop() {
    let identity = Arc::new(NodeIdentity::generate().unwrap());
    let peer = QuicPeerEndpoint::bind(
        "127.0.0.1:0".parse().unwrap(),
        identity,
        &[],
        QuicTransportConfig::default(),
    )
    .unwrap();
    peer.close();

    let accepted = tokio::time::timeout(Duration::from_secs(5), peer.accept()).await;
    assert!(
        accepted.is_ok(),
        "accept must return, not hang, after close"
    );
    assert!(
        accepted.unwrap().is_err(),
        "a closed endpoint accepts nothing"
    );
}
