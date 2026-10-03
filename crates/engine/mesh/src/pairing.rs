//! Secure peer pairing: the ceremony that turns "something I saw" into
//! "a peer the user accepted".
//!
//! # What pairing proves
//!
//! Discovery only says *a node may exist*. Pairing proves *which node it is* —
//! the channel is TLS-authenticated by the identity keys themselves — and
//! records *that the user accepted it* on **both** devices. Trust is
//! persisted only after both confirmations; one-sided confirmation, a code
//! mismatch, or a timeout leaves nothing behind.
//!
//! # The short authentication string
//!
//! Both sides independently derive a 6-digit code over a domain-separated
//! hash of the full transcript — both node ids, both key fingerprints, both
//! fresh nonces, and the pairing protocol version. A man-in-the-middle runs
//! two separate authenticated sessions, so the transcripts differ and the two
//! devices display different codes; the human comparing them is the detector.
//! The code changes for every ceremony: it is never `hash(fingerprint)`.
//!
//! Nonces are exchanged commit-then-reveal: the initiator sends only a hash of
//! its nonce, the responder answers with its own, and only then does the
//! initiator open the commitment. Whoever revealed last could otherwise grind
//! its nonce until the codes of two separate sessions match, which would
//! reduce the human comparison to a 20-bit search.
//!
//! # Minimal authority
//!
//! The pairing channel carries only [`PairingMessage`]s. Any capability
//! traffic — an advert, an invocation — is a protocol violation that closes
//! the connection. Nothing about the unpaired peer becomes routable,
//! discoverable-in-runtime, or executable.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::NodeId;
use apeireth_core::kernel::TraceId;
use apeireth_mesh_protocol::{
    KeyFingerprint, MeshEnvelope, MeshPayload, PairChallenge, PairCodeReady, PairComplete,
    PairConfirm, PairHello, PairReject, PairReveal, PairingMessage, PAIRING_COMMIT_DOMAIN,
    PAIRING_PROTOCOL_VERSION, PAIRING_SAS_DOMAIN,
};
use sha2::{Digest, Sha256};

use crate::error::MeshError;
use crate::identity::NodeIdentity;
use crate::transport::MeshConnection;
use crate::trust_store::TrustStore;

/// Where the ceremony stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    /// A discovery record exists; no channel yet.
    Discovered,
    /// The authenticated channel is being established.
    Connecting,
    /// The SAS is derived and waiting for the local user's decision.
    AwaitingUserVerification,
    /// This side's user confirmed; waiting for the peer's confirmation.
    Confirmed,
    /// Both sides confirmed; persisting trust.
    Persisting,
    /// Trust persisted; the peer is a KnownPeer.
    Paired,
    /// This side's user rejected.
    Rejected,
    /// The ceremony failed (timeout, mismatch, violation, transport).
    Failed,
}

/// The local user's decision about the displayed code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmDecision {
    /// The code matched what the other device displayed.
    Confirm,
    /// It did not, or the user declined.
    Reject,
}

/// Options for one ceremony.
#[derive(Debug, Clone)]
pub struct PairingOptions {
    /// Whole-ceremony deadline. On expiry: fail, persist nothing.
    pub timeout_ms: u64,
    /// Admission control for inbound ceremonies: the limiter and the source
    /// key (typically the remote address) the attempt is charged to.
    pub limiter: Option<(Arc<PairingLimiter>, String)>,
}

impl Default for PairingOptions {
    fn default() -> Self {
        Self {
            timeout_ms: 60_000,
            limiter: None,
        }
    }
}

impl PairingOptions {
    /// Charge inbound ceremonies to `source` against `limiter`.
    #[must_use]
    pub fn with_limiter(mut self, limiter: Arc<PairingLimiter>, source: impl Into<String>) -> Self {
        self.limiter = Some((limiter, source.into()));
        self
    }
}

