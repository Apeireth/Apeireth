//! Turning a *verified* reachability claim into a dial plan.
//!
//! The seam exists so there is exactly one place where signed candidates
//! become addresses, and so that only a verified record can ever produce one:
//! a candidate is *where* to dial, and it must never be reachable from an
//! unverified or unsigned source.

use apeireth_mesh_protocol::rendezvous::VerifiedPresence;

/// The endpoints to try for a peer, in dial order (host first).
///
/// Takes [`VerifiedPresence`], not a raw record, so the dial path cannot be
/// reached with an unverified address: verification happens once, where the
/// record arrives, and the type carries that fact forward.
///
/// Duplicates cannot appear: `PresenceRecord::verify` requires canonical order,
/// which rejects them upstream.
pub fn dial_plan(presence: &VerifiedPresence) -> Vec<String> {
    presence
        .dialable_candidates()
        .into_iter()
        .map(|(_kind, addr)| addr.to_string())
        .collect()
}
