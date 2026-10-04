//! Namespace NAT probe: one binary, two roles, used by
//! `scripts/test-mesh-nat.sh`.
//!
//! `observer` mode answers endpoint-observation probes (what the rendezvous
//! server does in production). `peer` mode binds the socket a peer would use,
//! asks the observer what public address it sees, and then hands that same
//! socket to a dual-role QUIC endpoint — proving the observed port is the port
//! QUIC would use for both dialing and accepting.
//!
//! It deliberately stops at observation. The punch/convergence step needs a
//! coordination driver that reads two `VerifiedPresence` records; that is not
//! wired yet, and this binary refuses to pretend otherwise.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh_protocol::observe::{
    decode_probe, encode_reply, OBSERVE_DATAGRAM_BYTES,
};
use apeireth_mesh_quic::config::QuicTransportConfig;
use apeireth_mesh_quic::endpoint::PreparedMeshEndpoint;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("observer") => observer(&args[1]).await,
        Some("peer") => peer(&args[1], &args[2]).await,
        _ => {
            eprintln!("usage: nat_probe observer <bind-addr>");
            eprintln!("       nat_probe peer <bind-addr> <observer-addr>");
            std::process::exit(2);
        }
    }
}

/// Answer observation probes forever, printing the address it bound.
async fn observer(bind: &str) {
    let bind: SocketAddr = bind.parse().expect("observer bind address");
    let socket = tokio::net::UdpSocket::bind(bind).await.expect("bind observer");
    println!("OBSERVER={}", socket.local_addr().expect("observer local addr"));
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
    println!("QUIC_LOCAL={}", endpoint.local_addr().expect("quic local addr"));
    endpoint.close();
}