/// Failure of a pairing ceremony. No variant leaves partial trust behind.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PairingError {
    /// The pairing protocol version did not match. Fail closed.
    #[error("pairing protocol mismatch: local {local}, remote {remote}")]
    VersionMismatch {
        /// Local pairing protocol version.
        local: u32,
        /// Remote pairing protocol version.
        remote: u32,
    },
    /// The two sides derived different codes (a possible man-in-the-middle).
    #[error("pairing code mismatch: local {local}, remote {remote}")]
    CodeMismatch {
        /// Locally derived SAS.
        local: String,
        /// The peer's claimed SAS.
        remote: String,
    },
    /// One side's user rejected the ceremony.
    #[error("pairing rejected: {0}")]
    Rejected(String),
    /// The peer's opened nonce does not match its commitment.
    #[error("pairing nonce does not match its commitment")]
    CommitmentMismatch,
    /// The ceremony exceeded its deadline.
    #[error("pairing timed out")]
    Timeout,
    /// A message arrived that does not belong on a pairing channel.
    #[error("pairing protocol violation: {0}")]
    Violation(String),
    /// The channel failed.
    #[error("pairing transport failed: {0}")]
    Transport(String),
    /// Trust could not be persisted.
    #[error("pairing trust store failed: {0}")]
    Store(String),
}

/// One completed ceremony's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedPeer {
    /// The peer's node identity (as claimed and key-authenticated).
    pub node: NodeId,
    /// The peer's key fingerprint (from its authenticated channel).
    pub key_fingerprint: KeyFingerprint,
    /// The endpoint persisted for future connections.
    pub endpoint: String,
    /// The code both users confirmed.
    pub sas: String,
}

/// The full ceremony transcript both sides derive the code from.
#[derive(Debug, Clone)]
pub struct PairingTranscript {
    /// Initiator's node id.
    pub initiator: NodeId,
    /// Responder's node id.
    pub responder: NodeId,
    /// Initiator's key fingerprint (authenticated by the channel).
    pub initiator_fingerprint: KeyFingerprint,
    /// Responder's key fingerprint (authenticated by the channel).
    pub responder_fingerprint: KeyFingerprint,
    /// Initiator's fresh nonce.
    pub initiator_nonce: [u8; 16],
    /// Responder's fresh nonce.
    pub responder_nonce: [u8; 16],
    /// Pairing protocol version.
    pub version: u32,
}

impl PairingTranscript {
    /// Derive the short authentication string: SHA-256 over the whole
    /// transcript with explicit domain separation, folded to 6 digits.
    ///
    /// Every input is bound — both identities, both keys, both nonces, the
    /// version — so two different transcripts (the MITM case) cannot produce
    /// the same code except by the deliberate 1-in-a-million collision that
    /// the user comparison exists to catch.
    pub fn sas(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(PAIRING_SAS_DOMAIN.as_bytes());
        hasher.update([0u8]);
        hasher.update(self.version.to_be_bytes());
        for part in [
            self.initiator.as_str().as_bytes(),
            self.responder.as_str().as_bytes(),
            self.initiator_fingerprint.as_str().as_bytes(),
            self.responder_fingerprint.as_str().as_bytes(),
        ] {
            hasher.update(part);
            hasher.update([0u8]);
        }
        hasher.update(self.initiator_nonce);
        hasher.update(self.responder_nonce);
        let digest = hasher.finalize();
        let value = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000;
        let raw = format!("{value:06}");
        format!("{} {}", &raw[..3], &raw[3..])
    }
}

/// Most distinct sources remembered at once; beyond it new sources are refused.
const MAX_TRACKED_SOURCES: usize = 4096;

/// Bounds on concurrent ceremonies (a LAN abuser must not spawn unbounded
/// pending pairing state).
#[derive(Debug)]
pub struct PairingLimiter {
    max_concurrent: usize,
    cooldown_ms: u64,
    active: Arc<std::sync::atomic::AtomicUsize>,
    last_attempt: Mutex<std::collections::BTreeMap<String, u64>>,
}

/// A held ceremony slot; released on drop.
pub struct PairingSlot {
    active: Arc<std::sync::atomic::AtomicUsize>,
}

