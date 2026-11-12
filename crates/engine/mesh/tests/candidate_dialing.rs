//! Authenticated candidate dialing.
//!
//! A candidate is *where* to dial; *who* answers is proved by the handshake
//! above the transport boundary. These tests pin down the dial *plan* — order,
//! per-attempt deadline, short-circuit, typed failure — with a scripted
//! transport. Nothing here is NAT evidence.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_mesh::identity::NodeIdentity;
use apeireth_mesh::relay::{now_ms, signed_presence_with_candidates};
use apeireth_mesh::{
    dial_plan, linked_pair, MeshConnection, MeshTransport, MeshTransportError, PeerConnector,
    Strategy,
};
use apeireth_mesh_protocol::rendezvous::{
    canonicalize_candidates, EndpointCandidate, EndpointCandidateKind, VerifiedPresence,
};
use apeireth_mesh_protocol::NodeId;
use async_trait::async_trait;

const SHORT: Duration = Duration::from_millis(150);

/// A transport that reaches exactly the endpoints it is told to reach, stalls
/// on the ones it is told to stall, and records every attempt in order.
struct Scripted {
    reachable: Vec<&'static str>,
    stall: Vec<&'static str>,
    attempts: Mutex<Vec<String>>,
}

impl Scripted {
    fn new(reachable: &[&'static str], stall: &[&'static str]) -> Arc<Self> {
        Arc::new(Self {
            reachable: reachable.to_vec(),
            stall: stall.to_vec(),
            attempts: Mutex::new(Vec::new()),
        })
    }

    fn attempts(&self) -> Vec<String> {
        self.attempts.lock().unwrap().clone()
    }
}

#[async_trait]
impl MeshTransport for Scripted {
    async fn dial(&self, endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        self.attempts.lock().unwrap().push(endpoint.to_string());
        if self.stall.iter().any(|stalled| *stalled == endpoint) {
            // Far longer than any deadline under test: the dialer must abandon
            // this candidate rather than wait it out.
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        if self
            .reachable
            .iter()
            .any(|reachable| *reachable == endpoint)
        {
            let (ours, _theirs) = linked_pair();
            return Ok(Arc::new(ours) as Arc<dyn MeshConnection>);
        }
        Err(MeshTransportError::Io("scripted: unreachable".to_string()))
    }
}

#[tokio::test]
async fn the_first_reachable_candidate_wins_and_later_ones_are_never_dialed() {
    let transport = Scripted::new(&["b"], &[]);
    let plan = vec!["a".to_string(), "b".to_string(), "c".to_string()];

    assert!(transport.dial_candidates(&plan, SHORT).await.is_ok());
    assert_eq!(
        transport.attempts(),
        vec!["a".to_string(), "b".to_string()],
        "success must stop the plan, so c is never dialed"
    );
}

#[tokio::test]
async fn a_plan_where_nothing_answers_fails_after_trying_all_of_it() {
    let transport = Scripted::new(&[], &[]);
    let plan = vec!["a".to_string(), "b".to_string()];

    assert!(transport.dial_candidates(&plan, SHORT).await.is_err());
    assert_eq!(transport.attempts(), plan);
}

#[tokio::test]
async fn an_empty_plan_is_a_typed_error_and_dials_nothing() {
    let transport = Scripted::new(&[], &[]);

    assert!(transport.dial_candidates(&[], SHORT).await.is_err());
    assert!(transport.attempts().is_empty());
}

#[tokio::test]
async fn a_stalled_candidate_is_abandoned_and_the_next_one_still_runs() {
    let transport = Scripted::new(&["b"], &["a"]);
    let plan = vec!["a".to_string(), "b".to_string()];

    assert!(transport.dial_candidates(&plan, SHORT).await.is_ok());
    assert_eq!(
        transport.attempts(),
        plan,
        "a candidate that never answers must not stall the whole attempt"
    );
}

#[tokio::test]
async fn the_strategy_order_still_decides_when_a_plan_is_involved() {
    let direct = Scripted::new(&[], &[]);
    let relay = Scripted::new(&["b"], &[]);
    let connector = PeerConnector::new(vec![
        Strategy {
            name: "direct",
            transport: Arc::clone(&direct) as Arc<dyn MeshTransport>,
        },
        Strategy {
            name: "relay",
            transport: Arc::clone(&relay) as Arc<dyn MeshTransport>,
        },
    ]);
    let plan = vec!["a".to_string(), "b".to_string()];

    assert!(connector.dial_candidates(&plan, SHORT).await.is_ok());
    assert_eq!(
        direct.attempts(),
        plan,
        "direct gets every candidate before relay is considered"
    );
    assert_eq!(relay.attempts(), plan);
}

#[test]
fn the_dial_plan_is_host_first_and_deduplicated() {
    let identity = NodeIdentity::generate().unwrap();
    let host = EndpointCandidate::new(
        EndpointCandidateKind::Host,
        "192.168.1.9:51820".parse().unwrap(),
    )
    .unwrap();
    let reflexive = EndpointCandidate::new(
        EndpointCandidateKind::ServerReflexive,
        "203.0.113.5:51820".parse().unwrap(),
    )
    .unwrap();

    // Deliberately out of order, with a duplicate: canonicalization is what
    // makes the plan deterministic and duplicate-free.
    let candidates = canonicalize_candidates(vec![reflexive, host.clone(), host]).unwrap();
    let record = signed_presence_with_candidates(
        &identity,
        NodeId::new("node-b").unwrap(),
        "relay.example".to_string(),
        7,
        60_000,
        true,
        candidates,
    )
    .unwrap();
    let verified = VerifiedPresence::verify(record, now_ms(), "relay.example").unwrap();

    assert_eq!(
        dial_plan(&verified),
        vec![
            "192.168.1.9:51820".to_string(),
            "203.0.113.5:51820".to_string()
        ],
        "host first, one entry per distinct candidate"
    );
}
