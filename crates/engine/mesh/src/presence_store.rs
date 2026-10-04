//! The canonical answer to "what is this peer's reachable state *right now*".
//!
//! It owns exactly one idea:
//!
//! ```text
//! peer identity -> the current authenticated reachability claim
//! ```
//!
//! and the three transitions that idea implies: replacement, expiry, and
//! incarnation conflict. It owns nothing else — not trust (that is the
//! `TrustStore`), not live links (the connection map), not route selection,
//! not capability ownership, not rendezvous. Keeping those separate is the
//! whole point: reachability is *metadata about where to dial*, and it changing
//! must never silently become "this peer is trusted" or "this session is dead".
//!
//! A stored presence is always a [`VerifiedPresence`], so nothing unverified can
//! enter, and every caller that reads it gets the verified form rather than
//! having to remember to verify.

use std::collections::HashMap;

use apeireth_core::kernel::NodeId;
use apeireth_mesh_protocol::rendezvous::VerifiedPresence;

/// What an upsert did. Callers act on the difference: an insertion has nothing
/// to invalidate, a refresh invalidates nothing, and a replacement invalidates
/// everything derived from the previous incarnation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceUpdate {
    /// The first presence for this peer.
    Inserted {
        /// The incarnation now current.
        session_id: u64,
    },
    /// The same incarnation, re-signed with a later lease. Nothing reachable
    /// changed, so nothing derived from it needs invalidating.
    Refreshed {
        /// The incarnation that was refreshed.
        session_id: u64,
    },
    /// A new incarnation superseded the old one. Every fact derived from
    /// `previous_session_id` — candidates, dial plans, punch sessions,
    /// reconnects — is now stale.
    Replaced {
        /// The incarnation that was current before.
        previous_session_id: u64,
        /// The incarnation now current.
        session_id: u64,
    },
}

/// Why a presence was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PresenceError {
    /// The same incarnation arrived describing *different* reachability.
    ///
    /// An incarnation is a stable snapshot: same `session_id` must mean same
    /// candidates. A peer that wants to move must mint a new incarnation, so
    /// this is refused rather than silently overwritten — otherwise anyone able
    /// to refresh a lease could quietly rewrite where a peer is dialled.
    #[error("presence incarnation {session_id} restated with different reachability")]
    IncarnationConflict {
        /// The contested incarnation.
        session_id: u64,
    },
    /// The presence was already expired at the instant it arrived.
    #[error("presence incarnation {session_id} is already expired")]
    Expired {
        /// The expired incarnation.
        session_id: u64,
    },
    /// A new incarnation that does not advance the lease already stored.
    ///
    /// Leases are monotonic, so a "new" incarnation carrying an expiry no later
    /// than the one it would replace is a replayed or out-of-order claim, not a
    /// move.
    #[error("presence incarnation {session_id} does not advance the stored lease")]
    StaleLease {
        /// The rejected incarnation.
        session_id: u64,
    },
}

/// The current authenticated reachability for each known peer.
#[derive(Debug, Default)]
pub struct PresenceStore {
    current: HashMap<NodeId, VerifiedPresence>,
}

impl PresenceStore {
    /// An empty store: nobody is known to be reachable.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update an already-verified presence.
    ///
    /// See [`PresenceUpdate`] for the three outcomes and [`PresenceError`] for
    /// the two refusals. `now_ms` is the caller's clock: a presence that has
    /// already lapsed is refused rather than stored, so `current` never returns
    /// something that was dead on arrival.
    pub fn upsert(
        &mut self,
        presence: VerifiedPresence,
        now_ms: u64,
    ) -> Result<PresenceUpdate, PresenceError> {
        let session_id = presence.session_id();
        if presence.expires_at_ms() <= now_ms {
            return Err(PresenceError::Expired { session_id });
        }
        let peer = presence.node_id().clone();

        let Some(existing) = self.current.get(&peer) else {
            self.current.insert(peer, presence);
            return Ok(PresenceUpdate::Inserted { session_id });
        };

        if existing.session_id() == session_id {
            if !same_reachability(existing, &presence) {
                return Err(PresenceError::IncarnationConflict { session_id });
            }
            // A lease refresh may extend the deadline and nothing else. It must
            // never move it backwards, so an older re-signed copy is ignored
            // rather than shortening the lease.
            if presence.expires_at_ms() > existing.expires_at_ms() {
                self.current.insert(peer, presence);
            }
            return Ok(PresenceUpdate::Refreshed { session_id });
        }

        let previous_session_id = existing.session_id();
        if presence.expires_at_ms() <= existing.expires_at_ms() {
            return Err(PresenceError::StaleLease { session_id });
        }
        self.current.insert(peer, presence);
        Ok(PresenceUpdate::Replaced {
            previous_session_id,
            session_id,
        })
    }

