//! Reusable in-process rate-limiter algorithms.
//!
//! Canonical implementation module. This is a library
//! primitive for token / leaky / fixed / sliding windows plus Retry-After
//! backoff. It is **not** a second governance hook: capability-level trust
//! tiers remain in `apeireth-governance::rate_limit`.
//!
//! Storage backends other than in-process maps (Redis / Memcached / file /
//! distributed) were honest stubs in the canonical and are not ported.

mod retry;
mod strategies;

pub use retry::{
    decide, Backoff, ConstantBackoff, ExponentialBackoff, RetryAfter, RetryOutcome, StopReason,
};
pub use strategies::{
    BucketConfig, FixedWindow, FixedWindowReset, LeakyBucket, LeakyBucketOverflow, SlidingWindow,
    SlidingWindowPrecision, TokenBucket,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Errors produced by the reusable limiter.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RateLimitError {
    /// Generic parameter error.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
    /// `rate_per_second` must be > 0.
    #[error("rate must be > 0")]
    ZeroRate,
    /// Burst / capacity must be > 0.
    #[error("burst must be > 0")]
    ZeroBurst,
    /// Window size must be > 0.
    #[error("window size must be > 0")]
    ZeroWindowSize,
    /// Blocking acquire exceeded `max_wait`.
    #[error("max wait exceeded waiting for {key}")]
    MaxWaitExceeded {
        /// Key that timed out.
        key: String,
    },
}

/// Result alias for limiter operations.
pub type RateLimitResult<T> = Result<T, RateLimitError>;

/// Algorithm selected for a [`KeyedLimiter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrategyKind {
    /// Token bucket (rate + burst, allows bursts).
    TokenBucket,
    /// Leaky bucket (smooth output).
    LeakyBucket,
    /// Fixed window (known boundary spike).
    FixedWindow,
    /// Sliding window (log or counter precision).
    SlidingWindow,
}

/// Strategy-specific parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyConfig {
    /// Algorithm kind.
    pub kind: StrategyKind,
    /// Window size (fixed / sliding).
    pub window_size: Option<Duration>,
    /// Sliding-window sub-bucket interval.
    pub slide_interval: Option<Duration>,
    /// Max requests per window (fixed / sliding).
    pub max_requests: Option<u32>,
    /// Sliding-window precision.
    pub precision: Option<SlidingWindowPrecision>,
    /// Leaky-bucket overflow policy.
    pub overflow_policy: Option<LeakyBucketOverflow>,
    /// Fixed-window reset policy.
    pub reset_strategy: Option<FixedWindowReset>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            kind: StrategyKind::TokenBucket,
            window_size: None,
            slide_interval: None,
            max_requests: None,
            precision: None,
            overflow_policy: None,
            reset_strategy: None,
        }
    }
}

/// Top-level limiter configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimiterConfig {
    /// Bucket / refill parameters.
    pub bucket: BucketConfig,
    /// Algorithm selection.
    pub strategy: StrategyConfig,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            bucket: BucketConfig::default(),
            strategy: StrategyConfig::default(),
        }
    }
}

impl RateLimiterConfig {
    fn validate(&self) -> RateLimitResult<()> {
        self.bucket.validate()?;
        if let Some(ws) = self.strategy.window_size {
            if ws.is_zero() {
                return Err(RateLimitError::ZeroWindowSize);
            }
        }
        if let Some(si) = self.strategy.slide_interval {
            if si.is_zero() {
                return Err(RateLimitError::InvalidParameter(
                    "slide_interval must be > 0".to_string(),
                ));
            }
        }
        if let Some(mr) = self.strategy.max_requests {
            if mr == 0 {
                return Err(RateLimitError::InvalidParameter(
                    "max_requests must be > 0".to_string(),
                ));
            }
        }
        Ok(())
    }
}

enum PerKeyState {
    Token(TokenBucket),
    Leaky(LeakyBucket),
    Fixed(FixedWindow),
    Sliding(SlidingWindow),
}

impl PerKeyState {
    fn try_acquire(&mut self, cost: u32, now: Instant) -> bool {
        match self {
            PerKeyState::Token(b) => b.try_acquire_at(cost, now),
            PerKeyState::Leaky(b) => b.try_acquire_at(cost, now),
            PerKeyState::Fixed(w) => w.try_acquire_at(now),
            PerKeyState::Sliding(w) => w.try_acquire_at(now),
        }
    }

