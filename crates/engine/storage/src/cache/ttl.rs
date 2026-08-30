//! Lazy TTL entries. Eager background scanning is not ported (needs a runtime).

use std::time::{Duration, Instant};

/// TTL evaluation mode. Eager scanning is recorded but not run by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TtlMode {
    /// Expire on get.
    Lazy,
    /// Background scan (not implemented here; flag only).
    Eager,
    /// Lazy + eager.
    Both,
}

impl TtlMode {
    /// All modes.
    pub const ALL: [TtlMode; 3] = [TtlMode::Lazy, TtlMode::Eager, TtlMode::Both];

    /// Lazy check enabled?
    pub const fn is_lazy_enabled(&self) -> bool {
        matches!(self, TtlMode::Lazy | TtlMode::Both)
    }

    /// Eager scan enabled?
    pub const fn is_eager_enabled(&self) -> bool {
        matches!(self, TtlMode::Eager | TtlMode::Both)
    }
}

impl Default for TtlMode {
    fn default() -> Self {
        TtlMode::Both
    }
}

impl std::fmt::Display for TtlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtlMode::Lazy => f.write_str("lazy"),
            TtlMode::Eager => f.write_str("eager"),
            TtlMode::Both => f.write_str("both"),
        }
    }
}

/// Value plus insertion time plus TTL.
#[derive(Debug, Clone)]
pub struct TtlEntry<V> {
    value: V,
    inserted_at: Instant,
    ttl: Duration,
}

impl<V> TtlEntry<V> {
    /// Insert now.
    pub fn new(value: V, ttl: Duration) -> Self {
        Self::with_inserted_at(value, ttl, Instant::now())
    }

    /// Insert at an explicit clock (tests / restore).
    pub fn with_inserted_at(value: V, ttl: Duration, inserted_at: Instant) -> Self {
        Self {
            value,
            inserted_at,
            ttl,
        }
    }

    /// Borrow the value.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Consume the value.
    pub fn into_value(self) -> V {
        self.value
    }

    /// Configured TTL.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Insertion instant.
    pub fn inserted_at(&self) -> Instant {
        self.inserted_at
    }

    /// Expired relative to now.
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    /// Expired relative to `now`.
    pub fn is_expired_at(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.inserted_at) >= self.ttl
    }

    /// Remaining TTL, or `None` if expired.
    pub fn remaining(&self) -> Option<Duration> {
        self.remaining_at(Instant::now())
    }

    /// Remaining TTL at `now`.
    pub fn remaining_at(&self, now: Instant) -> Option<Duration> {
        let elapsed = now.saturating_duration_since(self.inserted_at);
        if elapsed >= self.ttl {
            None
        } else {
            Some(self.ttl - elapsed)
        }
    }
}

/// TTL policy (eager interval is advisory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtlPolicy {
    /// Mode.
    pub mode: TtlMode,
    /// Eager scan interval (unused by lazy-only MemoryCache).
    pub scan_interval: Duration,
}

impl TtlPolicy {
    /// Both + 1s scan.
    pub const fn default_policy() -> Self {
        Self {
            mode: TtlMode::Both,
            scan_interval: Duration::from_secs(1),
        }
    }

    /// Lazy only.
    pub const fn lazy_only() -> Self {
        Self {
            mode: TtlMode::Lazy,
            scan_interval: Duration::from_secs(0),
        }
    }
}

impl Default for TtlPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_mode_flags() {
        assert!(TtlMode::Lazy.is_lazy_enabled());
        assert!(!TtlMode::Lazy.is_eager_enabled());
        assert!(!TtlMode::Eager.is_lazy_enabled());
        assert!(TtlMode::Eager.is_eager_enabled());
        assert!(TtlMode::Both.is_lazy_enabled());
        assert!(TtlMode::Both.is_eager_enabled());
        assert_eq!(TtlMode::ALL.len(), 3);
    }

    #[test]
    fn ttl_entry_not_expired() {
        let entry = TtlEntry::new("hello".to_string(), Duration::from_secs(60));
        assert!(!entry.is_expired());
        assert!(entry.remaining().is_some());
    }

    #[test]
    fn ttl_entry_expired_with_past_timestamp() {
        let past = Instant::now() - Duration::from_secs(120);
        let entry = TtlEntry::with_inserted_at("hello".to_string(), Duration::from_secs(60), past);
        assert!(entry.is_expired());
        assert!(entry.remaining().is_none());
    }

    #[test]
    fn ttl_entry_remaining() {
        let now = Instant::now();
        let entry = TtlEntry::with_inserted_at(
            "x".to_string(),
            Duration::from_secs(10),
            now - Duration::from_secs(3),
        );
        let rem = entry.remaining_at(now).unwrap();
        assert!(rem.as_secs() >= 6 && rem.as_secs() <= 7);
    }

    #[test]
    fn ttl_entry_into_value() {
        let entry = TtlEntry::new(String::from("v"), Duration::from_secs(1));
        assert_eq!(entry.into_value(), "v");
    }
}