    /// The peer's current, unexpired presence — or `None` if it is unknown or
    /// its lease has lapsed.
    ///
    /// Expiry here means only "reachability metadata is stale". It is
    /// deliberately *not* a signal to tear down an authenticated connection:
    /// link lifetime and presence lifetime are different things.
    pub fn current(&self, peer: &NodeId, now_ms: u64) -> Option<&VerifiedPresence> {
        self.current
            .get(peer)
            .filter(|presence| presence.expires_at_ms() > now_ms)
    }

    /// Drop every lapsed entry, returning the peers that were dropped so the
    /// caller can invalidate whatever was derived from them.
    pub fn prune_expired(&mut self, now_ms: u64) -> Vec<NodeId> {
        let expired: Vec<NodeId> = self
            .current
            .iter()
            .filter(|(_, presence)| presence.expires_at_ms() <= now_ms)
            .map(|(peer, _)| peer.clone())
            .collect();
        for peer in &expired {
            self.current.remove(peer);
        }
        expired
    }

    /// Forget a peer entirely (revocation, explicit unpairing).
    pub fn remove(&mut self, peer: &NodeId) -> Option<VerifiedPresence> {
        self.current.remove(peer)
    }

    /// How many peers are currently stored (expired entries included until
    /// pruned).
    pub fn len(&self) -> usize {
        self.current.len()
    }

    /// Whether nobody is stored.
    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }
}

