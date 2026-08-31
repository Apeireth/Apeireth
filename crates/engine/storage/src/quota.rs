//! Shadow / snapshot quota recovered from `legacy/archived/apeireth-rollback`.
//!
//! The canonical SnapshotService (file copy + git2) is not ported. This module
//! keeps the 71GB-incident quota algorithm: per-item TTL, per-item size cap,
//! total size cap, and LRU eviction of the oldest entries until the total
//! fits. Callers own the actual bytes on disk.

use std::cmp::Ordering;

/// Default per-shadow age cap (7 days). Engine `MAX_SHADOW_AGE_DAYS`.
pub const MAX_SHADOW_AGE_DAYS: u64 = 7;
/// Default per-shadow size cap (100 MiB). Engine `MAX_SHADOW_SIZE_BYTES`.
pub const MAX_SHADOW_SIZE_BYTES: u64 = 100 * 1024 * 1024;
/// Default total shadow size cap (2 GiB). Engine `MAX_TOTAL_SHADOW_SIZE_BYTES`.
pub const MAX_TOTAL_SHADOW_SIZE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Quota configuration. Defaults match the canonical 71GB-incident constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaConfig {
    /// Maximum age in days. Entries older than this are expired.
    pub max_age_days: u64,
    /// Maximum size of a single entry in bytes.
    pub max_item_bytes: u64,
    /// Maximum sum of live entry sizes in bytes.
    pub max_total_bytes: u64,
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self {
            max_age_days: MAX_SHADOW_AGE_DAYS,
            max_item_bytes: MAX_SHADOW_SIZE_BYTES,
            max_total_bytes: MAX_TOTAL_SHADOW_SIZE_BYTES,
        }
    }
}

/// One quota-tracked item. The caller supplies identity, age, and size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaItem {
    /// Stable identity (snapshot id, file path, …).
    pub id: String,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
    /// Occupied bytes.
    pub size_bytes: u64,
}

impl QuotaItem {
    /// Age in seconds relative to `now_unix`.
    pub fn age_seconds(&self, now_unix: u64) -> u64 {
        now_unix.saturating_sub(self.timestamp)
    }

    /// Age in whole days relative to `now_unix`.
    pub fn age_days(&self, now_unix: u64) -> u64 {
        self.age_seconds(now_unix) / 86_400
    }

    /// True when older than `cfg.max_age_days`.
    pub fn is_expired(&self, now_unix: u64, cfg: &QuotaConfig) -> bool {
        self.age_days(now_unix) > cfg.max_age_days
    }
}

/// Quota violation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QuotaError {
    /// A single item is larger than the per-item cap.
    #[error("item too large: {actual} bytes (max {max})")]
    ItemTooLarge {
        /// Actual size.
        actual: u64,
        /// Configured max.
        max: u64,
    },
    /// After TTL expiry + LRU, the remaining total still overflows.
    #[error("total size {actual} bytes overflows {max} even after LRU")]
    TotalOverflow {
        /// Remaining total.
        actual: u64,
        /// Configured max.
        max: u64,
    },
}

/// Result of applying TTL + size + LRU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaDecision {
    /// Items that may stay.
    pub keep: Vec<QuotaItem>,
    /// Items expired by TTL (age).
    pub expired: Vec<QuotaItem>,
    /// Items evicted by LRU to fit `max_total_bytes` (oldest first).
    pub evicted: Vec<QuotaItem>,
}

impl QuotaDecision {
    /// Bytes that remain after the decision.
    pub fn remaining_bytes(&self) -> u64 {
        self.keep.iter().map(|i| i.size_bytes).sum()
    }
}

/// Reject a single item that exceeds the per-item cap.
pub fn check_item_size(size_bytes: u64, cfg: &QuotaConfig) -> Result<(), QuotaError> {
    if size_bytes > cfg.max_item_bytes {
        Err(QuotaError::ItemTooLarge {
            actual: size_bytes,
            max: cfg.max_item_bytes,
        })
    } else {
        Ok(())
    }
}

/// Apply TTL expiry then LRU eviction until the live set fits `max_total_bytes`.
///
/// Order:
/// 1. Drop items whose age (days) exceeds `max_age_days`.
/// 2. If the remainder still exceeds `max_total_bytes`, evict oldest-first
///    (timestamp ascending, then id) until it fits.
/// 3. If a single remaining item is itself larger than `max_total_bytes`,
///    report [`QuotaError::TotalOverflow`] — LRU cannot help.
pub fn enforce_quota(
    items: Vec<QuotaItem>,
    now_unix: u64,
    cfg: &QuotaConfig,
) -> Result<QuotaDecision, QuotaError> {
    let mut expired = Vec::new();
    let mut live = Vec::new();
    for item in items {
        if item.is_expired(now_unix, cfg) {
            expired.push(item);
        } else {
            live.push(item);
        }
    }

    live.sort_by(|a, b| match a.timestamp.cmp(&b.timestamp) {
        Ordering::Equal => a.id.cmp(&b.id),
        other => other,
    });

    let mut evicted = Vec::new();
    let mut total: u64 = live.iter().map(|i| i.size_bytes).sum();
    let mut idx = 0;
    while total > cfg.max_total_bytes && idx < live.len() {
        if live[idx].size_bytes > cfg.max_total_bytes && live.len() - idx == 1 {
            return Err(QuotaError::TotalOverflow {
                actual: total,
                max: cfg.max_total_bytes,
            });
        }
        let victim = live.remove(idx);
        total = total.saturating_sub(victim.size_bytes);
        evicted.push(victim);
        // idx stays: the next oldest is now at the same position.
    }

    if total > cfg.max_total_bytes {
        return Err(QuotaError::TotalOverflow {
            actual: total,
            max: cfg.max_total_bytes,
        });
    }

    Ok(QuotaDecision {
        keep: live,
        expired,
        evicted,
    })
}

