//! Four classical limiter algorithms: token, leaky, fixed window, sliding window.
//!
//! All clocks are explicit `Instant` values so tests can advance time without
//! sleeping. `Instant::now()` wrappers live on the types for production callers.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{RateLimitError, RateLimitResult};

/// Bucket / refill parameters shared by token and leaky buckets.
#[derive(Debug, Clone, PartialEq)]
pub struct BucketConfig {
    /// Tokens (or drip units) added per second. Must be > 0.
    pub rate_per_second: f64,
    /// Capacity / burst. Must be > 0.
    pub burst: u32,
    /// Starting tokens (token bucket). Defaults to `burst`.
    pub initial_tokens: Option<u32>,
    /// Longest wait for a blocking acquire.
    pub max_wait: Option<Duration>,
    /// Sleep / poll interval used by blocking acquire.
    pub refill_interval: Duration,
}

impl Default for BucketConfig {
    fn default() -> Self {
        Self {
            rate_per_second: 10.0,
            burst: 20,
            initial_tokens: None,
            max_wait: Some(Duration::from_secs(5)),
            refill_interval: Duration::from_millis(100),
        }
    }
}

impl BucketConfig {
    /// Reject zero rate / burst / refill interval.
    pub fn validate(&self) -> RateLimitResult<()> {
        if self.rate_per_second <= 0.0 {
            return Err(RateLimitError::ZeroRate);
        }
        if self.burst == 0 {
            return Err(RateLimitError::ZeroBurst);
        }
        if self.refill_interval.is_zero() {
            return Err(RateLimitError::InvalidParameter(
                "refill_interval must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Token bucket with lazy refill.
#[derive(Debug)]
pub struct TokenBucket {
    capacity: f64,
    refill_rate: f64,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    /// Construct using `Instant::now()`.
    pub fn new(cfg: &BucketConfig) -> RateLimitResult<Self> {
        Self::new_at(cfg, Instant::now())
    }

    /// Construct at an explicit clock.
    pub fn new_at(cfg: &BucketConfig, now: Instant) -> RateLimitResult<Self> {
        cfg.validate()?;
        let burst = f64::from(cfg.burst);
        let initial = f64::from(cfg.initial_tokens.unwrap_or(cfg.burst));
        Ok(Self {
            capacity: burst,
            refill_rate: cfg.rate_per_second,
            tokens: initial.min(burst),
            last_refill: now,
        })
    }

    /// Non-blocking try at wall clock.
    pub fn try_acquire(&mut self, cost: u32) -> bool {
        self.try_acquire_at(cost, Instant::now())
    }

    /// Non-blocking try at `now`.
    pub fn try_acquire_at(&mut self, cost: u32, now: Instant) -> bool {
        if cost == 0 {
            return true;
        }
        self.refill(now);
        let cost_f = f64::from(cost);
        if self.tokens >= cost_f {
            self.tokens -= cost_f;
            true
        } else {
            false
        }
    }

    /// Return `cost` tokens, capped at capacity.
    pub fn release(&mut self, cost: u32) {
        if cost == 0 {
            return;
        }
        self.tokens = (self.tokens + f64::from(cost)).min(self.capacity);
    }

    /// Restore a full bucket.
    pub fn reset(&mut self) {
        self.reset_at(Instant::now());
    }

    /// Restore a full bucket at `now`.
    pub fn reset_at(&mut self, now: Instant) {
        self.tokens = self.capacity;
        self.last_refill = now;
    }

    /// Current tokens after a lazy refill.
    pub fn available_tokens(&mut self, now: Instant) -> f64 {
        self.refill(now);
        self.tokens
    }

    /// Burst capacity.
    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    /// Refill rate in tokens / second.
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }
        let added = elapsed * self.refill_rate;
        self.tokens = (self.tokens + added).min(self.capacity);
        self.last_refill = now;
    }
}

/// Overflow policy for a leaky bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeakyBucketOverflow {
    /// Reject when full.
    Drop,
    /// Caller may block until there is room.
    Block,
}

/// Leaky bucket with lazy drip.
#[derive(Debug)]
pub struct LeakyBucket {
    drip_rate: f64,
    capacity: f64,
    level: f64,
    last_drip: Instant,
    overflow_policy: LeakyBucketOverflow,
}

impl LeakyBucket {
    /// Construct using `Instant::now()`.
    pub fn new(cfg: &BucketConfig, overflow: LeakyBucketOverflow) -> RateLimitResult<Self> {
        Self::new_at(cfg, overflow, Instant::now())
    }

    /// Construct at an explicit clock.
    pub fn new_at(
        cfg: &BucketConfig,
        overflow: LeakyBucketOverflow,
        now: Instant,
    ) -> RateLimitResult<Self> {
        cfg.validate()?;
        Ok(Self {
            drip_rate: cfg.rate_per_second,
            capacity: f64::from(cfg.burst),
            level: 0.0,
            last_drip: now,
            overflow_policy: overflow,
        })
    }

    /// Overflow policy.
    pub fn overflow_policy(&self) -> LeakyBucketOverflow {
        self.overflow_policy
    }

    /// Non-blocking try at wall clock.
    pub fn try_acquire(&mut self, cost: u32) -> bool {
        self.try_acquire_at(cost, Instant::now())
    }

    /// Non-blocking try at `now`.
    pub fn try_acquire_at(&mut self, cost: u32, now: Instant) -> bool {
        if cost == 0 {
            return true;
        }
        self.drip(now);
        let cost_f = f64::from(cost);
        if self.level + cost_f <= self.capacity {
            self.level += cost_f;
            true
        } else {
            false
        }
    }

    /// Drain `cost` units (permit drop).
    pub fn release(&mut self, cost: u32) {
        if cost == 0 {
            return;
        }
        self.level = (self.level - f64::from(cost)).max(0.0);
    }

    /// Empty the bucket.
    pub fn reset(&mut self) {
        self.reset_at(Instant::now());
    }

    /// Empty the bucket at `now`.
    pub fn reset_at(&mut self, now: Instant) {
        self.level = 0.0;
        self.last_drip = now;
    }

    /// Current water level after a lazy drip.
    pub fn current_level(&mut self, now: Instant) -> f64 {
        self.drip(now);
        self.level
    }

    /// Capacity.
    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    /// Drip rate in units / second.
    pub fn drip_rate(&self) -> f64 {
        self.drip_rate
    }

    fn drip(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_drip).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }
        let leaked = elapsed * self.drip_rate;
        self.level = (self.level - leaked).max(0.0);
        self.last_drip = now;
    }
}

