//! Tamper-evident audit hash chain.
//!
//! This is an **integrity primitive**, not the runtime execution trace. The
//! canonical `ExecutionTrace` remains in `apeireth-runtime::canonical`; this
//! module only proves that an ordered sequence of records has not been mutated
//! after it was appended.
//!
//! # Canonical serialization
//!
//! Every record hash is computed from the same fixed field order:
//!
//! ```text
//! sequence:u64 (big-endian)
//! timestamp_epoch_millis:i64 (big-endian)
//! event_kind: length-prefixed bytes
//! subject: length-prefixed bytes
//! previous_hash: length-prefixed bytes (lower-case hex)
//! ```
//!
//! Length prefixes are `u64` big-endian byte lengths. This is unambiguous and
//! avoids hash instability from `HashMap` ordering or separator characters in
//! free-text fields.
//!
//! # Time
//!
//! `append` takes a canonical [`Timestamp`]; it never reads the wall clock.
//! Tests can therefore build deterministic chains with fixed timestamps.

use std::fmt;

use apeireth_core::kernel::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The fixed previous hash for the first record.
///
/// A zero hash is unambiguous and does not pretend to prove anything outside
/// the chain itself.
pub const GENESIS_PREVIOUS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

fn hash_record(
    sequence: u64,
    timestamp: Timestamp,
    event_kind: &str,
    subject: &str,
    previous_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sequence.to_be_bytes());
    hasher.update(timestamp.epoch_millis().to_be_bytes());
    update_len_prefixed(&mut hasher, event_kind.as_bytes());
    update_len_prefixed(&mut hasher, subject.as_bytes());
    update_len_prefixed(&mut hasher, previous_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn update_len_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// One ordered, linked audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Position in the chain, counting from zero.
    pub sequence: u64,
    /// When the event happened. Supplied by the caller; never read from the
    /// wall clock inside this module.
    pub timestamp: Timestamp,
    /// Stable event kind, e.g. `governance.evaluated`.
    pub event_kind: String,
    /// Subject or context for the event, e.g. `tool.shell` or `session.id`.
    pub subject: String,
    /// Previous record's `current_hash`, or [`GENESIS_PREVIOUS_HASH`] for the
    /// first record.
    pub previous_hash: String,
    /// SHA-256 of this record's canonical serialization.
    pub current_hash: String,
}

/// Why an audit chain failed verification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuditChainError {
    #[error("audit chain record {index} has sequence {found}, expected {expected}")]
    SequenceMismatch {
        index: usize,
        expected: u64,
        found: u64,
    },
    #[error("audit chain record {index} has an invalid genesis previous_hash")]
    GenesisHashMismatch { index: usize },
    #[error(
        "audit chain record {index} previous_hash does not point to the previous current_hash"
    )]
    PreviousHashMismatch { index: usize },
    #[error("audit chain record {index} current_hash is corrupted (tamper detected)")]
    RecordHashMismatch { index: usize },
}

/// An append-only, in-memory, tamper-evident audit chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditHashChain {
    records: Vec<AuditRecord>,
}

