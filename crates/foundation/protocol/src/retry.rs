//! Status-aware retry semantics: retryable-status classification, backoff
//! policies, and jittered sleep computation.
//!
//! Recovered from the legacy `apeireth-api::retry` module (R120/R121/R122):
//! a field-faithful translation of the OpenAI/Anthropic SDK retry patterns:
//! - **Status classification**: 4xx never retried except the whitelist
//!   `[408, 425, 429]`; 5xx always retried; `0` (network error, no status)
//!   always retried; 2xx/3xx never.
//! - **Backoff tiers** (compile-time hardcoded): `Aggressive` 1s/3s/10s,
//!   `Default` +30s, `Patient` +2m/+10m (Anthropic TS SDK shape; the legacy
//!   default, "reliable over fast"), `Custom`.
//! - **Jitter modes** (AWS SDK patterns): None / Full / Equal / Decorrelated.
//!
//! Differences from the donor, by design:
//! - The donor wrapped jitter in a `WithJitter(Box<BackoffPolicy>, JitterMode>)`
//!   variant purely to keep 1.0 call sites pattern-matchable. v2 has no such
//!   compatibility surface, so jitter is a plain field on [`RetryPolicy`].
//! - The donor's metrics came from `apeireth_telemetry::Counter`; here
//!   [`RetryCounters`] is a dependency-free atomic counter triple with the
//!   same three observables (attempts / exhausted / success_after).
//! - The donor drew randomness from a thread-local xorshift; here
//!   [`XorShift64`] is a seedable, injectable PRNG (same xorshift64
//!   constants), so jitter is deterministic under test and carries no
//!   hidden global state.
//!
//! Pure computation only — sleeping is the caller's job (this crate has no
//! async runtime by contract).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 4xx status codes that ARE retryable.
///
/// - `408` Request Timeout — upstream slow
/// - `425` Too Early — upstream-specific retry semantics
/// - `429` Too Many Requests — rate limiting (the main path)
pub const RETRYABLE_4XX: [u16; 3] = [408, 425, 429];

/// Classify an HTTP status as retryable.
///
/// Legacy 1:1 (`apeireth-api::retry::should_retry_status`):
/// - `0` = network error (send / read-body failed; no status available) → retry
/// - `5xx` → retry
/// - `4xx` → retry only when in [`RETRYABLE_4XX`]
/// - `2xx` / `3xx` → no retry
#[must_use]
pub fn should_retry_status(status: u16) -> bool {
    if status == 0 {
        // network error (send / read body failed)
        return true;
    }
    if (500..600).contains(&status) {
        return true;
    }
    if (400..500).contains(&status) {
        return RETRYABLE_4XX.contains(&status);
    }
    false
}

/// Backoff tier schedules (compile-time hardcoded durations).
///
/// Legacy 1:1 translation of the OpenAI Python SDK (`Default`) and Anthropic
/// TypeScript SDK (`Patient`) retry schedules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackoffPolicy {
    /// 1s / 3s / 10s (3 tiers; the legacy 1.0 behavior).
    Aggressive,
    /// 1s / 3s / 10s / 30s (4 tiers; OpenAI Python SDK 1:1).
    Default,
    /// 1s / 3s / 10s / 30s / 2m / 10m (6 tiers; Anthropic TS SDK 1:1;
    /// the legacy default — reliable over fast).
    Patient,
    /// Caller-provided tiers (critical paths tune their own schedule).
    Custom(Vec<Duration>),
}

impl BackoffPolicy {
    /// The tier durations in attempt order.
    pub fn to_durations(&self) -> Vec<Duration> {
        match self {
            BackoffPolicy::Aggressive => vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
            ],
            BackoffPolicy::Default => vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
            ],
            BackoffPolicy::Patient => vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
                Duration::from_secs(120),
                Duration::from_secs(600),
            ],
            BackoffPolicy::Custom(d) => d.clone(),
        }
    }

    /// Number of tiers.
    pub fn tier_count(&self) -> usize {
        self.to_durations().len()
    }
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        // Legacy decision #2: default Patient (reliable > fast).
        BackoffPolicy::Patient
    }
}