impl Drop for PairingSlot {
    fn drop(&mut self) {
        self.active
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl PairingLimiter {
    /// A limiter with the given concurrency cap and per-source cooldown.
    pub fn new(max_concurrent: usize, cooldown_ms: u64) -> Arc<Self> {
        Arc::new(Self {
            max_concurrent,
            cooldown_ms,
            active: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            last_attempt: Mutex::new(std::collections::BTreeMap::new()),
        })
    }

    /// Try to begin a ceremony for `source`. Fails closed on abuse.
    pub fn try_begin(&self, source: &str, now_ms: u64) -> Result<PairingSlot, PairingError> {
        let mut last = self.last_attempt.lock().unwrap_or_else(|p| p.into_inner());
        // Entries past their cooldown carry no information; dropping them
        // keeps the table bounded by the attempt rate, not by uptime.
        last.retain(|_, previous| now_ms.saturating_sub(*previous) < self.cooldown_ms);
        if last.contains_key(source) {
            return Err(PairingError::Rejected(
                "pairing cooldown active for this source".to_string(),
            ));
        }
        if last.len() >= MAX_TRACKED_SOURCES {
            return Err(PairingError::Rejected(
                "too many recent pairing sources".to_string(),
            ));
        }
        last.insert(source.to_string(), now_ms);
        drop(last);
        let current = self
            .active
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if current >= self.max_concurrent {
            self.active
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            return Err(PairingError::Rejected(
                "too many concurrent pairing sessions".to_string(),
            ));
        }
        Ok(PairingSlot {
            active: Arc::clone(&self.active),
        })
    }
}

/// A predictable nonce would break the SAS, so an RNG failure aborts the
/// ceremony instead of degrading.
fn fresh_nonce() -> Result<[u8; 16], PairingError> {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut nonce = [0u8; 16];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| PairingError::Transport("system random source unavailable".to_string()))?;
    Ok(nonce)
}

/// The initiator's commitment to its nonce, bound to its node id.
fn nonce_commitment(initiator: &NodeId, nonce: &[u8; 16]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(PAIRING_COMMIT_DOMAIN.as_bytes());
    hasher.update([0u8]);
    hasher.update(initiator.as_str().as_bytes());
    hasher.update([0u8]);
    hasher.update(nonce);
    hasher.finalize().into()
}

fn wall_clock_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// Message envelope helper for the pairing channel.
fn pair_envelope(from: &NodeId, to: &NodeId, message: PairingMessage) -> MeshEnvelope {
    MeshEnvelope::send(
        from.clone(),
        to.clone(),
        TraceId::new(),
        MeshPayload::Pair(message),
    )
}

/// Receive one pairing message; anything else is a violation that closes the
/// ceremony (minimal-authority channel).
async fn recv_pair(
    link: &Arc<dyn MeshConnection>,
    timeout_ms: u64,
) -> Result<PairingMessage, PairingError> {
    let envelope = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), link.recv())
        .await
        .map_err(|_| PairingError::Timeout)?
        .map_err(|error| PairingError::Transport(error.to_string()))?;
    match envelope.payload {
        MeshPayload::Pair(message) => Ok(message),
        other => {
            link.close();
            Err(PairingError::Violation(format!(
                "non-pairing payload on a pairing channel: {}",
                crate::kinds::payload_kind(&other)
            )))
        }
    }
}

/// Bind a claimed fingerprint to the channel's authenticated key.
fn bind_claim(
    claimed: &KeyFingerprint,
    presented: Option<KeyFingerprint>,
) -> Result<(), PairingError> {
    match presented {
        Some(presented) if presented == *claimed => Ok(()),
        Some(presented) => Err(PairingError::Violation(format!(
            "channel key {presented} does not match the claimed fingerprint {claimed}"
        ))),
        None => Err(PairingError::Violation(
            "pairing channel carries no authenticated key".to_string(),
        )),
    }
}