/// When a fixed window resets its counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixedWindowReset {
    /// Rotate when the window elapses.
    OnWindowEnd,
    /// Only rotate on explicit `reset`.
    OnDemand,
    /// Either automatic or explicit.
    Both,
}

/// Fixed window counter. Known defect: boundary spike of up to 2N.
#[derive(Debug)]
pub struct FixedWindow {
    window_size: Duration,
    max_requests: u32,
    window_start: Instant,
    counter: u32,
    reset_strategy: FixedWindowReset,
}

impl FixedWindow {
    /// Construct using `Instant::now()`.
    pub fn new(
        window_size: Duration,
        max_requests: u32,
        reset: FixedWindowReset,
    ) -> RateLimitResult<Self> {
        Self::new_at(window_size, max_requests, reset, Instant::now())
    }

    /// Construct at an explicit clock.
    pub fn new_at(
        window_size: Duration,
        max_requests: u32,
        reset: FixedWindowReset,
        now: Instant,
    ) -> RateLimitResult<Self> {
        if window_size.is_zero() {
            return Err(RateLimitError::ZeroWindowSize);
        }
        if max_requests == 0 {
            return Err(RateLimitError::InvalidParameter(
                "max_requests must be > 0".to_string(),
            ));
        }
        Ok(Self {
            window_size,
            max_requests,
            window_start: now,
            counter: 0,
            reset_strategy: reset,
        })
    }

    /// Non-blocking try at wall clock.
    pub fn try_acquire(&mut self) -> bool {
        self.try_acquire_at(Instant::now())
    }

