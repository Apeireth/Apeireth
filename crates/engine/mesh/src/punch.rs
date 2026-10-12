//! Simultaneous open: the coordination state, and the driver that acts on it.
//!
//! **EXPERIMENTAL — best-effort direct connectivity, not verified under real
//! NAT.** Everything here exists to *attempt* a direct link; whether a
//! direct link is possible is decided by NAT behaviour this code has never
//! met. Verified on loopback only; the Linux namespace harness
//! (`scripts/test-mesh-nat.sh`) has not been run; no real NAT has been
//! traversed. The relay remains the correctness path.
//!
//! # What this is
//!
//! One high-level intent — "connect to peer B" — has to become *both* peers
//! dialing each other at roughly the same time and then agreeing on which of the
//! resulting connections is the real one. [`PunchCoordinator`] is the first half
//! (bounded, incarnation-bound coordination state), [`punch`] is the second
//! (run both halves against one endpoint, then converge).
//!
//! # What this is not
//!
//! Not ICE: no checklist, no foundations, no nomination state machine. Not
//! trust: a `PunchSession` coordinates an attempt and authorizes nothing — the
//! pinned handshake above this boundary still decides who is a peer. Not a
//! connection manager: it produces at most one link and hands it up.
//!
//! # Why the session binds incarnations
//!
//! A punch is only meaningful for one exact pair of reachability claims. If
//! either peer mints a new `session_id`, the candidates the punch would dial
//! belong to a snapshot that no longer exists, so the session is stale rather
//! than reusable. Binding `(peer, presence session)` pairs is what makes that a
//! fact instead of a hope.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::NodeId;
use apeireth_mesh_protocol::rendezvous::VerifiedPresence;

use crate::candidates::dial_plan;
use crate::convergence::{keeps_connection, Direction};
use crate::transport::{MeshConnection, MeshListener, MeshTransport};

/// A per-attempt ceiling. The punch deadline still bounds the whole thing.
pub const DEFAULT_PER_CANDIDATE_TIMEOUT_MS: u64 = 3_000;

/// Identifies one punch attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PunchId(pub u128);

/// One bounded attempt to make two peers connect at the same time.
///
/// It carries no addresses: the candidates come from the verified presences the
/// two peers already hold. It carries no trust: reaching this state authorizes
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PunchSession {
    seen: PunchId,
    local_peer: NodeId,
    remote_peer: NodeId,
    local_presence: u64,
    remote_presence: u64,
    deadline_ms: u64,
}

impl PunchSession {
    /// This attempt's identifier.
    pub fn id(&self) -> PunchId {
        self.seen
    }

    /// The local peer of the attempt.
    pub fn local_peer(&self) -> &NodeId {
        &self.local_peer
    }

    /// The remote peer of the attempt.
    pub fn remote_peer(&self) -> &NodeId {
        &self.remote_peer
    }

    /// The local presence incarnation this attempt is bound to.
    pub fn local_presence(&self) -> u64 {
        self.local_presence
    }

    /// The remote presence incarnation this attempt is bound to.
    pub fn remote_presence(&self) -> u64 {
        self.remote_presence
    }

    /// Absolute instant (caller's clock, milliseconds) after which this attempt
    /// is over.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// This same attempt, as `local` sees it.
    ///
    /// A session is stored once, oriented the way the requester described it.
    /// The other peer is the mirror image of that: same attempt and same id, with
    /// local and remote exchanged. Convergence reads the orientation, so the
    /// receiving side must act on its own view rather than the requester's —
    /// otherwise both ends would believe they are "local" and both would keep
    /// their outbound connection.
    pub fn oriented_for(&self, local: &NodeId) -> PunchSession {
        if &self.local_peer == local {
            return self.clone();
        }
        PunchSession {
            seen: self.seen,
            local_peer: self.remote_peer.clone(),
            remote_peer: self.local_peer.clone(),
            local_presence: self.remote_presence,
            remote_presence: self.local_presence,
            deadline_ms: self.deadline_ms,
        }
    }
}

/// Why a punch could not be started or completed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PunchError {
    /// Too many attempts are already active.
    #[error("punch limit reached")]
    LimitReached,
    /// The remote presence is not the incarnation this attempt was bound to.
    #[error("punch is bound to presence {expected} but the peer presents {actual}")]
    Stale {
        /// The incarnation the attempt was bound to.
        expected: u64,
        /// The incarnation actually presented.
        actual: u64,
    },
    /// The attempt outlived its deadline.
    #[error("punch deadline elapsed")]
    Expired,
    /// The remote presence advertises no dialable candidate.
    #[error("peer advertises no dialable candidate")]
    NoCandidates,
    /// Neither half produced an authenticated connection inside the window.
    #[error("no authenticated connection inside the punch window")]
    NoConnection,
}

