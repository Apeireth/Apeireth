//! Canonical memory domain (M1B1).
//!
//! The domain owns the durable memory entity and its typed identity. This is
//! the only canonical representation of a memory item for future memory
//! retrieval and indexing work. The legacy crate root carries pre-canonical
//! memory types; new code must use this module instead.

use serde::{Deserialize, Serialize};

use apeireth_core::kernel::Timestamp;

use super::error::MemoryError;

/// Typed identifier for one canonical memory item.
///
/// The identifier is deliberately small: it is a validated non-empty string.
/// Core owns the generated/stability-tested identifier primitives; a
/// memory-specific id is introduced here because memory ids are persisted
/// domain state and core does not currently have one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemoryId(String);

impl MemoryId {
    /// Validates and wraps a memory identifier.
    ///
    /// The only constraints are that the id is non-empty, at most 256 bytes,
    /// and contains no control characters. Memory ids are caller-chosen domain
    /// keys, not capability ids, so the full stable-id grammar is deliberately
    /// not reused here.
    pub fn new(raw: impl Into<String>) -> Result<Self, MemoryError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(MemoryError::InvalidData(
                "memory id must not be empty".into(),
            ));
        }
        if raw.len() > 256 {
            return Err(MemoryError::InvalidData(
                "memory id must be at most 256 bytes".into(),
            ));
        }
        if raw.chars().any(char::is_control) {
            return Err(MemoryError::InvalidData(
                "memory id must not contain control characters".into(),
            ));
        }
        Ok(Self(raw))
    }

    /// Creates a `MemoryId` from a value that was already validated when it
    /// was written. This is intentionally `pub(crate)`; external callers must
    /// go through [`MemoryId::new`].
    pub(crate) fn from_validated(raw: String) -> Self {
        Self(raw)
    }

    /// The identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the id and returns the underlying `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A durable memory item.
///
/// Field-by-field justification:
///
/// - `id`: the domain key; persisted identity.
/// - `data`: the memory content itself.
/// - `importance`: canonical domain metadata. Persisted now, ranked in
///   M1B2; not interpreted in M1B1.
/// - `access_count`: canonical domain metadata used for ACT-R-style
///   retrieval in M1B2.
/// - `access_times`: canonical domain metadata; explicit timestamps for
///   retrieval. `Timestamp` is the canonical core time type.
/// - `created_at`: creation time, supplied by the caller or a canonical
///   `Clock`; never read from the wall clock inside the domain.
/// - `valid_from`: inclusive start of the temporal validity window.
/// - `valid_until`: exclusive end of the temporal validity window; `None`
///   means valid indefinitely.
/// - `is_tombstone`: whether the item is deleted/tombstoned. Normal
///   read/query paths exclude tombstones unless explicitly asked.
/// - `artifact_sig`: optional content/provenance fingerprint supplied by the
///   caller. Automatic hashing is deliberately not performed in the domain;
///   it is a content-addressing policy that can be layered later without
///   changing the entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: MemoryId,
    pub data: String,
    pub importance: f64,
    pub access_count: u32,
    pub access_times: Vec<Timestamp>,
    pub created_at: Timestamp,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
    pub is_tombstone: bool,
    pub artifact_sig: Option<String>,
}

impl MemoryItem {
    /// Creates a new item with conservative defaults.
    ///
    /// The caller still owns the temporal validity decision. By default an
    /// item is valid from its creation time and has no expiration.
    pub fn new(id: MemoryId, data: impl Into<String>, created_at: Timestamp) -> Self {
        Self {
            id,
            data: data.into(),
            importance: 0.0,
            access_count: 0,
            access_times: Vec::new(),
            created_at,
            valid_from: created_at,
            valid_until: None,
            is_tombstone: false,
            artifact_sig: None,
        }
    }

    /// Validates the parts of the entity that cannot be represented by the
    /// type system alone.
    pub fn validate(&self) -> Result<(), MemoryError> {
        if self.data.is_empty() {
            return Err(MemoryError::InvalidData(
                "memory data must not be empty".into(),
            ));
        }
        if !self.importance.is_finite() {
            return Err(MemoryError::InvalidData(
                "memory importance must be finite".into(),
            ));
        }
        if let Some(valid_until) = self.valid_until {
            if valid_until <= self.valid_from {
                return Err(MemoryError::InvalidData(
                    "memory valid_until must be strictly after valid_from".into(),
                ));
            }
        }
        Ok(())
    }

    /// Deterministic temporal validity filter.
    ///
    /// Boundary semantics:
    ///
    /// - `as_of < valid_from`: excluded.
    /// - `as_of == valid_from`: included (`valid_from` is inclusive).
    /// - `valid_until` is exclusive: `as_of < valid_until` is included.
    /// - `as_of == valid_until`: excluded.
    /// - `valid_until == None`: included for any `as_of >= valid_from`.
    pub fn is_effective_at(&self, as_of: Timestamp) -> bool {
        as_of >= self.valid_from && self.valid_until.map_or(true, |until| as_of < until)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::Timestamp;

    fn ts(ms: i64) -> Timestamp {
        Timestamp::from_epoch_millis(ms).unwrap()
    }

    #[test]
    fn temporal_validity_boundaries_are_defined() {
        let item = MemoryItem {
            id: MemoryId::new("m1").unwrap(),
            data: "x".into(),
            importance: 0.0,
            access_count: 0,
            access_times: vec![],
            created_at: ts(100),
            valid_from: ts(100),
            valid_until: Some(ts(200)),
            is_tombstone: false,
            artifact_sig: None,
        };

        assert!(!item.is_effective_at(ts(99)));
        assert!(item.is_effective_at(ts(100)));
        assert!(item.is_effective_at(ts(199)));
        assert!(!item.is_effective_at(ts(200)));

        let indefinite = MemoryItem {
            valid_until: None,
            ..item
        };
        assert!(indefinite.is_effective_at(ts(100)));
        assert!(indefinite.is_effective_at(ts(999)));
    }

    #[test]
    fn item_validation_rejects_bad_domain_data() {
        let item = MemoryItem::new(MemoryId::new("m1").unwrap(), "x", ts(100));
        assert!(item.validate().is_ok());

        let empty_data = MemoryItem::new(MemoryId::new("m2").unwrap(), "", ts(100));
        assert!(matches!(
            empty_data.validate(),
            Err(MemoryError::InvalidData(_))
        ));

        let nan_importance = MemoryItem {
            importance: f64::NAN,
            ..MemoryItem::new(MemoryId::new("m3").unwrap(), "x", ts(100))
        };
        assert!(matches!(
            nan_importance.validate(),
            Err(MemoryError::InvalidData(_))
        ));

        let backwards_window = MemoryItem {
            valid_until: Some(ts(100)),
            valid_from: ts(200),
            ..MemoryItem::new(MemoryId::new("m4").unwrap(), "x", ts(300))
        };
        assert!(matches!(
            backwards_window.validate(),
            Err(MemoryError::InvalidData(_))
        ));
    }

    #[test]
    fn memory_id_validates_length_and_content() {
        assert!(MemoryId::new("").is_err());
        assert!(MemoryId::new("a").is_ok());
        assert!(MemoryId::new("a".repeat(257)).is_err());
        assert!(MemoryId::new("line\nbreak").is_err());
    }
}