    /// Non-blocking try at `now`.
    pub fn try_acquire_at(&mut self, now: Instant) -> bool {
        self.maybe_rotate(now);
        if self.counter < self.max_requests {
            self.counter += 1;
            true
        } else {
            false
        }
    }

    /// Manual reset.
    pub fn reset(&mut self) {
        self.reset_at(Instant::now());
    }

    /// Manual reset at `now`.
    pub fn reset_at(&mut self, now: Instant) {
        self.counter = 0;
        self.window_start = now;
    }

    /// Current count (does not rotate).
    pub fn current_count(&self) -> u32 {
        self.counter
    }

    /// Remaining quota after a possible rotate.
    pub fn remaining(&mut self, now: Instant) -> u32 {
        self.maybe_rotate(now);
        self.max_requests.saturating_sub(self.counter)
    }

    /// Window size.
    pub fn window_size(&self) -> Duration {
        self.window_size
    }

    fn maybe_rotate(&mut self, now: Instant) {
        if self.reset_strategy == FixedWindowReset::OnDemand {
            return;
        }
        if now.saturating_duration_since(self.window_start) >= self.window_size {
            self.counter = 0;
            self.window_start = now;
        }
    }
}

/// Sliding-window precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlidingWindowPrecision {
    /// Exact timestamps (`O(requests)` memory).
    Log,
    /// Sub-bucket counters (`O(window/slide)` memory).
    Counter,
}

#[derive(Debug, Clone, Copy)]
struct SubBucket {
    start: Instant,
    count: u32,
}

/// Sliding window: log or counter mode. Avoids the fixed-window boundary spike.
#[derive(Debug)]
pub struct SlidingWindow {
    window_size: Duration,
    slide_interval: Duration,
    max_requests: u32,
    precision: SlidingWindowPrecision,
    epoch: Instant,
    log: VecDeque<Instant>,
    counter: VecDeque<SubBucket>,
}

impl SlidingWindow {
    /// Construct using `Instant::now()`.
    pub fn new(
        window_size: Duration,
        slide_interval: Duration,
        max_requests: u32,
        precision: SlidingWindowPrecision,
    ) -> RateLimitResult<Self> {
        Self::new_at(
            window_size,
            slide_interval,
            max_requests,
            precision,
            Instant::now(),
        )
    }

    /// Construct at an explicit clock.
    pub fn new_at(
        window_size: Duration,
        slide_interval: Duration,
        max_requests: u32,
        precision: SlidingWindowPrecision,
        now: Instant,
    ) -> RateLimitResult<Self> {
        if window_size.is_zero() {
            return Err(RateLimitError::ZeroWindowSize);
        }
        if max_requests == 0 {
            return Err(RateLimitError::InvalidParameter(
                "max_requests must be > 0".to_string(),
            ));
        }
        let slide = if slide_interval.is_zero() {
            window_size / 10
        } else {
            slide_interval
        };
        Ok(Self {
            window_size,
            slide_interval: slide,
            max_requests,
            precision,
            epoch: now,
            log: VecDeque::new(),
            counter: VecDeque::new(),
        })
    }

    /// Non-blocking try at wall clock.
    pub fn try_acquire(&mut self) -> bool {
        self.try_acquire_at(Instant::now())
    }

    /// Non-blocking try at `now`.
    pub fn try_acquire_at(&mut self, now: Instant) -> bool {
        match self.precision {
            SlidingWindowPrecision::Log => self.try_acquire_log(now),
            SlidingWindowPrecision::Counter => self.try_acquire_counter(now),
        }
    }

    /// Clear all counts.
    pub fn reset(&mut self) {
        self.log.clear();
        self.counter.clear();
    }

    /// Current in-window count.
    pub fn current_count(&mut self, now: Instant) -> u32 {
        match self.precision {
            SlidingWindowPrecision::Log => {
                self.prune_log(now);
                self.log.len() as u32
            }
            SlidingWindowPrecision::Counter => {
                self.prune_counter(now);
                self.counter.iter().map(|b| b.count).sum()
            }
        }
    }

    /// Remaining quota.
    pub fn remaining(&mut self, now: Instant) -> u32 {
        self.max_requests.saturating_sub(self.current_count(now))
    }