/// Whether two verified records describe the same *reachability*.
///
/// Lease bookkeeping — expiry and nonce — is excluded on purpose: the protocol
/// refreshes a lease by re-signing the same incarnation with a later expiry, and
/// that must not read as a conflict. What an incarnation does have to keep
/// identical is who is reachable, for which audience, at which addresses, and
/// whether it terminates relay circuits.
fn same_reachability(a: &VerifiedPresence, b: &VerifiedPresence) -> bool {
    let (left, right) = (a.record(), b.record());
    left.node_id == right.node_id
        && left.public_key == right.public_key
        && left.audience == right.audience
        && left.relay_capable == right.relay_capable
        && left.candidates == right.candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_mesh_protocol::rendezvous::{
        canonicalize_candidates, EndpointCandidate, EndpointCandidateKind,
    };

    use crate::identity::NodeIdentity;
    use crate::relay::{now_ms, signed_presence_with_candidates};

    fn candidate(kind: EndpointCandidateKind, addr: &str) -> EndpointCandidate {
        EndpointCandidate::new(kind, addr.parse().unwrap()).unwrap()
    }

    /// A real signed-and-verified presence, minted against the wall clock so the
    /// lease math is exercised for real rather than against a synthetic now.
    fn verified(
        identity: &NodeIdentity,
        peer: &str,
        session_id: u64,
        ttl_ms: u64,
        candidates: Vec<EndpointCandidate>,
    ) -> VerifiedPresence {
        let record = signed_presence_with_candidates(
            identity,
            NodeId::new(peer).unwrap(),
            "relay.example".to_string(),
            session_id,
            ttl_ms,
            true,
            canonicalize_candidates(candidates).unwrap(),
        )
        .unwrap();
        VerifiedPresence::verify(record, now_ms(), "relay.example").unwrap()
    }

    fn host(addr: &str) -> Vec<EndpointCandidate> {
        vec![candidate(EndpointCandidateKind::Host, addr)]
    }

    #[test]
    fn the_first_presence_for_a_peer_is_inserted() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();
        let away = NodeId::new("node-b").unwrap();

        let update = store
            .upsert(
                verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        assert_eq!(update, PresenceUpdate::Inserted { session_id: 1 });
        assert!(store.current(&away, now_ms()).is_some());
    }

    #[test]
    fn the_same_incarnation_at_the_same_reachability_is_idempotent() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();

        store
            .upsert(
                verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        let update = store
            .upsert(
                verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        assert_eq!(update, PresenceUpdate::Refreshed { session_id: 1 });
    }

    #[test]
    fn the_same_incarnation_at_different_reachability_is_refused() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();

        store
            .upsert(
                verified(&identity, "node-b", 1, 600_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        let conflict = store.upsert(
            verified(&identity, "node-b", 1, 600_000, host("192.168.1.44:51820")),
            now_ms(),
        );
        assert_eq!(
            conflict,
            Err(PresenceError::IncarnationConflict { session_id: 1 })
        );
        // The original claim survives untouched.
        let current = store
            .current(&NodeId::new("node-b").unwrap(), now_ms())
            .unwrap();
        assert_eq!(current.dialable_candidates().len(), 1);
        assert_eq!(
            current.dialable_candidates()[0].1,
            "192.168.1.9:51820".parse().unwrap()
        );
    }

    #[test]
    fn a_new_incarnation_replaces_the_old_one_and_reports_both_ids() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();

        store
            .upsert(
                verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        let update = store
            .upsert(
                verified(&identity, "node-b", 2, 600_000, host("192.168.1.44:51820")),
                now_ms(),
            )
            .unwrap();

        assert_eq!(
            update,
            PresenceUpdate::Replaced {
                previous_session_id: 1,
                session_id: 2
            }
        );
        let current = store
            .current(&NodeId::new("node-b").unwrap(), now_ms())
            .unwrap();
        assert_eq!(current.session_id(), 2);
        // The old incarnation's address is gone, not merged.
        assert_eq!(
            current.dialable_candidates()[0].1,
            "192.168.1.44:51820".parse().unwrap()
        );
    }

    #[test]
    fn an_expired_presence_is_refused_and_never_becomes_current() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();
        let peer = NodeId::new("node-b").unwrap();
        let presence = verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820"));

        let expires = presence.expires_at_ms();
        assert_eq!(
            store.upsert(presence, expires + 1),
            Err(PresenceError::Expired { session_id: 1 })
        );
        assert!(store.current(&peer, now_ms()).is_none());
        assert!(store.is_empty());
    }

    #[test]
    fn a_lease_that_has_lapsed_is_no_longer_current() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();
        let peer = NodeId::new("node-b").unwrap();
        let presence = verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820"));
        let expires = presence.expires_at_ms();

        store.upsert(presence, now_ms()).unwrap();
        assert!(store.current(&peer, expires - 1).is_some());
        assert!(store.current(&peer, expires).is_none());
    }

    #[test]
    fn pruning_reports_the_peers_whose_reachability_died() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();
        let peer = NodeId::new("node-b").unwrap();
        let presence = verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820"));
        let expires = presence.expires_at_ms();

        store.upsert(presence, now_ms()).unwrap();
        assert!(store.prune_expired(expires - 1).is_empty());
        assert_eq!(store.prune_expired(expires), vec![peer]);
        assert!(store.is_empty());
    }

    #[test]
    fn a_new_incarnation_must_advance_the_lease() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();

        store
            .upsert(
                verified(&identity, "node-b", 1, 600_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        // A "new" incarnation with a shorter lease than the one it would replace
        // is out-of-order or replayed, not a move.
        let stale = store.upsert(
            verified(&identity, "node-b", 2, 1_000, host("192.168.1.44:51820")),
            now_ms(),
        );
        assert_eq!(stale, Err(PresenceError::StaleLease { session_id: 2 }));
    }

    #[test]
    fn removing_a_peer_forgets_it() {
        let identity = NodeIdentity::generate().unwrap();
        let mut store = PresenceStore::new();
        let peer = NodeId::new("node-b").unwrap();

        store
            .upsert(
                verified(&identity, "node-b", 1, 60_000, host("192.168.1.9:51820")),
                now_ms(),
            )
            .unwrap();
        assert!(store.remove(&peer).is_some());
        assert!(store.current(&peer, now_ms()).is_none());
        assert!(store.remove(&peer).is_none());
    }
}
