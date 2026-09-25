//! Pairing wire messages: the ceremony that turns "something I saw" into
//! "a peer the user accepted".
//!
//! # Minimal authority channel
//!
//! These messages run over an authenticated but **unpinned** transport link —
//! a channel whose only purpose is the pairing ceremony. Carrying any other
//! mesh payload (an advert, an invocation) on a pairing session is a protocol
//! violation and closes the connection: until the user has accepted a peer on
//! both sides, that peer gets no capability surface of any kind.
//!
//! # Versioning
//!
//! Pairing has its own [`PAIRING_PROTOCOL_VERSION`] and fails closed on
//! mismatch — it is a distinct state machine, not an implicit reuse of the
//! mesh envelope version.

use serde::{Deserialize, Serialize};

use apeireth_core::kernel::NodeId;

use crate::identity::KeyFingerprint;

/// Pairing ceremony protocol version. Exact match or fail closed.
pub const PAIRING_PROTOCOL_VERSION: u32 = 1;

/// Domain separation for the short authentication string. Never hash a bare
/// fingerprint: the code must change for every ceremony.
pub const PAIRING_SAS_DOMAIN: &str = "APEIRETH-MESH-PAIRING-V1";

/// One side's opening of the ceremony: identity claim plus a fresh nonce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairHello {
    /// Pairing protocol version.
    pub pairing_version: u32,
    /// Claimed node identity (verified by key possession below).
    pub node_id: NodeId,
    /// Fingerprint of the identity key terminating this channel.
    pub fingerprint: KeyFingerprint,
    /// Fresh random nonce; part of the SAS input and the replay defence.
    pub nonce: [u8; 16],
}

/// The responder's answer to a [`PairHello`]: its own identity and nonce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairChallenge {
    /// Pairing protocol version.
    pub pairing_version: u32,
    /// Claimed node identity.
    pub node_id: NodeId,
    /// Fingerprint of the identity key terminating this channel.
    pub fingerprint: KeyFingerprint,
    /// Fresh random nonce from the responder.
    pub nonce: [u8; 16],
}

/// Both sides now hold the full transcript and can derive the same code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairCodeReady {
    /// The locally derived short authentication string (canonical 6-digit
    /// form). Sent so each side can assert agreement; the *user* is the one
    /// who compares what is displayed on both devices.
    pub sas: String,
}

/// This side's user confirmed the displayed code matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairConfirm {
    /// The confirmed SAS, echoed so a mismatched transcript is detected
    /// mechanically and not only by the human.
    pub sas: String,
}

/// This side's user rejected the ceremony (or the code did not match).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairReject {
    /// Why, for the other side's diagnostics.
    pub reason: String,
}

/// Both sides confirmed; the ceremony is complete and trust may be persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairComplete {
    /// The SAS both sides confirmed.
    pub sas: String,
}

/// The pairing message set. Deliberately closed: nothing else may travel on
/// this channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PairingMessage {
    /// Initiator opening.
    Hello(PairHello),
    /// Responder answer.
    Challenge(PairChallenge),
    /// Both sides can now derive the code.
    CodeReady(PairCodeReady),
    /// One side's user confirmed.
    Confirm(PairConfirm),
    /// One side's user rejected.
    Reject(PairReject),
    /// Ceremony complete.
    Complete(PairComplete),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pairing_message_set_round_trips() {
        let message = PairingMessage::Hello(PairHello {
            pairing_version: PAIRING_PROTOCOL_VERSION,
            node_id: NodeId::new("node-a").unwrap(),
            fingerprint: KeyFingerprint::of_public_key(&[7u8; 32]),
            nonce: [1u8; 16],
        });
        let json = serde_json::to_string(&message).unwrap();
        let back: PairingMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, back);
    }
}
