//! Per-host sliding-window rate limiting for controlled fetch.
//!
//! Ported semantics from legacy `apeireth-tool-fetch::rate_limit` (R231):
//! each host has an independent window of request timestamps; a request is
//! allowed only while fewer than `max_requests` timestamps fall inside
//! `window`. `wait_time` reports how long until the oldest recorded request
//! exits the window, so callers can surface a concrete backoff hint.
//!
//! Pure std (`HashMap` + `VecDeque` + `Instant`), no external dependency.
//! The limiter is process-local by design: it schedules *this* tool's own
//! egress politeness and makes no claim about cross-process or global limits.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Default limit: 60 requests per 60 seconds (canonical default).
pub const DEFAULT_MAX_REQUESTS: usize = 60;
/// Default window: 60 seconds (canonical default).
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(60);

/// Per-host sliding-window rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: usize,
    window: Duration,
    history: HashMap<String, VecDeque<Instant>>,
}

impl RateLimiter {
    /// Default limiter (60 requests / 60 seconds).
    pub fn new() -> Self {
        Self::with_limit(DEFAULT_MAX_REQUESTS, DEFAULT_WINDOW)
    }

    /// Limiter with an explicit `max_requests` per `window`.
    pub fn with_limit(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests: max_requests.max(1),
            window,
            history: HashMap::new(),
        }
    }

    /// True when one more request to `host` is allowed right now.
    pub fn check(&self, host: &str) -> bool {
        let now = Instant::now();
        match self.history.get(host) {
            None => true,
            Some(dq) => {
                let active = dq
                    .iter()
                    .filter(|&&t| now.duration_since(t) <= self.window)
                    .count();
                active < self.max_requests
            }
        }
    }

    /// Record one request to `host` at the current instant.
    ///
    /// Timestamps older than the window are pruned first so the deque stays
    /// bounded by `max_requests` for well-behaved callers.
    pub fn record(&mut self, host: &str) {
        let now = Instant::now();
        let dq = self
            .history
            .entry(host.to_string())
            .or_insert_with(VecDeque::new);
        while let Some(&front) = dq.front() {
            if now.duration_since(front) > self.window {
                dq.pop_front();
            } else {
                break;
            }
        }
        dq.push_back(now);
    }

    /// How long until the next request to `host` is allowed; `None` when it
    /// is allowed immediately (or the host was never recorded).
    pub fn wait_time(&self, host: &str) -> Option<Duration> {
        let now = Instant::now();
        let dq = self.history.get(host)?;
        if dq.len() < self.max_requests {
            return None;
        }
        let oldest = dq.front()?;
        let elapsed = now.duration_since(*oldest);
        if elapsed >= self.window {
            None
        } else {
            Some(self.window - elapsed)
        }
    }

    /// Number of hosts currently tracked.
    pub fn hosts(&self) -> usize {
        self.history.len()
    }

    /// Number of in-window requests recorded for `host`.
    pub fn count(&self, host: &str) -> usize {
        let now = Instant::now();
        match self.history.get(host) {
            None => 0,
            Some(dq) => dq
                .iter()
                .filter(|&&t| now.duration_since(t) <= self.window)
                .count(),
        }
    }

    /// Drop all host counters.
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Drop the counter for a single host. True when the host was tracked.
    pub fn clear_host(&mut self, host: &str) -> bool {
        self.history.remove(host).is_some()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_limiter_is_empty_and_allows() {
        let rl = RateLimiter::new();
        assert!(rl.check("example.com"));
        assert_eq!(rl.count("example.com"), 0);
        assert_eq!(rl.hosts(), 0);
    }

    #[test]
    fn record_then_check_rejects_over_limit() {
        let mut rl = RateLimiter::with_limit(3, Duration::from_secs(60));
        rl.record("a.com");
        rl.record("a.com");
        rl.record("a.com");
        assert_eq!(rl.count("a.com"), 3);
        assert!(
            !rl.check("a.com"),
            "4th request within window must be denied"
        );
    }

    #[test]
    fn different_hosts_are_independent() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_secs(60));
        rl.record("a.com");
        assert!(!rl.check("a.com"));
        assert!(rl.check("b.com"), "different host must stay independent");
    }

    #[test]
    fn wait_time_is_none_while_allowed() {
        let mut rl = RateLimiter::with_limit(2, Duration::from_secs(60));
        rl.record("a.com");
        assert_eq!(rl.wait_time("a.com"), None);
    }

    #[test]
    fn wait_time_reports_backoff_when_at_limit() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_millis(100));
        rl.record("a.com");
        let w = rl.wait_time("a.com");
        assert!(w.is_some());
        assert!(w.unwrap() <= Duration::from_millis(100));
    }

    #[test]
    fn sliding_window_expires() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_millis(50));
        rl.record("a.com");
        assert!(!rl.check("a.com"));
        std::thread::sleep(Duration::from_millis(70));
        assert!(
            rl.check("a.com"),
            "after the window the host must be allowed again"
        );
    }

    #[test]
    fn clear_resets_all_hosts() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_secs(60));
        rl.record("a.com");
        assert!(!rl.check("a.com"));
        rl.clear();
        assert!(rl.check("a.com"));
        assert_eq!(rl.hosts(), 0);
    }

    #[test]
    fn clear_host_removes_a_single_host() {
        let mut rl = RateLimiter::with_limit(2, Duration::from_secs(60));
        rl.record("a.com");
        rl.record("a.com");
        rl.record("b.com");
        assert_eq!(rl.hosts(), 2);
        assert!(rl.clear_host("a.com"));
        assert_eq!(rl.hosts(), 1);
        assert!(rl.check("a.com"), "cleared host must be allowed again");
        assert!(rl.check("b.com"), "host below its own limit stays allowed");
    }

    #[test]
    fn zero_limit_is_clamped_to_one() {
        let mut rl = RateLimiter::with_limit(0, Duration::from_secs(60));
        assert!(rl.check("a.com"));
        rl.record("a.com");
        assert!(!rl.check("a.com"), "clamped limit 1 denies the 2nd request");
    }
}