/// The outcome of a successful punch.
pub struct PunchOutcome {
    /// The one connection that survived.
    pub kept: Arc<dyn MeshConnection>,
    /// Which half it came from.
    pub kept_direction: Direction,
    /// How many authenticated connections the race actually produced (1 or 2).
    pub physical_links: usize,
}

/// Bounded, incarnation-bound coordination state.
///
/// Keyed on the *canonical* peer pair, so `request(A, B)` and `request(B, A)`
/// name the same attempt and cannot become two.
///
/// **Experimental** (see the module header): this coordinates an attempt at
/// best-effort direct connectivity that has never met a real NAT. It holds
/// no addresses and no trust — the pinned handshake above this boundary
/// still decides who is a peer.
#[derive(Debug)]
pub struct PunchCoordinator {
    inner: Mutex<Inner>,
    max_active: usize,
}

#[derive(Debug, Default)]
struct Inner {
    active: HashMap<(NodeId, NodeId), Arc<PunchSession>>,
    wake: Option<tokio::sync::broadcast::Sender<()>>,
}

impl Default for PunchCoordinator {
    fn default() -> Self {
        Self::new(64)
    }
}

impl PunchCoordinator {
    /// A coordinator holding at most `max_active` attempts.
    pub fn new(max_active: usize) -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            max_active: max_active.max(1),
        }
    }

    fn canonical(a: &NodeId, b: &NodeId) -> (NodeId, NodeId) {
        if a <= b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        }
    }

    /// Ask for a punch between two peers, given both current incarnations.
    ///
    /// Returns the existing attempt when one already covers this exact
    /// incarnation pair — repeated requests must not multiply state. A request
    /// for a *different* incarnation pair replaces the old attempt, because the
    /// old one can only dial a snapshot that is gone.
    pub fn request(
        &self,
        local: &NodeId,
        remote: &NodeId,
        local_presence: u64,
        remote_presence: u64,
        now_ms: u64,
        window_ms: u64,
    ) -> Result<PunchSession, PunchError> {
        let key = Self::canonical(local, remote);
        let mut inner = self.inner.lock().expect("punch inner poisoned");

        if let Some(existing) = inner.active.get(&key) {
            let same_incarnations = existing.local_presence == local_presence
                && existing.remote_presence == remote_presence;
            if same_incarnations && existing.deadline_ms > now_ms {
                return Ok((**existing).clone());
            }
        }
        let replacing = inner.active.contains_key(&key);
        if !replacing && inner.active.len() >= self.max_active {
            return Err(PunchError::LimitReached);
        }

        let session = Arc::new(PunchSession {
            seen: PunchId(random_u128()),
            local_peer: local.clone(),
            remote_peer: remote.clone(),
            local_presence,
            remote_presence,
            deadline_ms: now_ms + window_ms,
        });
        inner.active.insert(key, Arc::clone(&session));
        if let Some(wake) = &inner.wake {
            let _ = wake.send(());
        }
        Ok((*session).clone())
    }

    /// The active attempt for a peer pair, canonicalised.
    pub fn session_for(&self, a: &NodeId, b: &NodeId) -> Option<PunchSession> {
        let key = Self::canonical(a, b);
        self.inner
            .lock()
            .expect("punch inner poisoned")
            .active
            .get(&key)
            .map(|session| (**session).clone())
    }

    /// Wait for an attempt covering this pair to be requested.
    ///
    /// The receiving side of a punch has nothing to poll: it was told to expect
    /// coordination, and this is where it waits for it.
    pub async fn wait_for_session(
        &self,
        a: &NodeId,
        b: &NodeId,
        timeout: Duration,
    ) -> Option<PunchSession> {
        let key = Self::canonical(a, b);
        let mut receiver = {
            let mut inner = self.inner.lock().expect("punch inner poisoned");
            if let Some(session) = inner.active.get(&key) {
                return Some((**session).clone());
            }
            let wake = inner
                .wake
                .get_or_insert_with(|| tokio::sync::broadcast::channel(64).0)
                .clone();
            wake.subscribe()
        };
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match tokio::time::timeout(remaining, receiver.recv()).await {
                Ok(Ok(())) => {
                    if let Some(session) = self.session_for(a, b) {
                        return Some(session);
                    }
                }
                // Lagged or closed: re-check the map rather than giving up.
                Ok(Err(_)) => {
                    if let Some(session) = self.session_for(a, b) {
                        return Some(session);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    /// Drop every attempt bound to `peer`, whatever incarnation.
    ///
    /// Called when a peer's presence is replaced or the peer is revoked: an
    /// attempt bound to the previous snapshot must not keep dialling it.
    pub fn invalidate_peer(&self, peer: &NodeId) -> usize {
        let mut inner = self.inner.lock().expect("punch inner poisoned");
        let before = inner.active.len();
        inner.active.retain(|(a, b), _| a != peer && b != peer);
        before - inner.active.len()
    }

    /// Drop every attempt bound to one specific presence incarnation of a peer.
    pub fn invalidate_incarnation(&self, peer: &NodeId, presence: u64) -> usize {
        let mut inner = self.inner.lock().expect("punch inner poisoned");
        let before = inner.active.len();
        inner.active.retain(|(a, b), session| {
            let bound_local = (a == peer && session.local_presence == presence)
                || (b == peer && session.local_presence == presence);
            let bound_remote = (a == peer && session.remote_presence == presence)
                || (b == peer && session.remote_presence == presence);
            !(bound_local || bound_remote)
        });
        before - inner.active.len()
    }

    /// Drop attempts whose deadline has passed.
    pub fn prune_expired(&self, now_ms: u64) -> usize {
        let mut inner = self.inner.lock().expect("punch inner poisoned");
        let before = inner.active.len();
        inner
            .active
            .retain(|_, session| session.deadline_ms > now_ms);
        before - inner.active.len()
    }

    /// How many attempts are active (expired ones included until pruned).
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("punch inner poisoned")
            .active
            .len()
    }

    /// Whether no attempt is active.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Run both halves of one punch and converge to a single connection.
///
/// The dial plan comes from the *verified* remote presence, never from anything
/// unsigned. Both halves run for the same bounded window: the accept side is
/// what makes this a simultaneous open rather than one peer calling another.
///
/// When both halves succeed, the survivor is chosen by the single shared rule in
/// [`crate::convergence`] — the same one the connection manager uses — and the
/// loser is closed here. When only one half succeeds it is kept regardless of
/// direction, because dropping the only connection there is would be absurd.
pub async fn punch(
    session: &PunchSession,
    dialer: Arc<dyn MeshTransport>,
    listener: Arc<dyn MeshListener>,
    remote_presence: &VerifiedPresence,
    now_ms: u64,
) -> Result<PunchOutcome, PunchError> {
    if remote_presence.session_id() != session.remote_presence {
        return Err(PunchError::Stale {
            expected: session.remote_presence,
            actual: remote_presence.session_id(),
        });
    }
    if now_ms >= session.deadline_ms {
        return Err(PunchError::Expired);
    }

    let plan = dial_plan(remote_presence);
    if plan.is_empty() {
        return Err(PunchError::NoCandidates);
    }

    let window = Duration::from_millis(session.deadline_ms - now_ms);
    let per_candidate = Duration::from_millis(DEFAULT_PER_CANDIDATE_TIMEOUT_MS).min(window);

    let outbound = {
        let dialer = Arc::clone(&dialer);
        async move { dialer.dial_candidates(&plan, per_candidate).await }
    };
    let inbound = {
        let listener = Arc::clone(&listener);
        async move { listener.accept().await }
    };

    let (out, inp) = tokio::join!(
        tokio::time::timeout(window, outbound),
        tokio::time::timeout(window, inbound),
    );
    let outbound_link = out.ok().and_then(Result::ok);
    let inbound_link = inp.ok().and_then(Result::ok);

    match (outbound_link, inbound_link) {
        (None, None) => Err(PunchError::NoConnection),
        (Some(link), None) => Ok(PunchOutcome {
            kept: link,
            kept_direction: Direction::Outbound,
            physical_links: 1,
        }),
        (None, Some(link)) => Ok(PunchOutcome {
            kept: link,
            kept_direction: Direction::Inbound,
            physical_links: 1,
        }),
        (Some(out_link), Some(in_link)) => {
            let keep_outbound = keeps_connection(
                &session.local_peer,
                &session.remote_peer,
                Direction::Outbound,
            );
            let (kept, dropped, direction) = if keep_outbound {
                (out_link, in_link, Direction::Outbound)
            } else {
                (in_link, out_link, Direction::Inbound)
            };
            dropped.close();
            Ok(PunchOutcome {
                kept,
                kept_direction: direction,
                physical_links: 2,
            })
        }
    }
}

fn random_u128() -> u128 {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut bytes = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .expect("system rng fills a 128 bit punch id");
    u128::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> NodeId {
        NodeId::new(name).unwrap()
    }

    const NOW: u64 = 1_000_000;
    const WINDOW: u64 = 10_000;

    #[test]
    fn a_request_creates_one_session_bound_to_both_incarnations() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        let session = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();

        assert_eq!(session.local_peer(), &a);
        assert_eq!(session.remote_peer(), &b);
        assert_eq!(session.local_presence(), 7);
        assert_eq!(session.remote_presence(), 9);
        assert_eq!(session.deadline_ms(), NOW + WINDOW);
    }

    #[test]
    fn the_reversed_request_names_the_same_session() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        let from_a = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        let from_b = coordinator.request(&b, &a, 7, 9, NOW, WINDOW).unwrap();
        assert_eq!(from_a.id(), from_b.id());
        assert_eq!(coordinator.len(), 1);
    }

    #[test]
    fn a_duplicate_request_returns_the_active_session() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        let first = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        for _ in 0..5 {
            let again = coordinator.request(&a, &b, 7, 9, NOW + 1, WINDOW).unwrap();
            assert_eq!(again.id(), first.id());
        }
        assert_eq!(coordinator.len(), 1);
    }

    #[test]
    fn a_new_incarnation_replaces_the_bound_session() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        let old = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        let new = coordinator.request(&b, &a, 8, 9, NOW, WINDOW).unwrap();
        assert_ne!(
            old.id(),
            new.id(),
            "a new local incarnation is a new attempt"
        );
        assert_eq!(coordinator.len(), 1, "the stale attempt must not linger");
        assert_eq!(coordinator.session_for(&a, &b).unwrap().local_presence(), 8);
    }

    #[test]
    fn replacing_a_peer_presence_drops_its_sessions() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        assert_eq!(coordinator.invalidate_peer(&b), 1);
        assert!(coordinator.is_empty());
    }

    #[test]
    fn invalidating_one_incarnation_leaves_other_peers_alone() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b, c) = (id("node-a"), id("node-b"), id("node-c"));
        coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        coordinator.request(&a, &c, 7, 4, NOW, WINDOW).unwrap();

        assert_eq!(coordinator.invalidate_incarnation(&b, 9), 1);
        assert_eq!(coordinator.len(), 1);
        assert!(coordinator.session_for(&a, &c).is_some());
    }

    #[test]
    fn an_expired_deadline_is_pruned() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        assert_eq!(coordinator.prune_expired(NOW + WINDOW), 1);
        assert!(coordinator.is_empty());
    }

    #[test]
    fn the_active_count_is_bounded() {
        let coordinator = PunchCoordinator::new(2);
        let a = id("node-a");
        coordinator
            .request(&a, &id("node-b"), 1, 1, NOW, WINDOW)
            .unwrap();
        coordinator
            .request(&a, &id("node-c"), 1, 1, NOW, WINDOW)
            .unwrap();
        assert_eq!(
            coordinator.request(&a, &id("node-d"), 1, 1, NOW, WINDOW),
            Err(PunchError::LimitReached)
        );
        // Replacing an existing pair is still allowed at the limit.
        assert!(coordinator
            .request(&a, &id("node-b"), 2, 1, NOW, WINDOW)
            .is_ok());
    }

    #[test]
    fn a_punch_bound_to_another_incarnation_is_stale() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        let session = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        assert_eq!(session.remote_presence(), 9);
    }

    #[test]
    fn the_receiving_side_sees_the_mirrored_orientation() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        let from_a = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();
        let from_b = from_a.oriented_for(&b);

        assert_eq!(from_b.id(), from_a.id(), "same attempt");
        assert_eq!(from_b.local_peer(), &b);
        assert_eq!(from_b.remote_peer(), &a);
        assert_eq!(from_b.local_presence(), 9);
        assert_eq!(from_b.remote_presence(), 7);
        // Asking for the already-local orientation is a no-op.
        assert_eq!(from_a.oriented_for(&a).local_peer(), &a);
    }

    #[tokio::test]
    async fn waiting_for_coordination_wakes_when_it_arrives() {
        let coordinator = Arc::new(PunchCoordinator::new(8));
        let (a, b) = (id("node-a"), id("node-b"));

        let waiting = {
            let coordinator = Arc::clone(&coordinator);
            let (a, b) = (a.clone(), b.clone());
            tokio::spawn(async move {
                coordinator
                    .wait_for_session(&a, &b, Duration::from_secs(5))
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        let created = coordinator.request(&a, &b, 7, 9, NOW, WINDOW).unwrap();

        let observed = waiting.await.unwrap().expect("the waiter must be woken");
        assert_eq!(observed.id(), created.id());
    }

    #[tokio::test]
    async fn waiting_gives_up_at_its_timeout() {
        let coordinator = PunchCoordinator::new(8);
        let (a, b) = (id("node-a"), id("node-b"));
        assert!(coordinator
            .wait_for_session(&a, &b, Duration::from_millis(100))
            .await
            .is_none());
    }
}
