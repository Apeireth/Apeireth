//! Time primitives.
//!
//! The [`Clock`] trait is *re-exported* from [`crate::clock`] rather than
//! redefined. A second `Clock` in the same crate would be precisely the
//! duplicated-abstraction pattern this convergence exists to remove, and the
//! existing one is already the right shape: a trait plus a real clock plus a
//! virtual clock that can be advanced without waiting.
//!
//! Every canonical subsystem takes `Arc<dyn Clock>` rather than calling
//! `Utc::now()`, which is what makes the deterministic end-to-end test possible.

use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

pub use crate::clock::{system_clock, Clock, SystemClock, VirtualClock};

/// A UTC instant.
///
/// A newtype over `DateTime<Utc>` so that the canonical contracts do not spread a
/// third-party type across every public signature, and so the wire representation
/// (RFC 3339) is fixed in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Read the current instant from a clock.
    ///
    /// Prefer this over [`Timestamp::now`] anywhere the result is observable, so
    /// the caller stays testable.
    pub fn from_clock(clock: &dyn Clock) -> Self {
        Self(clock.now())
    }

    /// Read the current instant from the system clock.
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Wrap an existing `DateTime<Utc>`.
    pub const fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }

    /// Build from Unix epoch milliseconds, or `None` if out of range.
    pub fn from_epoch_millis(ms: i64) -> Option<Self> {
        match Utc.timestamp_millis_opt(ms) {
            chrono::LocalResult::Single(dt) => Some(Self(dt)),
            _ => None,
        }
    }

    /// The underlying `DateTime<Utc>`.
    pub const fn as_datetime(&self) -> DateTime<Utc> {
        self.0
    }

    /// Unix epoch milliseconds.
    pub fn epoch_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn timestamp_round_trips_through_epoch_millis() {
        let ts = Timestamp::from_epoch_millis(1_700_000_000_000).unwrap();
        assert_eq!(ts.epoch_millis(), 1_700_000_000_000);
    }

    #[test]
    fn timestamp_reads_from_an_injected_clock() {
        let start = Timestamp::from_epoch_millis(1_000_000)
            .unwrap()
            .as_datetime();
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(start));

        let t0 = Timestamp::from_clock(clock.as_ref());
        assert_eq!(t0.as_datetime(), start);
    }

    #[test]
    fn virtual_clock_advances_without_waiting() {
        let start = Timestamp::from_epoch_millis(1_000_000)
            .unwrap()
            .as_datetime();
        let clock = VirtualClock::new(start);

        let before = Timestamp::from_clock(&clock);
        clock.advance(chrono::Duration::seconds(90));
        let after = Timestamp::from_clock(&clock);

        assert_eq!(after.epoch_millis() - before.epoch_millis(), 90_000);
    }

    #[test]
    fn display_is_rfc3339() {
        let ts = Timestamp::from_epoch_millis(0).unwrap();
        assert!(ts.to_string().starts_with("1970-01-01T00:00:00"), "{ts}");
    }
}