/// Ids that [`enforce_quota`] decided to drop (expired ∪ evicted).
pub fn ids_to_delete(decision: &QuotaDecision) -> Vec<&str> {
    decision
        .expired
        .iter()
        .chain(decision.evicted.iter())
        .map(|i| item_id(i))
        .collect()
}

fn item_id(item: &QuotaItem) -> &str {
    &item.id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, ts: u64, size: u64) -> QuotaItem {
        QuotaItem {
            id: id.to_string(),
            timestamp: ts,
            size_bytes: size,
        }
    }

    fn cfg(max_age_days: u64, max_item: u64, max_total: u64) -> QuotaConfig {
        QuotaConfig {
            max_age_days,
            max_item_bytes: max_item,
            max_total_bytes: max_total,
        }
    }

    #[test]
    fn defaults_match_donor_71gb_constants() {
        let c = QuotaConfig::default();
        assert_eq!(c.max_age_days, 7);
        assert_eq!(c.max_item_bytes, 100 * 1024 * 1024);
        assert_eq!(c.max_total_bytes, 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn check_item_size_rejects_over_cap() {
        let c = cfg(7, 100, 1000);
        assert!(check_item_size(100, &c).is_ok());
        assert!(matches!(
            check_item_size(101, &c),
            Err(QuotaError::ItemTooLarge {
                actual: 101,
                max: 100
            })
        ));
    }

    #[test]
    fn ttl_expires_items_older_than_max_age_days() {
        let now = 10 * 86_400;
        let c = cfg(7, 1000, 10_000);
        let items = vec![item("old", 0, 10), item("fresh", now - 86_400, 10)];
        let d = enforce_quota(items, now, &c).unwrap();
        assert_eq!(
            d.expired.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["old"]
        );
        assert_eq!(
            d.keep.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["fresh"]
        );
        assert!(d.evicted.is_empty());
    }

    #[test]
    fn lru_evicts_oldest_until_total_fits() {
        let now = 1_000;
        let c = cfg(7, 1000, 25);
        let items = vec![item("a", 1, 10), item("b", 2, 10), item("c", 3, 10)];
        let d = enforce_quota(items, now, &c).unwrap();
        assert!(d.expired.is_empty());
        assert_eq!(
            d.evicted.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["a"]
        );
        assert_eq!(
            d.keep.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert_eq!(d.remaining_bytes(), 20);
    }

    #[test]
    fn lru_evicts_multiple_oldest() {
        let now = 1_000;
        let c = cfg(7, 1000, 10);
        let items = vec![item("a", 1, 10), item("b", 2, 10), item("c", 3, 10)];
        let d = enforce_quota(items, now, &c).unwrap();
        assert_eq!(
            d.evicted.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(
            d.keep.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["c"]
        );
    }

    #[test]
    fn ttl_runs_before_lru() {
        let now = 10 * 86_400;
        let c = cfg(7, 1000, 10);
        let items = vec![item("expired-big", 0, 100), item("keep", now, 10)];
        let d = enforce_quota(items, now, &c).unwrap();
        assert_eq!(d.expired[0].id, "expired-big");
        assert!(d.evicted.is_empty());
        assert_eq!(d.keep[0].id, "keep");
    }

    #[test]
    fn single_item_larger_than_total_cap_is_overflow() {
        let now = 1_000;
        let c = cfg(7, 10_000, 5);
        let items = vec![item("huge", 1, 10)];
        assert!(matches!(
            enforce_quota(items, now, &c),
            Err(QuotaError::TotalOverflow { actual: 10, max: 5 })
        ));
    }

    #[test]
    fn ids_to_delete_unions_expired_and_evicted() {
        let now = 10 * 86_400;
        let c = cfg(7, 1000, 10);
        let items = vec![
            item("old", 0, 10),
            item("a", now, 10),
            item("b", now + 1, 10),
        ];
        let d = enforce_quota(items, now, &c).unwrap();
        let mut ids: Vec<&str> = ids_to_delete(&d);
        ids.sort();
        assert_eq!(ids, ["a", "old"]);
    }

    #[test]
    fn empty_set_is_ok() {
        let d = enforce_quota(Vec::new(), 0, &QuotaConfig::default()).unwrap();
        assert!(d.keep.is_empty());
        assert_eq!(d.remaining_bytes(), 0);
    }
}