/// Run the ceremony from the initiating side.
///
/// The caller supplies the peer endpoint (typically from discovery) for
/// persistence; `user` is the headless-friendly confirmation hook (a CLI
/// prompts through it — never defaulting to yes).
pub async fn pair_outbound<F>(
    link: Arc<dyn MeshConnection>,
    identity: &NodeIdentity,
    my_node: &NodeId,
    claimed_peer: NodeId,
    peer_endpoint: String,
    store: &TrustStore,
    user: F,
    options: PairingOptions,
) -> Result<PairedPeer, PairingError>
where
    F: FnOnce(&str) -> ConfirmDecision,
{
    let nonce_a = fresh_nonce()?;
    link.send(pair_envelope(
        my_node,
        &claimed_peer,
        PairingMessage::Hello(PairHello {
            pairing_version: PAIRING_PROTOCOL_VERSION,
            node_id: my_node.clone(),
            fingerprint: identity.fingerprint().clone(),
            nonce_commitment: nonce_commitment(my_node, &nonce_a),
        }),
    ))
    .await
    .map_err(|e| PairingError::Transport(e.to_string()))?;

    let challenge = match recv_pair(&link, options.timeout_ms).await? {
        PairingMessage::Challenge(challenge) => challenge,
        PairingMessage::Reject(reject) => {
            return Err(PairingError::Rejected(reject.reason));
        }
        other => {
            link.close();
            return Err(PairingError::Violation(format!(
                "expected PairChallenge, got {}",
                crate::kinds::pairing_kind(&other)
            )));
        }
    };
    if challenge.pairing_version != PAIRING_PROTOCOL_VERSION {
        return Err(PairingError::VersionMismatch {
            local: PAIRING_PROTOCOL_VERSION,
            remote: challenge.pairing_version,
        });
    }
    bind_claim(&challenge.fingerprint, link.peer_fingerprint())?;
    if challenge.node_id != claimed_peer {
        link.close();
        return Err(PairingError::Violation(format!(
            "dialled {claimed_peer} but the peer identifies as {}",
            challenge.node_id
        )));
    }

    // Only now, with the responder's nonce on the table, open the commitment.
    link.send(pair_envelope(
        my_node,
        &challenge.node_id,
        PairingMessage::Reveal(PairReveal { nonce: nonce_a }),
    ))
    .await
    .map_err(|e| PairingError::Transport(e.to_string()))?;

    let transcript = PairingTranscript {
        initiator: my_node.clone(),
        responder: challenge.node_id.clone(),
        initiator_fingerprint: identity.fingerprint().clone(),
        responder_fingerprint: challenge.fingerprint.clone(),
        initiator_nonce: nonce_a,
        responder_nonce: challenge.nonce,
        version: PAIRING_PROTOCOL_VERSION,
    };
    let sas = transcript.sas();

    link.send(pair_envelope(
        my_node,
        &challenge.node_id,
        PairingMessage::CodeReady(PairCodeReady { sas: sas.clone() }),
    ))
    .await
    .map_err(|e| PairingError::Transport(e.to_string()))?;

    // The peer's own code must agree — a MITM's two sessions derive two codes.
    let peer_code = match recv_pair(&link, options.timeout_ms).await? {
        PairingMessage::CodeReady(code) => code.sas,
        PairingMessage::Reject(reject) => {
            return Err(PairingError::Rejected(reject.reason));
        }
        other => {
            link.close();
            return Err(PairingError::Violation(format!(
                "expected PairCodeReady, got {}",
                crate::kinds::pairing_kind(&other)
            )));
        }
    };
    if peer_code != sas {
        return Err(PairingError::CodeMismatch {
            local: sas,
            remote: peer_code,
        });
    }

    // The human decides with the code shown on BOTH devices.
    match user(&sas) {
        ConfirmDecision::Reject => {
            let _ = link
                .send(pair_envelope(
                    my_node,
                    &challenge.node_id,
                    PairingMessage::Reject(PairReject {
                        reason: "user rejected the pairing code".to_string(),
                    }),
                ))
                .await;
            Err(PairingError::Rejected("local user rejected".to_string()))
        }
        ConfirmDecision::Confirm => {
            link.send(pair_envelope(
                my_node,
                &challenge.node_id,
                PairingMessage::Confirm(PairConfirm { sas: sas.clone() }),
            ))
            .await
            .map_err(|e| PairingError::Transport(e.to_string()))?;
            match recv_pair(&link, options.timeout_ms).await? {
                PairingMessage::Confirm(confirm) if confirm.sas == sas => {
                    let _ = link
                        .send(pair_envelope(
                            my_node,
                            &challenge.node_id,
                            PairingMessage::Complete(PairComplete { sas: sas.clone() }),
                        ))
                        .await;
                    store
                        .record_paired(
                            challenge.node_id.clone(),
                            peer_endpoint.clone(),
                            challenge.fingerprint.clone(),
                        )
                        .map_err(|e| PairingError::Store(e.to_string()))?;
                    Ok(PairedPeer {
                        node: challenge.node_id,
                        key_fingerprint: challenge.fingerprint,
                        endpoint: peer_endpoint,
                        sas,
                    })
                }
                PairingMessage::Confirm(confirm) => Err(PairingError::CodeMismatch {
                    local: sas,
                    remote: confirm.sas,
                }),
                PairingMessage::Reject(reject) => Err(PairingError::Rejected(reject.reason)),
                other => {
                    link.close();
                    Err(PairingError::Violation(format!(
                        "expected PairConfirm, got {}",
                        crate::kinds::pairing_kind(&other)
                    )))
                }
            }
        }
    }
}

