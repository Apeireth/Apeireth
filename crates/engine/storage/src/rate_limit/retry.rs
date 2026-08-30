//! Retry / backoff helpers recovered from `apeireth-rate-limiter`.
//!
//! - Full-jitter exponential backoff (`random(0, min(cap, base * 2^attempt))`)
//! - HTTP `Retry-After` delta-seconds parsing (HTTP-date is an honest `None`)
//! - Decision that prefers server Retry-After over client backoff

use std::time::Duration;

/// Continue after a delay, or stop with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryOutcome {
    /// Wait `Duration` then retry.
    Retry(Duration),
    /// Stop retrying.
    Stop(StopReason),
}

/// Why a retry loop stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Attempt count hit the configured ceiling.
    MaxAttemptsExceeded {
        /// Attempts already made (1-indexed in the error payload).
        attempts: u32,
        /// Configured maximum.
        max: u32,
    },
    /// Cumulative wait exceeded the configured ceiling.
    MaxWaitExceeded {
        /// Elapsed wait.
        elapsed: Duration,
        /// Configured maximum.
        max: Duration,
    },
    /// Permanent (non-retryable) error.
    PermanentError(String),
}

/// Backoff policy: delay for attempt N plus optional ceilings.
pub trait Backoff: Send + Sync {
    /// Delay after `attempt` (0-indexed) failures. Includes jitter.
    fn next_delay(&self, attempt: u32) -> Duration;
    /// Cumulative wait ceiling. `Duration::ZERO` means unlimited.
    fn max_total_wait(&self) -> Duration;
    /// Attempt ceiling including the first try. `0` means unlimited.
    fn max_attempts(&self) -> u32;
}

/// Constant delay. Useful in tests; not recommended in production.
#[derive(Debug, Clone)]
pub struct ConstantBackoff {
    delay: Duration,
    max_attempts: u32,
}

impl ConstantBackoff {
    /// `max_attempts = 0` means unlimited.
    pub fn new(delay: Duration, max_attempts: u32) -> Self {
        Self {
            delay,
            max_attempts,
        }
    }
}

impl Backoff for ConstantBackoff {
    fn next_delay(&self, attempt: u32) -> Duration {
        if self.max_attempts > 0 && attempt >= self.max_attempts {
            self.delay * 2
        } else {
            self.delay
        }
    }

    fn max_total_wait(&self) -> Duration {
        if self.max_attempts == 0 {
            Duration::ZERO
        } else {
            self.delay * self.max_attempts
        }
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// Exponential backoff with deterministic full jitter.
///
/// Formula: `delay = mix(attempt) % min(cap, base * 2^attempt)`.
/// The mixer is hash-based so the crate does not take a RNG dependency.
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    base: Duration,
    cap: Duration,
    max_attempts: u32,
    max_total_wait: Duration,
}

impl ExponentialBackoff {
    /// `max_attempts = 0` / `max_total_wait = ZERO` means unlimited.
    pub fn new(base: Duration, cap: Duration, max_attempts: u32, max_total_wait: Duration) -> Self {
        Self {
            base,
            cap,
            max_attempts,
            max_total_wait,
        }
    }
}

impl Backoff for ExponentialBackoff {
    fn next_delay(&self, attempt: u32) -> Duration {
        let exp_factor = 1u32.checked_shl(attempt.min(31)).unwrap_or(u32::MAX);
        let upper = self.base.saturating_mul(exp_factor).min(self.cap);
        let nanos = upper.as_nanos();
        let jitter_nanos = if nanos == 0 {
            0
        } else {
            let seed = (attempt as u64).wrapping_mul(0x9E3779B97F4A7C15);
            let mixed = (seed ^ (seed >> 33)).wrapping_mul(0xFF51AFD7ED558CCD);
            let mixed = (mixed ^ (mixed >> 33)).wrapping_mul(0xC4CEB9FE1A85EC53);
            (mixed ^ (mixed >> 33)) % (nanos as u64)
        };
        Duration::from_nanos(jitter_nanos)
    }

