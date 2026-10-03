//! Adversarial behaviour of the rendezvous/relay server. The server is
//! untrusted infrastructure, but it must not be trivially knocked over, and
//! one connection must not be able to act in another's name.

use std::sync::Arc;
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::relay::signed_presence;
use apeireth_mesh_protocol::rendezvous::{
    decode_frame, encode_control, encode_relay, Frame, PresenceRecord, RelayCircuitId,
    RelayControl, RelayFrame, RendezvousRequest, RendezvousResponse,
};
use apeireth_mesh_protocol::NodeId;
use apeireth_mesh_rendezvous::{RendezvousRelayServer, ServerConfig};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const AUDIENCE: &str = "relay://hardening";

async fn start(tune: impl FnOnce(&mut ServerConfig)) -> std::net::SocketAddr {
    let mut config = ServerConfig::new("127.0.0.1:0".parse().unwrap(), AUDIENCE);
    tune(&mut config);
    let server = Arc::new(RendezvousRelayServer::bind(config).await.unwrap());
    let addr = server.local_addr();
    tokio::spawn(server.run());
    addr
}

fn node(name: &str) -> NodeId {
    NodeId::new(name).unwrap()
}

struct Client {
    stream: TcpStream,
    buf: Vec<u8>,
    identity: NodeIdentity,
    id: NodeId,
    session: u64,
}

impl Client {
    async fn connect(addr: std::net::SocketAddr, name: &str) -> Self {
        Self::connect_with(addr, name, NodeIdentity::generate().unwrap()).await
    }

    async fn connect_with(addr: std::net::SocketAddr, name: &str, identity: NodeIdentity) -> Self {
        Self {
            stream: TcpStream::connect(addr).await.unwrap(),
            buf: Vec::new(),
            identity,
            id: node(name),
            session: 7,
        }
    }

    fn presence(&self, ttl_ms: u64) -> PresenceRecord {
        signed_presence(
            &self.identity,
            self.id.clone(),
            AUDIENCE.to_string(),
            self.session,
            ttl_ms,
            true,
        )
    }

    async fn send<T: Serialize>(&mut self, value: &T) {
        let json = serde_json::to_vec(value).unwrap();
        self.stream.write_all(&encode_control(&json)).await.unwrap();
    }

    async fn next(&mut self) -> Option<Frame> {
        let mut chunk = [0u8; 4096];
        loop {
            if let Ok(Some((frame, used))) = decode_frame(&self.buf) {
                self.buf.drain(..used);
                return Some(frame);
            }
            match tokio::time::timeout(Duration::from_millis(500), self.stream.read(&mut chunk))
                .await
            {
                Ok(Ok(n)) if n > 0 => self.buf.extend_from_slice(&chunk[..n]),
                _ => return None,
            }
        }
    }

    async fn response(&mut self) -> RendezvousResponse {
        match self.next().await {
            Some(Frame::Control(json)) => serde_json::from_slice(&json).unwrap(),
            other => panic!("expected a rendezvous response, got {other:?}"),
        }
    }

    async fn relay_control(&mut self) -> Option<RelayControl> {
        match self.next().await {
            Some(Frame::Control(json)) => Some(serde_json::from_slice(&json).unwrap()),
            _ => None,
        }
    }

    async fn register(&mut self, presence: PresenceRecord) -> RendezvousResponse {
        self.send(&RendezvousRequest::Register { presence }).await;
        self.response().await
    }

    async fn lookup(&mut self, target: &str) -> Option<PresenceRecord> {
        let request = RendezvousRequest::Lookup {
            querant: self.id.clone(),
            target: node(target),
        };
        self.send(&request).await;
        match self.response().await {
            RendezvousResponse::Presence { presence } => presence,
            other => panic!("unexpected lookup answer {other:?}"),
        }
    }

    async fn is_closed(&mut self) -> bool {
        let mut byte = [0u8; 1];
        matches!(
            tokio::time::timeout(Duration::from_secs(2), self.stream.read(&mut byte)).await,
            Ok(Ok(0) | Err(_))
        )
    }
}

