//! Turning a *verified* reachability claim into a dial plan.
//!
//! The seam exists so there is exactly one place where signed candidates
//! become addresses, and so that only a verified record can ever produce one:
//! a candidate is *where* to dial, and it must never be reachable from an
//! unverified or unsigned source.

use apeireth_mesh_protocol::rendezvous::PresenceRecord;

/// The endpoints to try for a peer, in dial order (host first).
///
/// **The caller must have verified `record`** — [`PresenceRecord::verify`] or
/// [`PresenceRecord::verify_for`] — before calling this. This function
/// deliberately does not verify: it is pure translation, and the point of the
/// separation is that verification happens once, at the boundary where the
/// record arrives, and the dial path below it can never see an unverified
/// address.
///
/// Duplicates cannot appear: `PresenceRecord::verify` requires canonical order,
/// which rejects them upstream.
pub fn dial_plan(record: &PresenceRecord) -> Vec<String> {
    record
        .dialable_candidates()
        .into_iter()
        .map(|(_kind, addr)| addr.to_string())
        .collect()
}