    fn max_total_wait(&self) -> Duration {
        self.max_total_wait
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// Parsed HTTP `Retry-After` (RFC 7231 §7.1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAfter {
    /// Delta-seconds.
    Seconds(u64),
    /// Absolute unix epoch seconds. HTTP-date parsing is not implemented
    /// (no extra datetime parser dependency); construct this variant directly.
    AbsoluteTime(u64),
}

impl RetryAfter {
    /// Parse a header value. Numeric delta-seconds succeed; HTTP-date returns
    /// `None` so the caller can fall back to exponential backoff.
    pub fn parse(header: &str) -> Option<Self> {
        let trimmed = header.trim();
        trimmed.parse::<u64>().ok().map(RetryAfter::Seconds)
    }

    /// Convert to a wait duration relative to `now_epoch_secs`.
    /// Expired absolute times yield `Duration::ZERO` (retry immediately).
    pub fn to_duration(&self, now_epoch_secs: u64) -> Duration {
        match self {
            RetryAfter::Seconds(s) => Duration::from_secs(*s),
            RetryAfter::AbsoluteTime(epoch) => {
                if *epoch > now_epoch_secs {
                    Duration::from_secs(epoch - now_epoch_secs)
                } else {
                    Duration::ZERO
                }
            }
        }
    }
}

/// Combine backoff ceilings with an optional server Retry-After.
///
/// Order: max_attempts, max_total_wait already elapsed, Retry-After override,
/// then backoff delay — and refuse a delay that would exceed max_total_wait.
pub fn decide(
    backoff: &dyn Backoff,
    attempt: u32,
    retry_after: Option<RetryAfter>,
    elapsed: Duration,
    now_epoch_secs: u64,
) -> RetryOutcome {
    let max = backoff.max_attempts();
    if max > 0 && attempt >= max {
        return RetryOutcome::Stop(StopReason::MaxAttemptsExceeded {
            attempts: attempt + 1,
            max,
        });
    }
    let total = backoff.max_total_wait();
    if total > Duration::ZERO && elapsed >= total {
        return RetryOutcome::Stop(StopReason::MaxWaitExceeded {
            elapsed,
            max: total,
        });
    }
    let delay = if let Some(ra) = retry_after {
        ra.to_duration(now_epoch_secs)
    } else {
        backoff.next_delay(attempt)
    };
    if total > Duration::ZERO && elapsed + delay > total {
        return RetryOutcome::Stop(StopReason::MaxWaitExceeded {
            elapsed: elapsed + delay,
            max: total,
        });
    }
    RetryOutcome::Retry(delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_backoff_zero_max_attempts_is_unlimited() {
        let b = ConstantBackoff::new(Duration::from_millis(100), 0);
        assert_eq!(b.max_attempts(), 0);
        assert_eq!(b.max_total_wait(), Duration::ZERO);
        assert_eq!(b.next_delay(0), Duration::from_millis(100));
        assert_eq!(b.next_delay(5), Duration::from_millis(100));
    }

    #[test]
    fn constant_backoff_signals_overflow() {
        let b = ConstantBackoff::new(Duration::from_millis(100), 3);
        assert_eq!(b.next_delay(2), Duration::from_millis(100));
        assert_eq!(b.next_delay(3), Duration::from_millis(200));
    }

    #[test]
    fn exponential_backoff_jitter_within_cap() {
        let b = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(30),
            0,
            Duration::ZERO,
        );
        for _ in 0..100 {
            assert!(b.next_delay(0) <= Duration::from_millis(100));
        }
        for _ in 0..100 {
            assert!(b.next_delay(5) <= Duration::from_millis(3_200));
        }
        for _ in 0..100 {
            assert!(b.next_delay(100) <= Duration::from_secs(30));
        }
    }

    #[test]
    fn exponential_backoff_attempt_zero_no_panic() {
        let b = ExponentialBackoff::new(
            Duration::from_millis(1),
            Duration::from_millis(1),
            0,
            Duration::ZERO,
        );
        let _ = b.next_delay(0);
    }

    #[test]
    fn exponential_backoff_attempt_overflow_no_panic() {
        let b = ExponentialBackoff::new(
            Duration::from_millis(1),
            Duration::from_millis(1),
            0,
            Duration::ZERO,
        );
        let _ = b.next_delay(u32::MAX);
    }

    #[test]
    fn retry_after_parse_delta_seconds() {
        assert_eq!(RetryAfter::parse("120"), Some(RetryAfter::Seconds(120)));
        assert_eq!(RetryAfter::parse("0"), Some(RetryAfter::Seconds(0)));
        assert_eq!(RetryAfter::parse("  42  "), Some(RetryAfter::Seconds(42)));
    }

    #[test]
    fn retry_after_parse_http_date_returns_none() {
        let http_date = "Wed, 21 Oct 2015 07:28:00 GMT";
        assert_eq!(RetryAfter::parse(http_date), None);
    }

    #[test]
    fn retry_after_parse_invalid_returns_none() {
        assert_eq!(RetryAfter::parse("not a number"), None);
        assert_eq!(RetryAfter::parse(""), None);
        assert_eq!(RetryAfter::parse("12.5"), None);
    }

    #[test]
    fn retry_after_to_duration_seconds() {
        let ra = RetryAfter::Seconds(120);
        assert_eq!(ra.to_duration(0), Duration::from_secs(120));
    }

    #[test]
    fn retry_after_to_duration_future_absolute() {
        let now = 1_000_000;
        let ra = RetryAfter::AbsoluteTime(now + 60);
        assert_eq!(ra.to_duration(now), Duration::from_secs(60));
    }

    #[test]
    fn retry_after_to_duration_expired_returns_zero() {
        let now = 1_000_000;
        let ra = RetryAfter::AbsoluteTime(now - 60);
        assert_eq!(ra.to_duration(now), Duration::ZERO);
    }

    #[test]
    fn decide_normal_retry() {
        let b = ConstantBackoff::new(Duration::from_millis(100), 0);
        let outcome = decide(&b, 0, None, Duration::ZERO, 0);
        assert_eq!(outcome, RetryOutcome::Retry(Duration::from_millis(100)));
    }

    #[test]
    fn decide_max_attempts_exceeded() {
        let b = ConstantBackoff::new(Duration::from_millis(100), 2);
        let outcome = decide(&b, 2, None, Duration::ZERO, 0);
        assert!(matches!(
            outcome,
            RetryOutcome::Stop(StopReason::MaxAttemptsExceeded { .. })
        ));
    }

    #[test]
    fn decide_max_total_wait_exceeded() {
        let b = ConstantBackoff::new(Duration::from_millis(100), 2);
        let outcome = decide(&b, 0, None, Duration::from_millis(200), 0);
        assert!(matches!(
            outcome,
            RetryOutcome::Stop(StopReason::MaxWaitExceeded { .. })
        ));
    }

    #[test]
    fn decide_retry_after_overrides_backoff() {
        let b = ConstantBackoff::new(Duration::from_millis(100), 0);
        let ra = RetryAfter::Seconds(5);
        let outcome = decide(&b, 0, Some(ra), Duration::ZERO, 0);
        assert_eq!(outcome, RetryOutcome::Retry(Duration::from_secs(5)));
    }

    #[test]
    fn decide_retry_after_overflows_total_wait() {
        let b = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(10),
            0,
            Duration::from_secs(1),
        );
        let ra = RetryAfter::Seconds(2);
        let outcome = decide(&b, 0, Some(ra), Duration::from_millis(500), 0);
        assert!(matches!(
            outcome,
            RetryOutcome::Stop(StopReason::MaxWaitExceeded { .. })
        ));
    }
}
