//! The server's endpoint-observation socket: "what address does the outside
//! world see me at?"
//!
//! A node behind a NAT cannot learn its own public UDP address by reasoning —
//! only by asking someone outside to look. This suite proves the plumbing: a
//! well-formed probe is answered with the source address the server actually
//! observed, the nonce comes back verbatim, garbage is ignored, and the whole
//! facility is off unless it is configured.
//!
//! What this does **not** prove is that a real NAT is traversed. Everything
//! here is loopback, where every address is directly reachable. See
//! `docs/design/mesh-architecture.md` for the honest boundary.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh_protocol::observe::{decode_reply, encode_probe, OBSERVE_DATAGRAM_BYTES};
use apeireth_mesh_rendezvous::{RendezvousRelayServer, ServerConfig};

/// Run a server that answers observation probes; return its observation socket.
async fn start_observing() -> SocketAddr {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap(), "observe://test")
        .with_observe_addr("127.0.0.1:0".parse().unwrap());
    let server = Arc::new(RendezvousRelayServer::bind(config).await.unwrap());
    let observe_addr = server
        .local_observe_addr()
        .await
        .expect("observation is enabled by with_observe_addr");
    tokio::spawn(server.run());
    observe_addr
}

#[tokio::test]
async fn a_probe_is_answered_with_the_address_the_server_observed() {
    let observe_addr = start_observing().await;
    let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let local = client.local_addr().unwrap();
    let nonce = [0x5au8; 16];

    client
        .send_to(&encode_probe(&nonce), observe_addr)
        .await
        .unwrap();

    let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
    let (n, _from) = tokio::time::timeout(Duration::from_secs(5), client.recv_from(&mut buf))
        .await
        .expect("the observer answers within the timeout")
        .unwrap();
    let (echoed, observed) = decode_reply(&buf[..n]).expect("a well-formed reply");

    assert_eq!(echoed, nonce, "the reply must echo the probe's nonce");
    assert_eq!(
        observed, local,
        "the observer must report the source address it actually saw"
    );
}

#[tokio::test]
async fn a_different_client_sees_its_own_address_not_the_first_ones() {
    let observe_addr = start_observing().await;
    let first = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let second = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    assert_ne!(first.local_addr().unwrap(), second.local_addr().unwrap());

    for client in [&first, &second] {
        let nonce = [0x11u8; 16];
        client
            .send_to(&encode_probe(&nonce), observe_addr)
            .await
            .unwrap();
        let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
        let (n, _) = tokio::time::timeout(Duration::from_secs(5), client.recv_from(&mut buf))
            .await
            .expect("answered")
            .unwrap();
        let (_, observed) = decode_reply(&buf[..n]).unwrap();
        assert_eq!(
            observed,
            client.local_addr().unwrap(),
            "each client must be told its own address"
        );
    }
}

#[tokio::test]
async fn a_malformed_datagram_is_never_answered() {
    let observe_addr = start_observing().await;
    let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();

    for garbage in [
        b"not a probe".as_slice(),
        &[0u8; 8],
        &[0u8; OBSERVE_DATAGRAM_BYTES],
    ] {
        client.send_to(garbage, observe_addr).await.unwrap();
    }

    let mut buf = [0u8; 256];
    let answered =
        tokio::time::timeout(Duration::from_millis(300), client.recv_from(&mut buf)).await;
    assert!(
        answered.is_err(),
        "garbage must not elicit a reply — the socket is not a reflector"
    );
}

#[tokio::test]
async fn observation_is_off_unless_it_is_configured() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap(), "observe://test");
    let server = RendezvousRelayServer::bind(config).await.unwrap();
    assert!(
        server.local_observe_addr().await.is_none(),
        "a server without with_observe_addr must not open a UDP socket"
    );
}