    fn release(&mut self, cost: u32) {
        match self {
            PerKeyState::Token(b) => b.release(cost),
            PerKeyState::Leaky(b) => b.release(cost),
            PerKeyState::Fixed(_) | PerKeyState::Sliding(_) => {}
        }
    }

    fn reset(&mut self, now: Instant) {
        match self {
            PerKeyState::Token(b) => b.reset_at(now),
            PerKeyState::Leaky(b) => b.reset_at(now),
            PerKeyState::Fixed(w) => w.reset_at(now),
            PerKeyState::Sliding(w) => w.reset(),
        }
    }
}

struct InnerState {
    state: Mutex<HashMap<String, PerKeyState>>,
    hits: AtomicU64,
    misses: AtomicU64,
    total_attempts: AtomicU64,
}

impl InnerState {
    fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            total_attempts: AtomicU64::new(0),
        }
    }
}

/// Snapshot of limiter counters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RateLimiterStats {
    /// `try_acquire` call count.
    pub total_attempts: u64,
    /// Successful acquisitions.
    pub hits: u64,
    /// Rejected acquisitions.
    pub misses: u64,
    /// Currently tracked keys.
    pub tracked_keys: usize,
}

impl RateLimiterStats {
    /// Hits / attempts, or 0.0 when unused.
    pub fn hit_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_attempts as f64
        }
    }
}

/// RAII permit. Dropping it returns `cost` to token / leaky buckets.
pub struct AcquiredPermit {
    inner: Option<Arc<InnerState>>,
    key: String,
    cost: u32,
    acquired_at: Instant,
}

impl AcquiredPermit {
    fn new(inner: Arc<InnerState>, key: String, cost: u32, acquired_at: Instant) -> Self {
        Self {
            inner: Some(inner),
            key,
            cost,
            acquired_at,
        }
    }

    /// Cost deducted when the permit was issued.
    pub fn cost(&self) -> u32 {
        self.cost
    }

    /// Key the permit is bound to.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// How long the permit has been held.
    pub fn held_for(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.acquired_at)
    }

    /// Drop without returning tokens.
    pub fn forget(mut self) {
        self.inner.take();
    }
}

impl Drop for AcquiredPermit {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            if let Ok(mut map) = inner.state.lock() {
                if let Some(state) = map.get_mut(&self.key) {
                    state.release(self.cost);
                }
            }
        }
    }
}

impl std::fmt::Debug for AcquiredPermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcquiredPermit")
            .field("key", &self.key)
            .field("cost", &self.cost)
            .finish()
    }
}

/// Per-key in-process limiter.
pub struct KeyedLimiter {
    config: RateLimiterConfig,
    inner: Arc<InnerState>,
}