fn is_rejected(response: &RendezvousResponse) -> bool {
    matches!(response, RendezvousResponse::Rejected { .. })
}

#[tokio::test]
async fn a_different_key_cannot_take_over_a_live_node_id() {
    let addr = start(|_| {}).await;
    let mut owner = Client::connect(addr, "node-b").await;
    let presence = owner.presence(60_000);
    assert!(!is_rejected(&owner.register(presence).await));

    let mut squatter = Client::connect(addr, "node-b").await;
    let hostile = squatter.presence(60_000);
    assert!(is_rejected(&squatter.register(hostile).await));

    let found = owner.lookup("node-b").await.unwrap();
    assert_eq!(found.public_key, owner.identity.public_key().to_vec());
}

#[tokio::test]
async fn a_replayed_presence_cannot_rebind_a_node_to_another_connection() {
    let addr = start(|_| {}).await;
    let mut owner = Client::connect(addr, "node-b").await;
    let presence = owner.presence(60_000);
    assert!(!is_rejected(&owner.register(presence.clone()).await));

    let mut replayer = Client::connect(addr, "node-x").await;
    assert!(is_rejected(&replayer.register(presence).await));

    // The owner's own newer heartbeat still works.
    let renewed = owner.presence(90_000);
    assert!(!is_rejected(&owner.register(renewed).await));
}

#[tokio::test]
async fn a_lease_beyond_the_protocol_maximum_is_refused() {
    let addr = start(|_| {}).await;
    let mut client = Client::connect(addr, "node-b").await;
    let greedy = client.presence(24 * 60 * 60 * 1_000);
    assert!(is_rejected(&client.register(greedy).await));
    assert!(client.lookup("node-b").await.is_none());
}

#[tokio::test]
async fn one_connection_cannot_register_several_node_ids() {
    let addr = start(|_| {}).await;
    let mut client = Client::connect(addr, "node-b").await;
    let first = client.presence(60_000);
    assert!(!is_rejected(&client.register(first).await));

    client.id = node("node-other");
    let second = client.presence(60_000);
    assert!(is_rejected(&client.register(second).await));
}

#[tokio::test]
async fn only_the_registering_connection_can_withdraw() {
    let addr = start(|_| {}).await;
    let mut owner = Client::connect(addr, "node-b").await;
    let presence = owner.presence(60_000);
    owner.register(presence).await;

    let mut stranger = Client::connect(addr, "node-c").await;
    stranger
        .send(&RendezvousRequest::Withdraw {
            node_id: node("node-b"),
            session_id: owner.session,
        })
        .await;
    let _ = stranger.response().await;

    assert!(owner.lookup("node-b").await.is_some());
}