    /// Window size.
    pub fn window_size(&self) -> Duration {
        self.window_size
    }

    /// Sub-bucket interval.
    pub fn slide_interval(&self) -> Duration {
        self.slide_interval
    }

    fn try_acquire_log(&mut self, now: Instant) -> bool {
        self.prune_log(now);
        if (self.log.len() as u32) < self.max_requests {
            self.log.push_back(now);
            true
        } else {
            false
        }
    }

    fn prune_log(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window_size).unwrap_or(now);
        while let Some(&front) = self.log.front() {
            if front < cutoff {
                self.log.pop_front();
            } else {
                break;
            }
        }
    }

    fn try_acquire_counter(&mut self, now: Instant) -> bool {
        self.prune_counter(now);
        let total: u32 = self.counter.iter().map(|b| b.count).sum();
        if total < self.max_requests {
            let bucket_start = self.bucket_start(now);
            if let Some(last) = self.counter.back_mut() {
                if last.start == bucket_start {
                    last.count += 1;
                    return true;
                }
            }
            self.counter.push_back(SubBucket {
                start: bucket_start,
                count: 1,
            });
            true
        } else {
            false
        }
    }

    fn prune_counter(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window_size).unwrap_or(now);
        while let Some(&front) = self.counter.front() {
            if front.start < cutoff {
                self.counter.pop_front();
            } else {
                break;
            }
        }
    }

    fn bucket_start(&self, now: Instant) -> Instant {
        let slide_nanos = self.slide_interval.as_nanos().max(1) as u64;
        let since_epoch = now.saturating_duration_since(self.epoch).as_nanos() as u64;
        let bucket_index = since_epoch / slide_nanos;
        self.epoch + Duration::from_nanos(bucket_index * slide_nanos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(rate: f64, burst: u32) -> BucketConfig {
        BucketConfig {
            rate_per_second: rate,
            burst,
            initial_tokens: Some(burst),
            max_wait: Some(Duration::from_millis(500)),
            refill_interval: Duration::from_millis(10),
        }
    }

    #[test]
    fn token_new_rejects_zero_rate() {
        assert!(matches!(
            TokenBucket::new(&cfg(0.0, 10)),
            Err(RateLimitError::ZeroRate)
        ));
    }

    #[test]
    fn token_new_rejects_zero_burst() {
        assert!(matches!(
            TokenBucket::new(&cfg(10.0, 0)),
            Err(RateLimitError::ZeroBurst)
        ));
    }

    #[test]
    fn token_burst_drains_in_one_shot() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(&cfg(10.0, 5), now).unwrap();
        assert!(tb.try_acquire_at(5, now));
        assert!(!tb.try_acquire_at(1, now));
    }

    #[test]
    fn token_refill_over_time_restores_tokens() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(&cfg(100.0, 10), now).unwrap();
        assert!(tb.try_acquire_at(10, now));
        assert!(!tb.try_acquire_at(1, now));
        let later = now + Duration::from_millis(50);
        assert!(tb.try_acquire_at(1, later));
    }

    #[test]
    fn token_release_returns_tokens() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(
            &BucketConfig {
                rate_per_second: 0.01,
                burst: 10,
                initial_tokens: Some(10),
                max_wait: Some(Duration::from_millis(100)),
                refill_interval: Duration::from_millis(10),
            },
            now,
        )
        .unwrap();
        let _ = tb.try_acquire_at(5, now);
        tb.release(5);
        assert!(tb.try_acquire_at(5, now));
    }

    #[test]
    fn token_release_caps_at_capacity() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(&cfg(0.01, 10), now).unwrap();
        tb.release(1000);
        assert!(tb.try_acquire_at(10, now));
        assert!(!tb.try_acquire_at(1, now));
    }

    #[test]
    fn token_zero_cost_is_free() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(&cfg(0.01, 1), now).unwrap();
        for _ in 0..1000 {
            assert!(tb.try_acquire_at(0, now));
        }
    }

    #[test]
    fn token_reset_restores_to_full() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(&cfg(0.01, 10), now).unwrap();
        let _ = tb.try_acquire_at(10, now);
        assert!(!tb.try_acquire_at(1, now));
        tb.reset_at(now);
        assert!(tb.try_acquire_at(1, now));
    }

    #[test]
    fn token_available_tokens_reflects_refill() {
        let now = Instant::now();
        let mut tb = TokenBucket::new_at(&cfg(1000.0, 100), now).unwrap();
        let _ = tb.try_acquire_at(50, now);
        let later = now + Duration::from_millis(50);
        let avail = tb.available_tokens(later);
        assert!(avail > 50.0, "refill should apply, got {avail}");
        assert!(avail <= 100.0, "must not exceed capacity, got {avail}");
    }

    fn leaky_cfg(rate: f64, burst: u32) -> BucketConfig {
        BucketConfig {
            rate_per_second: rate,
            burst,
            initial_tokens: Some(0),
            max_wait: Some(Duration::from_millis(500)),
            refill_interval: Duration::from_millis(10),
        }
    }

    #[test]
    fn leaky_drops_when_overflow_drop() {
        let now = Instant::now();
        let mut lb =
            LeakyBucket::new_at(&leaky_cfg(0.01, 5), LeakyBucketOverflow::Drop, now).unwrap();
        for _ in 0..5 {
            assert!(lb.try_acquire_at(1, now));
        }
        assert!(!lb.try_acquire_at(1, now));
    }

    #[test]
    fn leaky_drips_over_time_makes_room() {
        let now = Instant::now();
        let mut lb =
            LeakyBucket::new_at(&leaky_cfg(100.0, 5), LeakyBucketOverflow::Drop, now).unwrap();
        for _ in 0..5 {
            assert!(lb.try_acquire_at(1, now));
        }
        assert!(!lb.try_acquire_at(1, now));
        let later = now + Duration::from_millis(30);
        assert!(lb.try_acquire_at(1, later));
    }

    #[test]
    fn leaky_reset_clears_level() {
        let now = Instant::now();
        let mut lb =
            LeakyBucket::new_at(&leaky_cfg(0.01, 5), LeakyBucketOverflow::Drop, now).unwrap();
        for _ in 0..5 {
            let _ = lb.try_acquire_at(1, now);
        }
        assert!(!lb.try_acquire_at(1, now));
        lb.reset_at(now);
        assert!(lb.try_acquire_at(5, now));
    }

    #[test]
    fn leaky_release_drains_level() {
        let now = Instant::now();
        let mut lb =
            LeakyBucket::new_at(&leaky_cfg(0.01, 10), LeakyBucketOverflow::Drop, now).unwrap();
        for _ in 0..5 {
            let _ = lb.try_acquire_at(1, now);
        }
        lb.release(3);
        let l = lb.current_level(now);
        assert!(l <= 2.0 + 0.1);
    }

    #[test]
    fn fixed_new_rejects_zero_window() {
        let r = FixedWindow::new(Duration::ZERO, 10, FixedWindowReset::OnWindowEnd);
        assert!(matches!(r, Err(RateLimitError::ZeroWindowSize)));
    }

    #[test]
    fn fixed_counts_within_window() {
        let now = Instant::now();
        let mut fw =
            FixedWindow::new_at(Duration::from_secs(10), 3, FixedWindowReset::OnWindowEnd, now)
                .unwrap();
        assert!(fw.try_acquire_at(now));
        assert!(fw.try_acquire_at(now));
        assert!(fw.try_acquire_at(now));
        assert!(!fw.try_acquire_at(now));
    }

    #[test]
    fn fixed_reset_clears_counter() {
        let now = Instant::now();
        let mut fw =
            FixedWindow::new_at(Duration::from_secs(10), 2, FixedWindowReset::OnDemand, now)
                .unwrap();
        assert!(fw.try_acquire_at(now));
        assert!(fw.try_acquire_at(now));
        assert!(!fw.try_acquire_at(now));
        fw.reset_at(now);
        assert!(fw.try_acquire_at(now));
    }

    #[test]
    fn fixed_boundary_spike_is_documented() {
        let now = Instant::now();
        let mut fw = FixedWindow::new_at(
            Duration::from_millis(50),
            2,
            FixedWindowReset::OnWindowEnd,
            now,
        )
        .unwrap();
        assert!(fw.try_acquire_at(now));
        assert!(fw.try_acquire_at(now));
        let later = now + Duration::from_millis(60);
        assert!(fw.try_acquire_at(later));
        assert!(fw.try_acquire_at(later));
    }

    #[test]
    fn fixed_on_demand_does_not_auto_rotate() {
        let now = Instant::now();
        let mut fw = FixedWindow::new_at(
            Duration::from_millis(20),
            1,
            FixedWindowReset::OnDemand,
            now,
        )
        .unwrap();
        assert!(fw.try_acquire_at(now));
        assert!(!fw.try_acquire_at(now));
        let later = now + Duration::from_millis(30);
        assert!(!fw.try_acquire_at(later));
        fw.reset_at(later);
        assert!(fw.try_acquire_at(later));
    }

    #[test]
    fn fixed_remaining_decreases() {
        let now = Instant::now();
        let mut fw =
            FixedWindow::new_at(Duration::from_secs(10), 3, FixedWindowReset::OnWindowEnd, now)
                .unwrap();
        assert_eq!(fw.remaining(now), 3);
        let _ = fw.try_acquire_at(now);
        assert_eq!(fw.remaining(now), 2);
    }

    #[test]
    fn sliding_new_rejects_zero_window() {
        let r = SlidingWindow::new(
            Duration::ZERO,
            Duration::from_millis(10),
            10,
            SlidingWindowPrecision::Log,
        );
        assert!(matches!(r, Err(RateLimitError::ZeroWindowSize)));
    }

    #[test]
    fn sliding_log_counts_within_window() {
        let now = Instant::now();
        let mut sw = SlidingWindow::new_at(
            Duration::from_secs(10),
            Duration::ZERO,
            3,
            SlidingWindowPrecision::Log,
            now,
        )
        .unwrap();
        assert!(sw.try_acquire_at(now));
        assert!(sw.try_acquire_at(now));
        assert!(sw.try_acquire_at(now));
        assert!(!sw.try_acquire_at(now));
    }

    #[test]
    fn sliding_log_drops_expired_entries() {
        let now = Instant::now();
        let mut sw = SlidingWindow::new_at(
            Duration::from_millis(50),
            Duration::ZERO,
            2,
            SlidingWindowPrecision::Log,
            now,
        )
        .unwrap();
        assert!(sw.try_acquire_at(now));
        assert!(sw.try_acquire_at(now));
        assert!(!sw.try_acquire_at(now));
        let later = now + Duration::from_millis(60);
        assert!(sw.try_acquire_at(later));
    }

    #[test]
    fn sliding_counter_counts_within_window() {
        let now = Instant::now();
        let mut sw = SlidingWindow::new_at(
            Duration::from_secs(10),
            Duration::from_millis(100),
            3,
            SlidingWindowPrecision::Counter,
            now,
        )
        .unwrap();
        assert!(sw.try_acquire_at(now));
        assert!(sw.try_acquire_at(now));
        assert!(sw.try_acquire_at(now));
        assert!(!sw.try_acquire_at(now));
    }

    #[test]
    fn sliding_reset_clears_state() {
        let now = Instant::now();
        let mut sw = SlidingWindow::new_at(
            Duration::from_secs(10),
            Duration::from_millis(10),
            2,
            SlidingWindowPrecision::Log,
            now,
        )
        .unwrap();
        let _ = sw.try_acquire_at(now);
        let _ = sw.try_acquire_at(now);
        assert!(!sw.try_acquire_at(now));
        sw.reset();
        assert!(sw.try_acquire_at(now));
    }

    #[test]
    fn sliding_remaining_decreases() {
        let now = Instant::now();
        let mut sw = SlidingWindow::new_at(
            Duration::from_secs(10),
            Duration::from_millis(10),
            3,
            SlidingWindowPrecision::Log,
            now,
        )
        .unwrap();
        assert_eq!(sw.remaining(now), 3);
        let _ = sw.try_acquire_at(now);
        assert_eq!(sw.remaining(now), 2);
    }
}