/// Jitter mode applied to each tier's sleep (AWS SDK retry patterns, 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JitterMode {
    /// No jitter — sleep exactly the tier duration (legacy 1.0 behavior).
    #[default]
    None,
    /// Full jitter: `sleep = random(0, base)`.
    Full,
    /// Equal jitter: `sleep = base/2 + random(0, base/2)`.
    Equal,
    /// Decorrelated jitter: `sleep = min(cap, random(base, prev*3))`.
    Decorrelated,
}

impl JitterMode {
    /// Stable name for logs / config.
    pub const fn as_str(&self) -> &'static str {
        match self {
            JitterMode::None => "none",
            JitterMode::Full => "full",
            JitterMode::Equal => "equal",
            JitterMode::Decorrelated => "decorrelated",
        }
    }
}

/// Seedable xorshift64 PRNG (legacy `fastrand_u64` without the thread-local).
///
/// Same shift constants as the donor (13 / 7 / 17). Not
/// cryptographically secure — jitter only, exactly as in the donor.
#[derive(Debug, Clone)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    /// Create from a nonzero seed (`0` is remapped to a fixed nonzero constant,
    /// mirroring the donor's initialization guard).
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed };
        Self { state }
    }

    /// Next pseudo-random u64.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform u64 in `[0, n]` (inclusive); returns `0` when `n == 0`.
    fn below_inclusive(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % (n + 1)
        }
    }
}

/// Compute one jittered sleep duration.
///
/// Legacy 1:1 (`apeireth-api::retry::jittered_sleep`) with randomness injected
/// via `rng` instead of a thread-local:
/// - `None`: `base` unchanged
/// - `Full`: `random(0, base)`
/// - `Equal`: `base/2 + random(0, base/2)`
/// - `Decorrelated`: `min(cap, random(base, prev*3))`, where a `prev` of `None`
///   degenerates the upper bound to `base`
///
/// `prev` is the previous actual sleep (`None` on the first retry); `cap` is
/// the longest tier duration (the decorrelated jitter ceiling).
#[must_use]
pub fn jittered_sleep(
    base: Duration,
    jitter: JitterMode,
    prev: Option<Duration>,
    cap: Duration,
    rng: &mut XorShift64,
) -> Duration {
    match jitter {
        JitterMode::None => base,
        JitterMode::Full => {
            let nanos = base.as_nanos() as u64;
            Duration::from_nanos(rng.below_inclusive(nanos))
        }
        JitterMode::Equal => {
            let half = base.as_nanos() as u64 / 2;
            if half == 0 {
                Duration::ZERO
            } else {
                Duration::from_nanos(half + rng.below_inclusive(half))
            }
        }
        JitterMode::Decorrelated => {
            let lo = base.as_nanos() as u64;
            let hi = match prev {
                None => lo,
                Some(p) => (p.as_nanos() as u64).saturating_mul(3).max(lo),
            };
            if hi <= lo {
                base.min(cap)
            } else {
                let r = lo + rng.next_u64() % (hi - lo + 1);
                Duration::from_nanos(r).min(cap)
            }
        }
    }
}

/// A backoff schedule plus the jitter mode to apply to it.
///
/// Legacy shape minus the compatibility shim: the donor modeled this as a
/// `WithJitter(Box<BackoffPolicy>, JitterMode)` enum variant; v2 has no legacy
/// match sites, so this is a plain pair.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetryPolicy {
    /// Tier schedule.
    pub backoff: BackoffPolicy,
    /// Jitter mode applied on top of each tier.
    pub jitter: JitterMode,
}