#[tokio::test]
async fn a_malformed_frame_drops_the_connection_and_its_registration() {
    let addr = start(|_| {}).await;
    let mut vandal = Client::connect(addr, "node-b").await;
    let presence = vandal.presence(60_000);
    vandal.register(presence).await;
    vandal
        .stream
        .write_all(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
        .await
        .unwrap();
    assert!(vandal.is_closed().await);

    let mut observer = Client::connect(addr, "node-c").await;
    assert!(observer.lookup("node-b").await.is_none());
}

#[tokio::test]
async fn a_connection_that_never_registers_is_dropped() {
    let addr = start(|config| config.registration_timeout_ms = 150).await;
    let mut idle = Client::connect(addr, "node-b").await;
    assert!(idle.is_closed().await);
}

#[tokio::test]
async fn connections_per_ip_are_bounded() {
    let addr = start(|config| config.max_connections_per_ip = 2).await;
    let _first = Client::connect(addr, "node-a").await;
    let _second = Client::connect(addr, "node-b").await;
    let mut third = Client::connect(addr, "node-c").await;
    assert!(third.is_closed().await);
}

/// Register `name` and return its client.
async fn registered(addr: std::net::SocketAddr, name: &str) -> Client {
    let mut client = Client::connect(addr, name).await;
    let presence = client.presence(60_000);
    assert!(!is_rejected(&client.register(presence).await));
    client
}

fn open(client: &Client, circuit: u8, target: &str) -> RelayControl {
    RelayControl::Open {
        circuit_id: RelayCircuitId::from_bytes([circuit; 16]),
        source: client.id.clone(),
        target: node(target),
        source_proof: client.presence(60_000),
    }
}

#[tokio::test]
async fn refused_opens_do_not_consume_the_per_node_circuit_budget() {
    let addr = start(|config| config.max_circuits_per_node = 2).await;
    let mut opener = registered(addr, "node-a").await;
    let mut target = registered(addr, "node-b").await;

    // Opens to an absent node are refused and must leave no accounting behind.
    for circuit in 1..=5u8 {
        let request = open(&opener, circuit, "node-absent");
        opener.send(&request).await;
        assert!(matches!(
            opener.relay_control().await,
            Some(RelayControl::Refused { .. })
        ));
    }

    // The full budget is still available...
    for circuit in 10..=11u8 {
        let request = open(&opener, circuit, "node-b");
        opener.send(&request).await;
        assert!(matches!(
            target.relay_control().await,
            Some(RelayControl::Open { .. })
        ));
    }
    // ...and exactly that budget.
    let request = open(&opener, 12, "node-b");
    opener.send(&request).await;
    assert!(matches!(
        opener.relay_control().await,
        Some(RelayControl::Refused { .. })
    ));
}

#[tokio::test]
async fn an_open_must_come_from_the_connection_registered_as_the_source() {
    let addr = start(|_| {}).await;
    let victim = registered(addr, "node-a").await;
    let _target = registered(addr, "node-b").await;

    // An attacker connection claims to be node-a using node-a's own (replayed)
    // proof, but is not the connection node-a registered on.
    let mut attacker = Client::connect(addr, "node-x").await;
    let request = open(&victim, 1, "node-b");
    attacker.send(&request).await;
    assert!(matches!(
        attacker.relay_control().await,
        Some(RelayControl::Refused { .. })
    ));
}

#[tokio::test]
async fn only_circuit_endpoints_can_put_frames_on_a_circuit() {
    let addr = start(|_| {}).await;
    let mut opener = registered(addr, "node-a").await;
    let mut target = registered(addr, "node-b").await;
    let mut outsider = registered(addr, "node-c").await;

    let circuit_id = RelayCircuitId::from_bytes([9; 16]);
    let request = open(&opener, 9, "node-b");
    opener.send(&request).await;
    assert!(matches!(
        target.relay_control().await,
        Some(RelayControl::Open { .. })
    ));
    let accept = RelayControl::Accept {
        circuit_id,
        target_proof: target.presence(60_000),
    };
    target.send(&accept).await;
    assert!(matches!(
        opener.relay_control().await,
        Some(RelayControl::Accept { .. })
    ));

    // The outsider tries to inject into, and then to close, someone else's circuit.
    let injected = RelayFrame {
        circuit_id,
        payload: b"injected".to_vec(),
    };
    outsider
        .stream
        .write_all(&encode_relay(&injected))
        .await
        .unwrap();
    outsider
        .send(&RelayControl::Close {
            circuit_id,
            reason: "hostile".into(),
        })
        .await;
    assert!(opener.next().await.is_none(), "nothing reaches the opener");
    assert!(target.next().await.is_none(), "nothing reaches the target");

    // The real endpoints are unaffected.
    let genuine = RelayFrame {
        circuit_id,
        payload: b"genuine".to_vec(),
    };
    opener
        .stream
        .write_all(&encode_relay(&genuine))
        .await
        .unwrap();
    match target.next().await {
        Some(Frame::Relay(frame)) => assert_eq!(frame.payload, b"genuine"),
        other => panic!("expected the genuine frame, got {other:?}"),
    }
}
