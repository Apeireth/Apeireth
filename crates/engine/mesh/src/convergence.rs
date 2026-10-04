//! Which of two simultaneous connections between one peer pair survives.
//!
//! Simultaneous open can produce two authenticated connections at once:
//! `A outbound -> B inbound`, and `B outbound -> A inbound`. Both peers must
//! independently reach the *same* answer, or each keeps a different one and the
//! pair ends up with two logical connections — two links, two adverts, and a
//! real chance of executing one remote call twice.
//!
//! So the rule is stated only over facts both ends already have and cannot
//! disagree about: the two stable `NodeId`s and the direction each end observes.
//! Never arrival order, never connection-completion timing, never a socket
//! address, never a QUIC connection id — none of those are stable authority, and
//! two ends can disagree about all of them.

use std::cmp::Ordering;

use apeireth_core::kernel::NodeId;

/// Which way a connection was established, from the local node's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// We dialed them.
    Outbound,
    /// They dialed us.
    Inbound,
}

/// Whether the connection in `direction` is the one that survives.
///
/// The connection **initiated by the lexicographically smaller `NodeId`** wins.
/// The smaller node therefore keeps its outbound connection and drops its
/// inbound one; the larger node keeps its inbound and drops its outbound. Both
/// ends evaluate this over the same two identities and the direction *they*
/// observe, which is what makes the two decisions name the same physical
/// connection.
///
/// A node that dialed itself (a degenerate self-connection) keeps the outbound
/// side, so the function stays total and deterministic.
pub fn keeps_connection(local: &NodeId, remote: &NodeId, direction: Direction) -> bool {
    match local.cmp(remote) {
        Ordering::Less => matches!(direction, Direction::Outbound),
        Ordering::Greater => matches!(direction, Direction::Inbound),
        Ordering::Equal => matches!(direction, Direction::Outbound),
    }
}

/// The same rule, asked the way the connection manager asks it: an incumbent
/// link already exists, a new connection has just authenticated for the same
/// peer, and exactly one of them may stay.
///
/// The survivor is the one whose *initiator* is the lexicographically smaller
/// `NodeId`, so the newcomer wins only when its initiator is that smaller node —
/// or the same node, which is the ordinary reconnect case.
pub fn new_connection_survives(incumbent_initiator: &NodeId, newcomer_initiator: &NodeId) -> bool {
    newcomer_initiator <= incumbent_initiator
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> NodeId {
        NodeId::new(name).unwrap()
    }

    #[test]
    fn the_lower_identity_keeps_its_outbound_connection() {
        let (a, b) = (id("node-a"), id("node-b"));
        assert!(a < b);
        assert!(keeps_connection(&a, &b, Direction::Outbound));
        assert!(!keeps_connection(&a, &b, Direction::Inbound));
    }

    #[test]
    fn the_higher_identity_keeps_its_inbound_connection() {
        let (a, b) = (id("node-a"), id("node-b"));
        assert!(keeps_connection(&b, &a, Direction::Inbound));
        assert!(!keeps_connection(&b, &a, Direction::Outbound));
    }

    #[test]
    fn both_ends_agree_about_each_physical_connection() {
        let (a, b) = (id("node-a"), id("node-b"));

        // Physical connection 1: a outbound -> b inbound. Both ends must say
        // "keep".
        assert_eq!(
            keeps_connection(&a, &b, Direction::Outbound),
            keeps_connection(&b, &a, Direction::Inbound)
        );
        assert!(keeps_connection(&a, &b, Direction::Outbound));

        // Physical connection 2: b outbound -> a inbound. Both ends must say
        // "drop".
        assert_eq!(
            keeps_connection(&b, &a, Direction::Outbound),
            keeps_connection(&a, &b, Direction::Inbound)
        );
        assert!(!keeps_connection(&b, &a, Direction::Outbound));
    }

    #[test]
    fn exactly_one_of_the_two_physical_connections_survives() {
        let (a, b) = (id("node-a"), id("node-b"));
        let first = keeps_connection(&a, &b, Direction::Outbound);
        let second = keeps_connection(&b, &a, Direction::Outbound);
        assert!(first ^ second, "exactly one direction must survive");
    }

    #[test]
    fn a_self_connection_keeps_the_outbound_side() {
        let a = id("node-a");
        assert!(keeps_connection(&a, &a, Direction::Outbound));
        assert!(!keeps_connection(&a, &a, Direction::Inbound));
    }

    #[test]
    fn the_newcomer_wins_only_when_its_initiator_is_the_smaller_node() {
        let (a, b) = (id("node-a"), id("node-b"));
        assert!(
            new_connection_survives(&b, &a),
            "a dialed us and a is smaller, so a's connection replaces b's"
        );
        assert!(
            !new_connection_survives(&a, &b),
            "b dialed us and b is larger, so b's connection must not evict a's"
        );
        assert!(
            new_connection_survives(&a, &a),
            "same initiator is the ordinary reconnect: the newcomer wins"
        );
    }
}