impl RetryPolicy {
    /// A policy with no jitter (legacy 1.0 behavior 1:1).
    pub fn new(backoff: BackoffPolicy) -> Self {
        Self {
            backoff,
            jitter: JitterMode::None,
        }
    }

    /// Attach a jitter mode (builder style, mirroring the donor's
    /// `BackoffPolicy::with_jitter` call shape).
    pub fn with_jitter(mut self, jitter: JitterMode) -> Self {
        self.jitter = jitter;
        self
    }

    /// Tier durations (delegates to the inner [`BackoffPolicy`]; jitter never
    /// changes the tier count, mirroring the donor's `WithJitter` semantics).
    pub fn to_durations(&self) -> Vec<Duration> {
        self.backoff.to_durations()
    }

    /// Tier count (mirrors [`BackoffPolicy::tier_count`]).
    pub fn tier_count(&self) -> usize {
        self.backoff.tier_count()
    }

    /// Compute the sleep for attempt `attempt` (0-based) given the previous
    /// actual sleep, drawing randomness from `rng`.
    ///
    /// Tiers beyond the schedule clamp to the last tier (the legacy retry
    /// loop never exceeded `backoffs.len()`, so this only matters for callers
    /// that keep looping).
    pub fn sleep_for(
        &self,
        attempt: usize,
        prev: Option<Duration>,
        rng: &mut XorShift64,
    ) -> Duration {
        let tiers = self.backoff.to_durations();
        if tiers.is_empty() {
            return Duration::ZERO;
        }
        let base = tiers[attempt.min(tiers.len() - 1)];
        let cap = tiers[tiers.len() - 1];
        jittered_sleep(base, self.jitter, prev, cap, rng)
    }
}

/// Retry observability counters (the legacy `RetryStats` triple without the
/// telemetry dependency).
///
/// - `attempts` — every retry attempt
/// - `exhausted` — all tiers consumed without success
/// - `success_after` — success following at least one failed attempt
#[derive(Debug, Default)]
pub struct RetryCounters {
    attempts: AtomicU64,
    exhausted: AtomicU64,
    success_after: AtomicU64,
}

