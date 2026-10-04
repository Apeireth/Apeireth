//! Bind-once endpoint preparation.
//!
//! The one invariant this suite exists for: the address an outside observer
//! reports must belong to the *same* UDP socket that QUIC will dial from.
//!
//! Everything here is loopback, where every address is directly reachable.
//! These tests prove socket identity and plumbing. They are **not** evidence of
//! NAT traversal.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh_protocol::observe::{decode_probe, encode_reply, OBSERVE_DATAGRAM_BYTES};
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::{ObserveError, PreparedMeshEndpoint};

/// A minimal observation responder: answer every well-formed probe with the
/// source address it was seen from. Mirrors what the rendezvous server does.
async fn spawn_observer() -> SocketAddr {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    tokio::spawn(async move {
        let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
        loop {
            let Ok((n, from)) = socket.recv_from(&mut buf).await else {
                return;
            };
            let Some(nonce) = decode_probe(&buf[..n]) else {
                continue;
            };
            let reply = encode_reply(&nonce, from);
            let _ = socket.send_to(&reply, from).await;
        }
    });
    addr
}

#[tokio::test]
async fn the_observed_address_belongs_to_the_prepared_socket() {
    let observer = spawn_observer().await;
    let prepared = PreparedMeshEndpoint::bind("127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let local = prepared.local_addr().unwrap();
    let observed = prepared
        .observe(observer, Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(
        observed, local,
        "the observer must report this socket's own mapping"
    );
}

#[tokio::test]
async fn quinn_keeps_the_exact_port_observation_reported() {
    let observer = spawn_observer().await;
    let identity = Arc::new(NodeIdentity::generate().unwrap());
    let prepared = PreparedMeshEndpoint::bind("127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let local = prepared.local_addr().unwrap();
    let observed = prepared
        .observe(observer, Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(observed, local);

    // Handoff: the same socket becomes the QUIC endpoint.
    let transport = prepared
        .into_quic_client(identity, &[], QuicTransportConfig::default())
        .unwrap();
    let quic_local = transport.local_addr().unwrap();
    assert_eq!(
        quic_local, local,
        "QUIC must dial from the port observation reported"
    );
    transport.close();
}

#[tokio::test]
async fn repeated_observation_keeps_one_local_port() {
    let observer = spawn_observer().await;
    let prepared = PreparedMeshEndpoint::bind("127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let first = prepared
        .observe(observer, Duration::from_secs(2))
        .await
        .unwrap();
    let second = prepared
        .observe(observer, Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(
        first, second,
        "a second probe must not open a second socket"
    );
    assert_eq!(first, prepared.local_addr().unwrap());
}

#[tokio::test]
async fn an_unreachable_observer_fails_typed_without_hanging() {
    let prepared = PreparedMeshEndpoint::bind("127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    // Nothing listens here. On Unix the probe is simply dropped; on Windows the
    // ICMP port-unreachable surfaces as a socket error on the next receive.
    // Both are legitimate typed failures; hanging or panicking is not.
    let dead: SocketAddr = "127.0.0.1:9".parse().unwrap();
    let error = prepared
        .observe(dead, Duration::from_millis(400))
        .await
        .unwrap_err();
    assert!(
        matches!(error, ObserveError::Timeout { .. } | ObserveError::Io(_)),
        "unexpected observation error: {error:?}"
    );
}