impl KeyedLimiter {
    /// Construct from a validated config.
    pub fn new(config: RateLimiterConfig) -> RateLimitResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            inner: Arc::new(InnerState::new()),
        })
    }

    /// Current strategy.
    pub fn strategy_kind(&self) -> StrategyKind {
        self.config.strategy.kind
    }

    /// Non-blocking try using wall `Instant::now()`.
    pub fn try_acquire(&self, key: &str, cost: u32) -> RateLimitResult<bool> {
        self.try_acquire_at(key, cost, Instant::now())
    }

    /// Non-blocking try at an explicit clock instant (tests / virtual clocks).
    pub fn try_acquire_at(&self, key: &str, cost: u32, now: Instant) -> RateLimitResult<bool> {
        self.inner.total_attempts.fetch_add(1, Ordering::Relaxed);
        let result = self.with_state_or_create(key, now, |s| s.try_acquire(cost, now))?;
        if result {
            self.inner.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.inner.misses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(result)
    }

    /// Blocking acquire using `std::thread::sleep` until `max_wait`.
    ///
    /// Prefer [`Self::try_acquire`] on async paths; this helper is for
    /// in-process callers that accept a short sleep.
    pub fn acquire(&self, key: &str, cost: u32) -> RateLimitResult<AcquiredPermit> {
        self.acquire_at(key, cost, Instant::now())
    }

    /// Blocking acquire starting at `now`.
    pub fn acquire_at(
        &self,
        key: &str,
        cost: u32,
        now: Instant,
    ) -> RateLimitResult<AcquiredPermit> {
        self.inner.total_attempts.fetch_add(1, Ordering::Relaxed);
        let max_wait = self
            .config
            .bucket
            .max_wait
            .unwrap_or(Duration::from_secs(5));
        let start = Instant::now();
        let refill_step = self
            .config
            .bucket
            .refill_interval
            .max(Duration::from_millis(1));
        let step = refill_step.min(max_wait.div_f64(4.0).max(Duration::from_millis(1)));

        loop {
            let elapsed = start.elapsed();
            let clock = now + elapsed;
            let acquired = self.with_state_or_create(key, clock, |s| s.try_acquire(cost, clock))?;
            if acquired {
                self.inner.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(AcquiredPermit::new(
                    Arc::clone(&self.inner),
                    key.to_string(),
                    cost,
                    clock,
                ));
            }
            if elapsed >= max_wait {
                self.inner.misses.fetch_add(1, Ordering::Relaxed);
                return Err(RateLimitError::MaxWaitExceeded {
                    key: key.to_string(),
                });
            }
            let should_block = match self.config.strategy.kind {
                StrategyKind::LeakyBucket => {
                    self.config
                        .strategy
                        .overflow_policy
                        .unwrap_or(LeakyBucketOverflow::Drop)
                        == LeakyBucketOverflow::Block
                }
                _ => true,
            };
            if !should_block {
                self.inner.misses.fetch_add(1, Ordering::Relaxed);
                return Err(RateLimitError::MaxWaitExceeded {
                    key: key.to_string(),
                });
            }
            let sleep_for = step.min(max_wait.saturating_sub(elapsed));
            if sleep_for.is_zero() {
                self.inner.misses.fetch_add(1, Ordering::Relaxed);
                return Err(RateLimitError::MaxWaitExceeded {
                    key: key.to_string(),
                });
            }
            std::thread::sleep(sleep_for);
        }
    }

    /// Drop per-key state.
    pub fn reset(&self, key: &str) {
        if let Ok(mut map) = self.inner.state.lock() {
            map.remove(key);
        }
    }

    /// Snapshot counters.
    pub fn stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            total_attempts: self.inner.total_attempts.load(Ordering::Relaxed),
            hits: self.inner.hits.load(Ordering::Relaxed),
            misses: self.inner.misses.load(Ordering::Relaxed),
            tracked_keys: self.inner.state.lock().map(|m| m.len()).unwrap_or(0),
        }
    }

    fn with_state_or_create<R>(
        &self,
        key: &str,
        now: Instant,
        f: impl FnOnce(&mut PerKeyState) -> R,
    ) -> RateLimitResult<R> {
        let mut map = self
            .inner
            .state
            .lock()
            .map_err(|_| RateLimitError::InvalidParameter("lock poisoned".to_string()))?;
        if !map.contains_key(key) {
            map.insert(key.to_string(), self.create_state(now)?);
        }
        let state = map.get_mut(key).expect("just inserted or already present");
        Ok(f(state))
    }

    fn create_state(&self, now: Instant) -> RateLimitResult<PerKeyState> {
        let bucket = &self.config.bucket;
        let strategy = &self.config.strategy;
        match strategy.kind {
            StrategyKind::TokenBucket => Ok(PerKeyState::Token(TokenBucket::new_at(bucket, now)?)),
            StrategyKind::LeakyBucket => {
                let overflow = strategy
                    .overflow_policy
                    .unwrap_or(LeakyBucketOverflow::Drop);
                Ok(PerKeyState::Leaky(LeakyBucket::new_at(
                    bucket, overflow, now,
                )?))
            }
            StrategyKind::FixedWindow => {
                let ws = strategy.window_size.ok_or_else(|| {
                    RateLimitError::InvalidParameter("FixedWindow needs window_size".to_string())
                })?;
                let mr = strategy.max_requests.ok_or_else(|| {
                    RateLimitError::InvalidParameter("FixedWindow needs max_requests".to_string())
                })?;
                let reset = strategy
                    .reset_strategy
                    .unwrap_or(FixedWindowReset::OnWindowEnd);
                Ok(PerKeyState::Fixed(FixedWindow::new_at(ws, mr, reset, now)?))
            }
            StrategyKind::SlidingWindow => {
                let ws = strategy.window_size.ok_or_else(|| {
                    RateLimitError::InvalidParameter("SlidingWindow needs window_size".to_string())
                })?;
                let mr = strategy.max_requests.ok_or_else(|| {
                    RateLimitError::InvalidParameter("SlidingWindow needs max_requests".to_string())
                })?;
                let si = strategy.slide_interval.unwrap_or(Duration::ZERO);
                let precision = strategy.precision.unwrap_or(SlidingWindowPrecision::Log);
                Ok(PerKeyState::Sliding(SlidingWindow::new_at(
                    ws, si, mr, precision, now,
                )?))
            }
        }
    }
}

