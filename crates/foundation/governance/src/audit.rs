//! Tamper-evident audit hash chain.
//!
//! This is an **integrity primitive**, not the runtime execution trace. The
//! canonical `ExecutionTrace` remains in `apeireth-runtime::canonical`; this
//! module only proves that an ordered sequence of records has not been mutated
//! after it was appended.
//!
//! # Where the primitive lives (R5)
//!
//! The **hash-chain primitive** (byte layout + SHA-256 + chaining) now lives in
//! `apeireth_core::hash_chain`: it is pure cryptography with no policy, so the
//! memory layer can use it without depending on governance. This module keeps
//! the **governance policy** (sequence continuity, record contents, error
//! variants) and only translates its fields into that generic layout ⇒ the
//! digest values are byte-for-byte unchanged (asserted in the tests below
//! against both the legacy `sha2` path and fixed historical constants).
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

use apeireth_core::hash_chain::{hash_linked, Chunk};
use apeireth_core::kernel::Timestamp;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The fixed previous hash for the first record.
///
/// A zero hash is unambiguous and does not pretend to prove anything outside
/// the chain itself.
///
/// The value now lives in `apeireth_core::hash_chain` (R5: the chain primitive
/// moved down to core so the memory layer can use it without a cross-domain
/// dependency). This alias keeps the governance-level constant — and its
/// value — byte-for-byte unchanged.
pub const GENESIS_PREVIOUS_HASH: &str = apeireth_core::hash_chain::GENESIS_PREVIOUS_HASH;

/// Compute one record's digest.
///
/// The byte layout lives in `apeireth_core::hash_chain` (R5); this function
/// only translates governance fields into that generic form and computes
/// **nothing** on its own ⇒ the digest values stay byte-for-byte identical to
/// the pre-R5 implementation (the audit ledger is an accounting book; a moved
/// byte is an incident).
fn hash_record(
    sequence: u64,
    timestamp: Timestamp,
    event_kind: &str,
    subject: &str,
    previous_hash: &str,
) -> String {
    hash_linked(
        previous_hash,
        &[
            Chunk::Raw(&sequence.to_be_bytes()),
            Chunk::Raw(&timestamp.epoch_millis().to_be_bytes()),
            Chunk::LenPrefixed(event_kind.as_bytes()),
            Chunk::LenPrefixed(subject.as_bytes()),
        ],
    )
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
    use sha2::{Digest, Sha256};

    fn ts(millis: i64) -> Timestamp {
        Timestamp::from_epoch_millis(millis).unwrap()
    }

    /// 旧路径（治理自带的 `sha2` 实现）——**只作对照**，证明 R5 下沉到 core
    /// 之后摘要值逐字节不变。
    ///
    /// 为什么留这份对照：审计账本是历史账本，「值没变」这件事必须有机器证据，
    /// 而不是靠人读代码确认「看起来一样」。
    fn hash_record_legacy(
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

    // -- R5: 下沉到 core 之后，摘要值必须逐字节不变 ---------------------------

    #[test]
    fn core_primitive_matches_legacy_sha2_path_byte_for_byte() {
        let inputs = [
            ("boot_system", "system", ts(1_700_000_000_000)),
            ("governance.evaluated", "tool.shell", ts(1_700_000_000_001)),
            (
                "capability.dispatched",
                "tool.calculator",
                ts(1_700_000_000_002),
            ),
            ("user_login", "alice", ts(1_700_000_000_003)),
        ];

        let mut chain = AuditHashChain::new();
        for (event_kind, subject, timestamp) in inputs {
            let record = chain.append(event_kind, subject, timestamp);
            let legacy = hash_record_legacy(
                record.sequence,
                record.timestamp,
                &record.event_kind,
                &record.subject,
                &record.previous_hash,
            );
            assert_eq!(
                record.current_hash, legacy,
                "审计账本的摘要值不能变：core 原语必须与旧 sha2 路径逐字节相同"
            );
        }
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn historical_digest_constants_are_unchanged() {
        // 常量锚点：取自下沉**之前**的实现，不是现算的。布局动一个字节即红。
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system", ts(1_700_000_000_000));
        chain.append("governance.evaluated", "tool.shell", ts(1_700_000_000_001));
        chain.append(
            "capability.dispatched",
            "tool.calculator",
            ts(1_700_000_000_002),
        );
        let records = chain.records();
        assert_eq!(
            records[0].current_hash,
            "0edff970aed2df8f22b0ba07e6ad4fce9087d5ffa5e60a4bc631d04ba7697d9e"
        );
        assert_eq!(
            records[1].current_hash,
            "f70b452d981613ba29faa08f1f286e889ceef81108c5f1ad8baece184db7b137"
        );
        assert_eq!(
            records[2].current_hash,
            "8c69db0599fe9bc59a0c0173f21373e24fa057b7c16d6a49b430b6d2398d94f9"
        );
    }
}
