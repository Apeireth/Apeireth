//! Peer lifecycle state.
//!
//! Deliberately not a distributed failure detector: one liveness timestamp and
//! four states are enough for Phase 0. Losing a peer makes its routes
//! undispatchable — it never deletes the peer's persistent identity and never
//! pretends the capabilities are permanently gone, because the very next
//! reconnect can bring the same projection back.

use apeireth_core::kernel::NodeId;

/// Where a peer connection is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    /// The handshake is in progress.
    Connecting,
    /// Handshaken and carrying traffic.
    Active,
    /// Timed out or the link dropped; its routes are not dispatchable.
    Unreachable,
    /// Deliberately closed or refused. Terminal until a new connection.
    Closed,
}

impl PeerState {
    /// Whether calls may be dispatched to this peer right now.
    pub const fn is_dispatchable(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Stable label for logs and reports.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Active => "active",
            Self::Unreachable => "unreachable",
            Self::Closed => "closed",
        }
    }
}

/// One known connection to one peer node.
#[derive(Debug, Clone)]
pub struct Peer {
    /// The peer's canonical identity (from the handshake, not the transport).
    pub node: NodeId,
    /// Display name reported during the handshake. Cosmetic only.
    pub name: String,
    /// Current lifecycle state.
    pub state: PeerState,
    /// Time of the last inbound message of any kind, in epoch milliseconds.
    /// Heartbeats count, but so does everything else.
    pub last_seen_ms: u64,
}

impl Peer {
    /// A peer that is still handshaking.
    pub fn connecting(node: NodeId, name: impl Into<String>, now_ms: u64) -> Self {
        Self {
            node,
            name: name.into(),
            state: PeerState::Connecting,
            last_seen_ms: now_ms,
        }
    }

    /// Record inbound liveness.
    pub fn note_seen(&mut self, now_ms: u64) {
        self.last_seen_ms = self.last_seen_ms.max(now_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_active_peer_accepts_dispatch() {
        for (state, dispatchable) in [
            (PeerState::Connecting, false),
            (PeerState::Active, true),
            (PeerState::Unreachable, false),
            (PeerState::Closed, false),
        ] {
            assert_eq!(state.is_dispatchable(), dispatchable, "{state:?}");
        }
    }

    #[test]
    fn liveness_only_moves_forward() {
        let mut peer = Peer::connecting(NodeId::new("node-b").unwrap(), "b", 100);
        peer.note_seen(50);
        assert_eq!(peer.last_seen_ms, 100);
        peer.note_seen(200);
        assert_eq!(peer.last_seen_ms, 200);
    }
}