/// Convenience: token bucket + in-memory keys.
pub fn token_bucket_in_memory(
    rate_per_second: f64,
    burst: u32,
    max_wait: Option<Duration>,
) -> RateLimitResult<KeyedLimiter> {
    KeyedLimiter::new(RateLimiterConfig {
        bucket: BucketConfig {
            rate_per_second,
            burst,
            initial_tokens: None,
            max_wait,
            refill_interval: Duration::from_millis(100),
        },
        strategy: StrategyConfig {
            kind: StrategyKind::TokenBucket,
            ..Default::default()
        },
    })
}

/// Convenience: leaky bucket + in-memory keys.
pub fn leaky_bucket_in_memory(
    rate_per_second: f64,
    capacity: u32,
    overflow: LeakyBucketOverflow,
) -> RateLimitResult<KeyedLimiter> {
    KeyedLimiter::new(RateLimiterConfig {
        bucket: BucketConfig {
            rate_per_second,
            burst: capacity,
            initial_tokens: Some(0),
            max_wait: Some(Duration::from_secs(5)),
            refill_interval: Duration::from_millis(100),
        },
        strategy: StrategyConfig {
            kind: StrategyKind::LeakyBucket,
            overflow_policy: Some(overflow),
            ..Default::default()
        },
    })
}

/// Convenience: fixed window + in-memory keys.
pub fn fixed_window_in_memory(
    window_size: Duration,
    max_requests: u32,
) -> RateLimitResult<KeyedLimiter> {
    KeyedLimiter::new(RateLimiterConfig {
        bucket: BucketConfig {
            rate_per_second: 1.0,
            burst: 1,
            initial_tokens: Some(0),
            max_wait: None,
            refill_interval: Duration::from_millis(100),
        },
        strategy: StrategyConfig {
            kind: StrategyKind::FixedWindow,
            window_size: Some(window_size),
            max_requests: Some(max_requests),
            reset_strategy: Some(FixedWindowReset::OnWindowEnd),
            ..Default::default()
        },
    })
}

/// Convenience: sliding window + in-memory keys.
pub fn sliding_window_in_memory(
    window_size: Duration,
    max_requests: u32,
    precision: SlidingWindowPrecision,
) -> RateLimitResult<KeyedLimiter> {
    KeyedLimiter::new(RateLimiterConfig {
        bucket: BucketConfig {
            rate_per_second: 1.0,
            burst: 1,
            initial_tokens: Some(0),
            max_wait: None,
            refill_interval: Duration::from_millis(100),
        },
        strategy: StrategyConfig {
            kind: StrategyKind::SlidingWindow,
            window_size: Some(window_size),
            slide_interval: Some(Duration::from_millis(50)),
            max_requests: Some(max_requests),
            precision: Some(precision),
            ..Default::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_constructor_basic() {
        let l = token_bucket_in_memory(10.0, 20, None).unwrap();
        assert!(l.try_acquire("k", 1).unwrap());
    }

    #[test]
    fn leaky_bucket_constructor_basic() {
        let l = leaky_bucket_in_memory(10.0, 5, LeakyBucketOverflow::Drop).unwrap();
        for _ in 0..5 {
            assert!(l.try_acquire("k", 1).unwrap());
        }
        assert!(!l.try_acquire("k", 1).unwrap());
    }

    #[test]
    fn fixed_window_constructor_basic() {
        let l = fixed_window_in_memory(Duration::from_secs(10), 3).unwrap();
        assert!(l.try_acquire("k", 1).unwrap());
        assert!(l.try_acquire("k", 1).unwrap());
        assert!(l.try_acquire("k", 1).unwrap());
        assert!(!l.try_acquire("k", 1).unwrap());
    }

    #[test]
    fn sliding_window_log_basic() {
        let l = sliding_window_in_memory(Duration::from_secs(10), 3, SlidingWindowPrecision::Log)
            .unwrap();
        for _ in 0..3 {
            assert!(l.try_acquire("k", 1).unwrap());
        }
        assert!(!l.try_acquire("k", 1).unwrap());
    }

    #[test]
    fn keyed_limiter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<KeyedLimiter>();
    }
}