impl AuditHashChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a record with an explicit timestamp.
    ///
    /// Returns the appended record.
    pub fn append(
        &mut self,
        event_kind: impl Into<String>,
        subject: impl Into<String>,
        timestamp: Timestamp,
    ) -> &AuditRecord {
        let sequence = self.records.len() as u64;
        let previous_hash = self
            .records
            .last()
            .map(|record| record.current_hash.clone())
            .unwrap_or_else(|| GENESIS_PREVIOUS_HASH.to_string());
        let event_kind = event_kind.into();
        let subject = subject.into();
        let current_hash = hash_record(sequence, timestamp, &event_kind, &subject, &previous_hash);

        self.records.push(AuditRecord {
            sequence,
            timestamp,
            event_kind,
            subject,
            previous_hash,
            current_hash,
        });
        self.records
            .last()
            .expect("record was just pushed and must be present")
    }

    /// The appended records, in order.
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the chain has no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The most recent record, if any.
    pub fn last(&self) -> Option<&AuditRecord> {
        self.records.last()
    }

    /// Verify the entire chain.
    ///
    /// Checks sequence continuity, genesis linkage, previous-hash pointers, and
    /// each record's current hash. An empty chain verifies successfully.
    pub fn verify(&self) -> Result<(), AuditChainError> {
        for (index, record) in self.records.iter().enumerate() {
            let expected_sequence = index as u64;
            if record.sequence != expected_sequence {
                return Err(AuditChainError::SequenceMismatch {
                    index,
                    expected: expected_sequence,
                    found: record.sequence,
                });
            }

            if index == 0 {
                if record.previous_hash != GENESIS_PREVIOUS_HASH {
                    return Err(AuditChainError::GenesisHashMismatch { index });
                }
            } else if record.previous_hash != self.records[index - 1].current_hash {
                return Err(AuditChainError::PreviousHashMismatch { index });
            }

            let expected_hash = hash_record(
                record.sequence,
                record.timestamp,
                &record.event_kind,
                &record.subject,
                &record.previous_hash,
            );
            if record.current_hash != expected_hash {
                return Err(AuditChainError::RecordHashMismatch { index });
            }
        }
        Ok(())
    }
}

impl fmt::Display for AuditHashChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AuditHashChain(records: {}, head: {})",
            self.records.len(),
            self.records
                .last()
                .map(|record| record.current_hash.as_str())
                .unwrap_or("none")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::Timestamp;

    fn ts(millis: i64) -> Timestamp {
        Timestamp::from_epoch_millis(millis).unwrap()
    }

    #[test]
    fn empty_chain_verifies() {
        let chain = AuditHashChain::new();
        assert!(chain.is_empty());
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn single_append_verifies() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.records()[0].previous_hash, GENESIS_PREVIOUS_HASH);
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn multiple_appends_verify() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        chain.append("governance.evaluated", "tool.shell", ts(1_700_000_000_001));
        chain.append(
            "capability.dispatched",
            "tool.calculator",
            ts(1_700_000_000_002),
        );
        assert_eq!(chain.len(), 3);
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn same_inputs_produce_deterministic_chains() {
        let inputs = [
            ("boot_system", "system", ts(1_700_000_000_000)),
            ("governance.evaluated", "tool.shell", ts(1_700_000_000_001)),
        ];

        let mut a = AuditHashChain::new();
        let mut b = AuditHashChain::new();
        for (event_kind, subject, timestamp) in inputs {
            a.append(event_kind, subject, timestamp);
            b.append(event_kind, subject, timestamp);
        }

        assert_eq!(a.records(), b.records());
        assert_eq!(a.records()[1].current_hash, b.records()[1].current_hash);
    }

    #[test]
    fn payload_tamper_fails() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        chain.append("user_login", "alice", ts(1_700_000_000_001));
        let mut tampered = chain.clone();
        tampered.records[1].event_kind = "tampered_action".into();
        assert!(matches!(
            tampered.verify(),
            Err(AuditChainError::RecordHashMismatch { index: 1 })
        ));
    }

    #[test]
    fn previous_hash_tamper_fails() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        chain.append("user_login", "alice", ts(1_700_000_000_001));
        let mut tampered = chain.clone();
        tampered.records[1].previous_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".into();
        assert!(matches!(
            tampered.verify(),
            Err(AuditChainError::PreviousHashMismatch { index: 1 })
        ));
    }

    #[test]
    fn reorder_fails() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        chain.append("user_login", "alice", ts(1_700_000_000_001));
        chain.append("execute_tool_shell", "alice", ts(1_700_000_000_002));
        let mut tampered = chain.clone();
        tampered.records.swap(1, 2);
        assert!(tampered.verify().is_err());
    }

    #[test]
    fn remove_middle_fails() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        chain.append("user_login", "alice", ts(1_700_000_000_001));
        chain.append("execute_tool_shell", "alice", ts(1_700_000_000_002));
        let mut tampered = chain.clone();
        tampered.records.remove(1);
        assert!(tampered.verify().is_err());
    }

    #[test]
    fn display_does_not_hide_or_leak_surprising_state() {
        let chain = AuditHashChain::new();
        assert!(chain.to_string().contains("records: 0"));
    }
}