impl RetryCounters {
    /// Create zeroed counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the retry-attempt counter.
    pub fn inc_attempt(&self) {
        self.attempts.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the backoff-exhausted counter.
    pub fn inc_exhausted(&self) {
        self.exhausted.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the success-after-retry counter.
    pub fn inc_success_after(&self) {
        self.success_after.fetch_add(1, Ordering::Relaxed);
    }

    /// Current attempt count.
    pub fn attempts(&self) -> u64 {
        self.attempts.load(Ordering::Relaxed)
    }

    /// Current exhausted count.
    pub fn exhausted(&self) -> u64 {
        self.exhausted.load(Ordering::Relaxed)
    }

    /// Current success-after count.
    pub fn success_after(&self) -> u64 {
        self.success_after.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- should_retry_status (ported 1:1 from donor) ----------

    #[test]
    fn should_retry_4xx_default_no() {
        assert!(!should_retry_status(400));
        assert!(!should_retry_status(401));
        assert!(!should_retry_status(403));
        assert!(!should_retry_status(404));
        assert!(!should_retry_status(422));
    }

    #[test]
    fn should_retry_4xx_whitelist_yes() {
        assert!(should_retry_status(408)); // Request Timeout
        assert!(should_retry_status(425)); // Too Early
        assert!(should_retry_status(429)); // Too Many Requests
    }

    #[test]
    fn should_retry_5xx_all_yes() {
        assert!(should_retry_status(500));
        assert!(should_retry_status(502));
        assert!(should_retry_status(503));
        assert!(should_retry_status(504));
        assert!(should_retry_status(599));
    }

    #[test]
    fn should_retry_2xx_3xx_no() {
        assert!(!should_retry_status(200));
        assert!(!should_retry_status(201));
        assert!(!should_retry_status(301));
        assert!(!should_retry_status(304));
    }

    #[test]
    fn should_retry_network_error_0_yes() {
        assert!(should_retry_status(0));
    }

    #[test]
    fn retryable_4xx_exactly_three() {
        let mut count = 0;
        for s in 400..500 {
            if should_retry_status(s) {
                count += 1;
            }
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn all_5xx_retryable() {
        for s in 500..600 {
            assert!(should_retry_status(s));
        }
    }

    // ---------- BackoffPolicy (ported 1:1) ----------

    #[test]
    fn backoff_aggressive_3_tiers() {
        let d = BackoffPolicy::Aggressive.to_durations();
        assert_eq!(
            d,
            vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10)
            ]
        );
        assert_eq!(BackoffPolicy::Aggressive.tier_count(), 3);
    }

    #[test]
    fn backoff_default_4_tiers() {
        let d = BackoffPolicy::Default.to_durations();
        assert_eq!(d.len(), 4);
        assert_eq!(d[3], Duration::from_secs(30));
        assert_eq!(BackoffPolicy::Default.tier_count(), 4);
    }

    #[test]
    fn backoff_patient_6_tiers() {
        let d = BackoffPolicy::Patient.to_durations();
        assert_eq!(
            d,
            vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
                Duration::from_secs(120),
                Duration::from_secs(600)
            ]
        );
        assert_eq!(BackoffPolicy::Patient.tier_count(), 6);
    }

    #[test]
    fn backoff_default_is_patient() {
        assert_eq!(BackoffPolicy::default(), BackoffPolicy::Patient);
    }

    #[test]
    fn backoff_patient_includes_default_includes_aggressive() {
        let patient = BackoffPolicy::Patient.to_durations();
        let default = BackoffPolicy::Default.to_durations();
        let aggressive = BackoffPolicy::Aggressive.to_durations();
        for (i, t) in aggressive.iter().enumerate() {
            assert_eq!(patient[i], *t);
            assert_eq!(default[i], *t);
        }
        assert_eq!(default[3], Duration::from_secs(30));
    }

    #[test]
    fn backoff_custom_user_defined() {
        let d = BackoffPolicy::Custom(vec![Duration::from_millis(500), Duration::from_secs(2)]);
        assert_eq!(d.tier_count(), 2);
        assert_eq!(d.to_durations()[0], Duration::from_millis(500));
    }

    // ---------- JitterMode + jittered_sleep ----------

    #[test]
    fn jitter_none_equals_base() {
        let mut rng = XorShift64::new(1);
        let base = Duration::from_secs(5);
        assert_eq!(
            jittered_sleep(base, JitterMode::None, None, Duration::from_secs(60), &mut rng),
            base
        );
    }

    #[test]
    fn jitter_full_in_range() {
        let mut rng = XorShift64::new(42);
        let base = Duration::from_secs(10);
        for _ in 0..50 {
            let r = jittered_sleep(base, JitterMode::Full, None, Duration::from_secs(60), &mut rng);
            assert!(r <= base, "Full jitter must be <= base");
        }
    }

    #[test]
    fn jitter_equal_in_range() {
        let mut rng = XorShift64::new(42);
        let base = Duration::from_secs(10);
        let half = base / 2;
        for _ in 0..50 {
            let r = jittered_sleep(base, JitterMode::Equal, None, Duration::from_secs(60), &mut rng);
            assert!(r >= half, "Equal jitter must be >= base/2");
            assert!(r <= base, "Equal jitter must be <= base");
        }
    }

    #[test]
    fn jitter_decorrelated_respects_cap_and_floor() {
        let mut rng = XorShift64::new(7);
        let base = Duration::from_secs(1);
        let prev = Duration::from_secs(5);
        let cap = Duration::from_secs(60);
        for _ in 0..50 {
            let r =
                jittered_sleep(base, JitterMode::Decorrelated, Some(prev), cap, &mut rng);
            assert!(r >= base, "Decorrelated must be >= base");
            assert!(r <= cap, "Decorrelated must respect cap");
        }
    }

    #[test]
    fn jitter_decorrelated_without_prev_uses_base_as_upper_bound() {
        let mut rng = XorShift64::new(7);
        let base = Duration::from_secs(3);
        let r = jittered_sleep(base, JitterMode::Decorrelated, None, Duration::from_secs(60), &mut rng);
        assert_eq!(r, base);
    }

    #[test]
    fn jitter_zero_base_returns_zero() {
        let mut rng = XorShift64::new(3);
        let cap = Duration::from_secs(60);
        assert_eq!(
            jittered_sleep(Duration::ZERO, JitterMode::None, None, cap, &mut rng),
            Duration::ZERO
        );
        assert_eq!(
            jittered_sleep(Duration::ZERO, JitterMode::Full, None, cap, &mut rng),
            Duration::ZERO
        );
        assert_eq!(
            jittered_sleep(Duration::ZERO, JitterMode::Equal, None, cap, &mut rng),
            Duration::ZERO
        );
    }

    #[test]
    fn jitter_mode_names_stable() {
        assert_eq!(JitterMode::None.as_str(), "none");
        assert_eq!(JitterMode::Full.as_str(), "full");
        assert_eq!(JitterMode::Equal.as_str(), "equal");
        assert_eq!(JitterMode::Decorrelated.as_str(), "decorrelated");
    }

    #[test]
    fn xorshift64_is_deterministic_and_nonzero_seeded() {
        let mut a = XorShift64::new(12345);
        let mut b = XorShift64::new(12345);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(a.next_u64(), 0);
        // zero seed remapped, still deterministic
        let mut c = XorShift64::new(0);
        let mut d = XorShift64::new(0);
        assert_eq!(c.next_u64(), d.next_u64());
    }

    // ---------- RetryPolicy (the de-shimmed WithJitter semantics) ----------

    #[test]
    fn retry_policy_default_jitter_is_none() {
        let p = RetryPolicy::new(BackoffPolicy::Patient);
        // None jitter → sleep_for returns the tier duration exactly.
        let mut rng = XorShift64::new(9);
        assert_eq!(p.sleep_for(0, None, &mut rng), Duration::from_secs(1));
        assert_eq!(p.sleep_for(5, None, &mut rng), Duration::from_secs(600));
        // Beyond schedule clamps to last tier.
        assert_eq!(p.sleep_for(99, None, &mut rng), Duration::from_secs(600));
    }

    #[test]
    fn retry_policy_with_jitter_changes_sleep_but_not_tiers() {
        let plain = RetryPolicy::new(BackoffPolicy::Patient);
        let jittered = plain.clone().with_jitter(JitterMode::Full);
        assert_eq!(jittered.to_durations(), plain.to_durations());
        assert_eq!(jittered.tier_count(), plain.tier_count());

        let mut rng = XorShift64::new(11);
        for _ in 0..20 {
            let r = jittered.sleep_for(0, None, &mut rng);
            assert!(r <= Duration::from_secs(1));
        }
    }

    #[test]
    fn retry_policy_empty_custom_schedule_is_zero() {
        let p = RetryPolicy::new(BackoffPolicy::Custom(vec![]));
        let mut rng = XorShift64::new(1);
        assert_eq!(p.sleep_for(0, None, &mut rng), Duration::ZERO);
    }

    // ---------- RetryCounters ----------

    #[test]
    fn retry_counters_count_all_three_observables() {
        let c = RetryCounters::new();
        assert_eq!((c.attempts(), c.exhausted(), c.success_after()), (0, 0, 0));
        c.inc_attempt();
        c.inc_attempt();
        c.inc_attempt();
        c.inc_exhausted();
        c.inc_success_after();
        assert_eq!((c.attempts(), c.exhausted(), c.success_after()), (3, 1, 1));
    }
}