/// Run the ceremony from the responding side. Mirror of [`pair_outbound`]:
/// same transcript, same code, same bilateral rule.
pub async fn pair_inbound<F>(
    link: Arc<dyn MeshConnection>,
    identity: &NodeIdentity,
    my_node: &NodeId,
    peer_endpoint: String,
    store: &TrustStore,
    user: F,
    options: PairingOptions,
) -> Result<PairedPeer, PairingError>
where
    F: FnOnce(&str) -> ConfirmDecision,
{
    let hello = match recv_pair(&link, options.timeout_ms).await? {
        PairingMessage::Hello(hello) => hello,
        other => {
            link.close();
            return Err(PairingError::Violation(format!(
                "expected PairHello, got {}",
                crate::kinds::pairing_kind(&other)
            )));
        }
    };
    if hello.pairing_version != PAIRING_PROTOCOL_VERSION {
        return Err(PairingError::VersionMismatch {
            local: PAIRING_PROTOCOL_VERSION,
            remote: hello.pairing_version,
        });
    }
    bind_claim(&hello.fingerprint, link.peer_fingerprint())?;

    let _slot = match &options.limiter {
        Some((limiter, source)) => Some(limiter.try_begin(source, wall_clock_ms())?),
        None => None,
    };

    let nonce_b = fresh_nonce()?;
    link.send(pair_envelope(
        my_node,
        &hello.node_id,
        PairingMessage::Challenge(PairChallenge {
            pairing_version: PAIRING_PROTOCOL_VERSION,
            node_id: my_node.clone(),
            fingerprint: identity.fingerprint().clone(),
            nonce: nonce_b,
        }),
    ))
    .await
    .map_err(|e| PairingError::Transport(e.to_string()))?;

    let initiator_nonce = match recv_pair(&link, options.timeout_ms).await? {
        PairingMessage::Reveal(reveal) => reveal.nonce,
        PairingMessage::Reject(reject) => {
            return Err(PairingError::Rejected(reject.reason));
        }
        other => {
            link.close();
            return Err(PairingError::Violation(format!(
                "expected PairReveal, got {}",
                crate::kinds::pairing_kind(&other)
            )));
        }
    };
    if nonce_commitment(&hello.node_id, &initiator_nonce) != hello.nonce_commitment {
        link.close();
        return Err(PairingError::CommitmentMismatch);
    }

    let transcript = PairingTranscript {
        initiator: hello.node_id.clone(),
        responder: my_node.clone(),
        initiator_fingerprint: hello.fingerprint.clone(),
        responder_fingerprint: identity.fingerprint().clone(),
        initiator_nonce,
        responder_nonce: nonce_b,
        version: PAIRING_PROTOCOL_VERSION,
    };
    let sas = transcript.sas();

    let peer_code = match recv_pair(&link, options.timeout_ms).await? {
        PairingMessage::CodeReady(code) => code.sas,
        PairingMessage::Reject(reject) => {
            return Err(PairingError::Rejected(reject.reason));
        }
        other => {
            link.close();
            return Err(PairingError::Violation(format!(
                "expected PairCodeReady, got {}",
                crate::kinds::pairing_kind(&other)
            )));
        }
    };
    if peer_code != sas {
        return Err(PairingError::CodeMismatch {
            local: sas,
            remote: peer_code,
        });
    }

    link.send(pair_envelope(
        my_node,
        &hello.node_id,
        PairingMessage::CodeReady(PairCodeReady { sas: sas.clone() }),
    ))
    .await
    .map_err(|e| PairingError::Transport(e.to_string()))?;

    match user(&sas) {
        ConfirmDecision::Reject => {
            let _ = link
                .send(pair_envelope(
                    my_node,
                    &hello.node_id,
                    PairingMessage::Reject(PairReject {
                        reason: "user rejected the pairing code".to_string(),
                    }),
                ))
                .await;
            Err(PairingError::Rejected("local user rejected".to_string()))
        }
        ConfirmDecision::Confirm => {
            link.send(pair_envelope(
                my_node,
                &hello.node_id,
                PairingMessage::Confirm(PairConfirm { sas: sas.clone() }),
            ))
            .await
            .map_err(|e| PairingError::Transport(e.to_string()))?;
            match recv_pair(&link, options.timeout_ms).await? {
                PairingMessage::Confirm(confirm) if confirm.sas == sas => {
                    let _ = link
                        .send(pair_envelope(
                            my_node,
                            &hello.node_id,
                            PairingMessage::Complete(PairComplete { sas: sas.clone() }),
                        ))
                        .await;
                    store
                        .record_paired(
                            hello.node_id.clone(),
                            peer_endpoint.clone(),
                            hello.fingerprint.clone(),
                        )
                        .map_err(|e| PairingError::Store(e.to_string()))?;
                    Ok(PairedPeer {
                        node: hello.node_id,
                        key_fingerprint: hello.fingerprint,
                        endpoint: peer_endpoint,
                        sas,
                    })
                }
                PairingMessage::Confirm(confirm) => Err(PairingError::CodeMismatch {
                    local: sas,
                    remote: confirm.sas,
                }),
                PairingMessage::Reject(reject) => Err(PairingError::Rejected(reject.reason)),
                other => {
                    link.close();
                    Err(PairingError::Violation(format!(
                        "expected PairConfirm, got {}",
                        crate::kinds::pairing_kind(&other)
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_mesh_protocol::KeyFingerprint;

    #[test]
    fn the_sas_binds_the_whole_transcript_and_changes_every_ceremony() {
        let transcript = PairingTranscript {
            initiator: NodeId::new("node-a").unwrap(),
            responder: NodeId::new("node-b").unwrap(),
            initiator_fingerprint: KeyFingerprint::of_public_key(&[1; 32]),
            responder_fingerprint: KeyFingerprint::of_public_key(&[2; 32]),
            initiator_nonce: [1u8; 16],
            responder_nonce: [2u8; 16],
            version: PAIRING_PROTOCOL_VERSION,
        };
        let sas = transcript.sas();
        assert_eq!(sas.len(), 7, "6 digits plus separator");
        assert_eq!(&sas[3..4], " ");
        assert_eq!(transcript.sas(), sas, "deterministic");

        // Fresh nonces → fresh code.
        let mut other = transcript.clone();
        other.initiator_nonce = [9u8; 16];
        assert_ne!(other.sas(), sas, "the code must change every ceremony");

        // Role swap → different code (transcript roles are bound).
        let mut swapped = transcript.clone();
        std::mem::swap(
            &mut swapped.initiator_fingerprint,
            &mut swapped.responder_fingerprint,
        );
        assert_ne!(swapped.sas(), sas, "roles bind the code");
    }

    #[test]
    fn a_mitm_with_two_sessions_sees_two_codes() {
        // A↔M and M↔B sessions produce different transcripts, so the two
        // devices the user compares can never agree silently.
        let a_side = PairingTranscript {
            initiator: NodeId::new("node-a").unwrap(),
            responder: NodeId::new("node-m").unwrap(),
            initiator_fingerprint: KeyFingerprint::of_public_key(&[1; 32]),
            responder_fingerprint: KeyFingerprint::of_public_key(&[9; 32]),
            initiator_nonce: [1u8; 16],
            responder_nonce: [3u8; 16],
            version: PAIRING_PROTOCOL_VERSION,
        };
        let b_side = PairingTranscript {
            initiator: NodeId::new("node-m").unwrap(),
            responder: NodeId::new("node-b").unwrap(),
            initiator_fingerprint: KeyFingerprint::of_public_key(&[9; 32]),
            responder_fingerprint: KeyFingerprint::of_public_key(&[2; 32]),
            initiator_nonce: [3u8; 16],
            responder_nonce: [2u8; 16],
            version: PAIRING_PROTOCOL_VERSION,
        };
        assert_ne!(a_side.sas(), b_side.sas(), "the user sees a mismatch");
    }

    #[test]
    fn the_limiter_bounds_concurrency_and_cools_sources_down() {
        let limiter = PairingLimiter::new(1, 1_000);
        let slot = limiter.try_begin("a", 0).unwrap();
        assert!(
            limiter.try_begin("b", 0).is_err(),
            "concurrent sessions are bounded"
        );
        drop(slot);
        assert!(limiter.try_begin("a", 100).is_err(), "cooldown enforced");
        assert!(limiter.try_begin("a", 1_200).is_ok(), "cooldown expires");
    }
}
